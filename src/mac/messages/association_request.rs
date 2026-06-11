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
            harq_processes_tx: HarqProcesses::new(0).unwrap(),
            max_harq_re_tx: MaxHarqReTx::new(0).unwrap(),
            harq_processes_rx: HarqProcesses::new(0).unwrap(),
            max_harq_re_rx: MaxHarqReTx::new(0).unwrap(),
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
            harq_processes_tx: HarqProcesses::new(2).unwrap(),
            max_harq_re_tx: MaxHarqReTx::new(5).unwrap(),
            harq_processes_rx: HarqProcesses::new(4).unwrap(),
            max_harq_re_rx: MaxHarqReTx::new(7).unwrap(),
            ft_mode: Some(FtModeFields {
                network_beacon_period: NetworkBeaconPeriod::Ms1000,
                cluster_beacon_period: ClusterBeaconPeriod::Ms1500,
                next_cluster_channel: AbsoluteChannel::new(0x0123).unwrap(),
                time_to_next: 0xCAFEBABE,
                current_cluster_channel: Some(AbsoluteChannel::new(0x0124).unwrap()),
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
}
