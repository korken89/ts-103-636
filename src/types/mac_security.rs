//! MAC Security Info IE field types.

// =========================================================================
// SecurityVersion
// =========================================================================

/// 2-bit MAC Security version (Table 6.4.3.1-1). Only Mode 1 is defined;
/// values `0b01`, `0b10`, `0b11` are reserved and rejected by the
/// constructor / parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum SecurityVersion {
    Mode1 = 0b00,
}

impl SecurityVersion {
    /// Raw 2-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from the raw 2-bit field. Returns `None` for reserved values
    /// `0b01`, `0b10`, `0b11` (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::Mode1),
            _ => None,
        }
    }
}

impl From<SecurityVersion> for u8 {
    fn from(value: SecurityVersion) -> Self {
        value.as_u8()
    }
}

// =========================================================================
// KeyIndex
// =========================================================================

/// 2-bit MAC Security key index. The transmitter increments this by one
/// when a new key is taken into use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeyIndex(u8);

impl KeyIndex {
    /// Construct from a 2-bit value. Returns `None` if `value > 3`.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        if value & !0x03 != 0 {
            return None;
        }
        Some(KeyIndex(value))
    }

    /// Underlying value (0..=3).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<KeyIndex> for u8 {
    fn from(value: KeyIndex) -> Self {
        value.0
    }
}

// =========================================================================
// SecurityIvType
// =========================================================================

/// 4-bit Security IV Type for Mode 1 (Table 6.4.3.1-2).
/// ETSI TS 103 636-4, clause 6.4.3.1, Table 6.4.3.1-2. Reserved values
/// `0b0011..=0b1111` are not representable; the constructor returns
/// `None` and the parser returns [`crate::ParsingError`] for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum SecurityIvType {
    OneTimeHpc = 0b0000,
    /// Resynchronizing HPC. Initiate Mode 1 security by using this HPC
    /// value in both uplink and downlink communication.
    ResynchronizingHpc = 0b0001,
    OneTimeHpcWithRequest = 0b0010,
}

impl SecurityIvType {
    /// Raw 4-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from the raw 4-bit field. Returns `None` for reserved values
    /// `0b0011..=0b1111` (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b0000 => Some(Self::OneTimeHpc),
            0b0001 => Some(Self::ResynchronizingHpc),
            0b0010 => Some(Self::OneTimeHpcWithRequest),
            _ => None,
        }
    }
}

impl From<SecurityIvType> for u8 {
    fn from(value: SecurityIvType) -> Self {
        value.as_u8()
    }
}
