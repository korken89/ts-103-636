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
//! Additional harnesses cover the PCC formats (same round-trip
//! property) and the secured-PDU framing of `Message::parse` under a
//! no-op crypto backend - the AES primitives themselves are out of
//! scope for model checking and stay with the fuzz targets and the
//! RustCrypto test suites.
//!
//! Run with Kani (`make verify`, or `cd verify && cargo kani`).

use ts_103_636::mac::messages::*;
use ts_103_636::mac::pdu::Message;
use ts_103_636::pcc::Pcc;
use ts_103_636::security::{KEY_LEN, MacCrypto, SecurityContext, cipher_range_for_verification};
use ts_103_636::types::{LongRdId, Mu};

/// Prove, for every buffer up to `$cap` bytes (all byte values, all
/// lengths, all valid mu where the codec takes one):
///
/// - `parse` cannot panic;
/// - every successfully parsed value serializes without panic and
///   without `SerializationError`, to exactly its stated
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

/// PCC: every input up to 10 bytes - the 5-byte Type 1 dispatch, the
/// 10-byte Type 2 F000/F001 dispatch, and every rejected length. A
/// successful parse re-serializes (without error) to bytes that parse
/// back to the identical value.
#[kani::proof]
fn pcc_codec_safe() {
    let buf: [u8; 10] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    if let Ok(pcc) = Pcc::parse(&buf[..len]) {
        let bytes = pcc.to_bytes().unwrap();
        assert_eq!(Pcc::parse(bytes.as_slice()).unwrap(), pcc);
    }
}

/// No-op crypto backend: lets Kani prove the secured-PDU framing
/// (header validation, cipher-range computation, MIC split and
/// compare) without bit-blasting AES. The MIC compare still runs -
/// the tag is all-zero, so the solver reaches both the BadMic reject
/// path and, for buffers whose trailer is zero, the verified-accept
/// path.
struct NoOpCrypto;

impl MacCrypto for NoOpCrypto {
    type Error = core::convert::Infallible;

    fn cmac(&mut self, _: &[u8; KEY_LEN], _: &[u8]) -> Result<[u8; 16], Self::Error> {
        Ok([0; 16])
    }

    fn ctr_apply(
        &mut self,
        _: &[u8; KEY_LEN],
        _: &[u8; 16],
        _: &mut [u8],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Cap for the PDU-framing harnesses: covers the largest common
/// header (10-byte Unicast), the 7-byte MAC Security Info IE, payload
/// IEs, and the 5-byte MIC trailer. 48 bytes proved too slow for CI
/// (>6 min solve); 32 keeps every framing branch reachable.
const PDU_CAP: usize = 32;

/// `Message::parse_unverified` never panics on any input up to
/// [`PDU_CAP`] bytes.
#[kani::proof]
fn parse_unverified_safe() {
    let buf: [u8; PDU_CAP] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    let _ = Message::parse_unverified(&buf[..len]);
}

/// Cap for the isolated cipher-range walk: the plaintext IE prefix
/// it scans holds the 7-byte MAC Security Info IE plus a couple of
/// leading IEs at most. 32 bytes solved in ~6 min; 16 keeps the
/// proof in CI budget.
const CIPHER_RANGE_CAP: usize = 16;

/// The UsedWithIe cipher-range walk in isolation: never panics, and
/// any computed range stays within the buffer (which is what makes
/// the in-place decrypt slice in `Message::parse` safe). The unwind
/// cap covers the 1-byte minimum IE advance over
/// [`CIPHER_RANGE_CAP`] bytes.
#[kani::proof]
#[kani::unwind(18)]
fn cipher_range_safe() {
    let buf: [u8; CIPHER_RANGE_CAP] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    if let Ok(Some(range)) = cipher_range_for_verification(&buf[..len]) {
        assert!(range.start <= range.end);
        assert!(range.end <= len);
    }
}

/// `Message::parse` (decrypt + verify + parse) never panics on any
/// input up to [`PDU_CAP`] bytes whose header does not select
/// UsedWithIe, under the no-op backend. The excluded UsedWithIe arm
/// only adds the IE-prefix walk proven by [`cipher_range_safe`]; the
/// monolithic proof over all headers lives behind the `slow-proofs`
/// feature.
///
/// The unwind bound is harness-global: it must cover the fixed-size
/// loops on this path (the 5-byte constant-time MIC compare, 16-byte
/// IV handling), while the data-dependent IE walk - which would need
/// a bound without the assume, since CBMC unwinds syntactically
/// before path conditions apply - is unreachable and its unwindings
/// prune immediately.
#[kani::proof]
#[kani::unwind(18)]
fn message_parse_loopfree_safe() {
    let mut buf: [u8; PDU_CAP] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    // MAC Security bits (byte 0, bits 5..4) != 0b10 (UsedWithIe).
    kani::assume(buf.is_empty() || (buf[0] >> 4) & 0b11 != 0b10);
    let ctx = SecurityContext {
        tx: LongRdId::new(0xAABB_CCDD).unwrap(),
        rx: LongRdId::new(0x1122_3344).unwrap(),
        hpc: kani::any(),
    };
    let _ = Message::parse(
        &mut buf[..len],
        &mut NoOpCrypto,
        &[0; KEY_LEN],
        &[0; KEY_LEN],
        &ctx,
    );
}

/// Monolithic framing proof over EVERY header form, including the
/// UsedWithIe IE-prefix walk. Proven SUCCESSFUL (583 checks) in
/// ~507s of solve time - too slow for CI, so it is opt-in:
///
/// ```text
/// cd verify && cargo kani --features slow-proofs \
///     --harness slow_message_parse_framing
/// ```
#[cfg(feature = "slow-proofs")]
#[kani::proof]
#[kani::unwind(34)]
fn slow_message_parse_framing() {
    let mut buf: [u8; PDU_CAP] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    let ctx = SecurityContext {
        tx: LongRdId::new(0xAABB_CCDD).unwrap(),
        rx: LongRdId::new(0x1122_3344).unwrap(),
        hpc: kani::any(),
    };
    let _ = Message::parse(
        &mut buf[..len],
        &mut NoOpCrypto,
        &[0; KEY_LEN],
        &[0; KEY_LEN],
        &ctx,
    );
}

/// Cap for the IE-stream and security-info walks: a header plus
/// several minimal IEs. Like [`CIPHER_RANGE_CAP`], 16 bytes keeps
/// the 1-byte-minimum-advance walk inside CI budget.
const IE_STREAM_CAP: usize = 16;

/// The full receive walk: split any input and iterate every IE in
/// the tail (mirrors the pdu_parse fuzz target). The walker
/// terminates without panicking on any malformed tail.
#[kani::proof]
#[kani::unwind(18)]
fn ie_stream_walk_safe() {
    let buf: [u8; IE_STREAM_CAP] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    if let Ok(msg) = Message::parse_unverified(&buf[..len]) {
        for ie in msg.tail_items() {
            if ie.is_err() {
                break;
            }
        }
    }
}

/// `Message::peek_security_info` (the HPC resynchronization path)
/// never panics on any input: it walks the plaintext IE prefix much
/// like the cipher-range computation.
#[kani::proof]
#[kani::unwind(18)]
fn peek_security_info_safe() {
    let buf: [u8; IE_STREAM_CAP] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= buf.len());
    let _ = Message::peek_security_info(&buf[..len]);
}
