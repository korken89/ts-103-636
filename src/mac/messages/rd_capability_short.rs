//! RD Capability IE (short form) body (generated codec re-export).
//!
//! The codec lives in [`generated::rd_capability_short`](super::generated::rd_capability_short); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::rd_capability_short::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    /// Golden vector hand-derived from Figure 6.4.3.15-1 / Table 6.4.3.15-1.
    ///
    /// Byte layout (8 bits):
    ///   bits 7-6: reserved = 0b00
    ///   bit 5:    CB_MC = 1  (supports association without Cluster Beacon monitoring)
    ///   bits 4-1: HARQ feedback delay = 4 subslots (0b0100 -> shifted to bits4-1 = 0b1000 = 0x08)
    ///   bit 0:    DWA = 1  (supports uplink data without association)
    ///
    ///   byte0 = 00_1_0100_1 = 0b0010_1001 = 0x29
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows RD Capability Short field layout"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 1] = [
            0b00_1_0100_1, // reserved(00) | CB_MC=1 | HARQ_delay=4 subslots | DWA=1
        ];
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(4).unwrap(), // 4 subslots
            dwa: true,
        };
        let mut buf = [0u8; 4];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(RdCapabilityShortParts::parse(&GOLDEN).unwrap(), parts);
    }

    #[test]
    fn rd_capability_short_round_trip() {
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(3).unwrap(),
            dwa: false,
        };
        let mut buf = [0; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 3);
        assert!(!parsed.dwa);
    }

    #[test]
    fn rd_capability_short_rejects_empty_buffer() {
        assert!(RdCapabilityShortParts::parse(&[]).is_err());
    }

    #[test]
    fn rd_capability_short_rejects_reserved_harq_feedback_delay() {
        // HarqFeedbackDelay only accepts 0..=6. Place 7 in bits 1..=4.
        // B0 = 0 | HARQ-delay (4) | dwa (1). HARQ=7 -> b0 = 7 << 1 = 0x0E.
        let buf = [0x0Eu8];
        assert!(RdCapabilityShortParts::parse(&buf).is_err());
    }

    #[test]
    fn rd_capability_short_serialize_rejects_empty_buffer() {
        let parts = RdCapabilityShortParts {
            cb_mc: false,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(0).unwrap(),
            dwa: false,
        };
        let mut buf = [0; 0];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn rd_capability_short_all_flags_set_round_trip() {
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(6).unwrap(),
            dwa: true,
        };
        let mut buf = [0; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 6);
        assert!(parsed.dwa);
    }

    #[test]
    fn rd_capability_short_dwa_only_round_trip() {
        // CB_MC=0, HARQ=0, DWA=1 -> B0 = 0x01
        let parts = RdCapabilityShortParts {
            cb_mc: false,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(0).unwrap(),
            dwa: true,
        };
        let mut buf = [0; 4];
        parts.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x01);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(!parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 0);
        assert!(parsed.dwa);
    }

    #[test]
    fn rd_capability_short_cb_mc_only_round_trip() {
        // CB_MC=1, HARQ=0, DWA=0 -> B0 = 0x20
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(0).unwrap(),
            dwa: false,
        };
        let mut buf = [0; 4];
        parts.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x20);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(parsed.cb_mc);
        assert!(!parsed.dwa);
    }

    #[test]
    fn rd_capability_short_bit_layout() {
        // Spec layout: bit 5 = CB_MC, bits 1..=4 = HARQ feedback delay,
        // bit 0 = DWA. Verify each via a deliberately constructed value.
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(5).unwrap(),
            dwa: true,
        };
        let mut buf = [0; 4];
        parts.serialize(&mut buf).unwrap();
        // 0x20 (CB_MC) | (5 << 1) (HARQ=5 -> 0x0A) | 0x01 (DWA) = 0x2B
        assert_eq!(buf[0], 0x2B);
    }
}
