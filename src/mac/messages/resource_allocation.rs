//! Resource Allocation IE body (generated codec re-export).
//!
//! The codec lives in [`generated::resource_allocation`](super::generated::resource_allocation); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

use crate::types::*;

pub use super::common::AllocationPair;
pub use super::generated::resource_allocation::*;

/// Repetition policy for a Resource Allocation. `None` in the parent
/// structure corresponds to on-wire Repeat = `0b000` (Single).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RepeatPolicy {
    pub mode: RepeatMode,
    /// `1` means the next frame/subslot. The `NonZero<u8>` type makes
    /// the spec's "`0` is not defined" rule unrepresentable.
    pub repetition: Repetition,
    /// `0xFF` means permanent.
    pub validity: Validity,
}

/// Optional fields shared by all non-`ReleaseAll` Resource Allocation
/// variants. Each field corresponds to one bit of the bitmap; `Some`
/// means the corresponding bit is set on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AllocationOptions {
    /// On-wire `Add` bit. `false` = new/replace, `true` = additional.
    pub add: bool,
    /// On-wire `ID` bit. `Some(..)` means the IE was sent on a beacon
    /// and identifies the recipient; `None` means unicast.
    pub recipient: Option<ShortRdId>,
    /// On-wire `Repeat` field. `None` = Single (0b000); `Some(..)` =
    /// one of the four repeating modes plus Repetition + Validity bytes.
    pub repeat: Option<RepeatPolicy>,
    /// On-wire `SFN` bit. `Some(..)` means the allocation becomes valid
    /// from the indicated SFN; `None` means immediately.
    pub sfn_value: Option<u8>,
    /// On-wire `Channel` bit. `Some(..)` overrides the channel the IE
    /// was received on.
    pub channel: Option<AbsoluteChannel>,
    /// On-wire `RLF` bit. `Some(..)` overrides the default
    /// `dectScheduledResourceFailure` timer.
    pub resource_failure_timer: Option<DectScheduledResourceFailure>,
}

/// Allocation-type-specific portion of a Resource Allocation IE body.
/// The outer enum makes the Allocation Type split unrepresentable in
/// the wrong variant: `Both` carries two pairs by construction; a
/// `Downlink` / `Uplink` carries exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResourceAllocationKind {
    /// On-wire Allocation Type = `0b00`. Body is exactly one byte;
    /// every other bitmap bit is don't-care.
    ReleaseAll,
    /// On-wire Allocation Type = `0b01`. One downlink pair.
    Downlink {
        /// Downlink allocation pair (start subslot + length).
        pair: AllocationPair,
        /// Shared optional fields gated by the bitmap.
        options: AllocationOptions,
    },
    /// On-wire Allocation Type = `0b10`. One uplink pair.
    Uplink {
        /// Uplink allocation pair (start subslot + length).
        pair: AllocationPair,
        /// Shared optional fields gated by the bitmap.
        options: AllocationOptions,
    },
    /// On-wire Allocation Type = `0b11`. Carries both DL and UL pairs.
    Both {
        /// Downlink allocation pair.
        dl: AllocationPair,
        /// Uplink allocation pair.
        ul: AllocationPair,
        /// Shared optional fields gated by the bitmap.
        options: AllocationOptions,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_allocation_release_all_round_trip() {
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::ReleaseAll,
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0b0000_0000); // alloc type 0b00 in high bits

        let parsed = ResourceAllocationParts::parse(&buf[..n], Mu::M1).unwrap();
        assert!(matches!(parsed.kind, ResourceAllocationKind::ReleaseAll));
    }

    #[test]
    fn resource_allocation_downlink_minimal_mu1_round_trip() {
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::Downlink {
                pair: AllocationPair {
                    start_subslot: 17,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(5).unwrap(),
                },
                options: AllocationOptions::default(),
            },
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 2 (bitmap) + 1 (start subslot, 8-bit) + 1 (length type+length) = 4
        assert_eq!(n, 4);

        let parsed = ResourceAllocationParts::parse(&buf[..n], Mu::M1).unwrap();
        match parsed.kind {
            ResourceAllocationKind::Downlink { pair, options } => {
                assert_eq!(pair.start_subslot, 17);
                assert!(matches!(pair.length_type, PacketLengthType::Subslot));
                assert_eq!(pair.length.as_u8(), 5);
                assert!(!options.add);
                assert!(options.recipient.is_none());
                assert!(options.repeat.is_none());
                assert!(options.sfn_value.is_none());
                assert!(options.channel.is_none());
                assert!(options.resource_failure_timer.is_none());
            }
            _ => panic!("expected downlink"),
        }
    }

    #[test]
    fn resource_allocation_both_with_all_options_mu8_round_trip() {
        let parts = ResourceAllocationParts {
            mu: Mu::M8,
            kind: ResourceAllocationKind::Both {
                dl: AllocationPair {
                    start_subslot: 0x100, // requires 9-bit (mu > 4)
                    length_type: PacketLengthType::Slot,
                    length: RaLength::new(3).unwrap(),
                },
                ul: AllocationPair {
                    start_subslot: 0x123,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(7).unwrap(),
                },
                options: AllocationOptions {
                    add: true,
                    recipient: Some(ShortRdId::new(0xABCD).unwrap()),
                    repeat: Some(RepeatPolicy {
                        mode: RepeatMode::PerFrame,
                        repetition: Repetition::new(4).unwrap(),
                        validity: Validity(100),
                    }),
                    sfn_value: Some(42),
                    channel: Some(AbsoluteChannel::new(0x1234).unwrap()),
                    resource_failure_timer: Some(DectScheduledResourceFailure::Ms200),
                },
            },
        };
        let mut buf = [0; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 2 (bitmap) + 3 (DL: 2 ss + 1 len) + 3 (UL) + 2 (recipient) + 2 (repeat) + 1 (sfn) + 2 (channel) + 1 (rlf) = 16
        assert_eq!(n, 16);

        let parsed = ResourceAllocationParts::parse(&buf[..n], Mu::M8).unwrap();
        match parsed.kind {
            ResourceAllocationKind::Both { dl, ul, options } => {
                assert_eq!(dl.start_subslot, 0x100);
                assert!(matches!(dl.length_type, PacketLengthType::Slot));
                assert_eq!(dl.length.as_u8(), 3);
                assert_eq!(ul.start_subslot, 0x123);
                assert!(matches!(ul.length_type, PacketLengthType::Subslot));
                assert_eq!(ul.length.as_u8(), 7);
                assert!(options.add);
                assert_eq!(options.recipient.unwrap().as_u16(), 0xABCD);
                let rp = options.repeat.unwrap();
                assert!(matches!(rp.mode, RepeatMode::PerFrame));
                assert_eq!(rp.repetition.as_u8(), 4);
                assert_eq!(rp.validity.0, 100);
                assert_eq!(options.sfn_value, Some(42));
                assert_eq!(options.channel.unwrap().as_u16(), 0x1234);
                assert_eq!(
                    options.resource_failure_timer,
                    Some(DectScheduledResourceFailure::Ms200)
                );
            }
            _ => panic!("expected both"),
        }
    }

    #[test]
    fn resource_allocation_serialize_rejects_start_subslot_overflow_mu1() {
        // mu=1 supports 8-bit start subslot (0..=255); 256 overflows.
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::Downlink {
                pair: AllocationPair {
                    start_subslot: 256,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(0).unwrap(),
                },
                options: AllocationOptions::default(),
            },
        };
        let mut buf = [0; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn resource_allocation_parser_rejects_reserved_repeat() {
        // alloc=DL, repeat=0b101 reserved.
        // B0 = 01 00 101 0 = 0b0100_1010
        let buf = [0b0100_1010, 0, 0, 0];
        assert!(ResourceAllocationParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn resource_allocation_parser_rejects_reserved_rlf_timer() {
        // alloc=DL, no add, no id, no repeat, no sfn.
        // B0 = 01 00 000 0 = 0b0100_0000
        // B1 = RLF=1 only (0x40).
        // pair = 1+1 bytes (mu=1) = ss=0, len=0
        // RLF byte = 0 (reserved).
        let buf = [0b0100_0000, 0x40, 0, 0, 0];
        assert!(ResourceAllocationParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn resource_allocation_parser_rejects_zero_repetition() {
        // alloc=DL, repeat=PerFrame (001), no others.
        // B0 = 01 00 001 0 = 0b0100_0010
        // B1 = 0
        // pair = 2 bytes
        // repetition byte = 0 (invalid), validity = 0
        let buf = [0b0100_0010, 0, 0, 0, 0, 0];
        assert!(ResourceAllocationParts::parse(&buf, Mu::M1).is_err());
    }
}
