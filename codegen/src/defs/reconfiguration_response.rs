//! Reconfiguration Response message: ETSI TS 103 636-4, clause 6.4.2.8,
//! Figure 6.4.2.8-1. Shares the Request wire layout; Number of Flows
//! = 0b111 means "all accepted" here instead of being reserved.

use super::reconfiguration_request::harq_group;
use crate::ir::{AllOnes, CountSlice, Field, Group, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    const TX_HARQ: Group = harq_group(
        "tx_harq",
        "`Some(..)` iff the TX HARQ flag is set (Request's TX HARQ was \
         NOT accepted, replacement fields follow).",
    );
    const RX_HARQ: Group = harq_group("rx_harq", "`Some(..)` iff the RX HARQ flag is set.");
    MessageDef {
        module: "reconfiguration_response",
        name: "ReconfigurationResponseParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.8, Figure 6.4.2.8-1",
        doc: "Owned representation of a Reconfiguration Response body (clause 6.4.2.8).",
        ie_type: Some("ReconfigurationResponse"),
        short_ie: None,
        imports: &[
            "use crate::mac::messages::reconfiguration::FlowChangeAcceptance;",
            "use crate::mac::messages::reconfiguration::HarqConfig;",
        ],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::PresenceFlag {
                of: "tx_harq",
                fig: "TX",
            },
            Item::PresenceFlag {
                of: "rx_harq",
                fig: "RX",
            },
            Item::Field(Field {
                name: "rd_capability_changed",
                fig: Some("RDC"),
                bits: 1,
                ty: Ty::Bool,
                doc: "`true` iff the RD Capability flag is set.",
            }),
            Item::Count {
                of: "flow_acceptance",
                bits: 3,
                fig: "N",
            },
            Item::Field(Field {
                name: "radio_resource",
                fig: Some("RR"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "RadioResourceChange",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Radio resource change indication.",
            }),
            Item::Optional(TX_HARQ),
            Item::Optional(RX_HARQ),
            Item::CountSlice(CountSlice {
                name: "flow_acceptance",
                fig: "Flow Entry",
                ty: "FlowEntry",
                getter: "as_raw",
                all_ones: AllOnes::All {
                    ty: "FlowChangeAcceptance",
                    all: "FlowChangeAcceptance::All",
                    specific: "FlowChangeAcceptance::Specific",
                },
                doc: "Which flow changes the responder is accepting.",
            }),
        ],
    }
}
