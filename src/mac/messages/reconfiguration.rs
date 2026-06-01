//! Reconfiguration Request and Response messages body.
//!
//! ETSI TS 103 636-4, clause §6.4.2.7 / 6.4.2.8.

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

// ---------------------------------------------------------------------------
// Reconfiguration Request / Response bodies (§6.4.2.7 / §6.4.2.8)
// ETSI TS 103 636-4, clauses 6.4.2.7-8, Figures 6.4.2.7-1 / 6.4.2.8-1.
// Shared wire layout, different semantics on flag bits and on the
// Number-of-Flows = 0b111 special case (reserved in Request, "all
// accepted" in Response).
// ---------------------------------------------------------------------------

/// HARQ configuration block carried in Reconfiguration messages.
/// Identical wire layout (3-bit processes + 5-bit max re-tx/re-rx) is
/// shared between the TX and RX positions; the field's direction is
/// determined by where the [`HarqConfig`] sits in the parent struct
/// (`tx_harq` vs `rx_harq`).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct HarqConfig {
    pub processes: HarqProcesses,
    pub max_re: MaxHarqReTx,
}

/// Owned representation of a Reconfiguration Request body (§6.4.2.7).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct ReconfigurationRequestParts<'a> {
    /// `Some(..)` iff the TX HARQ flag is set (configuration is requested
    /// to be modified, fields follow).
    pub tx_harq: Option<HarqConfig>,
    /// `Some(..)` iff the RX HARQ flag is set.
    pub rx_harq: Option<HarqConfig>,
    /// `true` iff the RD Capability flag is set (an RD Capability IE
    /// follows this body in the same MAC PDU).
    pub rd_capability_changed: bool,
    pub radio_resource: RadioResourceChange,
    /// 0..=6 flow change entries. Number of Flows = 7 is reserved in
    /// Request and is rejected by [`Self::serialize`].
    pub flows: &'a [FlowEntry],
}

/// Owned representation of a Reconfiguration Response body (§6.4.2.8).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct ReconfigurationResponseParts<'a> {
    /// `Some(..)` iff the TX HARQ flag is set (Request's TX HARQ was
    /// NOT accepted, replacement fields follow).
    pub tx_harq: Option<HarqConfig>,
    /// `Some(..)` iff the RX HARQ flag is set.
    pub rx_harq: Option<HarqConfig>,
    /// `true` iff the RD Capability flag is set.
    pub rd_capability_changed: bool,
    pub radio_resource: RadioResourceChange,
    /// Which flow changes the responder is accepting.
    pub flow_acceptance: FlowChangeAcceptance<'a>,
}

/// How many of the requested flow changes the FT is acknowledging.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FlowChangeAcceptance<'a> {
    /// All flow changes accepted as configured in the Reconfiguration
    /// Request (on-wire Number of Flows = 0b111, no flow octets).
    All,
    /// Specific (0..=6) flow changes accepted. Empty slice corresponds
    /// to on-wire Number of Flows = 0.
    Specific(&'a [FlowEntry]),
}

#[inline]
const fn reconf_flag_byte(
    tx_harq_set: bool,
    rx_harq_set: bool,
    rd_capability: bool,
    n_flows: u8,
    radio_resource: RadioResourceChange,
) -> u8 {
    let tx = if tx_harq_set { 0x80 } else { 0 };
    let rx = if rx_harq_set { 0x40 } else { 0 };
    let rdc = if rd_capability { 0x20 } else { 0 };
    tx | rx | rdc | ((n_flows & 0x07) << 2) | (radio_resource.as_u8() & 0x03)
}

const fn reconf_encoded_len(tx_harq_some: bool, rx_harq_some: bool, flow_count: usize) -> usize {
    let mut len = 1;
    if tx_harq_some {
        len += 1;
    }
    if rx_harq_some {
        len += 1;
    }
    len += flow_count;
    len
}

fn write_harq_octet(out: &mut u8, h: &HarqConfig) {
    *out = (h.processes.as_u8() << 5) | h.max_re.as_u8();
}

fn parse_harq_octet(byte: u8) -> Result<HarqConfig, ParsingError> {
    Ok(HarqConfig {
        processes: HarqProcesses::new(byte >> 5).ok_or(ParsingError::ReservedValue)?,
        max_re: MaxHarqReTx::new(byte & 0x1F).ok_or(ParsingError::ReservedValue)?,
    })
}

fn flows_as_slice(buffer: &[u8]) -> Result<&[FlowEntry], ParsingError> {
    // The reserved bit (mask 0x40) is ignored on receive per the
    // clause 6.4.1 convention; FlowEntry's accessors mask it out.
    // SAFETY: FlowEntry is #[repr(transparent)] over u8 and has no
    // validity invariant (any byte is a representable FlowEntry).
    Ok(unsafe { core::slice::from_raw_parts(buffer.as_ptr().cast::<FlowEntry>(), buffer.len()) })
}

impl ReconfigurationRequestParts<'_> {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        reconf_encoded_len(
            self.tx_harq.is_some(),
            self.rx_harq.is_some(),
            self.flows.len(),
        )
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if `flows.len() > 6` (Number of Flows
    /// = 7 is reserved by the spec), or if the buffer is too short.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        if self.flows.len() > 6 {
            return Err(ExcessiveBitsSet);
        }
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        out[0] = reconf_flag_byte(
            self.tx_harq.is_some(),
            self.rx_harq.is_some(),
            self.rd_capability_changed,
            self.flows.len() as u8,
            self.radio_resource,
        );

        let mut pos = 1;
        if let Some(h) = &self.tx_harq {
            write_harq_octet(&mut out[pos], h);
            pos += 1;
        }
        if let Some(h) = &self.rx_harq {
            write_harq_octet(&mut out[pos], h);
            pos += 1;
        }
        let mut i = 0;
        while i < self.flows.len() {
            out[pos] = self.flows[i].as_raw();
            pos += 1;
            i += 1;
        }
        Ok(pos)
    }

    /// Parse a Reconfiguration Request body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for: short buffer, Number of Flows = 7
    /// (reserved), reserved MAX HARQ Re-TX/Re-RX, or a Flow entry with
    /// its reserved bit set.
    pub fn parse(buffer: &[u8]) -> Result<ReconfigurationRequestParts<'_>, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let tx_set = b0 & 0x80 != 0;
        let rx_set = b0 & 0x40 != 0;
        let rd_cap = b0 & 0x20 != 0;
        let n_flows = (b0 >> 2) & 0x07;
        if n_flows == 0b111 {
            return Err(ParsingError::ReservedValue);
        }
        let radio_resource =
            RadioResourceChange::try_from_u8(b0 & 0x03).ok_or(ParsingError::ReservedValue)?;

        let need = reconf_encoded_len(tx_set, rx_set, n_flows as usize);
        if buffer.len() < need {
            return Err(ParsingError::Truncated);
        }

        let mut pos = 1;
        let tx_harq = if tx_set {
            let h = parse_harq_octet(buffer[pos])?;
            pos += 1;
            Some(h)
        } else {
            None
        };
        let rx_harq = if rx_set {
            let h = parse_harq_octet(buffer[pos])?;
            pos += 1;
            Some(h)
        } else {
            None
        };
        let flows = flows_as_slice(&buffer[pos..pos + n_flows as usize])?;

        Ok(ReconfigurationRequestParts {
            tx_harq,
            rx_harq,
            rd_capability_changed: rd_cap,
            radio_resource,
            flows,
        })
    }
}

impl ReconfigurationResponseParts<'_> {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        let flow_count = match self.flow_acceptance {
            FlowChangeAcceptance::All => 0,
            FlowChangeAcceptance::Specific(f) => f.len(),
        };
        reconf_encoded_len(self.tx_harq.is_some(), self.rx_harq.is_some(), flow_count)
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if a `Specific` flow list has more
    /// than 6 entries (Number of Flows = 7 is the `All` encoding here),
    /// or if the buffer is too short.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let (n_flows, flows): (u8, &[FlowEntry]) = match self.flow_acceptance {
            FlowChangeAcceptance::All => (0b111, &[]),
            FlowChangeAcceptance::Specific(f) => {
                if f.len() > 6 {
                    return Err(ExcessiveBitsSet);
                }
                (f.len() as u8, f)
            }
        };
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        out[0] = reconf_flag_byte(
            self.tx_harq.is_some(),
            self.rx_harq.is_some(),
            self.rd_capability_changed,
            n_flows,
            self.radio_resource,
        );

        let mut pos = 1;
        if let Some(h) = &self.tx_harq {
            write_harq_octet(&mut out[pos], h);
            pos += 1;
        }
        if let Some(h) = &self.rx_harq {
            write_harq_octet(&mut out[pos], h);
            pos += 1;
        }
        let mut i = 0;
        while i < flows.len() {
            out[pos] = flows[i].as_raw();
            pos += 1;
            i += 1;
        }
        Ok(pos)
    }

    /// Parse a Reconfiguration Response body.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for: short buffer, reserved MAX HARQ
    /// Re-TX/Re-RX value, or a Flow entry with its reserved bit set.
    pub fn parse(buffer: &[u8]) -> Result<ReconfigurationResponseParts<'_>, ParsingError> {
        if buffer.is_empty() {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let tx_set = b0 & 0x80 != 0;
        let rx_set = b0 & 0x40 != 0;
        let rd_cap = b0 & 0x20 != 0;
        let n_flows = (b0 >> 2) & 0x07;
        let radio_resource =
            RadioResourceChange::try_from_u8(b0 & 0x03).ok_or(ParsingError::ReservedValue)?;

        let flow_count = if n_flows == 0b111 {
            0
        } else {
            n_flows as usize
        };
        let need = reconf_encoded_len(tx_set, rx_set, flow_count);
        if buffer.len() < need {
            return Err(ParsingError::Truncated);
        }

        let mut pos = 1;
        let tx_harq = if tx_set {
            let h = parse_harq_octet(buffer[pos])?;
            pos += 1;
            Some(h)
        } else {
            None
        };
        let rx_harq = if rx_set {
            let h = parse_harq_octet(buffer[pos])?;
            pos += 1;
            Some(h)
        } else {
            None
        };
        let flow_acceptance = if n_flows == 0b111 {
            FlowChangeAcceptance::All
        } else {
            let slice = flows_as_slice(&buffer[pos..pos + flow_count])?;
            FlowChangeAcceptance::Specific(slice)
        };

        Ok(ReconfigurationResponseParts {
            tx_harq,
            rx_harq,
            rd_capability_changed: rd_cap,
            radio_resource,
            flow_acceptance,
        })
    }
}

impl<'a> MessageBody for ReconfigurationRequestParts<'a> {
    const IE_TYPE: IEType6bit = IEType6bit::ReconfigurationRequest;
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::encoded_len(self)
    }
    #[inline]
    fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        Self::serialize(self, out)
    }
}

impl<'a> MessageBody for ReconfigurationResponseParts<'a> {
    const IE_TYPE: IEType6bit = IEType6bit::ReconfigurationResponse;
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
    // FlowAction is already imported at the top of the test module.

    fn fe(action: FlowAction, raw: u8) -> FlowEntry {
        FlowEntry::new(action, FlowId::new(raw).unwrap())
    }

    #[test]
    fn reconfiguration_request_minimal_round_trip() {
        let parts = ReconfigurationRequestParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flows: &[],
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0);

        let parsed = ReconfigurationRequestParts::parse(&buf[..n]).unwrap();
        assert!(parsed.tx_harq.is_none());
        assert!(parsed.rx_harq.is_none());
        assert!(!parsed.rd_capability_changed);
        assert_eq!(parsed.radio_resource, RadioResourceChange::NoChange);
        assert_eq!(parsed.flows.len(), 0);
    }

    #[test]
    fn reconfiguration_request_full_round_trip() {
        let flows = [
            fe(FlowAction::SetupOrReconfigure, 0b000011),
            fe(FlowAction::Release, 0b000100),
        ];
        let parts = ReconfigurationRequestParts {
            tx_harq: Some(HarqConfig {
                processes: HarqProcesses::new(3).unwrap(),
                max_re: MaxHarqReTx::new(7).unwrap(),
            }),
            rx_harq: Some(HarqConfig {
                processes: HarqProcesses::new(2).unwrap(),
                max_re: MaxHarqReTx::new(5).unwrap(),
            }),
            rd_capability_changed: true,
            radio_resource: RadioResourceChange::ResourceAllocationIeIncluded,
            flows: &flows,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        // 1 (B0) + 2 (TX, RX HARQ) + 2 (flow entries) = 5
        assert_eq!(n, 5);
        // B0 = TX(1)|RX(1)|RDcap(1)|n_flows(010)|RR(11) = 0b1110_1011
        assert_eq!(buf[0], 0b1110_1011);

        let parsed = ReconfigurationRequestParts::parse(&buf[..n]).unwrap();
        let tx = parsed.tx_harq.unwrap();
        assert_eq!(tx.processes.as_u8(), 3);
        assert_eq!(tx.max_re.as_u8(), 7);
        let rx = parsed.rx_harq.unwrap();
        assert_eq!(rx.processes.as_u8(), 2);
        assert_eq!(rx.max_re.as_u8(), 5);
        assert!(parsed.rd_capability_changed);
        assert_eq!(
            parsed.radio_resource,
            RadioResourceChange::ResourceAllocationIeIncluded
        );
        assert_eq!(parsed.flows.len(), 2);
        assert_eq!(parsed.flows[0].action(), FlowAction::SetupOrReconfigure);
        assert_eq!(parsed.flows[0].flow_id().as_u8(), 0b000011);
        assert_eq!(parsed.flows[1].action(), FlowAction::Release);
        assert_eq!(parsed.flows[1].flow_id().as_u8(), 0b000100);
    }

    #[test]
    fn reconfiguration_request_serialize_rejects_reserved_n_flows() {
        let flows = [fe(FlowAction::SetupOrReconfigure, 0); 7];
        let parts = ReconfigurationRequestParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flows: &flows,
        };
        let mut buf = [0u8; 16];
        assert!(parts.serialize(&mut buf).is_err());
    }

    #[test]
    fn reconfiguration_request_parser_rejects_reserved_n_flows() {
        // n_flows = 7 in B0 (bits 4..=2 = 0b111).
        let buf = [0b0001_1100];
        assert!(ReconfigurationRequestParts::parse(&buf).is_err());
    }

    #[test]
    fn reconfiguration_response_accept_all_round_trip() {
        let parts = ReconfigurationResponseParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flow_acceptance: FlowChangeAcceptance::All,
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        // n_flows = 0b111 encoded into bits 4..=2 = 0b00011100.
        assert_eq!(buf[0], 0b0001_1100);

        let parsed = ReconfigurationResponseParts::parse(&buf[..n]).unwrap();
        assert!(matches!(parsed.flow_acceptance, FlowChangeAcceptance::All));
    }

    #[test]
    fn reconfiguration_response_specific_flows_round_trip() {
        let flows = [fe(FlowAction::Release, 0b000010)];
        let parts = ReconfigurationResponseParts {
            tx_harq: None,
            rx_harq: None,
            rd_capability_changed: false,
            radio_resource: RadioResourceChange::NoChange,
            flow_acceptance: FlowChangeAcceptance::Specific(&flows),
        };
        let mut buf = [0u8; 16];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 2);
        let parsed = ReconfigurationResponseParts::parse(&buf[..n]).unwrap();
        let got = match parsed.flow_acceptance {
            FlowChangeAcceptance::Specific(f) => f,
            _ => panic!("expected specific"),
        };
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].action(), FlowAction::Release);
        assert_eq!(got[0].flow_id().as_u8(), 0b000010);
    }
}
