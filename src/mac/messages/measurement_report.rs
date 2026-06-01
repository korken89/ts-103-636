//! Measurement Report IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.12.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Measurement Report IE body (§6.4.3.12)
// ---------------------------------------------------------------------------

/// Owned representation of a Measurement Report IE body.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct MeasurementReportParts {
    pub snr: Option<SnrMeasurement>,
    pub rssi_2: Option<Rssi2Measurement>,
    pub rssi_1: Option<Rssi1Measurement>,
    pub tx_count: Option<u8>,
    /// RACH bit. `false` = measurement from scheduled resources;
    /// `true` = measurement from Random Access response.
    pub from_rach: bool,
}

impl MeasurementReportParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let mut len = 1;
        if self.snr.is_some() {
            len += 1;
        }
        if self.rssi_2.is_some() {
            len += 1;
        }
        if self.rssi_1.is_some() {
            len += 1;
        }
        if self.tx_count.is_some() {
            len += 1;
        }
        len
    }

    /// Serialize into `out`. Returns the number of bytes written.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        let snr_bit = if self.snr.is_some() { 0x10 } else { 0 };
        let r2_bit = if self.rssi_2.is_some() { 0x08 } else { 0 };
        let r1_bit = if self.rssi_1.is_some() { 0x04 } else { 0 };
        let tx_bit = if self.tx_count.is_some() { 0x02 } else { 0 };
        let rach_bit = if self.from_rach { 0x01 } else { 0 };
        out[0] = snr_bit | r2_bit | r1_bit | tx_bit | rach_bit;
        let mut pos = 1;
        if let Some(v) = self.snr {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some(v) = self.rssi_2 {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some(v) = self.rssi_1 {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some(v) = self.tx_count {
            out[pos] = v;
            pos += 1;
        }
        Ok(pos)
    }

    /// Parse the bytes as `Self`.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let snr_set = b0 & 0x10 != 0;
        let r2_set = b0 & 0x08 != 0;
        let r1_set = b0 & 0x04 != 0;
        let tx_set = b0 & 0x02 != 0;
        let from_rach = b0 & 0x01 != 0;
        let mut pos = 1;
        let snr = if snr_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = SnrMeasurement(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let rssi_2 = if r2_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = Rssi2Measurement(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let rssi_1 = if r1_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = Rssi1Measurement(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let tx_count = if tx_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = buffer[pos];
            pos += 1;
            Some(v)
        } else {
            None
        };
        let _ = pos;
        Ok(Self {
            snr,
            rssi_2,
            rssi_1,
            tx_count,
            from_rach,
        })
    }
}

impl MessageBody for MeasurementReportParts {
    const IE_TYPE: IEType6bit = IEType6bit::MeasurementReport;
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::encoded_len(self)
    }
    #[inline]
    fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        Self::serialize(self, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
