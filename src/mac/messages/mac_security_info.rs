//! MAC Security Info IE body (generated codec re-export).
//!
//! The codec lives in [`generated::mac_security_info`](super::generated::mac_security_info); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::mac_security_info::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    #[test]
    fn mac_security_info_round_trip() {
        let parts = MacSecurityInfoParts {
            version: SecurityVersion::Mode1,
            key_index: KeyIndex::try_from_u8(2).unwrap(),
            iv_type: SecurityIvType::ResynchronizingHpc,
            hpc: 0xDEAD_BEEF,
        };
        let mut buf = [0; 8];
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
        let buf = [0; 4];
        assert!(MacSecurityInfoParts::parse(&buf).is_err());
    }

    // -------------------------------------------------------------------
    // Figure 6.4.3.1-1 hand-derived golden vector (fixed 5-byte layout)
    // B0: bits 7:6 = Version (2b) | bits 5:4 = Key Index (2b) | bits 3:0 = IV Type (4b)
    // B1-B4: HPC (32-bit big-endian Hyper Packet Counter)
    // -------------------------------------------------------------------

    /// Golden vector: Version=Mode1 (0b00), KeyIndex=3 (0b11),
    /// IvType=ResynchronizingHpc (0b0001, Table 6.4.3.1-2),
    /// HPC=0x1234_5678.
    /// Total: 5 bytes.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.1-1"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 5] = [
            0b00_11_0001, // Version=0b00 (Mode 1) | KeyIndex=0b11 (3) | IvType=0b0001 (ResynchronizingHpc)
            0x12,         // HPC byte 0 (MSB)
            0x34,         // HPC byte 1
            0x56,         // HPC byte 2
            0x78,         // HPC byte 3 (LSB)
        ];
        let parts = MacSecurityInfoParts {
            version: SecurityVersion::Mode1,              // Table 6.4.3.1-1: 0b00
            key_index: KeyIndex::try_from_u8(3).unwrap(), // 0b11
            iv_type: SecurityIvType::ResynchronizingHpc,  // Table 6.4.3.1-2: 0b0001
            hpc: 0x1234_5678,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(MacSecurityInfoParts::parse(&GOLDEN).unwrap(), parts);
    }
}
