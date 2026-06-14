//! Association Response message: ETSI TS 103 636-4, clause 6.4.2.5,
//! Figure 6.4.2.5-1 / Tables 6.4.2.5-1/-2.
//!
//! Enum-shaped body: the ACK/NACK bit selects Reject (2 bytes) or
//! Accept (bitmap + optional HARQ override / flow list / group).

use crate::ir::{
    AllEscape, Composite, Field, Group, Item, MessageDef, Repeat, Ty, VArm, VariantBody,
};

pub fn def() -> MessageDef {
    MessageDef {
        module: "association_response",
        name: "AssociationResponseParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.5, Figure 6.4.2.5-1, Tables 6.4.2.5-1/-2",
        doc: "Owned representation of an Association Response body.",
        ie_type: Some("AssociationResponse"),
        short_ie: None,
        imports: &[
            "use crate::mac::messages::association_response::AssociationAcceptParts;",
            "use crate::mac::messages::association_response::AssociationResponseParts;",
            "use crate::mac::messages::association_response::FlowAcceptance;",
            "use crate::mac::messages::association_response::GroupAssignment;",
            "use crate::mac::messages::association_response::HarqOverride;",
        ],
        ctx: &[],
        field_groups: &[],
        items: &[Item::VariantBody(VariantBody {
            name: "body",
            ty: "AssociationResponseParts",
            doc: "Reject / Accept split (unused: whole-enum form).",
            bits: 1,
            fig: "ACK",
            arms: &[
                VArm {
                    value: 0,
                    pattern: "AssociationResponseParts::Reject { cause, timer }",
                    len_pattern: "AssociationResponseParts::Reject { .. }",
                    binds: &[("cause", "cause"), ("timer", "timer")],
                    construct: "AssociationResponseParts::Reject { cause, timer }",
                    items: &[
                        Item::Const {
                            bits: 1,
                            value: 0,
                            fig: "ACK",
                        },
                        // The rest of byte 0 is don't-care on reject.
                        Item::Reserved { bits: 7 },
                        Item::Field(Field {
                            name: "cause",
                            fig: Some("Reject Cause"),
                            bits: 4,
                            ty: Ty::Fallible {
                                ty: "RejectCause",
                                ctor: "try_from_u8",
                                getter: "as_u8",
                            },
                            doc: "Reject cause (Table 6.4.2.5-1).",
                        }),
                        Item::Field(Field {
                            name: "timer",
                            fig: Some("Reject Timer"),
                            bits: 4,
                            ty: Ty::Fallible {
                                ty: "RejectTimer",
                                ctor: "try_from_u8",
                                getter: "as_u8",
                            },
                            doc: "Reject timer (Table 6.4.2.5-2).",
                        }),
                    ],
                },
                VArm {
                    value: 1,
                    pattern: "AssociationResponseParts::Accept(a)",
                    len_pattern: "AssociationResponseParts::Accept(a)",
                    binds: &[
                        ("harq_override", "a.harq_override"),
                        ("flow_acceptance", "a.flow_acceptance"),
                        ("group", "a.group"),
                    ],
                    construct: "AssociationResponseParts::Accept(AssociationAcceptParts { flow_acceptance, harq_override, group })",
                    items: &[
                        Item::Const {
                            bits: 1,
                            value: 1,
                            fig: "ACK",
                        },
                        Item::Reserved { bits: 1 },
                        Item::PresenceFlag {
                            of: "harq_override",
                            fig: "HM",
                        },
                        Item::Count {
                            of: "flow_acceptance",
                            bits: 3,
                            fig: "N",
                        },
                        Item::PresenceFlag {
                            of: "group",
                            fig: "G",
                        },
                        Item::Reserved { bits: 1 },
                        Item::Optional(Group {
                            name: "harq_override",
                            doc: "`Some(..)` iff the FT is overriding HARQ configuration.",
                            items: &[
                                Item::Field(Field {
                                    name: "harq_processes_rx",
                                    fig: Some("HARQ RX"),
                                    bits: 3,
                                    ty: Ty::Fallible {
                                        ty: "HarqProcesses",
                                        ctor: "try_from_u8",
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
                                        ctor: "try_from_u8",
                                        getter: "as_u8",
                                    },
                                    doc: "Maximum HARQ re-receptions.",
                                }),
                                Item::Field(Field {
                                    name: "harq_processes_tx",
                                    fig: Some("HARQ TX"),
                                    bits: 3,
                                    ty: Ty::Fallible {
                                        ty: "HarqProcesses",
                                        ctor: "try_from_u8",
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
                                        ctor: "try_from_u8",
                                        getter: "as_u8",
                                    },
                                    doc: "Maximum HARQ retransmissions.",
                                }),
                            ],
                            composite: Some(Composite {
                                ty: "HarqOverride",
                                construct: "HarqOverride { harq_processes_rx, max_harq_re_rx, harq_processes_tx, max_harq_re_tx }",
                                accessors: &[
                                    "v.harq_processes_rx",
                                    "v.max_harq_re_rx",
                                    "v.harq_processes_tx",
                                    "v.max_harq_re_tx",
                                ],
                            }),
                        }),
                        Item::Repeat(Repeat {
                            name: "flow_acceptance",
                            doc: "Which flows the FT is accepting.",
                            items: &[
                                Item::Reserved { bits: 2 },
                                Item::Field(Field {
                                    name: "flow_id",
                                    fig: Some("Flow ID"),
                                    bits: 6,
                                    ty: Ty::Fallible {
                                        ty: "FlowId",
                                        ctor: "try_from_u8",
                                        getter: "as_u8",
                                    },
                                    doc: "Accepted flow ID.",
                                }),
                            ],
                            composite: None,
                            bias: 0,
                            reserved_max: false,
                            all_escape: Some(AllEscape {
                                ty: "FlowAcceptance",
                                all: "FlowAcceptance::All",
                                specific: "FlowAcceptance::Specific",
                            }),
                            max_const: "MAX_RESPONSE_FLOWS",
                            max_doc: "Maximum number of flow IDs in a `FlowAcceptance::Specific` \
                                      list. The on-wire 3-bit Number of Flows field caps at 6; \
                                      `0b111` is the `All` encoding (no Flow ID octets follow).",
                        }),
                        Item::Optional(Group {
                            name: "group",
                            doc: "`Some(..)` iff the on-wire Group bit = 1.",
                            items: &[
                                Item::Reserved { bits: 1 },
                                Item::Field(Field {
                                    name: "group_id",
                                    fig: Some("Group ID"),
                                    bits: 7,
                                    ty: Ty::Fallible {
                                        ty: "GroupId",
                                        ctor: "try_from_u8",
                                        getter: "as_u8",
                                    },
                                    doc: "Assigned group ID.",
                                }),
                                Item::Reserved { bits: 1 },
                                Item::Field(Field {
                                    name: "resource_tag",
                                    fig: Some("Resource Tag"),
                                    bits: 7,
                                    ty: Ty::Fallible {
                                        ty: "ResourceTag",
                                        ctor: "try_from_u8",
                                        getter: "as_u8",
                                    },
                                    doc: "Assigned resource tag.",
                                }),
                            ],
                            composite: Some(Composite {
                                ty: "GroupAssignment",
                                construct: "GroupAssignment { group_id, resource_tag }",
                                accessors: &["v.group_id", "v.resource_tag"],
                            }),
                        }),
                    ],
                },
            ],
        })],
    }
}
