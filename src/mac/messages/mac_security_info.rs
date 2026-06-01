//! MAC Security Info IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.1.

use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// MAC Security Info IE body (§6.4.3.1)
// ETSI TS 103 636-4, clause 6.4.3.1, Figure 6.4.3.1-1, Tables 6.4.3.1-1/-2
// ---------------------------------------------------------------------------

/// Owned representation of a MAC Security Info IE body. 5 bytes fixed.
///
/// Byte layout:
/// * B0: `[Version (2) | Key Index (2) | Security IV Type (4)]`
/// * B1..=B4: Hyper Packet Counter (HPC), big-endian u32.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct MacSecurityInfoParts {
    pub version: SecurityVersion,
    pub key_index: KeyIndex,
    pub iv_type: SecurityIvType,
    /// Hyper Packet Counter.
    pub hpc: u32,
}

impl MacSecurityInfoParts {
    /// Body length in bytes (always 5).
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        5
    }

    /// Serialize the body. Writes exactly 5 bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is shorter than 5.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        if out.len() < 5 {
            return Err(ExcessiveBitsSet);
        }
        out[0] = (self.version.as_u8() << 6)
            | (self.key_index.as_u8() << 4)
            | (self.iv_type.as_u8() & 0x0F);
        let hpc = self.hpc.to_be_bytes();
        out[1] = hpc[0];
        out[2] = hpc[1];
        out[3] = hpc[2];
        out[4] = hpc[3];
        Ok(5)
    }

    /// Parse a MAC Security Info body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer, reserved Version
    /// (`0b01`/`0b10`/`0b11`), or reserved Security IV Type
    /// (`0b0011..=0b1111`).
    pub const fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 5 {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let version = match SecurityVersion::try_from_u8(b0 >> 6) {
            Some(v) => v,
            None => return Err(ParsingError::Truncated),
        };
        let key_index = match KeyIndex::new((b0 >> 4) & 0x03) {
            Some(k) => k,
            None => return Err(ParsingError::ReservedValue),
        };
        let iv_type = match SecurityIvType::try_from_u8(b0 & 0x0F) {
            Some(t) => t,
            None => return Err(ParsingError::ReservedValue),
        };
        let hpc = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
        Ok(Self {
            version,
            key_index,
            iv_type,
            hpc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mac_security_info_round_trip() {
        let parts = MacSecurityInfoParts {
            version: SecurityVersion::Mode1,
            key_index: KeyIndex::new(2).unwrap(),
            iv_type: SecurityIvType::ResynchronizingHpc,
            hpc: 0xDEAD_BEEF,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 5);
        // B0 = version(00) | key_index(10) | iv_type(0001) = 0b0010_0001
        assert_eq!(buf[0], 0b0010_0001);
        assert_eq!(&buf[1..5], &[0xDE, 0xAD, 0xBE, 0xEF]);

        let parsed = MacSecurityInfoParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.version, SecurityVersion::Mode1);
        assert_eq!(parsed.key_index.as_u8(), 2);
        assert_eq!(parsed.iv_type, SecurityIvType::ResynchronizingHpc);
        assert_eq!(parsed.hpc, 0xDEAD_BEEF);
    }

    #[test]
    fn mac_security_info_parser_rejects_reserved_version() {
        // Version = 0b01 reserved.
        let buf = [0b0100_0000, 0, 0, 0, 0];
        assert!(MacSecurityInfoParts::parse(&buf).is_err());
    }

    #[test]
    fn mac_security_info_parser_rejects_reserved_iv_type() {
        // Version = 0b00 (Mode 1), Key Index = 0b00, IV Type = 0b0011 (reserved).
        let buf = [0b0000_0011, 0, 0, 0, 0];
        assert!(MacSecurityInfoParts::parse(&buf).is_err());
    }

    #[test]
    fn mac_security_info_parser_rejects_short_buffer() {
        let buf = [0u8; 4];
        assert!(MacSecurityInfoParts::parse(&buf).is_err());
    }
}
