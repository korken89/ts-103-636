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
            .push(AbsoluteChannel::new(0x0100).unwrap())
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
            AbsoluteChannel::new(0x0100).unwrap(),
            AbsoluteChannel::new(0x0200).unwrap(),
            AbsoluteChannel::new(0x0300).unwrap(),
            AbsoluteChannel::new(0x1FFF).unwrap(),
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
}
