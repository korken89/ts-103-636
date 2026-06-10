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
            rd_id: BroadcastRdId::Short(ShortRdId::new(0xABCD).unwrap()),
            ack_nack: true,
            feedback: BroadcastFeedbackType::Mcs,
            resource_allocation_present: true,
            mcs_or_mimo_feedback: 0x07,
        };
        let mut buf = [0u8; 8];
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
            rd_id: BroadcastRdId::Long(LongRdId::new(0x1234_5678).unwrap()),
            ack_nack: false,
            feedback: BroadcastFeedbackType::NoFeedback,
            resource_allocation_present: false,
            mcs_or_mimo_feedback: 0,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 6);
        let parsed = BroadcastIndicationParts::parse(&buf[..n]).unwrap();
        match parsed.rd_id {
            BroadcastRdId::Long(l) => assert_eq!(l.as_u32(), 0x1234_5678),
            _ => panic!("expected long rd id"),
        }
    }
}
