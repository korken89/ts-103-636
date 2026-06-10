//! Joining Information IE: ETSI TS 103 636-4, clause 6.4.3.17,
//! Figure 6.4.3.17-1.

use crate::ir::{Field, Item, MessageDef, Repeat, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "joining_information",
        name: "JoiningInformationParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.17, Figure 6.4.3.17-1",
        doc: "Owned representation of a Joining Information IE body.",
        ie_type: Some("JoiningInformation"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 6 },
            Item::Count {
                of: "endpoints",
                bits: 2,
                fig: "N",
            },
            Item::Repeat(Repeat {
                name: "endpoints",
                doc: "1..=4 endpoint protocol values. The on-wire 2-bit count \
                      carries `endpoints.len() - 1`.",
                items: &[Item::Field(Field {
                    name: "endpoint",
                    fig: Some("Endpoint"),
                    bits: 16,
                    ty: Ty::Wrap {
                        ty: "EndpointProtocol",
                        construct: "EndpointProtocol({})",
                        deconstruct: "{}.0",
                    },
                    doc: "Endpoint protocol value.",
                })],
                composite: None,
                bias: 1,
                reserved_max: false,
                all_escape: None,
                max_const: "MAX_ENDPOINTS",
                max_doc: "Maximum number of endpoints (on-wire 2-bit count + 1 = 4).",
            }),
        ],
    }
}
