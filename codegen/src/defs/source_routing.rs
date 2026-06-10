//! Source Routing IE: ETSI TS 103 636-4, clause 6.4.3.16,
//! Figure 6.4.3.16-1.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "source_routing",
        name: "SourceRoutingParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.16, Figure 6.4.3.16-1",
        doc: "Owned representation of a Source Routing IE body (6 bytes).",
        ie_type: Some("SourceRouting"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "source_routing_id",
                fig: Some("Source Routing ID"),
                bits: 32,
                ty: Ty::Fallible {
                    ty: "LongRdId",
                    ctor: "new",
                    getter: "as_u32",
                },
                doc: "Long RD ID identifying the source route.",
            }),
            Item::Field(Field {
                name: "hop_limit",
                fig: Some("Hop Limit"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "Hop",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Maximum number of hops.",
            }),
            Item::Field(Field {
                name: "hop_count",
                fig: Some("Hop Count"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "Hop",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Hops travelled so far.",
            }),
            Item::Field(Field {
                name: "validity_timer",
                fig: Some("Validity Timer"),
                bits: 8,
                ty: Ty::Fallible {
                    ty: "SourceRoutingValidityTimer",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Route validity timer code.",
            }),
        ],
    }
}
