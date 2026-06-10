//! Route Info IE: ETSI TS 103 636-4, clause 6.4.3.2,
//! Figure 6.4.3.2-1 / Table 6.4.3.2-1.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "route_info",
        name: "RouteInfoParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.2, Figure 6.4.3.2-1, Table 6.4.3.2-1",
        doc: "Owned representation of a Route Info IE body (6 bytes fixed).",
        ie_type: Some("RouteInfo"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "sink_address",
                fig: Some("Sink Address"),
                bits: 32,
                ty: Ty::Fallible {
                    ty: "LongRdId",
                    ctor: "new",
                    getter: "as_u32",
                },
                doc: "Sink Address: Long RD ID of the sink (FT) of the route.",
            }),
            Item::Field(Field {
                name: "route_cost",
                fig: Some("Route Cost"),
                bits: 8,
                ty: Ty::Wrap {
                    ty: "RouteCost",
                    construct: "RouteCost({})",
                    deconstruct: "{}.0",
                },
                doc: "Route Cost towards the sink.",
            }),
            Item::Field(Field {
                name: "application_sequence_number",
                fig: Some("Application Sequence Number"),
                bits: 8,
                ty: Ty::Wrap {
                    ty: "ApplicationSequenceNumber",
                    construct: "ApplicationSequenceNumber({})",
                    deconstruct: "{}.0",
                },
                doc: "Application Sequence Number of the route advertisement.",
            }),
        ],
    }
}
