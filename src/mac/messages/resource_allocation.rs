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

    // -----------------------------------------------------------------------
    // Golden vectors - hand-derived from ETSI TS 103 636-4 clause 6.4.3.3,
    // Figure 6.4.3.3-1, Table 6.4.3.3-1.
    //
    // B0 layout: AT[1:0](b7..b6)|Add(b5)|ID(b4)|RPT[2:0](b3..b1)|SFN(b0)
    // B1 layout: CH(b7)|RLF(b6)|Reserved[5:0]
    // Start subslot: 8-bit when mu <= 4; 9-bit (in 2 bytes, high bit of
    //   first byte is the MSB of the 9-bit value) when mu > 4.
    // Length byte: LT(b7)|Length[6:0](b6..b0)
    // -----------------------------------------------------------------------

    /// Clause 6.4.3.3, minimal: Allocation Type = 00 (Release All) with
    /// Mu::M1. Exactly one byte (the AT bits; all other fields absent).
    /// Table 6.4.3.3-1: "No other fields are present in this IE."
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows resource-allocation field layout"
    )]
    fn golden_vector_minimal_resource_allocation() {
        const GOLDEN: [u8; 1] = [
            0b00_0_0_000_0, // AT=00(ReleaseAll); remaining bits don't care per spec
        ];
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::ReleaseAll,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(
            ResourceAllocationParts::parse(&GOLDEN, Mu::M1).unwrap(),
            parts
        );
    }

    /// Clause 6.4.3.3, full: AT=11(Both DL+UL) with Mu::M8 (mu > 4,
    /// so start subslot is 9-bit, encoded in 2 bytes). All optional fields
    /// present: Add=1, ID=1 (recipient), RPT=001 (PerFrame), SFN=1,
    /// CH=1, RLF=1.
    ///
    /// B0 = AT(11)|Add(1)|ID(1)|RPT(001)|SFN(1)
    ///    = 0b11_1_1_001_1 = 0xF3
    /// B1 = CH(1)|RLF(1)|Rsv = 0b1100_0000 = 0xC0
    /// DL start_subslot = 0x015A (342, 9-bit, to_be: [0x01, 0x5A])
    /// DL length byte = LT=1(Slot)|Length=20 = (1<<7)|20 = 0x80|0x14 = 0x94
    /// UL start_subslot = 0x00B4 (180, 9-bit, to_be: [0x00, 0xB4])
    /// UL length byte = LT=0(Subslot)|Length=10 = 0x0A
    /// recipient = 0xBEEF -> [0xBE, 0xEF]
    /// repetition = 3 (0x03), validity = 50 (0x32)
    /// sfn_value = 0x7E
    /// channel = 0x1234 (13-bit, as_u16()=0x1234, wire [0x12, 0x34])
    /// resource_failure_timer = Ms500 (0b0110), wire: v.as_u8() & 0x0F = 0x06
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows resource-allocation field layout"
    )]
    fn golden_vector_full_resource_allocation() {
        const GOLDEN: [u8; 16] = [
            0b11_1_1_001_1, // AT=11(Both) Add=1 ID=1(recipient) RPT=001(PerFrame) SFN=1
            0b11_000000,    // CH=1(channel) RLF=1(rlf_timer) Reserved=0
            0x01,           // DL start_subslot[8..1] (9-bit, MSB of 0x015A)
            0x5A,           // DL start_subslot[7..0]
            0x94,           // DL length_type=1(Slot) length=20 -> (1<<7)|20 = 0x94
            0x00,           // UL start_subslot[8..1] (9-bit, MSB of 0x00B4)
            0xB4,           // UL start_subslot[7..0]
            0x0A,           // UL length_type=0(Subslot) length=10 -> 0x0A
            0xBE,           // recipient ShortRdId 0xBEEF high byte
            0xEF,           // recipient ShortRdId 0xBEEF low byte
            0x03,           // repetition = 3 (next-next frame after this one)
            0x32,           // validity = 50 frames
            0x7E,           // sfn_value = 0x7E
            0x12,           // channel 0x1234 (13-bit) high byte
            0x34,           // channel 0x1234 low byte
            0x06,           // resource_failure_timer Ms500(0b0110); Table 6.4.3.3-2 code 0110=500ms
        ];
        let parts = ResourceAllocationParts {
            // mu=8 (M8) > 4 -> 9-bit start-subslot field (two bytes on wire)
            mu: Mu::M8,
            kind: ResourceAllocationKind::Both {
                dl: AllocationPair {
                    start_subslot: 0x015A,
                    length_type: PacketLengthType::Slot,
                    length: RaLength::new(20).unwrap(),
                },
                ul: AllocationPair {
                    start_subslot: 0x00B4,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(10).unwrap(),
                },
                options: AllocationOptions {
                    add: true,
                    recipient: Some(ShortRdId::new(0xBEEF).unwrap()),
                    repeat: Some(RepeatPolicy {
                        mode: RepeatMode::PerFrame, // Table 6.4.3.3-1 code 001
                        repetition: Repetition::new(3).unwrap(),
                        validity: Validity(50),
                    }),
                    sfn_value: Some(0x7E),
                    channel: Some(AbsoluteChannel::new(0x1234).unwrap()),
                    resource_failure_timer: Some(DectScheduledResourceFailure::Ms500),
                },
            },
        };
        let mut buf = [0u8; 24];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(
            ResourceAllocationParts::parse(&GOLDEN, Mu::M8).unwrap(),
            parts
        );
    }

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
