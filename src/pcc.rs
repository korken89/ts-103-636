//! Physical Control Channel (PCC) bitfield definitions.
//!
//! ETSI TS 103 636-4, clause 6.2.1.
//!
//! All three variants share a common 36-bit prefix. The bit layout is
//! big-endian, MSB-first within each byte (matching the over-the-air
//! order shown in Figures 6.2.1-1 and 6.2.1-2). Byte 0 is the first byte
//! transmitted.
//!
//! ```text
//!   PccType1 (40 bits, Format 000):        PccType2 F000 (80 bits):
//!   +-------+---+-------+------+           +-------+---+-------+------+
//!   |  hdr  |plt| pkt_l | snid |           |  hdr  |plt| pkt_l | snid |
//!   |  3b   |1b |  4b   |  8b  |           |  3b   |1b |  4b   |  8b  |
//!   +-------+---+-------+------+           +-------+---+-------+------+
//!   |          tx_id (16b)        |         |          tx_id (16b)        |
//!   +--------------+--------------+         +--------------+--------------+
//!   | tx_power |rsv|  df_mcs (3b) |         | tx_power |     df_mcs (4b) |
//!   |   4b     |1b |              |         |   4b     |                 |
//!   +----------+---+--------------+         +----------+-----------------+
//!                                            |  rx_id (16b)              |
//!                                            |  ss(2) rv(2) ndi(1) hp(3) |
//!                                            |  fb_fmt(4)  fb_info(12)  |
//! ```
//!
//! PccType2 F001 (Table 6.2.1-2a) is identical to F000 except that the
//! `rv/ndi/hp` bits are replaced by 6 reserved bits (set to zero);
//! the 2-bit Number of Spatial Streams field is retained.

use crate::types::{
    Feedback, Mcs, NetworkId8, PacketLength, PacketLengthType, ShortRdId, TransmitPower,
};
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// PccType1 - 40 bits
// ETSI TS 103 636-4, clause 6.2.1, Table 6.2.1-1, Figure 6.2.1-1
// ---------------------------------------------------------------------------

/// Physical Layer Control Field Type 1, Header Format 000. 40 bits total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PccType1 {
    pub packet_length_type: PacketLengthType,
    pub packet_length: PacketLength,
    pub short_network_id: NetworkId8,
    pub transmitter_identity: ShortRdId,
    pub transmit_power: TransmitPower,
    /// MCS used by the data field. Constrained to 0..=7 by the 3-bit field
    /// width; values 8..=11 are valid in the [`Mcs`] type but cannot fit a
    /// Type 1 PCC and will cause [`Self::to_bytes`] to fail.
    pub df_mcs: Mcs,
}

impl PccType1 {
    /// On-wire Header Format field value (3-bit).
    pub const HEADER_FORMAT: u8 = 0b000;
    /// Highest MCS index the 3-bit DF MCS field can carry.
    pub const MAX_DF_MCS: u8 = 0x7;

    /// Parse 5 bytes.
    ///
    /// The reserved bit (PCC bit 36) is ignored, per Table 6.2.1-1:
    /// "Set to zero by the transmitter. The Receiver shall ignore this
    /// bit."
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] if the header format bits are not `000`.
    pub fn from_bytes(bytes: &[u8; 5]) -> Result<Self, ParsingError> {
        // bytes[0] sits at u64 bits 32..=39; PCC bit N is at u64 bit (39-N).
        let word: u64 =
            u64::from_be_bytes([0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]]);
        let header_format = (word >> 37) & 0b111;
        if header_format != Self::HEADER_FORMAT as u64 {
            return Err(ParsingError::InvalidHeader);
        }
        Ok(Self {
            packet_length_type: PacketLengthType::from_bit(((word >> 36) & 1) != 0),
            packet_length: PacketLength::new(((word >> 32) & 0xF) as u8).expect("masked to 4 bits"),
            short_network_id: NetworkId8::new(((word >> 24) & 0xFF) as u8)
                .ok_or(ParsingError::ReservedValue)?,
            transmitter_identity: ShortRdId::new(((word >> 8) & 0xFFFF) as u16)
                .ok_or(ParsingError::ReservedValue)?,
            transmit_power: TransmitPower::new(((word >> 4) & 0xF) as u8)
                .expect("masked to 4 bits"),
            df_mcs: Mcs::new((word & 0x7) as u8).expect("masked to 3 bits"),
        })
    }

    /// Serialize to 5 bytes. The reserved bit is written as zero.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if `df_mcs` exceeds [`Self::MAX_DF_MCS`]
    /// (the 3-bit field can only carry MCS 0..=7).
    pub fn to_bytes(&self) -> Result<[u8; 5], ExcessiveBitsSet> {
        if self.df_mcs.as_u8() > Self::MAX_DF_MCS {
            return Err(ExcessiveBitsSet);
        }
        let mut word: u64 = 0;
        word |= (Self::HEADER_FORMAT as u64 & 0b111) << 37;
        word |= (u8::from(self.packet_length_type.to_bit()) as u64) << 36;
        word |= (u8::from(self.packet_length) as u64 & 0xF) << 32;
        word |= (u8::from(self.short_network_id) as u64) << 24;
        word |= (u16::from(self.transmitter_identity) as u64 & 0xFFFF) << 8;
        word |= (self.transmit_power.as_u8() as u64 & 0xF) << 4;
        // reserved bit stays 0
        word |= self.df_mcs.as_u8() as u64;
        let bytes = word.to_be_bytes();
        Ok([bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]])
    }
}

// ---------------------------------------------------------------------------
// PccType2F000 - 80 bits
// ETSI TS 103 636-4, clause 6.2.1, Table 6.2.1-2, Figure 6.2.1-2
// ---------------------------------------------------------------------------

/// Physical Layer Control Field Type 2, Header Format 000. 80 bits total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PccType2F000 {
    pub packet_length_type: PacketLengthType,
    pub packet_length: PacketLength,
    pub short_network_id: NetworkId8,
    pub transmitter_identity: ShortRdId,
    pub transmit_power: TransmitPower,
    /// MCS used by the data field. The 4-bit field exactly covers the
    /// defined [`Mcs`] range (0..=11) with 12..=15 reserved.
    pub df_mcs: Mcs,
    pub receiver_identity: ShortRdId,
    /// 2-bit field, indexes [`crate::constants::SPATIAL_STREAMS`].
    pub spatial_streams: u8,
    /// 2-bit DF Redundancy Version.
    pub df_redundancy_version: u8,
    pub df_new_data_indication: bool,
    /// 3-bit DF HARQ process number.
    pub df_harq_process_number: u8,
    /// Typed feedback format + info (clause 6.2.2).
    pub feedback: Feedback,
}

impl PccType2F000 {
    /// On-wire Header Format field value (3-bit).
    pub const HEADER_FORMAT: u8 = 0b000;

    /// Parse 10 bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] if the header format bits are not `000`, or
    /// if the 4-bit DF MCS field carries a reserved value (12..=15).
    pub fn from_bytes(bytes: &[u8; 10]) -> Result<Self, ParsingError> {
        // bytes[0] sits at u128 bits 120..=127; PCC bit N is at u128 bit (127-N).
        let word: u128 = u128::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], 0, 0, 0, 0, 0, 0,
        ]);
        let header_format = ((word >> 125) & 0b111) as u8;
        if header_format != Self::HEADER_FORMAT {
            return Err(ParsingError::InvalidHeader);
        }
        Ok(Self {
            packet_length_type: PacketLengthType::from_bit(((word >> 124) & 1) != 0),
            packet_length: PacketLength::new(((word >> 120) & 0xF) as u8)
                .expect("masked to 4 bits"),
            short_network_id: NetworkId8::new(((word >> 112) & 0xFF) as u8)
                .ok_or(ParsingError::ReservedValue)?,
            transmitter_identity: ShortRdId::new(((word >> 96) & 0xFFFF) as u16)
                .ok_or(ParsingError::ReservedValue)?,
            transmit_power: TransmitPower::new(((word >> 92) & 0xF) as u8)
                .expect("masked to 4 bits"),
            df_mcs: Mcs::new(((word >> 88) & 0xF) as u8).ok_or(ParsingError::ReservedValue)?,
            receiver_identity: ShortRdId::new(((word >> 72) & 0xFFFF) as u16)
                .ok_or(ParsingError::ReservedValue)?,
            spatial_streams: ((word >> 70) & 0b11) as u8,
            df_redundancy_version: ((word >> 68) & 0b11) as u8,
            df_new_data_indication: ((word >> 67) & 1) != 0,
            df_harq_process_number: ((word >> 64) & 0b111) as u8,
            feedback: Feedback::try_from_raw(
                ((word >> 60) & 0xF) as u8,
                ((word >> 48) & 0xFFF) as u16,
            )?,
        })
    }

    /// Serialize to 10 bytes.
    pub fn to_bytes(&self) -> [u8; 10] {
        let mut word: u128 = 0;
        word |= (Self::HEADER_FORMAT as u128 & 0b111) << 125;
        word |= (u8::from(self.packet_length_type.to_bit()) as u128) << 124;
        word |= (u8::from(self.packet_length) as u128 & 0xF) << 120;
        word |= (u8::from(self.short_network_id) as u128) << 112;
        word |= (u16::from(self.transmitter_identity) as u128 & 0xFFFF) << 96;
        word |= (self.transmit_power.as_u8() as u128 & 0xF) << 92;
        word |= (self.df_mcs.as_u8() as u128 & 0xF) << 88;
        word |= (u16::from(self.receiver_identity) as u128 & 0xFFFF) << 72;
        word |= (self.spatial_streams as u128 & 0b11) << 70;
        word |= (self.df_redundancy_version as u128 & 0b11) << 68;
        word |= (self.df_new_data_indication as u128) << 67;
        word |= (self.df_harq_process_number as u128 & 0b111) << 64;
        let (fb_format, fb_info) = self.feedback.to_raw();
        word |= (fb_format as u128 & 0xF) << 60;
        word |= ((fb_info as u128) & 0xFFF) << 48;
        let bytes = word.to_be_bytes();
        [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9],
        ]
    }
}

// ---------------------------------------------------------------------------
// PccType2F001 - 80 bits
// ETSI TS 103 636-4, clause 6.2.1, Table 6.2.1-2a
// ---------------------------------------------------------------------------

/// Physical Layer Control Field Type 2, Header Format 001 (no HARQ
/// feedback). 80 bits total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PccType2F001 {
    pub packet_length_type: PacketLengthType,
    pub packet_length: PacketLength,
    pub short_network_id: NetworkId8,
    pub transmitter_identity: ShortRdId,
    pub transmit_power: TransmitPower,
    /// MCS used by the data field. See [`PccType2F000::df_mcs`].
    pub df_mcs: Mcs,
    pub receiver_identity: ShortRdId,
    pub spatial_streams: u8,
    /// Typed feedback format + info (clause 6.2.2).
    pub feedback: Feedback,
}

impl PccType2F001 {
    /// On-wire Header Format field value (3-bit).
    pub const HEADER_FORMAT: u8 = 0b001;

    /// Parse 10 bytes.
    pub fn from_bytes(bytes: &[u8; 10]) -> Result<Self, ParsingError> {
        let word: u128 = u128::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], 0, 0, 0, 0, 0, 0,
        ]);
        let header_format = ((word >> 125) & 0b111) as u8;
        if header_format != Self::HEADER_FORMAT {
            return Err(ParsingError::InvalidHeader);
        }
        // 6 reserved bits at PCC positions 58..=63 are ignored.
        Ok(Self {
            packet_length_type: PacketLengthType::from_bit(((word >> 124) & 1) != 0),
            packet_length: PacketLength::new(((word >> 120) & 0xF) as u8)
                .expect("masked to 4 bits"),
            short_network_id: NetworkId8::new(((word >> 112) & 0xFF) as u8)
                .ok_or(ParsingError::ReservedValue)?,
            transmitter_identity: ShortRdId::new(((word >> 96) & 0xFFFF) as u16)
                .ok_or(ParsingError::ReservedValue)?,
            transmit_power: TransmitPower::new(((word >> 92) & 0xF) as u8)
                .expect("masked to 4 bits"),
            df_mcs: Mcs::new(((word >> 88) & 0xF) as u8).ok_or(ParsingError::ReservedValue)?,
            receiver_identity: ShortRdId::new(((word >> 72) & 0xFFFF) as u16)
                .ok_or(ParsingError::ReservedValue)?,
            spatial_streams: ((word >> 70) & 0b11) as u8,
            feedback: Feedback::try_from_raw(
                ((word >> 60) & 0xF) as u8,
                ((word >> 48) & 0xFFF) as u16,
            )?,
        })
    }

    /// Serialize to 10 bytes.
    pub fn to_bytes(&self) -> [u8; 10] {
        let mut word: u128 = 0;
        word |= (Self::HEADER_FORMAT as u128 & 0b111) << 125;
        word |= (u8::from(self.packet_length_type.to_bit()) as u128) << 124;
        word |= (u8::from(self.packet_length) as u128 & 0xF) << 120;
        word |= (u8::from(self.short_network_id) as u128) << 112;
        word |= (u16::from(self.transmitter_identity) as u128 & 0xFFFF) << 96;
        word |= (self.transmit_power.as_u8() as u128 & 0xF) << 92;
        word |= (self.df_mcs.as_u8() as u128 & 0xF) << 88;
        word |= (u16::from(self.receiver_identity) as u128 & 0xFFFF) << 72;
        word |= (self.spatial_streams as u128 & 0b11) << 70;
        // 6 reserved bits stay 0
        let (fb_format, fb_info) = self.feedback.to_raw();
        word |= (fb_format as u128 & 0xF) << 60;
        word |= ((fb_info as u128) & 0xFFF) << 48;
        let bytes = word.to_be_bytes();
        [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9],
        ]
    }
}

// ---------------------------------------------------------------------------
// Pcc - top-level enum that dispatches on header_format and length
// ---------------------------------------------------------------------------

/// Top-level Physical Control Field. Mirrors the nrfxlib
/// `nrf_modem_dect_phy_pcc_event` discriminator: 5 bytes -> Type 1,
/// 10 bytes -> Type 2 (with the header_format bits selecting F000 vs
/// F001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Pcc {
    /// Type 1, Header Format 000 (40 bits).
    Type1(PccType1),
    /// Type 2, Header Format 000 (80 bits, HARQ feedback requested).
    Type2F000(PccType2F000),
    /// Type 2, Header Format 001 (80 bits, no HARQ feedback).
    Type2F001(PccType2F001),
}

impl Pcc {
    /// Number of bytes when serialized.
    pub const fn size(&self) -> usize {
        match self {
            Pcc::Type1(_) => 5,
            Pcc::Type2F000(_) | Pcc::Type2F001(_) => 10,
        }
    }

    /// Parse a PCC from a byte slice. The slice length (5 or 10) selects
    /// Type 1 vs Type 2; for 10-byte input, the header_format bits in
    /// byte 0 select F000 vs F001.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] on:
    /// - unsupported length (not 5 or 10)
    /// - unrecognized header format in Type 2 (not 000 or 001)
    /// - any underlying field validation failure in the concrete parser
    pub fn parse(bytes: &[u8]) -> Result<Self, ParsingError> {
        match bytes.len() {
            5 => {
                let arr: &[u8; 5] = bytes[..5].try_into().expect("length checked");
                Ok(Pcc::Type1(PccType1::from_bytes(arr)?))
            }
            10 => {
                let header_format = bytes[0] >> 5;
                let arr: &[u8; 10] = bytes[..10].try_into().expect("length checked");
                match header_format {
                    0b000 => Ok(Pcc::Type2F000(PccType2F000::from_bytes(arr)?)),
                    0b001 => Ok(Pcc::Type2F001(PccType2F001::from_bytes(arr)?)),
                    _ => Err(ParsingError::InvalidHeader),
                }
            }
            // Neither a 5-byte Type 1 nor a 10-byte Type 2 buffer.
            _ => Err(ParsingError::BadLength),
        }
    }

    /// Serialize to a [`PccBytes`]: 5 bytes for Type 1, 10 for Type 2.
    ///
    /// # Errors
    ///
    /// Forwards the [`ExcessiveBitsSet`] from [`PccType1::to_bytes`] when a
    /// Type 1 PCC's `df_mcs` exceeds 7 (the 3-bit field's limit).
    pub fn to_bytes(&self) -> Result<PccBytes, ExcessiveBitsSet> {
        Ok(match self {
            Pcc::Type1(p) => PccBytes::Type1(p.to_bytes()?),
            Pcc::Type2F000(p) => PccBytes::Type2(p.to_bytes()),
            Pcc::Type2F001(p) => PccBytes::Type2(p.to_bytes()),
        })
    }
}

/// Output of [`Pcc::to_bytes`]: a fixed-size 5- or 10-byte array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PccBytes {
    /// 5-byte PCC (Type 1).
    Type1([u8; 5]),
    /// 10-byte PCC (Type 2 F000 or F001).
    Type2([u8; 10]),
}

impl PccBytes {
    /// Total number of bytes.
    #[expect(
        clippy::len_without_is_empty,
        reason = "byte buffers are never empty here"
    )]
    pub const fn len(&self) -> usize {
        match self {
            PccBytes::Type1(_) => 5,
            PccBytes::Type2(_) => 10,
        }
    }

    /// View as a `&[u8]`.
    pub fn as_slice(&self) -> &[u8] {
        match self {
            PccBytes::Type1(a) => a,
            PccBytes::Type2(a) => a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BufferStatus, CodebookIndex3, Cqi, HarqProcess};

    #[test]
    fn pcc_type1_round_trip() {
        let pcc = PccType1 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(7).unwrap(),
            short_network_id: NetworkId8::new(0xAB).unwrap(),
            transmitter_identity: ShortRdId::new(0x1234).unwrap(),
            transmit_power: TransmitPower::Dbm13,
            df_mcs: Mcs::new(5).unwrap(),
        };
        let bytes = pcc.to_bytes().unwrap();
        let parsed = PccType1::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    fn pcc_type1_ignores_set_reserved_bit() {
        // Table 6.2.1-1: "The Receiver shall ignore this bit."
        let pcc = PccType1 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(7).unwrap(),
            short_network_id: NetworkId8::new(0xAB).unwrap(),
            transmitter_identity: ShortRdId::new(0x1234).unwrap(),
            transmit_power: TransmitPower::Dbm13,
            df_mcs: Mcs::new(5).unwrap(),
        };
        let mut bytes = pcc.to_bytes().unwrap();
        // Reserved = PCC bit 36 = bit 3 of the last byte.
        bytes[4] |= 0x08;
        let parsed = PccType1::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    fn pcc_type1_rejects_mcs_out_of_3bit_range() {
        let pcc = PccType1 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(0).unwrap(),
            short_network_id: NetworkId8::new(0x01).unwrap(),
            transmitter_identity: ShortRdId::new(0x0001).unwrap(),
            transmit_power: TransmitPower::Dbm0,
            df_mcs: Mcs::new(8).unwrap(),
        };
        assert!(pcc.to_bytes().is_err());
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows PCC field layout"
    )]
    fn pcc_type1_rejects_bad_header_format() {
        // Header format = 001 (Type 2) -> Type 1 parser rejects
        let mut bytes = [0u8; 5];
        bytes[0] = 0b001_0_0000;
        assert!(PccType1::from_bytes(&bytes).is_err());
    }

    #[test]
    fn pcc_type2_f000_round_trip() {
        let pcc = PccType2F000 {
            packet_length_type: PacketLengthType::Slot,
            packet_length: PacketLength::new(0).unwrap(),
            short_network_id: NetworkId8::new(0x42).unwrap(),
            transmitter_identity: ShortRdId::new(0xCAFE).unwrap(),
            transmit_power: TransmitPower::Dbm0,
            df_mcs: Mcs::new(7).unwrap(),
            receiver_identity: ShortRdId::BROADCAST,
            spatial_streams: 0,
            df_redundancy_version: 1,
            df_new_data_indication: true,
            df_harq_process_number: 3,
            feedback: Feedback::Format1 {
                harq_process: HarqProcess::new(5).unwrap(),
                ack: true,
                buffer_status: BufferStatus::UpTo2048,
                cqi: Cqi::Mcs(Mcs::new(11).unwrap()),
            },
        };
        let bytes = pcc.to_bytes();
        let parsed = PccType2F000::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    fn pcc_type2_f001_round_trip() {
        let pcc = PccType2F001 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(15).unwrap(),
            short_network_id: NetworkId8::new(0x01).unwrap(),
            transmitter_identity: ShortRdId::new(0x0001).unwrap(),
            transmit_power: TransmitPower::DbmNeg40,
            df_mcs: Mcs::new(11).unwrap(),
            receiver_identity: ShortRdId::BROADCAST,
            spatial_streams: 3,
            feedback: Feedback::None,
        };
        let bytes = pcc.to_bytes();
        let parsed = PccType2F001::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows PCC field layout"
    )]
    fn pcc_type2_f001_header_format_bit() {
        // Header format = 001 (only the high 3 bits of byte 0 matter here)
        let mut bytes = [0u8; 10];
        bytes[0] = 0b001_0_0000;
        bytes[1] = 0x01; // short network id
        bytes[2] = 0xAB; // transmitter identity high byte
        bytes[3] = 0xCD; // transmitter identity low byte
        bytes[6] = 0xFF; // receiver identity high byte
        bytes[7] = 0xFF; // receiver identity low byte
        let parsed = PccType2F001::from_bytes(&bytes);
        assert!(parsed.is_ok(), "F001 should accept format 001");
        assert!(
            PccType2F000::from_bytes(&bytes).is_err(),
            "F000 should reject format 001"
        );
    }

    #[test]
    fn pcc_type1_byte_layout() {
        // Construct by setting df_mcs only, all others zero/None -> can't do (NonZero types)
        // Instead test with concrete values and check byte positions.
        let pcc = PccType1 {
            packet_length_type: PacketLengthType::Slot, // bit 3 set
            packet_length: PacketLength::new(0xA).unwrap(), // bits 4..=7 = 0xA
            short_network_id: NetworkId8::new(0x01).unwrap(),
            transmitter_identity: ShortRdId::new(0x0001).unwrap(),
            transmit_power: TransmitPower::Dbm0,
            df_mcs: Mcs::new(0).unwrap(),
        };
        let bytes = pcc.to_bytes().unwrap();
        // byte 0: 000_1_1010 = 0b0001_1010 = 0x1A
        assert_eq!(bytes[0], 0b0001_1010, "byte 0 = {:#010b}", bytes[0]);
    }

    #[test]
    fn pcc_enum_dispatches_type1() {
        let inner = PccType1 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(0).unwrap(),
            short_network_id: NetworkId8::new(0x01).unwrap(),
            transmitter_identity: ShortRdId::new(0x0001).unwrap(),
            transmit_power: TransmitPower::DbmNeg40,
            df_mcs: Mcs::new(0).unwrap(),
        };
        let pcc = Pcc::Type1(inner);
        let bytes = pcc.to_bytes().unwrap();
        assert_eq!(bytes.len(), 5);
        let parsed = Pcc::parse(bytes.as_slice()).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    fn pcc_enum_dispatches_type2_f000() {
        let inner = PccType2F000 {
            packet_length_type: PacketLengthType::Slot,
            packet_length: PacketLength::new(1).unwrap(),
            short_network_id: NetworkId8::new(0x42).unwrap(),
            transmitter_identity: ShortRdId::new(0xCAFE).unwrap(),
            transmit_power: TransmitPower::Dbm0,
            df_mcs: Mcs::new(4).unwrap(),
            receiver_identity: ShortRdId::BROADCAST,
            spatial_streams: 1,
            df_redundancy_version: 0,
            df_new_data_indication: false,
            df_harq_process_number: 0,
            feedback: Feedback::None,
        };
        let pcc = Pcc::Type2F000(inner);
        let bytes = pcc.to_bytes().unwrap();
        assert_eq!(bytes.len(), 10);
        let parsed = Pcc::parse(bytes.as_slice()).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    fn pcc_enum_dispatches_type2_f001() {
        let inner = PccType2F001 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(2).unwrap(),
            short_network_id: NetworkId8::new(0xAB).unwrap(),
            transmitter_identity: ShortRdId::new(0x1234).unwrap(),
            transmit_power: TransmitPower::Dbm13,
            df_mcs: Mcs::new(5).unwrap(),
            receiver_identity: ShortRdId::new(0x5678).unwrap(),
            spatial_streams: 0,
            feedback: Feedback::Format2 {
                codebook_index: CodebookIndex3::new(0).unwrap(),
                dual_layer: true,
                buffer_status: BufferStatus::UpTo32,
                cqi: Cqi::Mcs(Mcs::new(2).unwrap()),
            },
        };
        let pcc = Pcc::Type2F001(inner);
        let bytes = pcc.to_bytes().unwrap();
        assert_eq!(bytes.len(), 10);
        let parsed = Pcc::parse(bytes.as_slice()).unwrap();
        assert_eq!(parsed, pcc);
    }

    #[test]
    fn pcc_enum_rejects_bad_length() {
        assert!(Pcc::parse(&[0u8; 4]).is_err());
        assert!(Pcc::parse(&[0u8; 6]).is_err());
        assert!(Pcc::parse(&[0u8; 11]).is_err());
    }

    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows PCC field layout"
    )]
    fn pcc_enum_rejects_unknown_type2_header_format() {
        // 10 bytes, header_format = 010 in byte 0
        let mut bytes = [0u8; 10];
        bytes[0] = 0b010_0_0000;
        assert!(Pcc::parse(&bytes).is_err());
    }

    #[test]
    fn pcc_enum_size() {
        let t1 = Pcc::Type1(PccType1 {
            packet_length_type: PacketLengthType::Subslot,
            packet_length: PacketLength::new(0).unwrap(),
            short_network_id: NetworkId8::new(0x01).unwrap(),
            transmitter_identity: ShortRdId::new(0x0001).unwrap(),
            transmit_power: TransmitPower::DbmNeg40,
            df_mcs: Mcs::new(0).unwrap(),
        });
        assert_eq!(t1.size(), 5);
        let t2 = Pcc::Type2F000(PccType2F000 {
            packet_length_type: PacketLengthType::Slot,
            packet_length: PacketLength::new(0).unwrap(),
            short_network_id: NetworkId8::new(0x01).unwrap(),
            transmitter_identity: ShortRdId::new(0x0001).unwrap(),
            transmit_power: TransmitPower::DbmNeg40,
            df_mcs: Mcs::new(0).unwrap(),
            receiver_identity: ShortRdId::BROADCAST,
            spatial_streams: 0,
            df_redundancy_version: 0,
            df_new_data_indication: false,
            df_harq_process_number: 0,
            feedback: Feedback::None,
        });
        assert_eq!(t2.size(), 10);
    }
}
