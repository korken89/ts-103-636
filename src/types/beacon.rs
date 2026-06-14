//! Cluster / Network beacon field types.

// =========================================================================
// Sfn
// =========================================================================

/// 8-bit System Frame Number. Full range 0..=255 valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Sfn(pub u8);

// =========================================================================
// PowerConst
// =========================================================================

/// Power-constrained indicator. Shared by Cluster Beacon (clause 6.4.2.3),
/// Network Beacon (6.4.2.2), Association Request (6.4.2.4), and
/// Neighbouring (6.4.3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum PowerConst {
    Unconstrained,
    Constrained,
}

// =========================================================================
// NetworkBeaconPeriod
// =========================================================================

/// 4-bit Network beacon period code. Variant names encode the period in
/// milliseconds (Table 6.4.2.2-1). Reserved values 7..=15 are not
/// representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum NetworkBeaconPeriod {
    Ms50 = 0,
    Ms100 = 1,
    Ms500 = 2,
    Ms1000 = 3,
    Ms1500 = 4,
    Ms2000 = 5,
    Ms4000 = 6,
}

impl NetworkBeaconPeriod {
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
            0 => Some(Self::Ms50),
            1 => Some(Self::Ms100),
            2 => Some(Self::Ms500),
            3 => Some(Self::Ms1000),
            4 => Some(Self::Ms1500),
            5 => Some(Self::Ms2000),
            6 => Some(Self::Ms4000),
            _ => None,
        }
    }
    /// Period in milliseconds.
    #[must_use]
    pub const fn milliseconds(self) -> u16 {
        match self {
            Self::Ms50 => 50,
            Self::Ms100 => 100,
            Self::Ms500 => 500,
            Self::Ms1000 => 1000,
            Self::Ms1500 => 1500,
            Self::Ms2000 => 2000,
            Self::Ms4000 => 4000,
        }
    }
}

// =========================================================================
// ClusterBeaconPeriod
// =========================================================================

/// 4-bit Cluster beacon period code. Variant names encode the period
/// in milliseconds (Table 6.4.2.2-1, extended set). Reserved values
/// 11..=15 are not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum ClusterBeaconPeriod {
    Ms10 = 0,
    Ms50 = 1,
    Ms100 = 2,
    Ms500 = 3,
    Ms1000 = 4,
    Ms1500 = 5,
    Ms2000 = 6,
    Ms4000 = 7,
    Ms8000 = 8,
    Ms16000 = 9,
    Ms32000 = 10,
}

impl ClusterBeaconPeriod {
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
            0 => Some(Self::Ms10),
            1 => Some(Self::Ms50),
            2 => Some(Self::Ms100),
            3 => Some(Self::Ms500),
            4 => Some(Self::Ms1000),
            5 => Some(Self::Ms1500),
            6 => Some(Self::Ms2000),
            7 => Some(Self::Ms4000),
            8 => Some(Self::Ms8000),
            9 => Some(Self::Ms16000),
            10 => Some(Self::Ms32000),
            _ => None,
        }
    }
    /// Period in milliseconds.
    #[must_use]
    pub const fn milliseconds(self) -> u32 {
        match self {
            Self::Ms10 => 10,
            Self::Ms50 => 50,
            Self::Ms100 => 100,
            Self::Ms500 => 500,
            Self::Ms1000 => 1000,
            Self::Ms1500 => 1500,
            Self::Ms2000 => 2000,
            Self::Ms4000 => 4000,
            Self::Ms8000 => 8000,
            Self::Ms16000 => 16000,
            Self::Ms32000 => 32000,
        }
    }
}

// =========================================================================
// CountToTrigger
// =========================================================================

/// 4-bit Count-to-Trigger field (Cluster Beacon).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CountToTrigger(u8);

impl CountToTrigger {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        if value & !0x0F != 0 {
            return None;
        }
        Some(Self(value))
    }
    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<CountToTrigger> for u8 {
    fn from(value: CountToTrigger) -> Self {
        value.0
    }
}

// =========================================================================
// Quality
// =========================================================================

/// 2-bit quality field (used for both rel_quality and min_quality in
/// the Cluster Beacon body).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Quality(u8);

impl Quality {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        if value & !0x03 != 0 {
            return None;
        }
        Some(Self(value))
    }
    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<Quality> for u8 {
    fn from(value: Quality) -> Self {
        value.0
    }
}
