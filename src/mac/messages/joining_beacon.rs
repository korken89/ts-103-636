//! Joining Beacon message body (generated codec re-export).
//!
//! The codec lives in [`generated::joining_beacon`](super::generated::joining_beacon); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::joining_beacon::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use heapless::Vec;
    #[test]
    fn joining_beacon_single_channel_round_trip() {
        let mut channels = Vec::new();
        channels
            .push(AbsoluteChannel::try_from_u16(0x0100).unwrap())
            .unwrap();
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms1000,
            channels,
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf[0], 0b0000_1100);

        let parsed = JoiningBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.channels.len(), 1);
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(parsed.channels[0].as_u16(), 0x0100);
    }

    #[test]
    fn joining_beacon_four_channels_round_trip() {
        let channels = Vec::from_slice(&[
            AbsoluteChannel::try_from_u16(0x0100).unwrap(),
            AbsoluteChannel::try_from_u16(0x0200).unwrap(),
            AbsoluteChannel::try_from_u16(0x0300).unwrap(),
            AbsoluteChannel::try_from_u16(0x1FFF).unwrap(),
        ])
        .unwrap();
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms4000,
            channels,
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1 + 4 * 2);
        assert_eq!(buf[0], 0b1101_1000);

        let parsed = JoiningBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.channels.len(), 4);
        assert_eq!(parsed.channels[0].as_u16(), 0x0100);
        assert_eq!(parsed.channels[1].as_u16(), 0x0200);
        assert_eq!(parsed.channels[2].as_u16(), 0x0300);
        assert_eq!(parsed.channels[3].as_u16(), 0x1FFF);
    }

    #[test]
    fn joining_beacon_rejects_zero_channels() {
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms50,
            channels: Vec::new(),
        };
        let mut buf = [0; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn joining_beacon_parse_rejects_empty_buffer() {
        assert!(JoiningBeaconParts::parse(&[]).is_err());
    }

    // -------------------------------------------------------------------
    // Figure 6.4.2.10-1 hand-derived golden vectors
    // B0: bits 7:6 = N (channel count - 1), bits 5:2 = NB Period, bits 1:0 = Reserved
    // Each channel: 3-bit reserved + 13-bit absolute channel (big-endian u16)
    // -------------------------------------------------------------------

    /// Minimal golden vector: 1 channel (N=0b00), period=Ms2000 (code 5).
    /// Total: 3 bytes (1 header + 2 per channel).
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.2.10-1"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 3] = [
            0b00_0101_00, // N=0 (1 channel) | NB Period=5 (Ms2000) | Reserved=0
            0b000_00101,  // Reserved(3b)=0 | Channel 0x05F5 high bits
            0b11110101,   // Channel 0x05F5 low byte (absolute channel 1525)
        ];
        let mut channels = Vec::new();
        channels
            .push(AbsoluteChannel::try_from_u16(0x05F5).unwrap())
            .unwrap();
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms2000, // code 5
            channels,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = JoiningBeaconParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed, parts);
    }

    /// Full golden vector: 4 channels (N=0b11), period=Ms4000 (code 6).
    /// Channels: 0x0123, 0x07BC, 0x0E6F, 0x1ABC.
    /// Total: 9 bytes (1 header + 4*2 channel bytes).
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.2.10-1"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 9] = [
            0b11_0110_00, // N=3 (4 channels) | NB Period=6 (Ms4000) | Reserved=0
            0b000_00001,  // Reserved | Channel 0x0123 high (291)
            0b00100011,   // Channel 0x0123 low
            0b000_00111,  // Reserved | Channel 0x07BC high (1980)
            0b10111100,   // Channel 0x07BC low
            0b000_01110,  // Reserved | Channel 0x0E6F high (3695)
            0b01101111,   // Channel 0x0E6F low
            0b000_11010,  // Reserved | Channel 0x1ABC high (6844)
            0b10111100,   // Channel 0x1ABC low
        ];
        let channels = Vec::from_slice(&[
            AbsoluteChannel::try_from_u16(0x0123).unwrap(),
            AbsoluteChannel::try_from_u16(0x07BC).unwrap(),
            AbsoluteChannel::try_from_u16(0x0E6F).unwrap(),
            AbsoluteChannel::try_from_u16(0x1ABC).unwrap(),
        ])
        .unwrap();
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms4000, // code 6
            channels,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = JoiningBeaconParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed, parts);
    }
}
