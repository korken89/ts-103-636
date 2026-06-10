//! Reconfiguration Request / Response message bodies (generated
//! codec re-exports).
//!
//! The codecs live in
//! [`generated::reconfiguration_request`](super::generated::reconfiguration_request) and
//! [`generated::reconfiguration_response`](super::generated::reconfiguration_response);
//! the layout figures are in those modules' documentation. The tests
//! below are the drop-in equivalence oracle and predate the generated
//! codecs.

use crate::types::*;

pub use super::generated::reconfiguration_request::*;
pub use super::generated::reconfiguration_response::*;

/// HARQ configuration block carried in Reconfiguration messages.
/// Identical wire layout (3-bit processes + 5-bit max re-tx/re-rx) is
/// shared between the TX and RX positions; the field's direction is
/// determined by where the [`HarqConfig`] sits in the parent struct
/// (`tx_harq` vs `rx_harq`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct HarqConfig {
    pub processes: HarqProcesses,
    pub max_re: MaxHarqReTx,
}

/// How many of the requested flow changes the FT is acknowledging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FlowChangeAcceptance<'a> {
    /// All flow changes accepted as configured in the Reconfiguration
    /// Request (on-wire Number of Flows = 0b111, no flow octets).
    All,
    /// Specific (0..=6) flow changes accepted. Empty slice corresponds
    /// to on-wire Number of Flows = 0.
    Specific(&'a [FlowEntry]),
}

#[cfg(test)]
mod tests {
    use super::*;
    // FlowAction is already imported at the top of the test module.

    fn fe(action: FlowAction, raw: u8) -> FlowEntry {
        FlowEntry::new(action, FlowId::new(raw).unwrap())
    }

    #[test]
    fn reconfiguration_request_minimal_round_trip() {
        let parts = ReconfigurationRequestParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flows: &[],
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0);

        let parsed = ReconfigurationRequestParts::parse(&buf[..n]).unwrap();
        assert!(parsed.tx_harq.is_none());
        assert!(parsed.rx_harq.is_none());
        assert!(!parsed.rd_capability_changed);
        assert_eq!(parsed.radio_resource, RadioResourceChange::NoChange);
        assert_eq!(parsed.flows.len(), 0);
    }

    #[test]
    fn reconfiguration_request_full_round_trip() {
        let flows = [
            fe(FlowAction::SetupOrReconfigure, 0b000011),
            fe(FlowAction::Release, 0b000100),
        ];
        let parts = ReconfigurationRequestParts {
            tx_harq: Some(HarqConfig {
                processes: HarqProcesses::new(3).unwrap(),
                max_re: MaxHarqReTx::new(7).unwrap(),
            }),
            rx_harq: Some(HarqConfig {
                processes: HarqProcesses::new(2).unwrap(),
                max_re: MaxHarqReTx::new(5).unwrap(),
            }),
            rd_capability_changed: true,
            radio_resource: RadioResourceChange::ResourceAllocationIeIncluded,
            flows: &flows,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (B0) + 2 (TX, RX HARQ) + 2 (flow entries) = 5
        assert_eq!(n, 5);
        // B0 = TX(1)|RX(1)|RDcap(1)|n_flows(010)|RR(11) = 0b1110_1011
        assert_eq!(buf[0], 0b1110_1011);

        let parsed = ReconfigurationRequestParts::parse(&buf[..n]).unwrap();
        let tx = parsed.tx_harq.unwrap();
        assert_eq!(tx.processes.as_u8(), 3);
        assert_eq!(tx.max_re.as_u8(), 7);
        let rx = parsed.rx_harq.unwrap();
        assert_eq!(rx.processes.as_u8(), 2);
        assert_eq!(rx.max_re.as_u8(), 5);
        assert!(parsed.rd_capability_changed);
        assert_eq!(
            parsed.radio_resource,
            RadioResourceChange::ResourceAllocationIeIncluded
        );
        assert_eq!(parsed.flows.len(), 2);
        assert_eq!(parsed.flows[0].action(), FlowAction::SetupOrReconfigure);
        assert_eq!(parsed.flows[0].flow_id().as_u8(), 0b000011);
        assert_eq!(parsed.flows[1].action(), FlowAction::Release);
        assert_eq!(parsed.flows[1].flow_id().as_u8(), 0b000100);
    }

    #[test]
    fn reconfiguration_request_serialize_rejects_reserved_n_flows() {
        let flows = [fe(FlowAction::SetupOrReconfigure, 0); 7];
        let parts = ReconfigurationRequestParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flows: &flows,
        };
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn reconfiguration_request_parser_rejects_reserved_n_flows() {
        // n_flows = 7 in B0 (bits 4..=2 = 0b111).
        let buf = [0b0001_1100];
        assert!(ReconfigurationRequestParts::parse(&buf).is_err());
    }

    #[test]
    fn reconfiguration_response_accept_all_round_trip() {
        let parts = ReconfigurationResponseParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flow_acceptance: FlowChangeAcceptance::All,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        // n_flows = 0b111 encoded into bits 4..=2 = 0b00011100.
        assert_eq!(buf[0], 0b0001_1100);

        let parsed = ReconfigurationResponseParts::parse(&buf[..n]).unwrap();
        assert!(matches!(parsed.flow_acceptance, FlowChangeAcceptance::All));
    }

    #[test]
    fn reconfiguration_response_specific_flows_round_trip() {
        let flows = [fe(FlowAction::Release, 0b000010)];
        let parts = ReconfigurationResponseParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flow_acceptance: FlowChangeAcceptance::Specific(&flows),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 2);
        let parsed = ReconfigurationResponseParts::parse(&buf[..n]).unwrap();
        let got = match parsed.flow_acceptance {
            FlowChangeAcceptance::Specific(f) => f,
            _ => panic!("expected specific"),
        };
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].action(), FlowAction::Release);
        assert_eq!(got[0].flow_id().as_u8(), 0b000010);
    }
}
