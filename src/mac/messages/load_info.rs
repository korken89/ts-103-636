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
}
