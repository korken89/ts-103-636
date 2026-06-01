//! RD Capability IE (short form) body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.15.

use crate::mac::pdu::ShortMessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// RD Capability short IE body (§6.4.3.15)  -- 1 byte
// ---------------------------------------------------------------------------

/// Owned representation of a short-form RD Capability IE body (1 byte).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RdCapabilityShortParts {
    /// `CB_MC`: RD in FT mode supports association without monitoring
    /// Cluster Beacon messages.
    pub cb_mc: bool,
    pub harq_feedback_delay: HarqFeedbackDelay,
    /// `DWA`: RD in FT mode supports uplink data transmission without
    /// association.
    pub dwa: bool,
}

impl RdCapabilityShortParts {
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
        let cb_mc_bit = if self.cb_mc { 0x20 } else { 0 };
        let dwa_bit = if self.dwa { 0x01 } else { 0 };
        out[0] = cb_mc_bit | ((self.harq_feedback_delay.subslots() & 0x0F) << 1) | dwa_bit;
        Ok(1)
    }

    /// Parse the bytes as `Self`.
    pub const fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let cb_mc = b0 & 0x20 != 0;
        let harq_feedback_delay = match HarqFeedbackDelay::new((b0 >> 1) & 0x0F) {
            Some(d) => d,
            None => return Err(ParsingError::ReservedValue),
        };
        let dwa = b0 & 0x01 != 0;
        Ok(Self {
            cb_mc,
            harq_feedback_delay,
            dwa,
        })
    }
}

impl ShortMessageBody for RdCapabilityShortParts {
    const IE_TYPE: ShortIeType = ShortIeType::Len1(IEType5bitLen1::RdCapabilityShort);
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
    fn rd_capability_short_round_trip() {
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::new(3).unwrap(),
            dwa: false,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 3);
        assert!(!parsed.dwa);
    }

    #[test]
    fn rd_capability_short_rejects_empty_buffer() {
        assert!(RdCapabilityShortParts::parse(&[]).is_err());
    }

    #[test]
    fn rd_capability_short_rejects_reserved_harq_feedback_delay() {
        // HarqFeedbackDelay only accepts 0..=6. Place 7 in bits 1..=4.
        // B0 = 0 | HARQ-delay (4) | dwa (1). HARQ=7 -> b0 = 7 << 1 = 0x0E.
        let buf = [0x0Eu8];
        assert!(RdCapabilityShortParts::parse(&buf).is_err());
    }

    #[test]
    fn rd_capability_short_serialize_rejects_empty_buffer() {
        let parts = RdCapabilityShortParts {
            cb_mc: false,
            harq_feedback_delay: HarqFeedbackDelay::new(0).unwrap(),
            dwa: false,
        };
        let mut buf = [0u8; 0];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn rd_capability_short_all_flags_set_round_trip() {
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::new(6).unwrap(),
            dwa: true,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 6);
        assert!(parsed.dwa);
    }

    #[test]
    fn rd_capability_short_dwa_only_round_trip() {
        // CB_MC=0, HARQ=0, DWA=1 -> B0 = 0x01
        let parts = RdCapabilityShortParts {
            cb_mc: false,
            harq_feedback_delay: HarqFeedbackDelay::new(0).unwrap(),
            dwa: true,
        };
        let mut buf = [0u8; 4];
        parts.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x01);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(!parsed.cb_mc);
        assert_eq!(parsed.harq_feedback_delay.subslots(), 0);
        assert!(parsed.dwa);
    }

    #[test]
    fn rd_capability_short_cb_mc_only_round_trip() {
        // CB_MC=1, HARQ=0, DWA=0 -> B0 = 0x20
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::new(0).unwrap(),
            dwa: false,
        };
        let mut buf = [0u8; 4];
        parts.serialize(&mut buf).unwrap();
        assert_eq!(buf[0], 0x20);
        let parsed = RdCapabilityShortParts::parse(&buf).unwrap();
        assert!(parsed.cb_mc);
        assert!(!parsed.dwa);
    }

    #[test]
    fn rd_capability_short_bit_layout() {
        // Spec layout: bit 5 = CB_MC, bits 1..=4 = HARQ feedback delay,
        // bit 0 = DWA. Verify each via a deliberately constructed value.
        let parts = RdCapabilityShortParts {
            cb_mc: true,
            harq_feedback_delay: HarqFeedbackDelay::new(5).unwrap(),
            dwa: true,
        };
        let mut buf = [0u8; 4];
        parts.serialize(&mut buf).unwrap();
        // 0x20 (CB_MC) | (5 << 1) (HARQ=5 -> 0x0A) | 0x01 (DWA) = 0x2B
        assert_eq!(buf[0], 0x2B);
    }
}
