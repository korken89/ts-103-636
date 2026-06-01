//! Typed coding of the PCC Type 2 Feedback info field.
//!
//! ETSI TS 103 636-4, clause 6.2.2: the 4-bit Feedback format selects
//! how the 12-bit Feedback info is coded (Tables 6.2.2-1 and
//! 6.2.2-2a..2g), with the CQI and Buffer Status code points defined
//! in Tables 6.2.2-3 and 6.2.2-4.
//!
//! Bit packing follows the spec's MSB-first convention: the first
//! table row of each format occupies the most significant bits of the
//! 12-bit field.

use crate::ParsingError;
use crate::types::Mcs;

/// const-fn-compatible Option unwrap mapping `None` to
/// `ParsingError::ReservedValue`.
macro_rules! ok_or_reserved {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return Err(ParsingError::ReservedValue),
        }
    };
}

// =========================================================================
// HarqProcess
// =========================================================================

/// 3-bit HARQ process number (0..=7) referenced by feedback formats
/// 1, 3, 5, and 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HarqProcess(u8);

impl HarqProcess {
    /// Construct from a raw value. Returns `None` if `value > 7`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value <= 7 { Some(Self(value)) } else { None }
    }

    /// Raw process number.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

// =========================================================================
// Cqi
// =========================================================================

/// 4-bit Channel Quality Indicator (Table 6.2.2-3): the highest MCS
/// decodable at <= 10 percent BLER, or out of range when even MCS-0
/// is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Cqi {
    /// Code point 0000: out of range (MCS-0 not decodable at 10
    /// percent BLER).
    OutOfRange,
    /// Code points 0001..=1100: MCS-0..=MCS-11.
    Mcs(Mcs),
}

impl Cqi {
    /// Construct from the raw 4-bit code point. Returns `None` for
    /// the reserved values 1101..=1111 (and anything above 4 bits).
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::OutOfRange),
            1..=12 => match Mcs::new(value - 1) {
                Some(m) => Some(Self::Mcs(m)),
                None => None,
            },
            _ => None,
        }
    }

    /// Raw 4-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::OutOfRange => 0,
            Self::Mcs(m) => m.as_u8() + 1,
        }
    }
}

// =========================================================================
// BufferStatus
// =========================================================================

/// 4-bit Buffer Status (Table 6.2.2-4): remaining data in the sender's
/// MAC buffer after the MAC PDU carried in the associated data field.
/// All 16 code points are defined; variant names carry the upper bound
/// of the byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[expect(missing_docs, reason = "variant names mirror the Table 6.2.2-4 ranges")]
pub enum BufferStatus {
    Empty = 0b0000,
    UpTo16 = 0b0001,
    UpTo32 = 0b0010,
    UpTo64 = 0b0011,
    UpTo128 = 0b0100,
    UpTo256 = 0b0101,
    UpTo512 = 0b0110,
    UpTo1024 = 0b0111,
    UpTo2048 = 0b1000,
    UpTo4096 = 0b1001,
    UpTo8192 = 0b1010,
    UpTo16384 = 0b1011,
    UpTo32768 = 0b1100,
    UpTo65536 = 0b1101,
    UpTo131072 = 0b1110,
    Over131072 = 0b1111,
}

impl BufferStatus {
    /// Construct from the raw 4-bit code point. Returns `None` only
    /// for values above 4 bits (all 16 code points are defined).
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0b0000 => Self::Empty,
            0b0001 => Self::UpTo16,
            0b0010 => Self::UpTo32,
            0b0011 => Self::UpTo64,
            0b0100 => Self::UpTo128,
            0b0101 => Self::UpTo256,
            0b0110 => Self::UpTo512,
            0b0111 => Self::UpTo1024,
            0b1000 => Self::UpTo2048,
            0b1001 => Self::UpTo4096,
            0b1010 => Self::UpTo8192,
            0b1011 => Self::UpTo16384,
            0b1100 => Self::UpTo32768,
            0b1101 => Self::UpTo65536,
            0b1110 => Self::UpTo131072,
            0b1111 => Self::Over131072,
            _ => return None,
        })
    }

    /// Raw 4-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// =========================================================================
// MimoLayers
// =========================================================================

/// 2-bit MIMO feedback of format 5 (Table 6.2.2-2e). Code 0b11 is
/// reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum MimoLayers {
    /// Single layer, codebook index included.
    Single = 0b00,
    /// Dual layers, codebook index included.
    Dual = 0b01,
    /// Four layers, codebook index included.
    Four = 0b10,
}

impl MimoLayers {
    /// Construct from the raw 2-bit code point. Returns `None` for the
    /// reserved value 0b11 and anything above 2 bits.
    #[must_use]
    pub const fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0b00 => Some(Self::Single),
            0b01 => Some(Self::Dual),
            0b10 => Some(Self::Four),
            _ => None,
        }
    }

    /// Raw 2-bit code point.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// =========================================================================
// CodebookIndex3 / CodebookIndex6
// =========================================================================

/// 3-bit codebook index of feedback format 2 (single or dual layer
/// codebooks of ETSI TS 103 636-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CodebookIndex3(u8);

impl CodebookIndex3 {
    /// Construct from a raw value. Returns `None` if `value > 7`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value <= 7 { Some(Self(value)) } else { None }
    }

    /// Raw index.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// 6-bit codebook index of feedback format 5 (single, dual, or four
/// layer codebooks of ETSI TS 103 636-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CodebookIndex6(u8);

impl CodebookIndex6 {
    /// Construct from a raw value. Returns `None` if `value > 63`.
    #[must_use]
    #[inline]
    pub const fn new(value: u8) -> Option<Self> {
        if value <= 63 { Some(Self(value)) } else { None }
    }

    /// Raw index.
    #[must_use]
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

// =========================================================================
// Feedback
// =========================================================================

/// Typed Feedback format + 12-bit Feedback info of the PCC Type 2
/// header (clause 6.2.2). Covers every code point of Table 6.2.2-1:
/// formats 1..=7 are fully decoded, while `None`, `Reserved`, and
/// `Escape` keep the raw bits lossless so any received header can be
/// re-encoded byte-identically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Feedback {
    /// Format 0000: no feedback. The 12 info bits are ignored by the
    /// receiver and written as zero by [`Feedback::to_raw`].
    None,
    /// Format 1 (Table 6.2.2-2a): single HARQ ACK/NACK with buffer
    /// status and CQI.
    Format1 {
        /// HARQ process the feedback refers to.
        harq_process: HarqProcess,
        /// true = ACK, false = NACK.
        ack: bool,
        /// Sender's remaining buffer.
        buffer_status: BufferStatus,
        /// Channel quality indication.
        cqi: Cqi,
    },
    /// Format 2 (Table 6.2.2-2b): MIMO codebook feedback (single or
    /// dual layer) with buffer status and CQI.
    Format2 {
        /// Codebook index (3-bit registry).
        codebook_index: CodebookIndex3,
        /// false = single layer, true = dual layers.
        dual_layer: bool,
        /// Sender's remaining buffer.
        buffer_status: BufferStatus,
        /// Channel quality indication.
        cqi: Cqi,
    },
    /// Format 3 (Table 6.2.2-2c): two HARQ ACK/NACKs and CQI.
    Format3 {
        /// First HARQ process.
        harq_process_0: HarqProcess,
        /// true = ACK, false = NACK (first process).
        ack_0: bool,
        /// Second HARQ process.
        harq_process_1: HarqProcess,
        /// true = ACK, false = NACK (second process).
        ack_1: bool,
        /// Channel quality indication.
        cqi: Cqi,
    },
    /// Format 4 (Table 6.2.2-2d): 8-process HARQ feedback bitmap and
    /// CQI.
    Format4 {
        /// Bit N set = process N decoded successfully and its ACK has
        /// not been sent yet (bit 0 = process 0).
        harq_feedback_bitmap: u8,
        /// Channel quality indication.
        cqi: Cqi,
    },
    /// Format 5 (Table 6.2.2-2e): HARQ ACK/NACK with MIMO layer count
    /// and 6-bit codebook index.
    Format5 {
        /// HARQ process the feedback refers to.
        harq_process: HarqProcess,
        /// true = ACK, false = NACK.
        ack: bool,
        /// Layer count the codebook index refers to.
        mimo: MimoLayers,
        /// Codebook index (6-bit registry).
        codebook_index: CodebookIndex6,
    },
    /// Format 6 (Table 6.2.2-2f): implicit NACK for the given HARQ
    /// process (the retransmission shall use DF Redundancy Version 0),
    /// with buffer status and CQI.
    Format6 {
        /// HARQ process being NACKed.
        harq_process: HarqProcess,
        /// Sender's remaining buffer.
        buffer_status: BufferStatus,
        /// Channel quality indication.
        cqi: Cqi,
    },
    /// Format 7 (Table 6.2.2-2g): buffer status with optional CQI.
    Format7 {
        /// Sender's remaining buffer.
        buffer_status: BufferStatus,
        /// CQI when the CQI select bit is set, `None` otherwise (the
        /// CQI bits are ignored on receive in that case).
        cqi: Option<Cqi>,
    },
    /// Reserved formats 1000..=1110, kept raw for lossless
    /// passthrough.
    Reserved {
        /// The 4-bit format code (8..=14).
        format: u8,
        /// The 12 raw info bits.
        info: u16,
    },
    /// Format 1111 (Escape): proprietary 12 info bits, kept raw.
    Escape {
        /// The 12 raw info bits.
        info: u16,
    },
}

impl Feedback {
    /// Decode from the on-wire 4-bit format and 12-bit info fields.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError::ReservedValue`] if a subfield of a
    /// defined format carries a reserved code point (CQI 1101..=1111,
    /// MIMO 0b11), and [`ParsingError::BadLength`] if `format` exceeds
    /// 4 bits or `info` exceeds 12 bits. Reserved FORMATS (1000..=1110)
    /// and Escape decode losslessly instead of failing.
    pub const fn try_from_raw(format: u8, info: u16) -> Result<Self, ParsingError> {
        if format > 0xF || info > 0xFFF {
            return Err(ParsingError::BadLength);
        }
        let cqi4 = (info & 0xF) as u8;
        let bs_mid = ((info >> 4) & 0xF) as u8;
        Ok(match format {
            0b0000 => Self::None,
            0b0001 => Self::Format1 {
                harq_process: ok_or_reserved!(HarqProcess::new(((info >> 9) & 0x7) as u8)),
                ack: (info >> 8) & 1 != 0,
                buffer_status: ok_or_reserved!(BufferStatus::try_from_u8(bs_mid)),
                cqi: ok_or_reserved!(Cqi::try_from_u8(cqi4)),
            },
            0b0010 => Self::Format2 {
                codebook_index: ok_or_reserved!(CodebookIndex3::new(((info >> 9) & 0x7) as u8)),
                dual_layer: (info >> 8) & 1 != 0,
                buffer_status: ok_or_reserved!(BufferStatus::try_from_u8(bs_mid)),
                cqi: ok_or_reserved!(Cqi::try_from_u8(cqi4)),
            },
            0b0011 => Self::Format3 {
                harq_process_0: ok_or_reserved!(HarqProcess::new(((info >> 9) & 0x7) as u8)),
                ack_0: (info >> 8) & 1 != 0,
                harq_process_1: ok_or_reserved!(HarqProcess::new(((info >> 5) & 0x7) as u8)),
                ack_1: (info >> 4) & 1 != 0,
                cqi: ok_or_reserved!(Cqi::try_from_u8(cqi4)),
            },
            0b0100 => Self::Format4 {
                harq_feedback_bitmap: ((info >> 4) & 0xFF) as u8,
                cqi: ok_or_reserved!(Cqi::try_from_u8(cqi4)),
            },
            0b0101 => Self::Format5 {
                harq_process: ok_or_reserved!(HarqProcess::new(((info >> 9) & 0x7) as u8)),
                ack: (info >> 8) & 1 != 0,
                mimo: ok_or_reserved!(MimoLayers::try_from_u8(((info >> 6) & 0x3) as u8)),
                codebook_index: ok_or_reserved!(CodebookIndex6::new((info & 0x3F) as u8)),
            },
            // Format 6: the bit after the process number is reserved
            // (ignored on receive, zero on transmit).
            0b0110 => Self::Format6 {
                harq_process: ok_or_reserved!(HarqProcess::new(((info >> 9) & 0x7) as u8)),
                buffer_status: ok_or_reserved!(BufferStatus::try_from_u8(bs_mid)),
                cqi: ok_or_reserved!(Cqi::try_from_u8(cqi4)),
            },
            // Format 7: bs(4) | cqi select(1) | cqi(4) | reserved(3).
            // The CQI bits and the trailing reserved bits are ignored
            // when the select bit is clear.
            0b0111 => Self::Format7 {
                buffer_status: ok_or_reserved!(BufferStatus::try_from_u8(
                    ((info >> 8) & 0xF) as u8
                )),
                cqi: if (info >> 7) & 1 != 0 {
                    Some(ok_or_reserved!(Cqi::try_from_u8(((info >> 3) & 0xF) as u8)))
                } else {
                    None
                },
            },
            0b1111 => Self::Escape { info },
            _ => Self::Reserved { format, info },
        })
    }

    /// Encode to the on-wire 4-bit format and 12-bit info fields.
    /// Reserved bits are written as zero.
    #[must_use]
    pub const fn to_raw(self) -> (u8, u16) {
        match self {
            Self::None => (0b0000, 0),
            Self::Format1 {
                harq_process,
                ack,
                buffer_status,
                cqi,
            } => (
                0b0001,
                ((harq_process.as_u8() as u16) << 9)
                    | ((ack as u16) << 8)
                    | ((buffer_status.as_u8() as u16) << 4)
                    | cqi.as_u8() as u16,
            ),
            Self::Format2 {
                codebook_index,
                dual_layer,
                buffer_status,
                cqi,
            } => (
                0b0010,
                ((codebook_index.as_u8() as u16) << 9)
                    | ((dual_layer as u16) << 8)
                    | ((buffer_status.as_u8() as u16) << 4)
                    | cqi.as_u8() as u16,
            ),
            Self::Format3 {
                harq_process_0,
                ack_0,
                harq_process_1,
                ack_1,
                cqi,
            } => (
                0b0011,
                ((harq_process_0.as_u8() as u16) << 9)
                    | ((ack_0 as u16) << 8)
                    | ((harq_process_1.as_u8() as u16) << 5)
                    | ((ack_1 as u16) << 4)
                    | cqi.as_u8() as u16,
            ),
            Self::Format4 {
                harq_feedback_bitmap,
                cqi,
            } => (
                0b0100,
                ((harq_feedback_bitmap as u16) << 4) | cqi.as_u8() as u16,
            ),
            Self::Format5 {
                harq_process,
                ack,
                mimo,
                codebook_index,
            } => (
                0b0101,
                ((harq_process.as_u8() as u16) << 9)
                    | ((ack as u16) << 8)
                    | ((mimo.as_u8() as u16) << 6)
                    | codebook_index.as_u8() as u16,
            ),
            Self::Format6 {
                harq_process,
                buffer_status,
                cqi,
            } => (
                0b0110,
                ((harq_process.as_u8() as u16) << 9)
                    | ((buffer_status.as_u8() as u16) << 4)
                    | cqi.as_u8() as u16,
            ),
            Self::Format7 { buffer_status, cqi } => (
                0b0111,
                ((buffer_status.as_u8() as u16) << 8)
                    | match cqi {
                        Some(c) => (1 << 7) | ((c.as_u8() as u16) << 3),
                        None => 0,
                    },
            ),
            Self::Reserved { format, info } => (format & 0xF, info & 0xFFF),
            Self::Escape { info } => (0b1111, info & 0xFFF),
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::unusual_byte_groupings,
    reason = "binary grouping shows the 12-bit field layout per format"
)]
mod tests {
    use super::*;

    #[test]
    fn format1_golden_vector() {
        // Table 6.2.2-2a: process 5 (101), ACK (1), buffer 32<BS<=64
        // (0011), CQI MCS-2 (0011) -> 101_1_0011_0011.
        let fb = Feedback::Format1 {
            harq_process: HarqProcess::new(5).unwrap(),
            ack: true,
            buffer_status: BufferStatus::UpTo64,
            cqi: Cqi::Mcs(Mcs::new(2).unwrap()),
        };
        assert_eq!(fb.to_raw(), (0b0001, 0b101_1_0011_0011));
        assert_eq!(Feedback::try_from_raw(0b0001, 0b101_1_0011_0011), Ok(fb));
    }

    #[test]
    fn format3_golden_vector() {
        // Table 6.2.2-2c: process 1 NACK, process 7 ACK, CQI out of
        // range -> 001_0_111_1_0000.
        let fb = Feedback::Format3 {
            harq_process_0: HarqProcess::new(1).unwrap(),
            ack_0: false,
            harq_process_1: HarqProcess::new(7).unwrap(),
            ack_1: true,
            cqi: Cqi::OutOfRange,
        };
        assert_eq!(fb.to_raw(), (0b0011, 0b001_0_111_1_0000));
        assert_eq!(Feedback::try_from_raw(0b0011, 0b001_0_111_1_0000), Ok(fb));
    }

    #[test]
    fn format4_golden_vector() {
        // Table 6.2.2-2d: bitmap 0b1010_0001, CQI MCS-11 (1100)
        // -> 10100001_1100.
        let fb = Feedback::Format4 {
            harq_feedback_bitmap: 0b1010_0001,
            cqi: Cqi::Mcs(Mcs::new(11).unwrap()),
        };
        assert_eq!(fb.to_raw(), (0b0100, 0b1010_0001_1100));
        assert_eq!(Feedback::try_from_raw(0b0100, 0b1010_0001_1100), Ok(fb));
    }

    #[test]
    fn format5_golden_vector() {
        // Table 6.2.2-2e: process 2 ACK, four layers (10), codebook 33
        // (100001) -> 010_1_10_100001.
        let fb = Feedback::Format5 {
            harq_process: HarqProcess::new(2).unwrap(),
            ack: true,
            mimo: MimoLayers::Four,
            codebook_index: CodebookIndex6::new(33).unwrap(),
        };
        assert_eq!(fb.to_raw(), (0b0101, 0b010_1_10_100001));
        assert_eq!(Feedback::try_from_raw(0b0101, 0b010_1_10_100001), Ok(fb));
    }

    #[test]
    fn format6_reserved_bit_ignored() {
        // Table 6.2.2-2f: the bit after the process number is
        // reserved; a set bit must not change the decode and is
        // re-encoded as zero.
        let info_with_reserved = 0b011_1_0001_0010;
        let fb = Feedback::try_from_raw(0b0110, info_with_reserved).unwrap();
        assert_eq!(
            fb,
            Feedback::Format6 {
                harq_process: HarqProcess::new(3).unwrap(),
                buffer_status: BufferStatus::UpTo16,
                cqi: Cqi::Mcs(Mcs::new(1).unwrap()),
            }
        );
        assert_eq!(fb.to_raw(), (0b0110, 0b011_0_0001_0010));
    }

    #[test]
    fn format7_cqi_select_semantics() {
        // Table 6.2.2-2g: bs(4) | select(1) | cqi(4) | reserved(3).
        let with_cqi = Feedback::Format7 {
            buffer_status: BufferStatus::UpTo1024,
            cqi: Some(Cqi::Mcs(Mcs::new(4).unwrap())),
        };
        assert_eq!(with_cqi.to_raw(), (0b0111, 0b0111_1_0101_000));
        assert_eq!(
            Feedback::try_from_raw(0b0111, 0b0111_1_0101_000),
            Ok(with_cqi)
        );

        // Select clear: CQI and reserved bits ignored on receive, even
        // if they carry a reserved CQI code point; encoded as zero.
        let no_cqi = Feedback::try_from_raw(0b0111, 0b0111_0_1111_101).unwrap();
        assert_eq!(
            no_cqi,
            Feedback::Format7 {
                buffer_status: BufferStatus::UpTo1024,
                cqi: None,
            }
        );
        assert_eq!(no_cqi.to_raw(), (0b0111, 0b0111_0_0000_000));
    }

    #[test]
    fn none_ignores_info_bits() {
        // Table 6.2.2-1 format 0000: receiver shall ignore info bits.
        assert_eq!(Feedback::try_from_raw(0, 0xABC), Ok(Feedback::None));
        assert_eq!(Feedback::None.to_raw(), (0, 0));
    }

    #[test]
    fn reserved_and_escape_are_lossless() {
        let r = Feedback::try_from_raw(0b1010, 0x5A5).unwrap();
        assert_eq!(
            r,
            Feedback::Reserved {
                format: 0b1010,
                info: 0x5A5
            }
        );
        assert_eq!(r.to_raw(), (0b1010, 0x5A5));

        let e = Feedback::try_from_raw(0b1111, 0xFFF).unwrap();
        assert_eq!(e, Feedback::Escape { info: 0xFFF });
        assert_eq!(e.to_raw(), (0b1111, 0xFFF));
    }

    #[test]
    fn reserved_subfields_rejected() {
        // CQI 1101..=1111 reserved (Table 6.2.2-3).
        assert_eq!(
            Feedback::try_from_raw(0b0001, 0b000_0_0000_1101),
            Err(ParsingError::ReservedValue)
        );
        // MIMO 0b11 reserved (Table 6.2.2-2e).
        assert_eq!(
            Feedback::try_from_raw(0b0101, 0b000_0_11_000000),
            Err(ParsingError::ReservedValue)
        );
        // Out-of-range raw inputs.
        assert_eq!(
            Feedback::try_from_raw(0x10, 0),
            Err(ParsingError::BadLength)
        );
        assert_eq!(
            Feedback::try_from_raw(0, 0x1000),
            Err(ParsingError::BadLength)
        );
    }

    #[test]
    fn cqi_code_points() {
        assert_eq!(Cqi::OutOfRange.as_u8(), 0);
        assert_eq!(Cqi::Mcs(Mcs::new(0).unwrap()).as_u8(), 1);
        assert_eq!(Cqi::Mcs(Mcs::new(11).unwrap()).as_u8(), 12);
        assert!(Cqi::try_from_u8(13).is_none());
        assert!(Cqi::try_from_u8(15).is_none());
    }

    #[test]
    fn all_formats_round_trip() {
        let samples = [
            Feedback::None,
            Feedback::Format1 {
                harq_process: HarqProcess::new(0).unwrap(),
                ack: false,
                buffer_status: BufferStatus::Empty,
                cqi: Cqi::OutOfRange,
            },
            Feedback::Format2 {
                codebook_index: CodebookIndex3::new(7).unwrap(),
                dual_layer: true,
                buffer_status: BufferStatus::Over131072,
                cqi: Cqi::Mcs(Mcs::new(7).unwrap()),
            },
            Feedback::Format3 {
                harq_process_0: HarqProcess::new(6).unwrap(),
                ack_0: true,
                harq_process_1: HarqProcess::new(0).unwrap(),
                ack_1: false,
                cqi: Cqi::Mcs(Mcs::new(3).unwrap()),
            },
            Feedback::Format4 {
                harq_feedback_bitmap: 0xFF,
                cqi: Cqi::OutOfRange,
            },
            Feedback::Format5 {
                harq_process: HarqProcess::new(7).unwrap(),
                ack: false,
                mimo: MimoLayers::Single,
                codebook_index: CodebookIndex6::new(63).unwrap(),
            },
            Feedback::Format6 {
                harq_process: HarqProcess::new(1).unwrap(),
                buffer_status: BufferStatus::UpTo4096,
                cqi: Cqi::Mcs(Mcs::new(9).unwrap()),
            },
            Feedback::Format7 {
                buffer_status: BufferStatus::UpTo128,
                cqi: None,
            },
        ];
        for fb in samples {
            let (format, info) = fb.to_raw();
            assert_eq!(Feedback::try_from_raw(format, info), Ok(fb), "{fb:?}");
        }
    }
}
