//! MAC Security Info IE: ETSI TS 103 636-4, clause 6.4.3.1,
//! Figure 6.4.3.1-1 / Tables 6.4.3.1-1 and 6.4.3.1-2.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "mac_security_info",
        name: "MacSecurityInfoParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.1, Figure 6.4.3.1-1, Tables 6.4.3.1-1 and 6.4.3.1-2",
        doc: "Owned representation of a MAC Security Info IE body (5 bytes fixed).",
        ie_type: None,
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "version",
                fig: Some("Version"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "SecurityVersion",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Security version (only Mode 1 is defined).",
            }),
            Item::Field(Field {
                name: "key_index",
                fig: Some("Key Idx"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "KeyIndex",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Index of the integrity / cipher key pair in use.",
            }),
            Item::Field(Field {
                name: "iv_type",
                fig: Some("IV Type"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "SecurityIvType",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Security IV type (Table 6.4.3.1-2).",
            }),
            Item::Field(Field {
                name: "hpc",
                fig: Some("HPC"),
                bits: 32,
                ty: Ty::Raw,
                doc: "Hyper Packet Counter.",
            }),
        ],
    }
}
