//! Property: the full secured receive path must not panic on any
//! input. MIC verification rejects nearly everything, but the code
//! BEFORE verification runs on every frame: the cipher-range
//! computation (including the plaintext IE walk that locates the MAC
//! Security Info IE), the PSN extraction, the IV derivation, and the
//! in-place CTR pass.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ts_103_636::prelude::*;

fuzz_target!(|data: &[u8]| {
    let mut crypto = SoftwareCrypto;
    let ctx = SecurityContext {
        tx: LongRdId::new(0x1111_1111).unwrap(),
        rx: LongRdId::new(0x2222_2222).unwrap(),
        hpc: 0x1234_5678,
    };
    let keys = [0; 16];

    let mut buf = data.to_vec();
    if let Ok(pdu) = Message::parse(&mut buf, &mut crypto, &keys, &keys, &ctx) {
        // Whatever survives must expose a walkable IE stream.
        let msg = pdu.into_message();
        for ie in msg.tail_items() {
            if ie.is_err() {
                break;
            }
        }
    }

    // The pre-decryption peek must also hold up on arbitrary bytes.
    let _ = Message::peek_security_info(data);
});
