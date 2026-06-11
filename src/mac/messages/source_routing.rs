//! Source Routing IE body (generated codec re-export).
//!
//! The codec lives in [`generated::source_routing`](super::generated::source_routing); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::source_routing::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    // -----------------------------------------------------------------------
    // Golden vector - hand-derived from ETSI TS 103 636-4 clause 6.4.3.16,
    // Figure 6.4.3.16-1, Table 6.4.3.16-1.
    //
    // Fixed 6-byte layout:
    //   bytes[0..3] = source_routing_id (LongRdId, 32-bit, big-endian)
    //   bytes[4]    = hop_limit[3:0](b7..b4) | hop_count[3:0](b3..b0)
    //   bytes[5]    = validity_timer (8-bit code, Table 6.4.3.16-1)
    // -----------------------------------------------------------------------

    /// Clause 6.4.3.16, fixed layout (no optionals). Source Routing ID
    /// 0xDEAD_C0DE, hop-limit=12, hop-count=5, validity timer H2 (code=11).
    ///
    /// bytes[4] = (12 << 4) | 5 = 0b1100_0000 | 0b0000_0101 = 0xC5
    /// bytes[5] = H2 = 11 = 0x0B  (Table 6.4.3.16-1: "2 hours")
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows source-routing field layout"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 6] = [
            0xDE,        // source_routing_id[31..24] (LongRdId 0xDEAD_C0DE big-endian)
            0xAD,        // source_routing_id[23..16]
            0xC0,        // source_routing_id[15..8]
            0xDE,        // source_routing_id[7..0]
            0b1100_0101, // hop_limit=12(1100) | hop_count=5(0101)
            0x0B,        // validity_timer H2=11; Table 6.4.3.16-1 code 11 = 2 hours
        ];
        let parts = SourceRoutingParts {
            source_routing_id: LongRdId::new(0xDEAD_C0DE).unwrap(),
            hop_limit: Hop::new(12).unwrap(),
            hop_count: Hop::new(5).unwrap(),
            validity_timer: SourceRoutingValidityTimer::H2,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(SourceRoutingParts::parse(&GOLDEN).unwrap(), parts);
    }

    #[test]
    fn source_routing_round_trip() {
        let parts = SourceRoutingParts {
            source_routing_id: LongRdId::new(0xCAFEBABE).unwrap(),
            hop_limit: Hop::new(8).unwrap(),
            hop_count: Hop::new(3).unwrap(),
            validity_timer: SourceRoutingValidityTimer::H1,
        };
        let mut buf = [0; 8];
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
