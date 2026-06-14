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

    // -----------------------------------------------------------------------
    // Golden vectors - hand-derived from ETSI TS 103 636-4 clause 6.4.2.7
    // Figure 6.4.2.7-1.
    //
    // B0 layout (MSB first): TX(b7)|RX(b6)|RDC(b5)|N[2:0](b4..b2)|RR[1:0](b1..b0)
    // HARQ byte: HARQ_Proc[2:0](b7..b5) | MAX_HARQ_Re[4:0](b4..b0)
    // FlowEntry byte: Setup/Release(b7) | Reserved(b6) | FlowID[5:0](b5..b0)
    // -----------------------------------------------------------------------

    /// Clause 6.4.2.7, minimal: no TX/RX HARQ, 0 flows, RD cap = 0,
    /// Radio Resource = No Change. Single byte, all zeros.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows reconfiguration-request field layout"
    )]
    fn golden_vector_minimal_reconfiguration_request() {
        const GOLDEN: [u8; 1] = [
            0b0_0_0_000_00, // TX=0 RX=0 RDC=0 N=000 RR=NoChange(00)
        ];
        let parts = ReconfigurationRequestParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flows: &[],
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(ReconfigurationRequestParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Clause 6.4.2.7, full: TX+RX HARQ both set, RD cap set, 6 flow
    /// entries, Radio Resource = ResourceAllocationIeIncluded.
    ///
    /// B0 = TX(1)|RX(1)|RDC(1)|N=110|RR=11 = 0b11111011 = 0xFB
    /// TX HARQ: processes=5 (0b101), max_re=13 (0b01101) -> (5<<5)|13 = 0b10101101 = 0xAD
    /// RX HARQ: processes=2 (0b010), max_re=7  (0b00111) -> (2<<5)|7  = 0b01000111 = 0x47
    /// FlowEntry byte: S/R(b7)|Rsv(b6)|FlowID[5:0](b5..b0)
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows reconfiguration-request field layout"
    )]
    fn golden_vector_full_reconfiguration_request() {
        const GOLDEN: [u8; 9] = [
            0b1_1_1_110_11, // TX=1 RX=1 RDC=1 N=110(6) RR=ResourceAllocationIeIncluded(11)
            0b101_01101,    // TX HARQ: processes=5(101) max_re=13(01101)
            0b010_00111,    // RX HARQ: processes=2(010) max_re=7(00111)
            0b0_0_001010,   // flow[0]: SetupOrReconfigure(0) rsv=0 FlowID=0x0A
            0b1_0_010100,   // flow[1]: Release(1) rsv=0 FlowID=0x14
            0b0_0_000001,   // flow[2]: SetupOrReconfigure(0) rsv=0 FlowID=0x01
            0b1_0_000010,   // flow[3]: Release(1) rsv=0 FlowID=0x02
            0b0_0_000011,   // flow[4]: SetupOrReconfigure(0) rsv=0 FlowID=0x03
            0b1_0_000101,   // flow[5]: Release(1) rsv=0 FlowID=0x05
        ];
        let flows = [
            FlowEntry::new(
                FlowAction::SetupOrReconfigure,
                FlowId::try_from_u8(0x0A).unwrap(),
            ),
            FlowEntry::new(FlowAction::Release, FlowId::try_from_u8(0x14).unwrap()),
            FlowEntry::new(
                FlowAction::SetupOrReconfigure,
                FlowId::try_from_u8(0x01).unwrap(),
            ),
            FlowEntry::new(FlowAction::Release, FlowId::try_from_u8(0x02).unwrap()),
            FlowEntry::new(
                FlowAction::SetupOrReconfigure,
                FlowId::try_from_u8(0x03).unwrap(),
            ),
            FlowEntry::new(FlowAction::Release, FlowId::try_from_u8(0x05).unwrap()),
        ];
        let parts = ReconfigurationRequestParts {
            tx_harq: Some(HarqConfig {
                processes: HarqProcesses::try_from_u8(5).unwrap(),
                max_re: MaxHarqReTx::try_from_u8(13).unwrap(),
            }),
            rx_harq: Some(HarqConfig {
                processes: HarqProcesses::try_from_u8(2).unwrap(),
                max_re: MaxHarqReTx::try_from_u8(7).unwrap(),
            }),
            rd_capability_changed: true,
            radio_resource: RadioResourceChange::ResourceAllocationIeIncluded,
            flows: &flows,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(ReconfigurationRequestParts::parse(&GOLDEN).unwrap(), parts);
    }

    // -----------------------------------------------------------------------
    // Golden vectors - hand-derived from ETSI TS 103 636-4 clause 6.4.2.8
    // Figure 6.4.2.8-1.
    //
    // B0 layout identical to Request. N field meaning differs: N=111 means
    // "All flows accepted" (no flow bytes follow); N=0..=6 means that many
    // specific flow-acceptance entries.
    // -----------------------------------------------------------------------

    /// Clause 6.4.2.8, minimal: Specific(&[]) acceptance (N=000), no HARQ,
    /// no RD cap, Radio Resource = No Change. Single byte, all zeros.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows reconfiguration-response field layout"
    )]
    fn golden_vector_minimal_reconfiguration_response() {
        const GOLDEN: [u8; 1] = [
            0b0_0_0_000_00, // TX=0 RX=0 RDC=0 N=000(Specific 0) RR=NoChange(00)
        ];
        let parts = ReconfigurationResponseParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flow_acceptance: FlowChangeAcceptance::Specific(&[]),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = ReconfigurationResponseParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed.tx_harq, parts.tx_harq);
        assert_eq!(parsed.rx_harq, parts.rx_harq);
        assert_eq!(parsed.rd_capability_changed, parts.rd_capability_changed);
        assert_eq!(parsed.radio_resource, parts.radio_resource);
        assert!(
            matches!(parsed.flow_acceptance, FlowChangeAcceptance::Specific(s) if s.is_empty())
        );
    }

    /// Clause 6.4.2.8, full: TX+RX HARQ rejected, RD cap set, 6 specific
    /// flow entries accepted, Radio Resource = ResourceAllocationIeIncluded.
    ///
    /// B0 identical to the Request full vector (0xFB): TX/RX/RDC flags all
    /// set, N=110(6 flows), RR=11.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows reconfiguration-response field layout"
    )]
    fn golden_vector_full_reconfiguration_response() {
        const GOLDEN: [u8; 9] = [
            0b1_1_1_110_11, // TX=1 RX=1 RDC=1 N=110(6 flows) RR=ResourceAllocationIeIncluded(11)
            0b101_01101,    // TX HARQ: processes=5(101) max_re=13(01101) - alternative config
            0b010_00111,    // RX HARQ: processes=2(010) max_re=7(00111)
            0b0_0_001010,   // flow[0]: SetupOrReconfigure(0) rsv=0 FlowID=0x0A
            0b1_0_010100,   // flow[1]: Release(1) rsv=0 FlowID=0x14
            0b0_0_000001,   // flow[2]: SetupOrReconfigure(0) rsv=0 FlowID=0x01
            0b1_0_000010,   // flow[3]: Release(1) rsv=0 FlowID=0x02
            0b0_0_000011,   // flow[4]: SetupOrReconfigure(0) rsv=0 FlowID=0x03
            0b1_0_000101,   // flow[5]: Release(1) rsv=0 FlowID=0x05
        ];
        let flows = [
            FlowEntry::new(
                FlowAction::SetupOrReconfigure,
                FlowId::try_from_u8(0x0A).unwrap(),
            ),
            FlowEntry::new(FlowAction::Release, FlowId::try_from_u8(0x14).unwrap()),
            FlowEntry::new(
                FlowAction::SetupOrReconfigure,
                FlowId::try_from_u8(0x01).unwrap(),
            ),
            FlowEntry::new(FlowAction::Release, FlowId::try_from_u8(0x02).unwrap()),
            FlowEntry::new(
                FlowAction::SetupOrReconfigure,
                FlowId::try_from_u8(0x03).unwrap(),
            ),
            FlowEntry::new(FlowAction::Release, FlowId::try_from_u8(0x05).unwrap()),
        ];
        let parts = ReconfigurationResponseParts {
            tx_harq: Some(HarqConfig {
                processes: HarqProcesses::try_from_u8(5).unwrap(),
                max_re: MaxHarqReTx::try_from_u8(13).unwrap(),
            }),
            rx_harq: Some(HarqConfig {
                processes: HarqProcesses::try_from_u8(2).unwrap(),
                max_re: MaxHarqReTx::try_from_u8(7).unwrap(),
            }),
            rd_capability_changed: true,
            radio_resource: RadioResourceChange::ResourceAllocationIeIncluded,
            flow_acceptance: FlowChangeAcceptance::Specific(&flows),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = ReconfigurationResponseParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed.tx_harq, parts.tx_harq);
        assert_eq!(parsed.rx_harq, parts.rx_harq);
        assert_eq!(parsed.rd_capability_changed, parts.rd_capability_changed);
        assert_eq!(parsed.radio_resource, parts.radio_resource);
        let accepted = match parsed.flow_acceptance {
            FlowChangeAcceptance::Specific(s) => s,
            _ => panic!("expected Specific"),
        };
        assert_eq!(accepted.len(), 6);
        for (got, want) in accepted.iter().zip(flows.iter()) {
            assert_eq!(got.as_raw(), want.as_raw());
        }
    }

    fn fe(action: FlowAction, raw: u8) -> FlowEntry {
        FlowEntry::new(action, FlowId::try_from_u8(raw).unwrap())
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
        let mut buf = [0; 16];
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
                processes: HarqProcesses::try_from_u8(3).unwrap(),
                max_re: MaxHarqReTx::try_from_u8(7).unwrap(),
            }),
            rx_harq: Some(HarqConfig {
                processes: HarqProcesses::try_from_u8(2).unwrap(),
                max_re: MaxHarqReTx::try_from_u8(5).unwrap(),
            }),
            rd_capability_changed: true,
            radio_resource: RadioResourceChange::ResourceAllocationIeIncluded,
            flows: &flows,
        };
        let mut buf = [0; 16];
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
        let mut buf = [0; 16];
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
        let mut buf = [0; 16];
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
        let mut buf = [0; 16];
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
