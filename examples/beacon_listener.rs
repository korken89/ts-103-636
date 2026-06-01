//! Beacon listener, in-memory round-trip.
//!
//! Simulates an FT building and transmitting a Cluster Beacon, and a PT
//! receiving the bytes and extracting the fields it needs to synchronise
//! to the cluster (network ID, transmitter, SFN, channel info).
//!
//! This is the smallest meaningful flow against the crate: build one
//! body, push it as an IE in one call, parse the PDU back, walk the IE
//! stream, react to the Cluster Beacon body. If anything in here feels
//! rough, it will hurt for every flow.
//!
//! Run with `cargo run --example beacon_listener`.

use ts_103_636::prelude::*;

/// FT side: build a Cluster Beacon PDU into `buf` and return the slice
/// that was written.
fn ft_build_cluster_beacon(buf: &mut [u8], sfn: u8) -> &[u8] {
    let net = NetworkId24::new(0x12_3456).unwrap();
    let ft = LongRdId::new(0x2222_BBBB).unwrap();

    let body = ClusterBeaconParts {
        mu: Mu::M1,
        sfn: Sfn(sfn),
        power_const: PowerConst::Unconstrained,
        network_beacon_period: NetworkBeaconPeriod::Ms1000,
        cluster_beacon_period: ClusterBeaconPeriod::Ms100,
        count_to_trigger: CountToTrigger::new(3).unwrap(),
        rel_quality: Quality::new(0).unwrap(),
        min_quality: Quality::new(0).unwrap(),
        cluster_max_tx_power: Some(TransmitPower::Dbm13),
        frame_offset: None,
        next_cluster_channel: Some(AbsoluteChannel::new(0x1A4).unwrap()),
        time_to_next: Some(50_000),
    };

    MacPduBuilder::new(buf)
        .push_beacon(NotUsed, net, ft)
        .expect("buffer fits header")
        .push_body(&body)
        .expect("buffer fits body")
        .finish_without_security()
}

/// PT side: parse the received PDU and react to the Cluster Beacon.
/// Returns the SFN we observed.
fn pt_observe_beacon(received: &[u8]) -> u8 {
    let msg = Message::parse_unverified(received).expect("well-formed PDU");

    let (net_id, transmitter) = match msg.common {
        MacCommonHeader::Beacon(b) => (
            b.network_id_typed().expect("non-reserved network ID"),
            b.transmitter().expect("non-reserved transmitter"),
        ),
        _ => panic!("expected Beacon PDU"),
    };

    let mut observed_sfn: Option<u8> = None;
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if let AnyIeType::Type6bit(t) = ie.ie_number()
            && t == IEType6bit::ClusterBeacon
        {
            let body = ClusterBeaconParts::parse(ie.payload(), Mu::M1).expect("well-formed body");
            println!(
                "PT: cluster beacon from net=0x{:06x} tx=0x{:08x} sfn={} network_period={}ms cluster_period={}ms",
                u32::from(net_id),
                u32::from(transmitter),
                body.sfn.0,
                body.network_beacon_period.milliseconds(),
                body.cluster_beacon_period.milliseconds(),
            );
            if let Some(p) = body.cluster_max_tx_power {
                println!("PT: cluster max TX power = {p}");
            }
            if let (Some(ch), Some(t)) = (body.next_cluster_channel, body.time_to_next) {
                println!(
                    "PT: next cluster channel = 0x{:04x} in {} microseconds",
                    ch.as_u16(),
                    t
                );
            }
            observed_sfn = Some(body.sfn.0);
        }
    }
    observed_sfn.expect("beacon contained a cluster beacon body")
}

fn main() {
    let mut last_sfn: Option<u8> = None;
    for sfn in [0u8, 1, 2, 3] {
        let mut buf = [0u8; 64];
        let bytes = ft_build_cluster_beacon(&mut buf, sfn);
        println!("FT -> air: {} bytes (sfn={})", bytes.len(), sfn);
        let observed = pt_observe_beacon(bytes);
        if let Some(prev) = last_sfn {
            assert_eq!(observed, prev.wrapping_add(1));
        }
        last_sfn = Some(observed);
        println!();
    }
}
