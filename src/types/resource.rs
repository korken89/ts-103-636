//! Resource Allocation and RACH field types.

use core::num::NonZero;

// =========================================================================
// Repetition
// =========================================================================

/// 8-bit Repetition count. Value `0` is "not defined" by the spec and
/// is rejected by the constructor; `1` means the next frame/subslot.
/// Internal `NonZero<u8>` representation eliminates the runtime check
/// in parsers (the type invariant covers it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Repetition(NonZero<u8>);

impl Repetition {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        match NonZero::new(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }
    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0.get()
    }
}

impl From<Repetition> for u8 {
    fn from(value: Repetition) -> Self {
        value.0.get()
    }
}

// =========================================================================
// Validity
// =========================================================================

/// 8-bit Validity (frames). `0xFF` means permanent. Full range valid.
///
/// The inner `u8` is the on-wire byte value, already converted from the
/// spec's MSB-to-LSB column layout (column 0 = MSB) to a Rust native
/// `u8` by [`Message::parse`](crate::mac::pdu::Message::parse). Callers
/// that read `validity.0` see the same numeric value the spec table
/// names, not raw bit positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Validity(pub u8);

// =========================================================================
// AllocationType
// =========================================================================

/// 2-bit Allocation Type field of a Resource Allocation IE. All four
/// encodings are defined; no reserved value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum AllocationType {
    /// Release all scheduled resources.
    ReleaseAll = 0b00,
    Downlink = 0b01,
    Uplink = 0b10,
    /// Both downlink and uplink (first allocation pair = DL, second = UL).
    DownlinkAndUplink = 0b11,
}

impl AllocationType {
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
            0b00 => Some(Self::ReleaseAll),
            0b01 => Some(Self::Downlink),
            0b10 => Some(Self::Uplink),
            0b11 => Some(Self::DownlinkAndUplink),
            _ => None,
        }
    }
}

// =========================================================================
// RepeatMode
// =========================================================================

/// 3-bit Repeat field of a Resource Allocation IE. The encoding `0b000`
/// ("Single, no repeat") is represented by the absence of a
/// [`RepeatMode`] in the parent structure; this enum only carries the
/// repeating variants. Reserved values 0b101..=0b111 are not
/// representable. ETSI TS 103 636-4, clause 6.4.3.3, Table 6.4.3.3-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum RepeatMode {
    PerFrame = 0b001,
    PerSubslot = 0b010,
    PerFrameWithGroup = 0b011,
    PerSubslotWithGroup = 0b100,
}

impl RepeatMode {
    /// Raw u8 value carrying the spec encoding.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from the raw 3-bit field. Returns `None` for `0b000`
    /// (Single, no repeat: the caller should represent this by `None`
    /// at the policy level) and for reserved values `0b101..=0b111`.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b001 => Some(Self::PerFrame),
            0b010 => Some(Self::PerSubslot),
            0b011 => Some(Self::PerFrameWithGroup),
            0b100 => Some(Self::PerSubslotWithGroup),
            _ => None,
        }
    }
}

// =========================================================================
// DectScheduledResourceFailure
// =========================================================================

/// 4-bit `dectScheduledResourceFailure` timer code (Table 6.4.3.3-2).
/// Variant names encode the timer in milliseconds. Reserved values
/// `0b0000`, `0b0001`, and `0b1100..=0b1111` are not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum DectScheduledResourceFailure {
    Ms20 = 0b0010,
    Ms50 = 0b0011,
    Ms100 = 0b0100,
    Ms200 = 0b0101,
    Ms500 = 0b0110,
    Ms1000 = 0b0111,
    Ms1500 = 0b1000,
    Ms3000 = 0b1001,
    Ms4000 = 0b1010,
    Ms5000 = 0b1011,
}

impl DectScheduledResourceFailure {
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
            0b0010 => Some(Self::Ms20),
            0b0011 => Some(Self::Ms50),
            0b0100 => Some(Self::Ms100),
            0b0101 => Some(Self::Ms200),
            0b0110 => Some(Self::Ms500),
            0b0111 => Some(Self::Ms1000),
            0b1000 => Some(Self::Ms1500),
            0b1001 => Some(Self::Ms3000),
            0b1010 => Some(Self::Ms4000),
            0b1011 => Some(Self::Ms5000),
            _ => None,
        }
    }

    /// Timer value in milliseconds.
    #[must_use]
    pub const fn milliseconds(self) -> u16 {
        match self {
            Self::Ms20 => 20,
            Self::Ms50 => 50,
            Self::Ms100 => 100,
            Self::Ms200 => 200,
            Self::Ms500 => 500,
            Self::Ms1000 => 1000,
            Self::Ms1500 => 1500,
            Self::Ms3000 => 3000,
            Self::Ms4000 => 4000,
            Self::Ms5000 => 5000,
        }
    }
}

// =========================================================================
// RaLength
// =========================================================================

/// 7-bit Length field of a Resource Allocation IE / Random Access
/// Resource IE. Unit (subslot vs slot) is indicated by the accompanying
/// [`crate::types::PacketLengthType`]. Valid values 0..=127.
/// ETSI TS 103 636-4, clause 6.4.3.3 (and 6.4.3.4 by reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RaLength(u8);

impl RaLength {
    /// Construct from a 7-bit value. Returns `None` if `value > 127`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x7F != 0 {
            return None;
        }
        Some(RaLength(value))
    }

    /// Underlying value (0..=127).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<RaLength> for u8 {
    fn from(value: RaLength) -> Self {
        value.0
    }
}

// =========================================================================
// RachRepeatMode
// =========================================================================

/// 2-bit Repeat field of a Random Access Resource IE. The encoding
/// `0b00` ("Single, no repeat") is represented by the absence of a
/// repeat policy in the parent structure; this enum only carries the
/// repeating variants. The reserved value `0b11` is not representable.
/// ETSI TS 103 636-4, clause 6.4.3.4, Table 6.4.3.4-1.
///
/// Note: this is a *different* field from the 3-bit
/// [`RepeatMode`] used by the Resource Allocation IE (§6.4.3.3) -
/// same name in the spec, narrower encoding and smaller value set
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum RachRepeatMode {
    PerFrame = 0b01,
    PerSubslot = 0b10,
}

impl RachRepeatMode {
    /// Raw u8 value carrying the spec encoding.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from the raw 2-bit field. Returns `None` for `0b00` (Single)
    /// and the reserved `0b11`.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b01 => Some(Self::PerFrame),
            0b10 => Some(Self::PerSubslot),
            _ => None,
        }
    }
}

// =========================================================================
// Cwsig
// =========================================================================

/// 3-bit contention-window scaling field used by both `Cwmin_sig` and
/// `Cwmax_sig`. Valid range 0..=7. The actual CW values are
/// `CW_MIN = 8 x value` and `CW_MAX = 256 x value`.
/// ETSI TS 103 636-4, clause 6.4.3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cwsig(u8);

impl Cwsig {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x07 != 0 {
            return None;
        }
        Some(Cwsig(value))
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<Cwsig> for u8 {
    fn from(value: Cwsig) -> Self {
        value.0
    }
}

// =========================================================================
// ResponseWindow
// =========================================================================

/// 4-bit response window field. The actual window length is
/// `value + 1` subslots, so [`Self::subslots`] returns 1..=16.
/// ETSI TS 103 636-4, clause 6.4.3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ResponseWindow(u8);

impl ResponseWindow {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(raw: u8) -> Option<Self> {
        if raw & !0x0F != 0 {
            return None;
        }
        Some(ResponseWindow(raw))
    }

    /// Raw 4-bit field value (0..=15).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Actual response window length in subslots (raw + 1, so 1..=16).
    #[must_use]
    pub const fn subslots(self) -> u8 {
        self.0 + 1
    }
}

impl From<ResponseWindow> for u8 {
    fn from(value: ResponseWindow) -> Self {
        value.0
    }
}

// =========================================================================
// MaxRachLength
// =========================================================================

/// 4-bit MAX RACH Length field. Raw 0..=15 (the spec does not specify
/// value+1 semantics). ETSI TS 103 636-4, clause 6.4.3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MaxRachLength(u8);

impl MaxRachLength {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x0F != 0 {
            return None;
        }
        Some(MaxRachLength(value))
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<MaxRachLength> for u8 {
    fn from(value: MaxRachLength) -> Self {
        value.0
    }
}
