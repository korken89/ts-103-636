//! IE type registries as typed enums with spec discriminants.
//!
//! ETSI TS 103 636-4, clause 6.3.4: Table 6.3.4-2 (6-bit types),
//! Table 6.3.4-3 (5-bit types, payload length 0), and Table 6.3.4-4
//! (5-bit types, payload length 1). Reserved / unrecognized code
//! points are not representable here; the IE parser surfaces them as
//! `AnyIeType::Unknown*` so unknown IEs can still be skipped by
//! length.

// =========================================================================
// IEType6bit
// =========================================================================

/// 6-bit IE type (used with MAC Extension encodings 00, 01, 10).
/// Table 6.3.4-2; discriminants are the on-wire code points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(
    missing_docs,
    reason = "variant names mirror the Table 6.3.4-2 entries"
)]
pub enum IEType6bit {
    Padding = 0b000000,
    HigherLayerSignallingFlow1 = 0b000001,
    HigherLayerSignallingFlow2 = 0b000010,
    UserPlaneDataFlow1 = 0b000011,
    UserPlaneDataFlow2 = 0b000100,
    UserPlaneDataFlow3 = 0b000101,
    UserPlaneDataFlow4 = 0b000110,
    NetworkBeacon = 0b001000,
    ClusterBeacon = 0b001001,
    AssociationRequest = 0b001010,
    AssociationResponse = 0b001011,
    AssociationRelease = 0b001100,
    ReconfigurationRequest = 0b001101,
    ReconfigurationResponse = 0b001110,
    AdditionalMacMessages = 0b001111,
    MacSecurityInfo = 0b010000,
    RouteInfo = 0b010001,
    ResourceAllocation = 0b010010,
    RandomAccessResource = 0b010011,
    RdCapability = 0b010100,
    Neighbouring = 0b010101,
    BroadcastIndication = 0b010110,
    GroupAssignment = 0b010111,
    LoadInfo = 0b011000,
    MeasurementReport = 0b011001,
    SourceRouting = 0b011010,
    JoiningBeacon = 0b011011,
    JoiningInformation = 0b011100,
    Escape = 0b111110,
    IeTypeExtension = 0b111111,
}

impl IEType6bit {
    /// Construct from a raw 6-bit code point. Returns `None` for
    /// reserved values and for bytes with the high 2 bits set.
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0b000000 => Self::Padding,
            0b000001 => Self::HigherLayerSignallingFlow1,
            0b000010 => Self::HigherLayerSignallingFlow2,
            0b000011 => Self::UserPlaneDataFlow1,
            0b000100 => Self::UserPlaneDataFlow2,
            0b000101 => Self::UserPlaneDataFlow3,
            0b000110 => Self::UserPlaneDataFlow4,
            0b001000 => Self::NetworkBeacon,
            0b001001 => Self::ClusterBeacon,
            0b001010 => Self::AssociationRequest,
            0b001011 => Self::AssociationResponse,
            0b001100 => Self::AssociationRelease,
            0b001101 => Self::ReconfigurationRequest,
            0b001110 => Self::ReconfigurationResponse,
            0b001111 => Self::AdditionalMacMessages,
            0b010000 => Self::MacSecurityInfo,
            0b010001 => Self::RouteInfo,
            0b010010 => Self::ResourceAllocation,
            0b010011 => Self::RandomAccessResource,
            0b010100 => Self::RdCapability,
            0b010101 => Self::Neighbouring,
            0b010110 => Self::BroadcastIndication,
            0b010111 => Self::GroupAssignment,
            0b011000 => Self::LoadInfo,
            0b011001 => Self::MeasurementReport,
            0b011010 => Self::SourceRouting,
            0b011011 => Self::JoiningBeacon,
            0b011100 => Self::JoiningInformation,
            0b111110 => Self::Escape,
            0b111111 => Self::IeTypeExtension,
            _ => return None,
        })
    }

    /// Raw 6-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl From<IEType6bit> for u8 {
    fn from(value: IEType6bit) -> Self {
        value as u8
    }
}

// =========================================================================
// IEType5bitLen0
// =========================================================================

/// 5-bit IE type with payload length 0 (Table 6.3.4-3). The length
/// bit of the Short IE head selects between two independent
/// registries; this is the length = 0 one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(
    missing_docs,
    reason = "variant names mirror the Table 6.3.4-3 entries"
)]
pub enum IEType5bitLen0 {
    Padding = 0b00000,
    ConfigurationRequest = 0b00001,
    KeepAlive = 0b00010,
    MacSecurityInfo = 0b10000,
    Escape = 0b11110,
}

impl IEType5bitLen0 {
    /// Construct from a raw 5-bit code point. Returns `None` for
    /// reserved values and for bytes with the high 3 bits set.
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0b00000 => Self::Padding,
            0b00001 => Self::ConfigurationRequest,
            0b00010 => Self::KeepAlive,
            0b10000 => Self::MacSecurityInfo,
            0b11110 => Self::Escape,
            _ => return None,
        })
    }

    /// Raw 5-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// =========================================================================
// IEType5bitLen1
// =========================================================================

/// 5-bit IE type with payload length 1 (Table 6.3.4-4). The length
/// bit of the Short IE head selects between two independent
/// registries; this is the length = 1 one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(
    missing_docs,
    reason = "variant names mirror the Table 6.3.4-4 entries"
)]
pub enum IEType5bitLen1 {
    Padding = 0b00000,
    RadioDeviceStatus = 0b00001,
    RdCapabilityShort = 0b00010,
    AssociationControl = 0b00011,
    Escape = 0b11110,
}

impl IEType5bitLen1 {
    /// Construct from a raw 5-bit code point. Returns `None` for
    /// reserved values and for bytes with the high 3 bits set.
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0b00000 => Self::Padding,
            0b00001 => Self::RadioDeviceStatus,
            0b00010 => Self::RdCapabilityShort,
            0b00011 => Self::AssociationControl,
            0b11110 => Self::Escape,
            _ => return None,
        })
    }

    /// Raw 5-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// =========================================================================
// ShortIeType
// =========================================================================

/// A Short IE type together with its registry-selecting length bit
/// (Tables 6.3.4-3 / 6.3.4-4). The composite low-6-bit encoding used
/// in the Short IE head is `length_bit << 5 | type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ShortIeType {
    /// Payload length 0 registry (Table 6.3.4-3).
    Len0(IEType5bitLen0),
    /// Payload length 1 registry (Table 6.3.4-4).
    Len1(IEType5bitLen1),
}

impl ShortIeType {
    /// Construct from the raw composite (length bit at bit 5, 5-bit
    /// type in the low bits). Returns `None` if bits 6..=7 are set or
    /// the code point is reserved in its registry.
    #[must_use]
    pub const fn try_from_composite(byte: u8) -> Option<Self> {
        if byte & !0x3F != 0 {
            return None;
        }
        if byte & 0b10_0000 == 0 {
            match IEType5bitLen0::try_from_u8(byte & 0x1F) {
                Some(t) => Some(Self::Len0(t)),
                None => None,
            }
        } else {
            match IEType5bitLen1::try_from_u8(byte & 0x1F) {
                Some(t) => Some(Self::Len1(t)),
                None => None,
            }
        }
    }

    /// Composite low-6-bit encoding (length bit at bit 5, type in the
    /// low 5 bits) as used in the Short IE head.
    #[must_use]
    #[inline]
    pub const fn composite(self) -> u8 {
        match self {
            Self::Len0(t) => t.as_u8(),
            Self::Len1(t) => 0b10_0000 | t.as_u8(),
        }
    }

    /// Payload length in bytes implied by the registry: 0 or 1.
    #[expect(
        clippy::len_without_is_empty,
        reason = "emptiness is not distinct for this length flag"
    )]
    #[must_use]
    #[inline]
    pub const fn len(self) -> u8 {
        match self {
            Self::Len0(_) => 0,
            Self::Len1(_) => 1,
        }
    }

    /// True if this is a padding IE (either registry).
    #[must_use]
    #[inline]
    pub const fn is_padding(self) -> bool {
        matches!(
            self,
            Self::Len0(IEType5bitLen0::Padding) | Self::Len1(IEType5bitLen1::Padding)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ie6bit_rejects_reserved_and_excessive() {
        assert!(IEType6bit::try_from_u8(0x40).is_none());
        assert!(IEType6bit::try_from_u8(0xFF).is_none());
        // 011101 to 111101 reserved.
        assert!(IEType6bit::try_from_u8(0b011101).is_none());
        assert!(IEType6bit::try_from_u8(0b111101).is_none());
    }

    #[test]
    fn ie6bit_escape_and_extension_codepoints() {
        // Table 6.3.4-2: 111110 Escape, 111111 IE type extension.
        assert_eq!(IEType6bit::Escape.as_u8(), 0x3E);
        assert_eq!(IEType6bit::IeTypeExtension.as_u8(), 0x3F);
        assert_eq!(IEType6bit::try_from_u8(0x3E), Some(IEType6bit::Escape));
        assert_eq!(
            IEType6bit::try_from_u8(0x3F),
            Some(IEType6bit::IeTypeExtension)
        );
    }

    #[test]
    fn short_ie_composite_round_trip() {
        let t = ShortIeType::try_from_composite(0b1_00010).unwrap();
        assert_eq!(t, ShortIeType::Len1(IEType5bitLen1::RdCapabilityShort));
        assert_eq!(t.composite(), 0b1_00010);
        assert_eq!(t.len(), 1);

        let t = ShortIeType::try_from_composite(0b0_10000).unwrap();
        assert_eq!(t, ShortIeType::Len0(IEType5bitLen0::MacSecurityInfo));
        assert_eq!(t.len(), 0);

        // Bits 6..=7 set, or reserved code points, are rejected.
        assert!(ShortIeType::try_from_composite(0xFF).is_none());
        assert!(ShortIeType::try_from_composite(0b0_00011).is_none());
        assert!(ShortIeType::try_from_composite(0b1_00100).is_none());
    }

    #[test]
    fn short_ie_padding_detection() {
        assert!(ShortIeType::Len0(IEType5bitLen0::Padding).is_padding());
        assert!(ShortIeType::Len1(IEType5bitLen1::Padding).is_padding());
        assert!(!ShortIeType::Len0(IEType5bitLen0::ConfigurationRequest).is_padding());
    }
}
