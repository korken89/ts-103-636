//! Radio Device Status IE body (generated codec re-export).
//!
//! The codec lives in [`generated::radio_device_status`](super::generated::radio_device_status); the layout
//! figure is in that module's documentation. The tests below are the
//! drop-in equivalence oracle and predate the generated codec.

pub use super::generated::radio_device_status::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    /// Golden vector hand-derived from Figure 6.4.3.13-1 / Table 6.4.3.13-1.
    ///
    /// Byte layout (8 bits):
    ///   bit 7: reserved = 0
    ///   bit 6: Association = 1  (re-association needed)
    ///   bits 5-4: Status = 0b10 (NormalOperation, Table 6.4.3.13-1)
    ///   bits 3-0: Duration = 0b0110 = 6 (Ms1000, Table 6.4.3.13-1)
    ///
    ///   byte0 = 0_1_10_0110 = 0x66
    #[test]
    #[allow(
        clippy::unusual_byte_groupings,
        reason = "binary grouping shows Radio Device Status field layout"
    )]
    fn golden_vector() {
        const GOLDEN: [u8; 1] = [
            0b0_1_10_0110, // reserved(0) | assoc=1 | status=NormalOperation | duration=Ms1000
        ];
        let parts = RadioDeviceStatusParts {
            association_needed: true,
            status: RadioDeviceStatusFlag::NormalOperation, // code 0b10 (Table 6.4.3.13-1)
            duration: RadioDeviceStatusDuration::Ms1000,    // code 6 (Table 6.4.3.13-1)
        };
        let mut buf = [0u8; 4];
        assert_eq!(parts.serialize(&mut buf).unwrap(), GOLDEN.len());
        assert_eq!(buf[..GOLDEN.len()], GOLDEN);
        assert_eq!(RadioDeviceStatusParts::parse(&GOLDEN).unwrap(), parts);
    }

    #[test]
    fn radio_device_status_round_trip() {
        let parts = RadioDeviceStatusParts {
            association_needed: true,
            status: RadioDeviceStatusFlag::MemoryFull,
            duration: RadioDeviceStatusDuration::Ms400,
        };
        let mut buf = [0; 4];
        let n = parts.serialize(&mut buf).unwrap();
        assert_eq!(n, 1);
        let parsed = RadioDeviceStatusParts::parse(&buf).unwrap();
        assert!(parsed.association_needed);
        assert_eq!(parsed.status, RadioDeviceStatusFlag::MemoryFull);
        assert_eq!(parsed.duration, RadioDeviceStatusDuration::Ms400);
    }

    #[test]
    fn radio_device_status_rejects_reserved_status_flag() {
        // Status flag = 0b00 (reserved).
        let buf = [0; 1];
        assert!(RadioDeviceStatusParts::parse(&buf).is_err());
    }
}
