//! RD Capability IE (short form): ETSI TS 103 636-4, clause 6.4.3.15,
//! Figure 6.4.3.15-1.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "rd_capability_short",
        name: "RdCapabilityShortParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.15, Figure 6.4.3.15-1",
        doc: "Owned representation of a short-form RD Capability IE body (1 byte).",
        ie_type: None,
        short_ie: Some("ShortIeType::Len1(IEType5bitLen1::RdCapabilityShort)"),
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 2 },
            Item::Field(Field {
                name: "cb_mc",
                fig: Some("CBM"),
                bits: 1,
                ty: Ty::Bool,
                doc: "`CB_MC`: RD in FT mode supports association without \
                      monitoring Cluster Beacon messages.",
            }),
            Item::Field(Field {
                name: "harq_feedback_delay",
                fig: Some("HARQ Delay"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "HarqFeedbackDelay",
                    ctor: "try_from_u8",
                    getter: "subslots",
                },
                doc: "HARQ feedback delay in subslots (0..=6; 7..=15 reserved).",
            }),
            Item::Field(Field {
                name: "dwa",
                fig: Some("DWA"),
                bits: 1,
                ty: Ty::Bool,
                doc: "`DWA`: RD in FT mode supports uplink data transmission \
                      without association.",
            }),
        ],
    }
}
