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
            key_index: KeyIndex::new(2).unwrap(),
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
}
