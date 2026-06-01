//! MAC header and framing field types.

use crate::constants;
// =========================================================================
// MacExt
// =========================================================================

/// 2-bit MAC Extension field of an Information Element header. Variant
/// numeric values match the on-wire encoding and mirror
/// [`crate::constants::mac_ext`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum MacExt {
    /// IE type defines length (no explicit length field).
    NoLength = constants::mac_ext::NO_LENGTH,
    /// 8-bit length field follows the head byte.
    Length8Bit = constants::mac_ext::LENGTH_8BIT,
    /// 16-bit length field follows the head byte.
    Length16Bit = constants::mac_ext::LENGTH_16BIT,
    /// Short IE: 5-bit type with 1-bit length packed into the head byte.
    ShortIe = constants::mac_ext::SHORT_IE,
}

impl MacExt {
    /// Raw u8 value carrying the spec encoding.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Construct from a raw u8. Returns `None` for reserved values.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            constants::mac_ext::NO_LENGTH => Some(Self::NoLength),
            constants::mac_ext::LENGTH_8BIT => Some(Self::Length8Bit),
            constants::mac_ext::LENGTH_16BIT => Some(Self::Length16Bit),
            constants::mac_ext::SHORT_IE => Some(Self::ShortIe),
            _ => None,
        }
    }
}

impl From<MacExt> for u8 {
    fn from(value: MacExt) -> Self {
        value.as_u8()
    }
}

// ---------------------------------------------------------------------------
// MacHeaderTypeKind - 4-bit MAC Header Type field
// ETSI TS 103 636-4, clause 6.3.2, Table 6.3.2-2
// ---------------------------------------------------------------------------

// =========================================================================
// MacHeaderTypeKind
// =========================================================================

/// 4-bit MAC Header Type field (Table 6.3.2-2). Variant numeric values
/// match the on-wire encoding and mirror
/// [`crate::constants::mac_header_type`]. Values 4..=14 are reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum MacHeaderTypeKind {
    DataMacPdu = constants::mac_header_type::DATA_MAC_PDU,
    Beacon = constants::mac_header_type::BEACON,
    Unicast = constants::mac_header_type::UNICAST,
    RdBroadcast = constants::mac_header_type::RD_BROADCAST,
    Escape = constants::mac_header_type::ESCAPE,
}

impl MacHeaderTypeKind {
    /// Raw u8 value carrying the spec encoding.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Construct from a raw u8. Returns `None` for reserved values.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            constants::mac_header_type::DATA_MAC_PDU => Some(Self::DataMacPdu),
            constants::mac_header_type::BEACON => Some(Self::Beacon),
            constants::mac_header_type::UNICAST => Some(Self::Unicast),
            constants::mac_header_type::RD_BROADCAST => Some(Self::RdBroadcast),
            constants::mac_header_type::ESCAPE => Some(Self::Escape),
            _ => None,
        }
    }
}

impl From<MacHeaderTypeKind> for u8 {
    fn from(value: MacHeaderTypeKind) -> Self {
        value.as_u8()
    }
}

// ---------------------------------------------------------------------------
// HeaderFormat - 3-bit Header Format field of a PCC
// ETSI TS 103 636-4, clause 6.2.1
// ---------------------------------------------------------------------------

// =========================================================================
// HeaderFormat
// =========================================================================

/// 3-bit Header Format field of a PCC. Variant numeric values match the
/// on-wire encoding and mirror [`crate::constants::header_format`].
/// Values 2..=7 are reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum HeaderFormat {
    Format000 = constants::header_format::FORMAT_000,
    Format001 = constants::header_format::FORMAT_001,
}

impl HeaderFormat {
    /// Raw u8 value carrying the spec encoding.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Construct from a raw u8. Returns `None` for reserved values.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            constants::header_format::FORMAT_000 => Some(Self::Format000),
            constants::header_format::FORMAT_001 => Some(Self::Format001),
            _ => None,
        }
    }
}

impl From<HeaderFormat> for u8 {
    fn from(value: HeaderFormat) -> Self {
        value.as_u8()
    }
}

// ---------------------------------------------------------------------------
// SequenceNumber - 12-bit sequence number
// ETSI TS 103 636-4, clauses 6.3.3.1, 6.3.3.3, 6.3.3.4 (Figures *-1).
// ---------------------------------------------------------------------------

// =========================================================================
// SequenceNumber
// =========================================================================

/// 12-bit sequence number carried by Data MAC PDU, Unicast, and RD
/// Broadcast common headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SequenceNumber(u16);

impl SequenceNumber {
    /// Largest valid sequence number (`0xFFF`).
    pub const MAX: Self = SequenceNumber(0x0FFF);

    /// Construct from `value`. Returns `None` if `value` does not fit in
    /// 12 bits.
    #[must_use]
    #[inline]
    pub const fn new(value: u16) -> Option<Self> {
        if value & !0x0FFF != 0 {
            return None;
        }
        Some(SequenceNumber(value))
    }

    /// Underlying 12-bit value.
    #[must_use]
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

impl From<SequenceNumber> for u16 {
    fn from(value: SequenceNumber) -> Self {
        value.0
    }
}

// ---------------------------------------------------------------------------
// SetupCause - 3-bit Association Setup Cause
// ETSI TS 103 636-4, clause 6.4.2.4, Table 6.4.2.4-2
// ---------------------------------------------------------------------------

// =========================================================================
// MacSecurity
// =========================================================================

/// MAC Security field of the MAC header type (Table 6.3.2-1). The 2-bit
/// raw values are mirrored in [`crate::constants::mac_security`].
/// `u8::from(self)` gives the raw field; `MacSecurity::try_from(u8)`
/// parses one, rejecting the reserved value 0b11. Both directions are
/// also available as `const fn` ([`Self::as_u8`], [`Self::try_from_u8`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum MacSecurity {
    NotUsed = constants::mac_security::NOT_USED,
    UsedNoIe = constants::mac_security::USED_NO_IE,
    UsedWithIe = constants::mac_security::USED_WITH_IE,
}

impl MacSecurity {
    /// Raw 2-bit field value (`const`).
    /// Raw u8 value carrying the spec encoding.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from the raw 2-bit field, returning `None` for the reserved
    /// value 0b11 (`const`).
    /// Construct from a raw u8. Returns `None` for reserved values.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            constants::mac_security::NOT_USED => Some(Self::NotUsed),
            constants::mac_security::USED_NO_IE => Some(Self::UsedNoIe),
            constants::mac_security::USED_WITH_IE => Some(Self::UsedWithIe),
            _ => None,
        }
    }
}

impl From<MacSecurity> for u8 {
    fn from(value: MacSecurity) -> Self {
        value.as_u8()
    }
}

// ---------------------------------------------------------------------------
// PacketLengthType
// ETSI TS 103 636-4, clause 6.2.1, Tables 6.2.1-1 / 6.2.1-2 / 6.2.1-2a
// ---------------------------------------------------------------------------

// =========================================================================
// PacketLengthType
// =========================================================================

/// Indicates whether [`PacketLength`] counts subslots or slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum PacketLengthType {
    Subslot = 0,
    Slot = 1,
}

impl PacketLengthType {
    /// Construct from the 1-bit on-wire encoding.
    #[must_use]
    pub const fn from_bit(bit: bool) -> Self {
        if bit {
            PacketLengthType::Slot
        } else {
            PacketLengthType::Subslot
        }
    }

    /// Encode as the 1-bit on-wire value.
    #[must_use]
    pub const fn to_bit(self) -> bool {
        matches!(self, PacketLengthType::Slot)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for PacketLengthType {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self {
            PacketLengthType::Subslot => defmt::write!(f, "subslot"),
            PacketLengthType::Slot => defmt::write!(f, "slot"),
        }
    }
}

// ---------------------------------------------------------------------------
// PacketLength - signalled packet length (value+1 semantics)
// ETSI TS 103 636-4, clause 6.2.1
// ---------------------------------------------------------------------------

// =========================================================================
// PacketLength
// =========================================================================

/// 4-bit packet length field. `raw` is 0..=15, the actual count of subslots
/// (or slots, depending on [`PacketLengthType`]) is `raw + 1`, so units
/// span 1..=16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketLength(u8);

impl PacketLength {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(raw: u8) -> Option<Self> {
        if raw <= 15 {
            Some(PacketLength(raw))
        } else {
            None
        }
    }

    /// Raw 4-bit field value.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// The actual number of subslots or slots (raw + 1).
    #[must_use]
    pub const fn units(self) -> u8 {
        self.0 + 1
    }
}

impl From<PacketLength> for u8 {
    fn from(value: PacketLength) -> Self {
        value.0
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for PacketLength {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{}", self.units());
    }
}

// ---------------------------------------------------------------------------
// NetworkId24 - most significant 24 bits of a Network ID
// ETSI TS 103 636-4, clause 4.2.3.1
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_length_units_one_to_sixteen() {
        for raw in 0u8..16 {
            let pl = PacketLength::new(raw).unwrap();
            assert_eq!(pl.units(), raw + 1);
        }
        assert!(PacketLength::new(16).is_none());
    }
}
