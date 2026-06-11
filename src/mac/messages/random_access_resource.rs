//! Random Access Resource IE body (generated codec re-export).
//!
//! The codec lives in [`generated::random_access_resource`](super::generated::random_access_resource); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

use crate::types::*;

pub use super::generated::random_access_resource::*;

/// Repetition / Validity carried by a Random Access Resource IE when
/// the 2-bit Repeat field is non-zero. `None` in the parent struct
/// corresponds to on-wire Repeat = `0b00` (Single).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RachRepeatPolicy {
    pub mode: RachRepeatMode,
    /// `1` means the next frame/subslot. The `NonZero<u8>` type makes
    /// the spec's "`0` is not defined" rule unrepresentable.
    pub repetition: Repetition,
    /// `0xFF` means permanent.
    pub validity: Validity,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mac::messages::common::AllocationPair;

    /// Golden vector (minimal) hand-derived from Figure 6.4.3.4-1 / Table 6.4.3.4-1.
    ///
    /// Uses Mu::M1 (mu=1 <= 4, so start_subslot is 8 bits wide).
    /// No optional fields (repeat absent, sfn absent, channel absent, channel_2 absent).
    ///
    /// Byte layout:
    ///   byte 0 (bitmap):
    ///     bits 7-5: Reserved = 0
    ///     bits 4-3: Repeat = 0b00 (single, no repeat, fields absent)
    ///     bit 2: SFN = 0 (absent)
    ///     bit 1: Channel = 0 (absent)
    ///     bit 0: Chan_2 = 0 (absent)
    ///     -> 0x00
    ///   byte 1: start_subslot (8-bit) = 0xA5
    ///   byte 2: length_type | length
    ///     bit 7: LT = 1 (Slot)
    ///     bits 6-0: Length = 7 (RaLength)
    ///     -> 0b1_0000111 = 0x87
    ///   byte 3: max_length_type | max_rach_length | cwmin_sig
    ///     bit 7: MLT = 0 (Subslot)
    ///     bits 6-3: MAX RACH Len = 5 -> 0b0_0101
    ///     bits 2-0: CWmin = 3 -> 0b011
    ///     -> 0b0_0101_011 = 0x2B
    ///   byte 4: dect_delay | response_window | cwmax_sig
    ///     bit 7: DD = 0
    ///     bits 6-3: Resp Window = 7 -> 0b0111
    ///     bits 2-0: CWmax = 5 -> 0b101
    ///     -> 0b0_0111_101 = 0x3D
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Random Access Resource field layout"
    )]
    fn golden_vector_minimal() {
        // Mu::M1: mu=1 <= 4, so start_subslot occupies 1 byte (8-bit field).
        const GOLDEN: [u8; 5] = [
            0b000_00_0_0_0, // Reserved(3b)|Repeat=00|SFN=0|CH=0|CH2=0
            0xA5,           // start_subslot = 0xA5 (8-bit)
            0b1_0000111,    // LT=Slot | Length=7
            0b0_0101_011,   // MLT=Subslot | MaxRachLen=5 | CWmin=3
            0b0_0111_101,   // DD=0 | RespWindow=7 | CWmax=5
        ];
        let parts = RandomAccessResourceParts {
            mu: Mu::M1,
            pair: AllocationPair {
                start_subslot: 0xA5,
                length_type: PacketLengthType::Slot,
                length: RaLength::new(7).unwrap(),
            },
            max_length_type: PacketLengthType::Subslot,
            max_rach_length: MaxRachLength::new(5).unwrap(),
            cwmin_sig: Cwsig::new(3).unwrap(),
            dect_delay: false,
            response_window: ResponseWindow::new(7).unwrap(),
            cwmax_sig: Cwsig::new(5).unwrap(),
            repeat: None,
            sfn_value: None,
            channel: None,
            channel_2: None,
        };
        let mut buf = [0u8; 32];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(
            RandomAccessResourceParts::parse(&GOLDEN, Mu::M1).unwrap(),
            parts
        );
    }

    /// Golden vector (full) hand-derived from Figure 6.4.3.4-1 / Table 6.4.3.4-1.
    ///
    /// Uses Mu::M8 (mu=8 > 4, so start_subslot is 9 bits wide -> 2 bytes on wire).
    /// All optional fields present.
    ///
    /// Byte layout:
    ///   byte 0 (bitmap):
    ///     bits 7-5: Reserved = 0
    ///     bits 4-3: Repeat = 0b01 (PerFrame)
    ///     bit 2: SFN = 1 (present)
    ///     bit 1: Channel = 1 (present)
    ///     bit 0: Chan_2 = 1 (present)
    ///     -> 0b000_01_1_1_1 = 0x0F
    ///   bytes 1-2: start_subslot (9-bit) = 0x01AB -> [0x01, 0xAB]
    ///   byte 3: LT=Subslot(0) | Length=0x3F -> 0x3F
    ///   byte 4: MLT=Slot(1) | MaxRachLen=10 | CWmin=6
    ///     -> 0b1_1010_110 = 0xD6
    ///   byte 5: DD=1 | RespWindow=12 | CWmax=7
    ///     -> 0b1_1100_111 = 0xE7
    ///   byte 6: repetition = 3 -> 0x03
    ///   byte 7: validity = 0xAA -> 0xAA
    ///   byte 8: sfn_value = 0x42 -> 0x42
    ///   bytes 9-10: channel = 0x0DEF -> [0x0D, 0xEF]
    ///   bytes 11-12: channel_2 = 0x0BCD -> [0x0B, 0xCD]
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Random Access Resource field layout"
    )]
    fn golden_vector_full() {
        // Mu::M8: mu=8 > 4, so start_subslot occupies 2 bytes (9-bit field, upper 7 bits reserved).
        const GOLDEN: [u8; 13] = [
            0b000_01_1_1_1, // Reserved(3b)|Repeat=01(PerFrame)|SFN=1|CH=1|CH2=1
            0x01,
            0xAB,         // start_subslot = 0x01AB (9-bit, upper 7 bits zero)
            0b0_0111111,  // LT=Subslot | Length=63
            0b1_1010_110, // MLT=Slot | MaxRachLen=10 | CWmin=6
            0b1_1100_111, // DD=1 | RespWindow=12 | CWmax=7
            0x03,         // repetition = 3 (PerFrame repeat)
            0xAA,         // validity = 0xAA frames
            0x42,         // sfn_value = 0x42
            0x0D,
            0xEF, // channel = 0x0DEF
            0x0B,
            0xCD, // channel_2 = 0x0BCD
        ];
        let parts = RandomAccessResourceParts {
            mu: Mu::M8,
            pair: AllocationPair {
                start_subslot: 0x01AB,
                length_type: PacketLengthType::Subslot,
                length: RaLength::new(63).unwrap(),
            },
            max_length_type: PacketLengthType::Slot,
            max_rach_length: MaxRachLength::new(10).unwrap(),
            cwmin_sig: Cwsig::new(6).unwrap(),
            dect_delay: true,
            response_window: ResponseWindow::new(12).unwrap(),
            cwmax_sig: Cwsig::new(7).unwrap(),
            repeat: Some(RachRepeatPolicy {
                mode: RachRepeatMode::PerFrame, // code 0b01 (Table 6.4.3.4-1)
                repetition: Repetition::new(3).unwrap(),
                validity: Validity(0xAA),
            }),
            sfn_value: Some(0x42),
            channel: Some(AbsoluteChannel::new(0x0DEF).unwrap()),
            channel_2: Some(AbsoluteChannel::new(0x0BCD).unwrap()),
        };
        let mut buf = [0u8; 32];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(
            RandomAccessResourceParts::parse(&GOLDEN, Mu::M8).unwrap(),
            parts
        );
    }

    fn rach_minimal(mu: Mu) -> RandomAccessResourceParts {
        RandomAccessResourceParts {
            mu,
            pair: AllocationPair {
                start_subslot: 1,
                length_type: PacketLengthType::Subslot,
                length: RaLength::new(1).unwrap(),
            },
            max_length_type: PacketLengthType::Subslot,
            max_rach_length: MaxRachLength::new(2).unwrap(),
            cwmin_sig: Cwsig::new(1).unwrap(),
            dect_delay: false,
            response_window: ResponseWindow::new(3).unwrap(),
            cwmax_sig: Cwsig::new(2).unwrap(),
            repeat: None,
            sfn_value: None,
            channel: None,
            channel_2: None,
        }
    }

    #[test]
    fn rach_minimal_mu1_round_trip() {
        let parts = rach_minimal(Mu::M1);
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (bitmap) + 1 (ss) + 1 (len) + 2 (max+dd) = 5
        assert_eq!(n, 5);
        let parsed = RandomAccessResourceParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(parsed.pair.start_subslot, 1);
        assert_eq!(parsed.pair.length.as_u8(), 1);
        assert_eq!(parsed.max_rach_length.as_u8(), 2);
        assert_eq!(parsed.cwmin_sig.as_u8(), 1);
        assert!(!parsed.dect_delay);
        assert_eq!(parsed.response_window.as_u8(), 3);
        assert_eq!(parsed.response_window.subslots(), 4);
        assert_eq!(parsed.cwmax_sig.as_u8(), 2);
        assert!(parsed.repeat.is_none());
        assert!(parsed.sfn_value.is_none());
        assert!(parsed.channel.is_none());
        assert!(parsed.channel_2.is_none());
    }

    #[test]
    fn rach_full_options_mu8_round_trip() {
        let mut parts = rach_minimal(Mu::M8);
        parts.dect_delay = true;
        parts.repeat = Some(RachRepeatPolicy {
            mode: RachRepeatMode::PerSubslot,
            repetition: Repetition::new(2).unwrap(),
            validity: Validity(0xFF),
        });
        parts.sfn_value = Some(0x80);
        parts.channel = Some(AbsoluteChannel::new(0x1FFF).unwrap());
        parts.channel_2 = Some(AbsoluteChannel::new(0x0001).unwrap());
        parts.pair.start_subslot = 0x1AB; // 9-bit at mu=8

        let mut buf = [0; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (bitmap) + 2 (ss) + 1 (len) + 2 (max+dd) + 2 (repeat) + 1 (sfn) + 2 (ch) + 2 (ch2) = 13
        assert_eq!(n, 13);

        let parsed = RandomAccessResourceParts::parse(&buf[..n], Mu::M8).unwrap();
        assert_eq!(parsed.pair.start_subslot, 0x1AB);
        assert!(parsed.dect_delay);
        let r = parsed.repeat.unwrap();
        assert!(matches!(r.mode, RachRepeatMode::PerSubslot));
        assert_eq!(r.repetition.as_u8(), 2);
        assert_eq!(r.validity.0, 0xFF);
        assert_eq!(parsed.sfn_value, Some(0x80));
        assert_eq!(parsed.channel.unwrap().as_u16(), 0x1FFF);
        assert_eq!(parsed.channel_2.unwrap().as_u16(), 0x0001);
    }

    #[test]
    fn rach_parser_rejects_reserved_repeat() {
        // bitmap with Repeat = 0b11 (reserved). All other bits 0.
        // B0 = 000 11 0 0 0 = 0b0001_1000
        let buf = [0b0001_1000, 0, 0, 0, 0];
        assert!(RandomAccessResourceParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn rach_parser_rejects_zero_repetition() {
        // Repeat = 01 (PerFrame), no other options.
        // B0 = 000 01 0 0 0 = 0b0000_1000
        // mu=1: pair=2 bytes, then max+dd 2 bytes, then repetition(0)+validity
        let buf = [0b0000_1000, 0, 0, 0, 0, 0, 0];
        assert!(RandomAccessResourceParts::parse(&buf, Mu::M1).is_err());
    }
}
