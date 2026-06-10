//! Group Assignment IE body (generated codec re-export).
//!
//! The codec lives in [`generated::group_assignment`](super::generated::group_assignment); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

use crate::types::*;

pub use super::generated::group_assignment::*;

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
