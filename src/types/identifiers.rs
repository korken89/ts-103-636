//! Network / Radio Device identifier newtypes.

use core::fmt;
use core::num::NonZero;

// =========================================================================
// NetworkId24
// =========================================================================

/// Most significant 24 bits of a Network ID. Transmitted in Beacon
/// headers; used for scrambling non-beacon packets. Invariant: non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId24(NonZero<u32>);

impl NetworkId24 {
    /// Create from a 24-bit value. Returns `None` if zero or if any of the
    /// upper 8 bits are set.
    #[must_use]
    #[inline]
    pub const fn new(value: u32) -> Option<Self> {
        if value & 0xFF_FF_FF == 0 {
            return None;
        }
        if value & !0xFF_FF_FF != 0 {
            return None;
        }
        match NonZero::new(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }
}

impl From<NetworkId24> for u32 {
    fn from(value: NetworkId24) -> Self {
        value.0.get()
    }
}

impl fmt::Display for NetworkId24 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:06x}", self.0.get())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for NetworkId24 {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "0x{:06x}", self.0.get());
    }
}

// =========================================================================
// NetworkId8
// =========================================================================

/// Least significant 8 bits of a Network ID. Transmitted in the PHY
/// control field. Invariant: non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId8(NonZero<u8>);

impl NetworkId8 {
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

impl From<NetworkId8> for u8 {
    fn from(value: NetworkId8) -> Self {
        value.0.get()
    }
}

impl fmt::Display for NetworkId8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.0.get())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for NetworkId8 {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "0x{:02x}", self.0.get());
    }
}

// =========================================================================
// NetworkId32
// =========================================================================

/// Full 32-bit Network ID. Composed of [`NetworkId24`] (MSB) and
/// [`NetworkId8`] (LSB). Invariant: neither part is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId32(NonZero<u32>);

impl NetworkId32 {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u32) -> Option<Self> {
        if (value >> 8) == 0 || (value & 0xFF) == 0 {
            return None;
        }
        match NonZero::new(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }

    /// Most-significant 24 bits (Network ID MSB part).
    #[must_use]
    pub const fn msb(self) -> NetworkId24 {
        NetworkId24::new(self.as_u32() >> 8).expect("NetworkId32 invariant")
    }

    /// Least-significant 8 bits (Network ID LSB part).
    #[must_use]
    pub const fn lsb(self) -> NetworkId8 {
        NetworkId8::new((self.as_u32() & 0xFF) as u8).expect("NetworkId32 invariant")
    }
}

impl From<NetworkId32> for u32 {
    fn from(value: NetworkId32) -> Self {
        value.0.get()
    }
}

impl fmt::Display for NetworkId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08x}", self.0.get())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for NetworkId32 {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "0x{:08x}", self.0.get());
    }
}

// =========================================================================
// ShortRdId
// =========================================================================

/// 16-bit Short RD ID. Invariant: non-zero (0x0000 is reserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortRdId(NonZero<u16>);

impl ShortRdId {
    /// Broadcast address 0xFFFF.
    pub const BROADCAST: Self = ShortRdId(NonZero::new(0xFFFF).unwrap());

    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u16) -> Option<Self> {
        match NonZero::new(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0.get()
    }

    /// `true` if this is the broadcast Short RD ID (`0xFFFF`).
    #[must_use]
    pub const fn is_broadcast(self) -> bool {
        self.0.get() == 0xFFFF
    }
}

impl From<ShortRdId> for u16 {
    fn from(value: ShortRdId) -> Self {
        value.0.get()
    }
}

impl fmt::Display for ShortRdId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_broadcast() {
            write!(f, "BROADCAST")
        } else {
            write!(f, "0x{:04x}", self.0.get())
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for ShortRdId {
    fn format(&self, f: defmt::Formatter<'_>) {
        if self.is_broadcast() {
            defmt::write!(f, "BROADCAST");
        } else {
            defmt::write!(f, "0x{:04x}", self.0.get());
        }
    }
}

// =========================================================================
// LongRdId
// =========================================================================

/// 32-bit Long RD ID. Invariant: non-zero (0x0000_0000 is reserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LongRdId(NonZero<u32>);

impl LongRdId {
    /// Backend address 0xFFFF_FFFE.
    pub const BACKEND: Self = LongRdId(NonZero::new(0xFFFF_FFFE).unwrap());
    /// Broadcast address 0xFFFF_FFFF.
    pub const BROADCAST: Self = LongRdId(NonZero::new(0xFFFF_FFFF).unwrap());

    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn new(value: u32) -> Option<Self> {
        match NonZero::new(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }

    /// `true` if this is the reserved backend Long RD ID (`0xFFFF_FFFE`).
    #[must_use]
    pub const fn is_backend(self) -> bool {
        self.0.get() == 0xFFFF_FFFE
    }

    /// `true` if this is the broadcast Long RD ID (`0xFFFF_FFFF`).
    #[must_use]
    pub const fn is_broadcast(self) -> bool {
        self.0.get() == 0xFFFF_FFFF
    }
}

impl From<LongRdId> for u32 {
    fn from(value: LongRdId) -> Self {
        value.0.get()
    }
}

impl fmt::Display for LongRdId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_broadcast() {
            write!(f, "BROADCAST")
        } else if self.is_backend() {
            write!(f, "BACKEND")
        } else {
            write!(f, "0x{:08x}", self.0.get())
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for LongRdId {
    #[expect(
        clippy::if_same_then_else,
        reason = "defmt::write! moves the literal into linker data, so the blocks only look identical"
    )]
    fn format(&self, f: defmt::Formatter<'_>) {
        if self.is_broadcast() {
            defmt::write!(f, "BROADCAST");
        } else if self.is_backend() {
            defmt::write!(f, "BACKEND");
        } else {
            defmt::write!(f, "0x{:08x}", self.0.get());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_id_32_rejects_partial_zero() {
        // MSB zero (high 24 bits) - reject
        assert!(NetworkId32::new(0x0000_00FF).is_none());
        // LSB zero (low 8 bits) - reject
        assert!(NetworkId32::new(0xFF_FF_FF_00).is_none());
        // Both nonzero - accept
        assert!(NetworkId32::new(0x12_34_56_78).is_some());
    }
}
