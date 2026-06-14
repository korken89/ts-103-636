//! Broadcast Indication IE: ETSI TS 103 636-4, clause 6.4.3.7,
//! Figure 6.4.3.7-1 / Table 6.4.3.7-1.

use crate::ir::{Arm, Field, Item, MessageDef, Ty, Variant};

pub fn def() -> MessageDef {
    MessageDef {
        module: "broadcast_indication",
        name: "BroadcastIndicationParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.7, Figure 6.4.3.7-1, Table 6.4.3.7-1",
        doc: "Owned representation of a Broadcast Indication IE body.",
        ie_type: Some("BroadcastIndication"),
        short_ie: None,
        imports: &["use crate::mac::messages::broadcast_indication::BroadcastRdId;"],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "indication_type",
                fig: Some("Ind Type"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "IndicationType",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Indication type (Table 6.4.3.7-1).",
            }),
            Item::Selector {
                of: "rd_id",
                fig: "ID",
            },
            Item::Field(Field {
                name: "ack_nack",
                fig: Some("A"),
                bits: 1,
                ty: Ty::Bool,
                doc: "ACK/NACK bit. Meaningful only when \
                      `indication_type == IndicationType::RandomAccessResponse`; for \
                      other indication types the spec ignores this field.",
            }),
            Item::Field(Field {
                name: "feedback",
                fig: Some("FB"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "BroadcastFeedbackType",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Feedback type carried in the MCS / MIMO feedback byte.",
            }),
            Item::Field(Field {
                name: "resource_allocation_present",
                fig: Some("RA"),
                bits: 1,
                ty: Ty::Bool,
                doc: "On-wire `Resource Allocation` bit. `true` means a Resource \
                      Allocation IE for the RD follows in this MAC PDU.",
            }),
            Item::Variant(Variant {
                name: "rd_id",
                fig: "RD ID",
                ty: "BroadcastRdId",
                doc: "RD ID; the on-wire `IDType` bit selects Short or Long.",
                narrow: Arm {
                    path: "BroadcastRdId::Short",
                    bits: 16,
                    ty: Ty::Fallible {
                        ty: "ShortRdId",
                        ctor: "try_from_u16",
                        getter: "as_u16",
                    },
                },
                wide: Arm {
                    path: "BroadcastRdId::Long",
                    bits: 32,
                    ty: Ty::Fallible {
                        ty: "LongRdId",
                        ctor: "try_from_u32",
                        getter: "as_u32",
                    },
                },
            }),
            Item::Field(Field {
                name: "mcs_or_mimo_feedback",
                fig: Some("MCS / MIMO Feedback"),
                bits: 8,
                ty: Ty::Raw,
                doc: "Raw MCS / MIMO feedback byte. Interpretation depends on \
                      [`Self::feedback`]; see clause 6.4.3.7 Table 6.4.3.7-1.",
            }),
        ],
    }
}
