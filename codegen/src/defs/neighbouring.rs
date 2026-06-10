//! Neighbouring IE: ETSI TS 103 636-4, clause 6.4.3.6,
//! Figure 6.4.3.6-1 / Table 6.4.3.6-1.
//!
//! The bitmap bit order differs from the payload field order; the
//! payload follows the Optional item order below.

use crate::ir::{Composite, Field, Group, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "neighbouring",
        name: "NeighbouringParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.6, Figure 6.4.3.6-1, Table 6.4.3.6-1",
        doc: "Owned representation of a Neighbouring IE body.",
        ie_type: Some("Neighbouring"),
        short_ie: None,
        imports: &["use crate::mac::messages::neighbouring::RadioDeviceClass;"],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 1 },
            Item::PresenceFlag {
                of: "long_rd_id",
                fig: "ID",
            },
            Item::PresenceFlag {
                of: "radio_device_class",
                fig: "RDC",
            },
            Item::PresenceFlag {
                of: "snr",
                fig: "SNR",
            },
            Item::PresenceFlag {
                of: "rssi_2",
                fig: "R2",
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
            Item::Optional(Group {
                name: "long_rd_id",
                doc: "Long RD ID of the neighbouring RD.",
                items: &[Item::Field(Field {
                    name: "long_rd_id",
                    fig: Some("Long RD ID"),
                    bits: 32,
                    ty: Ty::Fallible {
                        ty: "LongRdId",
                        ctor: "new",
                        getter: "as_u32",
                    },
                    doc: "Long RD ID of the neighbouring RD.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "next_cluster_channel",
                doc: "Next cluster channel of the neighbouring RD.",
                items: &[
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
                        doc: "Next cluster channel of the neighbouring RD.",
                    }),
                ],
                composite: None,
            }),
            Item::Optional(Group {
                name: "time_to_next",
                doc: "Time until the indicated RD's cluster beacon period starts \
                      (microseconds).",
                items: &[Item::Field(Field {
                    name: "time_to_next",
                    fig: Some("Time To Next"),
                    bits: 32,
                    ty: Ty::Raw,
                    doc: "Time to next (microseconds).",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "rssi_2",
                doc: "RSSI-2 measurement result.",
                items: &[Item::Field(Field {
                    name: "rssi_2",
                    fig: Some("RSSI-2"),
                    bits: 8,
                    ty: Ty::Wrap {
                        ty: "Rssi2Measurement",
                        construct: "Rssi2Measurement({})",
                        deconstruct: "{}.0",
                    },
                    doc: "RSSI-2 measurement result.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "snr",
                doc: "SNR measurement result.",
                items: &[Item::Field(Field {
                    name: "snr",
                    fig: Some("SNR"),
                    bits: 8,
                    ty: Ty::Wrap {
                        ty: "SnrMeasurement",
                        construct: "SnrMeasurement({})",
                        deconstruct: "{}.0",
                    },
                    doc: "SNR measurement result.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "radio_device_class",
                doc: "Radio Device Class (mu / beta) of the neighbouring RD.",
                items: &[
                    Item::Field(Field {
                        name: "mu",
                        fig: Some("mu"),
                        bits: 3,
                        ty: Ty::Fallible {
                            ty: "RdClassMu",
                            ctor: "try_from_u8",
                            getter: "as_u8",
                        },
                        doc: "Radio Device Class mu.",
                    }),
                    Item::Field(Field {
                        name: "beta",
                        fig: Some("beta"),
                        bits: 4,
                        ty: Ty::Fallible {
                            ty: "RdClassBeta",
                            ctor: "try_from_u8",
                            getter: "as_u8",
                        },
                        doc: "Radio Device Class beta.",
                    }),
                    Item::Reserved { bits: 1 },
                ],
                composite: Some(Composite {
                    ty: "RadioDeviceClass",
                    construct: "RadioDeviceClass { mu, beta }",
                    accessors: &["v.mu", "v.beta"],
                }),
            }),
        ],
    }
}
