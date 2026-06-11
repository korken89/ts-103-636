//! Flow, route, group, and source-routing field types.

use super::ie_types::IEType6bit;
// =========================================================================
// RouteCost
// =========================================================================

/// 8-bit Route Cost carried in a Route Info IE. Smaller values mean lower
/// cost; the value increases linearly with distance and must be
/// incremented by at least 1 per hop. The full `u8` range 0..=255 is
/// valid on the wire, so the inner field is public.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RouteCost(pub u8);

// =========================================================================
// ApplicationSequenceNumber
// =========================================================================

/// 8-bit Application Sequence Number carried in a Route Info IE. The
/// sender increments this when the application data has changed; mesh
/// receivers use it to detect that they need to re-fetch from their
/// next hop. The full `u8` range 0..=255 is valid on the wire, so the
/// inner field is public. ETSI TS 103 636-4, clause 6.4.3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ApplicationSequenceNumber(pub u8);

// =========================================================================
// GroupId
// =========================================================================

/// 7-bit Group ID used to identify a pre-allocated group in the
/// Association Response and Group Assignment IE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GroupId(u8);

impl GroupId {
    /// Construct from a 7-bit value. Returns `None` if `value > 127`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x7F != 0 {
            return None;
        }
        Some(GroupId(value))
    }

    /// Underlying value (0..=127).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<GroupId> for u8 {
    fn from(value: GroupId) -> Self {
        value.0
    }
}

// =========================================================================
// ResourceTag
// =========================================================================

/// 7-bit Resource Tag indicating the index position of the resource tag
/// in the RD's resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ResourceTag(u8);

impl ResourceTag {
    /// Construct from a 7-bit value. Returns `None` if `value > 127`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x7F != 0 {
            return None;
        }
        Some(ResourceTag(value))
    }

    /// Underlying value (0..=127).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<ResourceTag> for u8 {
    fn from(value: ResourceTag) -> Self {
        value.0
    }
}

// =========================================================================
// Hop
// =========================================================================

/// 4-bit hop counter. Full 0..=15 range valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Hop(u8);

impl Hop {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x0F != 0 {
            return None;
        }
        Some(Hop(value))
    }
    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl From<Hop> for u8 {
    fn from(value: Hop) -> Self {
        value.0
    }
}

// =========================================================================
// SourceRoutingValidityTimer
// =========================================================================

/// 8-bit Source Routing registration validity timer. Valid codes 0..=19;
/// 20..=255 reserved. The variant names label the interval; use
/// [`Self::seconds`] for a programmatic duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum SourceRoutingValidityTimer {
    NotDefined = 0,
    S1 = 1,
    S2 = 2,
    S10 = 3,
    S30 = 4,
    Min1 = 5,
    Min2 = 6,
    Min5 = 7,
    Min10 = 8,
    Min30 = 9,
    H1 = 10,
    H2 = 11,
    H5 = 12,
    H10 = 13,
    H20 = 14,
    H50 = 15,
    H100 = 16,
    H200 = 17,
    H500 = 18,
    H1000 = 19,
}

impl SourceRoutingValidityTimer {
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
            0 => Some(Self::NotDefined),
            1 => Some(Self::S1),
            2 => Some(Self::S2),
            3 => Some(Self::S10),
            4 => Some(Self::S30),
            5 => Some(Self::Min1),
            6 => Some(Self::Min2),
            7 => Some(Self::Min5),
            8 => Some(Self::Min10),
            9 => Some(Self::Min30),
            10 => Some(Self::H1),
            11 => Some(Self::H2),
            12 => Some(Self::H5),
            13 => Some(Self::H10),
            14 => Some(Self::H20),
            15 => Some(Self::H50),
            16 => Some(Self::H100),
            17 => Some(Self::H200),
            18 => Some(Self::H500),
            19 => Some(Self::H1000),
            _ => None,
        }
    }
    /// Validity interval in seconds. `NotDefined` returns `None`.
    #[must_use]
    pub const fn seconds(self) -> Option<u32> {
        match self {
            Self::NotDefined => None,
            Self::S1 => Some(1),
            Self::S2 => Some(2),
            Self::S10 => Some(10),
            Self::S30 => Some(30),
            Self::Min1 => Some(60),
            Self::Min2 => Some(120),
            Self::Min5 => Some(300),
            Self::Min10 => Some(600),
            Self::Min30 => Some(1_800),
            Self::H1 => Some(3_600),
            Self::H2 => Some(7_200),
            Self::H5 => Some(18_000),
            Self::H10 => Some(36_000),
            Self::H20 => Some(72_000),
            Self::H50 => Some(180_000),
            Self::H100 => Some(360_000),
            Self::H200 => Some(720_000),
            Self::H500 => Some(1_800_000),
            Self::H1000 => Some(3_600_000),
        }
    }
}

// =========================================================================
// FlowAction
// =========================================================================

/// 1-bit Setup/Release indicator carried in a [`FlowEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum FlowAction {
    /// The flow indicated in the Flow ID is for setup or reconfiguration.
    SetupOrReconfigure = 0,
    /// The flow indicated in the Flow ID is released.
    Release = 1,
}

impl FlowAction {
    /// Raw 1-bit field (`const`).
    #[must_use]
    pub const fn as_bit(self) -> u8 {
        self as u8
    }

    /// Construct from the 1-bit raw field (`const`).
    #[must_use]
    pub const fn from_bit(bit: bool) -> Self {
        if bit {
            Self::Release
        } else {
            Self::SetupOrReconfigure
        }
    }
}

// =========================================================================
// FlowEntry
// =========================================================================

/// Packed Setup/Release + Flow ID byte used by Reconfiguration Request /
/// Response messages.
///
/// Byte layout (ETSI bit numbering, MSB first):
///   * ETSI bit 0 (mask 0x80): Setup/Release
///   * ETSI bit 1 (mask 0x40): Reserved (must be 0)
///   * ETSI bits 2..=7 (mask 0x3F): Flow ID
///
/// `#[repr(transparent)]` so a slice of bytes whose reserved bits have
/// been validated to be 0 can be reinterpreted as `&[FlowEntry]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct FlowEntry(u8);

impl FlowEntry {
    /// Build from a typed action + flow ID (`const`).
    #[must_use]
    #[inline]
    pub const fn new(action: FlowAction, flow_id: FlowId) -> Self {
        Self((action.as_bit() << 7) | (flow_id.as_u8() & 0x3F))
    }

    /// Construct from a raw on-wire byte. Returns `None` if the reserved
    /// bit (ETSI bit 1 / mask 0x40) is set (`const`).
    #[must_use]
    pub const fn try_from_raw(byte: u8) -> Option<Self> {
        if byte & 0x40 != 0 {
            return None;
        }
        Some(Self(byte))
    }

    /// Raw on-wire byte (`const`).
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self.0
    }

    /// Action (Setup/Release) decoded from the high bit (`const`).
    #[must_use]
    pub const fn action(self) -> FlowAction {
        FlowAction::from_bit(self.0 & 0x80 != 0)
    }

    /// Flow ID decoded from the low 6 bits (`const`).
    #[must_use]
    pub const fn flow_id(self) -> FlowId {
        // Invariant: low 6 bits always fit, by construction.
        match FlowId::new(self.0 & 0x3F) {
            Some(f) => f,
            None => unreachable!(),
        }
    }
}

// =========================================================================
// RadioResourceChange
// =========================================================================

/// 2-bit Radio Resource field carried in Reconfiguration Request /
/// Response. All four encodings are defined; no reserved value.
/// ETSI TS 103 636-4, clauses 6.4.2.7, 6.4.2.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum RadioResourceChange {
    NoChange = 0b00,
    RequestMore = 0b01,
    RequestLess = 0b10,
    /// A Resource Allocation IE follows this body in the same MAC PDU.
    ResourceAllocationIeIncluded = 0b11,
}

impl RadioResourceChange {
    /// Raw 2-bit field value (`const`).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse from the raw 2-bit field. All four values are defined, so
    /// this only returns `None` for inputs that exceed 2 bits (`const`).
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::NoChange),
            0b01 => Some(Self::RequestMore),
            0b10 => Some(Self::RequestLess),
            0b11 => Some(Self::ResourceAllocationIeIncluded),
            _ => None,
        }
    }
}

impl From<RadioResourceChange> for u8 {
    fn from(value: RadioResourceChange) -> Self {
        value.as_u8()
    }
}

// =========================================================================
// FlowId
// =========================================================================

/// 6-bit Flow ID. The spec defines values as entries in the
/// [IE type table](crate::types::IEType6bit), so the on-wire encoding is
/// the same as a 6-bit IE type. Values 0..=63 are accepted; semantically
/// only the user-plane and higher-layer-signalling IE type values are
/// meaningful. ETSI TS 103 636-4, clauses 6.4.2.4, 6.4.2.5.
///
/// `#[repr(transparent)]` so a slice of bytes that have all been
/// validated to fit in 6 bits can be reinterpreted as `&[FlowId]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct FlowId(u8);

impl FlowId {
    /// Construct from a 6-bit value. Returns `None` if `value > 63`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value & !0x3F != 0 {
            return None;
        }
        Some(FlowId(value))
    }

    /// Underlying value (0..=63).
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Convenience: build from an [`IEType6bit`].
    #[must_use]
    #[inline]
    pub const fn from_ie_type(ie: IEType6bit) -> Self {
        // IEType6bit discriminants are 6-bit code points by construction.
        FlowId(ie as u8)
    }
}

impl From<FlowId> for u8 {
    fn from(value: FlowId) -> Self {
        value.0
    }
}
