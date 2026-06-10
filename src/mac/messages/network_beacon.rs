//! Network Beacon message body (generated codec re-export).
//!
//! The codec lives in [`generated::network_beacon`](super::generated::network_beacon); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::network_beacon::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use heapless::Vec;
    #[test]
    fn network_beacon_parts_round_trip_minimal() {
        let parts = NetworkBeaconParts {
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            next_cluster_channel: AbsoluteChannel::new(0x0100).unwrap(),
            time_to_next: 0x1234_5678,
            cluster_max_tx_power: None,
            current_cluster_channel: None,
            additional_channels: Vec::new(),
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, parts.encoded_len());
        assert_eq!(n, 8);

        let parsed = NetworkBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.power_const, PowerConst::Unconstrained);
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms100);
        assert_eq!(parsed.cluster_beacon_period, ClusterBeaconPeriod::Ms100);
        assert_eq!(parsed.next_cluster_channel.as_u16(), 0x0100);
        assert_eq!(parsed.time_to_next, 0x1234_5678);
        assert!(parsed.cluster_max_tx_power.is_none());
        assert!(parsed.current_cluster_channel.is_none());
        assert_eq!(parsed.additional_channels.len(), 0);
    }

    #[test]
    fn network_beacon_parts_round_trip_full() {
        let additional_channels = Vec::from_slice(&[
            AbsoluteChannel::new(0x0200).unwrap(),
            AbsoluteChannel::new(0x0300).unwrap(),
            AbsoluteChannel::new(0x0400).unwrap(),
        ])
        .unwrap();
        let parts = NetworkBeaconParts {
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms1000,
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000,
            next_cluster_channel: AbsoluteChannel::new(0x0100).unwrap(),
            time_to_next: 0xDEAD_BEEF,
            cluster_max_tx_power: Some(TransmitPower::Dbm13),
            current_cluster_channel: Some(AbsoluteChannel::new(0x00FE).unwrap()),
            additional_channels,
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 17);

        let parsed = NetworkBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.power_const, PowerConst::Constrained);
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(parsed.cluster_beacon_period, ClusterBeaconPeriod::Ms1000);
        assert_eq!(parsed.next_cluster_channel.as_u16(), 0x0100);
        assert_eq!(parsed.time_to_next, 0xDEAD_BEEF);
        assert_eq!(parsed.cluster_max_tx_power, Some(TransmitPower::Dbm13));
        assert_eq!(parsed.current_cluster_channel.unwrap().as_u16(), 0x00FE);
        assert_eq!(parsed.additional_channels.len(), 3);
        assert_eq!(parsed.additional_channels[0].as_u16(), 0x0200);
        assert_eq!(parsed.additional_channels[1].as_u16(), 0x0300);
        assert_eq!(parsed.additional_channels[2].as_u16(), 0x0400);
    }
}
