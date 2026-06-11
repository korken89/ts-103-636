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
