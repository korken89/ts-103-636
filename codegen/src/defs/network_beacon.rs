//! Network Beacon message: ETSI TS 103 636-4, clause 6.4.2.2,
//! Figure 6.4.2.2-1 / Table 6.4.2.2-1.

use crate::ir::{Field, Group, Item, MessageDef, Repeat, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "network_beacon",
        name: "NetworkBeaconParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.2, Figure 6.4.2.2-1, Table 6.4.2.2-1",
        doc: "Owned representation of a Network Beacon body.",
        ie_type: Some("NetworkBeacon"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 3 },
            Item::PresenceFlag {
                of: "cluster_max_tx_power",
                fig: "TXP",
            },
            Item::Field(Field {
                name: "power_const",
                fig: Some("PC"),
                bits: 1,
                ty: Ty::Wrap {
                    ty: "PowerConst",
                    construct: "if {} == 0 { PowerConst::Unconstrained } else { PowerConst::Constrained }",
                    deconstruct: "matches!({}, PowerConst::Constrained) as u8",
                },
                doc: "Power-constrained indicator.",
            }),
            Item::PresenceFlag {
                of: "current_cluster_channel",
                fig: "CC",
            },
            Item::Count {
                of: "additional_channels",
                bits: 2,
                fig: "N",
            },
            Item::Field(Field {
                name: "network_beacon_period",
                fig: Some("NB Period"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "NetworkBeaconPeriod",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Network beacon period code (4 bits).",
            }),
            Item::Field(Field {
                name: "cluster_beacon_period",
                fig: Some("CB Period"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "ClusterBeaconPeriod",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Cluster beacon period code (4 bits).",
            }),
            Item::Reserved { bits: 3 },
            Item::Field(Field {
                name: "next_cluster_channel",
                fig: Some("Next Cluster Channel"),
                bits: 13,
                ty: Ty::Fallible {
                    ty: "AbsoluteChannel",
                    ctor: "try_from_u16",
                    getter: "as_u16",
                },
                doc: "Next cluster channel (13-bit).",
            }),
            Item::Field(Field {
                name: "time_to_next",
                fig: Some("Time To Next"),
                bits: 32,
                ty: Ty::Raw,
                doc: "Time to next beacon period in microseconds (32-bit).",
            }),
            Item::Optional(Group {
                name: "cluster_max_tx_power",
                doc: "Optional cluster maximum TX power (4-bit field).",
                items: &[
                    Item::Reserved { bits: 4 },
                    Item::Field(Field {
                        name: "cluster_max_tx_power",
                        fig: Some("Max TX Power"),
                        bits: 4,
                        ty: Ty::Fallible {
                            ty: "TransmitPower",
                            ctor: "try_from_u8",
                            getter: "as_u8",
                        },
                        doc: "Cluster maximum TX power.",
                    }),
                ],
                composite: None,
            }),
            Item::Optional(Group {
                name: "current_cluster_channel",
                doc: "Optional current cluster channel.",
                items: &[
                    Item::Reserved { bits: 3 },
                    Item::Field(Field {
                        name: "current_cluster_channel",
                        fig: Some("Current Cluster Channel"),
                        bits: 13,
                        ty: Ty::Fallible {
                            ty: "AbsoluteChannel",
                            ctor: "try_from_u16",
                            getter: "as_u16",
                        },
                        doc: "Current cluster channel.",
                    }),
                ],
                composite: None,
            }),
            Item::Repeat(Repeat {
                name: "additional_channels",
                doc: "Additional Network Beacon channels (up to \
                      [`MAX_ADDITIONAL_CHANNELS`]).",
                items: &[
                    Item::Reserved { bits: 3 },
                    Item::Field(Field {
                        name: "additional_channel",
                        fig: Some("Additional Channel"),
                        bits: 13,
                        ty: Ty::Fallible {
                            ty: "AbsoluteChannel",
                            ctor: "try_from_u16",
                            getter: "as_u16",
                        },
                        doc: "Additional Network Beacon channel.",
                    }),
                ],
                composite: None,
                bias: 0,
                reserved_max: false,
                all_escape: None,
                max_const: "MAX_ADDITIONAL_CHANNELS",
                max_doc: "Maximum number of additional Network Beacon channels \
                          (2-bit count field).",
            }),
        ],
    }
}
