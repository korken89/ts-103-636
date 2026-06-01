//! Broadcast Indication IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.7.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Broadcast Indication IE body (§6.4.3.7)
// ---------------------------------------------------------------------------

/// RD ID carried by a Broadcast Indication IE. The variant is selected
/// by the on-wire `IDType` bit (0 = Short, 1 = Long).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum BroadcastRdId {
    Short(ShortRdId),
    Long(LongRdId),
}

/// Owned representation of a Broadcast Indication IE body.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct BroadcastIndicationParts {
    pub indication_type: IndicationType,
    pub rd_id: BroadcastRdId,
    /// ACK/NACK bit. Meaningful only when
    /// `indication_type == IndicationType::RandomAccessResponse`; for
    /// other indication types the spec ignores this field.
    pub ack_nack: bool,
    pub feedback: BroadcastFeedbackType,
    /// On-wire `Resource Allocation` bit. `true` means a Resource
    /// Allocation IE for the RD follows in this MAC PDU.
    pub resource_allocation_present: bool,
    /// Raw MCS / MIMO feedback byte. Interpretation depends on
    /// [`Self::feedback`]; see §6.4.3.7 Table 6.4.3.7-1.
    pub mcs_or_mimo_feedback: u8,
}

impl BroadcastIndicationParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let id_len = match self.rd_id {
            BroadcastRdId::Short(_) => 2,
            BroadcastRdId::Long(_) => 4,
        };
        1 + id_len + 1
    }

    /// Serialize into `out`. Returns the number of bytes written.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        let id_type_bit = match self.rd_id {
            BroadcastRdId::Short(_) => 0,
            BroadcastRdId::Long(_) => 0x10,
        };
        let ack_bit = if self.ack_nack { 0x08 } else { 0 };
        let ra_bit = if self.resource_allocation_present {
            0x01
        } else {
            0
        };
        out[0] = ((self.indication_type.as_u8() & 0x07) << 5)
            | id_type_bit
            | ack_bit
            | ((self.feedback.as_u8() & 0x03) << 1)
            | ra_bit;
        let mut pos = 1;
        match self.rd_id {
            BroadcastRdId::Short(s) => {
                let raw = s.as_u16().to_be_bytes();
                out[pos] = raw[0];
                out[pos + 1] = raw[1];
                pos += 2;
            }
            BroadcastRdId::Long(l) => {
                let raw = l.as_u32().to_be_bytes();
                out[pos] = raw[0];
                out[pos + 1] = raw[1];
                out[pos + 2] = raw[2];
                out[pos + 3] = raw[3];
                pos += 4;
            }
        }
        out[pos] = self.mcs_or_mimo_feedback;
        Ok(pos + 1)
    }

    /// Parse the bytes as `Self`.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let indication_type =
            IndicationType::try_from_u8(b0 >> 5).ok_or(ParsingError::ReservedValue)?;
        let long_id = b0 & 0x10 != 0;
        let ack_nack = b0 & 0x08 != 0;
        let feedback = BroadcastFeedbackType::try_from_u8((b0 >> 1) & 0x03)
            .ok_or(ParsingError::ReservedValue)?;
        let ra = b0 & 0x01 != 0;
        let id_len = if long_id { 4 } else { 2 };
        if buffer.len() < 1 + id_len + 1 {
            return Err(ParsingError::Truncated);
        }
        let rd_id = if long_id {
            let raw = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
            BroadcastRdId::Long(LongRdId::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            let raw = u16::from_be_bytes([buffer[1], buffer[2]]);
            BroadcastRdId::Short(ShortRdId::new(raw).ok_or(ParsingError::ReservedValue)?)
        };
        let mcs_or_mimo_feedback = buffer[1 + id_len];
        Ok(Self {
            indication_type,
            rd_id,
            ack_nack,
            feedback,
            resource_allocation_present: ra,
            mcs_or_mimo_feedback,
        })
    }
}

impl MessageBody for BroadcastIndicationParts {
    const IE_TYPE: IEType6bit = IEType6bit::BroadcastIndication;
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
    fn broadcast_indication_short_rd_id_round_trip() {
        let parts = BroadcastIndicationParts {
            indication_type: IndicationType::RandomAccessResponse,
            rd_id: BroadcastRdId::Short(ShortRdId::new(0xABCD).unwrap()),
            ack_nack: true,
            feedback: BroadcastFeedbackType::Mcs,
            resource_allocation_present: true,
            mcs_or_mimo_feedback: 0x07,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4);
        let parsed = BroadcastIndicationParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.indication_type, IndicationType::RandomAccessResponse);
        match parsed.rd_id {
            BroadcastRdId::Short(s) => assert_eq!(s.as_u16(), 0xABCD),
            _ => panic!("expected short rd id"),
        }
        assert!(parsed.ack_nack);
        assert_eq!(parsed.feedback, BroadcastFeedbackType::Mcs);
        assert!(parsed.resource_allocation_present);
        assert_eq!(parsed.mcs_or_mimo_feedback, 0x07);
    }

    #[test]
    fn broadcast_indication_long_rd_id_round_trip() {
        let parts = BroadcastIndicationParts {
            indication_type: IndicationType::Paging,
            rd_id: BroadcastRdId::Long(LongRdId::new(0x1234_5678).unwrap()),
            ack_nack: false,
            feedback: BroadcastFeedbackType::NoFeedback,
            resource_allocation_present: false,
            mcs_or_mimo_feedback: 0,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 6);
        let parsed = BroadcastIndicationParts::parse(&buf[..n]).unwrap();
        match parsed.rd_id {
            BroadcastRdId::Long(l) => assert_eq!(l.as_u32(), 0x1234_5678),
            _ => panic!("expected long rd id"),
        }
    }
}
