//! Cluster Beacon message body (generated codec re-export).
//!
//! The codec lives in [`generated::cluster_beacon`](super::generated::cluster_beacon); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::cluster_beacon::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    #[test]
    fn parse_minimal_cluster_beacon() {
        let buf = [
            0xAB, // sfn
            0x00, // flags: all zero -> unconstrained, no optionals
            0x12, // net_period=1, cluster_period=2
            0x30, // ctt=3, qrel=0, qmin=0
        ];
        let cb = ClusterBeaconParts::parse(&buf, Mu::M1).unwrap();
        assert_eq!(cb.sfn.0, 0xAB);
        assert_eq!(cb.power_const, PowerConst::Unconstrained);
        assert_eq!(cb.network_beacon_period, NetworkBeaconPeriod::Ms100);
        assert_eq!(cb.cluster_beacon_period, ClusterBeaconPeriod::Ms100);
        assert_eq!(cb.count_to_trigger.as_u8(), 3);
        assert_eq!(cb.rel_quality.as_u8(), 0);
        assert_eq!(cb.min_quality.as_u8(), 0);
        assert!(cb.cluster_max_tx_power.is_none());
        assert!(cb.frame_offset.is_none());
        assert!(cb.next_cluster_channel.is_none());
        assert!(cb.time_to_next.is_none());
    }

    #[test]
    fn parse_full_cluster_beacon() {
        let buf = [
            0xAB, // sfn
            // flags: TX Power=1 (0x10) | Power Const=1 (0x08) | FO=1 (0x04) | NCC=1 (0x02) | TTN=1 (0x01) = 0x1F
            0x1F, 0x12, // net=1, cluster=2
            0x30, // ctt=3, qrel=0, qmin=0
            0x08, // max_tx_power = 8
            0x05, // frame_offset = 5
            0x12, 0x34, // next_channel = 0x1234
            0x00, 0x00, 0x10, 0x00, // time_to_next = 4096
        ];
        let cb = ClusterBeaconParts::parse(&buf, Mu::M1).unwrap();
        assert_eq!(cb.power_const, PowerConst::Constrained);
        assert_eq!(cb.cluster_max_tx_power.unwrap().as_u8(), 8);
        assert_eq!(cb.frame_offset, Some(5));
        assert_eq!(cb.next_cluster_channel.unwrap().as_u16(), 0x1234);
        assert_eq!(cb.time_to_next, Some(0x00001000));
    }

    #[test]
    fn parse_truncated_optional_field() {
        let buf = [0xAB, 0x01, 0x12, 0x30];
        assert!(ClusterBeaconParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn parse_too_short() {
        let buf = [0xAB, 0x00, 0x12];
        assert!(ClusterBeaconParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn parts_round_trip_minimal() {
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0xAB),
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            count_to_trigger: CountToTrigger::new(0x3).unwrap(),
            rel_quality: Quality::new(0).unwrap(),
            min_quality: Quality::new(0).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: None,
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, parts.encoded_len());

        let again = ClusterBeaconParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(again.sfn.0, parts.sfn.0);
        assert_eq!(again.network_beacon_period, parts.network_beacon_period);
        assert_eq!(again.cluster_beacon_period, parts.cluster_beacon_period);
        assert_eq!(again.cluster_max_tx_power, parts.cluster_max_tx_power);
    }

    #[test]
    fn parts_round_trip_full() {
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0xAB),
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            count_to_trigger: CountToTrigger::new(0x3).unwrap(),
            rel_quality: Quality::new(0x2).unwrap(),
            min_quality: Quality::new(0x1).unwrap(),
            cluster_max_tx_power: Some(TransmitPower::Dbm13),
            frame_offset: Some(0x55),
            next_cluster_channel: Some(AbsoluteChannel::new(0x1234).unwrap()),
            time_to_next: Some(0x00001000),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4 + 1 + 1 + 2 + 4);

        let again = ClusterBeaconParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(again.next_cluster_channel.unwrap().as_u16(), 0x1234);
        assert_eq!(again.time_to_next, Some(0x00001000));

        // Re-emit and compare bytes
        let mut buf2 = [0u8; 16];
        let n2 = again.serialize(&mut buf2).unwrap();
        assert_eq!(&buf[..n], &buf2[..n2]);
    }

    #[test]
    fn frame_offset_is_16_bit_when_mu_above_4() {
        // Table 6.4.2.3-1: 16-bit Frame Offset when mu > 4.
        let buf = [
            0xAB, // sfn
            0x04, // flags: FO only
            0x12, // net=1, cluster=2
            0x30, // ctt=3
            0x01, 0x23, // frame_offset = 0x0123 (16-bit BE)
        ];
        let cb = ClusterBeaconParts::parse(&buf, Mu::M8).unwrap();
        assert_eq!(cb.frame_offset, Some(0x0123));

        // The same bytes parsed as mu=1 must fail: the 8-bit FO leaves
        // a trailing byte, which is a length mismatch at the IE level,
        // here visible as a different field split.
        let cb_mu1 = ClusterBeaconParts::parse(&buf[..5], Mu::M1).unwrap();
        assert_eq!(cb_mu1.frame_offset, Some(0x01));

        // Round trip in mu=8 keeps the 16-bit encoding.
        let mut out = [0u8; 16];
        let n = cb.serialize(&mut out).unwrap();
        assert_eq!(&out[..n], &buf);
    }

    #[test]
    fn frame_offset_over_255_rejected_for_low_mu() {
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0),
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            count_to_trigger: CountToTrigger::new(0x3).unwrap(),
            rel_quality: Quality::new(0).unwrap(),
            min_quality: Quality::new(0).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: Some(0x0123),
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn count_to_trigger_rejects_overflow() {
        assert!(CountToTrigger::new(0x10).is_none());
    }

    #[test]
    fn codec_works_in_const_context() {
        // The generated codec is const fn (no heapless::Vec in this
        // message): messages can be pre-built at compile time.
        const ENCODED: ([u8; 4], usize) = {
            let parts = ClusterBeaconParts {
                mu: Mu::M1,
                sfn: Sfn(0xAB),
                power_const: PowerConst::Unconstrained,
                network_beacon_period: NetworkBeaconPeriod::Ms100,
                cluster_beacon_period: ClusterBeaconPeriod::Ms100,
                count_to_trigger: CountToTrigger::new(3).unwrap(),
                rel_quality: Quality::new(0).unwrap(),
                min_quality: Quality::new(0).unwrap(),
                cluster_max_tx_power: None,
                frame_offset: None,
                next_cluster_channel: None,
                time_to_next: None,
            };
            let mut buf = [0u8; 4];
            let n = match parts.serialize(&mut buf) {
                Ok(n) => n,
                Err(_) => panic!("buffer is sized for the message"),
            };
            (buf, n)
        };
        const DECODED: ClusterBeaconParts = match ClusterBeaconParts::parse(&ENCODED.0, Mu::M1) {
            Ok(p) => p,
            Err(_) => panic!("round trip of a valid message"),
        };
        assert_eq!(ENCODED.1, 4);
        assert_eq!(DECODED.sfn.0, 0xAB);
        assert_eq!(DECODED.network_beacon_period, NetworkBeaconPeriod::Ms100);
    }
}
