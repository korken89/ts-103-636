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
