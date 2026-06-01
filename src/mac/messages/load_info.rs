//! Load Info IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.10.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Load Info IE body (§6.4.3.10)
// ---------------------------------------------------------------------------

/// Owned representation of a Load Info IE body.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct LoadInfoParts {
    pub traffic_load: LoadPercentage,
    /// MAX number of associated devices. Encoded as 8-bit on the wire
    /// when `value <= 255`, otherwise as 16-bit.
    pub max_associated_rds: u16,
    pub currently_associated_ft_mode: LoadPercentage,
    pub currently_associated_pt_mode: Option<LoadPercentage>,
    pub rach_load: Option<LoadPercentage>,
    /// Pair of (free %, busy %), both or neither present.
    pub channel_load: Option<(LoadPercentage, LoadPercentage)>,
}

impl LoadInfoParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let mut len = 2 + if self.max_associated_rds > 255 { 2 } else { 1 } + 1;
        if self.currently_associated_pt_mode.is_some() {
            len += 1;
        }
        if self.rach_load.is_some() {
            len += 1;
        }
        if self.channel_load.is_some() {
            len += 2;
        }
        len
    }

    /// Serialize into `out`. Returns the number of bytes written.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        let max_assoc_bit = if self.max_associated_rds > 255 {
            0x08
        } else {
            0
        };
        let pt_bit = if self.currently_associated_pt_mode.is_some() {
            0x04
        } else {
            0
        };
        let rach_bit = if self.rach_load.is_some() { 0x02 } else { 0 };
        let chload_bit = if self.channel_load.is_some() { 0x01 } else { 0 };
        out[0] = max_assoc_bit | pt_bit | rach_bit | chload_bit;
        out[1] = self.traffic_load.0;
        let mut pos = 2;
        if self.max_associated_rds > 255 {
            let raw = self.max_associated_rds.to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        } else {
            out[pos] = self.max_associated_rds as u8;
            pos += 1;
        }
        out[pos] = self.currently_associated_ft_mode.0;
        pos += 1;
        if let Some(v) = self.currently_associated_pt_mode {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some(v) = self.rach_load {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some((free, busy)) = self.channel_load {
            out[pos] = free.0;
            out[pos + 1] = busy.0;
            pos += 2;
        }
        Ok(pos)
    }

    /// Parse the bytes as `Self`.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 2 {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let max_assoc_16bit = b0 & 0x08 != 0;
        let pt_load = b0 & 0x04 != 0;
        let rach_load = b0 & 0x02 != 0;
        let channel_load = b0 & 0x01 != 0;
        let traffic_load = LoadPercentage(buffer[1]);
        let mut pos = 2;
        let max_associated_rds = if max_assoc_16bit {
            if buffer.len() < pos + 2 {
                return Err(ParsingError::Truncated);
            }
            let v = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]);
            pos += 2;
            v
        } else {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = buffer[pos] as u16;
            pos += 1;
            v
        };
        if buffer.len() < pos + 1 {
            return Err(ParsingError::Truncated);
        }
        let currently_associated_ft_mode = LoadPercentage(buffer[pos]);
        pos += 1;
        let currently_associated_pt_mode = if pt_load {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = LoadPercentage(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let rach_load = if rach_load {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = LoadPercentage(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let channel_load = if channel_load {
            if buffer.len() < pos + 2 {
                return Err(ParsingError::Truncated);
            }
            let v = (LoadPercentage(buffer[pos]), LoadPercentage(buffer[pos + 1]));
            pos += 2;
            Some(v)
        } else {
            None
        };
        let _ = pos;
        Ok(Self {
            traffic_load,
            max_associated_rds,
            currently_associated_ft_mode,
            currently_associated_pt_mode,
            rach_load,
            channel_load,
        })
    }
}

impl MessageBody for LoadInfoParts {
    const IE_TYPE: IEType6bit = IEType6bit::LoadInfo;
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
    fn load_info_minimal_round_trip() {
        let parts = LoadInfoParts {
            traffic_load: LoadPercentage(128),
            max_associated_rds: 100,
            currently_associated_ft_mode: LoadPercentage(50),
            currently_associated_pt_mode: None,
            rach_load: None,
            channel_load: None,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4);
        let parsed = LoadInfoParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.traffic_load.0, 128);
        assert_eq!(parsed.max_associated_rds, 100);
        assert_eq!(parsed.currently_associated_ft_mode.0, 50);
        assert!(parsed.currently_associated_pt_mode.is_none());
        assert!(parsed.rach_load.is_none());
        assert!(parsed.channel_load.is_none());
    }

    #[test]
    fn load_info_full_round_trip_with_16bit_max() {
        let parts = LoadInfoParts {
            traffic_load: LoadPercentage(0xFF),
            max_associated_rds: 1024,
            currently_associated_ft_mode: LoadPercentage(75),
            currently_associated_pt_mode: Some(LoadPercentage(25)),
            rach_load: Some(LoadPercentage(40)),
            channel_load: Some((LoadPercentage(60), LoadPercentage(20))),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (bitmap) + 1 (traffic) + 2 (max 16-bit) + 1 (FT) + 1 (PT) + 1 (RACH) + 2 (channel) = 9
        assert_eq!(n, 9);
        let parsed = LoadInfoParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.max_associated_rds, 1024);
        assert_eq!(parsed.currently_associated_pt_mode.unwrap().0, 25);
        assert_eq!(parsed.rach_load.unwrap().0, 40);
        assert_eq!(parsed.channel_load.unwrap().0.0, 60);
        assert_eq!(parsed.channel_load.unwrap().1.0, 20);
    }

    #[test]
    fn parse_rejects_two_byte_payload() {
        // Fuzzer-found regression (crash-e70c623d): the minimum-length
        // prefix passes, but the mandatory 8-bit MAX associated RDs
        // field is missing. Must be Truncated, not an OOB index.
        assert!(matches!(
            LoadInfoParts::parse(&[0x02, 0x02]),
            Err(ParsingError::Truncated)
        ));
        assert!(matches!(
            LoadInfoParts::parse(&[0x00, 0x00]),
            Err(ParsingError::Truncated)
        ));
    }
}
