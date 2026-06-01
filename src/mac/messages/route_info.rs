//! Route Info IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.2.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Route Info IE body (§6.4.3.2)
// ETSI TS 103 636-4, clause 6.4.3.2, Figure 6.4.3.2-1
// ---------------------------------------------------------------------------

/// Owned representation of a Route Info IE body. 6 bytes fixed, no
/// reserved bits.
///
/// Byte layout:
/// * B0..=B3: Sink Address (Long RD ID of the FT), big-endian u32.
/// * B4: Route Cost.
/// * B5: Application Sequence Number.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RouteInfoParts {
    pub sink_address: LongRdId,
    pub route_cost: RouteCost,
    pub application_sequence_number: ApplicationSequenceNumber,
}

impl RouteInfoParts {
    /// Body length in bytes (always 6).
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        6
    }

    /// Serialize the body. Writes exactly 6 bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] only if the buffer is shorter than 6.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        if out.len() < 6 {
            return Err(ExcessiveBitsSet);
        }
        let sink = self.sink_address.as_u32().to_be_bytes();
        out[0] = sink[0];
        out[1] = sink[1];
        out[2] = sink[2];
        out[3] = sink[3];
        out[4] = self.route_cost.0;
        out[5] = self.application_sequence_number.0;
        Ok(6)
    }

    /// Parse a Route Info body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer or a zero sink address
    /// (the reserved value of [`LongRdId`]).
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 6 {
            return Err(ParsingError::Truncated);
        }
        let sink_raw = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        Ok(Self {
            sink_address: LongRdId::new(sink_raw).ok_or(ParsingError::ReservedValue)?,
            route_cost: RouteCost(buffer[4]),
            application_sequence_number: ApplicationSequenceNumber(buffer[5]),
        })
    }
}

impl MessageBody for RouteInfoParts {
    const IE_TYPE: IEType6bit = IEType6bit::RouteInfo;
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
    fn route_info_round_trip() {
        let parts = RouteInfoParts {
            sink_address: LongRdId::new(0xAABBCCDD).unwrap(),
            route_cost: RouteCost(42),
            application_sequence_number: ApplicationSequenceNumber(7),
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 6);
        assert_eq!(&buf[..6], &[0xAA, 0xBB, 0xCC, 0xDD, 42, 7]);

        let parsed = RouteInfoParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.sink_address.as_u32(), 0xAABBCCDD);
        assert_eq!(parsed.route_cost.0, 42);
        assert_eq!(parsed.application_sequence_number.0, 7);
    }

    #[test]
    fn route_info_parser_rejects_zero_sink() {
        // Sink Address = 0 is the reserved value of LongRdId.
        let buf = [0u8; 6];
        assert!(RouteInfoParts::parse(&buf).is_err());
    }

    #[test]
    fn route_info_parser_rejects_short_buffer() {
        let buf = [0u8; 5];
        assert!(RouteInfoParts::parse(&buf).is_err());
    }
}
