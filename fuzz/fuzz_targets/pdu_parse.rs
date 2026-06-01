//! Property: no over-the-air byte sequence may panic the MAC PDU
//! splitter or the IE stream walker.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ts_103_636::prelude::*;

fuzz_target!(|data: &[u8]| {
    if let Ok(msg) = Message::parse_unverified(data) {
        // Walk the whole IE stream; the iterator must terminate and
        // must not panic on any malformed tail.
        for ie in msg.tail_items() {
            if ie.is_err() {
                break;
            }
        }
    }
});
