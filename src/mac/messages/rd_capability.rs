//! RD Capability IE (long form) body.
//!
//! ETSI TS 103 636-4, clause 6.4.3.5, Figure 6.4.3.5-1, Table 6.4.3.5-1.
//!
//! Wire format: a 7-octet fixed part (3 header octets + 4 capability
//! octets), followed by 0..=7 additional 5-octet PHY capability blocks.
//! The fixed part carries no mu/beta: with `Number of PHY capabilities`
//! = 000 the RD supports the single numerology already in use on the
//! connection. Each additional block starts with a mu/beta octet and
//! repeats the 4 capability octets (with D_Delay / HalfDup reserved:
//! those two flags are RD-wide and live only in the fixed part).

use heapless::Vec;

use crate::mac::pdu::MessageBody;
use crate::types::*;
use crate::{ExcessiveBitsSet, ParsingError};

/// Maximum number of additional PHY capability blocks (3-bit count field).
pub const MAX_ADDITIONAL_PHY: usize = 7;

/// The 4-octet PHY capability core shared by the fixed part and the
/// additional blocks of an RD Capability IE (rows 4..=7 and 9..=12 of
/// Figure 6.4.3.5-1). Carries no mu/beta: the fixed part has none on
/// the wire, and the additional blocks prepend it separately (see
/// [`AdditionalPhyCapability`]).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct PhyCapability {
    pub rd_power_class: RdPowerClass,
    pub max_nss_for_rx: Nss,
    pub rx_for_tx_diversity: Nss,
    pub rx_gain: RxGain,
    pub max_mcs: Mcs,
    pub soft_buffer_size: SoftBufferSize,
    pub num_harq_processes: NumHarqProcesses,
    pub harq_feedback_delay: HarqFeedbackDelay,
}

/// One additional 5-octet PHY capability block: the mu/beta octet
/// followed by the 4-octet capability core.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AdditionalPhyCapability {
    /// Radio Device Class mu (subcarrier scaling factor).
    pub mu: RdClassMu,
    /// Radio Device Class beta (DFT scaling factor).
    pub beta: RdClassBeta,
    /// Capability core for this numerology.
    pub phy: PhyCapability,
}

#[inline]
const fn pack_power_nss(rd_power: RdPowerClass, max_nss: Nss, rx_tx_div: Nss) -> u8 {
    (rd_power.as_u8() << 4) | (max_nss.as_u8() << 2) | rx_tx_div.as_u8()
}

#[inline]
const fn pack_gain_mcs(rx_gain: RxGain, max_mcs: Mcs) -> u8 {
    (rx_gain.as_u8() << 4) | (max_mcs.as_u8() & 0x0F)
}

#[inline]
const fn pack_buf_harq(sbs: SoftBufferSize, nhp: NumHarqProcesses) -> u8 {
    (sbs.as_u8() << 4) | (nhp.as_u8() << 2)
}

#[inline]
const fn pack_mu_beta(mu: RdClassMu, beta: RdClassBeta) -> u8 {
    (mu.as_u8() << 5) | (beta.as_u8() << 1)
}

fn unpack_power_nss(byte: u8) -> Result<(RdPowerClass, Nss, Nss), ParsingError> {
    let rd_power =
        RdPowerClass::try_from_u8((byte >> 4) & 0x07).ok_or(ParsingError::ReservedValue)?;
    let max_nss = Nss::try_from_u8((byte >> 2) & 0x03).ok_or(ParsingError::ReservedValue)?;
    let rx_tx_div = Nss::try_from_u8(byte & 0x03).ok_or(ParsingError::ReservedValue)?;
    Ok((rd_power, max_nss, rx_tx_div))
}

fn unpack_gain_mcs(byte: u8) -> Result<(RxGain, Mcs), ParsingError> {
    let rx_gain = RxGain::try_from_u8(byte >> 4).ok_or(ParsingError::ReservedValue)?;
    let max_mcs = Mcs::new(byte & 0x0F).ok_or(ParsingError::ReservedValue)?;
    Ok((rx_gain, max_mcs))
}

fn unpack_buf_harq(byte: u8) -> Result<(SoftBufferSize, NumHarqProcesses), ParsingError> {
    let sbs = SoftBufferSize::try_from_u8(byte >> 4).ok_or(ParsingError::ReservedValue)?;
    let nhp =
        NumHarqProcesses::try_from_u8((byte >> 2) & 0x03).ok_or(ParsingError::ReservedValue)?;
    Ok((sbs, nhp))
}

fn unpack_mu_beta(byte: u8) -> Result<(RdClassMu, RdClassBeta), ParsingError> {
    let mu = RdClassMu::try_from_u8(byte >> 5).ok_or(ParsingError::ReservedValue)?;
    let beta = RdClassBeta::try_from_u8((byte >> 1) & 0x0F).ok_or(ParsingError::ReservedValue)?;
    Ok((mu, beta))
}

/// Write the 4-octet capability core. `delay_flags` carries the
/// D_Delay / HalfDup bits for the fixed part (zero for additional
/// blocks, where those bits are reserved).
fn write_core(out: &mut [u8], phy: &PhyCapability, delay_flags: u8) {
    out[0] = pack_power_nss(
        phy.rd_power_class,
        phy.max_nss_for_rx,
        phy.rx_for_tx_diversity,
    );
    out[1] = pack_gain_mcs(phy.rx_gain, phy.max_mcs);
    out[2] = pack_buf_harq(phy.soft_buffer_size, phy.num_harq_processes);
    out[3] = (phy.harq_feedback_delay.subslots() << 4) | delay_flags;
}

fn read_core(buf: &[u8]) -> Result<PhyCapability, ParsingError> {
    let (rd_power_class, max_nss_for_rx, rx_for_tx_diversity) = unpack_power_nss(buf[0])?;
    let (rx_gain, max_mcs) = unpack_gain_mcs(buf[1])?;
    let (soft_buffer_size, num_harq_processes) = unpack_buf_harq(buf[2])?;
    let harq_feedback_delay =
        HarqFeedbackDelay::new(buf[3] >> 4).ok_or(ParsingError::ReservedValue)?;
    Ok(PhyCapability {
        rd_power_class,
        max_nss_for_rx,
        rx_for_tx_diversity,
        rx_gain,
        max_mcs,
        soft_buffer_size,
        num_harq_processes,
        harq_feedback_delay,
    })
}

/// Owned representation of an RD Capability IE body.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[expect(missing_docs, reason = "field names mirror spec-figure column labels")]
pub struct RdCapabilityParts {
    pub release: Release,
    pub group_as: bool,
    pub paging: bool,
    pub operating_modes: OperatingModes,
    pub mesh: bool,
    pub schedul: bool,
    pub mac_security: MacSecuritySupport,
    pub dlc_service_type: DlcServiceType,
    /// Capability core of the fixed part. Applies to the numerology
    /// already in use on the connection (the fixed part carries no
    /// mu/beta on the wire).
    pub base_phy: PhyCapability,
    /// RD-wide `DECT_Delay` capability (encoded in the fixed part's
    /// HARQ feedback delay octet).
    pub d_delay: bool,
    /// RD-wide `HalfDup` capability (encoded in the fixed part's
    /// HARQ feedback delay octet).
    pub half_dup: bool,
    /// Up to [`MAX_ADDITIONAL_PHY`] additional PHY capability blocks.
    pub additional_phy: Vec<AdditionalPhyCapability, MAX_ADDITIONAL_PHY>,
}

impl RdCapabilityParts {
    /// Number of bytes [`Self::serialize`] will write.
    #[must_use]
    #[inline]
    pub fn encoded_len(&self) -> usize {
        7 + 5 * self.additional_phy.len()
    }

    /// Serialize the body into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExcessiveBitsSet`] if the buffer is too short.
    pub fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(ExcessiveBitsSet);
        }

        out[0] = ((self.additional_phy.len() as u8 & 0x07) << 5) | (self.release.as_u8() & 0x1F);
        let group_as_bit = if self.group_as { 0x20 } else { 0 };
        let paging_bit = if self.paging { 0x10 } else { 0 };
        let mesh_bit = if self.mesh { 0x02 } else { 0 };
        let schedul_bit = if self.schedul { 0x01 } else { 0 };
        out[1] = group_as_bit
            | paging_bit
            | ((self.operating_modes.as_u8() & 0x03) << 2)
            | mesh_bit
            | schedul_bit;
        out[2] = (self.mac_security.as_u8() << 5) | ((self.dlc_service_type.as_u8() & 0x07) << 2);

        let dd = if self.d_delay { 0x08 } else { 0 };
        let hd = if self.half_dup { 0x04 } else { 0 };
        write_core(&mut out[3..7], &self.base_phy, dd | hd);

        let mut pos = 7;
        for additional in &self.additional_phy {
            out[pos] = pack_mu_beta(additional.mu, additional.beta);
            write_core(&mut out[pos + 1..pos + 5], &additional.phy, 0);
            pos += 5;
        }
        Ok(pos)
    }

    /// Parse the bytes as `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError`] for short buffer or any reserved enum
    /// value.
    pub fn parse(buffer: &[u8]) -> Result<Self, ParsingError> {
        if buffer.len() < 7 {
            return Err(ParsingError::Truncated);
        }
        let b0 = buffer[0];
        let additional_count = (b0 >> 5) as usize;
        let release = Release::try_from_u8(b0 & 0x1F).ok_or(ParsingError::ReservedValue)?;

        let b1 = buffer[1];
        let group_as = b1 & 0x20 != 0;
        let paging = b1 & 0x10 != 0;
        let operating_modes =
            OperatingModes::try_from_u8((b1 >> 2) & 0x03).ok_or(ParsingError::ReservedValue)?;
        let mesh = b1 & 0x02 != 0;
        let schedul = b1 & 0x01 != 0;

        let b2 = buffer[2];
        let mac_security =
            MacSecuritySupport::try_from_u8(b2 >> 5).ok_or(ParsingError::ReservedValue)?;
        let dlc_service_type =
            DlcServiceType::try_from_u8((b2 >> 2) & 0x07).ok_or(ParsingError::ReservedValue)?;

        let base_phy = read_core(&buffer[3..7])?;
        let d_delay = buffer[6] & 0x08 != 0;
        let half_dup = buffer[6] & 0x04 != 0;

        let need_extra = additional_count * 5;
        if buffer.len() < 7 + need_extra {
            return Err(ParsingError::Truncated);
        }
        let mut additional_phy = Vec::new();
        for i in 0..additional_count {
            let off = 7 + i * 5;
            let (mu, beta) = unpack_mu_beta(buffer[off])?;
            let phy = read_core(&buffer[off + 1..off + 5])?;
            additional_phy
                .push(AdditionalPhyCapability { mu, beta, phy })
                .expect("additional_count <= MAX_ADDITIONAL_PHY by 3-bit field width");
        }

        Ok(Self {
            release,
            group_as,
            paging,
            operating_modes,
            mesh,
            schedul,
            mac_security,
            dlc_service_type,
            base_phy,
            d_delay,
            half_dup,
            additional_phy,
        })
    }
}

impl MessageBody for RdCapabilityParts {
    const IE_TYPE: IEType6bit = IEType6bit::RdCapability;
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
    fn sample_phy() -> PhyCapability {
        PhyCapability {
            rd_power_class: RdPowerClass::ClassII,
            max_nss_for_rx: Nss::N1,
            rx_for_tx_diversity: Nss::N1,
            rx_gain: RxGain::Db0,
            max_mcs: Mcs::new(4).unwrap(),
            soft_buffer_size: SoftBufferSize::Bytes64000,
            num_harq_processes: NumHarqProcesses::P4,
            harq_feedback_delay: HarqFeedbackDelay::new(2).unwrap(),
        }
    }

    fn sample_additional_phy() -> AdditionalPhyCapability {
        AdditionalPhyCapability {
            mu: RdClassMu::M4,
            beta: RdClassBeta::B8,
            phy: PhyCapability {
                rd_power_class: RdPowerClass::ClassIV,
                max_nss_for_rx: Nss::N2,
                rx_for_tx_diversity: Nss::N4,
                rx_gain: RxGain::Db6,
                max_mcs: Mcs::new(9).unwrap(),
                soft_buffer_size: SoftBufferSize::Bytes256000,
                num_harq_processes: NumHarqProcesses::P8,
                harq_feedback_delay: HarqFeedbackDelay::new(5).unwrap(),
            },
        }
    }

    #[test]
    fn rd_capability_minimal_round_trip() {
        let parts = RdCapabilityParts {
            release: Release::R2,
            group_as: false,
            paging: true,
            operating_modes: OperatingModes::Both,
            mesh: false,
            schedul: true,
            mac_security: MacSecuritySupport::Mode1Supported,
            dlc_service_type: DlcServiceType::Type0123,
            base_phy: sample_phy(),
            d_delay: true,
            half_dup: false,
            additional_phy: Vec::new(),
        };
        let mut buf = [0u8; 64];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 7);

        let parsed = RdCapabilityParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.release, Release::R2);
        assert!(parsed.paging);
        assert!(matches!(parsed.operating_modes, OperatingModes::Both));
        assert!(parsed.schedul);
        assert_eq!(parsed.mac_security, MacSecuritySupport::Mode1Supported);
        assert_eq!(parsed.dlc_service_type, DlcServiceType::Type0123);
        assert_eq!(parsed.base_phy.max_mcs.as_u8(), 4);
        assert!(parsed.d_delay);
        assert!(!parsed.half_dup);
        assert_eq!(parsed.additional_phy.len(), 0);
    }

    #[test]
    fn rd_capability_golden_vector_one_additional_phy() {
        // Hand-derived from Figure 6.4.3.5-1 / Table 6.4.3.5-1:
        // fixed part 7 octets, one additional 5-octet block.
        let additional_phy = Vec::from_slice(&[sample_additional_phy()]).unwrap();
        let parts = RdCapabilityParts {
            release: Release::R2,
            group_as: false,
            paging: true,
            operating_modes: OperatingModes::Both,
            mesh: false,
            schedul: true,
            mac_security: MacSecuritySupport::Mode1Supported,
            dlc_service_type: DlcServiceType::Type0123,
            base_phy: sample_phy(),
            d_delay: true,
            half_dup: false,
            additional_phy,
        };
        let mut buf = [0u8; 64];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 12);
        assert_eq!(
            &buf[..n],
            &[
                // count=1 (bits 0-2) | release=R2 (bits 3-7)
                0x22, // paging | operating modes=Both | schedul
                0x19, // mac security=Mode1 | dlc service type=Type0123
                0x30, // power class II | max NSS 1 | rx-tx-div 1
                0x10, // rx gain 0 dB (code 5) | max MCS 4
                0x54, // soft buffer 64000 (code 3) | HARQ proc 4 (code 2)
                0x38, // HARQ fb delay 2 | D_Delay=1 | HalfDup=0
                0x28, // additional block: mu=4 (code 2) | beta=8 (code 3)
                0x46, // power class IV | max NSS 2 | rx-tx-div 4
                0x36, // rx gain +6 dB (code 8) | max MCS 9
                0x89, // soft buffer 256000 (code 5) | HARQ proc 8 (code 3)
                0x5C, // HARQ fb delay 5 | reserved
                0x50,
            ]
        );

        let parsed = RdCapabilityParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.additional_phy.len(), 1);
        let a0 = &parsed.additional_phy[0];
        assert_eq!(a0.mu, RdClassMu::M4);
        assert_eq!(a0.beta, RdClassBeta::B8);
        assert_eq!(a0.phy.rd_power_class, RdPowerClass::ClassIV);
        assert_eq!(a0.phy.max_mcs.as_u8(), 9);
    }

    #[test]
    fn rd_capability_with_two_additional_phy_round_trip() {
        let second = AdditionalPhyCapability {
            mu: RdClassMu::M1,
            beta: RdClassBeta::B1,
            phy: sample_phy(),
        };
        let additional_phy = Vec::from_slice(&[sample_additional_phy(), second]).unwrap();
        let parts = RdCapabilityParts {
            release: Release::R4,
            group_as: true,
            paging: false,
            operating_modes: OperatingModes::FtOnly,
            mesh: true,
            schedul: false,
            mac_security: MacSecuritySupport::NotSupported,
            dlc_service_type: DlcServiceType::Type2,
            base_phy: sample_phy(),
            d_delay: false,
            half_dup: true,
            additional_phy,
        };
        let mut buf = [0u8; 64];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 7 + 2 * 5);

        let parsed = RdCapabilityParts::parse(&buf[..n]).unwrap();
        assert_eq!(parsed.release, Release::R4);
        assert!(parsed.group_as);
        assert!(parsed.mesh);
        assert!(parsed.half_dup);
        assert_eq!(parsed.additional_phy.len(), 2);
        let a0 = &parsed.additional_phy[0];
        assert_eq!(a0.mu, RdClassMu::M4);
        assert_eq!(a0.beta, RdClassBeta::B8);
        assert_eq!(a0.phy.rd_power_class, RdPowerClass::ClassIV);
        assert_eq!(a0.phy.max_mcs.as_u8(), 9);
        let a1 = &parsed.additional_phy[1];
        assert_eq!(a1.mu, RdClassMu::M1);
        assert_eq!(a1.phy.max_mcs.as_u8(), 4);
    }

    #[test]
    fn rd_capability_parser_rejects_truncated_fixed_part() {
        let buf = [0u8; 6];
        assert!(RdCapabilityParts::parse(&buf).is_err());
    }

    #[test]
    fn rd_capability_parser_rejects_truncated_additional_block() {
        // count=1 but only the fixed part present.
        let mut buf = [0u8; 64];
        let parts = RdCapabilityParts {
            release: Release::R2,
            group_as: false,
            paging: false,
            operating_modes: OperatingModes::PtOnly,
            mesh: false,
            schedul: false,
            mac_security: MacSecuritySupport::NotSupported,
            dlc_service_type: DlcServiceType::Type0,
            base_phy: sample_phy(),
            d_delay: false,
            half_dup: false,
            additional_phy: Vec::from_slice(&[sample_additional_phy()]).unwrap(),
        };
        let n = parts.serialize(&mut buf).unwrap();
        assert!(RdCapabilityParts::parse(&buf[..n - 5]).is_err());
    }

    #[test]
    fn rd_capability_parser_rejects_reserved_release() {
        let buf = [0u8; 7];
        assert!(RdCapabilityParts::parse(&buf).is_err());
    }

    #[test]
    fn rd_capability_parser_rejects_reserved_operating_modes() {
        let buf = [0x01, 0b0000_1100, 0, 0, 0, 0, 0];
        assert!(RdCapabilityParts::parse(&buf).is_err());
    }
}
