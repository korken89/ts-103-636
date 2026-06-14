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
            count_to_trigger: CountToTrigger::try_from_u8(0x3).unwrap(),
            rel_quality: Quality::try_from_u8(0).unwrap(),
            min_quality: Quality::try_from_u8(0).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: None,
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0; 16];
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
            count_to_trigger: CountToTrigger::try_from_u8(0x3).unwrap(),
            rel_quality: Quality::try_from_u8(0x2).unwrap(),
            min_quality: Quality::try_from_u8(0x1).unwrap(),
            cluster_max_tx_power: Some(TransmitPower::Dbm13),
            frame_offset: Some(0x55),
            next_cluster_channel: Some(AbsoluteChannel::try_from_u16(0x1234).unwrap()),
            time_to_next: Some(0x00001000),
        };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4 + 1 + 1 + 2 + 4);

        let again = ClusterBeaconParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(again.next_cluster_channel.unwrap().as_u16(), 0x1234);
        assert_eq!(again.time_to_next, Some(0x00001000));

        // Re-emit and compare bytes
        let mut buf2 = [0; 16];
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
        let mut out = [0; 16];
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
            count_to_trigger: CountToTrigger::try_from_u8(0x3).unwrap(),
            rel_quality: Quality::try_from_u8(0).unwrap(),
            min_quality: Quality::try_from_u8(0).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: Some(0x0123),
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn count_to_trigger_rejects_overflow() {
        assert!(CountToTrigger::try_from_u8(0x10).is_none());
    }

    /// Golden vector (minimal) hand-derived from Figure 6.4.2.3-1 and Table 6.4.2.3-1.
    ///
    /// Uses Mu::M1 (mu <= 4, Frame Offset would be 8-bit if present, but FO is absent here).
    /// All optional fields absent: cluster_max_tx_power=None, frame_offset=None,
    /// next_cluster_channel=None, time_to_next=None -> 4 bytes total.
    ///
    /// Chosen values:
    ///   SFN = 0xA5
    ///   TX Power (TXP flag) = 0 (cluster_max_tx_power absent)
    ///   Power Const (PC) = Constrained -> bit 3 = 1
    ///   FO = 0, NC = 0, TTN = 0
    ///   Network Beacon Period = Ms500 (code 2, Table 6.4.2.2-1)
    ///   Cluster Beacon Period = Ms1000 (code 4, Table 6.4.2.2-1)
    ///   Count To Trigger = 5 (code 0b0101, Table 6.4.2.3-1)
    ///   Rel Quality = 6 dB (code 2 = 0b10)
    ///   Min Quality = 3 dB (code 1 = 0b01)
    ///
    /// Byte 0: SFN = 0xA5
    /// Byte 1: [Reserved(3) | TXP(1) | PC(1) | FO(1) | NC(1) | TTN(1)]
    ///       = [000 | 0 | 1 | 0 | 0 | 0] = 0b00001000 = 0x08
    /// Byte 2: [NB Period(4) | CB Period(4)] = [0010 | 0100] = 0x24
    /// Byte 3: [CTT(4) | RelQ(2) | MinQ(2)] = [0101 | 10 | 01] = 0b01011001 = 0x59
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_minimal() {
        // Mu::M1: mu <= 4, so Frame Offset (if present) would be 8-bit.
        // Minimal vector omits all optional fields -> 4 bytes.
        const GOLDEN: [u8; 4] = [
            0xA5,            // SFN = 0xA5
            0b000_0_1_0_0_0, // Reserved | TXP=0 | PC=Constrained(1) | FO=0 | NC=0 | TTN=0
            0b0010_0100,     // NB Period=Ms500(2) | CB Period=Ms1000(4)
            0b0101_10_01,    // CTT=5 | RelQuality=6dB(2) | MinQuality=3dB(1)
        ];
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0xA5),
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms500,
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000,
            count_to_trigger: CountToTrigger::try_from_u8(5).unwrap(),
            rel_quality: Quality::try_from_u8(2).unwrap(),
            min_quality: Quality::try_from_u8(1).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: None,
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(ClusterBeaconParts::parse(&GOLDEN, Mu::M1).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.2.3-1 and Table 6.4.2.3-1.
    ///
    /// Uses Mu::M8 (mu > 4), so Frame Offset is 16-bit on the wire (Table 6.4.2.3-1).
    /// All optional fields present: cluster_max_tx_power, frame_offset (16-bit),
    /// next_cluster_channel, time_to_next -> 4+1+2+2+4 = 13 bytes.
    ///
    /// Chosen values:
    ///   SFN = 0xBE
    ///   cluster_max_tx_power = Dbm10 (code 10 = 0b1010, Table 6.2.1-3a)
    ///   frame_offset = 0x1234 (16-bit, mu=8 > 4)
    ///   next_cluster_channel = AbsoluteChannel(0x1A00) (13-bit, within 1..=0x1FFF)
    ///   time_to_next = 0xCAFE_BABE
    ///   (same NB/CB period, CTT, quality as minimal)
    ///
    /// Byte 0: SFN = 0xBE
    /// Byte 1: [Reserved(3) | TXP(1) | PC(1) | FO(1) | NC(1) | TTN(1)]
    ///       = [000 | 1 | 1 | 1 | 1 | 1] = 0b00011111 = 0x1F
    /// Byte 2: [NB Period(4) | CB Period(4)] = [0010 | 0100] = 0x24
    /// Byte 3: [CTT(4) | RelQ(2) | MinQ(2)] = [0101 | 10 | 01] = 0x59
    /// Byte 4: cluster_max_tx_power: low nibble = 0b0000_1010 = 0x0A
    /// Bytes 5-6: frame_offset 0x1234 big-endian = 0x12, 0x34
    /// Bytes 7-8: next_cluster_channel 0x1A00 & 0x1FFF = 0x1A00 -> [0x1A, 0x00]
    /// Bytes 9-12: time_to_next 0xCAFE_BABE -> [0xCA, 0xFE, 0xBA, 0xBE]
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_full() {
        // Mu::M8: mu > 4, so Frame Offset is 16-bit on the wire.
        const GOLDEN: [u8; 13] = [
            0xBE,            // SFN = 0xBE
            0b000_1_1_1_1_1, // Reserved | TXP=1 | PC=Constrained(1) | FO=1 | NC=1 | TTN=1
            0b0010_0100,     // NB Period=Ms500(2) | CB Period=Ms1000(4)
            0b0101_10_01,    // CTT=5 | RelQuality=6dB(2) | MinQuality=3dB(1)
            0b0000_1010,     // cluster_max_tx_power = Dbm10 (code 10, low nibble)
            0x12,            // frame_offset high byte (16-bit for mu>4)
            0x34,            // frame_offset low byte
            0x1A,            // next_cluster_channel high byte (0x1A00 & 0x1FFF)
            0x00,            // next_cluster_channel low byte
            0xCA,            // time_to_next byte 0 (0xCAFEBABE)
            0xFE,            // time_to_next byte 1
            0xBA,            // time_to_next byte 2
            0xBE,            // time_to_next byte 3
        ];
        let parts = ClusterBeaconParts {
            mu: Mu::M8,
            sfn: Sfn(0xBE),
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms500,
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000,
            count_to_trigger: CountToTrigger::try_from_u8(5).unwrap(),
            rel_quality: Quality::try_from_u8(2).unwrap(),
            min_quality: Quality::try_from_u8(1).unwrap(),
            cluster_max_tx_power: Some(TransmitPower::Dbm10),
            frame_offset: Some(0x1234),
            next_cluster_channel: Some(AbsoluteChannel::try_from_u16(0x1A00).unwrap()),
            time_to_next: Some(0xCAFE_BABE),
        };
        let mut buf = [0u8; 13];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(ClusterBeaconParts::parse(&GOLDEN, Mu::M8).unwrap(), parts);
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
                count_to_trigger: CountToTrigger::try_from_u8(3).unwrap(),
                rel_quality: Quality::try_from_u8(0).unwrap(),
                min_quality: Quality::try_from_u8(0).unwrap(),
                cluster_max_tx_power: None,
                frame_offset: None,
                next_cluster_channel: None,
                time_to_next: None,
            };
            let mut buf = [0; 4];
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
