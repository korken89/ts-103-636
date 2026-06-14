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

    /// Hand-derived PhyCapability used in both golden vectors.
    ///
    /// Encoding of the 4 capability octets (Figure 6.4.3.5-1 rows 4-7):
    ///   byte 0 (rd_power_class | max_nss_for_rx | rx_for_tx_diversity):
    ///     rd_power_class = ClassIII = code 0b010 -> bits 6-4
    ///     max_nss_for_rx = N2 = code 0b01 -> bits 3-2
    ///     rx_for_tx_diversity = N1 = code 0b00 -> bits 1-0
    ///     -> (0b010 << 4) | (0b01 << 2) | 0b00 = 0x20 | 0x04 = 0x24
    ///   byte 1 (rx_gain | max_mcs):
    ///     rx_gain = Db2 = code 6 = 0b0110 -> bits 7-4
    ///     max_mcs = 7 -> bits 3-0
    ///     -> (6 << 4) | 7 = 0x67
    ///   byte 2 (soft_buffer_size | num_harq_processes | reserved):
    ///     soft_buffer_size = Bytes512000 = code 6 -> bits 7-4
    ///     num_harq_processes = P8 = code 0b11 -> bits 3-2
    ///     -> (6 << 4) | (0b11 << 2) = 0x60 | 0x0C = 0x6C
    ///   byte 3 (harq_feedback_delay in base) = 6 subslots -> (6 << 4) = 0x60,
    ///           plus d_delay/half_dup flags (base-only octet)
    fn golden_phy() -> PhyCapability {
        PhyCapability {
            rd_power_class: RdPowerClass::ClassIII, // code 0b010
            max_nss_for_rx: Nss::N2,                // code 0b01
            rx_for_tx_diversity: Nss::N1,           // code 0b00
            rx_gain: RxGain::Db2,                   // code 6
            max_mcs: Mcs::try_from_u8(7).unwrap(),
            soft_buffer_size: SoftBufferSize::Bytes512000, // code 6
            num_harq_processes: NumHarqProcesses::P8,      // code 0b11
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(6).unwrap(), // 6 subslots
        }
    }

    /// Golden vector (minimal) hand-derived from Figure 6.4.3.5-1 / Table 6.4.3.5-1.
    ///
    /// No additional PHY blocks.  7 bytes total.
    ///
    ///   byte 0: N=0 (bits7-5=000) | release=R3 (code 3 = 0b00011)
    ///     -> 0x03
    ///   byte 1: R(3b)=0 | GA=0 | PG=1 | OM=FtOnly(0b01) | M=0 | S=0
    ///     paging bit4=1 -> 0x10; OperatingModes::FtOnly = 0b01 -> (0b01 << 2) = 0x04
    ///     -> 0x14
    ///   byte 2: mac_security=Mode1Supported(0b001) | dlc_service_type=Type1(0b001) | R(2b)=0
    ///     -> (0b001 << 5) | (0b001 << 2) = 0x20 | 0x04 = 0x24
    ///   byte 3: capability core octet 0 = 0x24  (as derived above)
    ///   byte 4: capability core octet 1 = 0x67
    ///   byte 5: capability core octet 2 = 0x6C
    ///   byte 6: harq_fb_delay=6 | d_delay=1 | half_dup=0 | R(2b)=0
    ///     -> (6 << 4) | 0x08 = 0x68
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows RD Capability IE field layout"
    )]
    fn golden_vector_minimal() {
        const GOLDEN: [u8; 7] = [
            0b000_00011,      // N=0 | release=R3 (code 3)
            0b000_0_1_01_0_0, // R(3b)|GA=0|PG=1|OM=FtOnly(01)|M=0|S=0 = 0x14
            0b001_001_00,     // mac_security=Mode1(001) | dlc_type=Type1(001) | R(2b)
            0b0_010_01_00,    // R|rd_power_class=ClassIII(010)|max_nss=N2(01)|rxtx_div=N1(00)
            0b0110_0111,      // rx_gain=Db2(6) | max_mcs=7
            0b0110_11_00,     // soft_buf=Bytes512000(6) | harq_proc=P8(11) | R(2b)
            0b0110_1_0_00,    // harq_fb_delay=6 | d_delay=1 | half_dup=0 | R(2b)
        ];
        let parts = RdCapabilityParts {
            release: Release::R3, // code 3
            group_as: false,
            paging: true,
            operating_modes: OperatingModes::FtOnly, // code 0b01
            mesh: false,
            schedul: false,
            mac_security: MacSecuritySupport::Mode1Supported, // code 0b001
            dlc_service_type: DlcServiceType::Type1,          // code 0b001
            base_phy: golden_phy(),
            d_delay: true,
            half_dup: false,
            additional_phy: Vec::new(),
        };
        let mut buf = [0u8; 64];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(RdCapabilityParts::parse(&GOLDEN).unwrap(), parts);
    }

    /// Golden vector (full) hand-derived from Figure 6.4.3.5-1 / Table 6.4.3.5-1.
    ///
    /// MAX_ADDITIONAL_PHY = 7 additional 5-octet blocks.  7 + 7*5 = 42 bytes total.
    ///
    /// Each additional block: mu=M2(code 1) | beta=B16(code 5) | same phy as base.
    ///   octet 0: (mu_code << 5) | (beta_code << 1) = (1 << 5) | (5 << 1) = 0x20 | 0x0A = 0x2A
    ///   octet 1: 0x24  (same as base_phy byte 3)
    ///   octet 2: 0x67  (same as base_phy byte 4)
    ///   octet 3: 0x6C  (same as base_phy byte 5)
    ///   octet 4: harq_feedback_delay only (no d_delay/half_dup in additional block):
    ///            (6 << 4) = 0x60
    ///
    /// Fixed header byte 0: N=7 (bits7-5=0b111), release=R3 -> (7<<5)|3 = 0xE3
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows RD Capability IE field layout"
    )]
    fn golden_vector_full() {
        // Additional block repeated 7 times (MAX_ADDITIONAL_PHY).
        // mu=M2 code=1, beta=B16 code=5.
        const ADD: [u8; 5] = [
            0b001_0101_0,  // mu=M2(code 1)|beta=B16(code 5)|R=0 = 0x2A
            0b0_010_01_00, // R|ClassIII(010)|N2(01)|N1(00) = 0x24
            0b0110_0111,   // rx_gain=Db2(6)|max_mcs=7 = 0x67
            0b0110_11_00,  // soft_buf=512000(6)|P8(11)|R(2b) = 0x6C
            0b0110_0000,   // harq_fb_delay=6 (no d_delay/half_dup in additional block) = 0x60
        ];
        const GOLDEN: [u8; 42] = [
            0b111_00011,      // N=7 | release=R3 (code 3) -> 0xE3
            0b000_0_1_01_0_0, // same byte1 as minimal = 0x14
            0b001_001_00,     // same byte2 as minimal = 0x24
            0b0_010_01_00,    // same byte3
            0b0110_0111,      // same byte4
            0b0110_11_00,     // same byte5
            0b0110_1_0_00,    // same byte6 = 0x68
            // 7 additional blocks:
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 0
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 1
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 2
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 3
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 4
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 5
            ADD[0],
            ADD[1],
            ADD[2],
            ADD[3],
            ADD[4], // block 6
        ];
        let one_block = AdditionalPhyCapability {
            mu: RdClassMu::M2,      // code 1 (Table 6.4.3.5-1)
            beta: RdClassBeta::B16, // code 5 (Table 6.4.3.5-1)
            phy: golden_phy(),
        };
        let mut additional_phy = Vec::new();
        for _ in 0..MAX_ADDITIONAL_PHY {
            additional_phy.push(one_block).unwrap();
        }
        let parts = RdCapabilityParts {
            release: Release::R3,
            group_as: false,
            paging: true,
            operating_modes: OperatingModes::FtOnly,
            mesh: false,
            schedul: false,
            mac_security: MacSecuritySupport::Mode1Supported,
            dlc_service_type: DlcServiceType::Type1,
            base_phy: golden_phy(),
            d_delay: true,
            half_dup: false,
            additional_phy,
        };
        let mut buf = [0u8; 64];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(RdCapabilityParts::parse(&GOLDEN).unwrap(), parts);
    }

    fn sample_phy() -> PhyCapability {
        PhyCapability {
            rd_power_class: RdPowerClass::ClassII,
            max_nss_for_rx: Nss::N1,
            rx_for_tx_diversity: Nss::N1,
            rx_gain: RxGain::Db0,
            max_mcs: Mcs::try_from_u8(4).unwrap(),
            soft_buffer_size: SoftBufferSize::Bytes64000,
            num_harq_processes: NumHarqProcesses::P4,
            harq_feedback_delay: HarqFeedbackDelay::try_from_u8(2).unwrap(),
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
                max_mcs: Mcs::try_from_u8(9).unwrap(),
                soft_buffer_size: SoftBufferSize::Bytes256000,
                num_harq_processes: NumHarqProcesses::P8,
                harq_feedback_delay: HarqFeedbackDelay::try_from_u8(5).unwrap(),
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
        let mut buf = [0; 64];
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
        let mut buf = [0; 64];
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
        let mut buf = [0; 64];
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
        let buf = [0; 6];
        assert!(RdCapabilityParts::parse(&buf).is_err());
    }

    #[test]
    fn rd_capability_parser_rejects_truncated_additional_block() {
        // count=1 but only the fixed part present.
        let mut buf = [0; 64];
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
        let buf = [0; 7];
        assert!(RdCapabilityParts::parse(&buf).is_err());
    }

    #[test]
    fn rd_capability_parser_rejects_reserved_operating_modes() {
        let buf = [0x01, 0b0000_1100, 0, 0, 0, 0, 0];
        assert!(RdCapabilityParts::parse(&buf).is_err());
    }
}
