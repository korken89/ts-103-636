//! Symbolic verification harnesses for the ts-103-636 codecs.
//!
//! One `*_codec_safe` proof per generated message: parsing never
//! panics on ANY input - every byte value, every length up to the
//! stated cap, every valid mu where the codec takes one - and every
//! parsed value serializes without panic to exactly its
//! `encoded_len`, with re-parsing yielding the identical value
//! (parse-serialize-parse identity). `make verify` runs them with
//! Kani, one harness per core.
//!
//! Run with Kani (`make verify`, or `cd verify && cargo kani`).
//! The harnesses also work under Soteria Rust's Kani mode
//! (`soteria-rust exec --kani verify/`), but as of soteria 0.1.0 its
//! frontend rejects the `Option<NonZero<_>>`-niche layouts our
//! identifier types use, and its path enumeration blows up on the
//! larger parsers.

use ts_103_636::mac::messages::*;
use ts_103_636::types::Mu;

/// Prove, for every buffer up to `$cap` bytes (all byte values, all
/// lengths, all valid mu where the codec takes one):
///
/// - `parse` cannot panic;
/// - every successfully parsed value serializes without panic and
///   without `ExcessiveBitsSet`, to exactly its stated
///   `encoded_len`;
/// - re-parsing the serialized bytes yields the identical value
///   (parse-serialize-parse identity).
///
/// (Serialization of values built directly by the user rather than
/// by `parse` is NOT covered here.)
///
/// The cap is chosen at or above the message's maximum encoded
/// length; for messages with repeats or zero-copy tails it bounds how
/// many elements the proof covers.
macro_rules! codec_safe {
    ($(#[$attr:meta])* $name:ident, $ty:ty, $cap:expr) => {
        $(#[$attr])*
        #[kani::proof]
        fn $name() {
            let buf: [u8; $cap] = kani::any();
            let len: usize = kani::any();
            kani::assume(len <= buf.len());
            if let Ok(parts) = <$ty>::parse(&buf[..len]) {
                let mut out = [0; $cap];
                let n = parts.serialize(&mut out).unwrap();
                assert_eq!(parts.encoded_len(), n);
                assert_eq!(<$ty>::parse(&out[..n]).unwrap(), parts);
            }
        }
    };
    ($(#[$attr:meta])* $name:ident, $ty:ty, $cap:expr, mu) => {
        $(#[$attr])*
        #[kani::proof]
        fn $name() {
            let buf: [u8; $cap] = kani::any();
            let len: usize = kani::any();
            kani::assume(len <= buf.len());
            let mu_raw: u8 = kani::any();
            let Some(mu) = Mu::new(mu_raw) else { return };
            if let Ok(parts) = <$ty>::parse(&buf[..len], mu) {
                let mut out = [0; $cap];
                let n = parts.serialize(&mut out).unwrap();
                assert_eq!(parts.encoded_len(), n);
                assert_eq!(<$ty>::parse(&out[..n], mu).unwrap(), parts);
            }
        }
    };
}

codec_safe!(association_control_codec_safe, AssociationControlParts, 8);
codec_safe!(association_release_codec_safe, AssociationReleaseParts, 8);
codec_safe!(
    // MAX_REQUEST_FLOWS = 6 element loops (parse and PartialEq).
    #[kani::unwind(8)]
    association_request_codec_safe,
    AssociationRequestParts,
    32
);
codec_safe!(
    // MAX_RESPONSE_FLOWS = 6 element loop in serialize; cap unwinding.
    #[kani::unwind(8)]
    association_response_codec_safe,
    AssociationResponseParts,
    32
);
codec_safe!(broadcast_indication_codec_safe, BroadcastIndicationParts, 16);
codec_safe!(cluster_beacon_codec_safe, ClusterBeaconParts, 18, mu);
codec_safe!(
    // Zero-copy tail: serialize and PartialEq walk up to the 24-byte cap.
    #[kani::unwind(26)]
    group_assignment_codec_safe,
    GroupAssignmentParts,
    24
);
codec_safe!(
    // MAX_ENDPOINTS = 4 element loop; cap unwinding past it.
    #[kani::unwind(6)]
    joining_beacon_codec_safe,
    JoiningBeaconParts,
    16
);
codec_safe!(
    // MAX_ENDPOINTS = 4 element loops (parse and PartialEq).
    #[kani::unwind(6)]
    joining_information_codec_safe,
    JoiningInformationParts,
    16
);
codec_safe!(load_info_codec_safe, LoadInfoParts, 24);
codec_safe!(mac_security_info_codec_safe, MacSecurityInfoParts, 8);
codec_safe!(measurement_report_codec_safe, MeasurementReportParts, 12);
codec_safe!(neighbouring_codec_safe, NeighbouringParts, 16);
codec_safe!(
    // MAX_ADDITIONAL_CHANNELS = 3 element loop; cap unwinding past it.
    #[kani::unwind(5)]
    network_beacon_codec_safe,
    NetworkBeaconParts,
    20
);
codec_safe!(radio_device_status_codec_safe, RadioDeviceStatusParts, 8);
codec_safe!(
    random_access_resource_codec_safe,
    RandomAccessResourceParts,
    24,
    mu
);
codec_safe!(
    // MAX_ADDITIONAL_PHY = 7 element loop; cap unwinding past it.
    #[kani::unwind(9)]
    rd_capability_codec_safe,
    RdCapabilityParts,
    32
);
codec_safe!(rd_capability_short_codec_safe, RdCapabilityShortParts, 4);
codec_safe!(
    // Zero-copy tail: serialize copies up to the full 24-byte cap.
    #[kani::unwind(26)]
    reconfiguration_request_codec_safe,
    ReconfigurationRequestParts,
    24
);
codec_safe!(
    // Zero-copy tail: serialize copies up to the full 24-byte cap.
    #[kani::unwind(26)]
    reconfiguration_response_codec_safe,
    ReconfigurationResponseParts,
    24
);
codec_safe!(
    resource_allocation_codec_safe,
    ResourceAllocationParts,
    32,
    mu
);
codec_safe!(route_info_codec_safe, RouteInfoParts, 8);
codec_safe!(source_routing_codec_safe, SourceRoutingParts, 32);
