//! Joining Beacon message body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.10, Figure 6.4.2.10-1.

use heapless::Vec;

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

/// Maximum number of Joining Beacon channels (on-wire 2-bit count + 1 = 4).
pub const MAX_CHANNELS: usize = 4;

/// Owned representation of a Joining Beacon body. Always 1..=4 channels.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct JoiningBeaconParts {
    /// Network beacon period code (4 bits, see Table 6.4.2.2-1).
    pub network_beacon_period: NetworkBeaconPeriod,
    /// 1..=4 Network Beacon channels.
    pub channels: Vec<AbsoluteChannel, MAX_CHANNELS>,
}

impl JoiningBeaconParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        1 + self.channels.len() * 2
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if `channels.len()` is 0 or if the
    /// buffer is too short. The `> MAX_CHANNELS` case is unrepresentable
    /// by the `heapless::Vec`'s capacity.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let n = self.channels.len();
        if n == 0 {
            return Err(ExcessiveBitsSet);
        }
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        let count_field = (n as u8 - 1) & 0x03;
        out[0] = (count_field << 6) | (self.network_beacon_period.as_u8() << 2);

        let mut pos = 1;
        for ch in &self.channels {
            let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }
        Ok(pos)
    }

    /// Parse a Joining Beacon body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer or reserved-value
    /// rejection.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let count = ((b0 >> 6) + 1) as usize;
        let network_beacon_period = NetworkBeaconPeriod::try_from_u8((b0 >> 2) & 0x0F)
            .ok_or(ParsingError::ReservedValue)?;
        if buffer.len() < 1 + count * 2 {
            return Err(ParsingError::Truncated);
        }
        let mut channels = Vec::new();
        for i in 0..count {
            let p = 1 + i * 2;
            let raw = u16::from_be_bytes([buffer[p], buffer[p + 1]]) & 0x1FFF;
            let ch = AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?;
            channels
                .push(ch)
                .expect("count <= MAX_CHANNELS by construction");
        }
        Ok(Self {
            network_beacon_period,
            channels,
        })
    }
}

impl MessageBody for JoiningBeaconParts {
    const IE_TYPE: IEType6bit = IEType6bit::JoiningBeacon;
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
    fn joining_beacon_single_channel_round_trip() {
        let mut channels = Vec::new();
        channels
            .push(AbsoluteChannel::new(0x0100).unwrap())
            .unwrap();
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms1000,
            channels,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf[0], 0b0000_1100);

        let parsed = JoiningBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.channels.len(), 1);
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(parsed.channels[0].as_u16(), 0x0100);
    }

    #[test]
    fn joining_beacon_four_channels_round_trip() {
        let channels = Vec::from_slice(&[
            AbsoluteChannel::new(0x0100).unwrap(),
            AbsoluteChannel::new(0x0200).unwrap(),
            AbsoluteChannel::new(0x0300).unwrap(),
            AbsoluteChannel::new(0x1FFF).unwrap(),
        ])
        .unwrap();
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms4000,
            channels,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1 + 4 * 2);
        assert_eq!(buf[0], 0b1101_1000);

        let parsed = JoiningBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.channels.len(), 4);
        assert_eq!(parsed.channels[0].as_u16(), 0x0100);
        assert_eq!(parsed.channels[1].as_u16(), 0x0200);
        assert_eq!(parsed.channels[2].as_u16(), 0x0300);
        assert_eq!(parsed.channels[3].as_u16(), 0x1FFF);
    }

    #[test]
    fn joining_beacon_rejects_zero_channels() {
        let parts = JoiningBeaconParts {
            network_beacon_period: NetworkBeaconPeriod::Ms50,
            channels: Vec::new(),
        };
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn joining_beacon_parse_rejects_empty_buffer() {
        assert!(JoiningBeaconParts::parse(&[]).is_err());
    }
}
