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
