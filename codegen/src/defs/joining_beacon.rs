//! Joining Beacon message: ETSI TS 103 636-4, clause 6.4.2.10,
//! Figure 6.4.2.10-1.

use crate::ir::{Field, Item, MessageDef, Repeat, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "joining_beacon",
        name: "JoiningBeaconParts",
        spec: "ETSI TS 103 636-4, clause 6.4.2.10, Figure 6.4.2.10-1",
        doc: "Owned representation of a Joining Beacon body. Always 1..=4 channels.",
        ie_type: Some("JoiningBeacon"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Count {
                of: "channels",
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
                doc: "Network beacon period code (4 bits, see Table 6.4.2.2-1).",
            }),
            Item::Reserved { bits: 2 },
            Item::Repeat(Repeat {
                name: "channels",
                doc: "1..=4 Network Beacon channels.",
                items: &[
                    Item::Reserved { bits: 3 },
                    Item::Field(Field {
                        name: "channel",
                        fig: Some("Channel"),
                        bits: 13,
                        ty: Ty::Fallible {
                            ty: "AbsoluteChannel",
                            ctor: "try_from_u16",
                            getter: "as_u16",
                        },
                        doc: "Network Beacon channel.",
                    }),
                ],
                composite: None,
                bias: 1,
                reserved_max: false,
                all_escape: None,
                max_const: "MAX_CHANNELS",
                max_doc: "Maximum number of Joining Beacon channels (on-wire 2-bit count + 1 = 4).",
            }),
        ],
    }
}
