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
}
