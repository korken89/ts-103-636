//! Joining Information IE body (generated codec re-export).
//!
//! The codec lives in [`generated::joining_information`](super::generated::joining_information); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::joining_information::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use heapless::Vec;
    #[test]
    fn joining_information_round_trip() {
        let endpoints = Vec::from_slice(&[
            EndpointProtocol(0x1111),
            EndpointProtocol(0x2222),
            EndpointProtocol(0x3333),
        ])
        .unwrap();
        let parts = JoiningInformationParts { endpoints };
        let mut buf = [0; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 7);

        let parsed = JoiningInformationParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.endpoints.len(), 3);
        assert_eq!(parsed.endpoints[0].0, 0x1111);
        assert_eq!(parsed.endpoints[1].0, 0x2222);
        assert_eq!(parsed.endpoints[2].0, 0x3333);
    }

    #[test]
    fn joining_information_rejects_zero_endpoints() {
        let parts = JoiningInformationParts {
            endpoints: Vec::new(),
        };
        let mut buf = [0; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn joining_information_parse_rejects_empty_buffer() {
        assert!(JoiningInformationParts::parse(&[]).is_err());
    }

    // -------------------------------------------------------------------
    // Figure 6.4.3.17-1 hand-derived golden vectors
    // B0: bits 7:2 = Reserved(0), bits 1:0 = N (endpoints_count - 1)
    // Each endpoint: 16-bit big-endian Protocol Endpoint value
    // -------------------------------------------------------------------

    /// Minimal golden vector: 1 endpoint (N=0b00).
    /// Total: 3 bytes (1 header + 2 per endpoint).
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.17-1"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 3] = [
            0b000000_00, // Reserved=0 | N=0 (1 endpoint)
            0xAB,        // EP 0xABCD high byte
            0xCD,        // EP 0xABCD low byte
        ];
        let endpoints = Vec::from_slice(&[EndpointProtocol(0xABCD)]).unwrap();
        let parts = JoiningInformationParts { endpoints };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = JoiningInformationParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed, parts);
    }

    /// Full golden vector: 4 endpoints (N=0b11), MAX_ENDPOINTS capacity.
    /// Total: 9 bytes (1 header + 4*2 endpoint bytes).
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows IE field layout per Figure 6.4.3.17-1"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 9] = [
            0b000000_11, // Reserved=0 | N=3 (4 endpoints)
            0x12,
            0x34, // EP 0x1234
            0x56,
            0x78, // EP 0x5678
            0x9A,
            0xBC, // EP 0x9ABC
            0xDE,
            0xF0, // EP 0xDEF0
        ];
        let endpoints = Vec::from_slice(&[
            EndpointProtocol(0x1234),
            EndpointProtocol(0x5678),
            EndpointProtocol(0x9ABC),
            EndpointProtocol(0xDEF0),
        ])
        .unwrap();
        let parts = JoiningInformationParts { endpoints };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        let parsed = JoiningInformationParts::parse(&GOLDEN).unwrap();
        assert_eq!(parsed, parts);
    }
}
