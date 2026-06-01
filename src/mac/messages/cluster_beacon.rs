//! Cluster Beacon message body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.3, Figure 6.4.2.3-1.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

/// Owned representation of a Cluster Beacon body.
///
/// Use [`Self::encoded_len`] to size a buffer and [`Self::serialize`] to
/// write the body into it. [`Self::parse`] decodes a payload back into a
/// `ClusterBeaconParts` directly.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ClusterBeaconParts {
    /// PHY subcarrier scaling factor in effect for this cluster. Not
    /// itself on the wire: it selects the Frame Offset field width
    /// (8 bits when mu <= 4, 16 bits otherwise, Table 6.4.2.3-1).
    pub mu: Mu,
    /// SFN (System Frame Number), byte 0.
    pub sfn: Sfn,
    /// Power-constrained indicator (bit 7 of byte 1).
    pub power_const: PowerConst,
    /// Network beacon period code (4 bits, high nibble of byte 2).
    pub network_beacon_period: NetworkBeaconPeriod,
    /// Cluster beacon period code (4 bits, low nibble of byte 2).
    pub cluster_beacon_period: ClusterBeaconPeriod,
    /// Count to trigger (4 bits, high nibble of byte 3).
    pub count_to_trigger: CountToTrigger,
    /// Relative quality (2 bits, byte 3 bits 3..=2).
    pub rel_quality: Quality,
    /// Minimum quality (2 bits, byte 3 bits 1..=0).
    pub min_quality: Quality,
    /// Optional 4-bit cluster maximum TX power field.
    pub cluster_max_tx_power: Option<TransmitPower>,
    /// Optional frame offset in subslots (8-bit on the wire when
    /// `mu <= 4`, 16-bit otherwise).
    pub frame_offset: Option<u16>,
    /// Optional next cluster channel (13-bit AbsoluteChannel).
    pub next_cluster_channel: Option<AbsoluteChannel>,
    /// Optional time-to-next-channel in microseconds (32-bit).
    pub time_to_next: Option<u32>,
}

/// Frame Offset width in bytes for the given `mu` (Table 6.4.2.3-1).
#[inline]
const fn frame_offset_byte_len(mu: Mu) -> usize {
    if mu.as_u8() <= 4 { 1 } else { 2 }
}

impl ClusterBeaconParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let mut len = 4;
        if self.cluster_max_tx_power.is_some() {
            len += 1;
        }
        if self.frame_offset.is_some() {
            len += frame_offset_byte_len(self.mu);
        }
        if self.next_cluster_channel.is_some() {
            len += 2;
        }
        if self.time_to_next.is_some() {
            len += 4;
        }
        len
    }

    /// Serialize the body into a buffer. Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is too short for
    /// [`Self::encoded_len`], or if `frame_offset` exceeds 255 while
    /// `mu <= 4` selects the 8-bit field width. All other field-width
    /// invariants are enforced at field-construction time by the typed
    /// wrappers.
    pub const fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        out[0] = self.sfn.0;

        // Byte 1 layout (§6.4.2.3):
        //   Reserved (3) | TX Power (1) | Power Const (1) |
        //   Frame Offset (1) | Next Channel (1) | Time To Next (1)
        let mut flags: u8 = 0;
        if self.cluster_max_tx_power.is_some() {
            flags |= 0x10;
        }
        if matches!(self.power_const, PowerConst::Constrained) {
            flags |= 0x08;
        }
        if self.frame_offset.is_some() {
            flags |= 0x04;
        }
        if self.next_cluster_channel.is_some() {
            flags |= 0x02;
        }
        if self.time_to_next.is_some() {
            flags |= 0x01;
        }
        out[1] = flags;

        out[2] = (self.network_beacon_period.as_u8() << 4) | self.cluster_beacon_period.as_u8();
        out[3] = (self.count_to_trigger.as_u8() << 4)
            | (self.rel_quality.as_u8() << 2)
            | self.min_quality.as_u8();

        let mut pos = 4;
        if let Some(p) = self.cluster_max_tx_power {
            out[pos] = p.as_u8() & 0x0F;
            pos += 1;
        }
        if let Some(fo) = self.frame_offset {
            if frame_offset_byte_len(self.mu) == 1 {
                if fo > 0xFF {
                    return Err(ExcessiveBitsSet);
                }
                out[pos] = fo as u8;
                pos += 1;
            } else {
                let raw = fo.to_be_bytes();
                out[pos] = raw[0];
                out[pos + 1] = raw[1];
                pos += 2;
            }
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

        Ok(pos)
    }

    /// Parse a Cluster Beacon body. `mu` is the cluster's subcarrier
    /// scaling factor (known from the PHY configuration); it selects
    /// the Frame Offset field width per Table 6.4.2.3-1.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] if the buffer is too short for the fixed
    /// 4-byte prefix plus all indicated optional fields, or if any field
    /// carries a reserved value.
    pub fn parse(buffer: &[u8], mu: Mu) -> Result<Self, ParsingError> {
        if buffer.len() < 4 {
            return Err(ParsingError::Truncated);
        }
        let b1 = buffer[1];
        let has_tx_power = b1 & 0x10 != 0;
        let has_frame_offset = b1 & 0x04 != 0;
        let has_next_channel = b1 & 0x02 != 0;
        let has_time_to_next = b1 & 0x01 != 0;

        let mut need = 4;
        if has_tx_power {
            need += 1;
        }
        if has_frame_offset {
            need += frame_offset_byte_len(mu);
        }
        if has_next_channel {
            need += 2;
        }
        if has_time_to_next {
            need += 4;
        }
        if buffer.len() < need {
            return Err(ParsingError::Truncated);
        }

        let power_const = if b1 & 0x08 == 0 {
            PowerConst::Unconstrained
        } else {
            PowerConst::Constrained
        };

        let b2 = buffer[2];
        let network_beacon_period =
            NetworkBeaconPeriod::try_from_u8(b2 >> 4).ok_or(ParsingError::ReservedValue)?;
        let cluster_beacon_period =
            ClusterBeaconPeriod::try_from_u8(b2 & 0x0F).ok_or(ParsingError::ReservedValue)?;

        let b3 = buffer[3];
        let count_to_trigger = CountToTrigger::new(b3 >> 4).ok_or(ParsingError::ReservedValue)?;
        let rel_quality = Quality::new((b3 >> 2) & 0x03).ok_or(ParsingError::ReservedValue)?;
        let min_quality = Quality::new(b3 & 0x03).ok_or(ParsingError::ReservedValue)?;

        let mut pos = 4;
        let cluster_max_tx_power = if has_tx_power {
            let v = TransmitPower::new(buffer[pos] & 0x0F).ok_or(ParsingError::ReservedValue)?;
            pos += 1;
            Some(v)
        } else {
            None
        };
        let frame_offset = if has_frame_offset {
            if frame_offset_byte_len(mu) == 1 {
                let v = u16::from(buffer[pos]);
                pos += 1;
                Some(v)
            } else {
                let v = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]);
                pos += 2;
                Some(v)
            }
        } else {
            None
        };
        let next_cluster_channel = if has_next_channel {
            let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            pos += 2;
            Some(AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            None
        };
        let time_to_next = if has_time_to_next {
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
        let _ = pos;

        Ok(Self {
            mu,
            sfn: Sfn(buffer[0]),
            power_const,
            network_beacon_period,
            cluster_beacon_period,
            count_to_trigger,
            rel_quality,
            min_quality,
            cluster_max_tx_power,
            frame_offset,
            next_cluster_channel,
            time_to_next,
        })
    }
}

impl MessageBody for ClusterBeaconParts {
    const IE_TYPE: IEType6bit = IEType6bit::ClusterBeacon;
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
    fn parse_minimal_cluster_beacon() {
        let buf = [
            0xAB, // sfn
            0x00, // flags: all zero -> unconstrained, no optionals
            0x12, // net_period=1, cluster_period=2
            0x30, // ctt=3, qrel=0, qmin=0
        ];
        let cb = ClusterBeaconParts::parse(&buf, Mu::M1).unwrap();
        assert_eq!(cb.sfn.0, 0xAB);
        assert_eq!(cb.power_const, PowerConst::Unconstrained);
        assert_eq!(cb.network_beacon_period, NetworkBeaconPeriod::Ms100);
        assert_eq!(cb.cluster_beacon_period, ClusterBeaconPeriod::Ms100);
        assert_eq!(cb.count_to_trigger.as_u8(), 3);
        assert_eq!(cb.rel_quality.as_u8(), 0);
        assert_eq!(cb.min_quality.as_u8(), 0);
        assert!(cb.cluster_max_tx_power.is_none());
        assert!(cb.frame_offset.is_none());
        assert!(cb.next_cluster_channel.is_none());
        assert!(cb.time_to_next.is_none());
    }

    #[test]
    fn parse_full_cluster_beacon() {
        let buf = [
            0xAB, // sfn
            // flags: TX Power=1 (0x10) | Power Const=1 (0x08) | FO=1 (0x04) | NCC=1 (0x02) | TTN=1 (0x01) = 0x1F
            0x1F, 0x12, // net=1, cluster=2
            0x30, // ctt=3, qrel=0, qmin=0
            0x08, // max_tx_power = 8
            0x05, // frame_offset = 5
            0x12, 0x34, // next_channel = 0x1234
            0x00, 0x00, 0x10, 0x00, // time_to_next = 4096
        ];
        let cb = ClusterBeaconParts::parse(&buf, Mu::M1).unwrap();
        assert_eq!(cb.power_const, PowerConst::Constrained);
        assert_eq!(cb.cluster_max_tx_power.unwrap().as_u8(), 8);
        assert_eq!(cb.frame_offset, Some(5));
        assert_eq!(cb.next_cluster_channel.unwrap().as_u16(), 0x1234);
        assert_eq!(cb.time_to_next, Some(0x00001000));
    }

    #[test]
    fn parse_truncated_optional_field() {
        let buf = [0xAB, 0x01, 0x12, 0x30];
        assert!(ClusterBeaconParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn parse_too_short() {
        let buf = [0xAB, 0x00, 0x12];
        assert!(ClusterBeaconParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn parts_round_trip_minimal() {
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0xAB),
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            count_to_trigger: CountToTrigger::new(0x3).unwrap(),
            rel_quality: Quality::new(0).unwrap(),
            min_quality: Quality::new(0).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: None,
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, parts.encoded_len());

        let again = ClusterBeaconParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(again.sfn.0, parts.sfn.0);
        assert_eq!(again.network_beacon_period, parts.network_beacon_period);
        assert_eq!(again.cluster_beacon_period, parts.cluster_beacon_period);
        assert_eq!(again.cluster_max_tx_power, parts.cluster_max_tx_power);
    }

    #[test]
    fn parts_round_trip_full() {
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0xAB),
            power_const: PowerConst::Constrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            count_to_trigger: CountToTrigger::new(0x3).unwrap(),
            rel_quality: Quality::new(0x2).unwrap(),
            min_quality: Quality::new(0x1).unwrap(),
            cluster_max_tx_power: Some(TransmitPower::Dbm13),
            frame_offset: Some(0x55),
            next_cluster_channel: Some(AbsoluteChannel::new(0x1234).unwrap()),
            time_to_next: Some(0x00001000),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4 + 1 + 1 + 2 + 4);

        let again = ClusterBeaconParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(again.next_cluster_channel.unwrap().as_u16(), 0x1234);
        assert_eq!(again.time_to_next, Some(0x00001000));

        // Re-emit and compare bytes
        let mut buf2 = [0u8; 16];
        let n2 = again.serialize(&mut buf2).unwrap();
        assert_eq!(&buf[..n], &buf2[..n2]);
    }

    #[test]
    fn frame_offset_is_16_bit_when_mu_above_4() {
        // Table 6.4.2.3-1: 16-bit Frame Offset when mu > 4.
        let buf = [
            0xAB, // sfn
            0x04, // flags: FO only
            0x12, // net=1, cluster=2
            0x30, // ctt=3
            0x01, 0x23, // frame_offset = 0x0123 (16-bit BE)
        ];
        let cb = ClusterBeaconParts::parse(&buf, Mu::M8).unwrap();
        assert_eq!(cb.frame_offset, Some(0x0123));

        // The same bytes parsed as mu=1 must fail: the 8-bit FO leaves
        // a trailing byte, which is a length mismatch at the IE level,
        // here visible as a different field split.
        let cb_mu1 = ClusterBeaconParts::parse(&buf[..5], Mu::M1).unwrap();
        assert_eq!(cb_mu1.frame_offset, Some(0x01));

        // Round trip in mu=8 keeps the 16-bit encoding.
        let mut out = [0u8; 16];
        let n = cb.serialize(&mut out).unwrap();
        assert_eq!(&out[..n], &buf);
    }

    #[test]
    fn frame_offset_over_255_rejected_for_low_mu() {
        let parts = ClusterBeaconParts {
            mu: Mu::M1,
            sfn: Sfn(0),
            power_const: PowerConst::Unconstrained,
            network_beacon_period: NetworkBeaconPeriod::Ms100,
            cluster_beacon_period: ClusterBeaconPeriod::Ms100,
            count_to_trigger: CountToTrigger::new(0x3).unwrap(),
            rel_quality: Quality::new(0).unwrap(),
            min_quality: Quality::new(0).unwrap(),
            cluster_max_tx_power: None,
            frame_offset: Some(0x0123),
            next_cluster_channel: None,
            time_to_next: None,
        };
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn count_to_trigger_rejects_overflow() {
        assert!(CountToTrigger::new(0x10).is_none());
    }
}
