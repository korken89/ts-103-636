//! Association Release message body (generated codec re-export).
//!
//! The codec lives in [`generated::association_release`](super::generated::association_release); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::association_release::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    #[test]
    fn association_release_round_trip() {
        let parts = AssociationReleaseParts {
            cause: ReleaseCause::Mobility,
        };
        let mut buf = [0; 4];
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

    /// Golden vector hand-derived from Figure 6.4.2.6-1 and Table 6.4.2.6-1.
    ///
    /// Chosen value:
    ///   Release Cause = SecurityError (code 7 = 0b0111, Table 6.4.2.6-1)
    ///
    /// Byte 0 = [Release Cause(4) | Reserved(4)]
    ///        = [0111 | 0000]
    ///        = 0b0111_0000 = 0x70
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 1] = [
            0b0111_0000, // Release Cause = SecurityError(7) | Reserved = 0
        ];
        let parts = AssociationReleaseParts {
            cause: ReleaseCause::SecurityError,
        };
        let mut buf = [0u8; 1];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(AssociationReleaseParts::parse(&GOLDEN).unwrap(), parts);
    }
}
