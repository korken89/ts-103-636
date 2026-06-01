//! Association Request message body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.4.

use heapless::Vec;

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// AssociationRequest body (§6.4.2.4)
// ETSI TS 103 636-4, clause 6.4.2.4, Figure 6.4.2.4-1, Tables 6.4.2.4-1/-2
// ---------------------------------------------------------------------------

/// Maximum number of flow IDs in an Association Request. The on-wire
/// 3-bit Number of Flows field caps at 6 (`0b111` is reserved).
pub const MAX_REQUEST_FLOWS: usize = 6;

/// Owned representation of an Association Request body. Parse and
/// serialize directly; no separate borrowed view.
///
/// Invariants enforced at the type level:
/// * `Current Cluster Channel` only exists inside [`FtModeFields`], i.e.
///   it cannot be present without `FT mode = 1`.
/// * `Number of Flows = 7` is reserved by the spec; `flow_ids` is capped
///   at [`MAX_REQUEST_FLOWS`] by its `heapless::Vec` capacity.
/// * `Setup cause = 7` is reserved; the [`SetupCause`] enum cannot
///   represent it, and parse rejects on-wire `0b111`.
/// * `MAX HARQ Re-TX/Re-RX = 0b11111` is reserved; the [`MaxHarqReTx`]
///   newtype rejects it.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct AssociationRequestParts {
    pub setup_cause: SetupCause,
    pub power_const: PowerConst,
    /// 0..=6 flow IDs. The on-wire `Number of Flows` field is derived
    /// from `flow_ids.len()`.
    pub flow_ids: Vec<FlowId, MAX_REQUEST_FLOWS>,
    pub harq_processes_tx: HarqProcesses,
    pub max_harq_re_tx: MaxHarqReTx,
    pub harq_processes_rx: HarqProcesses,
    pub max_harq_re_rx: MaxHarqReTx,
    /// FT-mode-specific fields. `Some(..)` iff the on-wire FT mode bit is
    /// set. Bundling the Current Cluster Channel inside makes the
    /// `Current => FT mode` invariant unrepresentable.
    pub ft_mode: Option<FtModeFields>,
}

/// FT-mode block of an Association Request (always 7 bytes; +2 if
/// `current_cluster_channel` is `Some`).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct FtModeFields {
    /// Network beacon period code (4 bits, see Table 6.4.2.2-1).
    pub network_beacon_period: NetworkBeaconPeriod,
    /// Cluster beacon period code (4 bits, see Table 6.4.2.2-1).
    pub cluster_beacon_period: ClusterBeaconPeriod,
    pub next_cluster_channel: AbsoluteChannel,
    /// Time to next beacon period, microseconds.
    pub time_to_next: u32,
    pub current_cluster_channel: Option<AbsoluteChannel>,
}

impl AssociationRequestParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        let mut len = 4 + self.flow_ids.len();
        if let Some(ft) = &self.ft_mode {
            len += 7;
            if ft.current_cluster_channel.is_some() {
                len += 2;
            }
        }
        len
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is too short. The
    /// `flow_ids.len() > 6` case is unrepresentable by the heapless
    /// vector's capacity.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        // B0: Setup cause (3) | Number of Flows (3) | Power Const (1) | FT mode (1)
        let n_flows = self.flow_ids.len() as u8;
        let pc_bit = if matches!(self.power_const, PowerConst::Constrained) {
            0x02
        } else {
            0
        };
        let ft_bit = if self.ft_mode.is_some() { 0x01 } else { 0 };
        out[0] = (self.setup_cause.as_u8() << 5) | (n_flows << 2) | pc_bit | ft_bit;

        // B1: Current (1) | Reserved (7)
        let current_bit = if self
            .ft_mode
            .as_ref()
            .and_then(|ft| ft.current_cluster_channel)
            .is_some()
        {
            0x80
        } else {
            0
        };
        out[1] = current_bit;

        // B2: HARQ Processes TX (3) | MAX HARQ Re-TX (5)
        out[2] = (self.harq_processes_tx.as_u8() << 5) | self.max_harq_re_tx.as_u8();

        // B3: HARQ Processes RX (3) | MAX HARQ Re-RX (5)
        out[3] = (self.harq_processes_rx.as_u8() << 5) | self.max_harq_re_rx.as_u8();

        // Flow ID octets.
        let mut pos = 4;
        for fid in &self.flow_ids {
            // ETSI bits 0..=1 reserved (high 2 bits in C), bits 2..=7 = Flow ID (low 6).
            out[pos] = fid.as_u8() & 0x3F;
            pos += 1;
        }

        // FT-mode block.
        if let Some(ft) = &self.ft_mode {
            out[pos] = (ft.network_beacon_period.as_u8() << 4) | ft.cluster_beacon_period.as_u8();
            pos += 1;
            let nc = (ft.next_cluster_channel.as_u16() & 0x1FFF).to_be_bytes();
            out[pos] = nc[0];
            out[pos + 1] = nc[1];
            pos += 2;
            let tt = ft.time_to_next.to_be_bytes();
            out[pos] = tt[0];
            out[pos + 1] = tt[1];
            out[pos + 2] = tt[2];
            out[pos + 3] = tt[3];
            pos += 4;

            if let Some(cc) = ft.current_cluster_channel {
                let raw = (cc.as_u16() & 0x1FFF).to_be_bytes();
                out[pos] = raw[0];
                out[pos + 1] = raw[1];
                pos += 2;
            }
        }

        Ok(pos)
    }

    /// Parse a body buffer into an [`AssociationRequestParts`].
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for any of:
    /// * Buffer too short for the indicated structure.
    /// * Reserved Setup cause value `0b111`.
    /// * Reserved Number of Flows value `0b111`.
    /// * `Current = 1` without `FT mode = 1`.
    /// * A zero in a `NonZero`-backed field (e.g. channel).
    /// * Reserved MAX HARQ Re-TX/Re-RX value `0b11111`.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 4 {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let b1 = buffer[1];
        let b2 = buffer[2];
        let b3 = buffer[3];

        let setup_cause = SetupCause::try_from_u8(b0 >> 5).ok_or(ParsingError::ReservedValue)?;
        let n_flows = (b0 >> 2) & 0x07;
        if n_flows == 0b111 {
            return Err(ParsingError::ReservedValue);
        }
        let power_const = if b0 & 0x02 != 0 {
            PowerConst::Constrained
        } else {
            PowerConst::Unconstrained
        };
        let ft_mode_set = b0 & 0x01 != 0;
        // The Current bit is only meaningful when FT mode is set (the
        // Current Cluster Channel field is otherwise absent); a set bit
        // with FT mode 0 is ignored per the receiver-ignores-reserved
        // convention of clause 6.4.1.
        let current_set = ft_mode_set && b1 & 0x80 != 0;
        let harq_processes_tx = HarqProcesses::new(b2 >> 5).ok_or(ParsingError::ReservedValue)?;
        let max_harq_re_tx = MaxHarqReTx::new(b2 & 0x1F).ok_or(ParsingError::ReservedValue)?;
        let harq_processes_rx = HarqProcesses::new(b3 >> 5).ok_or(ParsingError::ReservedValue)?;
        let max_harq_re_rx = MaxHarqReTx::new(b3 & 0x1F).ok_or(ParsingError::ReservedValue)?;

        let need = 4
            + n_flows as usize
            + if ft_mode_set { 7 } else { 0 }
            + if current_set { 2 } else { 0 };
        if buffer.len() < need {
            return Err(ParsingError::Truncated);
        }

        // Flow IDs.
        let flow_start = 4;
        let flow_end = flow_start + n_flows as usize;
        let mut flow_ids = Vec::new();
        // Top 2 bits of each flow octet are reserved: receiver ignores.
        for b in &buffer[flow_start..flow_end] {
            let fid = FlowId::new(b & 0x3F).ok_or(ParsingError::ReservedValue)?;
            flow_ids
                .push(fid)
                .expect("n_flows < 7 by reserved-value check above");
        }

        let mut pos = flow_end;
        let ft_mode = if ft_mode_set {
            let nbp = NetworkBeaconPeriod::try_from_u8(buffer[pos] >> 4)
                .ok_or(ParsingError::ReservedValue)?;
            let cbp = ClusterBeaconPeriod::try_from_u8(buffer[pos] & 0x0F)
                .ok_or(ParsingError::ReservedValue)?;
            pos += 1;
            let nc_raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
            let next_cluster_channel =
                AbsoluteChannel::new(nc_raw).ok_or(ParsingError::ReservedValue)?;
            pos += 2;
            let time_to_next = u32::from_be_bytes([
                buffer[pos],
                buffer[pos + 1],
                buffer[pos + 2],
                buffer[pos + 3],
            ]);
            pos += 4;
            let current_cluster_channel = if current_set {
                let raw = u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x1FFF;
                let ch = AbsoluteChannel::new(raw).ok_or(ParsingError::ReservedValue)?;
                pos += 2;
                Some(ch)
            } else {
                None
            };
            Some(FtModeFields {
                network_beacon_period: nbp,
                cluster_beacon_period: cbp,
                next_cluster_channel,
                time_to_next,
                current_cluster_channel,
            })
        } else {
            None
        };

        // Ensure pos == need (no trailing bytes belonging to this body).
        debug_assert_eq!(pos, need);

        Ok(AssociationRequestParts {
            setup_cause,
            power_const,
            flow_ids,
            harq_processes_tx,
            max_harq_re_tx,
            harq_processes_rx,
            max_harq_re_rx,
            ft_mode,
        })
    }
}

impl MessageBody for AssociationRequestParts {
    const IE_TYPE: IEType6bit = IEType6bit::AssociationRequest;
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
    fn ar_minimal() -> AssociationRequestParts {
        AssociationRequestParts {
            setup_cause: SetupCause::InitialAssociation,
            power_const: PowerConst::Unconstrained,
            flow_ids: Vec::new(),
            harq_processes_tx: HarqProcesses::new(0).unwrap(),
            max_harq_re_tx: MaxHarqReTx::new(0).unwrap(),
            harq_processes_rx: HarqProcesses::new(0).unwrap(),
            max_harq_re_rx: MaxHarqReTx::new(0).unwrap(),
            ft_mode: None,
        }
    }

    #[test]
    fn association_request_round_trip_minimal() {
        let parts = ar_minimal();
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, parts.encoded_len());
        assert_eq!(n, 4);

        let parsed = AssociationRequestParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.setup_cause, SetupCause::InitialAssociation);
        assert_eq!(parsed.power_const, PowerConst::Unconstrained);
        assert_eq!(parsed.flow_ids.len(), 0);
        assert!(parsed.ft_mode.is_none());
    }

    #[test]
    fn association_request_round_trip_with_flows_and_ft_mode() {
        let flow_ids = Vec::from_slice(&[
            FlowId::from_ie_type(IEType6bit::UserPlaneDataFlow1),
            FlowId::from_ie_type(IEType6bit::UserPlaneDataFlow2),
        ])
        .unwrap();
        let parts = AssociationRequestParts {
            setup_cause: SetupCause::Mobility,
            power_const: PowerConst::Constrained,
            flow_ids,
            harq_processes_tx: HarqProcesses::new(2).unwrap(),
            max_harq_re_tx: MaxHarqReTx::new(5).unwrap(),
            harq_processes_rx: HarqProcesses::new(4).unwrap(),
            max_harq_re_rx: MaxHarqReTx::new(7).unwrap(),
            ft_mode: Some(FtModeFields {
                network_beacon_period: NetworkBeaconPeriod::Ms1000,
                cluster_beacon_period: ClusterBeaconPeriod::Ms1500,
                next_cluster_channel: AbsoluteChannel::new(0x0123).unwrap(),
                time_to_next: 0xCAFEBABE,
                current_cluster_channel: Some(AbsoluteChannel::new(0x0124).unwrap()),
            }),
        };
        let mut buf = [0u8; 32];
        let n = parts.serialize(&mut buf).unwrap();
        // 4 (header) + 2 (flow IDs) + 7 (FT block) + 2 (Current) = 15
        assert_eq!(n, 15);

        let parsed = AssociationRequestParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.setup_cause, SetupCause::Mobility);
        assert_eq!(parsed.power_const, PowerConst::Constrained);
        assert_eq!(parsed.flow_ids.len(), 2);
        assert_eq!(parsed.flow_ids[0].as_u8(), parts.flow_ids[0].as_u8());
        assert_eq!(parsed.flow_ids[1].as_u8(), parts.flow_ids[1].as_u8());
        assert_eq!(parsed.harq_processes_tx.as_u8(), 2);
        assert_eq!(parsed.max_harq_re_tx.as_u8(), 5);
        assert_eq!(parsed.harq_processes_rx.as_u8(), 4);
        assert_eq!(parsed.max_harq_re_rx.as_u8(), 7);
        let ft = parsed.ft_mode.unwrap();
        assert_eq!(ft.network_beacon_period, NetworkBeaconPeriod::Ms1000);
        assert_eq!(ft.cluster_beacon_period, ClusterBeaconPeriod::Ms1500);
        assert_eq!(ft.next_cluster_channel.as_u16(), 0x0123);
        assert_eq!(ft.time_to_next, 0xCAFEBABE);
        assert_eq!(ft.current_cluster_channel.unwrap().as_u16(), 0x0124);
    }

    #[test]
    fn association_request_parser_rejects_reserved_setup_cause() {
        // SetupCause = 0b111 (Reserved). Other fields zero.
        let buf = [0b1110_0000, 0, 0, 0];
        assert!(AssociationRequestParts::parse(&buf).is_err());
    }

    #[test]
    fn association_request_parser_ignores_current_without_ft_mode() {
        // FT mode = 0 (B0 bit 0 = 0), Current = 1 (B1 bit 7 = 1). The
        // Current Cluster Channel field is absent without FT mode, so
        // the bit is ignored (receiver-ignores-reserved convention).
        let buf = [0, 0x80, 0, 0];
        let parts = AssociationRequestParts::parse(&buf).unwrap();
        assert!(parts.ft_mode.is_none());
    }

    #[test]
    #[expect(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn association_request_parser_ignores_reserved_flow_octet_bits() {
        // One flow with the two reserved top bits set: receiver ignores
        // them and reads the 6-bit flow ID.
        let buf = [0b000_001_0_0, 0, 0, 0, 0xC1];
        let parts = AssociationRequestParts::parse(&buf).unwrap();
        assert_eq!(parts.flow_ids.len(), 1);
        assert_eq!(parts.flow_ids[0].as_u8(), 1);
    }

    #[test]
    #[expect(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn association_request_parser_rejects_reserved_number_of_flows() {
        // Setup cause = 0 (valid), Number of Flows = 0b111 reserved.
        let buf = [
            0b000_111_00, // Setup cause=0, N flows=7, Power Const=0, FT=0
            0,
            0,
            0,
        ];
        assert!(AssociationRequestParts::parse(&buf).is_err());
    }
}
