# Alpha.2 recovery checkpoint — 18 October 2026

This report covers the reconstructed priority fixes. It does not sign off PRD §31.

| Check | Observed result | Limits |
|---|---|---|
| Shared Rust unit tests | 34 passed, zero failed | Linux core execution; includes script, audio filtering, timing, insertion readback and prior regressions |
| Application static checks | Rustfmt and Clippy all-targets/no-deps with warnings denied passed | Upstream vendored warnings remain; Windows-only code needs its native workflow |
| Benchmark-metric tests | 5 passed | NFC/punctuation rules, edit distance, content preservation and nearest-rank p95 |
| Linux release build | Passed; `--version` reports 0.2.0-alpha.2 | CLI/worker, not a Linux desktop application |
| File-worker smoke | Four checks passed | Bengali WAV path with synthetic silence; conflicting audio inputs; missing file; next successful request; no ASR score |
| Benchmark failure accounting | Two input rows retained, one intentional missing-file error | Synthetic silence plus an intentionally absent file; not a Bengali corpus |
| Model used in local smoke | Small Q5_1, 190085487 bytes, SHA-256 verified against the existing manifest | Integrity verification does not qualify recognition quality |
| Windows native fixture | Planned, not delivered in this recovery checkpoint | No actual insertion-matrix pass is claimed |
| Real-device/third-party acceptance | Not run here | Windows mic/permissions, browser/Word/Telegram, network, RAM, DPI, accessibility, update/rollback and long run remain |

The workflow tests and builds Windows/MSVC and Linux. Its outcome is independent evidence and must be checked for the exact commit; this file does not assume success. The app-only artifact has no model weights and is not a full offline installation package. No stable release, Bengali accuracy score, complete V1/V2 delivery, trained VAD or browser direct insertion is claimed.

Reproduce the deterministic checks using docs/BUILD.md. Run the file-worker smoke with `python3 scripts/smoke-file-worker.py target/release/voicetotext models/ggml-small-q5_1.bin`. The application does not download that model. Test files and model weights are not committed.

The local checks above preceded a workspace reset. The reconstructed published revision is verified separately by its exact CI status; previous results are not silently attributed to a new revision.
