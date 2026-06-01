//! Measurement, status, and indication field types.

// =========================================================================
// IndicationType
// =========================================================================

/// 3-bit Indication Type field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum IndicationType {
    Paging = 0,
    RandomAccessResponse = 1,
}

impl IndicationType {
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
            0 => Some(Self::Paging),
            1 => Some(Self::RandomAccessResponse),
            _ => None,
        }
    }
}

// =========================================================================
// BroadcastFeedbackType
// =========================================================================

/// 2-bit Feedback type field of a Broadcast Indication IE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum BroadcastFeedbackType {
    NoFeedback = 0b00,
    Mcs = 0b01,
    Mimo2Antenna = 0b10,
    Mimo4Antenna = 0b11,
}

impl BroadcastFeedbackType {
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
            0b00 => Some(Self::NoFeedback),
            0b01 => Some(Self::Mcs),
            0b10 => Some(Self::Mimo2Antenna),
            0b11 => Some(Self::Mimo4Antenna),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Load percentage (Load Info IE)
// ---------------------------------------------------------------------------

// =========================================================================
// LoadPercentage
// =========================================================================

/// 8-bit percentage value (0..=255 maps to 0..=100% linearly; 0x00 = 0%,
/// 0xFF = 100%).
///
/// The inner `u8` is the on-wire byte value, already converted from the
/// spec's MSB-to-LSB column layout (column 0 = MSB) to a Rust native
/// `u8` by [`Message::parse`](crate::mac::pdu::Message::parse). Callers
/// that read `pct.0` see the same numeric value the spec table names,
/// not raw bit positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LoadPercentage(pub u8);

// ---------------------------------------------------------------------------
// Radio Device Status IE enums (§6.4.3.13)
// ---------------------------------------------------------------------------

// =========================================================================
// RadioDeviceStatusFlag
// =========================================================================

/// 2-bit Status flag. Reserved values 0b00 and 0b11 are not
/// representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum RadioDeviceStatusFlag {
    MemoryFull = 0b01,
    NormalOperation = 0b10,
}

impl RadioDeviceStatusFlag {
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
            0b01 => Some(Self::MemoryFull),
            0b10 => Some(Self::NormalOperation),
            _ => None,
        }
    }
}

// =========================================================================
// RadioDeviceStatusDuration
// =========================================================================

/// 4-bit Duration field of a Radio Device Status IE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum RadioDeviceStatusDuration {
    Ms50 = 0,
    Ms100 = 1,
    Ms200 = 2,
    Ms400 = 3,
    Ms600 = 4,
    Ms800 = 5,
    Ms1000 = 6,
    Ms1500 = 7,
    Ms2000 = 8,
    Ms3000 = 9,
    Ms4000 = 10,
    Unknown = 11,
}

impl RadioDeviceStatusDuration {
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
            2 => Some(Self::Ms200),
            3 => Some(Self::Ms400),
            4 => Some(Self::Ms600),
            5 => Some(Self::Ms800),
            6 => Some(Self::Ms1000),
            7 => Some(Self::Ms1500),
            8 => Some(Self::Ms2000),
            9 => Some(Self::Ms3000),
            10 => Some(Self::Ms4000),
            11 => Some(Self::Unknown),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Hop limit / Hop count (Source Routing IE)
// ---------------------------------------------------------------------------

// =========================================================================
// DlDataReception
// =========================================================================

/// 3-bit DLdataReception field. Values 6 and 7 are reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum DlDataReception {
    Ms0 = 0,
    Ms5 = 1,
    Ms10 = 2,
    Ms20 = 3,
    Ms40 = 4,
    Ms80 = 5,
}

impl DlDataReception {
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
            0 => Some(Self::Ms0),
            1 => Some(Self::Ms5),
            2 => Some(Self::Ms10),
            3 => Some(Self::Ms20),
            4 => Some(Self::Ms40),
            5 => Some(Self::Ms80),
            _ => None,
        }
    }
}

// =========================================================================
// UlPeriod
// =========================================================================

/// 4-bit ULPeriod field. 12 valid codes; 12..=15 reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum UlPeriod {
    S10 = 0,
    S20 = 1,
    S48 = 2,
    S90 = 3,
    Min5 = 4,
    Min10 = 5,
    Min30 = 6,
    H1 = 7,
    H6 = 8,
    H12 = 9,
    H24 = 10,
    H48 = 11,
}

impl UlPeriod {
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
            0 => Some(Self::S10),
            1 => Some(Self::S20),
            2 => Some(Self::S48),
            3 => Some(Self::S90),
            4 => Some(Self::Min5),
            5 => Some(Self::Min10),
            6 => Some(Self::Min30),
            7 => Some(Self::H1),
            8 => Some(Self::H6),
            9 => Some(Self::H12),
            10 => Some(Self::H24),
            11 => Some(Self::H48),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// EndpointProtocol - 16-bit Joining Information EP value
// ---------------------------------------------------------------------------

// =========================================================================
// EndpointProtocol
// =========================================================================

/// 16-bit Protocol Endpoint value (Joining Information IE).
///
/// The inner `u16` holds the two on-wire bytes as a big-endian numeric
/// value (column 0 of the spec figure = MSB = bit 15 of this `u16`).
/// Callers that read `ep.0` see the same value the spec table names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct EndpointProtocol(pub u16);

// ---------------------------------------------------------------------------
// RSSI / SNR measurement codes (Neighbouring IE, Measurement Report IE)
// ETSI TS 103 636-4 references ETSI TS 103 636-2 for value semantics.
// ---------------------------------------------------------------------------

// =========================================================================
// Rssi1Measurement
// =========================================================================

/// 8-bit RSSI-1 measurement code (value coding defined in ETSI
/// TS 103 636-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rssi1Measurement(pub u8);

// =========================================================================
// Rssi2Measurement
// =========================================================================

/// 8-bit RSSI-2 measurement code (value coding defined in ETSI
/// TS 103 636-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rssi2Measurement(pub u8);

// =========================================================================
// SnrMeasurement
// =========================================================================

/// 8-bit SNR measurement code (value coding defined in ETSI
/// TS 103 636-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SnrMeasurement(pub u8);

// ---------------------------------------------------------------------------
// RD Capability IE enums
// ETSI TS 103 636-4, clause 6.4.3.5, Table 6.4.3.5-1
// ---------------------------------------------------------------------------
