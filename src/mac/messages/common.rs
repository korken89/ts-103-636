//! Shared building blocks reused by multiple message bodies.
//!
//! Currently: [`AllocationPair`] and its wire helpers, used by Resource
//! Allocation (§6.4.3.3) and Random Access Resource (§6.4.3.4).

use crate::types::{Mu, PacketLengthType, RaLength};
use crate::{ExcessiveBitsSet, ParsingError};

/// A (start_subslot, length_type, length) triple used by Resource
/// Allocation and Random Access Resource IEs. The start_subslot is
/// 8-bit or 9-bit depending on the PHY subcarrier scaling factor
/// [`Mu`] (8-bit when `mu <= 4`, otherwise 9-bit).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct AllocationPair {
    /// First subslot of the allocation in the indicated frame.
    /// On the wire this is an 8-bit field for `mu <= 4` and a 9-bit
    /// field for `mu > 4`; valid range therefore depends on `mu` and is
    /// checked at serialize / parse time.
    pub start_subslot: u16,
    pub length_type: PacketLengthType,
    pub length: RaLength,
}

#[inline]
const fn start_subslot_byte_len(mu: Mu) -> usize {
    if mu.as_u8() <= 4 { 1 } else { 2 }
}

#[inline]
const fn start_subslot_max(mu: Mu) -> u16 {
    if mu.as_u8() <= 4 { 0xFF } else { 0x1FF }
}

/// Number of bytes [`write_pair`] / [`read_pair`] will consume for `mu`.
#[inline]
#[must_use]
pub const fn pair_byte_len(mu: Mu) -> usize {
    start_subslot_byte_len(mu) + 1
}

/// Serialize an [`AllocationPair`] into `out` for the given `mu`. Returns
/// the number of bytes written.
///
/// # Errors
///
/// Returns [`ExcessiveBitsSet`] if `pair.start_subslot` exceeds the
/// mu-dependent maximum, or if `out` is shorter than
/// [`pair_byte_len`] for this `mu`.
pub fn write_pair(
    out: &mut [u8],
    mu: Mu,
    pair: &AllocationPair,
) -> Result<usize, ExcessiveBitsSet> {
    if pair.start_subslot > start_subslot_max(mu) {
        return Err(ExcessiveBitsSet);
    }
    if out.len() < pair_byte_len(mu) {
        return Err(ExcessiveBitsSet);
    }
    let mut pos = 0;
    if mu.as_u8() <= 4 {
        out[pos] = pair.start_subslot as u8;
        pos += 1;
    } else {
        // High byte: 7 reserved bits + 1 bit of start subslot.
        out[pos] = ((pair.start_subslot >> 8) & 0x01) as u8;
        out[pos + 1] = (pair.start_subslot & 0xFF) as u8;
        pos += 2;
    }
    let lt_bit = if matches!(pair.length_type, PacketLengthType::Slot) {
        0x80
    } else {
        0
    };
    out[pos] = lt_bit | (pair.length.as_u8() & 0x7F);
    pos += 1;
    Ok(pos)
}

/// Parse an [`AllocationPair`] from `buffer` for the given `mu`.
///
/// # Errors
///
/// Returns [`ParsingError`] if the buffer is shorter than
/// [`pair_byte_len`] or if the length field is reserved.
pub fn read_pair(buffer: &[u8], mu: Mu) -> Result<AllocationPair, ParsingError> {
    let need = pair_byte_len(mu);
    if buffer.len() < need {
        return Err(ParsingError::Truncated);
    }
    let (start_subslot, length_off) = if mu.as_u8() <= 4 {
        (buffer[0] as u16, 1)
    } else {
        (((buffer[0] as u16 & 0x01) << 8) | buffer[1] as u16, 2)
    };
    let lt = if buffer[length_off] & 0x80 != 0 {
        PacketLengthType::Slot
    } else {
        PacketLengthType::Subslot
    };
    let length = RaLength::new(buffer[length_off] & 0x7F).ok_or(ParsingError::ReservedValue)?;
    Ok(AllocationPair {
        start_subslot,
        length_type: lt,
        length,
    })
}
