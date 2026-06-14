//! PT-side association initiator, in-memory round-trip.
//!
//! Simulates the PT side of a fresh association sequence:
//!   1. The PT listens for a Joining Beacon (FT advertises which
//!      Network Beacon channels it cares about).
//!   2. The PT picks the FT it heard from and sends an Association
//!      Request on a Unicast PDU.
//!   3. The FT replies with an Association Response (accept).
//!   4. The PT confirms the accept body and is now associated.
//!
//! Together with `beacon_listener` (Cluster Beacon) and
//! `association_responder` (FT-side Request -> Response), this rounds
//! out the basic association handshake from both sides.

use heapless::Vec;
use ts_103_636::prelude::*;

const PT_ID: u32 = 0x1111_AAAA;
const FT_ID: u32 = 0x2222_BBBB;
const NETWORK_ID: u32 = 0x12_3456;

/// FT side: advertise a Joining Beacon listing the channels it serves.
fn ft_build_joining_beacon(buf: &mut [u8]) -> &[u8] {
    let net = NetworkId24::try_from_u32(NETWORK_ID).unwrap();
    let ft = LongRdId::try_from_u32(FT_ID).unwrap();

    let channels = Vec::from_slice(&[
        AbsoluteChannel::try_from_u16(0x01A4).unwrap(),
        AbsoluteChannel::try_from_u16(0x01A8).unwrap(),
    ])
    .unwrap();

    let body = JoiningBeaconParts {
        network_beacon_period: NetworkBeaconPeriod::Ms1000,
        channels,
    };

    MacPduBuilder::new(buf)
        .push_beacon(NotUsed, net, ft)
        .expect("buffer fits header")
        .push_body(&body)
        .expect("buffer fits body")
        .finish_without_security()
}

/// State the PT carries between hearing a beacon and sending its
/// request. In a real implementation this would also include the
/// channel and any RACH timing extracted from a Resource Allocation IE.
#[derive(Debug)]
struct DiscoveredCluster {
    network_id: NetworkId24,
    ft_id: LongRdId,
}

/// PT side: parse the Joining Beacon and remember which FT to talk to.
fn pt_listen_for_joining(received: &[u8]) -> DiscoveredCluster {
    let msg = Message::parse_unverified(received).expect("well-formed PDU");

    let (net_id, transmitter) = match msg.common {
        MacCommonHeader::Beacon(b) => (
            b.network_id_typed().expect("non-reserved network ID"),
            b.transmitter().expect("non-reserved transmitter"),
        ),
        _ => panic!("expected Beacon PDU"),
    };

    let mut joining: Option<JoiningBeaconParts> = None;
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(ie.ie_number(), AnyIeType::Type6bit(t) if t == IEType6bit::JoiningBeacon) {
            joining = Some(
                JoiningBeaconParts::parse(ie.payload()).expect("well-formed Joining Beacon body"),
            );
        }
    }
    let joining = joining.expect("PDU contains a Joining Beacon body");

    println!(
        "PT: heard Joining Beacon: network=0x{:06x} ft=0x{:08x} period={}ms channels={}",
        u32::from(net_id),
        u32::from(transmitter),
        joining.network_beacon_period.milliseconds(),
        joining.channels.len(),
    );
    for (i, ch) in joining.channels.iter().enumerate() {
        println!("PT:   channel[{i}] = 0x{:04x}", ch.as_u16());
    }

    DiscoveredCluster {
        network_id: net_id,
        ft_id: transmitter,
    }
}

/// PT side: build an Association Request targeted at the discovered FT.
fn pt_build_association_request<'a>(buf: &'a mut [u8], cluster: &DiscoveredCluster) -> &'a [u8] {
    let pt = LongRdId::try_from_u32(PT_ID).unwrap();
    let psn = SequenceNumber::try_from_u16(1).unwrap();
    let _ = cluster.network_id; // PT would also store this for the secured PDU path.

    let body = AssociationRequestParts {
        setup_cause: SetupCause::InitialAssociation,
        power_const: PowerConst::Unconstrained,
        flow_ids: Vec::new(),
        harq_processes_tx: HarqProcesses::try_from_u8(2).unwrap(),
        max_harq_re_tx: MaxHarqReTx::try_from_u8(5).unwrap(),
        harq_processes_rx: HarqProcesses::try_from_u8(2).unwrap(),
        max_harq_re_rx: MaxHarqReTx::try_from_u8(5).unwrap(),
        ft_mode: None,
    };

    MacPduBuilder::new(buf)
        .push_unicast(NotUsed, false, psn, cluster.ft_id, pt)
        .expect("buffer fits header")
        .push_body(&body)
        .expect("buffer fits body")
        .finish_without_security()
}

/// FT side (stub): accept the request and build a plain Accept response.
fn ft_accept(received: &[u8], response_buf: &mut [u8]) -> usize {
    let msg = Message::parse_unverified(received).expect("well-formed PDU");

    let (their_id, _their_psn) = match msg.common {
        MacCommonHeader::Unicast(u) => (
            u.transmitter().expect("non-reserved transmitter"),
            u.sequence_number(),
        ),
        _ => panic!("expected Unicast PDU"),
    };

    let mut had_request = false;
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(ie.ie_number(), AnyIeType::Type6bit(t) if t == IEType6bit::AssociationRequest) {
            had_request = AssociationRequestParts::parse(ie.payload()).is_ok();
        }
    }
    assert!(had_request);

    let response_body = AssociationResponseParts::Accept(AssociationAcceptParts {
        flow_acceptance: FlowAcceptance::All,
        harq_override: None,
        group: None,
    });

    let ft = LongRdId::try_from_u32(FT_ID).unwrap();
    let response_psn = SequenceNumber::try_from_u16(1).unwrap();
    MacPduBuilder::new(response_buf)
        .push_unicast(NotUsed, false, response_psn, their_id, ft)
        .expect("buffer fits header")
        .push_body(&response_body)
        .expect("buffer fits body")
        .finish_without_security()
        .len()
}

/// PT side: parse the Association Response and confirm we were accepted.
fn pt_confirm_accept(received: &[u8]) {
    let msg = Message::parse_unverified(received).expect("well-formed PDU");
    let mut accepted = false;
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(ie.ie_number(), AnyIeType::Type6bit(t) if t == IEType6bit::AssociationResponse)
        {
            let body = AssociationResponseParts::parse(ie.payload())
                .expect("well-formed Association Response body");
            match body {
                AssociationResponseParts::Accept(_) => {
                    accepted = true;
                    println!("PT: Association accepted");
                }
                AssociationResponseParts::Reject { cause, timer } => {
                    panic!("expected accept; got reject: cause={cause:?}, timer={timer:?}");
                }
            }
        }
    }
    assert!(accepted);
}

fn main() {
    // Step 1: FT advertises a Joining Beacon, PT receives it.
    let mut jb_buf = [0; 64];
    let jb_bytes = ft_build_joining_beacon(&mut jb_buf);
    println!("FT -> air: {} bytes of Joining Beacon", jb_bytes.len());
    let cluster = pt_listen_for_joining(jb_bytes);

    // Step 2: PT sends an Association Request to the discovered FT.
    let mut req_buf = [0; 64];
    let req_bytes = pt_build_association_request(&mut req_buf, &cluster);
    println!("PT -> FT: {} bytes of Association Request", req_bytes.len());

    // Step 3: FT processes the request and sends an Accept.
    let mut resp_buf = [0; 64];
    let resp_len = ft_accept(req_bytes, &mut resp_buf);
    println!("FT -> PT: {} bytes of Association Response", resp_len);

    // Step 4: PT confirms.
    pt_confirm_accept(&resp_buf[..resp_len]);
}
