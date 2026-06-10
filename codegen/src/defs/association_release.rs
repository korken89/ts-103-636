//! Association Release message: ETSI TS 103 636-4, clause 6.4.2.6,
//! Figure 6.4.2.6-1 / Table 6.4.2.6-1.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "association_release",
        name: "AssociationReleaseParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.6, Figure 6.4.2.6-1, Table 6.4.2.6-1",
        doc: "Owned representation of an Association Release body (1 byte).",
        ie_type: Some("AssociationRelease"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "cause",
                fig: Some("Release Cause"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "ReleaseCause",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Release Cause (Table 6.4.2.6-1).",
            }),
            // The receiver ignores the low nibble (clause 6.4.1).
            Item::Reserved { bits: 4 },
        ],
    }
}
