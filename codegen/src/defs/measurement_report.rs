//! Measurement Report IE: ETSI TS 103 636-4, clause 6.4.3.12,
//! Figure 6.4.3.12-1 / Table 6.4.3.12-1.

use crate::ir::{Field, Group, Item, MessageDef, Ty};

pub fn def() -> MessageDef {
    MessageDef {
        module: "measurement_report",
        name: "MeasurementReportParts",
        spec: "ETSI TS 103 636-4, clause 6.4.3.12, Figure 6.4.3.12-1, Table 6.4.3.12-1",
        doc: "Owned representation of a Measurement Report IE body.",
        ie_type: Some("MeasurementReport"),
        short_ie: None,
        imports: &[],
        ctx: &[],
        field_groups: &[],
        items: &[
            Item::Reserved { bits: 3 },
            Item::PresenceFlag {
                of: "snr",
                fig: "SNR",
            },
            Item::PresenceFlag {
                of: "rssi_2",
                fig: "R2",
            },
            Item::PresenceFlag {
                of: "rssi_1",
                fig: "R1",
            },
            Item::PresenceFlag {
                of: "tx_count",
                fig: "TX",
            },
            Item::Field(Field {
                name: "from_rach",
                fig: Some("RA"),
                bits: 1,
                ty: Ty::Bool,
                doc: "RACH bit. `false` = measurement from scheduled resources; \
                      `true` = measurement from Random Access response.",
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
                name: "rssi_1",
                doc: "RSSI-1 measurement result.",
                items: &[Item::Field(Field {
                    name: "rssi_1",
                    fig: Some("RSSI-1"),
                    bits: 8,
                    ty: Ty::Wrap {
                        ty: "Rssi1Measurement",
                        construct: "Rssi1Measurement({})",
                        deconstruct: "{}.0",
                    },
                    doc: "RSSI-1 measurement result.",
                })],
                composite: None,
            }),
            Item::Optional(Group {
                name: "tx_count",
                doc: "Number of transmitted packets since the last report.",
                items: &[Item::Field(Field {
                    name: "tx_count",
                    fig: Some("TX Count"),
                    bits: 8,
                    ty: Ty::Raw,
                    doc: "Number of transmitted packets since the last report.",
                })],
                composite: None,
            }),
        ],
    }
}
