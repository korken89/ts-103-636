//! Radio Device Status IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.13.

use crate::mac::pdu::ShortMessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Radio Device Status IE body (§6.4.3.13)  -- 1 byte
// ---------------------------------------------------------------------------

/// Owned representation of a Radio Device Status IE body (1 byte).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RadioDeviceStatusParts {
    pub association_needed: bool,
    pub status: RadioDeviceStatusFlag,
    pub duration: RadioDeviceStatusDuration,
}

impl RadioDeviceStatusParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        1
    }

    /// Serialize into `out`. Returns the number of bytes written.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        if out.is_empty() {
            return Err(ExcessiveBitsSet);
        }
        let assoc_bit = if self.association_needed { 0x40 } else { 0 };
        out[0] = assoc_bit | ((self.status.as_u8() & 0x03) << 4) | (self.duration.as_u8() & 0x0F);
        Ok(1)
    }

    /// Parse the bytes as `Self`.
    pub const fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let association_needed = b0 & 0x40 != 0;
        let status = match RadioDeviceStatusFlag::try_from_u8((b0 >> 4) & 0x03) {
            Some(s) => s,
            None => return Err(ParsingError::Truncated),
        };
        let duration = match RadioDeviceStatusDuration::try_from_u8(b0 & 0x0F) {
            Some(d) => d,
            None => return Err(ParsingError::ReservedValue),
        };
        Ok(Self {
            association_needed,
            status,
            duration,
        })
    }
}

impl ShortMessageBody for RadioDeviceStatusParts {
    const IE_TYPE: ShortIeType = ShortIeType::Len1(IEType5bitLen1::RadioDeviceStatus);
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
    fn radio_device_status_round_trip() {
        let parts = RadioDeviceStatusParts {
            association_needed: true,
            status: RadioDeviceStatusFlag::MemoryFull,
            duration: RadioDeviceStatusDuration::Ms400,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = RadioDeviceStatusParts::parse(&buf).unwrap();
        assert!(parsed.association_needed);
        assert_eq!(parsed.status, RadioDeviceStatusFlag::MemoryFull);
        assert_eq!(parsed.duration, RadioDeviceStatusDuration::Ms400);
    }

    #[test]
    fn radio_device_status_rejects_reserved_status_flag() {
        // Status flag = 0b00 (reserved).
        let buf = [0u8; 1];
        assert!(RadioDeviceStatusParts::parse(&buf).is_err());
    }
}
