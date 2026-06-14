//! Secured user-plane round-trip (Mode 1 MAC security).
//!
//! Companion to `data_flow.rs`. Same sensor scenario, but every PDU is
//! built with [`MacPduBuilder::finish_with_security`] and parsed with
//! [`Message::parse`], exercising AES-128-CTR encryption and
//! AES-128-CMAC integrity protection per ETSI TS 103 636-4 §5.9.1.
//!
//! What this example shows beyond the unsecured version:
//! * [`SecurityContext`] setup: per-direction `(tx, rx, hpc)` tuple.
//! * The `UsedNoIe` security mode (the common steady-state choice once
//!   the receiver has been told the HPC out-of-band).
//! * Real ciphertext on the wire: the User Plane Data Flow IE payload
//!   is unrecognizable after `finish_with_security`.
//! * MIC verification: flipping a single ciphertext byte makes
//!   [`Message::parse`] reject the PDU with
//!   [`MacSecurityError::BadMic`].
//!
//! What this example does NOT show:
//! * `UsedWithIe`: when the transmitter embeds a [`MacSecurityInfoParts`]
//!   IE so the receiver can recover the HPC inline. Useful during
//!   initial association or HPC resynchronization. Same crypto primitives
//!   apply; the only difference is that the security info IE is pushed
//!   via [`MacPduBuilder::push_mac_security_info`] right after the
//!   common header and before any other IEs.
//! * Production key management: real systems derive `int_key`/`cipher_key`
//!   from the master key via a KDF and rotate them per HPC bump.
//!
//! Requires the default `software-crypto` feature. Run with
//! `cargo run --example secured_data_flow`.

use ts_103_636::prelude::*;
use ts_103_636::security::{MacSecurityError, SecurityContext, software::SoftwareCrypto};

const PT_ID: u32 = 0x1111_AAAA;
const FT_ID: u32 = 0x2222_BBBB;

/// Long-lived secrets the two endpoints share. In production these come
/// from the association-time key agreement; here we hard-code them.
struct Keys {
    integrity: [u8; 16],
    cipher: [u8; 16],
}

/// Application-level packet, same shape as in `data_flow.rs`.
struct SensorSample {
    channel: u8,
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

/// PT-side: build an encrypted, MIC-authenticated Unicast PDU carrying
/// the sample on User Plane Data Flow 1.
fn pt_build_secured_data<'a>(
    buf: &'a mut [u8],
    crypto: &mut SoftwareCrypto,
    keys: &Keys,
    ctx: &SecurityContext,
    psn: SequenceNumber,
    sample: &SensorSample,
) -> &'a [u8] {
    let pt = LongRdId::try_from_u32(PT_ID).unwrap();
    let ft = LongRdId::try_from_u32(FT_ID).unwrap();

    let mut payload = [0; SensorSample::ENCODED_LEN];
    sample.encode(&mut payload);

    let ie = InformationElement::new_6bit_with_length(IEType6bit::UserPlaneDataFlow1, &payload)
        .expect("payload fits in IE length");

    // `UsedNoIe` selects MAC security mode 1 with the security info
    // carried out-of-band. `finish_with_security` appends the 5-byte
    // MIC, then encrypts the spec-defined range with AES-128-CTR using
    // an IV derived from `ctx` and the PSN captured by `push_unicast`.
    MacPduBuilder::new(buf)
        .push_unicast(UsedNoIe, false, psn, ft, pt)
        .expect("buffer fits header")
        .push_ie(&ie)
        .expect("buffer fits IE")
        .finish_with_security(crypto, &keys.integrity, &keys.cipher, ctx)
        .expect("crypto succeeds")
}

/// FT-side: decrypt + verify the PT's secured PDU and react.
///
/// Returns `Ok(decoded_sample)` if the MIC verifies and the payload
/// parses, or the [`MacSecurityError`] that caused rejection.
fn ft_receive_secured(
    buf: &mut [u8],
    crypto: &mut SoftwareCrypto,
    keys: &Keys,
    ctx: &SecurityContext,
) -> Result<SensorSample, MacSecurityError<core::convert::Infallible>> {
    let msg = Message::parse(buf, crypto, &keys.integrity, &keys.cipher, ctx)?
        .secured()
        // This link mandates security: treat a plaintext PDU as a
        // protocol violation. ParsedPdu makes that decision explicit.
        .expect("link requires secured PDUs");
    // After `parse`, `msg.tail` is plaintext.
    for ie in msg.tail_items() {
        let ie = ie.expect("well-formed IE");
        if matches!(
            ie.ie_number(),
            AnyIeType::Type6bit(t) if t == IEType6bit::UserPlaneDataFlow1,
        ) {
            return Ok(SensorSample::decode(ie.payload()).expect("3-byte payload"));
        }
    }
    panic!("secured PDU did not contain a User Plane Data Flow 1 IE");
}

fn main() {
    let mut crypto = SoftwareCrypto;
    let keys = Keys {
        integrity: [0x11; 16], // K_int
        cipher: [0x22; 16],    // K_cipher
    };

    // One SecurityContext per direction. Both ends keep both contexts.
    let pt_to_ft_ctx = SecurityContext {
        tx: LongRdId::try_from_u32(PT_ID).unwrap(),
        rx: LongRdId::try_from_u32(FT_ID).unwrap(),
        hpc: 0x0000_1000, // PT's TX HPC
    };
    let ft_to_pt_ctx = SecurityContext {
        tx: LongRdId::try_from_u32(FT_ID).unwrap(),
        rx: LongRdId::try_from_u32(PT_ID).unwrap(),
        hpc: 0x0000_2000, // FT's TX HPC
    };

    // -------------------------------------------------------------------
    // Two-message round-trip: PT data sample, FT app-level ack.
    // -------------------------------------------------------------------
    let sample = SensorSample {
        channel: 1,
        sample: 0x0123,
    };
    let pt_psn = SequenceNumber::try_from_u16(1).unwrap();

    // PT -> FT (encrypted)
    let mut tx_buf = [0; 64];
    let secured_len = pt_build_secured_data(
        &mut tx_buf,
        &mut crypto,
        &keys,
        &pt_to_ft_ctx,
        pt_psn,
        &sample,
    )
    .len();
    println!(
        "PT -> FT: {} bytes (secured). First IE payload bytes (ciphertext):",
        secured_len
    );
    // The cipher range covers everything after the MAC common header
    // (UsedNoIe), so the IE header + body are all encrypted.
    print!("  ");
    for byte in &tx_buf[..secured_len] {
        print!("{byte:02x} ");
    }
    println!("\n");

    // FT -> decrypts in place. Need a clone because parse mutates.
    let mut rx_buf = tx_buf;
    let decoded = ft_receive_secured(
        &mut rx_buf[..secured_len],
        &mut crypto,
        &keys,
        &pt_to_ft_ctx,
    )
    .expect("MIC verifies");
    println!(
        "FT decoded sample: channel={} sample={:#06x}",
        decoded.channel, decoded.sample
    );

    // FT -> PT (encrypted ack on Higher Layer Signalling Flow 1)
    let ack_psn = SequenceNumber::try_from_u16(1).unwrap();
    let mut ack_tx_buf = [0; 64];
    let ack_payload = 0x0001_u16.to_be_bytes();
    let ack_ie = InformationElement::new_6bit_with_length(
        IEType6bit::HigherLayerSignallingFlow1,
        &ack_payload,
    )
    .expect("ack fits");
    let pt = LongRdId::try_from_u32(PT_ID).unwrap();
    let ft = LongRdId::try_from_u32(FT_ID).unwrap();
    let ack_secured_len = MacPduBuilder::new(&mut ack_tx_buf)
        .push_unicast(UsedNoIe, false, ack_psn, pt, ft)
        .expect("buffer fits header")
        .push_ie(&ack_ie)
        .expect("buffer fits IE")
        .finish_with_security(&mut crypto, &keys.integrity, &keys.cipher, &ft_to_pt_ctx)
        .expect("crypto succeeds")
        .len();
    println!("FT -> PT: {} bytes (secured ack)\n", ack_secured_len);

    // PT -> decrypts the ack.
    let mut ack_rx_buf = ack_tx_buf;
    let ack_msg = Message::parse(
        &mut ack_rx_buf[..ack_secured_len],
        &mut crypto,
        &keys.integrity,
        &keys.cipher,
        &ft_to_pt_ctx,
    )
    .expect("ack MIC verifies")
    .secured()
    .expect("ack is secured");
    let ack_ie = ack_msg.tail_items().next().unwrap().unwrap();
    let ack_seq = u16::from_be_bytes([ack_ie.payload()[0], ack_ie.payload()[1]]);
    println!("PT verified ack, sequence = {ack_seq}");

    // -------------------------------------------------------------------
    // Tamper detection: flip one ciphertext byte and watch
    // `parse` reject the PDU.
    // -------------------------------------------------------------------
    println!("\n--- Tamper detection ---");
    let mut tampered = tx_buf;
    // Flip a byte somewhere inside the ciphertext (avoid the MAC
    // header itself; that's not encrypted).
    let tamper_offset = secured_len - 8;
    tampered[tamper_offset] ^= 0x40;
    println!(
        "Flipped byte at offset {tamper_offset}: 0x{:02x} -> 0x{:02x}",
        tx_buf[tamper_offset], tampered[tamper_offset],
    );
    match ft_receive_secured(
        &mut tampered[..secured_len],
        &mut crypto,
        &keys,
        &pt_to_ft_ctx,
    ) {
        Ok(_) => panic!("tamper detection failed: PDU accepted despite flipped byte"),
        Err(MacSecurityError::BadMic) => {
            println!("FT rejected the tampered PDU: MacSecurityError::BadMic");
        }
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}
