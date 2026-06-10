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
        let mut buf = [0u8; 8];
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
        let mut buf = [0u8; 8];
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
        let mut buf = [0u8; 8];
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
        let mut buf = [0u8; 8];
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
        let mut buf = [0u8; 1];
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
        let mut buf = [0u8; 8];
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
}
