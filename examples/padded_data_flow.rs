//! Padding a PDU to a PHY-mandated size.
//!
//! DECT-2020 NR+ PHY transmits a Transport Block of a specific size
//! (see [`subslot::compute_tbs`]: a function of subslot count, MCS,
//! beta, mu, and N_ss). The MAC layer is expected to fill that block
//! exactly; any gap between the IE stream and the TBS is occupied by
//! Padding IEs. This example builds a small PDU and uses the
//! `finish_*_padded` family to fill it to a chosen target size.
//!
//! Three scenarios:
//!
//! 1. **Unsecured + small gap (1 byte)** - uses a single 5-bit Short
//!    Padding IE (`0xC0`).
//! 2. **Unsecured + larger gap (~14 bytes)** - uses a 6-bit Padding IE
//!    with an 8-bit length field (`0x40` head, length byte, zeroed
//!    payload), per clause 6.4.3.8.
//! 3. **Secured to TBS** - `finish_with_security_padded` accounts for
//!    the 5-byte MIC internally; the caller specifies the final
//!    on-wire length and the helper fills + encrypts + MICs.
//!
//! Run with `cargo run --example padded_data_flow`.

use ts_103_636::prelude::*;
use ts_103_636::security::{SecurityContext, software::SoftwareCrypto};
use ts_103_636::subslot::compute_tbs;

const PT_ID: u32 = 0x1111_AAAA;
const FT_ID: u32 = 0x2222_BBBB;

/// 3-byte application payload (same as `data_flow.rs`).
fn sensor_bytes(channel: u8, sample: u16) -> [u8; 3] {
    let mut out = [0u8; 3];
    out[0] = channel;
    out[1..3].copy_from_slice(&sample.to_be_bytes());
    out
}

fn main() {
    // ---------------------------------------------------------------
    // 1. Small gap: a single Short Padding IE fills 1 byte exactly.
    // ---------------------------------------------------------------
    {
        let payload = sensor_bytes(1, 0x0123);
        let ie = InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, &payload)
            .unwrap();
        let pt = LongRdId::new(PT_ID).unwrap();
        let ft = LongRdId::new(FT_ID).unwrap();
        let psn = SequenceNumber::new(1).unwrap();

        // Build without padding first to see the bare length.
        let mut buf_bare = [0u8; 64];
        let bare_len = MacPduBuilder::new(&mut buf_bare)
            .push_unicast(NotUsed, false, psn, ft, pt)
            .unwrap()
            .push_ie(&ie)
            .unwrap()
            .finish_without_security()
            .len();
        println!("Scenario 1 (small gap): bare PDU is {bare_len} bytes");

        // Now pad to bare_len + 1.
        let mut buf = [0u8; 64];
        let target = bare_len + 1;
        let padded = MacPduBuilder::new(&mut buf)
            .push_unicast(NotUsed, false, psn, ft, pt)
            .unwrap()
            .push_ie(&ie)
            .unwrap()
            .finish_without_security_padded(target)
            .unwrap();
        println!(
            "  padded to {} bytes, last byte = 0x{:02x} (5-bit Short Padding head)",
            padded.len(),
            padded[padded.len() - 1],
        );
        assert_eq!(padded[padded.len() - 1], 0xC0);

        // Walk the IE stream to verify the dispatch sees both IEs.
        let msg = Message::parse_unverified(padded).unwrap();
        let kinds: heapless::Vec<_, 4> = msg.tail_items().map(|r| r.unwrap().ie_number()).collect();
        println!("  IEs in PDU: {kinds:?}\n");
    }

    // ---------------------------------------------------------------
    // 2. Large gap: one 8-bit-length Padding IE absorbs everything.
    // ---------------------------------------------------------------
    {
        let payload = sensor_bytes(2, 0xCAFE);
        let ie = InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, &payload)
            .unwrap();
        let pt = LongRdId::new(PT_ID).unwrap();
        let ft = LongRdId::new(FT_ID).unwrap();
        let psn = SequenceNumber::new(2).unwrap();

        let target = 32;
        let mut buf = [0u8; 64];
        let padded = MacPduBuilder::new(&mut buf)
            .push_unicast(NotUsed, false, psn, ft, pt)
            .unwrap()
            .push_ie(&ie)
            .unwrap()
            .finish_without_security_padded(target)
            .unwrap();
        println!("Scenario 2 (large gap): padded to {} bytes", padded.len());
        println!(
            "  first padding bytes = 0x{:02x} 0x{:02x} (Padding IE head + 8-bit length)",
            padded[16], padded[17],
        );
        assert_eq!(padded[16], 0x40);

        // The parser sees the original IE followed by a single Padding
        // IE whose 8-bit length covers the rest of the PDU.
        let msg = Message::parse_unverified(padded).unwrap();
        for ie in msg.tail_items() {
            println!("  -> {:?}", ie.unwrap().ie_number());
        }
        println!();
    }

    // ---------------------------------------------------------------
    // 3. Secured PDU padded to a PHY transport block size.
    // ---------------------------------------------------------------
    {
        // Example PHY config: 2 subslots, MCS 1, beta 1, mu 1, single
        // spatial stream. compute_tbs returns the TBS in BITS per ETSI
        // 5.3; the MAC fills (tbs / 8) bytes.
        let phy_subslots = 2u8;
        let phy_mcs = Mcs::new(1).unwrap();
        let phy_beta = Beta::new(1).unwrap();
        let phy_mu = Mu::new(1).unwrap();
        let phy_nss = 1;
        let tbs_bits = compute_tbs(phy_subslots, phy_mcs, phy_beta, phy_mu, phy_nss)
            .expect("supported PHY parameters") as usize;
        let tbs_bytes = tbs_bits / 8;
        println!(
            "Scenario 3 (secured to TBS): PHY config -> TBS = {tbs_bits} bits = {tbs_bytes} bytes",
        );

        let mut crypto = SoftwareCrypto;
        let int_key = [0x11u8; 16];
        let cipher_key = [0x22u8; 16];
        let psn = SequenceNumber::new(1).unwrap();
        let ctx = SecurityContext {
            tx: LongRdId::new(PT_ID).unwrap(),
            rx: LongRdId::new(FT_ID).unwrap(),
            hpc: 0x1000,
        };
        let payload = sensor_bytes(3, 0x4242);
        let ie = InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, &payload)
            .unwrap();

        let mut buf = [0u8; 256];
        let secured = MacPduBuilder::new(&mut buf)
            .push_unicast(UsedNoIe, false, psn, ctx.rx, ctx.tx)
            .unwrap()
            .push_ie(&ie)
            .unwrap()
            .finish_with_security_padded(tbs_bytes, &mut crypto, &int_key, &cipher_key, &ctx)
            .expect("crypto + padding succeeds");
        println!(
            "  built {} bytes (exactly TBS), trailing 5 bytes = MIC, IE stream + padding encrypted",
            secured.len()
        );
        assert_eq!(secured.len(), tbs_bytes);

        // Receiver decrypts, verifies MIC, and walks the (now plaintext)
        // IE stream. The Padding IE is visible alongside the data IE.
        let mut rx_buf = buf;
        let msg = Message::parse(
            &mut rx_buf[..tbs_bytes],
            &mut crypto,
            &int_key,
            &cipher_key,
            &ctx,
        )
        .expect("MIC verifies")
        .secured()
        .expect("PDU was built secured");
        for ie in msg.tail_items() {
            println!("  -> {:?}", ie.unwrap().ie_number());
        }
    }
}
