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
}
