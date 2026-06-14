//! FT-side association responder, in-memory round-trip.
//!
//! Simulates a PT sending an Association Request to an FT, the FT
//! parsing it, deciding accept/reject, and replying with an Association
//! Response. No hardware required; everything happens in local buffers
//! so you can `cargo run --example association_responder` to exercise
//! the build + parse paths against each other.
//!
//! The goal of this example is to be the thing a new user reads first
//! to learn how to use the crate from a MAC implementation. If parts of
//! it feel awkward, those are exactly the bits of API we should
//! sharpen.

use heapless::Vec;
use ts_103_636::prelude::*;

/// Long RD IDs of the two endpoints in this example.
const PT_ID: u32 = 0x1111_AAAA;
const FT_ID: u32 = 0x2222_BBBB;

/// PT side: build an Association Request PDU into `buf` and return the
/// slice that was written.
fn pt_build_association_request(buf: &mut [u8]) -> &[u8] {
    let pt = LongRdId::try_from_u32(PT_ID).unwrap();
    let ft = LongRdId::try_from_u32(FT_ID).unwrap();
    let psn = SequenceNumber::try_from_u16(1).unwrap();

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
        .push_unicast(NotUsed, false, psn, ft, pt)
        .expect("buffer fits header")
        .push_body(&body)
        .expect("buffer fits body")
        .finish_without_security()
}

/// FT side: parse `received` and decide how to respond. Build the
/// response PDU into `response_buf` and return the slice that was
/// written.
fn ft_receive_and_respond<'a>(received: &[u8], response_buf: &'a mut [u8]) -> &'a [u8] {
    let msg = Message::parse_unverified(received).expect("well-formed PDU");

    let (their_id, _our_id, their_psn) = match msg.common {
        MacCommonHeader::Unicast(u) => (
            u.transmitter().expect("non-reserved transmitter"),
            u.receiver().expect("non-reserved receiver"),
            u.sequence_number(),
        ),
        _ => panic!("expected a Unicast PDU"),
    };

    let mut request: Option<AssociationRequestParts> = None;
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(ie.ie_number(), AnyIeType::Type6bit(t) if t == IEType6bit::AssociationRequest) {
            request = Some(
                AssociationRequestParts::parse(ie.payload())
                    .expect("well-formed Association Request body"),
            );
        }
    }
    let request = request.expect("PDU contains an Association Request");

    println!(
        "FT received Association Request from {:08x}, psn={}, setup_cause={:?}",
        u32::from(their_id),
        their_psn.as_u16(),
        request.setup_cause,
    );

    let response_body = AssociationResponseParts::Accept(AssociationAcceptParts {
        flow_acceptance: FlowAcceptance::All,
        harq_override: None,
        group: None,
    });

    let pt = LongRdId::try_from_u32(PT_ID).unwrap();
    let ft = LongRdId::try_from_u32(FT_ID).unwrap();
    let response_psn = SequenceNumber::try_from_u16(1).unwrap();

    MacPduBuilder::new(response_buf)
        .push_unicast(NotUsed, false, response_psn, pt, ft)
        .expect("buffer fits header")
        .push_body(&response_body)
        .expect("buffer fits body")
        .finish_without_security()
}

fn main() {
    let mut request_buf = [0; 256];
    let request_bytes = pt_build_association_request(&mut request_buf);
    println!(
        "PT -> FT: {} bytes of Association Request",
        request_bytes.len()
    );

    let mut response_buf = [0; 256];
    let response_bytes = ft_receive_and_respond(request_bytes, &mut response_buf);
    println!(
        "FT -> PT: {} bytes of Association Response",
        response_bytes.len()
    );

    let response_msg = Message::parse_unverified(response_bytes).expect("well-formed response PDU");
    let mut found_accept = false;
    for ie in response_msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(ie.ie_number(), AnyIeType::Type6bit(t) if t == IEType6bit::AssociationResponse)
        {
            let body = AssociationResponseParts::parse(ie.payload())
                .expect("well-formed Association Response body");
            match body {
                AssociationResponseParts::Accept(_) => {
                    found_accept = true;
                    println!("PT: Association accepted by FT");
                }
                AssociationResponseParts::Reject { cause, timer } => {
                    panic!("expected accept; got reject: cause={cause:?}, timer={timer:?}");
                }
            }
        }
    }
    assert!(found_accept);
}
