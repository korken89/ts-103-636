//! Association / reconfiguration / HARQ / capability field types.

// =========================================================================
// SetupCause
// =========================================================================

/// 3-bit Association Setup Cause (Table 6.4.2.4-2). The reserved value
/// `0b111` is not representable; constructing a [`SetupCause`] from `0b111`
/// returns `None` and parsing it returns [`crate::ParsingError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum SetupCause {
    InitialAssociation = 0b000,
    NewFlowSet = 0b001,
    Mobility = 0b010,
    ReassociationAfterError = 0b011,
    /// Change of own operating channel. Valid only when the RD operates
    /// also in FT mode.
    ChannelChange = 0b100,
    /// Originally associated in PT mode, switched to/from FT mode.
    ModeChange = 0b101,
    PagingResponse = 0b110,
}

impl SetupCause {
    /// Raw 3-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from a raw 3-bit field. Returns `None` for the reserved value
    /// `0b111` (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b000 => Some(Self::InitialAssociation),
            0b001 => Some(Self::NewFlowSet),
            0b010 => Some(Self::Mobility),
            0b011 => Some(Self::ReassociationAfterError),
            0b100 => Some(Self::ChannelChange),
            0b101 => Some(Self::ModeChange),
            0b110 => Some(Self::PagingResponse),
            _ => None,
        }
    }
}

impl From<SetupCause> for u8 {
    fn from(value: SetupCause) -> Self {
        value.as_u8()
    }
}

// =========================================================================
// HarqProcesses
// =========================================================================

/// 3-bit HARQ process count field. Valid values 0..=7. The spec does not
/// restrict the encoding further in the Association Request / Response
/// context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HarqProcesses(u8);

impl HarqProcesses {
    /// Construct from a 3-bit value. Returns `None` if `value > 7`.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        if value & !0x07 != 0 {
            return None;
        }
        Some(HarqProcesses(value))
    }

    /// Underlying value (0..=7).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<HarqProcesses> for u8 {
    fn from(value: HarqProcesses) -> Self {
        value.0
    }
}

// =========================================================================
// RejectCause
// =========================================================================

/// 4-bit Association Reject Cause (Table 6.4.2.5-2). Reserved values
/// 5..=15 are not representable; the constructor returns `None` and the
/// parser returns [`crate::ParsingError`] for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum RejectCause {
    NoSufficientRadioCapacity = 0b0000,
    NoSufficientHwCapacity = 0b0001,
    ShortRdIdConflict = 0b0010,
    NonSecuredNotAccepted = 0b0011,
    OtherReason = 0b0100,
}

impl RejectCause {
    /// Raw 4-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from a raw 4-bit field. Returns `None` for reserved values
    /// 5..=15 (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b0000 => Some(Self::NoSufficientRadioCapacity),
            0b0001 => Some(Self::NoSufficientHwCapacity),
            0b0010 => Some(Self::ShortRdIdConflict),
            0b0011 => Some(Self::NonSecuredNotAccepted),
            0b0100 => Some(Self::OtherReason),
            _ => None,
        }
    }
}

impl From<RejectCause> for u8 {
    fn from(value: RejectCause) -> Self {
        value.as_u8()
    }
}

// =========================================================================
// RejectTimer
// =========================================================================

/// 4-bit Reject Timer (Table 6.4.2.5-2). The variant names encode the
/// timer in seconds. Reserved values 9..=15 are not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum RejectTimer {
    S0 = 0b0000,
    S5 = 0b0001,
    S10 = 0b0010,
    S30 = 0b0011,
    S60 = 0b0100,
    S120 = 0b0101,
    S180 = 0b0110,
    S300 = 0b0111,
    S600 = 0b1000,
}

impl RejectTimer {
    /// Raw 4-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from a raw 4-bit field. Returns `None` for reserved values
    /// 9..=15 (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b0000 => Some(Self::S0),
            0b0001 => Some(Self::S5),
            0b0010 => Some(Self::S10),
            0b0011 => Some(Self::S30),
            0b0100 => Some(Self::S60),
            0b0101 => Some(Self::S120),
            0b0110 => Some(Self::S180),
            0b0111 => Some(Self::S300),
            0b1000 => Some(Self::S600),
            _ => None,
        }
    }

    /// Timer duration in seconds.
    #[must_use]
    pub const fn seconds(self) -> u16 {
        match self {
            Self::S0 => 0,
            Self::S5 => 5,
            Self::S10 => 10,
            Self::S30 => 30,
            Self::S60 => 60,
            Self::S120 => 120,
            Self::S180 => 180,
            Self::S300 => 300,
            Self::S600 => 600,
        }
    }
}

impl From<RejectTimer> for u8 {
    fn from(value: RejectTimer) -> Self {
        value.as_u8()
    }
}

// =========================================================================
// ReleaseCause
// =========================================================================

/// 4-bit Association Release Cause (Table 6.4.2.6-1). Reserved values
/// `0b1011`, `0b1110`, and `0b1111` are not representable; the
/// constructor returns `None` and the parser returns
/// [`crate::ParsingError`] for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum ReleaseCause {
    ConnectionTermination = 0b0000,
    Mobility = 0b0001,
    LongInactivity = 0b0010,
    IncompatibleConfiguration = 0b0011,
    NoSufficientHwOrMemoryResource = 0b0100,
    NoSufficientRadioResources = 0b0101,
    BadRadioQuality = 0b0110,
    SecurityError = 0b0111,
    ShortRdIdConflictPtSide = 0b1000,
    ShortRdIdConflictFtSide = 0b1001,
    NotAssociated = 0b1010,
    NotOperatingInFtMode = 0b1100,
    OtherError = 0b1101,
}

impl ReleaseCause {
    /// Raw 4-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from a raw 4-bit field. Returns `None` for reserved values
    /// `0b1011`, `0b1110`, `0b1111` (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b0000 => Some(Self::ConnectionTermination),
            0b0001 => Some(Self::Mobility),
            0b0010 => Some(Self::LongInactivity),
            0b0011 => Some(Self::IncompatibleConfiguration),
            0b0100 => Some(Self::NoSufficientHwOrMemoryResource),
            0b0101 => Some(Self::NoSufficientRadioResources),
            0b0110 => Some(Self::BadRadioQuality),
            0b0111 => Some(Self::SecurityError),
            0b1000 => Some(Self::ShortRdIdConflictPtSide),
            0b1001 => Some(Self::ShortRdIdConflictFtSide),
            0b1010 => Some(Self::NotAssociated),
            0b1100 => Some(Self::NotOperatingInFtMode),
            0b1101 => Some(Self::OtherError),
            _ => None,
        }
    }
}

impl From<ReleaseCause> for u8 {
    fn from(value: ReleaseCause) -> Self {
        value.as_u8()
    }
}

// =========================================================================
// MaxHarqReTx
// =========================================================================

/// 5-bit MAX HARQ Re-TX / Re-RX field. Valid values 0..=30; the highest
/// 5-bit value `0b11111` is reserved by the spec and rejected by the
/// constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MaxHarqReTx(u8);

impl MaxHarqReTx {
    /// Largest non-reserved value (`30`).
    pub const MAX: Self = MaxHarqReTx(30);

    /// Construct from a 5-bit value. Returns `None` if the value does not
    /// fit in 5 bits or if it is the reserved `0b11111` value.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        if value & !0x1F != 0 || value == 0b11111 {
            return None;
        }
        Some(MaxHarqReTx(value))
    }

    /// Underlying value (0..=30).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<MaxHarqReTx> for u8 {
    fn from(value: MaxHarqReTx) -> Self {
        value.0
    }
}

// =========================================================================
// Release
// =========================================================================

/// 5-bit Release field of an RD Capability IE. Variant numeric values
/// match the on-wire encoding. Reserved values 0 and 5..=31 are not
/// representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum Release {
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
}

impl Release {
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
            1 => Some(Self::R1),
            2 => Some(Self::R2),
            3 => Some(Self::R3),
            4 => Some(Self::R4),
            _ => None,
        }
    }
}

// =========================================================================
// OperatingModes
// =========================================================================

/// 2-bit Operating modes field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum OperatingModes {
    PtOnly = 0b00,
    FtOnly = 0b01,
    Both = 0b10,
}

impl OperatingModes {
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
            0b00 => Some(Self::PtOnly),
            0b01 => Some(Self::FtOnly),
            0b10 => Some(Self::Both),
            _ => None,
        }
    }
}

// =========================================================================
// MacSecuritySupport
// =========================================================================

/// 3-bit MAC security support field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum MacSecuritySupport {
    NotSupported = 0b000,
    Mode1Supported = 0b001,
}

impl MacSecuritySupport {
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
            0b000 => Some(Self::NotSupported),
            0b001 => Some(Self::Mode1Supported),
            _ => None,
        }
    }
}

// =========================================================================
// DlcServiceType
// =========================================================================

/// 3-bit DLC service type capability field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum DlcServiceType {
    Type0 = 0b000,
    Type1 = 0b001,
    Type2 = 0b010,
    /// DLC Service type 3, type 2, and type 1 are supported.
    Type3and2and1 = 0b011,
    /// DLC Service types 0, 1, 2, 3 are supported.
    Type0123 = 0b100,
}

impl DlcServiceType {
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
            0b000 => Some(Self::Type0),
            0b001 => Some(Self::Type1),
            0b010 => Some(Self::Type2),
            0b011 => Some(Self::Type3and2and1),
            0b100 => Some(Self::Type0123),
            _ => None,
        }
    }
}

// =========================================================================
// HarqFeedbackDelay
// =========================================================================

/// 4-bit HARQ feedback delay field, in subslots. Valid 0..=6; values
/// 7..=15 are reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HarqFeedbackDelay(u8);

impl HarqFeedbackDelay {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(subslots: u8) -> Option<Self> {
        if subslots > 6 {
            return None;
        }
        Some(HarqFeedbackDelay(subslots))
    }

    /// Number of subslots.
    #[must_use]
    pub const fn subslots(self) -> u8 {
        self.0
    }
}

impl From<HarqFeedbackDelay> for u8 {
    fn from(value: HarqFeedbackDelay) -> Self {
        value.0
    }
}
