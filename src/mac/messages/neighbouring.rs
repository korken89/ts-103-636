//! Neighbouring IE body (generated codec re-export).
//!
//! The codec lives in [`generated::neighbouring`](super::generated::neighbouring); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

use crate::types::*;

pub use super::generated::neighbouring::*;

/// Pair of Radio Device Class μ + β co-occurring in several IE bodies
/// (Neighbouring, RD Capability additional PHY block, etc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RadioDeviceClass {
    pub mu: RdClassMu,
    pub beta: RdClassBeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden vector (minimal) hand-derived from Figure 6.4.3.6-1 / Table 6.4.3.6-1.
    ///
    /// All optional fields absent.  Byte layout:
    ///
    ///   byte0:
    ///     bit 7: Reserved = 0
    ///     bit 6: ID (long_rd_id present) = 0
    ///     bit 5: RDC (radio_device_class present) = 0
    ///     bit 4: SNR (snr present) = 0
    ///     bit 3: R2 (rssi_2 present) = 0
    ///     bit 2: PC (power_const) = 0 (Unconstrained)
    ///     bit 1: NC (next_cluster_channel present) = 0
    ///     bit 0: TTN (time_to_next present) = 0
    ///     -> 0x00
    ///   byte1:
    ///     bits 7-4: NB Period = NetworkBeaconPeriod::Ms500 = code 2 = 0b0010
    ///     bits 3-0: CB Period = ClusterBeaconPeriod::Ms500 = code 3 = 0b0011
    ///     -> 0x23
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Neighbouring IE field layout"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 2] = [
            0b0_0_0_0_0_0_0_0, // R|ID|RDC|SNR|R2|PC|NC|TTN all zero
            0b0010_0011,       // NB period=Ms500(2) | CB period=Ms500(3)
        ];
        let parts = NeighbouringParts {
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms500, // code 2
            cluster_beacon_period: ClusterBeaconPeriod::Ms500, // code 3
            long_rd_id: None,
            next_cluster_channel: None,
            time_to_next: None,
            rssi_2: None,
            snr: None,
            radio_device_class: None,
        };
        let mut buf = [0u8; 32];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(NeighbouringParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.3.6-1 / Table 6.4.3.6-1.
    ///
    /// All optional fields present.  Byte layout:
    ///
    ///   byte0:
    ///     bit 7: Reserved = 0
    ///     bit 6: ID = 1  (long_rd_id present)
    ///     bit 5: RDC = 1 (radio_device_class present)
    ///     bit 4: SNR = 1 (snr present)
    ///     bit 3: R2 = 1  (rssi_2 present)
    ///     bit 2: PC = 1  (power_const = Constrained)
    ///     bit 1: NC = 1  (next_cluster_channel present)
    ///     bit 0: TTN = 1 (time_to_next present)
    ///     -> 0b0111_1111 = 0x7F
    ///   byte1:
    ///     NB Period = Ms2000 = code 5 = 0b0101
    ///     CB Period = Ms2000 = code 6 = 0b0110
    ///     -> 0b0101_0110 = 0x56
    ///   bytes 2-5: long_rd_id = 0xDEAD_CAFE -> [0xDE, 0xAD, 0xCA, 0xFE]
    ///   bytes 6-7: next_cluster_channel = 0x1234 (13-bit) -> [0x12, 0x34]
    ///   bytes 8-11: time_to_next = 0x0012_3456 -> [0x00, 0x12, 0x34, 0x56]
    ///   byte 12: rssi_2 = 0xA5
    ///   byte 13: snr = 0x5A
    ///   byte 14: radio_device_class: mu=M8(code 3) | beta=B4(code 2)
    ///     = ((3 & 0x07) << 5) | ((2 & 0x0F) << 1) = 0x60 | 0x04 = 0x64
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Neighbouring IE field layout"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 15] = [
            0b0_1_1_1_1_1_1_1, // R=0|ID|RDC|SNR|R2|PC|NC|TTN all set
            0b0101_0110,       // NB period=Ms2000(5) | CB period=Ms2000(6)
            0xDE,
            0xAD,
            0xCA,
            0xFE, // long_rd_id = 0xDEAD_CAFE
            0x12,
            0x34, // next_cluster_channel = 0x1234 (13-bit)
            0x00,
            0x12,
            0x34,
            0x56,         // time_to_next = 0x0012_3456 us
            0xA5,         // rssi_2
            0x5A,         // snr
            0b011_0010_0, // mu=M8(code 3, bits7-5) | beta=B4(code 2, bits4-1) | R=0
        ];
        let parts = NeighbouringParts {
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms2000, // code 5
            cluster_beacon_period: ClusterBeaconPeriod::Ms2000, // code 6
            long_rd_id: Some(LongRdId::new(0xDEAD_CAFE).unwrap()),
            next_cluster_channel: Some(AbsoluteChannel::new(0x1234).unwrap()),
            time_to_next: Some(0x0012_3456),
            rssi_2: Some(Rssi2Measurement(0xA5)),
            snr: Some(SnrMeasurement(0x5A)),
            radio_device_class: Some(RadioDeviceClass {
                mu: RdClassMu::M8,     // code 3 (Table 6.4.3.6-1)
                beta: RdClassBeta::B4, // code 2 (Table 6.4.3.6-1)
            }),
        };
        let mut buf = [0u8; 32];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(NeighbouringParts::parse(&GOLDEN).unwrap(), parts);
    }

    #[test]
    fn neighbouring_minimal_round_trip() {
        let parts = NeighbouringParts {
            network_beacon_period: NetworkBeaconPeriod::Ms1000,
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000,
            power_const: PowerConst::Unconstrained,
            long_rd_id: None,
            next_cluster_channel: None,
            time_to_next: None,
            rssi_2: None,
            snr: None,
            radio_device_class: None,
        };
        let mut buf = [0; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 2);
        let parsed = NeighbouringParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(parsed.cluster_beacon_period, ClusterBeaconPeriod::Ms1000);
        assert!(parsed.long_rd_id.is_none());
        assert!(parsed.next_cluster_channel.is_none());
        assert!(parsed.time_to_next.is_none());
        assert!(parsed.rssi_2.is_none());
        assert!(parsed.snr.is_none());
        assert!(parsed.radio_device_class.is_none());
    }

    #[test]
    fn neighbouring_full_round_trip() {
        let parts = NeighbouringParts {
            network_beacon_period: NetworkBeaconPeriod::Ms2000,
            cluster_beacon_period: ClusterBeaconPeriod::Ms2000,
            power_const: PowerConst::Constrained,
            long_rd_id: Some(LongRdId::new(0xDEADBEEF).unwrap()),
            next_cluster_channel: Some(AbsoluteChannel::new(0x1ABC).unwrap()),
            time_to_next: Some(0xDEAD_C0DE),
            rssi_2: Some(Rssi2Measurement(0x7F)),
            snr: Some(SnrMeasurement(0x12)),
            radio_device_class: Some(RadioDeviceClass {
                mu: RdClassMu::M4,
                beta: RdClassBeta::B12,
            }),
        };
        let mut buf = [0; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 2 + 4 + 2 + 4 + 1 + 1 + 1 = 15
        assert_eq!(n, 15);

        let parsed = NeighbouringParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.power_const, PowerConst::Constrained);
        assert_eq!(parsed.long_rd_id.unwrap().as_u32(), 0xDEADBEEF);
        assert_eq!(parsed.next_cluster_channel.unwrap().as_u16(), 0x1ABC);
        assert_eq!(parsed.time_to_next, Some(0xDEAD_C0DE));
        assert_eq!(parsed.rssi_2.unwrap().0, 0x7F);
        assert_eq!(parsed.snr.unwrap().0, 0x12);
        let rdc = parsed.radio_device_class.unwrap();
        assert_eq!(rdc.mu, RdClassMu::M4);
        assert_eq!(rdc.beta, RdClassBeta::B12);
    }

    #[test]
    fn neighbouring_parser_rejects_zero_long_rd_id() {
        // ID flag set with zero Long RD ID.
        // B0 = 0 1 0 0 0 0 0 0 = 0x40
        // B1 = 0
        // Long RD ID bytes all 0 -> reserved value of LongRdId
        let buf = [0x40, 0, 0, 0, 0, 0];
        assert!(NeighbouringParts::parse(&buf).is_err());
    }
}
