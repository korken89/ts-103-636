//! Joining Information IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.17.

use heapless::Vec;

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

/// Maximum number of endpoints (on-wire 2-bit count + 1 = 4).
pub const MAX_ENDPOINTS: usize = 4;

/// Owned representation of a Joining Information IE body.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct JoiningInformationParts {
    /// 1..=4 endpoint protocol values. The on-wire 2-bit count carries
    /// `endpoints.len() - 1`.
    pub endpoints: Vec<EndpointProtocol, MAX_ENDPOINTS>,
}

impl JoiningInformationParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        1 + self.endpoints.len() * 2
    }

    /// Serialize into `out`. Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] for an empty `endpoints` list (on-wire
    /// count is `len - 1` so zero is unrepresentable) or short buffer.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let n = self.endpoints.len();
        if n == 0 {
            return Err(ExcessiveBitsSet);
        }
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        out[0] = (n as u8 - 1) & 0x03;
        let mut pos = 1;
        for ep in &self.endpoints {
            let raw = ep.0.to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }
        Ok(pos)
    }

    /// Parse the bytes as `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let count = ((buffer[0] & 0x03) + 1) as usize;
        if buffer.len() < 1 + count * 2 {
            return Err(ParsingError::Truncated);
        }
        let mut endpoints = Vec::new();
        for i in 0..count {
            let p = 1 + i * 2;
            let ep = EndpointProtocol(u16::from_be_bytes([buffer[p], buffer[p + 1]]));
            endpoints
                .push(ep)
                .expect("count <= MAX_ENDPOINTS by construction");
        }
        Ok(Self { endpoints })
    }
}

impl MessageBody for JoiningInformationParts {
    const IE_TYPE: IEType6bit = IEType6bit::JoiningInformation;
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::encoded_len(self)
    }
    #[inline]
    fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        Self::serialize(self, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn joining_information_round_trip() {
        let endpoints = Vec::from_slice(&[
            EndpointProtocol(0x1111),
            EndpointProtocol(0x2222),
            EndpointProtocol(0x3333),
        ])
        .unwrap();
        let parts = JoiningInformationParts { endpoints };
        let mut buf = [0u8; 16];
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
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn joining_information_parse_rejects_empty_buffer() {
        assert!(JoiningInformationParts::parse(&[]).is_err());
    }
}
