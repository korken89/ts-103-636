//! Reconfiguration Request message: ETSI TS 103 636-4, clause 6.4.2.7,
//! Figure 6.4.2.7-1.

use crate::ir::{AllOnes, Composite, CountSlice, Field, Group, Item, MessageDef, Ty};

/// The shared 1-byte HARQ configuration block (3-bit processes +
/// 5-bit max re-tx/re-rx), used in both the TX and RX positions.
pub const fn harq_group(name: &'static str, doc: &'static str) -> Group {
    Group {
        name,
        doc,
        items: &[
            Item::Field(Field {
                name: "processes",
                fig: Some("HARQ Proc"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "HarqProcesses",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Number of HARQ processes.",
            }),
            Item::Field(Field {
                name: "max_re",
                fig: Some("MAX HARQ Re"),
                bits: 5,
                ty: Ty::Fallible {
                    ty: "MaxHarqReTx",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Maximum HARQ retransmissions / re-receptions.",
            }),
        ],
        composite: Some(Composite {
            ty: "HarqConfig",
            construct: "HarqConfig { processes, max_re }",
            accessors: &["v.processes", "v.max_re"],
        }),
    }
}

pub fn def() -> MessageDef {
    const TX_HARQ: Group = harq_group(
        "tx_harq",
        "`Some(..)` iff the TX HARQ flag is set (configuration is \
         requested to be modified, fields follow).",
    );
    const RX_HARQ: Group = harq_group("rx_harq", "`Some(..)` iff the RX HARQ flag is set.");
    MessageDef {
        module: "reconfiguration_request",
        name: "ReconfigurationRequestParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.7, Figure 6.4.2.7-1",
        doc: "Owned representation of a Reconfiguration Request body (clause 6.4.2.7).",
        ie_type: Some("ReconfigurationRequest"),
        short_ie: None,
        imports: &["use crate::mac::messages::reconfiguration::HarqConfig;"],
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
                doc: "`true` iff the RD Capability flag is set (an RD Capability \
                      IE follows this body in the same MAC PDU).",
            }),
            Item::Count {
                of: "flows",
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
                name: "flows",
                fig: "Flow Entry",
                ty: "FlowEntry",
                getter: "as_raw",
                all_ones: AllOnes::Reserved,
                doc: "0..=6 flow change entries. Number of Flows = 7 is reserved \
                      in Request and is rejected by serialize.",
            }),
        ],
    }
}
