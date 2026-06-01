//! Association Release message body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.6.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// AssociationRelease body (§6.4.2.6)
// ETSI TS 103 636-4, clause 6.4.2.6, Figure 6.4.2.6-1, Table 6.4.2.6-1
// ---------------------------------------------------------------------------

/// Owned representation of an Association Release body. Single byte:
/// [Release Cause (4 bits, ETSI 0..=3) | Reserved (4 bits, ETSI 4..=7)].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct AssociationReleaseParts {
    pub cause: ReleaseCause,
}

impl AssociationReleaseParts {
    /// Body length in bytes (always 1).
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        1
    }

    /// Serialize the body. Always writes exactly one byte.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] only if `out.is_empty()`.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        if out.is_empty() {
            return Err(ExcessiveBitsSet);
        }
        out[0] = self.cause.as_u8() << 4;
        Ok(1)
    }

    /// Parse an Association Release body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer or a reserved Release
    /// Cause value (`0b1011`, `0b1110`, `0b1111`).
    pub const fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let cause = match ReleaseCause::try_from_u8(buffer[0] >> 4) {
            Some(c) => c,
            None => return Err(ParsingError::Truncated),
        };
        Ok(Self { cause })
    }
}

impl MessageBody for AssociationReleaseParts {
    const IE_TYPE: IEType6bit = IEType6bit::AssociationRelease;
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
    fn association_release_round_trip() {
        let parts = AssociationReleaseParts {
            cause: ReleaseCause::Mobility,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        // B0 high nibble = cause (0b0001), low nibble reserved = 0
        assert_eq!(buf[0], 0b0001_0000);

        let parsed = AssociationReleaseParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.cause, ReleaseCause::Mobility);
    }

    #[test]
    fn association_release_parser_rejects_reserved_causes() {
        // 0b1011, 0b1110, 0b1111 are reserved per Table 6.4.2.6-1.
        for cause in [0b1011_u8, 0b1110, 0b1111] {
            let buf = [cause << 4];
            assert!(
                AssociationReleaseParts::parse(&buf).is_err(),
                "cause {cause:#06b} should be reserved"
            );
        }
    }

    #[test]
    fn association_release_parser_ignores_reserved_low_nibble() {
        // Reserved low nibble may carry any bits; parser should accept.
        let buf = [0b0001_1111]; // cause = Mobility, low nibble all 1s
        let parsed = AssociationReleaseParts::parse(&buf).unwrap();
        assert_eq!(parsed.cause, ReleaseCause::Mobility);
    }
}
