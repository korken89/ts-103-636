//! Association Request message: ETSI TS 103 636-4, clause 6.4.2.4,
//! Figure 6.4.2.4-1 / Tables 6.4.2.4-1/-2.

use crate::ir::{Composite, Field, Group, Item, MessageDef, Repeat, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "association_request",
        name: "AssociationRequestParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.4, Figure 6.4.2.4-1, Tables 6.4.2.4-1/-2",
        doc: "Owned representation of an Association Request body.",
        ie_type: Some("AssociationRequest"),
        short_ie: None,
        imports: &["use crate::mac::messages::association_request::FtModeFields;"],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "setup_cause",
                fig: Some("Setup Cause"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "SetupCause",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Association setup cause (Table 6.4.2.4-1).",
            }),
            Item::Count {
                of: "flow_ids",
                bits: 3,
                fig: "N",
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
                of: "ft_mode",
                fig: "FT",
            },
            Item::PresenceFlag {
                of: "ft_mode.current_cluster_channel",
                fig: "CC",
            },
            Item::Reserved { bits: 7 },
            Item::Field(Field {
                name: "harq_processes_tx",
                fig: Some("HARQ TX"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "HarqProcesses",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Number of HARQ processes for transmission.",
            }),
            Item::Field(Field {
                name: "max_harq_re_tx",
                fig: Some("MAX Re-TX"),
                bits: 5,
                ty: Ty::Fallible {
                    ty: "MaxHarqReTx",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Maximum HARQ retransmissions.",
            }),
            Item::Field(Field {
                name: "harq_processes_rx",
                fig: Some("HARQ RX"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "HarqProcesses",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Number of HARQ processes for reception.",
            }),
            Item::Field(Field {
                name: "max_harq_re_rx",
                fig: Some("MAX Re-RX"),
                bits: 5,
                ty: Ty::Fallible {
                    ty: "MaxHarqReTx",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Maximum HARQ re-receptions.",
            }),
            Item::Repeat(Repeat {
                name: "flow_ids",
                doc: "0..=6 flow IDs. The on-wire `Number of Flows` field is \
                      derived from `flow_ids.len()`.",
                items: &[
                    Item::Reserved { bits: 2 },
                    Item::Field(Field {
                        name: "flow_id",
                        fig: Some("Flow ID"),
                        bits: 6,
                        ty: Ty::Fallible {
                            ty: "FlowId",
                            ctor: "new",
                            getter: "as_u8",
                        },
                        doc: "Flow ID.",
                    }),
                ],
                composite: None,
                bias: 0,
                reserved_max: true,
                all_escape: None,
                max_const: "MAX_REQUEST_FLOWS",
                max_doc: "Maximum number of flow IDs in an Association Request. The \
                          on-wire 3-bit Number of Flows field caps at 6 (`0b111` is \
                          reserved).",
            }),
            Item::Optional(Group {
                name: "ft_mode",
                doc: "FT-mode-specific fields. `Some(..)` iff the on-wire FT mode \
                      bit is set. Bundling the Current Cluster Channel inside makes \
                      the `Current => FT mode` invariant unrepresentable.",
                items: &[
                    Item::Field(Field {
                        name: "network_beacon_period",
                        fig: Some("NB Period"),
                        bits: 4,
                        ty: Ty::Fallible {
                            ty: "NetworkBeaconPeriod",
                            ctor: "try_from_u8",
                            getter: "as_u8",
                        },
                        doc: "Network beacon period code (4 bits, see Table 6.4.2.2-1).",
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
                        doc: "Cluster beacon period code (4 bits, see Table 6.4.2.2-1).",
                    }),
                    Item::Reserved { bits: 3 },
                    Item::Field(Field {
                        name: "next_cluster_channel",
                        fig: Some("Next Cluster Channel"),
                        bits: 13,
                        ty: Ty::Fallible {
                            ty: "AbsoluteChannel",
                            ctor: "new",
                            getter: "as_u16",
                        },
                        doc: "Next cluster channel.",
                    }),
                    Item::Field(Field {
                        name: "time_to_next",
                        fig: Some("Time To Next"),
                        bits: 32,
                        ty: Ty::Raw,
                        doc: "Time to next beacon period, microseconds.",
                    }),
                    Item::Optional(Group {
                        name: "current_cluster_channel",
                        doc: "Current cluster channel, when it differs from the next.",
                        items: &[
                            Item::Reserved { bits: 3 },
                            Item::Field(Field {
                                name: "current_cluster_channel",
                                fig: Some("Current Cluster Channel"),
                                bits: 13,
                                ty: Ty::Fallible {
                                    ty: "AbsoluteChannel",
                                    ctor: "new",
                                    getter: "as_u16",
                                },
                                doc: "Current cluster channel.",
                            }),
                        ],
                        composite: None,
                    }),
                ],
                composite: Some(Composite {
                    ty: "FtModeFields",
                    construct: "FtModeFields { network_beacon_period, cluster_beacon_period, next_cluster_channel, time_to_next, current_cluster_channel }",
                    accessors: &[
                        "v.network_beacon_period",
                        "v.cluster_beacon_period",
                        "v.next_cluster_channel",
                        "v.time_to_next",
                    ],
                }),
            }),
        ],
    }
}
