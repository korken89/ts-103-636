//! Route Info IE body (generated codec re-export).
//!
//! The codec lives in [`generated::route_info`](super::generated::route_info); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::route_info::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    // -----------------------------------------------------------------------
    // Golden vector - hand-derived from ETSI TS 103 636-4 clause 6.4.3.2,
    // Figure 6.4.3.2-1.
    //
    // Fixed 6-byte layout:
    //   bytes[0..3] = sink_address (LongRdId, 32-bit, big-endian)
    //   bytes[4]    = route_cost (8-bit)
    //   bytes[5]    = application_sequence_number (8-bit)
    // -----------------------------------------------------------------------

    /// Clause 6.4.3.2, fixed layout (no optionals). Sink address 0x1234_5678,
    /// route cost 0xA5 (lower = cheaper; increased by >= 1 per hop), ASN 0x3C.
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows route-info field layout"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 6] = [
            0x12, // sink_address[31..24] (LongRdId 0x1234_5678 big-endian)
            0x34, // sink_address[23..16]
            0x56, // sink_address[15..8]
            0x78, // sink_address[7..0]
            0xA5, // route_cost = 165 (distinctive non-trivial value)
            0x3C, // application_sequence_number = 60
        ];
        let parts = RouteInfoParts {
            sink_address: LongRdId::new(0x1234_5678).unwrap(),
            route_cost: RouteCost(0xA5),
            application_sequence_number: ApplicationSequenceNumber(0x3C),
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(RouteInfoParts::parse(&GOLDEN).unwrap(), parts);
    }

    #[test]
    fn route_info_round_trip() {
        let parts = RouteInfoParts {
            sink_address: LongRdId::new(0xAABBCCDD).unwrap(),
            route_cost: RouteCost(42),
            application_sequence_number: ApplicationSequenceNumber(7),
        };
        let mut buf = [0; 8];
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
        let buf = [0; 6];
        assert!(RouteInfoParts::parse(&buf).is_err());
    }

    #[test]
    fn route_info_parser_rejects_short_buffer() {
        let buf = [0; 5];
        assert!(RouteInfoParts::parse(&buf).is_err());
    }
}
