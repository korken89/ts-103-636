//! Association Control IE: ETSI TS 103 636-4, clause 6.4.3.18,
//! Figure 6.4.3.18-1 / Table 6.4.3.18-1.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "association_control",
        name: "AssociationControlParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.18, Figure 6.4.3.18-1, Table 6.4.3.18-1",
        doc: "Owned representation of an Association Control IE body (1 byte).",
        ie_type: None,
        short_ie: Some("ShortIeType::Len1(IEType5bitLen1::AssociationControl)"),
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "cb_m",
                fig: Some("CBM"),
                bits: 1,
                ty: Ty::Bool,
                doc: "`CB_M`: `false` = associated RD maintains cluster beacon \
                      reception; `true` = it does not.",
            }),
            Item::Field(Field {
                name: "dl_data_reception",
                fig: Some("DL Data"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "DlDataReception",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "DL data reception timing (Table 6.4.3.18-1).",
            }),
            Item::Field(Field {
                name: "ul_period",
                fig: Some("UL Period"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "UlPeriod",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "UL period (Table 6.4.3.18-1).",
            }),
        ],
    }
}
