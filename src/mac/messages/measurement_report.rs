//! Measurement Report IE body (generated codec re-export).
//!
//! The codec lives in [`generated::measurement_report`](super::generated::measurement_report); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::measurement_report::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    #[test]
    fn measurement_report_round_trip() {
        let parts = MeasurementReportParts {
            snr: Some(SnrMeasurement(0x10)),
            rssi_2: Some(Rssi2Measurement(0x20)),
            rssi_1: None,
            tx_count: Some(0xFF),
            from_rach: true,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4);
        let parsed = MeasurementReportParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.snr.unwrap().0, 0x10);
        assert_eq!(parsed.rssi_2.unwrap().0, 0x20);
        assert!(parsed.rssi_1.is_none());
        assert_eq!(parsed.tx_count, Some(0xFF));
        assert!(parsed.from_rach);
    }

    #[test]
    fn measurement_report_rejects_empty_buffer() {
        assert!(MeasurementReportParts::parse(&[]).is_err());
    }

    #[test]
    fn measurement_report_rejects_truncated_payload() {
        // B0 sets the SNR present bit but no following byte is supplied.
        let buf = [0x10u8];
        assert!(MeasurementReportParts::parse(&buf).is_err());
    }

    #[test]
    fn measurement_report_all_present_round_trip() {
        let parts = MeasurementReportParts {
            snr: Some(SnrMeasurement(0x10)),
            rssi_2: Some(Rssi2Measurement(0x20)),
            rssi_1: Some(Rssi1Measurement(0x30)),
            tx_count: Some(0xFF),
            from_rach: false,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 5);
        let parsed = MeasurementReportParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.snr.unwrap().0, 0x10);
        assert_eq!(parsed.rssi_2.unwrap().0, 0x20);
        assert_eq!(parsed.rssi_1.unwrap().0, 0x30);
        assert_eq!(parsed.tx_count, Some(0xFF));
        assert!(!parsed.from_rach);
    }

    #[test]
    fn measurement_report_header_only_round_trip() {
        // All optional fields absent; from_rach = false. Single byte body.
        let parts = MeasurementReportParts {
            snr: None,
            rssi_2: None,
            rssi_1: None,
            tx_count: None,
            from_rach: false,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0); // Every bitmap bit clear.
        let parsed = MeasurementReportParts::parse(&buf[..n]).unwrap();
        assert!(parsed.snr.is_none());
        assert!(parsed.rssi_2.is_none());
        assert!(parsed.rssi_1.is_none());
        assert!(parsed.tx_count.is_none());
        assert!(!parsed.from_rach);
    }

    #[test]
    fn measurement_report_from_rach_only_round_trip() {
        // Only the RACH bit set; no measurement payloads.
        let parts = MeasurementReportParts {
            snr: None,
            rssi_2: None,
            rssi_1: None,
            tx_count: None,
            from_rach: true,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0x01);
        let parsed = MeasurementReportParts::parse(&buf[..n]).unwrap();
        assert!(parsed.from_rach);
        assert!(parsed.snr.is_none());
    }

    #[test]
    fn measurement_report_serialize_rejects_short_buffer() {
        let parts = MeasurementReportParts {
            snr: Some(SnrMeasurement(0x10)),
            rssi_2: None,
            rssi_1: None,
            tx_count: None,
            from_rach: false,
        };
        // encoded_len = 2 (header + snr); buffer has only 1 byte.
        let mut buf = [0; 1];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn measurement_report_bitmap_layout() {
        // Verify each presence bit's position. Spec layout:
        // bit 4 = SNR, bit 3 = RSSI-2, bit 2 = RSSI-1, bit 1 = TxCount,
        // bit 0 = RACH.
        let snr_only = MeasurementReportParts {
            snr: Some(SnrMeasurement(0xAA)),
            rssi_2: None,
            rssi_1: None,
            tx_count: None,
            from_rach: false,
        };
        let mut buf = [0; 8];
        snr_only.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x10);

        let r2_only = MeasurementReportParts {
            snr: None,
            rssi_2: Some(Rssi2Measurement(0xAA)),
            rssi_1: None,
            tx_count: None,
            from_rach: false,
        };
        r2_only.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x08);

        let r1_only = MeasurementReportParts {
            snr: None,
            rssi_2: None,
            rssi_1: Some(Rssi1Measurement(0xAA)),
            tx_count: None,
            from_rach: false,
        };
        r1_only.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x04);

        let tx_only = MeasurementReportParts {
            snr: None,
            rssi_2: None,
            rssi_1: None,
            tx_count: Some(0xAA),
            from_rach: false,
        };
        tx_only.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x02);
    }

    #[test]
    fn measurement_report_rejects_truncated_rssi_2() {
        // Bitmap claims RSSI-2 (and SNR) present, but no RSSI-2 byte follows.
        // B0 = 0x10 (SNR) | 0x08 (RSSI-2) = 0x18; only SNR byte present.
        let buf = [0x18u8, 0xAA];
        assert!(MeasurementReportParts::parse(&buf).is_err());
    }

    // -------------------------------------------------------------------
    // Figure 6.4.3.12-1 hand-derived golden vectors
    // B0: bits 7:5 = Reserved(0) | bit4=SNR | bit3=RSSI-2 | bit2=RSSI-1
    //     bit1=TX count | bit0=RACH
    // Optional bytes follow in SNR, RSSI-2, RSSI-1, TX count order.
    // -------------------------------------------------------------------

    /// Minimal golden vector: all optional measurement fields absent,
    /// from_rach=false (measurement from scheduled resources).
    /// Total: 1 byte.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.12-1"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 1] = [
            0b000_0_0_0_0_0, // Reserved | SNR=0 | RSSI2=0 | RSSI1=0 | TX=0 | RACH=0
        ];
        let parts = MeasurementReportParts {
            from_rach: false,
            snr: None,
            rssi_2: None,
            rssi_1: None,
            tx_count: None,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(MeasurementReportParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Full golden vector: all optional fields present, from_rach=true
    /// (Table 6.4.3.12-1: measurement from Random Access response).
    /// Total: 5 bytes.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.12-1"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 5] = [
            0b000_1_1_1_1_1, // Reserved | SNR=1 | RSSI2=1 | RSSI1=1 | TX=1 | RACH=1
            0xA5,            // SNR result = 0xA5 (Table 6.4.3.12-1, coded per TS 103 636-2)
            0x7B,            // RSSI-2 result = 0x7B
            0x3C,            // RSSI-1 result = 0x3C
            0x0A,            // TX Count result = 10 attempts
        ];
        let parts = MeasurementReportParts {
            from_rach: true,                      // bit 0: RACH=1
            snr: Some(SnrMeasurement(0xA5)),      // bit 4
            rssi_2: Some(Rssi2Measurement(0x7B)), // bit 3
            rssi_1: Some(Rssi1Measurement(0x3C)), // bit 2
            tx_count: Some(0x0A),                 // bit 1
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(MeasurementReportParts::parse(&GOLDEN).unwrap(), parts);
    }
}
