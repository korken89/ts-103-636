//! PHY numerology and field types.

use core::fmt;
use core::num::NonZero;

use crate::constants;
// =========================================================================
// Mu
// =========================================================================

/// Subcarrier scaling factor mu. Allowed values: 1, 2, 4, 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mu(u8);

#[expect(
    missing_docs,
    reason = "associated-constant names encode the value: M1 = mu=1, etc."
)]
impl Mu {
    pub const M1: Self = Mu(1);
    pub const M2: Self = Mu(2);
    pub const M4: Self = Mu(4);
    pub const M8: Self = Mu(8);

    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            1 | 2 | 4 | 8 => Some(Mu(value)),
            _ => None,
        }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Index into PHY numerology arrays (0 -> mu=1, 1 -> mu=2, 2 -> mu=4, 3 -> mu=8).
    #[must_use]
    pub const fn idx(self) -> usize {
        self.0.trailing_zeros() as usize
    }

    /// Number of OFDM symbols per slot for this mu.
    #[must_use]
    pub const fn n_sym_slot(self) -> u8 {
        constants::N_SYM_SLOT[self.idx()]
    }

    /// Number of subslots per slot for this mu.
    #[must_use]
    pub const fn n_subslot(self) -> u8 {
        constants::N_SUBSLOT[self.idx()]
    }

    /// Number of OFDM symbols in GI+STF combined for this mu.
    #[must_use]
    pub const fn n_gi_plus_stf_sym(self) -> u8 {
        constants::N_GI_PLUS_STF_SYM[self.idx()]
    }
}

impl From<Mu> for u8 {
    fn from(value: Mu) -> Self {
        value.0
    }
}

impl fmt::Display for Mu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mu={}", self.0)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Mu {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "mu={}", self.0);
    }
}

// =========================================================================
// Beta
// =========================================================================

/// Fourier transform scaling factor beta. Allowed values: 1, 2, 4, 8, 12, 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Beta(u8);

#[expect(
    missing_docs,
    reason = "associated-constant names encode the value: B1 = beta=1, etc."
)]
impl Beta {
    pub const B1: Self = Beta(1);
    pub const B2: Self = Beta(2);
    pub const B4: Self = Beta(4);
    pub const B8: Self = Beta(8);
    pub const B12: Self = Beta(12);
    pub const B16: Self = Beta(16);

    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            1 | 2 | 4 | 8 | 12 | 16 => Some(Beta(value)),
            _ => None,
        }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Index into PHY numerology arrays (0 -> beta=1, ..., 5 -> beta=16).
    #[must_use]
    pub const fn idx(self) -> usize {
        match self.0 {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            12 => 4,
            16 => 5,
            _ => panic!("Beta invariant guarantees a known value"),
        }
    }

    /// Number of occupied subcarriers for this beta.
    #[must_use]
    pub const fn n_occ(self) -> u16 {
        constants::N_OCC[self.idx()]
    }

    /// DFT size for this beta.
    #[must_use]
    pub const fn n_fft(self) -> u16 {
        constants::N_FFT[self.idx()]
    }

    /// Cyclic prefix length in samples for this beta.
    #[must_use]
    pub const fn n_cp(self) -> u16 {
        constants::N_CP[self.idx()]
    }
}

impl From<Beta> for u8 {
    fn from(value: Beta) -> Self {
        value.0
    }
}

impl fmt::Display for Beta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "beta={}", self.0)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Beta {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "beta={}", self.0);
    }
}

// =========================================================================
// Mcs
// =========================================================================

/// MCS index 0..=11. Note: PCC Type 1 (Table 6.2.1-1) has a 3-bit DF MCS
/// field, so only MCS 0..=7 fit; MCC 8..=11 require Type 2 (4-bit field,
/// Table 6.2.1-2 / 6.2.1-2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mcs(u8);

impl Mcs {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        if value <= 11 { Some(Mcs(value)) } else { None }
    }

    /// Raw value carried by this newtype.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Row of the MCS table for this index.
    #[must_use]
    pub const fn entry(self) -> &'static constants::McsEntry {
        &constants::MCS_TABLE[self.0 as usize]
    }

    /// Bits per modulation symbol.
    #[must_use]
    pub const fn n_bps(self) -> u8 {
        self.entry().n_bps
    }

    /// Code rate as (numerator, denominator).
    #[must_use]
    pub const fn code_rate(self) -> (u8, u8) {
        (self.entry().code_rate_num, self.entry().code_rate_den)
    }

    /// Modulation scheme.
    #[must_use]
    pub const fn modulation(self) -> constants::Modulation {
        self.entry().modulation
    }

    /// Human-readable description, e.g. "QPSK r=1/2". Always `Some` for
    /// valid indices.
    #[must_use]
    pub const fn description(self) -> Option<&'static str> {
        match self.0 {
            0 => Some("pi/2-BPSK r=1/2"),
            1 => Some("QPSK r=1/2"),
            2 => Some("QPSK r=3/4"),
            3 => Some("QAM16 r=1/2"),
            4 => Some("QAM16 r=3/4"),
            5 => Some("QAM64 r=2/3"),
            6 => Some("QAM64 r=3/4"),
            7 => Some("QAM64 r=5/6"),
            8 => Some("QAM256 r=3/4"),
            9 => Some("QAM256 r=5/6"),
            10 => Some("QAM1024 r=3/4"),
            11 => Some("QAM1024 r=5/6"),
            _ => None,
        }
    }
}

impl From<Mcs> for u8 {
    fn from(value: Mcs) -> Self {
        value.0
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Mcs {
    fn format(&self, f: defmt::Formatter<'_>) {
        if let Some(desc) = self.description() {
            defmt::write!(f, "MCS-{} ({})", self.0, desc);
        } else {
            defmt::write!(f, "MCS-{}", self.0);
        }
    }
}

// =========================================================================
// RdPowerClass
// =========================================================================

/// 3-bit RD Power Class field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum RdPowerClass {
    ClassI = 0b000,
    ClassII = 0b001,
    ClassIII = 0b010,
    ClassIV = 0b011,
}

impl RdPowerClass {
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
            0b000 => Some(Self::ClassI),
            0b001 => Some(Self::ClassII),
            0b010 => Some(Self::ClassIII),
            0b011 => Some(Self::ClassIV),
            _ => None,
        }
    }
}

// =========================================================================
// Nss
// =========================================================================

/// 2-bit "number of spatial streams" field (`Max NSS for RX` and
/// `RX for TX diversity` in RD Capability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum Nss {
    N1 = 0b00,
    N2 = 0b01,
    N4 = 0b10,
    N8 = 0b11,
}

impl Nss {
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
            0b00 => Some(Self::N1),
            0b01 => Some(Self::N2),
            0b10 => Some(Self::N4),
            0b11 => Some(Self::N8),
            _ => None,
        }
    }

    /// Numeric value (1, 2, 4, or 8).
    #[must_use]
    pub const fn streams(self) -> u8 {
        match self {
            Self::N1 => 1,
            Self::N2 => 2,
            Self::N4 => 4,
            Self::N8 => 8,
        }
    }
}

// =========================================================================
// RxGain
// =========================================================================

/// 4-bit RX Gain field. Variants encode the dB value as a signed integer
/// in the variant name (e.g. `DbNeg10` = -10 dB). Reserved values 9..=15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum RxGain {
    DbNeg10 = 0,
    DbNeg8 = 1,
    DbNeg6 = 2,
    DbNeg4 = 3,
    DbNeg2 = 4,
    Db0 = 5,
    Db2 = 6,
    Db4 = 7,
    Db6 = 8,
}

impl RxGain {
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
            0 => Some(Self::DbNeg10),
            1 => Some(Self::DbNeg8),
            2 => Some(Self::DbNeg6),
            3 => Some(Self::DbNeg4),
            4 => Some(Self::DbNeg2),
            5 => Some(Self::Db0),
            6 => Some(Self::Db2),
            7 => Some(Self::Db4),
            8 => Some(Self::Db6),
            _ => None,
        }
    }

    /// Gain in dB.
    #[must_use]
    pub const fn db(self) -> i8 {
        match self {
            Self::DbNeg10 => -10,
            Self::DbNeg8 => -8,
            Self::DbNeg6 => -6,
            Self::DbNeg4 => -4,
            Self::DbNeg2 => -2,
            Self::Db0 => 0,
            Self::Db2 => 2,
            Self::Db4 => 4,
            Self::Db6 => 6,
        }
    }
}

// =========================================================================
// SoftBufferSize
// =========================================================================

/// 4-bit Soft-buffer size field. Variant names encode the size in
/// bytes; reserved values 9..=15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum SoftBufferSize {
    Bytes16000 = 0,
    Bytes25344 = 1,
    Bytes32000 = 2,
    Bytes64000 = 3,
    Bytes128000 = 4,
    Bytes256000 = 5,
    Bytes512000 = 6,
    Bytes1024000 = 7,
    Bytes2048000 = 8,
}

impl SoftBufferSize {
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
            0 => Some(Self::Bytes16000),
            1 => Some(Self::Bytes25344),
            2 => Some(Self::Bytes32000),
            3 => Some(Self::Bytes64000),
            4 => Some(Self::Bytes128000),
            5 => Some(Self::Bytes256000),
            6 => Some(Self::Bytes512000),
            7 => Some(Self::Bytes1024000),
            8 => Some(Self::Bytes2048000),
            _ => None,
        }
    }

    /// Buffer size in bytes.
    #[must_use]
    pub const fn bytes(self) -> u32 {
        match self {
            Self::Bytes16000 => 16_000,
            Self::Bytes25344 => 25_344,
            Self::Bytes32000 => 32_000,
            Self::Bytes64000 => 64_000,
            Self::Bytes128000 => 128_000,
            Self::Bytes256000 => 256_000,
            Self::Bytes512000 => 512_000,
            Self::Bytes1024000 => 1_024_000,
            Self::Bytes2048000 => 2_048_000,
        }
    }
}

// =========================================================================
// NumHarqProcesses
// =========================================================================

/// 2-bit "number of parallel HARQ processes" field. All four encodings
/// are defined (1, 2, 4, 8 processes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum NumHarqProcesses {
    P1 = 0b00,
    P2 = 0b01,
    P4 = 0b10,
    P8 = 0b11,
}

impl NumHarqProcesses {
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
            0b00 => Some(Self::P1),
            0b01 => Some(Self::P2),
            0b10 => Some(Self::P4),
            0b11 => Some(Self::P8),
            _ => None,
        }
    }

    /// Number of HARQ processes carried by this code.
    #[must_use]
    pub const fn processes(self) -> u8 {
        match self {
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P4 => 4,
            Self::P8 => 8,
        }
    }
}

// =========================================================================
// RdClassMu
// =========================================================================

/// 3-bit Radio Device Class μ field (subcarrier scaling factor μ).
/// Variant numeric values match the on-wire encoding. Reserved values
/// 4..=7 are not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum RdClassMu {
    M1 = 0,
    M2 = 1,
    M4 = 2,
    M8 = 3,
}

impl RdClassMu {
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
            0 => Some(Self::M1),
            1 => Some(Self::M2),
            2 => Some(Self::M4),
            3 => Some(Self::M8),
            _ => None,
        }
    }

    /// As a [`Mu`] value (1, 2, 4, or 8).
    #[must_use]
    pub const fn as_mu(self) -> Mu {
        match self {
            Self::M1 => Mu::M1,
            Self::M2 => Mu::M2,
            Self::M4 => Mu::M4,
            Self::M8 => Mu::M8,
        }
    }
}

// =========================================================================
// RdClassBeta
// =========================================================================

/// 4-bit Radio Device Class β field (DFT scaling factor β). Reserved
/// values 6..=15 are not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names encode the spec value")]
pub enum RdClassBeta {
    B1 = 0,
    B2 = 1,
    B4 = 2,
    B8 = 3,
    B12 = 4,
    B16 = 5,
}

impl RdClassBeta {
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
            0 => Some(Self::B1),
            1 => Some(Self::B2),
            2 => Some(Self::B4),
            3 => Some(Self::B8),
            4 => Some(Self::B12),
            5 => Some(Self::B16),
            _ => None,
        }
    }

    /// As a [`Beta`] value.
    #[must_use]
    pub const fn as_beta(self) -> Beta {
        match self {
            Self::B1 => Beta::B1,
            Self::B2 => Beta::B2,
            Self::B4 => Beta::B4,
            Self::B8 => Beta::B8,
            Self::B12 => Beta::B12,
            Self::B16 => Beta::B16,
        }
    }
}

// =========================================================================
// AbsoluteChannel
// =========================================================================

/// Absolute radio channel number. 13-bit range (1..=0x1FFF), non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsoluteChannel(NonZero<u16>);

impl AbsoluteChannel {
    /// Construct from a raw value. Returns `None` on out-of-range input.
    #[must_use]
    #[inline]
    pub const fn try_from_u16(value: u16) -> Option<Self> {
        if value > 0x1FFF {
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
    pub const fn as_u16(self) -> u16 {
        self.0.get()
    }
}

impl From<AbsoluteChannel> for u16 {
    fn from(value: AbsoluteChannel) -> Self {
        value.0.get()
    }
}

impl fmt::Display for AbsoluteChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for AbsoluteChannel {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{}", self.0.get());
    }
}

// =========================================================================
// TransmitPower
// =========================================================================

/// 4-bit Transmit Power field (Table 6.2.1-3a); discriminants are the
/// on-wire code points and all 16 are defined. Variant names carry the
/// dBm value.
///
/// Table 6.2.1-3b (the low-power-device variant where code points
/// 0000..=0011 are reserved) is deliberately not modeled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(
    missing_docs,
    reason = "variant names mirror the Table 6.2.1-3a entries"
)]
pub enum TransmitPower {
    DbmNeg40 = 0b0000,
    DbmNeg30 = 0b0001,
    DbmNeg20 = 0b0010,
    DbmNeg16 = 0b0011,
    DbmNeg12 = 0b0100,
    DbmNeg8 = 0b0101,
    DbmNeg4 = 0b0110,
    Dbm0 = 0b0111,
    Dbm4 = 0b1000,
    Dbm7 = 0b1001,
    Dbm10 = 0b1010,
    Dbm13 = 0b1011,
    Dbm16 = 0b1100,
    Dbm19 = 0b1101,
    Dbm21 = 0b1110,
    Dbm23 = 0b1111,
}

impl TransmitPower {
    /// Construct from the raw 4-bit code point. Returns `None` only
    /// for values above 4 bits (all 16 code points are defined).
    #[must_use]
    pub const fn try_from_u8(field: u8) -> Option<Self> {
        Some(match field {
            0b0000 => Self::DbmNeg40,
            0b0001 => Self::DbmNeg30,
            0b0010 => Self::DbmNeg20,
            0b0011 => Self::DbmNeg16,
            0b0100 => Self::DbmNeg12,
            0b0101 => Self::DbmNeg8,
            0b0110 => Self::DbmNeg4,
            0b0111 => Self::Dbm0,
            0b1000 => Self::Dbm4,
            0b1001 => Self::Dbm7,
            0b1010 => Self::Dbm10,
            0b1011 => Self::Dbm13,
            0b1100 => Self::Dbm16,
            0b1101 => Self::Dbm19,
            0b1110 => Self::Dbm21,
            0b1111 => Self::Dbm23,
            _ => return None,
        })
    }

    /// Raw 4-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// TX power in dBm (all Table 6.2.1-3a values are whole dBm).
    #[must_use]
    pub const fn dbm(self) -> i8 {
        match self {
            Self::DbmNeg40 => -40,
            Self::DbmNeg30 => -30,
            Self::DbmNeg20 => -20,
            Self::DbmNeg16 => -16,
            Self::DbmNeg12 => -12,
            Self::DbmNeg8 => -8,
            Self::DbmNeg4 => -4,
            Self::Dbm0 => 0,
            Self::Dbm4 => 4,
            Self::Dbm7 => 7,
            Self::Dbm10 => 10,
            Self::Dbm13 => 13,
            Self::Dbm16 => 16,
            Self::Dbm19 => 19,
            Self::Dbm21 => 21,
            Self::Dbm23 => 23,
        }
    }
}

impl From<TransmitPower> for u8 {
    fn from(value: TransmitPower) -> Self {
        value as u8
    }
}

impl fmt::Display for TransmitPower {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} dBm", self.dbm())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcs_description_covers_all_indices() {
        for i in 0..12 {
            let m = Mcs::try_from_u8(i).unwrap();
            assert!(m.description().is_some(), "MCS {} has no description", i);
        }
        assert!(Mcs::try_from_u8(12).is_none());
    }

    #[test]
    fn beta_idx_exhaustive() {
        for v in [1, 2, 4, 8, 12, 16] {
            assert!(Beta::try_from_u8(v).is_some());
        }
        for v in [0, 3, 5, 7, 9, 11, 13, 15, 17, 32, 255] {
            assert!(
                Beta::try_from_u8(v).is_none(),
                "Beta({}) should be invalid",
                v
            );
        }
    }

    #[test]
    fn transmit_power_table_matches_spec() {
        // Table 6.2.1-3a, exhaustive.
        let expected: [i8; 16] = [
            -40, -30, -20, -16, -12, -8, -4, 0, 4, 7, 10, 13, 16, 19, 21, 23,
        ];
        for (code, dbm) in expected.iter().enumerate() {
            let p = TransmitPower::try_from_u8(code as u8).unwrap();
            assert_eq!(p.dbm(), *dbm, "code point {code}");
            assert_eq!(p.as_u8(), code as u8);
        }
        assert!(TransmitPower::try_from_u8(16).is_none());
    }

    #[test]
    fn absolute_channel_13_bit_range() {
        assert!(AbsoluteChannel::try_from_u16(0).is_none());
        assert!(AbsoluteChannel::try_from_u16(1).is_some());
        assert!(AbsoluteChannel::try_from_u16(0x1FFF).is_some());
        assert!(AbsoluteChannel::try_from_u16(0x2000).is_none());
    }
}
