//! Network Beacon message body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.2, Figure 6.4.2.2-1 / Table 6.4.2.2-1.

use heapless::Vec;

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

/// Maximum number of additional Network Beacon channels (2-bit count field).
pub const MAX_ADDITIONAL_CHANNELS: usize = 3;

/// Owned representation of a Network Beacon body.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct NetworkBeaconParts {
    pub power_const: PowerConst,
    /// Network beacon period code (4 bits).
    pub network_beacon_period: NetworkBeaconPeriod,
    /// Cluster beacon period code (4 bits).
    pub cluster_beacon_period: ClusterBeaconPeriod,
    /// Next cluster channel (13-bit).
    pub next_cluster_channel: AbsoluteChannel,
    /// Time to next beacon period in microseconds (32-bit).
    pub time_to_next: u32,
    /// Optional cluster maximum TX power (4-bit field).
    pub cluster_max_tx_power: Option<TransmitPower>,
    /// Optional current cluster channel.
    pub current_cluster_channel: Option<AbsoluteChannel>,
    /// Additional Network Beacon channels (up to [`MAX_ADDITIONAL_CHANNELS`]).
    pub additional_channels: Vec<AbsoluteChannel, MAX_ADDITIONAL_CHANNELS>,
}

impl NetworkBeaconParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        let mut len = 8;
        if self.cluster_max_tx_power.is_some() {
            len += 1;
        }
        if self.current_cluster_channel.is_some() {
            len += 2;
        }
        len += self.additional_channels.len() * 2;
        len
    }

    /// Serialize the body into a buffer. Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is too short.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        let mut flags: u8 = 0;
        if self.cluster_max_tx_power.is_some() {
            flags |= 0x10;
        }
        if matches!(self.power_const, PowerConst::Constrained) {
            flags |= 0x08;
        }
        if self.current_cluster_channel.is_some() {
            flags |= 0x04;
        }
        flags |= (self.additional_channels.len() as u8) & 0x03;
        out[0] = flags;

        out[1] = (self.network_beacon_period.as_u8() << 4) | self.cluster_beacon_period.as_u8();

        let nc = (self.next_cluster_channel.as_u16() & 0x1FFF).to_be_bytes();
        out[2] = nc[0];
        out[3] = nc[1];

        let tt = self.time_to_next.to_be_bytes();
        out[4] = tt[0];
        out[5] = tt[1];
        out[6] = tt[2];
        out[7] = tt[3];

        let mut pos = 8;
        if let Some(p) = self.cluster_max_tx_power {
            out[pos] = p.as_u8() & 0x0F;
            pos += 1;
        }
        if let Some(ch) = self.current_cluster_channel {
            let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }
        for ch in &self.additional_channels {
            let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }
        Ok(pos)
    }

    /// Parse a Network Beacon body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer or reserved-value
    /// rejection.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 8 {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let has_tx_power = b0 & 0x10 != 0;
        let power_const = if b0 & 0x08 == 0 {
            PowerConst::Unconstrained
        } else {
            PowerConst::Constrained
        };
        let has_current_channel = b0 & 0x04 != 0;
        let extras_count = (b0 & 0x03) as usize;

        let mut need = 8;
        if has_tx_power {
            need += 1;
        }
        if has_current_channel {
            need += 2;
        }
        need += extras_count * 2;
        if buffer.len() < need {
            return Err(ParsingError::Truncated);
        }

        let b1 = buffer[1];
        let network_beacon_period =
            NetworkBeaconPeriod::try_from_u8(b1 >> 4).ok_or(ParsingError::ReservedValue)?;
        let cluster_beacon_period =
            ClusterBeaconPeriod::try_from_u8(b1 & 0x0F).ok_or(ParsingError::ReservedValue)?;

        let next_cluster_channel =
            AbsoluteChannel::new(u16::from_be_bytes([buffer[2], buffer[3]]) & 0x1FFF)
                .ok_or(ParsingError::ReservedValue)?;
        let time_to_next = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);

        let mut pos = 8;
        let cluster_max_tx_power = if has_tx_power {
            let v = TransmitPower::new(buffer[pos] & 0x0F).ok_or(ParsingError::ReservedValue)?;
            pos += 1;
            Some(v)
        } else {
            None
        };
        let current_cluster_channel = if has_current_channel {
            let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            pos += 2;
            Some(AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            None
        };
        let mut additional_channels = Vec::new();
        for _ in 0..extras_count {
            let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            pos += 2;
            let ch = AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?;
            additional_channels
                .push(ch)
                .expect("extras_count <= MAX_ADDITIONAL_CHANNELS by 2-bit field width");
        }
        let _ = pos;

        Ok(Self {
            power_const,
            network_beacon_period,
            cluster_beacon_period,
            next_cluster_channel,
            time_to_next,
            cluster_max_tx_power,
            current_cluster_channel,
            additional_channels,
        })
    }
}

impl MessageBody for NetworkBeaconParts {
    const IE_TYPE: IEType6bit = IEType6bit::NetworkBeacon;
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
    fn network_beacon_parts_round_trip_minimal() {
        let parts = NetworkBeaconParts {
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            next_cluster_channel: AbsoluteChannel::new(0x0100).unwrap(),
            time_to_next: 0x1234_5678,
            cluster_max_tx_power: None,
            current_cluster_channel: None,
            additional_channels: Vec::new(),
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, parts.encoded_len());
        assert_eq!(n, 8);

        let parsed = NetworkBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.power_const, PowerConst::Unconstrained);
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms100);
        assert_eq!(parsed.cluster_beacon_period, ClusterBeaconPeriod::Ms100);
        assert_eq!(parsed.next_cluster_channel.as_u16(), 0x0100);
        assert_eq!(parsed.time_to_next, 0x1234_5678);
        assert!(parsed.cluster_max_tx_power.is_none());
        assert!(parsed.current_cluster_channel.is_none());
        assert_eq!(parsed.additional_channels.len(), 0);
    }

    #[test]
    fn network_beacon_parts_round_trip_full() {
        let additional_channels = Vec::from_slice(&[
            AbsoluteChannel::new(0x0200).unwrap(),
            AbsoluteChannel::new(0x0300).unwrap(),
            AbsoluteChannel::new(0x0400).unwrap(),
        ])
        .unwrap();
        let parts = NetworkBeaconParts {
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms1000,
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000,
            next_cluster_channel: AbsoluteChannel::new(0x0100).unwrap(),
            time_to_next: 0xDEAD_BEEF,
            cluster_max_tx_power: Some(TransmitPower::Dbm13),
            current_cluster_channel: Some(AbsoluteChannel::new(0x00FE).unwrap()),
            additional_channels,
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 17);

        let parsed = NetworkBeaconParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.power_const, PowerConst::Constrained);
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(parsed.cluster_beacon_period, ClusterBeaconPeriod::Ms1000);
        assert_eq!(parsed.next_cluster_channel.as_u16(), 0x0100);
        assert_eq!(parsed.time_to_next, 0xDEAD_BEEF);
        assert_eq!(parsed.cluster_max_tx_power, Some(TransmitPower::Dbm13));
        assert_eq!(parsed.current_cluster_channel.unwrap().as_u16(), 0x00FE);
        assert_eq!(parsed.additional_channels.len(), 3);
        assert_eq!(parsed.additional_channels[0].as_u16(), 0x0200);
        assert_eq!(parsed.additional_channels[1].as_u16(), 0x0300);
        assert_eq!(parsed.additional_channels[2].as_u16(), 0x0400);
    }
}
