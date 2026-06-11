//! Association Control IE body (generated codec re-export).
//!
//! The codec lives in [`generated::association_control`](super::generated::association_control); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::association_control::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    #[test]
    fn association_control_round_trip() {
        let parts = AssociationControlParts {
            cb_m: true,
            dl_data_reception: DlDataReception::Ms40,
            ul_period: UlPeriod::H6,
        };
        let mut buf = [0; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = AssociationControlParts::parse(&buf).unwrap();
        assert!(parsed.cb_m);
        assert_eq!(parsed.dl_data_reception, DlDataReception::Ms40);
        assert_eq!(parsed.ul_period, UlPeriod::H6);
    }

    #[test]
    fn association_control_rejects_reserved_dl_reception() {
        // DLdataReception = 6 (reserved).
        // B0 = 0 110 0000 = 0x60
        let buf = [0x60u8; 1];
        assert!(AssociationControlParts::parse(&buf).is_err());
    }

    /// Golden vector hand-derived from Figure 6.4.3.18-1 and Table 6.4.3.18-1.
    ///
    /// Chosen values:
    ///   CB_M = 1 (does not maintain Cluster Beacon reception)
    ///   DLdataReception = Ms20 (code 3 = 0b011, Table 6.4.3.18-1)
    ///   ULPeriod = Min30 (code 6 = 0b0110, Table 6.4.3.18-1)
    ///
    /// Byte 0 = [CB_M(1) | DLdata(3) | ULPeriod(4)]
    ///        = [1 | 011 | 0110]
    ///        = 0b1011_0110 = 0xB6
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows field layout"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 1] = [
            0b1_011_0110, // CB_M=1 | DLdataReception=Ms20(3) | ULPeriod=Min30(6)
        ];
        let parts = AssociationControlParts {
            cb_m: true,
            dl_data_reception: DlDataReception::Ms20,
            ul_period: UlPeriod::Min30,
        };
        let mut buf = [0u8; 1];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, GOLDEN.len());
        assert_eq!(buf[..n], GOLDEN);
        assert_eq!(AssociationControlParts::parse(&GOLDEN).unwrap(), parts);
    }
}
