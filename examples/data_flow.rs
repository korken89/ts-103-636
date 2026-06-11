//! User-plane data round-trip with custom application payloads.
//!
//! The other three examples build *typed* MAC messages (Cluster Beacon,
//! Association Request/Response, Joining Beacon) using
//! [`MacPduBuilder::push_body`]. This example shows the other half of
//! the picture: how to carry arbitrary application bytes inside a PDU
//! using the User Plane Data and Higher Layer Signalling IE types,
//! which have no fixed structure on the wire and so go through
//! [`MacPduBuilder::push_ie`] instead.
//!
//! The scenario: a sensor PT periodically reports a `(channel, sample)`
//! pair to its FT; the FT acknowledges each report with an
//! application-level sequence number. Both directions use plain
//! Unicast PDUs without MAC security; production code would build the
//! same IEs and call `finish_with_security` instead.
//!
//! Run with `cargo run --example data_flow`.

use ts_103_636::prelude::*;

const PT_ID: u32 = 0x1111_AAAA;
const FT_ID: u32 = 0x2222_BBBB;

/// Application-level packet that the PT carries inside a
/// User Plane Data Flow 1 IE. Wire format is whatever the application
/// agrees on; here we use a compact 3-byte layout.
struct SensorSample {
    /// 8-bit sensor channel identifier.
    channel: u8,
    /// 16-bit big-endian sample value (e.g., raw ADC reading).
    sample: u16,
}

impl SensorSample {
    const ENCODED_LEN: usize = 3;

    fn encode(&self, out: &mut [u8; Self::ENCODED_LEN]) {
        out[0] = self.channel;
        out[1..3].copy_from_slice(&self.sample.to_be_bytes());
    }

    fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < Self::ENCODED_LEN {
            return None;
        }
        Some(Self {
            channel: buf[0],
            sample: u16::from_be_bytes([buf[1], buf[2]]),
        })
    }
}

/// PT side: build a Unicast PDU containing a single User Plane Data
/// Flow 1 IE carrying one [`SensorSample`].
fn pt_build_data<'a>(buf: &'a mut [u8], psn: SequenceNumber, sample: &SensorSample) -> &'a [u8] {
    let pt = LongRdId::new(PT_ID).unwrap();
    let ft = LongRdId::new(FT_ID).unwrap();

    // Encode the application payload into a small stack buffer.
    let mut payload = [0; SensorSample::ENCODED_LEN];
    sample.encode(&mut payload);

    // Wrap the opaque bytes in an IE. There is no typed Parts struct
    // for User Plane Data Flow 1 because the on-wire format is
    // application-defined; push_ie takes care of writing the IE header
    // and length, then copying the payload through verbatim.
    let ie = InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, &payload)
        .expect("payload fits in IE length field");

    MacPduBuilder::new(buf)
        .push_unicast(NotUsed, false, psn, ft, pt)
        .expect("buffer fits header")
        .push_ie(&ie)
        .expect("buffer fits IE")
        .finish_without_security()
}

/// FT side: parse the PT's PDU, hand each User Plane Data Flow IE to
/// the application, and build an ack PDU carrying the application
/// sequence number on Higher Layer Signalling Flow 1.
fn ft_receive_and_ack<'a>(
    received: &[u8],
    response_buf: &'a mut [u8],
    ack_psn: SequenceNumber,
    app_ack_seq: u16,
) -> &'a [u8] {
    let msg = Message::parse_unverified(received).expect("well-formed PDU");

    let pt_id = match msg.common {
        MacCommonHeader::Unicast(u) => u.transmitter().expect("non-reserved transmitter"),
        _ => panic!("expected Unicast PDU"),
    };

    // Walk the IE stream. For each IE we see, decide if our application
    // layer wants it. Here we only care about User Plane Data Flow 1;
    // anything else we'd typically log and ignore (or forward).
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        match ie.ie_number() {
            AnyIeType::Type6bit(IEType6bit::UserPlaneDataFlow1) => {
                let sample = SensorSample::decode(ie.payload()).expect("3-byte payload");
                println!(
                    "FT: sample from pt=0x{:08x} on channel {} = {:#06x}",
                    u32::from(pt_id),
                    sample.channel,
                    sample.sample,
                );
            }
            other => {
                println!("FT: ignoring IE {other:?}");
            }
        }
    }

    // Build the ack: a 2-byte big-endian application sequence number
    // carried in a Higher Layer Signalling Flow 1 IE.
    let ack_bytes = app_ack_seq.to_be_bytes();
    let ack_ie = InformationElement::new_6bit_with_length(
        IEType6bit::HigherLayerSignallingFlow1,
        &ack_bytes,
    )
    .expect("payload fits in IE length field");

    let pt = LongRdId::new(PT_ID).unwrap();
    let ft = LongRdId::new(FT_ID).unwrap();
    MacPduBuilder::new(response_buf)
        .push_unicast(NotUsed, false, ack_psn, pt, ft)
        .expect("buffer fits header")
        .push_ie(&ack_ie)
        .expect("buffer fits IE")
        .finish_without_security()
}

/// PT side: parse the ack and pull the application sequence number out.
fn pt_observe_ack(received: &[u8]) -> u16 {
    let msg = Message::parse_unverified(received).expect("well-formed ack PDU");
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(
            ie.ie_number(),
            AnyIeType::Type6bit(t) if t == IEType6bit::HigherLayerSignallingFlow1,
        ) {
            let payload = ie.payload();
            assert_eq!(payload.len(), 2, "app ack is 2 bytes");
            return u16::from_be_bytes([payload[0], payload[1]]);
        }
    }
    panic!("ack PDU did not contain a Higher Layer Signalling Flow 1 IE");
}

fn main() {
    // Simulate four sensor samples being reported and acknowledged.
    let samples = [
        SensorSample {
            channel: 1,
            sample: 0x0123,
        },
        SensorSample {
            channel: 1,
            sample: 0x0125,
        },
        SensorSample {
            channel: 2,
            sample: 0xCAFE,
        },
        SensorSample {
            channel: 1,
            sample: 0x0127,
        },
    ];

    for (i, sample) in samples.iter().enumerate() {
        // PT -> FT
        let psn = SequenceNumber::new((i + 1) as u16).unwrap();
        let mut req_buf = [0; 64];
        let req_bytes = pt_build_data(&mut req_buf, psn, sample);
        println!(
            "PT -> FT: {} bytes carrying ch={} sample={:#06x}",
            req_bytes.len(),
            sample.channel,
            sample.sample,
        );

        // FT -> PT
        let mut ack_buf = [0; 64];
        let app_ack_seq = (i + 1) as u16;
        let ack_bytes = ft_receive_and_ack(req_bytes, &mut ack_buf, psn, app_ack_seq);
        println!(
            "FT -> PT: {} bytes carrying ack seq={}",
            ack_bytes.len(),
            app_ack_seq
        );

        // PT verifies the ack
        let observed_seq = pt_observe_ack(ack_bytes);
        assert_eq!(observed_seq, app_ack_seq);
        println!("PT: ack matches (seq={observed_seq})\n");
    }
}
