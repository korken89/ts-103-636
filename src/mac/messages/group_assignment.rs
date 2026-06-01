//! Group Assignment IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.9.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Group Assignment IE body (§6.4.3.9)
// ---------------------------------------------------------------------------

/// One entry of a Group Assignment body's resource tag tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct GroupResourceTagEntry(u8);

impl GroupResourceTagEntry {
    /// Special Resource Tag value indicating "broadcast for all members".
    pub const BROADCAST: u8 = 0x7F;

    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    pub const fn new(direct_inverted: bool, resource_tag: ResourceTag) -> Self {
        let d = if direct_inverted { 0x80 } else { 0 };
        Self(d | (resource_tag.as_u8() & 0x7F))
    }

    /// Raw on-wire byte.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self.0
    }

    /// `true` iff the Direct bit is set (assignment direction inverted
    /// from Resource Allocation default).
    #[must_use]
    pub const fn direct_inverted(self) -> bool {
        self.0 & 0x80 != 0
    }

    /// Resource Tag carried in this entry.
    #[must_use]
    pub const fn resource_tag(self) -> ResourceTag {
        match ResourceTag::new(self.0 & 0x7F) {
            Some(t) => t,
            None => unreachable!(),
        }
    }
}

/// Owned representation of a Group Assignment IE body.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct GroupAssignmentParts<'a> {
    /// On-wire `Single` bit. `true` = single resource assignment for the
    /// group member; `false` = multiple resource assignments follow.
    pub single: bool,
    pub group_id: GroupId,
    pub tags: &'a [GroupResourceTagEntry],
}

impl GroupAssignmentParts<'_> {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        1 + self.tags.len()
    }

    /// Serialize into `out`. Returns the number of bytes written.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        let single_bit = if self.single { 0x80 } else { 0 };
        out[0] = single_bit | (self.group_id.as_u8() & 0x7F);
        let mut i = 0;
        while i < self.tags.len() {
            out[1 + i] = self.tags[i].as_raw();
            i += 1;
        }
        Ok(len)
    }

    /// Parse the bytes as `Self`.
    pub fn parse(buffer: &[u8]) -> Result<GroupAssignmentParts<'_>, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let single = b0 & 0x80 != 0;
        let group_id = GroupId::new(b0 & 0x7F).ok_or(ParsingError::ReservedValue)?;
        // SAFETY: GroupResourceTagEntry is #[repr(transparent)] over u8.
        // Every byte in &buffer[1..] is a valid bit pattern for u8, and we
        // do not impose any reserved-bit invariant on it (Direct bit + 7
        // bit Resource Tag covers all 8 bits).
        let tags: &[GroupResourceTagEntry] = unsafe {
            core::slice::from_raw_parts(
                buffer[1..].as_ptr().cast::<GroupResourceTagEntry>(),
                buffer.len() - 1,
            )
        };
        Ok(GroupAssignmentParts {
            single,
            group_id,
            tags,
        })
    }
}

impl<'a> MessageBody for GroupAssignmentParts<'a> {
    const IE_TYPE: IEType6bit = IEType6bit::GroupAssignment;
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::encoded_len(self)
    }
    #[inline]
    fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        Self::serialize(self, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_assignment_round_trip() {
        let tags = [
            GroupResourceTagEntry::new(false, ResourceTag::new(0x12).unwrap()),
            GroupResourceTagEntry::new(true, ResourceTag::new(0x7F).unwrap()),
        ];
        let parts = GroupAssignmentParts {
            single: true,
            group_id: GroupId::new(0x42).unwrap(),
            tags: &tags,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 3);
        let parsed = GroupAssignmentParts::parse(&buf[..n]).unwrap();
        assert!(parsed.single);
        assert_eq!(parsed.group_id.as_u8(), 0x42);
        assert_eq!(parsed.tags.len(), 2);
        assert!(!parsed.tags[0].direct_inverted());
        assert_eq!(parsed.tags[0].resource_tag().as_u8(), 0x12);
        assert!(parsed.tags[1].direct_inverted());
        assert_eq!(parsed.tags[1].resource_tag().as_u8(), 0x7F);
    }

    #[test]
    fn group_assignment_rejects_empty_buffer() {
        assert!(GroupAssignmentParts::parse(&[]).is_err());
    }

    #[test]
    fn group_assignment_header_only_round_trip() {
        // Minimum body: 1 byte (Single + GroupId) with no resource tags.
        let parts = GroupAssignmentParts {
            single: false,
            group_id: GroupId::new(0x10).unwrap(),
            tags: &[],
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = GroupAssignmentParts::parse(&buf[..n]).unwrap();
        assert!(!parsed.single);
        assert_eq!(parsed.group_id.as_u8(), 0x10);
        assert_eq!(parsed.tags.len(), 0);
    }

    #[test]
    fn group_assignment_serialize_rejects_short_buffer() {
        let parts = GroupAssignmentParts {
            single: true,
            group_id: GroupId::new(0x42).unwrap(),
            tags: &[GroupResourceTagEntry::new(
                false,
                ResourceTag::new(0x12).unwrap(),
            )],
        };
        // encoded_len = 2 (header + 1 tag); buffer has only 1 byte.
        let mut buf = [0u8; 1];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn group_assignment_broadcast_tag_value() {
        // The BROADCAST constant must equal the on-wire representation of
        // a non-inverted tag with the BROADCAST resource tag value (0x7F).
        let entry = GroupResourceTagEntry::new(
            false,
            ResourceTag::new(GroupResourceTagEntry::BROADCAST).unwrap(),
        );
        assert_eq!(entry.as_raw(), GroupResourceTagEntry::BROADCAST);
        assert!(!entry.direct_inverted());
        assert_eq!(entry.resource_tag().as_u8(), 0x7F);
    }

    #[test]
    fn group_assignment_tag_direct_bit_layout() {
        // Direct = 0 puts the high bit clear; Direct = 1 sets bit 7.
        let no_inv = GroupResourceTagEntry::new(false, ResourceTag::new(0x05).unwrap());
        let inv = GroupResourceTagEntry::new(true, ResourceTag::new(0x05).unwrap());
        assert_eq!(no_inv.as_raw(), 0x05);
        assert_eq!(inv.as_raw(), 0x85);
        assert!(no_inv.resource_tag().as_u8() == inv.resource_tag().as_u8());
    }

    #[test]
    fn group_assignment_bitmap_layout() {
        // B0 has Single in bit 7, GroupId in bits 0..=6. Verify the
        // serialized bits match this exact layout.
        let parts = GroupAssignmentParts {
            single: true,
            group_id: GroupId::new(0x05).unwrap(),
            tags: &[],
        };
        let mut buf = [0u8; 8];
        parts.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x85);

        let parts2 = GroupAssignmentParts {
            single: false,
            group_id: GroupId::new(0x05).unwrap(),
            tags: &[],
        };
        let mut buf2 = [0u8; 8];
        parts2.serialize(&mut buf2).unwrap();
        assert_eq!(buf2[0], 0x05);
    }
}
