# Changelog

## 0.2.0-alpha.2 — development checkpoint

- Read WAV files in the cancellable inference process, retain a path for Retry, and validate duration/layout before decoding.
- Use anti-alias resampling; hold automatic insertion for unexpected scripts in explicit language presets.
- Refuse SRT export for invalid, overlapping or out-of-bounds timestamps without fabricating timings.
- Address native Edit/RichEdit controls directly for insertion and bounded owned Undo; verify UTF-16 and control line endings; reject multiline output in single-line fields.
- Use Windows Unicode model paths without a model-sized GNU byte buffer; forward CPU compatibility flags explicitly into CMake.
- Record benchmark configuration hashes and failures, preserve raw/normalized metrics, and correct nearest-rank p95.
- Add public contribution/privacy/security guidance and Linux/Windows build workflows.

The PRD is not complete. Browser UI Automation, complete independent UI language/onboarding, evaluated trained VAD, model selection, real-device QA and macOS/Linux desktop adapters remain open. No stable Bengali accuracy score or complete V1/V2 claim is made.

## 0.2.0-alpha.1

Original recovered engineering alpha; see docs/QA-REPORT.md for historical evidence.
