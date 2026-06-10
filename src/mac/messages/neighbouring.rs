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
        let mut buf = [0u8; 32];
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
        let mut buf = [0u8; 32];
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
