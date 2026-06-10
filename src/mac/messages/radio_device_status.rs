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
    #[test]
    fn radio_device_status_round_trip() {
        let parts = RadioDeviceStatusParts {
            association_needed: true,
            status: RadioDeviceStatusFlag::MemoryFull,
            duration: RadioDeviceStatusDuration::Ms400,
        };
        let mut buf = [0u8; 4];
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
        let buf = [0u8; 1];
        assert!(RadioDeviceStatusParts::parse(&buf).is_err());
    }
}
