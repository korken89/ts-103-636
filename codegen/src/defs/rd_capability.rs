//! RD Capability IE (long form): ETSI TS 103 636-4, clause 6.4.3.5,
//! Figure 6.4.3.5-1 / Table 6.4.3.5-1.
//!
//! A 7-octet fixed part (3 header octets + 4 capability octets),
//! followed by 0..=7 additional 5-octet PHY capability blocks. The
//! 4-octet capability core repeats in each block (with the D_Delay /
//! HalfDup flags reserved there: those two are RD-wide and live only
//! in the fixed part).

use crate::ir::{Composite, Field, FieldGroup, Item, MessageDef, Repeat, Ty};

/// The 4-octet PHY capability core (rows 4..=7 / 9..=12 of the
/// figure), minus the D_Delay / HalfDup bit positions which differ
/// between the fixed part and the additional blocks.
const fn core_fields() -> [Item; 8] {
    [
        Item::Field(Field {
            name: "rd_power_class",
            fig: Some("Power Class"),
            bits: 3,
            ty: Ty::Fallible {
                ty: "RdPowerClass",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "RD power class.",
        }),
        Item::Field(Field {
            name: "max_nss_for_rx",
            fig: Some("NSS RX"),
            bits: 2,
            ty: Ty::Fallible {
                ty: "Nss",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "Maximum number of spatial streams for reception.",
        }),
        Item::Field(Field {
            name: "rx_for_tx_diversity",
            fig: Some("RXTX Div"),
            bits: 2,
            ty: Ty::Fallible {
                ty: "Nss",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "RX antennas usable for TX diversity.",
        }),
        Item::Field(Field {
            name: "rx_gain",
            fig: Some("RX Gain"),
            bits: 4,
            ty: Ty::Fallible {
                ty: "RxGain",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "Receiver gain code.",
        }),
        Item::Field(Field {
            name: "max_mcs",
            fig: Some("MAX MCS"),
            bits: 4,
            ty: Ty::Fallible {
                ty: "Mcs",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "Maximum supported MCS.",
        }),
        Item::Field(Field {
            name: "soft_buffer_size",
            fig: Some("Soft Buffer"),
            bits: 4,
            ty: Ty::Fallible {
                ty: "SoftBufferSize",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "HARQ soft buffer size code.",
        }),
        Item::Field(Field {
            name: "num_harq_processes",
            fig: Some("HARQ Proc"),
            bits: 2,
            ty: Ty::Fallible {
                ty: "NumHarqProcesses",
                ctor: "try_from_u8",
                getter: "as_u8",
            },
            doc: "Number of HARQ processes code.",
        }),
        Item::Field(Field {
            name: "harq_feedback_delay",
            fig: Some("HARQ Delay"),
            bits: 4,
            ty: Ty::Fallible {
                ty: "HarqFeedbackDelay",
                ctor: "try_from_u8",
                getter: "subslots",
            },
            doc: "HARQ feedback delay in subslots.",
        }),
    ]
}

const CORE: [Item; 8] = core_fields();

pub fn def() -> MessageDef {
    MessageDef {
        module: "rd_capability",
        name: "RdCapabilityParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.5, Figure 6.4.3.5-1, Table 6.4.3.5-1",
        doc: "Owned representation of an RD Capability IE body.",
        ie_type: Some("RdCapability"),
        short_ie: None,
        imports: &[
            "use crate::mac::messages::rd_capability::AdditionalPhyCapability;",
            "use crate::mac::messages::rd_capability::PhyCapability;",
        ],
        ctx: &[],
        field_groups: &[FieldGroup {
            name: "base_phy",
            ty: "PhyCapability",
            doc: "Capability core of the fixed part. Applies to the numerology \
                  already in use on the connection (the fixed part carries no \
                  mu/beta on the wire).",
            leaves: &[
                "rd_power_class",
                "max_nss_for_rx",
                "rx_for_tx_diversity",
                "rx_gain",
                "max_mcs",
                "soft_buffer_size",
                "num_harq_processes",
                "harq_feedback_delay",
            ],
            construct: "PhyCapability { rd_power_class, max_nss_for_rx, rx_for_tx_diversity, \
                        rx_gain, max_mcs, soft_buffer_size, num_harq_processes, \
                        harq_feedback_delay }",
            accessors: &[
                "self.base_phy.rd_power_class",
                "self.base_phy.max_nss_for_rx",
                "self.base_phy.rx_for_tx_diversity",
                "self.base_phy.rx_gain",
                "self.base_phy.max_mcs",
                "self.base_phy.soft_buffer_size",
                "self.base_phy.num_harq_processes",
                "self.base_phy.harq_feedback_delay",
            ],
        }],
        items: &[
            Item::Count {
                of: "additional_phy",
                bits: 3,
                fig: "N",
            },
            Item::Field(Field {
                name: "release",
                fig: Some("Release"),
                bits: 5,
                ty: Ty::Fallible {
                    ty: "Release",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "DECT-2020 release.",
            }),
            Item::Reserved { bits: 2 },
            Item::Field(Field {
                name: "group_as",
                fig: Some("GA"),
                bits: 1,
                ty: Ty::Bool,
                doc: "Group assignment support.",
            }),
            Item::Field(Field {
                name: "paging",
                fig: Some("PG"),
                bits: 1,
                ty: Ty::Bool,
                doc: "Paging support.",
            }),
            Item::Field(Field {
                name: "operating_modes",
                fig: Some("OM"),
                bits: 2,
                ty: Ty::Fallible {
                    ty: "OperatingModes",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Supported operating modes (PT / FT / both).",
            }),
            Item::Field(Field {
                name: "mesh",
                fig: Some("M"),
                bits: 1,
                ty: Ty::Bool,
                doc: "Mesh system operation support.",
            }),
            Item::Field(Field {
                name: "schedul",
                fig: Some("S"),
                bits: 1,
                ty: Ty::Bool,
                doc: "Scheduled access support.",
            }),
            Item::Field(Field {
                name: "mac_security",
                fig: Some("Security"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "MacSecuritySupport",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "MAC security support.",
            }),
            Item::Field(Field {
                name: "dlc_service_type",
                fig: Some("DLC Type"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "DlcServiceType",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "Supported DLC service types.",
            }),
            Item::Reserved { bits: 2 },
            // Bytes 3..=6: capability core plus the RD-wide flags.
            Item::Reserved { bits: 1 },
            CORE[0],
            CORE[1],
            CORE[2],
            CORE[3],
            CORE[4],
            CORE[5],
            CORE[6],
            Item::Reserved { bits: 2 },
            CORE[7],
            Item::Field(Field {
                name: "d_delay",
                fig: Some("DD"),
                bits: 1,
                ty: Ty::Bool,
                doc: "RD-wide `DECT_Delay` capability (encoded in the fixed \
                      part's HARQ feedback delay octet).",
            }),
            Item::Field(Field {
                name: "half_dup",
                fig: Some("HD"),
                bits: 1,
                ty: Ty::Bool,
                doc: "RD-wide `HalfDup` capability (encoded in the fixed \
                      part's HARQ feedback delay octet).",
            }),
            Item::Reserved { bits: 2 },
            Item::Repeat(Repeat {
                name: "additional_phy",
                doc: "Up to [`MAX_ADDITIONAL_PHY`] additional PHY capability blocks.",
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
                    Item::Reserved { bits: 1 },
                    CORE[0],
                    CORE[1],
                    CORE[2],
                    CORE[3],
                    CORE[4],
                    CORE[5],
                    CORE[6],
                    Item::Reserved { bits: 2 },
                    CORE[7],
                    // D_Delay / HalfDup are reserved in additional blocks.
                    Item::Reserved { bits: 4 },
                ],
                composite: Some(Composite {
                    ty: "AdditionalPhyCapability",
                    construct: "AdditionalPhyCapability { mu, beta, phy: PhyCapability { \
                                rd_power_class, max_nss_for_rx, rx_for_tx_diversity, rx_gain, \
                                max_mcs, soft_buffer_size, num_harq_processes, \
                                harq_feedback_delay } }",
                    accessors: &[
                        "v.mu",
                        "v.beta",
                        "v.phy.rd_power_class",
                        "v.phy.max_nss_for_rx",
                        "v.phy.rx_for_tx_diversity",
                        "v.phy.rx_gain",
                        "v.phy.max_mcs",
                        "v.phy.soft_buffer_size",
                        "v.phy.num_harq_processes",
                        "v.phy.harq_feedback_delay",
                    ],
                }),
                bias: 0,
                reserved_max: false,
                all_escape: None,
                max_const: "MAX_ADDITIONAL_PHY",
                max_doc: "Maximum number of additional PHY capability blocks \
                          (3-bit count field).",
            }),
        ],
    }
}
