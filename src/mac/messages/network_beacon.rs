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

    /// Golden vector (minimal) hand-derived from Figure 6.4.2.2-1 / Table 6.4.2.2-1.
    ///
    /// No optional fields; zero additional channels.  Byte layout:
    ///
    ///   byte0:
    ///     bits 7-5: Reserved = 0b000
    ///     bit 4: TXP (cluster_max_tx_power present) = 0
    ///     bit 3: PC (power_const = Constrained?) = 0 (Unconstrained)
    ///     bit 2: CC (current_cluster_channel present) = 0
    ///     bits 1-0: N (additional_channels count) = 0b00
    ///     -> 0x00
    ///   byte1:
    ///     bits 7-4: NB period = Ms1000 = code 3 = 0b0011
    ///     bits 3-0: CB period = Ms1000 = code 4 = 0b0100
    ///     -> 0b0011_0100 = 0x34
    ///   bytes 2-3: next_cluster_channel = 0x0555 (13-bit) -> [0x05, 0x55]
    ///   bytes 4-7: time_to_next = 0xABCD_1234 -> [0xAB, 0xCD, 0x12, 0x34]
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Network Beacon field layout"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 8] = [
            0b000_0_0_0_00, // reserved(3b) | TXP=0 | PC=0 | CC=0 | N=0
            0b0011_0100,    // NB period=Ms1000(3) | CB period=Ms1000(4)
            0x05,
            0x55, // next_cluster_channel = 0x0555
            0xAB,
            0xCD,
            0x12,
            0x34, // time_to_next = 0xABCD_1234 us
        ];
        let parts = NetworkBeaconParts {
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms1000, // code 3
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000, // code 4
            next_cluster_channel: AbsoluteChannel::new(0x0555).unwrap(),
            time_to_next: 0xABCD_1234,
            cluster_max_tx_power: None,
            current_cluster_channel: None,
            additional_channels: Vec::new(),
        };
        let mut buf = [0u8; 32];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(NetworkBeaconParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.2.2-1 / Table 6.4.2.2-1.
    ///
    /// All optional fields present; N = MAX_ADDITIONAL_CHANNELS = 3.  Byte layout:
    ///
    ///   byte0:
    ///     bits 7-5: Reserved = 0b000
    ///     bit 4: TXP = 1 (cluster_max_tx_power present)
    ///     bit 3: PC = 1  (power_const = Constrained)
    ///     bit 2: CC = 1  (current_cluster_channel present)
    ///     bits 1-0: N = 0b11 = 3 additional channels
    ///     -> 0b000_1_1_1_11 = 0x1F
    ///   byte1:
    ///     bits 7-4: NB period = Ms4000 = code 6 = 0b0110
    ///     bits 3-0: CB period = Ms4000 = code 7 = 0b0111
    ///     -> 0b0110_0111 = 0x67
    ///   bytes 2-3: next_cluster_channel = 0x1ABC -> [0x1A, 0xBC]
    ///   bytes 4-7: time_to_next = 0xDEAD_BEEF -> [0xDE, 0xAD, 0xBE, 0xEF]
    ///   byte 8: cluster_max_tx_power = Dbm13 = code 0b1011 = 11 -> 0x0B
    ///   bytes 9-10: current_cluster_channel = 0x0123 -> [0x01, 0x23]
    ///   bytes 11-12: additional_channel[0] = 0x0234 -> [0x02, 0x34]
    ///   bytes 13-14: additional_channel[1] = 0x0345 -> [0x03, 0x45]
    ///   bytes 15-16: additional_channel[2] = 0x0456 -> [0x04, 0x56]
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Network Beacon field layout"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 17] = [
            0b000_1_1_1_11, // reserved(3b) | TXP=1 | PC=1 | CC=1 | N=3
            0b0110_0111,    // NB period=Ms4000(6) | CB period=Ms4000(7)
            0x1A,
            0xBC, // next_cluster_channel = 0x1ABC
            0xDE,
            0xAD,
            0xBE,
            0xEF, // time_to_next = 0xDEAD_BEEF us
            0x0B, // cluster_max_tx_power = Dbm13 (code 11 = 0b1011)
            0x01,
            0x23, // current_cluster_channel = 0x0123
            0x02,
            0x34, // additional_channel[0] = 0x0234
            0x03,
            0x45, // additional_channel[1] = 0x0345
            0x04,
            0x56, // additional_channel[2] = 0x0456
        ];
        let parts = NetworkBeaconParts {
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms4000, // code 6
            cluster_beacon_period: ClusterBeaconPeriod::Ms4000, // code 7
            next_cluster_channel: AbsoluteChannel::new(0x1ABC).unwrap(),
            time_to_next: 0xDEAD_BEEF,
            cluster_max_tx_power: Some(TransmitPower::Dbm13), // code 0b1011 = 11
            current_cluster_channel: Some(AbsoluteChannel::new(0x0123).unwrap()),
            additional_channels: Vec::from_slice(&[
                AbsoluteChannel::new(0x0234).unwrap(),
                AbsoluteChannel::new(0x0345).unwrap(),
                AbsoluteChannel::new(0x0456).unwrap(),
            ])
            .unwrap(),
        };
        let mut buf = [0u8; 32];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(NetworkBeaconParts::parse(&GOLDEN).unwrap(), parts);
    }

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
        let mut buf = [0; 32];
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
        let mut buf = [0; 32];
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
