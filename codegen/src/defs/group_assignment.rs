//! Group Assignment IE: ETSI TS 103 636-4, clause 6.4.3.9,
//! Figure 6.4.3.9-1.

use crate::ir::{Field, Item, MessageDef, TailSlice, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "group_assignment",
        name: "GroupAssignmentParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.9, Figure 6.4.3.9-1",
        doc: "Owned representation of a Group Assignment IE body.",
        ie_type: Some("GroupAssignment"),
        short_ie: None,
        imports: &["use crate::mac::messages::group_assignment::GroupResourceTagEntry;"],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "single",
                fig: Some("S"),
                bits: 1,
                ty: Ty::Bool,
                doc: "On-wire `Single` bit. `true` = single resource assignment \
                      for the group member; `false` = multiple resource \
                      assignments follow.",
            }),
            Item::Field(Field {
                name: "group_id",
                fig: Some("Group ID"),
                bits: 7,
                ty: Ty::Fallible {
                    ty: "GroupId",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Group ID the assignment applies to.",
            }),
            Item::TailSlice(TailSlice {
                name: "tags",
                fig: "Resource Tag Entry",
                ty: "GroupResourceTagEntry",
                getter: "as_raw",
                doc: "Resource tag entries, one per remaining body byte.",
            }),
        ],
    }
}
