//! Resource Allocation IE body.
//!
//! ETSI TS 103 636-4, clause §6.4.3.3.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Resource Allocation IE body (§6.4.3.3)
// ETSI TS 103 636-4, clause 6.4.3.3, Figure 6.4.3.3-1, Tables 6.4.3.3-1/-2
// ---------------------------------------------------------------------------

pub use super::common::{AllocationPair, pair_byte_len, read_pair, write_pair};

/// Repetition policy for a Resource Allocation. `None` in the parent
/// structure corresponds to on-wire Repeat = `0b000` (Single).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RepeatPolicy {
    pub mode: RepeatMode,
    /// `1` means the next frame/subslot. The `NonZero<u8>` type makes
    /// the spec's "`0` is not defined" rule unrepresentable.
    pub repetition: Repetition,
    /// `0xFF` means permanent.
    pub validity: Validity,
}

/// Optional fields shared by all non-`ReleaseAll` Resource Allocation
/// variants. Each field corresponds to one bit of the bitmap; `Some`
/// means the corresponding bit is set on the wire.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AllocationOptions {
    /// On-wire `Add` bit. `false` = new/replace, `true` = additional.
    pub add: bool,
    /// On-wire `ID` bit. `Some(..)` means the IE was sent on a beacon
    /// and identifies the recipient; `None` means unicast.
    pub recipient: Option<ShortRdId>,
    /// On-wire `Repeat` field. `None` = Single (0b000); `Some(..)` =
    /// one of the four repeating modes plus Repetition + Validity bytes.
    pub repeat: Option<RepeatPolicy>,
    /// On-wire `SFN` bit. `Some(..)` means the allocation becomes valid
    /// from the indicated SFN; `None` means immediately.
    pub sfn_value: Option<u8>,
    /// On-wire `Channel` bit. `Some(..)` overrides the channel the IE
    /// was received on.
    pub channel: Option<AbsoluteChannel>,
    /// On-wire `RLF` bit. `Some(..)` overrides the default
    /// `dectScheduledResourceFailure` timer.
    pub resource_failure_timer: Option<DectScheduledResourceFailure>,
}

/// Owned representation of a Resource Allocation IE body. Wraps a
/// [`ResourceAllocationKind`] together with the PHY subcarrier scaling
/// factor [`Mu`] that determines the wire encoding of the start subslot
/// (8 vs. 9 bit). Bundling `mu` into the body removes it from the
/// `encoded_len` / `serialize` / `parse` signatures and lets this type
/// implement [`crate::mac::pdu::MessageBody`].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ResourceAllocationParts {
    /// PHY subcarrier scaling factor that determines the on-wire start
    /// subslot width.
    pub mu: Mu,
    /// Allocation-type-specific body.
    pub kind: ResourceAllocationKind,
}

/// Allocation-type-specific portion of a Resource Allocation IE body.
/// The outer enum makes the Allocation Type split unrepresentable in
/// the wrong variant: `Both` carries two pairs by construction; a
/// `Downlink` / `Uplink` carries exactly one.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResourceAllocationKind {
    /// On-wire Allocation Type = `0b00`. Body is exactly one byte;
    /// every other bitmap bit is don't-care.
    ReleaseAll,
    /// On-wire Allocation Type = `0b01`. One downlink pair.
    Downlink {
        /// Downlink allocation pair (start subslot + length).
        pair: AllocationPair,
        /// Shared optional fields gated by the bitmap.
        options: AllocationOptions,
    },
    /// On-wire Allocation Type = `0b10`. One uplink pair.
    Uplink {
        /// Uplink allocation pair (start subslot + length).
        pair: AllocationPair,
        /// Shared optional fields gated by the bitmap.
        options: AllocationOptions,
    },
    /// On-wire Allocation Type = `0b11`. Carries both DL and UL pairs.
    Both {
        /// Downlink allocation pair.
        dl: AllocationPair,
        /// Uplink allocation pair.
        ul: AllocationPair,
        /// Shared optional fields gated by the bitmap.
        options: AllocationOptions,
    },
}

#[inline]
const fn alloc_options_extra_len(o: &AllocationOptions) -> usize {
    let mut len = 0;
    if o.recipient.is_some() {
        len += 2;
    }
    if o.repeat.is_some() {
        len += 2;
    }
    if o.sfn_value.is_some() {
        len += 1;
    }
    if o.channel.is_some() {
        len += 2;
    }
    if o.resource_failure_timer.is_some() {
        len += 1;
    }
    len
}

fn write_options(out: &mut [u8], options: &AllocationOptions) -> Result<usize, ExcessiveBitsSet> {
    let mut pos = 0;
    if let Some(rd) = options.recipient {
        let bytes = rd.as_u16().to_be_bytes();
        out[pos] = bytes[0];
        out[pos + 1] = bytes[1];
        pos += 2;
    }
    if let Some(r) = &options.repeat {
        out[pos] = r.repetition.as_u8();
        out[pos + 1] = r.validity.0;
        pos += 2;
    }
    if let Some(sfn) = options.sfn_value {
        out[pos] = sfn;
        pos += 1;
    }
    if let Some(ch) = options.channel {
        let raw = (ch.as_u16() & 0x1FFF).to_be_bytes();
        out[pos] = raw[0];
        out[pos + 1] = raw[1];
        pos += 2;
    }
    if let Some(rf) = options.resource_failure_timer {
        out[pos] = rf.as_u8() & 0x0F;
        pos += 1;
    }
    Ok(pos)
}

fn read_options(
    buffer: &[u8],
    id_set: bool,
    repeat_mode: Option<RepeatMode>,
    sfn_set: bool,
    channel_set: bool,
    rlf_set: bool,
) -> Result<(AllocationOptions, usize), ParsingError> {
    let mut pos = 0;
    let recipient = if id_set {
        if buffer.len() < pos + 2 {
            return Err(ParsingError::Truncated);
        }
        let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]);
        pos += 2;
        Some(ShortRdId::new(raw).ok_or(ParsingError::ReservedValue)?)
    } else {
        None
    };
    let repeat = if let Some(mode) = repeat_mode {
        if buffer.len() < pos + 2 {
            return Err(ParsingError::Truncated);
        }
        let repetition = Repetition::new(buffer[pos]).ok_or(ParsingError::ReservedValue)?;
        let validity = Validity(buffer[pos + 1]);
        pos += 2;
        Some(RepeatPolicy {
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
    let resource_failure_timer = if rlf_set {
        if buffer.len() < pos + 1 {
            return Err(ParsingError::Truncated);
        }
        let v = buffer[pos] & 0x0F;
        pos += 1;
        Some(DectScheduledResourceFailure::try_from_u8(v).ok_or(ParsingError::ReservedValue)?)
    } else {
        None
    };
    Ok((
        AllocationOptions {
            add: false, // filled by caller from B0
            recipient,
            repeat,
            sfn_value,
            channel,
            resource_failure_timer,
        },
        pos,
    ))
}

impl ResourceAllocationParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        match &self.kind {
            ResourceAllocationKind::ReleaseAll => 1,
            ResourceAllocationKind::Downlink { options, .. }
            | ResourceAllocationKind::Uplink { options, .. } => {
                2 + pair_byte_len(self.mu) + alloc_options_extra_len(options)
            }
            ResourceAllocationKind::Both { options, .. } => {
                2 + 2 * pair_byte_len(self.mu) + alloc_options_extra_len(options)
            }
        }
    }

    /// Serialize the body. The 8-vs-9-bit Start Subslot encoding is
    /// determined by [`Self::mu`].
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] for short buffer, a Start Subslot
    /// exceeding the μ-dependent maximum (255 or 511), or a Length
    /// exceeding 7 bits.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }
        let mu = self.mu;

        match &self.kind {
            ResourceAllocationKind::ReleaseAll => {
                // B0 = Allocation type (0b00) shifted to high bits; rest 0.
                out[0] = AllocationType::ReleaseAll.as_u8() << 6;
                Ok(1)
            }
            _ => {
                // Common bitmap construction.
                let (alloc_type, pair_one, pair_two, options) = match &self.kind {
                    ResourceAllocationKind::ReleaseAll => unreachable!(),
                    ResourceAllocationKind::Downlink { pair, options } => {
                        (AllocationType::Downlink, pair, None, options)
                    }
                    ResourceAllocationKind::Uplink { pair, options } => {
                        (AllocationType::Uplink, pair, None, options)
                    }
                    ResourceAllocationKind::Both { dl, ul, options } => {
                        (AllocationType::DownlinkAndUplink, dl, Some(ul), options)
                    }
                };
                let add_bit = if options.add { 0x20 } else { 0 };
                let id_bit = if options.recipient.is_some() { 0x10 } else { 0 };
                let repeat_bits =
                    options.repeat.as_ref().map(|p| p.mode.as_u8()).unwrap_or(0) & 0x07;
                let sfn_bit = if options.sfn_value.is_some() { 0x01 } else { 0 };
                out[0] =
                    (alloc_type.as_u8() << 6) | add_bit | id_bit | (repeat_bits << 1) | sfn_bit;
                let channel_bit = if options.channel.is_some() { 0x80 } else { 0 };
                let rlf_bit = if options.resource_failure_timer.is_some() {
                    0x40
                } else {
                    0
                };
                out[1] = channel_bit | rlf_bit;

                let mut pos = 2;
                pos += write_pair(&mut out[pos..], mu, pair_one)?;
                if let Some(p2) = pair_two {
                    pos += write_pair(&mut out[pos..], mu, p2)?;
                }
                pos += write_options(&mut out[pos..], options)?;
                Ok(pos)
            }
        }
    }

    /// Parse a Resource Allocation body using the given PHY `mu`. The
    /// returned `Self` carries `mu` so subsequent `encoded_len` /
    /// `serialize` calls don't need it as an argument.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer, reserved Repeat value
    /// (`0b101..=0b111`), reserved `dectScheduledResourceFailure` code,
    /// reserved Length value, zero Repetition, or zero in a
    /// `NonZero`-backed field.
    pub fn parse(buffer: &[u8], mu: Mu) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let alloc_type = AllocationType::try_from_u8(b0 >> 6).ok_or(ParsingError::ReservedValue)?;
        if matches!(alloc_type, AllocationType::ReleaseAll) {
            return Ok(ResourceAllocationParts {
                mu,
                kind: ResourceAllocationKind::ReleaseAll,
            });
        }
        if buffer.len() < 2 {
            return Err(ParsingError::Truncated);
        }
        let b1 = buffer[1];
        let add = b0 & 0x20 != 0;
        let id_set = b0 & 0x10 != 0;
        let repeat_raw = (b0 >> 1) & 0x07;
        let sfn_set = b0 & 0x01 != 0;
        let channel_set = b1 & 0x80 != 0;
        let rlf_set = b1 & 0x40 != 0;

        let repeat_mode = if repeat_raw == 0 {
            None
        } else {
            Some(RepeatMode::try_from_u8(repeat_raw).ok_or(ParsingError::ReservedValue)?)
        };

        let mut pos = 2;
        let p1 = read_pair(&buffer[pos..], mu)?;
        pos += pair_byte_len(mu);
        let p2 = if matches!(alloc_type, AllocationType::DownlinkAndUplink) {
            let p = read_pair(&buffer[pos..], mu)?;
            pos += pair_byte_len(mu);
            Some(p)
        } else {
            None
        };
        let (mut options, _) = read_options(
            &buffer[pos..],
            id_set,
            repeat_mode,
            sfn_set,
            channel_set,
            rlf_set,
        )?;
        options.add = add;

        let kind = match (alloc_type, p2) {
            (AllocationType::Downlink, None) => {
                ResourceAllocationKind::Downlink { pair: p1, options }
            }
            (AllocationType::Uplink, None) => ResourceAllocationKind::Uplink { pair: p1, options },
            (AllocationType::DownlinkAndUplink, Some(ul)) => ResourceAllocationKind::Both {
                dl: p1,
                ul,
                options,
            },
            _ => return Err(ParsingError::ReservedValue),
        };
        Ok(ResourceAllocationParts { mu, kind })
    }
}

impl MessageBody for ResourceAllocationParts {
    const IE_TYPE: IEType6bit = IEType6bit::ResourceAllocation;
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
    fn resource_allocation_release_all_round_trip() {
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::ReleaseAll,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0b0000_0000); // alloc type 0b00 in high bits

        let parsed = ResourceAllocationParts::parse(&buf[..n], Mu::M1).unwrap();
        assert!(matches!(parsed.kind, ResourceAllocationKind::ReleaseAll));
    }

    #[test]
    fn resource_allocation_downlink_minimal_mu1_round_trip() {
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::Downlink {
                pair: AllocationPair {
                    start_subslot: 17,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(5).unwrap(),
                },
                options: AllocationOptions::default(),
            },
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 2 (bitmap) + 1 (start subslot, 8-bit) + 1 (length type+length) = 4
        assert_eq!(n, 4);

        let parsed = ResourceAllocationParts::parse(&buf[..n], Mu::M1).unwrap();
        match parsed.kind {
            ResourceAllocationKind::Downlink { pair, options } => {
                assert_eq!(pair.start_subslot, 17);
                assert!(matches!(pair.length_type, PacketLengthType::Subslot));
                assert_eq!(pair.length.as_u8(), 5);
                assert!(!options.add);
                assert!(options.recipient.is_none());
                assert!(options.repeat.is_none());
                assert!(options.sfn_value.is_none());
                assert!(options.channel.is_none());
                assert!(options.resource_failure_timer.is_none());
            }
            _ => panic!("expected downlink"),
        }
    }

    #[test]
    fn resource_allocation_both_with_all_options_mu8_round_trip() {
        let parts = ResourceAllocationParts {
            mu: Mu::M8,
            kind: ResourceAllocationKind::Both {
                dl: AllocationPair {
                    start_subslot: 0x100, // requires 9-bit (mu > 4)
                    length_type: PacketLengthType::Slot,
                    length: RaLength::new(3).unwrap(),
                },
                ul: AllocationPair {
                    start_subslot: 0x123,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(7).unwrap(),
                },
                options: AllocationOptions {
                    add: true,
                    recipient: Some(ShortRdId::new(0xABCD).unwrap()),
                    repeat: Some(RepeatPolicy {
                        mode: RepeatMode::PerFrame,
                        repetition: Repetition::new(4).unwrap(),
                        validity: Validity(100),
                    }),
                    sfn_value: Some(42),
                    channel: Some(AbsoluteChannel::new(0x1234).unwrap()),
                    resource_failure_timer: Some(DectScheduledResourceFailure::Ms200),
                },
            },
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 2 (bitmap) + 3 (DL: 2 ss + 1 len) + 3 (UL) + 2 (recipient) + 2 (repeat) + 1 (sfn) + 2 (channel) + 1 (rlf) = 16
        assert_eq!(n, 16);

        let parsed = ResourceAllocationParts::parse(&buf[..n], Mu::M8).unwrap();
        match parsed.kind {
            ResourceAllocationKind::Both { dl, ul, options } => {
                assert_eq!(dl.start_subslot, 0x100);
                assert!(matches!(dl.length_type, PacketLengthType::Slot));
                assert_eq!(dl.length.as_u8(), 3);
                assert_eq!(ul.start_subslot, 0x123);
                assert!(matches!(ul.length_type, PacketLengthType::Subslot));
                assert_eq!(ul.length.as_u8(), 7);
                assert!(options.add);
                assert_eq!(options.recipient.unwrap().as_u16(), 0xABCD);
                let rp = options.repeat.unwrap();
                assert!(matches!(rp.mode, RepeatMode::PerFrame));
                assert_eq!(rp.repetition.as_u8(), 4);
                assert_eq!(rp.validity.0, 100);
                assert_eq!(options.sfn_value, Some(42));
                assert_eq!(options.channel.unwrap().as_u16(), 0x1234);
                assert_eq!(
                    options.resource_failure_timer,
                    Some(DectScheduledResourceFailure::Ms200)
                );
            }
            _ => panic!("expected both"),
        }
    }

    #[test]
    fn resource_allocation_serialize_rejects_start_subslot_overflow_mu1() {
        // mu=1 supports 8-bit start subslot (0..=255); 256 overflows.
        let parts = ResourceAllocationParts {
            mu: Mu::M1,
            kind: ResourceAllocationKind::Downlink {
                pair: AllocationPair {
                    start_subslot: 256,
                    length_type: PacketLengthType::Subslot,
                    length: RaLength::new(0).unwrap(),
                },
                options: AllocationOptions::default(),
            },
        };
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn resource_allocation_parser_rejects_reserved_repeat() {
        // alloc=DL, repeat=0b101 reserved.
        // B0 = 01 00 101 0 = 0b0100_1010
        let buf = [0b0100_1010, 0, 0, 0];
        assert!(ResourceAllocationParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn resource_allocation_parser_rejects_reserved_rlf_timer() {
        // alloc=DL, no add, no id, no repeat, no sfn.
        // B0 = 01 00 000 0 = 0b0100_0000
        // B1 = RLF=1 only (0x40).
        // pair = 1+1 bytes (mu=1) = ss=0, len=0
        // RLF byte = 0 (reserved).
        let buf = [0b0100_0000, 0x40, 0, 0, 0];
        assert!(ResourceAllocationParts::parse(&buf, Mu::M1).is_err());
    }

    #[test]
    fn resource_allocation_parser_rejects_zero_repetition() {
        // alloc=DL, repeat=PerFrame (001), no others.
        // B0 = 01 00 001 0 = 0b0100_0010
        // B1 = 0
        // pair = 2 bytes
        // repetition byte = 0 (invalid), validity = 0
        let buf = [0b0100_0010, 0, 0, 0, 0, 0];
        assert!(ResourceAllocationParts::parse(&buf, Mu::M1).is_err());
    }
}
