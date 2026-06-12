# Single source of truth for all checks: CI runners (GitHub or GitLab)
# just call `make ci`, and a developer runs the identical suite locally
# with plain `make` (or any individual target) before pushing.

# nRF9151 (Cortex-M33F)
THUMB_TARGET ?= thumbv8m.main-none-eabihf

.DEFAULT_GOAL := ci

.PHONY: ci fmt lint test no-sw-crypto thumb doc fuzz-smoke codegen codegen-check verify

ci: codegen-check fmt lint test no-sw-crypto thumb doc

# Regenerate src/mac/messages/generated/ from codegen/src/defs/.
codegen:
	cargo run -p ts-103-636-codegen

# Drift guard: fail if the committed generated files do not match
# what the definitions produce. (The codegen crate's own tests run
# as part of the workspace `test` target.)
codegen-check:
	cargo run -p ts-103-636-codegen -- --check

# Formatting check over the whole workspace (no changes applied).
fmt:
	cargo fmt --all --check

# Clippy over every target and feature configuration, warnings fatal.
# The workspace pass also lints the codegen and fuzz crates.
lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy --all-targets --features defmt -- -D warnings
	cargo clippy --no-default-features -- -D warnings

# Host test suite: the whole workspace (lib + codegen snapshot test;
# the fuzz binaries build but carry no tests), then the lib again
# with the defmt feature.
test:
	cargo test --workspace
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

# Short fuzz pass over every target (not part of `make ci`). Requires
# cargo-fuzz; -s none because AddressSanitizer needs nightly and the
# pinned toolchain is stable. FUZZ_JOBS spawns that many libFuzzer
# workers sharing the corpus (default: all cores).
FUZZ_TARGETS := pdu_parse message_bodies parse_secure
FUZZ_SECONDS ?= 30
FUZZ_JOBS ?= $(shell nproc)
fuzz-smoke:
	for t in $(FUZZ_TARGETS); do \
		cargo fuzz run $$t -s none fuzz/corpus/$$t fuzz/seeds/$$t -- \
			-jobs=$(FUZZ_JOBS) -workers=$(FUZZ_JOBS) \
			-max_total_time=$(FUZZ_SECONDS) || exit 1; \
	done
	rm -f fuzz-*.log

# Symbolic verification (not part of `make ci`): parse never panics
# and parse-serialize-parse is the identity, per message, up to the
# buffer caps in verify/src/lib.rs. Requires Kani (the Nix dev shell
# provides it). The grep guard fails the run if a generated message
# has no harness.
VERIFY_JOBS ?= $(shell nproc)
verify:
	@for m in $(basename $(notdir $(wildcard src/mac/messages/generated/*.rs))); do \
		grep -q "$${m}_codec_safe" verify/src/lib.rs \
			|| { echo "missing harness: $${m}_codec_safe"; exit 1; }; \
		grep -q "$${m}_serialize_total" verify/src/lib.rs \
			|| { echo "missing harness: $${m}_serialize_total"; exit 1; }; \
	done
	cd verify && cargo kani -j $(VERIFY_JOBS) --output-format=terse
