//! Association Request message body (generated codec re-export).
//!
//! The codec lives in [`generated::association_request`](super::generated::association_request); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

use crate::types::*;

pub use super::generated::association_request::*;

/// FT-mode block of an Association Request (always 7 bytes; +2 if
/// `current_cluster_channel` is `Some`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct FtModeFields {
    /// Network beacon period code (4 bits, see Table 6.4.2.2-1).
    pub network_beacon_period: NetworkBeaconPeriod,
    /// Cluster beacon period code (4 bits, see Table 6.4.2.2-1).
    pub cluster_beacon_period: ClusterBeaconPeriod,
    pub next_cluster_channel: AbsoluteChannel,
    /// Time to next beacon period, microseconds.
    pub time_to_next: u32,
    pub current_cluster_channel: Option<AbsoluteChannel>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;
    fn ar_minimal() -> AssociationRequestParts {
        AssociationRequestParts {
            setup_cause: SetupCause::InitialAssociation,
            power_const: PowerConst::Unconstrained,
            flow_ids: Vec::new(),
            harq_processes_tx: HarqProcesses::try_from_u8(0).unwrap(),
            max_harq_re_tx: MaxHarqReTx::try_from_u8(0).unwrap(),
            harq_processes_rx: HarqProcesses::try_from_u8(0).unwrap(),
            max_harq_re_rx: MaxHarqReTx::try_from_u8(0).unwrap(),
            ft_mode: None,
        }
    }

    #[test]
    fn association_request_round_trip_minimal() {
        let parts = ar_minimal();
        let mut buf = [0; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, parts.encoded_len());
        assert_eq!(n, 4);

        let parsed = AssociationRequestParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.setup_cause, SetupCause::InitialAssociation);
        assert_eq!(parsed.power_const, PowerConst::Unconstrained);
        assert_eq!(parsed.flow_ids.len(), 0);
        assert!(parsed.ft_mode.is_none());
    }

    #[test]
    fn association_request_round_trip_with_flows_and_ft_mode() {
        let flow_ids = Vec::from_slice(&[
            FlowId::from_ie_type(IEType6bit::UserPlaneDataFlow1),
            FlowId::from_ie_type(IEType6bit::UserPlaneDataFlow2),
        ])
        .unwrap();
        let parts = AssociationRequestParts {
            setup_cause: SetupCause::Mobility,
            power_const: PowerConst::Constrained,
            flow_ids,
            harq_processes_tx: HarqProcesses::try_from_u8(2).unwrap(),
            max_harq_re_tx: MaxHarqReTx::try_from_u8(5).unwrap(),
            harq_processes_rx: HarqProcesses::try_from_u8(4).unwrap(),
            max_harq_re_rx: MaxHarqReTx::try_from_u8(7).unwrap(),
            ft_mode: Some(FtModeFields {
                network_beacon_period: NetworkBeaconPeriod::Ms1000,
                cluster_beacon_period: ClusterBeaconPeriod::Ms1500,
                next_cluster_channel: AbsoluteChannel::try_from_u16(0x0123).unwrap(),
                time_to_next: 0xCAFEBABE,
                current_cluster_channel: Some(AbsoluteChannel::try_from_u16(0x0124).unwrap()),
            }),
        };
        let mut buf = [0; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 4 (header) + 2 (flow IDs) + 7 (FT block) + 2 (Current) = 15
        assert_eq!(n, 15);

        let parsed = AssociationRequestParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.setup_cause, SetupCause::Mobility);
        assert_eq!(parsed.power_const, PowerConst::Constrained);
        assert_eq!(parsed.flow_ids.len(), 2);
        assert_eq!(parsed.flow_ids[0].as_u8(), parts.flow_ids[0].as_u8());
        assert_eq!(parsed.flow_ids[1].as_u8(), parts.flow_ids[1].as_u8());
        assert_eq!(parsed.harq_processes_tx.as_u8(), 2);
        assert_eq!(parsed.max_harq_re_tx.as_u8(), 5);
        assert_eq!(parsed.harq_processes_rx.as_u8(), 4);
        assert_eq!(parsed.max_harq_re_rx.as_u8(), 7);
        let ft = parsed.ft_mode.unwrap();
        assert_eq!(ft.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(ft.cluster_beacon_period, ClusterBeaconPeriod::Ms1500);
        assert_eq!(ft.next_cluster_channel.as_u16(), 0x0123);
        assert_eq!(ft.time_to_next, 0xCAFEBABE);
        assert_eq!(ft.current_cluster_channel.unwrap().as_u16(), 0x0124);
    }

    #[test]
    fn association_request_parser_rejects_reserved_setup_cause() {
        // SetupCause = 0b111 (Reserved). Other fields zero.
        let buf = [0b1110_0000, 0, 0, 0];
        assert!(AssociationRequestParts::parse(&buf).is_err());
    }

    #[test]
    fn association_request_parser_ignores_current_without_ft_mode() {
        // FT mode = 0 (B0 bit 0 = 0), Current = 1 (B1 bit 7 = 1). The
        // Current Cluster Channel field is absent without FT mode, so
        // the bit is ignored (receiver-ignores-reserved convention).
        let buf = [0, 0x80, 0, 0];
        let parts = AssociationRequestParts::parse(&buf).unwrap();
        assert!(parts.ft_mode.is_none());
    }

    #[test]
    #[expect(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn association_request_parser_ignores_reserved_flow_octet_bits() {
        // One flow with the two reserved top bits set: receiver ignores
        // them and reads the 6-bit flow ID.
        let buf = [0b000_001_0_0, 0, 0, 0, 0xC1];
        let parts = AssociationRequestParts::parse(&buf).unwrap();
        assert_eq!(parts.flow_ids.len(), 1);
        assert_eq!(parts.flow_ids[0].as_u8(), 1);
    }

    #[test]
    #[expect(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn association_request_parser_rejects_reserved_number_of_flows() {
        // Setup cause = 0 (valid), Number of Flows = 0b111 reserved.
        let buf = [
            0b000_111_00, // Setup cause=0, N flows=7, Power Const=0, FT=0
            0,
            0,
            0,
        ];
        assert!(AssociationRequestParts::parse(&buf).is_err());
    }

    /// Golden vector (minimal) hand-derived from Figure 6.4.2.4-1 and Tables 6.4.2.4-1/-2.
    ///
    /// No flows, no FT-mode block -> 4 bytes.
    ///
    /// Chosen values:
    ///   Setup Cause = Mobility (code 0b010, Table 6.4.2.4-2)
    ///   Power Const = Constrained (1)
    ///   Number of Flows = 0 (no flow IDs)
    ///   FT mode = absent (0)
    ///   Current = absent (0) -> byte 1 = 0x00
    ///   HARQ Processes TX = 3 (0b011)
    ///   MAX HARQ Re-TX = 5 (0b00101)
    ///   HARQ Processes RX = 4 (0b100)
    ///   MAX HARQ Re-RX = 7 (0b00111)
    ///
    /// Byte 0: [Setup Cause(3) | N Flows(3) | PC(1) | FT(1)]
    ///   = (0b010 << 5) | (0b000 << 2) | (1 << 1) | 0
    ///   = 0x40 | 0x00 | 0x02 | 0x00 = 0x42
    /// Byte 1: 0x00 (Current=0, reserved)
    /// Byte 2: [HARQ TX(3) | MAX Re-TX(5)] = (0b011 << 5) | 0b00101 = 0x65
    /// Byte 3: [HARQ RX(3) | MAX Re-RX(5)] = (0b100 << 5) | 0b00111 = 0x87
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 4] = [
            0b010_000_1_0, // Setup Cause=Mobility(010) | N Flows=0(000) | PC=Constrained(1) | FT=0
            0b0_0000000,   // Current=0 | Reserved
            0b011_00101,   // HARQ TX=3 | MAX Re-TX=5
            0b100_00111,   // HARQ RX=4 | MAX Re-RX=7
        ];
        let parts = AssociationRequestParts {
            setup_cause: SetupCause::Mobility,
            power_const: PowerConst::Constrained,
            harq_processes_tx: HarqProcesses::try_from_u8(3).unwrap(),
            max_harq_re_tx: MaxHarqReTx::try_from_u8(5).unwrap(),
            harq_processes_rx: HarqProcesses::try_from_u8(4).unwrap(),
            max_harq_re_rx: MaxHarqReTx::try_from_u8(7).unwrap(),
            flow_ids: Vec::new(),
            ft_mode: None,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(AssociationRequestParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.2.4-1 and Tables 6.4.2.4-1/-2.
    ///
    /// Maximum flows (6), FT-mode block with Current Cluster Channel present
    /// -> 4 + 6 + 7 + 2 = 19 bytes.
    ///
    /// Chosen values:
    ///   Setup Cause = ReassociationAfterError (code 0b011, Table 6.4.2.4-2)
    ///   Power Const = Unconstrained (0)
    ///   Number of Flows = 6 (MAX_REQUEST_FLOWS)
    ///   FT mode = present (1), Current = present (1)
    ///   HARQ TX=3, MAX Re-TX=5, HARQ RX=4, MAX Re-RX=7 (same as minimal)
    ///   Flow IDs: 0x01, 0x02, 0x03, 0x0A, 0x15, 0x20
    ///   Network Beacon Period = Ms2000 (code 5, Table 6.4.2.2-1)
    ///   Cluster Beacon Period = Ms4000 (code 7, Table 6.4.2.2-1)
    ///   Next Cluster Channel = AbsoluteChannel(0x1234)
    ///   Time To Next = 0xDEAD_BEEF
    ///   Current Cluster Channel = AbsoluteChannel(0x0567)
    ///
    /// Byte 0: (0b011 << 5) | (0b110 << 2) | (0 << 1) | 1 = 0x60 | 0x18 | 0 | 1 = 0x79
    /// Byte 1: Current=1, ft_mode present -> 0x80
    /// Byte 2: 0x65 (same HARQ TX)
    /// Byte 3: 0x87 (same HARQ RX)
    /// Bytes 4-9: flow IDs (low 6 bits each)
    /// Byte 10: NB Period(4)|CB Period(4) = (5<<4)|7 = 0x57
    /// Bytes 11-12: next_cluster_channel 0x1234 -> [0x12, 0x34]
    /// Bytes 13-16: time_to_next 0xDEAD_BEEF -> [0xDE, 0xAD, 0xBE, 0xEF]
    /// Bytes 17-18: current_cluster_channel 0x0567 -> [0x05, 0x67]
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 19] = [
            0b011_110_0_1, // Setup Cause=ReassocErr(011)|N Flows=6(110)|PC=Uncons(0)|FT=1
            0b1_0000000,   // Current=1 | Reserved (ft_mode + current_cluster_channel present)
            0b011_00101,   // HARQ TX=3 | MAX Re-TX=5
            0b100_00111,   // HARQ RX=4 | MAX Re-RX=7
            0x01,          // Flow ID 0 = 0x01
            0x02,          // Flow ID 1 = 0x02
            0x03,          // Flow ID 2 = 0x03
            0x0A,          // Flow ID 3 = 0x0A
            0x15,          // Flow ID 4 = 0x15
            0x20,          // Flow ID 5 = 0x20
            0b0101_0111,   // NB Period=Ms2000(5) | CB Period=Ms4000(7)
            0x12,          // Next Cluster Channel high byte (0x1234 & 0x1FFF)
            0x34,          // Next Cluster Channel low byte
            0xDE,          // Time To Next byte 0 (0xDEAD_BEEF)
            0xAD,          // Time To Next byte 1
            0xBE,          // Time To Next byte 2
            0xEF,          // Time To Next byte 3
            0x05,          // Current Cluster Channel high byte (0x0567 & 0x1FFF)
            0x67,          // Current Cluster Channel low byte
        ];
        let flow_ids = Vec::from_slice(&[
            FlowId::try_from_u8(0x01).unwrap(),
            FlowId::try_from_u8(0x02).unwrap(),
            FlowId::try_from_u8(0x03).unwrap(),
            FlowId::try_from_u8(0x0A).unwrap(),
            FlowId::try_from_u8(0x15).unwrap(),
            FlowId::try_from_u8(0x20).unwrap(),
        ])
        .unwrap();
        let parts = AssociationRequestParts {
            setup_cause: SetupCause::ReassociationAfterError,
            power_const: PowerConst::Unconstrained,
            harq_processes_tx: HarqProcesses::try_from_u8(3).unwrap(),
            max_harq_re_tx: MaxHarqReTx::try_from_u8(5).unwrap(),
            harq_processes_rx: HarqProcesses::try_from_u8(4).unwrap(),
            max_harq_re_rx: MaxHarqReTx::try_from_u8(7).unwrap(),
            flow_ids,
            ft_mode: Some(FtModeFields {
                network_beacon_period: NetworkBeaconPeriod::Ms2000,
                cluster_beacon_period: ClusterBeaconPeriod::Ms4000,
                next_cluster_channel: AbsoluteChannel::try_from_u16(0x1234).unwrap(),
                time_to_next: 0xDEAD_BEEF,
                current_cluster_channel: Some(AbsoluteChannel::try_from_u16(0x0567).unwrap()),
            }),
        };
        let mut buf = [0u8; 19];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(AssociationRequestParts::parse(&GOLDEN).unwrap(), parts);
    }
}
