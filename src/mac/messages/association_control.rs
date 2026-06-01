//! Association Control IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.18.

use crate::mac::pdu::ShortMessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Association Control IE body (§6.4.3.18)  -- 1 byte
// ---------------------------------------------------------------------------

/// Owned representation of an Association Control IE body (1 byte).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct AssociationControlParts {
    /// `CB_M`: `false` = associated RD maintains cluster beacon reception;
    /// `true` = it does not.
    pub cb_m: bool,
    pub dl_data_reception: DlDataReception,
    pub ul_period: UlPeriod,
}

impl AssociationControlParts {
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
        let cb_m_bit = if self.cb_m { 0x80 } else { 0 };
        out[0] = cb_m_bit
            | ((self.dl_data_reception.as_u8() & 0x07) << 4)
            | (self.ul_period.as_u8() & 0x0F);
        Ok(1)
    }

    /// Parse the bytes as `Self`.
    pub const fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let cb_m = b0 & 0x80 != 0;
        let dl_data_reception = match DlDataReception::try_from_u8((b0 >> 4) & 0x07) {
            Some(d) => d,
            None => return Err(ParsingError::ReservedValue),
        };
        let ul_period = match UlPeriod::try_from_u8(b0 & 0x0F) {
            Some(u) => u,
            None => return Err(ParsingError::ReservedValue),
        };
        Ok(Self {
            cb_m,
            dl_data_reception,
            ul_period,
        })
    }
}

impl ShortMessageBody for AssociationControlParts {
    const IE_TYPE: ShortIeType = ShortIeType::Len1(IEType5bitLen1::AssociationControl);
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
    fn association_control_round_trip() {
        let parts = AssociationControlParts {
            cb_m: true,
            dl_data_reception: DlDataReception::Ms40,
            ul_period: UlPeriod::H6,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = AssociationControlParts::parse(&buf).unwrap();
        assert!(parsed.cb_m);
        assert_eq!(parsed.dl_data_reception, DlDataReception::Ms40);
        assert_eq!(parsed.ul_period, UlPeriod::H6);
    }

    #[test]
    fn association_control_rejects_reserved_dl_reception() {
        // DLdataReception = 6 (reserved).
        // B0 = 0 110 0000 = 0x60
        let buf = [0x60u8; 1];
        assert!(AssociationControlParts::parse(&buf).is_err());
    }
}
