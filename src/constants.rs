//! Constants from ETSI TS 103 636-3 (Physical layer) and TS 103 636-4 (MAC layer).
//!
//! Each constant is annotated with the spec clause and table it comes from so
//! values can be cross-checked against the PDFs.

// ---------------------------------------------------------------------------
// PHY: Frame structure - ETSI TS 103 636-3, clauses 4.3-4.4
// ---------------------------------------------------------------------------

/// Radio frame duration in milliseconds.
pub const T_FRAME_MS: u16 = 10;

/// Number of slots per radio frame.
pub const N_SLOT_FRAME: u8 = 24;

/// Radio frame duration in microseconds (10 ms, clause 4.2 of ETSI
/// TS 103 636-3).
///
/// There is deliberately no integer slot-duration constant: a slot is
/// `T_FRAME_US / 24` = 416.67 us, which is not integral in
/// microseconds or nanoseconds. Derive slot timing from the frame
/// duration to avoid accumulating rounding drift (24 x 416 us would
/// lose 16 us per frame).
pub const T_FRAME_US: u32 = 10_000;

/// Number of OFDM symbols per subslot (all numerologies).
pub const N_SYM_SUBSLOT: u8 = 5;

/// Number of OFDM symbols per slot indexed by subcarrier scaling factor mu.
///
/// ETSI TS 103 636-3, Table 4.3-1 (or equivalent - verify when reading PHY spec).
pub const N_SYM_SLOT: [u8; 4] = [10, 20, 40, 80];

/// Number of subslots per slot indexed by subcarrier scaling factor mu.
pub const N_SUBSLOT: [u8; 4] = [2, 4, 8, 16];

/// Number of OFDM symbols in GI+STF combined, indexed by mu.
pub const N_GI_PLUS_STF_SYM: [u8; 4] = [2, 3, 3, 4];

/// Number of occupied subcarriers, indexed by beta (Fourier transform scaling factor).
pub const N_OCC: [u16; 6] = [56, 112, 224, 448, 672, 896];

/// DFT size for each beta.
pub const N_FFT: [u16; 6] = [64, 128, 256, 512, 768, 1024];

/// Cyclic prefix in samples for each beta (N_FFT / 8).
pub const N_CP: [u16; 6] = [8, 16, 32, 64, 96, 128];

/// Number of PCC resource elements (always 98 occupied subcarriers).
/// ETSI TS 103 636-3, clause 6.1.2 (verify).
pub const N_PCC_RE: u16 = 98;

// ---------------------------------------------------------------------------
// PHY: CRC and coding - ETSI TS 103 636-3, clauses 6.1.2-6.1.3
// ---------------------------------------------------------------------------

/// Transport block CRC length in bits (gCRC24A / gCRC24B).
pub const CRC_L: u16 = 24;

/// PCC CRC length in bits (gCRC16).
pub const CRC_PCC_L: u16 = 16;

/// Maximum turbo code block size in bits.
pub const Z: u16 = 2048;

// ---------------------------------------------------------------------------
// PHY: Modulation and coding schemes - ETSI TS 103 636-3, Annex A, Table A-1
// ---------------------------------------------------------------------------

/// Modulation scheme used by an MCS entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Modulation {
    /// pi/2-BPSK (MCS 0).
    Bpsk,
    /// QPSK (MCS 1-2).
    Qpsk,
    /// 16-QAM (MCS 3-4).
    Qam16,
    /// 64-QAM (MCS 5-7).
    Qam64,
    /// 256-QAM (MCS 8-9).
    Qam256,
    /// 1024-QAM (MCS 10-11).
    Qam1024,
}

/// One row of the MCS table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct McsEntry {
    pub modulation: Modulation,
    /// Bits per modulation symbol.
    pub n_bps: u8,
    /// Code rate numerator.
    pub code_rate_num: u8,
    /// Code rate denominator.
    pub code_rate_den: u8,
}

/// MCS table, Annex A Table A-1. Index = MCS index 0..=11.
#[rustfmt::skip]
pub const MCS_TABLE: [McsEntry; 12] = [
    McsEntry { modulation: Modulation::Bpsk,    n_bps: 1,  code_rate_num: 1, code_rate_den: 2 },
    McsEntry { modulation: Modulation::Qpsk,    n_bps: 2,  code_rate_num: 1, code_rate_den: 2 },
    McsEntry { modulation: Modulation::Qpsk,    n_bps: 2,  code_rate_num: 3, code_rate_den: 4 },
    McsEntry { modulation: Modulation::Qam16,   n_bps: 4,  code_rate_num: 1, code_rate_den: 2 },
    McsEntry { modulation: Modulation::Qam16,   n_bps: 4,  code_rate_num: 3, code_rate_den: 4 },
    McsEntry { modulation: Modulation::Qam64,   n_bps: 6,  code_rate_num: 2, code_rate_den: 3 },
    McsEntry { modulation: Modulation::Qam64,   n_bps: 6,  code_rate_num: 3, code_rate_den: 4 },
    McsEntry { modulation: Modulation::Qam64,   n_bps: 6,  code_rate_num: 5, code_rate_den: 6 },
    McsEntry { modulation: Modulation::Qam256,  n_bps: 8,  code_rate_num: 3, code_rate_den: 4 },
    McsEntry { modulation: Modulation::Qam256,  n_bps: 8,  code_rate_num: 5, code_rate_den: 6 },
    McsEntry { modulation: Modulation::Qam1024, n_bps: 10, code_rate_num: 3, code_rate_den: 4 },
    McsEntry { modulation: Modulation::Qam1024, n_bps: 10, code_rate_num: 5, code_rate_den: 6 },
];

// ---------------------------------------------------------------------------
// PHY: DRS - ETSI TS 103 636-3, clause 5.2.3
// ---------------------------------------------------------------------------

/// DRS spacing in time domain (OFDM symbols) for N_TX_eff <= 2.
pub const DRS_TIME_STEP_DEFAULT: u8 = 5;

/// DRS spacing in time domain for N_TX_eff >= 4.
pub const DRS_TIME_STEP_4PLUS: u8 = 10;

// ---------------------------------------------------------------------------
// MAC: Identities - ETSI TS 103 636-4, clause 4.2.3
// ---------------------------------------------------------------------------

/// Reserved Long RD ID.
pub const LONG_RD_ID_RESERVED: u32 = 0x0000_0000;

/// Backend Long RD ID (TS 103 636-4, clause 4.2.3.2).
pub const LONG_RD_ID_BACKEND: u32 = 0xFFFF_FFFE;

/// Broadcast Long RD ID (TS 103 636-4, clause 4.2.3.2).
pub const LONG_RD_ID_BROADCAST: u32 = 0xFFFF_FFFF;

/// Reserved Short RD ID (TS 103 636-4, clause 4.2.3.3).
pub const SHORT_RD_ID_RESERVED: u16 = 0x0000;

/// Broadcast Short RD ID (TS 103 636-4, clause 4.2.3.3).
pub const SHORT_RD_ID_BROADCAST: u16 = 0xFFFF;

// ---------------------------------------------------------------------------
// MAC: MAC Header Type - ETSI TS 103 636-4, clause 6.3.2, Tables 6.3.2-1, 6.3.2-2
// ---------------------------------------------------------------------------

/// MAC Version field value (only value specified, clause 6.3.2).
pub const MAC_VERSION: u8 = 0;

/// MAC Security field values (Table 6.3.2-1).
pub mod mac_security {
    /// MAC security not used.
    pub const NOT_USED: u8 = 0;
    /// MAC security used, no IE present.
    pub const USED_NO_IE: u8 = 1;
    /// MAC security used, MAC Security Info IE present.
    pub const USED_WITH_IE: u8 = 2;
}

/// MAC Header Type field values (Table 6.3.2-2).
pub mod mac_header_type {
    /// DATA MAC PDU header, clause 6.3.3.1.
    pub const DATA_MAC_PDU: u8 = 0x0;
    /// Beacon Header, clause 6.3.3.2.
    pub const BEACON: u8 = 0x1;
    /// Unicast Header, clause 6.3.3.3.
    pub const UNICAST: u8 = 0x2;
    /// RD Broadcasting Header, clause 6.3.3.4.
    pub const RD_BROADCAST: u8 = 0x3;
    /// Escape.
    pub const ESCAPE: u8 = 0xF;
}

// ---------------------------------------------------------------------------
// MAC: MAC Multiplexing Extension - ETSI TS 103 636-4, clause 6.3.4, Table 6.3.4-1
// ---------------------------------------------------------------------------

/// MAC Extension field values (Table 6.3.4-1).
pub mod mac_ext {
    /// IE type defines length (no explicit length field).
    pub const NO_LENGTH: u8 = 0b00;
    /// 8-bit length follows.
    pub const LENGTH_8BIT: u8 = 0b01;
    /// 16-bit length follows.
    pub const LENGTH_16BIT: u8 = 0b10;
    /// Short IE: 5-bit type with 1-bit length encoded in the same byte.
    pub const SHORT_IE: u8 = 0b11;
}

// ---------------------------------------------------------------------------
// MAC: 6-bit IE type registry - ETSI TS 103 636-4, clause 6.3.4, Table 6.3.4-2
// ---------------------------------------------------------------------------

/// 6-bit IE type codes (raw byte values). For a typed wrapper see
/// [`crate::types::IEType6bit`].
pub mod ie_6bit {
    /// Padding (length defined by the type).
    pub const PADDING: u8 = 0b000000;
    /// Higher layer signalling, flow 1.
    pub const HIGHER_LAYER_SIG_FLOW_1: u8 = 0b000001;
    /// Higher layer signalling, flow 2.
    pub const HIGHER_LAYER_SIG_FLOW_2: u8 = 0b000010;
    /// User plane data, flow 1.
    pub const USER_PLANE_FLOW_1: u8 = 0b000011;
    /// User plane data, flow 2.
    pub const USER_PLANE_FLOW_2: u8 = 0b000100;
    /// User plane data, flow 3.
    pub const USER_PLANE_FLOW_3: u8 = 0b000101;
    /// User plane data, flow 4.
    pub const USER_PLANE_FLOW_4: u8 = 0b000110;
    /// Reserved (0b000111).
    /// Network Beacon.
    pub const NETWORK_BEACON: u8 = 0b001000;
    /// Cluster Beacon.
    pub const CLUSTER_BEACON: u8 = 0b001001;
    /// Association Request.
    pub const ASSOCIATION_REQUEST: u8 = 0b001010;
    /// Association Response.
    pub const ASSOCIATION_RESPONSE: u8 = 0b001011;
    /// Association Release.
    pub const ASSOCIATION_RELEASE: u8 = 0b001100;
    /// Reconfiguration Request.
    pub const RECONFIGURATION_REQUEST: u8 = 0b001101;
    /// Reconfiguration Response.
    pub const RECONFIGURATION_RESPONSE: u8 = 0b001110;
    /// Additional MAC messages.
    pub const ADDITIONAL_MAC_MESSAGE: u8 = 0b001111;
    /// MAC Security Info.
    pub const MAC_SECURITY_INFO: u8 = 0b010000;
    /// Route Info.
    pub const ROUTE_INFO: u8 = 0b010001;
    /// Resource allocation.
    pub const RESOURCE_ALLOCATION: u8 = 0b010010;
    /// Random Access Resource.
    pub const RANDOM_ACCESS_RESOURCE: u8 = 0b010011;
    /// RD capability.
    pub const RD_CAPABILITY: u8 = 0b010100;
    /// Neighbouring.
    pub const NEIGHBOURING: u8 = 0b010101;
    /// Broadcast Indication.
    pub const BROADCAST_INDICATION: u8 = 0b010110;
    /// Group Assignment.
    pub const GROUP_ASSIGNMENT: u8 = 0b010111;
    /// Load Info.
    pub const LOAD_INFO: u8 = 0b011000;
    /// Measurement Report.
    pub const MEASUREMENT_REPORT: u8 = 0b011001;
    /// Source Routing.
    pub const SOURCE_ROUTING: u8 = 0b011010;
    /// Joining Beacon.
    pub const JOINING_BEACON: u8 = 0b011011;
    /// Joining Information.
    pub const JOINING_INFORMATION: u8 = 0b011100;
    // 0b011101 to 0b111101 are reserved (Table 6.3.4-2).
    /// Escape (proprietary extensions).
    pub const ESCAPE: u8 = 0b111110;
    /// IE type extension.
    pub const IE_TYPE_EXTENSION: u8 = 0b111111;
}

// ---------------------------------------------------------------------------
// MAC: Physical Header Field - ETSI TS 103 636-4, clause 6.2.1
// ---------------------------------------------------------------------------

/// Header Format values (3 bits, clause 6.2.1).
pub mod header_format {
    /// Type 1 PCC layout (Table 6.2.1-1).
    pub const FORMAT_000: u8 = 0b000;
    /// Type 2 PCC layout (Tables 6.2.1-2 / 6.2.1-2a).
    pub const FORMAT_001: u8 = 0b001;
}

// ---------------------------------------------------------------------------
// MAC: Transmit Power - ETSI TS 103 636-4, clause 6.2.1, Table 6.2.1-3a
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// MAC: Spatial Streams - ETSI TS 103 636-4, clause 6.2.1, Table 6.2.1-4
// ---------------------------------------------------------------------------

/// Number of spatial streams indexed by 2-bit field (Table 6.2.1-4).
pub const SPATIAL_STREAMS: [u8; 4] = [1, 2, 4, 8];

// ---------------------------------------------------------------------------
// MAC: Feedback formats - ETSI TS 103 636-4, clause 6.2.2, Table 6.2.2-1
// ---------------------------------------------------------------------------

/// Feedback Format field values (Table 6.2.2-1, 4 bits).
pub mod feedback_format {
    #![expect(missing_docs, reason = "constant names mirror Table 6.2.2-1")]
    pub const NO_FEEDBACK: u8 = 0b0000;
    pub const FORMAT_1: u8 = 0b0001;
    pub const FORMAT_2: u8 = 0b0010;
    pub const FORMAT_3: u8 = 0b0011;
    pub const FORMAT_4: u8 = 0b0100;
    pub const FORMAT_5: u8 = 0b0101;
    pub const FORMAT_6: u8 = 0b0110;
    pub const FORMAT_7: u8 = 0b0111;
    pub const ESCAPE: u8 = 0b1111;
}

// ---------------------------------------------------------------------------
// Subslot calculation helper
// ---------------------------------------------------------------------------

/// Maximum number of subslots in a single transmission.
///
/// The 4-bit packet_length field is `value+1`, so 0..=15 -> 1..=16 units
/// (clause 6.2.1, Tables 6.2.1-1/-2/-2a).
pub const MAX_SUBSLOTS: u8 = 16;
