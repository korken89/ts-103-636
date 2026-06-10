//! Radio Device Status IE: ETSI TS 103 636-4, clause 6.4.3.13,
//! Figure 6.4.3.13-1 / Table 6.4.3.13-1.

use crate::ir::{Field, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "radio_device_status",
        name: "RadioDeviceStatusParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.13, Figure 6.4.3.13-1, Table 6.4.3.13-1",
        doc: "Owned representation of a Radio Device Status IE body (1 byte).",
        ie_type: None,
        short_ie: Some("ShortIeType::Len1(IEType5bitLen1::RadioDeviceStatus)"),
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 1 },
            Item::Field(Field {
                name: "association_needed",
                fig: Some("A"),
                bits: 1,
                ty: Ty::Bool,
                doc: "Association: the RD requests a new association \
                      (e.g. after detecting an error situation).",
            }),
            Item::Field(Field {
                name: "status",
                fig: Some("Status"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "RadioDeviceStatusFlag",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Status flag (Table 6.4.3.13-1).",
            }),
            Item::Field(Field {
                name: "duration",
                fig: Some("Duration"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "RadioDeviceStatusDuration",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Expected duration of the indicated status \
                      (Table 6.4.3.13-1).",
            }),
        ],
    }
}
