//! Random Access Resource IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.4.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

use super::common::{AllocationPair, pair_byte_len, read_pair, write_pair};

// ---------------------------------------------------------------------------
// Random Access Resource IE body (§6.4.3.4)
// ETSI TS 103 636-4, clause 6.4.3.4, Figure 6.4.3.4-1, Table 6.4.3.4-1
// ---------------------------------------------------------------------------

/// Repetition / Validity carried by a Random Access Resource IE when
/// the 2-bit Repeat field is non-zero. `None` in the parent struct
/// corresponds to on-wire Repeat = `0b00` (Single).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RachRepeatPolicy {
    pub mode: RachRepeatMode,
    /// `1` means the next frame/subslot. The `NonZero<u8>` type makes
    /// the spec's "`0` is not defined" rule unrepresentable.
    pub repetition: Repetition,
    /// `0xFF` means permanent.
    pub validity: Validity,
}

/// Owned representation of a Random Access Resource IE body. Carries
/// the PHY subcarrier scaling factor [`Mu`] alongside the body fields
/// so that [`Self::encoded_len`] and [`Self::serialize`] don't need it
/// as a parameter, which lets this type implement
/// [`crate::mac::pdu::MessageBody`].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RandomAccessResourceParts {
    /// PHY subcarrier scaling factor that determines the on-wire start
    /// subslot width.
    pub mu: Mu,
    /// Start subslot + Length type + Length (same encoding as a
    /// Resource Allocation pair). Start subslot is 8-bit or 9-bit
    /// depending on [`Self::mu`].
    pub pair: AllocationPair,
    pub max_length_type: PacketLengthType,
    pub max_rach_length: MaxRachLength,
    pub cwmin_sig: Cwsig,
    /// On-wire `DECT_Delay` bit (1 bit). `false` = response window
    /// starts from the subslot `n + HARQ feedback delay + 1`;
    /// `true` = response window starts 0.5 frames after the start of
    /// the Random Access transmission.
    pub dect_delay: bool,
    pub response_window: ResponseWindow,
    pub cwmax_sig: Cwsig,
    /// `Some(..)` iff the on-wire Repeat field is 0b01 or 0b10.
    pub repeat: Option<RachRepeatPolicy>,
    /// `Some(..)` iff the SFN bit is set.
    pub sfn_value: Option<u8>,
    /// `Some(..)` iff the Channel bit is set.
    pub channel: Option<AbsoluteChannel>,
    /// `Some(..)` iff the Chan_2 bit is set. Indicates the channel for
    /// the random access response message when it differs from the
    /// channel where the IE was received.
    pub channel_2: Option<AbsoluteChannel>,
}

impl RandomAccessResourceParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        let mut len = 1 + pair_byte_len(self.mu) + 2; // bitmap + pair + (max + dect-delay) bytes
        if self.repeat.is_some() {
            len += 2;
        }
        if self.sfn_value.is_some() {
            len += 1;
        }
        if self.channel.is_some() {
            len += 2;
        }
        if self.channel_2.is_some() {
            len += 2;
        }
        len
    }

    /// Serialize the body. The 8-vs-9-bit Start Subslot encoding is
    /// determined by [`Self::mu`].
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] for short buffer or for a Start
    /// Subslot that exceeds the μ-dependent maximum.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        let mu = self.mu;

        // B0: bitmap
        let repeat_bits = self.repeat.as_ref().map(|p| p.mode.as_u8()).unwrap_or(0) & 0x03;
        let sfn_bit = if self.sfn_value.is_some() { 0x04 } else { 0 };
        let channel_bit = if self.channel.is_some() { 0x02 } else { 0 };
        let chan2_bit = if self.channel_2.is_some() { 0x01 } else { 0 };
        out[0] = (repeat_bits << 3) | sfn_bit | channel_bit | chan2_bit;

        let mut pos = 1;
        pos += write_pair(&mut out[pos..], mu, &self.pair)?;

        let max_lt_bit = if matches!(self.max_length_type, PacketLengthType::Slot) {
            0x80
        } else {
            0
        };
        out[pos] =
            max_lt_bit | ((self.max_rach_length.as_u8() & 0x0F) << 3) | self.cwmin_sig.as_u8();
        pos += 1;
        let dd_bit = if self.dect_delay { 0x80 } else { 0 };
        out[pos] = dd_bit | ((self.response_window.as_u8() & 0x0F) << 3) | self.cwmax_sig.as_u8();
        pos += 1;

        if let Some(r) = &self.repeat {
            out[pos] = r.repetition.as_u8();
            out[pos + 1] = r.validity.0;
            pos += 2;
        }
        if let Some(sfn) = self.sfn_value {
            out[pos] = sfn;
            pos += 1;
        }
        if let Some(ch) = self.channel {
            let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }
        if let Some(ch) = self.channel_2 {
            let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = raw[0];
            out[pos + 1] = raw[1];
            pos += 2;
        }

        Ok(pos)
    }

    /// Parse a Random Access Resource body using the given PHY `mu`.
    /// The returned `Self` carries `mu` so subsequent `encoded_len` /
    /// `serialize` calls don't need it as an argument.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer, reserved Repeat
    /// (`0b11`), reserved Length value, zero Repetition, or zero in a
    /// `NonZero`-backed field.
    pub fn parse(buffer: &[u8], mu: Mu) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let repeat_raw = (b0 >> 3) & 0x03;
        let sfn_set = b0 & 0x04 != 0;
        let channel_set = b0 & 0x02 != 0;
        let chan2_set = b0 & 0x01 != 0;

        let repeat_mode = if repeat_raw == 0 {
            None
        } else {
            Some(RachRepeatMode::try_from_u8(repeat_raw).ok_or(ParsingError::ReservedValue)?)
        };

        // Mandatory tail: pair + 2 bytes.
        let pair_len = pair_byte_len(mu);
        let need_mandatory = 1 + pair_len + 2;
        if buffer.len() < need_mandatory {
            return Err(ParsingError::Truncated);
        }
        let mut pos = 1;
        let pair = read_pair(&buffer[pos..], mu)?;
        pos += pair_len;
        let b_max = buffer[pos];
        let max_lt = if b_max & 0x80 != 0 {
            PacketLengthType::Slot
        } else {
            PacketLengthType::Subslot
        };
        let max_rach_length =
            MaxRachLength::new((b_max >> 3) & 0x0F).ok_or(ParsingError::ReservedValue)?;
        let cwmin_sig = Cwsig::new(b_max & 0x07).ok_or(ParsingError::ReservedValue)?;
        pos += 1;
        let b_dd = buffer[pos];
        let dect_delay = b_dd & 0x80 != 0;
        let response_window =
            ResponseWindow::new((b_dd >> 3) & 0x0F).ok_or(ParsingError::ReservedValue)?;
        let cwmax_sig = Cwsig::new(b_dd & 0x07).ok_or(ParsingError::ReservedValue)?;
        pos += 1;

        let repeat = if let Some(mode) = repeat_mode {
            if buffer.len() < pos + 2 {
                return Err(ParsingError::Truncated);
            }
            let repetition = Repetition::new(buffer[pos]).ok_or(ParsingError::ReservedValue)?;
            let validity = Validity(buffer[pos + 1]);
            pos += 2;
            Some(RachRepeatPolicy {
                mode,
                repetition,
                validity,
            })
        } else {
            None
        };
        let sfn_value = if sfn_set {
            if buffer.len() < pos + 1 {
                return Err(ParsingError::Truncated);
            }
            let v = buffer[pos];
            pos += 1;
            Some(v)
        } else {
            None
        };
        let channel = if channel_set {
            if buffer.len() < pos + 2 {
                return Err(ParsingError::Truncated);
            }
            let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            pos += 2;
            Some(AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            None
        };
        let channel_2 = if chan2_set {
            if buffer.len() < pos + 2 {
                return Err(ParsingError::Truncated);
            }
            let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            pos += 2;
            Some(AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?)
        } else {
            None
        };
        let _ = pos;

        Ok(Self {
            mu,
            pair,
            max_length_type: max_lt,
            max_rach_length,
            cwmin_sig,
            dect_delay,
            response_window,
            cwmax_sig,
            repeat,
            sfn_value,
            channel,
            channel_2,
        })
    }
}

impl MessageBody for RandomAccessResourceParts {
    const IE_TYPE: IEType6bit = IEType6bit::RandomAccessResource;
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
    fn rach_minimal(mu: Mu) -> RandomAccessResourceParts {
        RandomAccessResourceParts {
            mu,
            pair: AllocationPair {
                start_subslot: 1,
                length_type: PacketLengthType::Subslot,
                length: RaLength::new(1).unwrap(),
            },
            max_length_type: PacketLengthType::Subslot,
            max_rach_length: MaxRachLength::new(2).unwrap(),
            cwmin_sig: Cwsig::new(1).unwrap(),
            dect_delay: false,
            response_window: ResponseWindow::new(3).unwrap(),
            cwmax_sig: Cwsig::new(2).unwrap(),
            repeat: None,
            sfn_value: None,
            channel: None,
            channel_2: None,
        }
    }

    #[test]
    fn rach_minimal_mu1_round_trip() {
        let parts = rach_minimal(Mu::M1);
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (bitmap) + 1 (ss) + 1 (len) + 2 (max+dd) = 5
        assert_eq!(n, 5);
        let parsed = RandomAccessResourceParts::parse(&buf[..n], Mu::M1).unwrap();
        assert_eq!(parsed.pair.start_subslot, 1);
        assert_eq!(parsed.pair.length.as_u8(), 1);
        assert_eq!(parsed.max_rach_length.as_u8(), 2);
        assert_eq!(parsed.cwmin_sig.as_u8(), 1);
        assert!(!parsed.dect_delay);
        assert_eq!(parsed.response_window.as_u8(), 3);
        assert_eq!(parsed.response_window.subslots(), 4);
        assert_eq!(parsed.cwmax_sig.as_u8(), 2);
        assert!(parsed.repeat.is_none());
        assert!(parsed.sfn_value.is_none());
        assert!(parsed.channel.is_none());
        assert!(parsed.channel_2.is_none());
    }

    #[test]
    fn rach_full_options_mu8_round_trip() {
        let mut parts = rach_minimal(Mu::M8);
        parts.dect_delay = true;
        parts.repeat = Some(RachRepeatPolicy {
            mode: RachRepeatMode::PerSubslot,
            repetition: Repetition::new(2).unwrap(),
            validity: Validity(0xFF),
        });
        parts.sfn_value = Some(0x80);
        parts.channel = Some(AbsoluteChannel::new(0x1FFF).unwrap());
        parts.channel_2 = Some(AbsoluteChannel::new(0x0001).unwrap());
        parts.pair.start_subslot = 0x1AB; // 9-bit at mu=8

        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (bitmap) + 2 (ss) + 1 (len) + 2 (max+dd) + 2 (repeat) + 1 (sfn) + 2 (ch) + 2 (ch2) = 13
        assert_eq!(n, 13);

        let parsed = RandomAccessResourceParts::parse(&buf[..n], Mu::M8).unwrap();
        assert_eq!(parsed.pair.start_subslot, 0x1AB);
        assert!(parsed.dect_delay);
        let r = parsed.repeat.unwrap();
        assert!(matches!(r.mode, RachRepeatMode::PerSubslot));
        assert_eq!(r.repetition.as_u8(), 2);
        assert_eq!(r.validity.0, 0xFF);
        assert_eq!(parsed.sfn_value, Some(0x80));
        assert_eq!(parsed.channel.unwrap().as_u16(), 0x1FFF);
        assert_eq!(parsed.channel_2.unwrap().as_u16(), 0x0001);
    }

    #[test]
    fn rach_parser_rejects_reserved_repeat() {
        // bitmap with Repeat = 0b11 (reserved). All other bits 0.
        // B0 = 000 11 0 0 0 = 0b0001_1000
        let buf = [0b0001_1000, 0, 0, 0, 0];
        assert!(RandomAccessResourceParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn rach_parser_rejects_zero_repetition() {
        // Repeat = 01 (PerFrame), no other options.
        // B0 = 000 01 0 0 0 = 0b0000_1000
        // mu=1: pair=2 bytes, then max+dd 2 bytes, then repetition(0)+validity
        let buf = [0b0000_1000, 0, 0, 0, 0, 0, 0];
        assert!(RandomAccessResourceParts::parse(&buf, Mu::M1).is_err());
    }
}
