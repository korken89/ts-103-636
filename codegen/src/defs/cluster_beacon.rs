//! Cluster Beacon message: ETSI TS 103 636-4, clause 6.4.2.3,
//! Figure 6.4.2.3-1 / Table 6.4.2.3-1.
//!
//! The Frame Offset width is selected by the PHY `mu` context
//! parameter (8-bit when `mu <= 4`, 16-bit otherwise).

use crate::ir::{Ctx, Field, Group, Item, MessageDef, Switch, Ty, Wide};

pub fn def() -> MessageDef {
    MessageDef {
        module: "cluster_beacon",
        name: "ClusterBeaconParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.3, Figure 6.4.2.3-1, Table 6.4.2.3-1",
        doc: "Owned representation of a Cluster Beacon body.",
        ie_type: Some("ClusterBeacon"),
        short_ie: None,
        imports: &[],
        ctx: &[Ctx {
            name: "mu",
            ty: "Mu",
            doc: "PHY subcarrier scaling factor in effect for this cluster. Not \
                  itself on the wire: it selects the Frame Offset field width \
                  (8 bits when mu <= 4, 16 bits otherwise, Table 6.4.2.3-1).",
        }],
        field_groups: &[],
        items: &[
            Item::Field(Field {
                name: "sfn",
                fig: Some("SFN"),
                bits: 8,
                ty: Ty::Wrap {
                    ty: "Sfn",
                    construct: "Sfn({})",
                    deconstruct: "{}.0",
                },
                doc: "SFN (System Frame Number), byte 0.",
            }),
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
                doc: "Power-constrained indicator (bit 7 of byte 1).",
            }),
            Item::PresenceFlag {
                of: "frame_offset",
                fig: "FO",
            },
            Item::PresenceFlag {
                of: "next_cluster_channel",
                fig: "NC",
            },
            Item::PresenceFlag {
                of: "time_to_next",
                fig: "TTN",
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
                doc: "Network beacon period code (4 bits, high nibble of byte 2).",
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
                doc: "Cluster beacon period code (4 bits, low nibble of byte 2).",
            }),
            Item::Field(Field {
                name: "count_to_trigger",
                fig: Some("CTT"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "CountToTrigger",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Count to trigger (4 bits, high nibble of byte 3).",
            }),
            Item::Field(Field {
                name: "rel_quality",
                fig: Some("RelQ"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "Quality",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Relative quality (2 bits, byte 3 bits 3..=2).",
            }),
            Item::Field(Field {
                name: "min_quality",
                fig: Some("MinQ"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "Quality",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Minimum quality (2 bits, byte 3 bits 1..=0).",
            }),
            Item::Optional(Group {
                name: "cluster_max_tx_power",
                doc: "Optional 4-bit cluster maximum TX power field.",
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
                name: "frame_offset",
                doc: "Optional frame offset in subslots (8-bit on the wire when \
                      `mu <= 4`, 16-bit otherwise).",
                items: &[Item::Switch(Switch {
                    field: Field {
                        name: "frame_offset",
                        fig: Some("Frame Offset"),
                        bits: 16,
                        ty: Ty::Raw,
                        doc: "Frame offset in subslots.",
                    },
                    narrow_bits: 8,
                    wide_bits: 16,
                    wide: Wide::Ctx("{mu}.as_u8() > 4"),
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "next_cluster_channel",
                doc: "Optional next cluster channel (13-bit AbsoluteChannel).",
                items: &[
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
                        doc: "Next cluster channel.",
                    }),
                ],
                composite: None,
            }),
            Item::Optional(Group {
                name: "time_to_next",
                doc: "Optional time-to-next-channel in microseconds (32-bit).",
                items: &[Item::Field(Field {
                    name: "time_to_next",
                    fig: Some("Time To Next"),
                    bits: 32,
                    ty: Ty::Raw,
                    doc: "Time to next (microseconds).",
                })],
                composite: None,
            }),
        ],
    }
}
