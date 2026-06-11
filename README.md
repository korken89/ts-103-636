# ts-103-636

[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

`#![no_std]` types, parsers, builders, and security helpers for
[DECT-2020 New Radio (NR+)](https://www.etsi.org/technologies/dect),
covering:

- **ETSI TS 103 636-3** (Physical layer): MCS table, TX power, PCC
  (Physical Header Field) types, transport block size (TBS) computation
- **ETSI TS 103 636-4** (MAC layer, Release 2): common headers,
  information elements, message bodies, Mode 1 security (AES-128-CMAC
  MIC + AES-128-CTR cipher per §5.9)

Built for embedded use (nRF9151 / nrfxlib in particular) and designed
so the full secured PDU round-trip is exercisable via `cargo test`
without hardware. There is no `alloc` requirement: all builders write
into caller-provided `&mut [u8]` buffers.

## Status

Spec coverage is broad and round-trip-tested:

- **PHY**: MCS table, TX power table, PCC Type 1 / Type 2 (F000 +
  F001), TBS quantization (clause 5.3) validated against the nRF9151
  TBS table for every supported `(mcs, subslot_index)` pair
- **MAC common headers**: Data, Beacon, Unicast, RD Broadcast
- **MAC information elements / messages**: Cluster Beacon, Network
  Beacon, Joining Beacon, Association Request / Response / Release,
  Reconfiguration Request / Response, MAC Security Info, Route Info,
  Resource Allocation, Random Access Resource, RD Capability (long +
  short), Neighbouring, Broadcast Indication, Group Assignment, Load
  Info, Measurement Report, Radio Device Status, Source Routing,
  Joining Information, Association Control
- **Security**: software AES-128-CMAC + CTR backend (`SoftwareCrypto`)
  plus a `MacCrypto` trait so a hardware accelerator can be swapped
  in without touching the layout code

Every message keeps its inline unit tests (`cargo test` runs the full
suite on the host), and the parsers are additionally fuzzed with
coverage-guided libFuzzer targets. Run `make` for the complete check
suite (generated-code drift check, fmt, clippy, tests, the
hardware-crypto configuration, nRF9151 target builds, and docs) - it
is exactly what CI runs. `make fuzz-smoke` runs a short fuzz pass
over every target (requires `cargo-fuzz`), and `make verify` proves
with Kani that every generated parser is panic-free for all inputs
up to per-message size caps and that parse-serialize-parse is the
identity on every parseable input. The Nix dev shell (`nix develop`)
provides the pinned toolchain, `cargo-fuzz`, and Kani.

## Design

- **Generated codecs.** Every MAC message codec under
  `src/mac/messages/generated/` is emitted by the `codegen/` crate
  from a declarative layout definition (`codegen/src/defs/`, one file
  per message, written row-by-row against the spec figures). The
  extraction of the standard's layouts into these definitions was
  AI-assisted; every definition cites its spec clause and figure, and
  each generated module's documentation carries an ASCII wire-layout
  figure precisely so the layouts can be audited against the standard
  side by side. The output is committed and human-readable - direct
  indexing with static offsets, in the same shape as handwritten
  code. `make codegen`
  regenerates; CI fails if the committed output drifts from the
  definitions. Support types with behavior (enums like
  `ResourceAllocationKind`, composites like `PhyCapability`) stay
  handwritten; the generated modules implement their codecs.
- **`const fn` codecs.** For every message that does not carry a
  variable-length list, `encoded_len` / `serialize` / `parse` are
  `const fn`: beacons, security info, status IEs and the like can be
  pre-built (or even pre-parsed) at compile time.

- **Typed everywhere.** Every bit-constrained or invariant-bearing
  field has a typed wrapper. Bit-width errors surface at field
  construction, not on the wire. Fields whose value set is fixed by
  the spec are enums with the spec's discriminant values.
- **`Parts` structs, zero-copy where it counts.** Each body has a
  `*Parts` struct with typed fields end-to-end, used for both
  building and parsing. PDU walking (`Message<'a>`, the IE iterator)
  borrows the receive buffer, and bodies carrying opaque byte runs
  (Group Assignment tags, Reconfiguration flow entries) parse them as
  zero-copy `&'a [T]` slices instead of collecting.
- **Typestate PDU builder.** `MacPduBuilder<'a, State>` enforces the
  correct call order at compile time: you cannot push an IE before a
  header, cannot finish-with-security from an unsecured state, and
  cannot forget to push the MAC Security Info IE first when security =
  `UsedWithIe`.
- **Transparent MIC trailer.** `Message::parse_unverified` strips the 5-byte MIC
  from the tail based on the header's security field, so callers
  always see the body slice. `Message::parse` decrypts,
  verifies, and parses in one call.
- **ETSI MSB-first bit numbering.** Spec figures number bits
  left-to-right starting at 0 (column 0 = MSB). Conversion to C/Rust
  bit numbering is documented below.

## Usage

### Build an unsecured beacon PDU

```rust
use ts_103_636::mac::pdu::{MacPduBuilder, NotUsed};
use ts_103_636::types::{LongRdId, NetworkId24};

let mut buf = [0; 64];
let net = NetworkId24::new(0x123456).unwrap();
let tx  = LongRdId::new(0xAABBCCDD).unwrap();

let written = MacPduBuilder::new(&mut buf)
    .push_beacon(NotUsed, net, tx)
    .unwrap()
    .finish_without_security();
# assert!(!written.is_empty());
```

### Build a secured unicast PDU + parse it back (Mode 1, software backend)

```rust
# #[cfg(feature = "software-crypto")] {
use ts_103_636::mac::pdu::{MacPduBuilder, Message, UsedNoIe};
use ts_103_636::security::{SecurityContext, SoftwareCrypto};
use ts_103_636::types::{LongRdId, SequenceNumber};

let integrity_key = [0; 16];
let cipher_key    = [0; 16];

let tx  = LongRdId::new(0xAABBCCDD).unwrap();
let rx  = LongRdId::new(0x11223344).unwrap();
let psn = SequenceNumber::new(0x123).unwrap();

let ctx = SecurityContext { tx, rx, hpc: 0 };
let mut crypto = SoftwareCrypto;

// Sender side: build + cipher + MIC in one call. The builder captures
// the PSN from `push_unicast` so the IV derivation can't drift.
let mut tx_buf = [0; 256];
let len = MacPduBuilder::new(&mut tx_buf)
    .push_unicast(UsedNoIe, true, psn, rx, tx)
    .unwrap()
    .finish_with_security(&mut crypto, &integrity_key, &cipher_key, &ctx)
    .unwrap()
    .len();

// Receiver side: decrypt + verify + parse in one call. The PSN is
// read from the received (plaintext) common header; the header is
// covered by the MIC, so a tampered PSN fails verification. The
// returned ParsedPdu encodes whether the MIC was verified
// (Secured) or the PDU was plaintext (Unsecured), so unsecured
// traffic flows through the same entry point without ever being
// mistaken for verified data.
let mut rx_buf = tx_buf;
let _msg = Message::parse(
    &mut rx_buf[..len],
    &mut crypto,
    &integrity_key,
    &cipher_key,
    &ctx,
)
.unwrap()
.secured()
.expect("this link requires security");
# }
```

### Compute TBS for arbitrary PHY config

```rust
use ts_103_636::subslot::compute_tbs;
use ts_103_636::types::{Beta, Mcs, Mu};

let tbs = compute_tbs(
    /* subslot_count */ 6,
    Mcs::new(4).unwrap(),
    Beta::B1,
    Mu::M1,
    /* n_ss */ 1,
);
assert_eq!(tbs, Some(4024)); // matches the nRF9151 reference table.
```

## ETSI bit numbering convention

ETSI (and 3GPP, ITU-T) figures use MSB-first bit numbering, also known
as "first-transmitted-bit" numbering. In the spec figures each byte is
drawn as eight columns numbered `0..=7` left-to-right, where:

- Column `0` (leftmost) is the MSB of the byte, i.e. bit `7` in C/Rust.
- Column `7` (rightmost) is the LSB of the byte, i.e. bit `0` in C/Rust.

So the figure's bit numbers run in the opposite direction from
CPU/Rust bit numbering, but the on-wire and in-memory layout is
exactly what you'd expect: the leftmost field in the figure sits in
the high-order bits of the byte. There is no byte- or bit-reversal -
only a different counting direction in the figure labels.

### Mask quick-reference

| ETSI bit (figure column) | C/Rust bit | Mask  |
|--------------------------|------------|-------|
| 0                        | 7          | 0x80  |
| 1                        | 6          | 0x40  |
| 2                        | 5          | 0x20  |
| 3                        | 4          | 0x10  |
| 4                        | 3          | 0x08  |
| 5                        | 2          | 0x04  |
| 6                        | 1          | 0x02  |
| 7                        | 0          | 0x01  |

### Worked example

In Section 6.4.2.3, the Cluster Beacon body's flag byte (byte 1) is
drawn across ETSI bits 0..=7 like this:

```text
+------+------+------+------+------+------+------+------+
| R    | R    | R    | TXp  | PC   | FO   | NC   | TTN  |   field
+------+------+------+------+------+------+------+------+
| 0    | 1    | 2    | 3    | 4    | 5    | 6    | 7    |   ETSI bit
| 7    | 6    | 5    | 4    | 3    | 2    | 1    | 0    |   C/Rust bit
| 0x80 | 0x40 | 0x20 | 0x10 | 0x08 | 0x04 | 0x02 | 0x01 |   mask
+------+------+------+------+------+------+------+------+
```

So `TX Power` at ETSI bit 3 uses mask `0x10`, `Power Const` at ETSI
bit 4 uses mask `0x08`, and the Reserved span at ETSI bits 0..=2 sits
in the top three C/Rust bits (mask `0xE0`).

Multi-byte values (e.g. the 13-bit absolute channel) are big-endian in
byte order in addition to MSB-first inside each byte. In practice
this means `to_be_bytes` / `from_be_bytes` work directly.

## Features

| Feature           | Default | Effect |
|-------------------|---------|--------|
| `software-crypto` | on      | Pulls in `aes` + `cmac` + `ctr` and exposes `SoftwareCrypto`, a software `MacCrypto` backend |
| `defmt`           | off     | Derives `defmt::Format` on the public types and adapters |

Disable `software-crypto` (e.g. with `default-features = false`) when
you are wiring a hardware crypto accelerator behind your own
`MacCrypto` impl and want to drop the RustCrypto dependencies
entirely.

## Repository layout

The repo is a Cargo workspace; only the `ts-103-636` library is
published. The auxiliary crates exist so the full tool chain is
covered by the same `make ci` run:

| Path       | Crate                | Purpose |
|------------|----------------------|---------|
| `/`        | `ts-103-636`         | The published `#![no_std]` library |
| `codegen/` | `ts-103-636-codegen` | Snapshot generator for `src/mac/messages/generated/` (`make codegen`, drift-checked in CI) |
| `fuzz/`    | `ts-103-636-fuzz`    | libFuzzer targets (`make fuzz-smoke`); seeds are committed, the working corpus is not |
| `verify/`  | `ts-103-636-verify`  | Kani proof harnesses (`make verify`); standalone, not a workspace member |

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any
additional terms or conditions.

## Spec references

- ETSI TS 103 636-3 V2.1.1 (2024-10): DECT-2020 New Radio (NR) Part 3:
  Physical layer
- ETSI TS 103 636-4 V2.1.1 (2024-10): DECT-2020 New Radio (NR) Part 4:
  MAC layer, Release 2

This crate is an independent implementation of the public ETSI
standards; it is not endorsed by or affiliated with ETSI or Nordic
Semiconductor.

## Prior art

The IE and PDU shapes were initially modeled on
[hophop](https://codeberg.org/silanos/hophop)'s `ts-103-636-numbers` /
`ts-103-636-utils` crates (MIT OR Apache-2.0).
