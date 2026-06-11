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
        let mut buf = [0; 16];
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
        let mut buf = [0; 8];
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
        let mut buf = [0; 1];
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
        let mut buf = [0; 8];
        parts.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x85);

        let parts2 = GroupAssignmentParts {
            single: false,
            group_id: GroupId::new(0x05).unwrap(),
            tags: &[],
        };
        let mut buf2 = [0; 8];
        parts2.serialize(&mut buf2).unwrap();
        assert_eq!(buf2[0], 0x05);
    }

    // -------------------------------------------------------------------
    // Figure 6.4.3.9-1 hand-derived golden vectors
    // B0: bit 7 = Single, bits 6:0 = Group ID
    // Bn: bit 7 = Direct, bits 6:0 = Resource Tag
    // -------------------------------------------------------------------

    /// Minimal golden vector: Single=1, GroupId=0x1A, no tag entries.
    /// Total: 1 byte.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.9-1"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 1] = [
            0b1_0011010, // Single=1 | GroupId=0x1A
        ];
        let parts = GroupAssignmentParts {
            single: true,
            group_id: GroupId::new(0x1A).unwrap(),
            tags: &[],
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = GroupAssignmentParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed, parts);
    }

    /// Full golden vector: Single=0, GroupId=0x35, three tag entries.
    /// Tag 0: Direct=0, Tag=0x12  (not inverted, tag=18)
    /// Tag 1: Direct=1, Tag=0x2B  (inverted, tag=43)
    /// Tag 2: Direct=0, Tag=0x7F  (broadcast tag, Table 6.4.3.9-1)
    /// Total: 4 bytes.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.9-1"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 4] = [
            0b0_0110101, // Single=0 | GroupId=0x35
            0b0_0010010, // Direct=0 | Resource Tag=0x12  (Table 6.4.3.9-1: direction per RA IE)
            0b1_0101011, // Direct=1 | Resource Tag=0x2B  (direction inverted)
            0b0_1111111, // Direct=0 | Resource Tag=0x7F  (broadcast for all group members)
        ];
        let tags = [
            GroupResourceTagEntry::new(false, ResourceTag::new(0x12).unwrap()),
            GroupResourceTagEntry::new(true, ResourceTag::new(0x2B).unwrap()),
            GroupResourceTagEntry::new(false, ResourceTag::new(0x7F).unwrap()),
        ];
        let parts = GroupAssignmentParts {
            single: false,
            group_id: GroupId::new(0x35).unwrap(),
            tags: &tags,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = GroupAssignmentParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed, parts);
    }
}
