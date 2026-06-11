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
}
