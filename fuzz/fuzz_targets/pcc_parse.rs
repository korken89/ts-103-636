//! Property: no PCC byte sequence (any length) may panic the PHY
//! header parser, and accepted headers must re-encode losslessly.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ts_103_636::pcc::Pcc;

fuzz_target!(|data: &[u8]| {
    if let Ok(pcc) = Pcc::parse(data) {
        // Round-trip property: anything we accept must serialize back
        // to the exact input bytes (the reserved bits we ignore on
        // receive are the one allowed difference, so compare through a
        // second parse instead of byte equality).
        let reparsed = match pcc {
            Pcc::Type1(h) => Pcc::parse(&h.to_bytes().expect("serialize accepted header")),
            Pcc::Type2F000(h) => Pcc::parse(&h.to_bytes()),
            Pcc::Type2F001(h) => Pcc::parse(&h.to_bytes()),
        };
        assert_eq!(reparsed.expect("re-parse own output"), pcc);
    }
});
