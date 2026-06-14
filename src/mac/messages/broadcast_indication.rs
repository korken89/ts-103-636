//! Broadcast Indication IE body (generated codec re-export).
//!
//! The codec lives in [`generated::broadcast_indication`](super::generated::broadcast_indication); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

use crate::types::*;

pub use super::generated::broadcast_indication::*;

/// RD ID carried by a Broadcast Indication IE. The variant is selected
/// by the on-wire `IDType` bit (0 = Short, 1 = Long).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "variant names encode the spec entry")]
pub enum BroadcastRdId {
    Short(ShortRdId),
    Long(LongRdId),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broadcast_indication_short_rd_id_round_trip() {
        let parts = BroadcastIndicationParts {
            indication_type: IndicationType::RandomAccessResponse,
            rd_id: BroadcastRdId::Short(ShortRdId::try_from_u16(0xABCD).unwrap()),
            ack_nack: true,
            feedback: BroadcastFeedbackType::Mcs,
            resource_allocation_present: true,
            mcs_or_mimo_feedback: 0x07,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 4);
        let parsed = BroadcastIndicationParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.indication_type, IndicationType::RandomAccessResponse);
        match parsed.rd_id {
            BroadcastRdId::Short(s) => assert_eq!(s.as_u16(), 0xABCD),
            _ => panic!("expected short rd id"),
        }
        assert!(parsed.ack_nack);
        assert_eq!(parsed.feedback, BroadcastFeedbackType::Mcs);
        assert!(parsed.resource_allocation_present);
        assert_eq!(parsed.mcs_or_mimo_feedback, 0x07);
    }

    #[test]
    fn broadcast_indication_long_rd_id_round_trip() {
        let parts = BroadcastIndicationParts {
            indication_type: IndicationType::Paging,
            rd_id: BroadcastRdId::Long(LongRdId::try_from_u32(0x1234_5678).unwrap()),
            ack_nack: false,
            feedback: BroadcastFeedbackType::NoFeedback,
            resource_allocation_present: false,
            mcs_or_mimo_feedback: 0,
        };
        let mut buf = [0; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 6);
        let parsed = BroadcastIndicationParts::parse(&buf[..n]).unwrap();
        match parsed.rd_id {
            BroadcastRdId::Long(l) => assert_eq!(l.as_u32(), 0x1234_5678),
            _ => panic!("expected long rd id"),
        }
    }

    /// Golden vector (minimal) hand-derived from Figure 6.4.3.7-1 and Table 6.4.3.7-1.
    ///
    /// "Minimal" uses the shorter Short RD-ID form (IDType=0 -> 16-bit ID, 4 bytes total).
    ///
    /// Chosen values:
    ///   Indication Type = RandomAccessResponse (code 1 = 0b001)
    ///   IDType = Short (0)
    ///   ACK/NACK = 1 (correctly received)
    ///   Feedback = Mcs (code 1 = 0b01)
    ///   Resource Allocation = 1 (IE follows)
    ///   RD ID = 0xABCD (Short)
    ///   MCS/MIMO Feedback = 0x0A
    ///
    /// Byte 0 layout: [Ind Type(3) | IDType(1) | A(1) | FB(2) | RA(1)]
    ///   bits [7:5] = indication_type = 0b001
    ///   bit  [4]   = IDType = 0 (Short)
    ///   bit  [3]   = ack_nack = 1
    ///   bits [2:1] = feedback = 0b01
    ///   bit  [0]   = resource_allocation_present = 1
    ///   = 0b001_0_1_01_1 = 0x2B
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 4] = [
            0b001_0_1_01_1, // Ind Type=RAR(1) | IDType=Short(0) | ACK=1 | FB=Mcs(01) | RA=1
            0xAB,           // Short RD-ID high byte (0xABCD)
            0xCD,           // Short RD-ID low byte
            0x0A,           // MCS/MIMO feedback byte
        ];
        let parts = BroadcastIndicationParts {
            indication_type: IndicationType::RandomAccessResponse,
            ack_nack: true,
            feedback: BroadcastFeedbackType::Mcs,
            resource_allocation_present: true,
            rd_id: BroadcastRdId::Short(ShortRdId::try_from_u16(0xABCD).unwrap()),
            mcs_or_mimo_feedback: 0x0A,
        };
        let mut buf = [0u8; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(BroadcastIndicationParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.3.7-1 and Table 6.4.3.7-1.
    ///
    /// "Full" uses the Long RD-ID form (IDType=1 -> 32-bit ID, 6 bytes total).
    ///
    /// Chosen values:
    ///   Indication Type = Paging (code 0 = 0b000)
    ///   IDType = Long (1)
    ///   ACK/NACK = 0 (reserved / not applicable for Paging)
    ///   Feedback = Mimo4Antenna (code 3 = 0b11)
    ///   Resource Allocation = 1 (IE follows)
    ///   RD ID = 0x12345678 (Long)
    ///   MCS/MIMO Feedback = 0xC3 (4-antenna MIMO feedback, Table 6.4.3.7-1)
    ///
    /// Byte 0: [Ind Type(3) | IDType(1) | A(1) | FB(2) | RA(1)]
    ///   bits [7:5] = 0b000 (Paging)
    ///   bit  [4]   = 1 (Long)
    ///   bit  [3]   = 0
    ///   bits [2:1] = 0b11 (Mimo4Antenna)
    ///   bit  [0]   = 1
    ///   = 0b000_1_0_11_1 = 0x17
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector_full() {
        const GOLDEN: [u8; 6] = [
            0b000_1_0_11_1, // Ind Type=Paging(0) | IDType=Long(1) | ACK=0 | FB=Mimo4(11) | RA=1
            0x12,           // Long RD-ID byte 0 (0x12345678)
            0x34,           // Long RD-ID byte 1
            0x56,           // Long RD-ID byte 2
            0x78,           // Long RD-ID byte 3
            0xC3,           // MCS/MIMO feedback: MIMO_4_antenna codebook (Table 6.4.3.7-1)
        ];
        let parts = BroadcastIndicationParts {
            indication_type: IndicationType::Paging,
            ack_nack: false,
            feedback: BroadcastFeedbackType::Mimo4Antenna,
            resource_allocation_present: true,
            rd_id: BroadcastRdId::Long(LongRdId::try_from_u32(0x1234_5678).unwrap()),
            mcs_or_mimo_feedback: 0xC3,
        };
        let mut buf = [0u8; 6];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(BroadcastIndicationParts::parse(&GOLDEN).unwrap(), parts);
    }
}
