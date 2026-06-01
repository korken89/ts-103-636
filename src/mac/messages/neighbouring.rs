//! Neighbouring IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.6.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Neighbouring IE body (§6.4.3.6)
// ETSI TS 103 636-4, clause 6.4.3.6, Figure 6.4.3.6-1, Table 6.4.3.6-1
// ---------------------------------------------------------------------------

/// Pair of Radio Device Class μ + β co-occurring in several IE bodies
/// (Neighbouring, RD Capability additional PHY block, etc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RadioDeviceClass {
    pub mu: RdClassMu,
    pub beta: RdClassBeta,
}

/// Owned representation of a Neighbouring IE body.
///
/// Byte layout (§6.4.3.6, Figure 6.4.3.6-1):
/// * B0 bitmap: `Reserved | ID | μ | SNR | RSSI-2 | Power Const | Next Channel | TimeToNext`
///   (all 1-bit fields; Power Const is a value, the rest are presence
///   flags except Reserved).
/// * B1: `Network beacon period (4) | Cluster Beacon period (4)`.
/// * Then, in order: Long RD ID (4 B), Next Cluster Channel (2 B),
///   Time to next (4 B), RSSI-2 (1 B), SNR (1 B), Radio Device Class
///   μ/β (1 B), each present iff the corresponding bitmap bit is set.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct NeighbouringParts {
    /// Network beacon period code (4 bits).
    pub network_beacon_period: NetworkBeaconPeriod,
    /// Cluster beacon period code (4 bits).
    pub cluster_beacon_period: ClusterBeaconPeriod,
    pub power_const: PowerConst,
    pub long_rd_id: Option<LongRdId>,
    pub next_cluster_channel: Option<AbsoluteChannel>,
    /// Time until the indicated RD's cluster beacon period starts (μs).
    pub time_to_next: Option<u32>,
    pub rssi_2: Option<Rssi2Measurement>,
    pub snr: Option<SnrMeasurement>,
    pub radio_device_class: Option<RadioDeviceClass>,
}

impl NeighbouringParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let mut len = 2;
        if self.long_rd_id.is_some() {
            len += 4;
        }
        if self.next_cluster_channel.is_some() {
            len += 2;
        }
        if self.time_to_next.is_some() {
            len += 4;
        }
        if self.rssi_2.is_some() {
            len += 1;
        }
        if self.snr.is_some() {
            len += 1;
        }
        if self.radio_device_class.is_some() {
            len += 1;
        }
        len
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is too short.
    /// Period-code validity is enforced at field-construction time.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        let mut flags: u8 = 0;
        if self.long_rd_id.is_some() {
            flags |= 0x40;
        }
        if self.radio_device_class.is_some() {
            flags |= 0x20;
        }
        if self.snr.is_some() {
            flags |= 0x10;
        }
        if self.rssi_2.is_some() {
            flags |= 0x08;
        }
        if matches!(self.power_const, PowerConst::Constrained) {
            flags |= 0x04;
        }
        if self.next_cluster_channel.is_some() {
            flags |= 0x02;
        }
        if self.time_to_next.is_some() {
            flags |= 0x01;
        }
        out[0] = flags;
        out[1] = (self.network_beacon_period.as_u8() << 4) | self.cluster_beacon_period.as_u8();

        let mut pos = 2;
        if let Some(id) = self.long_rd_id {
            let raw = id.as_u32().to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            out[pos + 2] = raw[2];
            out[pos + 3] = raw[3];
            pos += 4;
        }
        if let Some(ch) = self.next_cluster_channel {
            let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }
        if let Some(t) = self.time_to_next {
            let raw = t.to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            out[pos + 2] = raw[2];
            out[pos + 3] = raw[3];
            pos += 4;
        }
        if let Some(v) = self.rssi_2 {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some(v) = self.snr {
            out[pos] = v.0;
            pos += 1;
        }
        if let Some(rdc) = self.radio_device_class {
            // [μ (3) | β (4) | Reserved (1)]
            out[pos] = (rdc.mu.as_u8() << 5) | (rdc.beta.as_u8() << 1);
            pos += 1;
        }
        Ok(pos)
    }

    /// Parse a Neighbouring body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer, zero in a
    /// `NonZero`-backed field, or a reserved Radio Device Class value.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 2 {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let id_set = b0 & 0x40 != 0;
        let mu_set = b0 & 0x20 != 0;
        let snr_set = b0 & 0x10 != 0;
        let rssi2_set = b0 & 0x08 != 0;
        let power_const = if b0 & 0x04 != 0 {
            PowerConst::Constrained
        } else {
            PowerConst::Unconstrained
        };
        let next_ch_set = b0 & 0x02 != 0;
        let ttn_set = b0 & 0x01 != 0;

        let b1 = buffer[1];
        let network_beacon_period =
            NetworkBeaconPeriod::try_from_u8(b1 >> 4).ok_or(ParsingError::ReservedValue)?;
        let cluster_beacon_period =
            ClusterBeaconPeriod::try_from_u8(b1 & 0x0F).ok_or(ParsingError::ReservedValue)?;

        let mut pos = 2;
        let long_rd_id = if id_set {
            if buffer.len() < pos + 4 {
                return Err(ParsingError::Truncated);
            }
            let raw = u32::from_be_bytes([
                buffer[pos],
                buffer[pos + 1],
                buffer[pos + 2],
                buffer[pos + 3],
            ]);
            pos += 4;
            Some(LongRdId::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            None
        };
        let next_cluster_channel = if next_ch_set {
            if buffer.len() < pos + 2 {
                return Err(ParsingError::Truncated);
            }
            let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            pos += 2;
            Some(AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            None
        };
        let time_to_next = if ttn_set {
            if buffer.len() < pos + 4 {
                return Err(ParsingError::Truncated);
            }
            let v = u32::from_be_bytes([
                buffer[pos],
                buffer[pos + 1],
                buffer[pos + 2],
                buffer[pos + 3],
            ]);
            pos += 4;
            Some(v)
        } else {
            None
        };
        let rssi_2 = if rssi2_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = Rssi2Measurement(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let snr = if snr_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = SnrMeasurement(buffer[pos]);
            pos += 1;
            Some(v)
        } else {
            None
        };
        let radio_device_class = if mu_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let byte = buffer[pos];
            let mu = RdClassMu::try_from_u8(byte >> 5).ok_or(ParsingError::ReservedValue)?;
            let beta =
                RdClassBeta::try_from_u8((byte >> 1) & 0x0F).ok_or(ParsingError::ReservedValue)?;
            pos += 1;
            Some(RadioDeviceClass { mu, beta })
        } else {
            None
        };
        let _ = pos;

        Ok(Self {
            network_beacon_period,
            cluster_beacon_period,
            power_const,
            long_rd_id,
            next_cluster_channel,
            time_to_next,
            rssi_2,
            snr,
            radio_device_class,
        })
    }
}

impl MessageBody for NeighbouringParts {
    const IE_TYPE: IEType6bit = IEType6bit::Neighbouring;
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
    fn neighbouring_minimal_round_trip() {
        let parts = NeighbouringParts {
            network_beacon_period: NetworkBeaconPeriod::Ms1000,
            cluster_beacon_period: ClusterBeaconPeriod::Ms1000,
            power_const: PowerConst::Unconstrained,
            long_rd_id: None,
            next_cluster_channel: None,
            time_to_next: None,
            rssi_2: None,
            snr: None,
            radio_device_class: None,
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 2);
        let parsed = NeighbouringParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(parsed.cluster_beacon_period, ClusterBeaconPeriod::Ms1000);
        assert!(parsed.long_rd_id.is_none());
        assert!(parsed.next_cluster_channel.is_none());
        assert!(parsed.time_to_next.is_none());
        assert!(parsed.rssi_2.is_none());
        assert!(parsed.snr.is_none());
        assert!(parsed.radio_device_class.is_none());
    }

    #[test]
    fn neighbouring_full_round_trip() {
        let parts = NeighbouringParts {
            network_beacon_period: NetworkBeaconPeriod::Ms2000,
            cluster_beacon_period: ClusterBeaconPeriod::Ms2000,
            power_const: PowerConst::Constrained,
            long_rd_id: Some(LongRdId::new(0xDEADBEEF).unwrap()),
            next_cluster_channel: Some(AbsoluteChannel::new(0x1ABC).unwrap()),
            time_to_next: Some(0xDEAD_C0DE),
            rssi_2: Some(Rssi2Measurement(0x7F)),
            snr: Some(SnrMeasurement(0x12)),
            radio_device_class: Some(RadioDeviceClass {
                mu: RdClassMu::M4,
                beta: RdClassBeta::B12,
            }),
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 2 + 4 + 2 + 4 + 1 + 1 + 1 = 15
        assert_eq!(n, 15);

        let parsed = NeighbouringParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.power_const, PowerConst::Constrained);
        assert_eq!(parsed.long_rd_id.unwrap().as_u32(), 0xDEADBEEF);
        assert_eq!(parsed.next_cluster_channel.unwrap().as_u16(), 0x1ABC);
        assert_eq!(parsed.time_to_next, Some(0xDEAD_C0DE));
        assert_eq!(parsed.rssi_2.unwrap().0, 0x7F);
        assert_eq!(parsed.snr.unwrap().0, 0x12);
        let rdc = parsed.radio_device_class.unwrap();
        assert_eq!(rdc.mu, RdClassMu::M4);
        assert_eq!(rdc.beta, RdClassBeta::B12);
    }

    #[test]
    fn neighbouring_parser_rejects_zero_long_rd_id() {
        // ID flag set with zero Long RD ID.
        // B0 = 0 1 0 0 0 0 0 0 = 0x40
        // B1 = 0
        // Long RD ID bytes all 0 -> reserved value of LongRdId
        let buf = [0x40, 0, 0, 0, 0, 0];
        assert!(NeighbouringParts::parse(&buf).is_err());
    }
}
