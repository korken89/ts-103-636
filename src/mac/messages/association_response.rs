//! Association Response message body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.5.

use heapless::Vec;

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// AssociationResponse body (§6.4.2.5)
// ETSI TS 103 636-4, clause 6.4.2.5, Figure 6.4.2.5-1, Tables 6.4.2.5-1/-2
// ---------------------------------------------------------------------------

/// Maximum number of flow IDs in a `FlowAcceptance::Specific` list.
/// The on-wire 3-bit Number of Flows field caps at 6; `0b111` is the
/// `All` encoding (no Flow ID octets follow).
pub const MAX_RESPONSE_FLOWS: usize = 6;

/// Owned representation of an Association Response body. The Reject vs.
/// Accept split is modeled at the enum level so a reject can never
/// accidentally carry flow / HARQ / group fields, and vice-versa.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub enum AssociationResponseParts {
    /// Association rejected. On the wire: ACK/NACK bit = 0; the entire
    /// remainder of byte 0 is don't-care for the receiver.
    Reject {
        cause: RejectCause,
        timer: RejectTimer,
    },
    /// Association accepted. On the wire: ACK/NACK bit = 1.
    Accept(AssociationAcceptParts),
}

/// Fields present only on accept.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AssociationAcceptParts {
    /// Which flows the FT is accepting. `All` corresponds to on-wire
    /// Number of Flows = `0b111`; `Specific(..)` to 0..=6 explicit flow
    /// IDs.
    pub flow_acceptance: FlowAcceptance,
    /// `Some(..)` iff the FT is overriding HARQ configuration
    /// (on-wire HARQ-mod bit = 1). When `None`, the requested HARQ
    /// configuration is accepted as-is.
    pub harq_override: Option<HarqOverride>,
    /// `Some(..)` iff the on-wire Group bit = 1.
    pub group: Option<GroupAssignment>,
}

/// HARQ configuration override carried in an accepted Association Response.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct HarqOverride {
    pub harq_processes_rx: HarqProcesses,
    pub max_harq_re_rx: MaxHarqReTx,
    pub harq_processes_tx: HarqProcesses,
    pub max_harq_re_tx: MaxHarqReTx,
}

/// Group / resource-tag pair carried in an accepted Association Response.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct GroupAssignment {
    pub group_id: GroupId,
    pub resource_tag: ResourceTag,
}

/// How many flows the FT is accepting.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FlowAcceptance {
    /// All flows requested in the Association Request are accepted as-is
    /// (on-wire Number of Flows = `0b111`, no Flow ID octets).
    All,
    /// Specific (0..=6) flow IDs accepted. Empty vec is allowed and
    /// corresponds to on-wire Number of Flows = 0.
    Specific(Vec<FlowId, MAX_RESPONSE_FLOWS>),
}

impl AssociationResponseParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        match self {
            AssociationResponseParts::Reject { .. } => 2,
            AssociationResponseParts::Accept(a) => {
                let mut len = 1;
                if a.harq_override.is_some() {
                    len += 2;
                }
                len += match &a.flow_acceptance {
                    FlowAcceptance::All => 0,
                    FlowAcceptance::Specific(ids) => ids.len(),
                };
                if a.group.is_some() {
                    len += 2;
                }
                len
            }
        }
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is too short. The
    /// `Specific.len() > 6` case is unrepresentable by the heapless
    /// vector's capacity.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        match self {
            AssociationResponseParts::Reject { cause, timer } => {
                // ACK/NACK = 0, rest of B0 don't-care, written as 0.
                out[0] = 0;
                // B1: [Reject Cause (4) | Reject Timer (4)]
                out[1] = (cause.as_u8() << 4) | timer.as_u8();
                Ok(2)
            }
            AssociationResponseParts::Accept(a) => {
                let n_flows = match &a.flow_acceptance {
                    FlowAcceptance::All => 0b111,
                    FlowAcceptance::Specific(ids) => ids.len() as u8,
                };
                // B0:
                //   ACK/NACK (1) | Reserved (1) | HARQ-mod (1) |
                //   Number of Flows (3) | Group (1) | Reserved (1)
                let harq_mod_bit = if a.harq_override.is_some() { 0x20 } else { 0 };
                let group_bit = if a.group.is_some() { 0x02 } else { 0 };
                out[0] = 0x80 | harq_mod_bit | (n_flows << 2) | group_bit;

                let mut pos = 1;
                if let Some(h) = &a.harq_override {
                    // B1: HARQ Processes RX (3) | MAX HARQ Re-RX (5)
                    out[pos] = (h.harq_processes_rx.as_u8() << 5) | h.max_harq_re_rx.as_u8();
                    pos += 1;
                    // B2: HARQ Processes TX (3) | MAX HARQ Re-TX (5)
                    out[pos] = (h.harq_processes_tx.as_u8() << 5) | h.max_harq_re_tx.as_u8();
                    pos += 1;
                }

                if let FlowAcceptance::Specific(ids) = &a.flow_acceptance {
                    for fid in ids {
                        // [Reserved (2) | Flow ID (6)]
                        out[pos] = fid.as_u8() & 0x3F;
                        pos += 1;
                    }
                }

                if let Some(g) = a.group {
                    // [Reserved (1) | Group ID (7)]
                    out[pos] = g.group_id.as_u8() & 0x7F;
                    pos += 1;
                    // [Reserved (1) | Resource Tag (7)]
                    out[pos] = g.resource_tag.as_u8() & 0x7F;
                    pos += 1;
                }

                Ok(pos)
            }
        }
    }

    /// Parse an Association Response body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for: short buffer, reserved Reject Cause
    /// (5..=15), reserved Reject Timer (9..=15), reserved MAX HARQ
    /// Re-TX/Re-RX (`0b11111`), Flow ID byte with reserved bits set,
    /// Group ID / Resource Tag byte with reserved bit set, or zero in a
    /// `NonZero`-backed field.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let ack = b0 & 0x80 != 0;
        if !ack {
            // NACK path: need 2 bytes total. Bits 1..=7 of B0 ignored.
            if buffer.len() < 2 {
                return Err(ParsingError::Truncated);
            }
            let b1 = buffer[1];
            let cause = RejectCause::try_from_u8(b1 >> 4).ok_or(ParsingError::ReservedValue)?;
            let timer = RejectTimer::try_from_u8(b1 & 0x0F).ok_or(ParsingError::ReservedValue)?;
            return Ok(AssociationResponseParts::Reject { cause, timer });
        }

        let harq_mod_set = b0 & 0x20 != 0;
        let n_flows = (b0 >> 2) & 0x07;
        let group_set = b0 & 0x02 != 0;

        let flow_count = if n_flows == 0b111 {
            0
        } else {
            n_flows as usize
        };
        let need =
            1 + if harq_mod_set { 2 } else { 0 } + flow_count + if group_set { 2 } else { 0 };
        if buffer.len() < need {
            return Err(ParsingError::Truncated);
        }

        let mut pos = 1;
        let harq_override = if harq_mod_set {
            let h_rx = buffer[pos];
            pos += 1;
            let h_tx = buffer[pos];
            pos += 1;
            Some(HarqOverride {
                harq_processes_rx: HarqProcesses::new(h_rx >> 5)
                    .ok_or(ParsingError::ReservedValue)?,
                max_harq_re_rx: MaxHarqReTx::new(h_rx & 0x1F).ok_or(ParsingError::ReservedValue)?,
                harq_processes_tx: HarqProcesses::new(h_tx >> 5)
                    .ok_or(ParsingError::ReservedValue)?,
                max_harq_re_tx: MaxHarqReTx::new(h_tx & 0x1F).ok_or(ParsingError::ReservedValue)?,
            })
        } else {
            None
        };

        let flow_acceptance = if n_flows == 0b111 {
            FlowAcceptance::All
        } else {
            let flow_end = pos + flow_count;
            let mut flows = Vec::new();
            // Top 2 bits of each flow octet are reserved: receiver
            // ignores.
            for b in &buffer[pos..flow_end] {
                let fid = FlowId::new(b & 0x3F).ok_or(ParsingError::ReservedValue)?;
                flows
                    .push(fid)
                    .expect("flow_count <= MAX_RESPONSE_FLOWS by 3-bit-field constraint");
            }
            pos = flow_end;
            FlowAcceptance::Specific(flows)
        };

        let group = if group_set {
            // The high bit of both octets is reserved: receiver ignores.
            let g_byte = buffer[pos];
            let t_byte = buffer[pos + 1];
            pos += 2;
            Some(GroupAssignment {
                group_id: GroupId::new(g_byte & 0x7F).ok_or(ParsingError::ReservedValue)?,
                resource_tag: ResourceTag::new(t_byte & 0x7F).ok_or(ParsingError::ReservedValue)?,
            })
        } else {
            None
        };

        debug_assert_eq!(pos, need);

        Ok(AssociationResponseParts::Accept(AssociationAcceptParts {
            flow_acceptance,
            harq_override,
            group,
        }))
    }
}

impl MessageBody for AssociationResponseParts {
    const IE_TYPE: IEType6bit = IEType6bit::AssociationResponse;
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::encoded_len(self)
    }
    #[inline]
    fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        Self::serialize(self, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn association_response_reject_round_trip() {
        let parts = AssociationResponseParts::Reject {
            cause: RejectCause::ShortRdIdConflict,
            timer: RejectTimer::S60,
        };
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 2);
        let parsed = AssociationResponseParts::parse(&buf[..n]).unwrap();
        match parsed {
            AssociationResponseParts::Reject { cause, timer } => {
                assert_eq!(cause, RejectCause::ShortRdIdConflict);
                assert_eq!(timer, RejectTimer::S60);
            }
            _ => panic!("expected reject"),
        }
    }

    #[test]
    fn association_response_accept_all_flows_no_harq_no_group_round_trip() {
        let parts = AssociationResponseParts::Accept(AssociationAcceptParts {
            flow_acceptance: FlowAcceptance::All,
            harq_override: None,
            group: None,
        });
        let mut buf = [0u8; 8];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        // B0 = ACK(1) | reserved(0) | harq-mod(0) | n_flows(0b111) | group(0) | reserved(0)
        //    = 0b1_0_0_111_00 = 0x9C
        assert_eq!(buf[0], 0x9C);
        let parsed = AssociationResponseParts::parse(&buf[..n]).unwrap();
        match parsed {
            AssociationResponseParts::Accept(a) => {
                assert!(matches!(a.flow_acceptance, FlowAcceptance::All));
                assert!(a.harq_override.is_none());
                assert!(a.group.is_none());
            }
            _ => panic!("expected accept"),
        }
    }

    #[test]
    fn association_response_accept_full_round_trip() {
        let flows = Vec::from_slice(&[
            FlowId::new(0b000011).unwrap(),
            FlowId::new(0b000100).unwrap(),
        ])
        .unwrap();
        let parts = AssociationResponseParts::Accept(AssociationAcceptParts {
            flow_acceptance: FlowAcceptance::Specific(flows),
            harq_override: Some(HarqOverride {
                harq_processes_rx: HarqProcesses::new(2).unwrap(),
                max_harq_re_rx: MaxHarqReTx::new(5).unwrap(),
                harq_processes_tx: HarqProcesses::new(4).unwrap(),
                max_harq_re_tx: MaxHarqReTx::new(7).unwrap(),
            }),
            group: Some(GroupAssignment {
                group_id: GroupId::new(0x42).unwrap(),
                resource_tag: ResourceTag::new(0x21).unwrap(),
            }),
        });
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (B0) + 2 (HARQ override) + 2 (flow ids) + 2 (group) = 7
        assert_eq!(n, 7);

        let parsed = AssociationResponseParts::parse(&buf[..n]).unwrap();
        let a = match parsed {
            AssociationResponseParts::Accept(a) => a,
            _ => panic!("expected accept"),
        };
        let h = a.harq_override.unwrap();
        assert_eq!(h.harq_processes_rx.as_u8(), 2);
        assert_eq!(h.max_harq_re_rx.as_u8(), 5);
        assert_eq!(h.harq_processes_tx.as_u8(), 4);
        assert_eq!(h.max_harq_re_tx.as_u8(), 7);
        let parsed_flows = match a.flow_acceptance {
            FlowAcceptance::Specific(f) => f,
            _ => panic!("expected specific flows"),
        };
        assert_eq!(parsed_flows.len(), 2);
        assert_eq!(parsed_flows[0].as_u8(), 0b000011);
        assert_eq!(parsed_flows[1].as_u8(), 0b000100);
        let g = a.group.unwrap();
        assert_eq!(g.group_id.as_u8(), 0x42);
        assert_eq!(g.resource_tag.as_u8(), 0x21);
    }

    #[test]
    fn association_response_parser_rejects_reserved_reject_cause() {
        // ACK=0, B1 = reject cause 5 (reserved) | reject timer 0
        let buf = [0x00, 0b0101_0000];
        assert!(AssociationResponseParts::parse(&buf).is_err());
    }

    #[test]
    fn association_response_parser_rejects_reserved_reject_timer() {
        // ACK=0, B1 = reject cause 0 | reject timer 9 (reserved)
        let buf = [0x00, 0b0000_1001];
        assert!(AssociationResponseParts::parse(&buf).is_err());
    }
}
