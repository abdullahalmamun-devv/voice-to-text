# Contributing

This is an engineering alpha. Read docs/PRODUCT-REQUIREMENTS.md and docs/REQUIREMENTS-STATUS.md before changing scope. Official functionality remains free, open source and entirely local at runtime.

Use the pinned Rust toolchain, CMake and a C++ compiler as described in docs/BUILD.md. Dependencies and local patches are vendored. Run `cargo fmt --check`, `cargo test --offline --locked --lib`, and `cargo clippy --offline --locked --all-targets --no-deps -- -D warnings`.

Keep changes small and report which operating systems were actually tested. Cross-compilation is build evidence, not microphone, accessibility, insertion or recognition acceptance. Add regression tests for consequential failures. Never add automatic downloads, cloud inference, accounts, telemetry or hidden speech persistence.

Do not commit credentials, personal recordings or private transcripts. Use consented, rights-cleared fixtures. Bug reports may describe failures without attaching speech. Document dependency patches and preserve upstream licenses.
