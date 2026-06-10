//! Resource Allocation IE: ETSI TS 103 636-4, clause 6.4.3.3,
//! Figure 6.4.3.3-1 / Tables 6.4.3.3-1/-2.
//!
//! Enum-shaped body: the 2-bit Allocation Type selects ReleaseAll
//! (1 byte), Downlink / Uplink (one pair) or Both (two pairs); the
//! non-trivial arms share the bitmap and optional tail.

use crate::ir::{
    Composite, Ctx, Field, Group, Item, MessageDef, ModeTag, Switch, Ty, VArm, VariantBody, Wide,
};

/// Bitmap bits shared by the Downlink / Uplink / Both arms (after
/// the Allocation Type) plus the second flag byte.
const BITMAP: [Item; 7] = [
    Item::Field(Field {
        name: "add",
        fig: Some("A"),
        bits: 1,
        ty: Ty::Bool,
        doc: "On-wire `Add` bit. `false` = new/replace, `true` = additional.",
    }),
    Item::PresenceFlag {
        of: "recipient",
        fig: "ID",
    },
    Item::ModeTag(ModeTag {
        of: "repeat",
        bits: 3,
        fig: "RPT",
        ty: "RepeatMode",
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
        of: "resource_failure_timer",
        fig: "RLF",
    },
    Item::Reserved { bits: 6 },
];

/// One (start subslot, length type, length) pair with the
/// mu-dependent 8/9-bit start subslot.
const fn pair(name: &'static str, accessors: &'static [&'static str]) -> Item {
    Item::Inline(Group {
        name,
        doc: "Allocation pair (start subslot + length).",
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
            accessors,
        }),
    })
}

/// Optional tail shared by the non-trivial arms.
const OPTIONS: [Item; 5] = [
    Item::Optional(Group {
        name: "recipient",
        doc: "On-wire `ID` bit payload.",
        items: &[Item::Field(Field {
            name: "recipient",
            fig: Some("Short RD ID"),
            bits: 16,
            ty: Ty::Fallible {
                ty: "ShortRdId",
                ctor: "new",
                getter: "as_u16",
            },
            doc: "Recipient of a beacon-carried allocation.",
        })],
        composite: None,
    }),
    Item::Optional(Group {
        name: "repeat",
        doc: "On-wire `Repeat` field payload.",
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
            ty: "RepeatPolicy",
            construct: "RepeatPolicy { mode, repetition, validity }",
            accessors: &["v.repetition", "v.validity"],
        }),
    }),
    Item::Optional(Group {
        name: "sfn_value",
        doc: "On-wire `SFN` bit payload.",
        items: &[Item::Field(Field {
            name: "sfn_value",
            fig: Some("SFN Value"),
            bits: 8,
            ty: Ty::Raw,
            doc: "SFN from which the allocation is valid.",
        })],
        composite: None,
    }),
    Item::Optional(Group {
        name: "channel",
        doc: "On-wire `Channel` bit payload.",
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
                doc: "Channel override for the allocation.",
            }),
        ],
        composite: None,
    }),
    Item::Optional(Group {
        name: "resource_failure_timer",
        doc: "On-wire `RLF` bit payload.",
        items: &[
            Item::Reserved { bits: 4 },
            Item::Field(Field {
                name: "resource_failure_timer",
                fig: Some("RLF Timer"),
                bits: 4,
                ty: Ty::Fallible {
                    ty: "DectScheduledResourceFailure",
                    ctor: "try_from_u8",
                    getter: "as_u8",
                },
                doc: "`dectScheduledResourceFailure` timer override.",
            }),
        ],
        composite: None,
    }),
];

const OPTIONS_BINDS: [(&str, &str); 6] = [
    ("add", "options.add"),
    ("recipient", "options.recipient"),
    ("repeat", "options.repeat"),
    ("sfn_value", "options.sfn_value"),
    ("channel", "options.channel"),
    ("resource_failure_timer", "options.resource_failure_timer"),
];

const PAIR_SINGLE: Item = pair(
    "pair",
    &[
        "self.pair.start_subslot",
        "self.pair.length_type",
        "self.pair.length",
    ],
);
const PAIR_DL: Item = pair(
    "dl",
    &["self.dl.start_subslot", "self.dl.length_type", "self.dl.length"],
);
const PAIR_UL: Item = pair(
    "ul",
    &["self.ul.start_subslot", "self.ul.length_type", "self.ul.length"],
);

pub fn def() -> MessageDef {
    MessageDef {
        module: "resource_allocation",
        name: "ResourceAllocationParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.3, Figure 6.4.3.3-1, Tables 6.4.3.3-1/-2",
        doc: "Owned representation of a Resource Allocation IE body. Wraps a \
              [`ResourceAllocationKind`] together with the PHY subcarrier scaling \
              factor [`Mu`] that determines the wire encoding of the start subslot \
              (8 vs. 9 bit). Bundling `mu` into the body removes it from the \
              `encoded_len` / `serialize` / `parse` signatures and lets this type \
              implement [`crate::mac::pdu::MessageBody`].",
        ie_type: Some("ResourceAllocation"),
        short_ie: None,
        imports: &[
            "use crate::mac::messages::common::AllocationPair;",
            "use crate::mac::messages::resource_allocation::AllocationOptions;",
            "use crate::mac::messages::resource_allocation::RepeatPolicy;",
            "use crate::mac::messages::resource_allocation::ResourceAllocationKind;",
        ],
        ctx: &[Ctx {
            name: "mu",
            ty: "Mu",
            doc: "PHY subcarrier scaling factor that determines the on-wire start \
                  subslot width.",
        }],
        field_groups: &[],
        items: &[Item::VariantBody(VariantBody {
            name: "kind",
            ty: "ResourceAllocationKind",
            doc: "Allocation-type-specific body.",
            bits: 2,
            fig: "AT",
            arms: &[
                VArm {
                    value: 0,
                    pattern: "ResourceAllocationKind::ReleaseAll",
                    len_pattern: "ResourceAllocationKind::ReleaseAll",
                    binds: &[],
                    construct: "ResourceAllocationKind::ReleaseAll",
                    items: &[
                        Item::Const {
                            bits: 2,
                            value: 0,
                            fig: "AT",
                        },
                        Item::Reserved { bits: 6 },
                    ],
                },
                VArm {
                    value: 1,
                    pattern: "ResourceAllocationKind::Downlink { pair, options }",
                    len_pattern: "ResourceAllocationKind::Downlink { options, .. }",
                    binds: &[
                        ("pair", "pair"),
                        OPTIONS_BINDS[0],
                        OPTIONS_BINDS[1],
                        OPTIONS_BINDS[2],
                        OPTIONS_BINDS[3],
                        OPTIONS_BINDS[4],
                        OPTIONS_BINDS[5],
                    ],
                    construct: "ResourceAllocationKind::Downlink { pair, options: AllocationOptions { add, recipient, repeat, sfn_value, channel, resource_failure_timer } }",
                    items: &[
                        Item::Const {
                            bits: 2,
                            value: 1,
                            fig: "AT",
                        },
                        BITMAP[0],
                        BITMAP[1],
                        BITMAP[2],
                        BITMAP[3],
                        BITMAP[4],
                        BITMAP[5],
                        BITMAP[6],
                        PAIR_SINGLE,
                        OPTIONS[0],
                        OPTIONS[1],
                        OPTIONS[2],
                        OPTIONS[3],
                        OPTIONS[4],
                    ],
                },
                VArm {
                    value: 2,
                    pattern: "ResourceAllocationKind::Uplink { pair, options }",
                    len_pattern: "ResourceAllocationKind::Uplink { options, .. }",
                    binds: &[
                        ("pair", "pair"),
                        OPTIONS_BINDS[0],
                        OPTIONS_BINDS[1],
                        OPTIONS_BINDS[2],
                        OPTIONS_BINDS[3],
                        OPTIONS_BINDS[4],
                        OPTIONS_BINDS[5],
                    ],
                    construct: "ResourceAllocationKind::Uplink { pair, options: AllocationOptions { add, recipient, repeat, sfn_value, channel, resource_failure_timer } }",
                    items: &[
                        Item::Const {
                            bits: 2,
                            value: 2,
                            fig: "AT",
                        },
                        BITMAP[0],
                        BITMAP[1],
                        BITMAP[2],
                        BITMAP[3],
                        BITMAP[4],
                        BITMAP[5],
                        BITMAP[6],
                        PAIR_SINGLE,
                        OPTIONS[0],
                        OPTIONS[1],
                        OPTIONS[2],
                        OPTIONS[3],
                        OPTIONS[4],
                    ],
                },
                VArm {
                    value: 3,
                    pattern: "ResourceAllocationKind::Both { dl, ul, options }",
                    len_pattern: "ResourceAllocationKind::Both { options, .. }",
                    binds: &[
                        ("dl", "dl"),
                        ("ul", "ul"),
                        OPTIONS_BINDS[0],
                        OPTIONS_BINDS[1],
                        OPTIONS_BINDS[2],
                        OPTIONS_BINDS[3],
                        OPTIONS_BINDS[4],
                        OPTIONS_BINDS[5],
                    ],
                    construct: "ResourceAllocationKind::Both { dl, ul, options: AllocationOptions { add, recipient, repeat, sfn_value, channel, resource_failure_timer } }",
                    items: &[
                        Item::Const {
                            bits: 2,
                            value: 3,
                            fig: "AT",
                        },
                        BITMAP[0],
                        BITMAP[1],
                        BITMAP[2],
                        BITMAP[3],
                        BITMAP[4],
                        BITMAP[5],
                        BITMAP[6],
                        PAIR_DL,
                        PAIR_UL,
                        OPTIONS[0],
                        OPTIONS[1],
                        OPTIONS[2],
                        OPTIONS[3],
                        OPTIONS[4],
                    ],
                },
            ],
        })],
    }
}
