# Single source of truth for all checks: CI runners (GitHub or GitLab)
# just call `make ci`, and a developer runs the identical suite locally
# with plain `make` (or any individual target) before pushing.

# nRF9151 (Cortex-M33F)
THUMB_TARGET ?= thumbv8m.main-none-eabihf

.DEFAULT_GOAL := ci

.PHONY: ci fmt lint test no-sw-crypto thumb doc fuzz-smoke codegen codegen-check

ci: codegen-check fmt lint test no-sw-crypto thumb doc

# Regenerate src/mac/messages/generated/ from codegen/src/defs/.
codegen:
	cargo run --manifest-path codegen/Cargo.toml

# Drift guard: fail if the committed generated files do not match
# what the definitions produce.
codegen-check:
	cargo run --manifest-path codegen/Cargo.toml -- --check
	cargo test --manifest-path codegen/Cargo.toml

# Formatting check (no changes applied).
fmt:
	cargo fmt --check

# Clippy over every target and feature configuration, warnings fatal.
lint:
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --features defmt -- -D warnings
	cargo clippy --no-default-features -- -D warnings

# Host test suite, with and without the defmt feature.
test:
	cargo test
	cargo test --features defmt

# Hardware-crypto configuration: software-crypto disabled, so the
# RustCrypto dependencies must not be required to build.
no-sw-crypto:
	cargo build --no-default-features
	cargo build --no-default-features --features defmt

# Embedded target builds: the crate's stated purpose. Covers the
# default (software-crypto), hardware-crypto, and defmt configurations.
thumb:
	cargo build --target $(THUMB_TARGET)
	cargo build --target $(THUMB_TARGET) --no-default-features
	cargo build --target $(THUMB_TARGET) --no-default-features --features defmt

# Rustdoc with warnings fatal (catches broken intra-doc links).
doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Short fuzz pass over every target (not part of `make ci`: run it
# locally or from a scheduled CI job). Requires cargo-fuzz. On stable
# toolchains the sanitizer must be disabled (-s none); with nightly
# you can drop that to get AddressSanitizer too. libFuzzer does not
# parallelize by itself: FUZZ_JOBS spawns that many worker processes
# sharing the corpus (default: all cores).
FUZZ_TARGETS := pdu_parse message_bodies pcc_parse parse_secure
FUZZ_SECONDS ?= 30
FUZZ_JOBS ?= $(shell nproc)
fuzz-smoke:
	for t in $(FUZZ_TARGETS); do \
		cargo fuzz run $$t -s none fuzz/corpus/$$t fuzz/seeds/$$t -- \
			-jobs=$(FUZZ_JOBS) -workers=$(FUZZ_JOBS) \
			-max_total_time=$(FUZZ_SECONDS) || exit 1; \
	done
	rm -f fuzz-*.log
