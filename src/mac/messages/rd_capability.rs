//! RD Capability IE (long form) body (generated codec re-export).
//!
//! The codec lives in [`generated::rd_capability`](super::generated::rd_capability); the layout
//! figure is in that module's documentation. The tests below (including
//! the hand-derived golden vector) are the drop-in equivalence oracle
//! and predate the generated codec.

use crate::types::*;

pub use super::generated::rd_capability::*;

/// The 4-octet PHY capability core shared by the fixed part and the
/// additional blocks of an RD Capability IE (rows 4..=7 and 9..=12 of
/// Figure 6.4.3.5-1). Carries no mu/beta: the fixed part has none on
/// the wire, and the additional blocks prepend it separately (see
/// [`AdditionalPhyCapability`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AdditionalPhyCapability {
    /// Radio Device Class mu (subcarrier scaling factor).
    pub mu: RdClassMu,
    /// Radio Device Class beta (DFT scaling factor).
    pub beta: RdClassBeta,
    /// Capability core for this numerology.
    pub phy: PhyCapability,
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;
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
