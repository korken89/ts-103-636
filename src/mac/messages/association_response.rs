//! Association Response message body (generated codec re-export).
//!
//! The codec lives in [`generated::association_response`](super::generated::association_response); the layout
//! figure is in that module's documentation. The enum and its payload
//! types stay here; the generated module implements their codec. The
//! tests below are the drop-in equivalence oracle and predate the
//! generated codec.

use heapless::Vec;

use crate::types::*;

pub use super::generated::association_response::*;

/// Owned representation of an Association Response body. The Reject vs.
/// Accept split is modeled at the enum level so a reject can never
/// accidentally carry flow / HARQ / group fields, and vice-versa.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub enum AssociationResponseParts {
    /// Association rejected. On the wire: ACK/NACK bit = 0; the entire
    /// remainder of byte 0 is don't-care for the receiver.
    Reject {
        cause: RejectCause,
        timer: RejectTimer,
    },
    /// Association accepted. On the wire: ACK/NACK bit = 1.
    Accept(AssociationAcceptParts),
}

/// Fields present only on accept.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AssociationAcceptParts {
    /// Which flows the FT is accepting. `All` corresponds to on-wire
    /// Number of Flows = `0b111`; `Specific(..)` to 0..=6 explicit flow
    /// IDs.
    pub flow_acceptance: FlowAcceptance,
    /// `Some(..)` iff the FT is overriding HARQ configuration
    /// (on-wire HARQ-mod bit = 1). When `None`, the requested HARQ
    /// configuration is accepted as-is.
    pub harq_override: Option<HarqOverride>,
    /// `Some(..)` iff the on-wire Group bit = 1.
    pub group: Option<GroupAssignment>,
}

/// HARQ configuration override carried in an accepted Association Response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct HarqOverride {
    pub harq_processes_rx: HarqProcesses,
    pub max_harq_re_rx: MaxHarqReTx,
    pub harq_processes_tx: HarqProcesses,
    pub max_harq_re_tx: MaxHarqReTx,
}

/// Group / resource-tag pair carried in an accepted Association Response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct GroupAssignment {
    pub group_id: GroupId,
    pub resource_tag: ResourceTag,
}

/// How many flows the FT is accepting.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FlowAcceptance {
    /// All flows requested in the Association Request are accepted as-is
    /// (on-wire Number of Flows = `0b111`, no Flow ID octets).
    All,
    /// Specific (0..=6) flow IDs accepted. Empty vec is allowed and
    /// corresponds to on-wire Number of Flows = 0.
    Specific(Vec<FlowId, MAX_RESPONSE_FLOWS>),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn association_response_reject_round_trip() {
        let parts = AssociationResponseParts::Reject {
            cause: RejectCause::ShortRdIdConflict,
            timer: RejectTimer::S60,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 2);
        let parsed = AssociationResponseParts::parse(&buf[..n]).unwrap();
        match parsed {
            AssociationResponseParts::Reject { cause, timer } => {
                assert_eq!(cause, RejectCause::ShortRdIdConflict);
                assert_eq!(timer, RejectTimer::S60);
            }
            _ => panic!("expected reject"),
        }
    }

    #[test]
    fn association_response_accept_all_flows_no_harq_no_group_round_trip() {
        let parts = AssociationResponseParts::Accept(AssociationAcceptParts {
            flow_acceptance: FlowAcceptance::All,
            harq_override: None,
            group: None,
        });
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        // B0 = ACK(1) | reserved(0) | harq-mod(0) | n_flows(0b111) | group(0) | reserved(0)
        //    = 0b1_0_0_111_00 = 0x9C
        assert_eq!(buf[0], 0x9C);
        let parsed = AssociationResponseParts::parse(&buf[..n]).unwrap();
        match parsed {
            AssociationResponseParts::Accept(a) => {
                assert!(matches!(a.flow_acceptance, FlowAcceptance::All));
                assert!(a.harq_override.is_none());
                assert!(a.group.is_none());
            }
            _ => panic!("expected accept"),
        }
    }

    #[test]
    fn association_response_accept_full_round_trip() {
        let flows = Vec::from_slice(&[
            FlowId::new(0b000011).unwrap(),
            FlowId::new(0b000100).unwrap(),
        ])
        .unwrap();
        let parts = AssociationResponseParts::Accept(AssociationAcceptParts {
            flow_acceptance: FlowAcceptance::Specific(flows),
            harq_override: Some(HarqOverride {
                harq_processes_rx: HarqProcesses::new(2).unwrap(),
                max_harq_re_rx: MaxHarqReTx::new(5).unwrap(),
                harq_processes_tx: HarqProcesses::new(4).unwrap(),
                max_harq_re_tx: MaxHarqReTx::new(7).unwrap(),
            }),
            group: Some(GroupAssignment {
                group_id: GroupId::new(0x42).unwrap(),
                resource_tag: ResourceTag::new(0x21).unwrap(),
            }),
        });
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (B0) + 2 (HARQ override) + 2 (flow ids) + 2 (group) = 7
        assert_eq!(n, 7);

        let parsed = AssociationResponseParts::parse(&buf[..n]).unwrap();
        let a = match parsed {
            AssociationResponseParts::Accept(a) => a,
            _ => panic!("expected accept"),
        };
        let h = a.harq_override.unwrap();
        assert_eq!(h.harq_processes_rx.as_u8(), 2);
        assert_eq!(h.max_harq_re_rx.as_u8(), 5);
        assert_eq!(h.harq_processes_tx.as_u8(), 4);
        assert_eq!(h.max_harq_re_tx.as_u8(), 7);
        let parsed_flows = match a.flow_acceptance {
            FlowAcceptance::Specific(f) => f,
            _ => panic!("expected specific flows"),
        };
        assert_eq!(parsed_flows.len(), 2);
        assert_eq!(parsed_flows[0].as_u8(), 0b000011);
        assert_eq!(parsed_flows[1].as_u8(), 0b000100);
        let g = a.group.unwrap();
        assert_eq!(g.group_id.as_u8(), 0x42);
        assert_eq!(g.resource_tag.as_u8(), 0x21);
    }

    #[test]
    fn association_response_parser_rejects_reserved_reject_cause() {
        // ACK=0, B1 = reject cause 5 (reserved) | reject timer 0
        let buf = [0x00, 0b0101_0000];
        assert!(AssociationResponseParts::parse(&buf).is_err());
    }

    #[test]
    fn association_response_parser_rejects_reserved_reject_timer() {
        // ACK=0, B1 = reject cause 0 | reject timer 9 (reserved)
        let buf = [0x00, 0b0000_1001];
        assert!(AssociationResponseParts::parse(&buf).is_err());
    }

    /// Golden vector (minimal) hand-derived from Figure 6.4.2.5-1 and Tables 6.4.2.5-1/-2.
    ///
    /// Minimal = Reject variant (ACK/NACK=0), 2 bytes.
    ///
    /// Chosen values:
    ///   Reject Cause = NonSecuredNotAccepted (code 3 = 0b0011, Table 6.4.2.5-2)
    ///   Reject Timer = S120 (code 5 = 0b0101, Table 6.4.2.5-2)
    ///
    /// Byte 0: ACK/NACK=0, reserved bits = 0x00
    /// Byte 1: [Reject Cause(4) | Reject Timer(4)]
    ///       = [0011 | 0101] = 0b0011_0101 = 0x35
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 2] = [
            0b0_0000000, // ACK/NACK=0 (Reject) | Reserved
            0b0011_0101, // Reject Cause=NonSecuredNotAccepted(3) | Reject Timer=S120(5)
        ];
        let parts = AssociationResponseParts::Reject {
            cause: RejectCause::NonSecuredNotAccepted,
            timer: RejectTimer::S120,
        };
        let mut buf = [0u8; 2];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(AssociationResponseParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.2.5-1 and Table 6.4.2.5-1.
    ///
    /// Full = Accept with all optional fields:
    ///   HARQ override (HM=1), 6 specific flow IDs, Group (G=1).
    ///   Total: 1 + 2 + 6 + 2 = 11 bytes.
    ///
    /// Chosen values:
    ///   ACK/NACK = 1 (Accept)
    ///   HM = 1 (HARQ override present)
    ///   Number of Flows = 6 (MAX_RESPONSE_FLOWS; Specific list)
    ///   Group = 1 (Group ID + Resource Tag present)
    ///   HARQ RX = 2, MAX Re-RX = 5
    ///   HARQ TX = 3, MAX Re-TX = 10
    ///   Flow IDs: 0x01, 0x02, 0x03, 0x0A, 0x15, 0x20
    ///   Group ID = 0x55, Resource Tag = 0x2A
    ///
    /// Byte 0: ACK=1 | R=0 | HM=1 | N=6(0b110) | G=1 | R=0
    ///   = 0x80 | 0x20 | (6<<2) | 0x02 = 0x80 | 0x20 | 0x18 | 0x02 = 0xBA
    /// Byte 1: [HARQ RX (3 bits) | MAX Re-RX (5 bits)] = (0b010 << 5) | 0b00101 = 0x45
    /// Byte 2: [HARQ TX (3 bits) | MAX Re-TX (5 bits)] = (0b011 << 5) | 0b01010 = 0x6A
    /// Bytes 3-8: Flow IDs (low 6 bits each)
    /// Byte 9: Group ID = 0x55 (7 bits)
    /// Byte 10: Resource Tag = 0x2A (7 bits)
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 11] = [
            0b1_0_1_110_1_0, // ACK=1 | R=0 | HM=1 | N=6(110) | G=1 | R=0
            0b010_00101,     // HARQ RX=2 | MAX Re-RX=5
            0b011_01010,     // HARQ TX=3 | MAX Re-TX=10
            0x01,            // Flow ID 0 = 0x01
            0x02,            // Flow ID 1 = 0x02
            0x03,            // Flow ID 2 = 0x03
            0x0A,            // Flow ID 3 = 0x0A
            0x15,            // Flow ID 4 = 0x15
            0x20,            // Flow ID 5 = 0x20
            0x55,            // Group ID = 0x55 (7-bit)
            0x2A,            // Resource Tag = 0x2A (7-bit)
        ];
        let flows = Vec::from_slice(&[
            FlowId::new(0x01).unwrap(),
            FlowId::new(0x02).unwrap(),
            FlowId::new(0x03).unwrap(),
            FlowId::new(0x0A).unwrap(),
            FlowId::new(0x15).unwrap(),
            FlowId::new(0x20).unwrap(),
        ])
        .unwrap();
        let parts = AssociationResponseParts::Accept(AssociationAcceptParts {
            flow_acceptance: FlowAcceptance::Specific(flows),
            harq_override: Some(HarqOverride {
                harq_processes_rx: HarqProcesses::new(2).unwrap(),
                max_harq_re_rx: MaxHarqReTx::new(5).unwrap(),
                harq_processes_tx: HarqProcesses::new(3).unwrap(),
                max_harq_re_tx: MaxHarqReTx::new(10).unwrap(),
            }),
            group: Some(GroupAssignment {
                group_id: GroupId::new(0x55).unwrap(),
                resource_tag: ResourceTag::new(0x2A).unwrap(),
            }),
        });
        let mut buf = [0u8; 11];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(AssociationResponseParts::parse(&GOLDEN).unwrap(), parts);
    }
}
