//! Random Access Resource IE: ETSI TS 103 636-4, clause 6.4.3.4,
//! Figure 6.4.3.4-1 / Table 6.4.3.4-1.

use crate::ir::{Composite, Ctx, Field, Group, Item, MessageDef, ModeTag, Switch, Ty, Wide};

pub fn def() -> MessageDef {
    MessageDef {
        module: "random_access_resource",
        name: "RandomAccessResourceParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.4, Figure 6.4.3.4-1, Table 6.4.3.4-1",
        doc: "Owned representation of a Random Access Resource IE body. Carries \
              the PHY subcarrier scaling factor [`Mu`] alongside the body fields \
              so that [`Self::encoded_len`] and [`Self::serialize`] don't need it \
              as a parameter, which lets this type implement \
              [`crate::mac::pdu::MessageBody`].",
        ie_type: Some("RandomAccessResource"),
        short_ie: None,
        imports: &[
            "use crate::mac::messages::common::AllocationPair;",
            "use crate::mac::messages::random_access_resource::RachRepeatPolicy;",
        ],
        ctx: &[Ctx {
            name: "mu",
            ty: "Mu",
            doc: "PHY subcarrier scaling factor that determines the on-wire start \
                  subslot width.",
        }],
        field_groups: &[],
        items: &[
            // Byte 0: bitmap.
            Item::Reserved { bits: 3 },
            Item::ModeTag(ModeTag {
                of: "repeat",
                bits: 2,
                fig: "RPT",
                ty: "RachRepeatMode",
                ctor: "try_from_u8",
                accessor: "mode.as_u8()",
            }),
            Item::PresenceFlag {
                of: "sfn_value",
                fig: "SFN",
            },
            Item::PresenceFlag {
                of: "channel",
                fig: "CH",
            },
            Item::PresenceFlag {
                of: "channel_2",
                fig: "CH2",
            },
            // Start subslot + length pair (mu-dependent width).
            Item::Inline(Group {
                name: "pair",
                doc: "Start subslot + Length type + Length (same encoding as a \
                      Resource Allocation pair). Start subslot is 8-bit or 9-bit \
                      depending on [`Self::mu`].",
                items: &[
                    Item::Switch(Switch {
                        field: Field {
                            name: "start_subslot",
                            fig: Some("Start Subslot"),
                            bits: 16,
                            ty: Ty::Raw,
                            doc: "First subslot of the allocation.",
                        },
                        narrow_bits: 8,
                        wide_bits: 9,
                        wide: Wide::Ctx("{mu}.as_u8() > 4"),
                    }),
                    Item::Field(Field {
                        name: "length_type",
                        fig: Some("LT"),
                        bits: 1,
                        ty: Ty::Wrap {
                            ty: "PacketLengthType",
                            construct: "if {} == 0 { PacketLengthType::Subslot } else { PacketLengthType::Slot }",
                            deconstruct: "matches!({}, PacketLengthType::Slot) as u8",
                        },
                        doc: "Length type of the allocation.",
                    }),
                    Item::Field(Field {
                        name: "length",
                        fig: Some("Length"),
                        bits: 7,
                        ty: Ty::Fallible {
                            ty: "RaLength",
                            ctor: "new",
                            getter: "as_u8",
                        },
                        doc: "Allocation length.",
                    }),
                ],
                composite: Some(Composite {
                    ty: "AllocationPair",
                    construct: "AllocationPair { start_subslot, length_type, length }",
                    accessors: &[
                        "self.pair.start_subslot",
                        "self.pair.length_type",
                        "self.pair.length",
                    ],
                }),
            }),
            // MAX RACH length byte.
            Item::Field(Field {
                name: "max_length_type",
                fig: Some("MLT"),
                bits: 1,
                ty: Ty::Wrap {
                    ty: "PacketLengthType",
                    construct: "if {} == 0 { PacketLengthType::Subslot } else { PacketLengthType::Slot }",
                    deconstruct: "matches!({}, PacketLengthType::Slot) as u8",
                },
                doc: "Length type of the MAX RACH length.",
            }),
            Item::Field(Field {
                name: "max_rach_length",
                fig: Some("MAX RACH Len"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "MaxRachLength",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Maximum RACH transmission length.",
            }),
            Item::Field(Field {
                name: "cwmin_sig",
                fig: Some("CWmin"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "Cwsig",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Minimum contention window exponent.",
            }),
            // DECT delay byte.
            Item::Field(Field {
                name: "dect_delay",
                fig: Some("DD"),
                bits: 1,
                ty: Ty::Bool,
                doc: "On-wire `DECT_Delay` bit (1 bit). `false` = response window \
                      starts from the subslot `n + HARQ feedback delay + 1`; \
                      `true` = response window starts 0.5 frames after the start \
                      of the Random Access transmission.",
            }),
            Item::Field(Field {
                name: "response_window",
                fig: Some("Resp Window"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "ResponseWindow",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Response window length code.",
            }),
            Item::Field(Field {
                name: "cwmax_sig",
                fig: Some("CWmax"),
                bits: 3,
                ty: Ty::Fallible {
                    ty: "Cwsig",
                    ctor: "new",
                    getter: "as_u8",
                },
                doc: "Maximum contention window exponent.",
            }),
            // Optional regions.
            Item::Optional(Group {
                name: "repeat",
                doc: "`Some(..)` iff the on-wire Repeat field is 0b01 or 0b10.",
                items: &[
                    Item::Field(Field {
                        name: "repetition",
                        fig: Some("Repetition"),
                        bits: 8,
                        ty: Ty::Fallible {
                            ty: "Repetition",
                            ctor: "new",
                            getter: "as_u8",
                        },
                        doc: "Repetition interval.",
                    }),
                    Item::Field(Field {
                        name: "validity",
                        fig: Some("Validity"),
                        bits: 8,
                        ty: Ty::Wrap {
                            ty: "Validity",
                            construct: "Validity({})",
                            deconstruct: "{}.0",
                        },
                        doc: "Validity time.",
                    }),
                ],
                composite: Some(Composite {
                    ty: "RachRepeatPolicy",
                    construct: "RachRepeatPolicy { mode, repetition, validity }",
                    accessors: &["v.repetition", "v.validity"],
                }),
            }),
            Item::Optional(Group {
                name: "sfn_value",
                doc: "`Some(..)` iff the SFN bit is set.",
                items: &[Item::Field(Field {
                    name: "sfn_value",
                    fig: Some("SFN Value"),
                    bits: 8,
                    ty: Ty::Raw,
                    doc: "SFN from which the resource is valid.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "channel",
                doc: "`Some(..)` iff the Channel bit is set.",
                items: &[
                    Item::Reserved { bits: 3 },
                    Item::Field(Field {
                        name: "channel",
                        fig: Some("Channel"),
                        bits: 13,
                        ty: Ty::Fallible {
                            ty: "AbsoluteChannel",
                            ctor: "new",
                            getter: "as_u16",
                        },
                        doc: "Channel of the random access resource.",
                    }),
                ],
                composite: None,
            }),
            Item::Optional(Group {
                name: "channel_2",
                doc: "`Some(..)` iff the Chan_2 bit is set. Indicates the channel \
                      for the random access response message when it differs from \
                      the channel where the IE was received.",
                items: &[
                    Item::Reserved { bits: 3 },
                    Item::Field(Field {
                        name: "channel_2",
                        fig: Some("Channel 2"),
                        bits: 13,
                        ty: Ty::Fallible {
                            ty: "AbsoluteChannel",
                            ctor: "new",
                            getter: "as_u16",
                        },
                        doc: "Channel for the random access response.",
                    }),
                ],
                composite: None,
            }),
        ],
    }
}
