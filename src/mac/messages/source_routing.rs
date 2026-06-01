//! Source Routing IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.16.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Source Routing IE body (§6.4.3.16)  -- 6 bytes fixed
// ---------------------------------------------------------------------------

/// Owned representation of a Source Routing IE body (6 bytes).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct SourceRoutingParts {
    pub source_routing_id: LongRdId,
    pub hop_limit: Hop,
    pub hop_count: Hop,
    pub validity_timer: SourceRoutingValidityTimer,
}

impl SourceRoutingParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        6
    }

    /// Serialize into `out`. Returns the number of bytes written.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        if out.len() < 6 {
            return Err(ExcessiveBitsSet);
        }
        let id = self.source_routing_id.as_u32().to_be_bytes();
        out[0] = id[0];
        out[1] = id[1];
        out[2] = id[2];
        out[3] = id[3];
        out[4] = (self.hop_limit.as_u8() << 4) | (self.hop_count.as_u8() & 0x0F);
        out[5] = self.validity_timer.as_u8();
        Ok(6)
    }

    /// Parse the bytes as `Self`.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 6 {
            return Err(ParsingError::Truncated);
        }
        let id_raw = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let source_routing_id = LongRdId::new(id_raw).ok_or(ParsingError::ReservedValue)?;
        let hop_limit = Hop::new(buffer[4] >> 4).ok_or(ParsingError::ReservedValue)?;
        let hop_count = Hop::new(buffer[4] & 0x0F).ok_or(ParsingError::ReservedValue)?;
        let validity_timer = SourceRoutingValidityTimer::try_from_u8(buffer[5])
            .ok_or(ParsingError::ReservedValue)?;
        Ok(Self {
            source_routing_id,
            hop_limit,
            hop_count,
            validity_timer,
        })
    }
}

impl MessageBody for SourceRoutingParts {
    const IE_TYPE: IEType6bit = IEType6bit::SourceRouting;
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
    fn source_routing_round_trip() {
        let parts = SourceRoutingParts {
            source_routing_id: LongRdId::new(0xCAFEBABE).unwrap(),
            hop_limit: Hop::new(8).unwrap(),
            hop_count: Hop::new(3).unwrap(),
            validity_timer: SourceRoutingValidityTimer::H1,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 6);
        let parsed = SourceRoutingParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.source_routing_id.as_u32(), 0xCAFEBABE);
        assert_eq!(parsed.hop_limit.as_u8(), 8);
        assert_eq!(parsed.hop_count.as_u8(), 3);
        assert_eq!(parsed.validity_timer.seconds(), Some(3600));
    }

    #[test]
    fn source_routing_rejects_reserved_validity_timer() {
        let buf = [0xAA, 0xBB, 0xCC, 0xDD, 0, 20]; // 20 reserved
        assert!(SourceRoutingParts::parse(&buf).is_err());
    }
}
