//! Load Info IE: ETSI TS 103 636-4, clause 6.4.3.10,
//! Figure 6.4.3.10-1.

use crate::ir::{Composite, Field, Group, Item, MessageDef, Switch, Ty, Wide};

pub fn def() -> MessageDef {
    MessageDef {
        module: "load_info",
        name: "LoadInfoParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.10, Figure 6.4.3.10-1",
        doc: "Owned representation of a Load Info IE body.",
        ie_type: Some("LoadInfo"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 4 },
            Item::Selector {
                of: "max_associated_rds",
                fig: "M",
            },
            Item::PresenceFlag {
                of: "currently_associated_pt_mode",
                fig: "PT",
            },
            Item::PresenceFlag {
                of: "rach_load",
                fig: "RL",
            },
            Item::PresenceFlag {
                of: "channel_load",
                fig: "CL",
            },
            Item::Field(Field {
                name: "traffic_load",
                fig: Some("Traffic Load"),
                bits: 8,
                ty: Ty::Wrap {
                    ty: "LoadPercentage",
                    construct: "LoadPercentage({})",
                    deconstruct: "{}.0",
                },
                doc: "Traffic load percentage.",
            }),
            Item::Switch(Switch {
                field: Field {
                    name: "max_associated_rds",
                    fig: Some("MAX Associated RDs"),
                    bits: 16,
                    ty: Ty::Raw,
                    doc: "MAX number of associated devices. Encoded as 8-bit on \
                          the wire when `value <= 255`, otherwise as 16-bit.",
                },
                narrow_bits: 8,
                wide_bits: 16,
                wide: Wide::ValueOverflow,
            }),
            Item::Field(Field {
                name: "currently_associated_ft_mode",
                fig: Some("Associated FT Mode"),
                bits: 8,
                ty: Ty::Wrap {
                    ty: "LoadPercentage",
                    construct: "LoadPercentage({})",
                    deconstruct: "{}.0",
                },
                doc: "Percentage of associated RDs operating in FT mode.",
            }),
            Item::Optional(Group {
                name: "currently_associated_pt_mode",
                doc: "Percentage of associated RDs operating in PT mode.",
                items: &[Item::Field(Field {
                    name: "currently_associated_pt_mode",
                    fig: Some("Associated PT Mode"),
                    bits: 8,
                    ty: Ty::Wrap {
                        ty: "LoadPercentage",
                        construct: "LoadPercentage({})",
                        deconstruct: "{}.0",
                    },
                    doc: "Percentage of associated RDs operating in PT mode.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "rach_load",
                doc: "RACH load percentage.",
                items: &[Item::Field(Field {
                    name: "rach_load",
                    fig: Some("RACH Load"),
                    bits: 8,
                    ty: Ty::Wrap {
                        ty: "LoadPercentage",
                        construct: "LoadPercentage({})",
                        deconstruct: "{}.0",
                    },
                    doc: "RACH load percentage.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "channel_load",
                doc: "Pair of (free %, busy %), both or neither present.",
                items: &[
                    Item::Field(Field {
                        name: "free",
                        fig: Some("Free"),
                        bits: 8,
                        ty: Ty::Wrap {
                            ty: "LoadPercentage",
                            construct: "LoadPercentage({})",
                            deconstruct: "{}.0",
                        },
                        doc: "Percentage of free subslots.",
                    }),
                    Item::Field(Field {
                        name: "busy",
                        fig: Some("Busy"),
                        bits: 8,
                        ty: Ty::Wrap {
                            ty: "LoadPercentage",
                            construct: "LoadPercentage({})",
                            deconstruct: "{}.0",
                        },
                        doc: "Percentage of busy subslots.",
                    }),
                ],
                composite: Some(Composite {
                    ty: "(LoadPercentage, LoadPercentage)",
                    construct: "(free, busy)",
                    accessors: &["v.0", "v.1"],
                }),
            }),
        ],
    }
}
