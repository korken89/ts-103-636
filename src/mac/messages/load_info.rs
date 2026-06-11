//! Load Info IE body (generated codec re-export).
//!
//! The codec lives in [`generated::load_info`](super::generated::load_info); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle (including the fuzzer-found truncation
//! regression) and predate the generated codec.

pub use super::generated::load_info::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParsingError;
    use crate::types::*;
    #[test]
    fn load_info_minimal_round_trip() {
        let parts = LoadInfoParts {
            traffic_load: LoadPercentage(128),
            max_associated_rds: 100,
            currently_associated_ft_mode: LoadPercentage(50),
            currently_associated_pt_mode: None,
            rach_load: None,
            channel_load: None,
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4);
        let parsed = LoadInfoParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.traffic_load.0, 128);
        assert_eq!(parsed.max_associated_rds, 100);
        assert_eq!(parsed.currently_associated_ft_mode.0, 50);
        assert!(parsed.currently_associated_pt_mode.is_none());
        assert!(parsed.rach_load.is_none());
        assert!(parsed.channel_load.is_none());
    }

    #[test]
    fn load_info_full_round_trip_with_16bit_max() {
        let parts = LoadInfoParts {
            traffic_load: LoadPercentage(0xFF),
            max_associated_rds: 1024,
            currently_associated_ft_mode: LoadPercentage(75),
            currently_associated_pt_mode: Some(LoadPercentage(25)),
            rach_load: Some(LoadPercentage(40)),
            channel_load: Some((LoadPercentage(60), LoadPercentage(20))),
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (bitmap) + 1 (traffic) + 2 (max 16-bit) + 1 (FT) + 1 (PT) + 1 (RACH) + 2 (channel) = 9
        assert_eq!(n, 9);
        let parsed = LoadInfoParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.max_associated_rds, 1024);
        assert_eq!(parsed.currently_associated_pt_mode.unwrap().0, 25);
        assert_eq!(parsed.rach_load.unwrap().0, 40);
        assert_eq!(parsed.channel_load.unwrap().0.0, 60);
        assert_eq!(parsed.channel_load.unwrap().1.0, 20);
    }

    #[test]
    fn parse_rejects_two_byte_payload() {
        // Fuzzer-found regression (crash-e70c623d): the minimum-length
        // prefix passes, but the mandatory 8-bit MAX associated RDs
        // field is missing. Must be Truncated, not an OOB index.
        assert!(matches!(
            LoadInfoParts::parse(&[0x02, 0x02]),
            Err(ParsingError::Truncated)
        ));
        assert!(matches!(
            LoadInfoParts::parse(&[0x00, 0x00]),
            Err(ParsingError::Truncated)
        ));
    }

    // -------------------------------------------------------------------
    // Figure 6.4.3.10-1 hand-derived golden vectors
    // B0: bits 7:4 Reserved | bit 3 = M (max_assoc 16-bit) | bit 2 = PT
    //     bit 1 = RL | bit 0 = CL
    // B1: Traffic Load percentage (8 bit)
    // B2[+B3]: MAX associated RDs (8-bit when M=0, 16-bit when M=1)
    // B next: FT mode percentage (always present)
    // B next: PT mode percentage (present when PT=1)
    // B next: RACH load percentage (present when RL=1)
    // B next: Free subslot percentage (present when CL=1)
    // B next: Busy subslot percentage (present when CL=1)
    // -------------------------------------------------------------------

    /// Minimal golden vector: 8-bit MAX RDs, no optional fields.
    /// Total: 4 bytes.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.10-1"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 4] = [
            0b0000_0000, // Reserved | M=0 (8-bit max) | PT=0 | RL=0 | CL=0
            0x40,        // Traffic Load = 64 (~25 %)
            0x32,        // MAX associated RDs = 50 (8-bit)
            0x80,        // Currently associated FT mode = 128 (~50 %)
        ];
        let parts = LoadInfoParts {
            traffic_load: LoadPercentage(0x40),
            max_associated_rds: 0x32,
            currently_associated_ft_mode: LoadPercentage(0x80),
            currently_associated_pt_mode: None,
            rach_load: None,
            channel_load: None,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(LoadInfoParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Full golden vector: 16-bit MAX RDs, all optional fields present.
    /// Total: 9 bytes.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.10-1"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 9] = [
            0b0000_1111, // Reserved | M=1 (16-bit max) | PT=1 | RL=1 | CL=1
            0xC8,        // Traffic Load = 200 (~78 %)
            0x01,        // MAX associated RDs = 500 (0x01F4) high byte
            0xF4,        // MAX associated RDs = 500 (0x01F4) low byte
            0x55,        // Currently associated FT mode = 85 (~33 %)
            0x2A,        // Currently associated PT mode = 42 (~16 %)
            0x64,        // RACH Load = 100 (~39 %)
            0x80,        // Subslots detected free = 128 (~50 %)
            0x40,        // Subslots detected busy = 64 (~25 %)
        ];
        let parts = LoadInfoParts {
            traffic_load: LoadPercentage(0xC8),
            max_associated_rds: 0x01F4, // 500 > 255, triggers 16-bit encoding (M=1)
            currently_associated_ft_mode: LoadPercentage(0x55),
            currently_associated_pt_mode: Some(LoadPercentage(0x2A)),
            rach_load: Some(LoadPercentage(0x64)),
            channel_load: Some((LoadPercentage(0x80), LoadPercentage(0x40))),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(LoadInfoParts::parse(&GOLDEN).unwrap(), parts);
    }
}
