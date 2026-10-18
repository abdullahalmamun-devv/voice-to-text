# Gate 0 — partial engineering report

Date: 2026-10-11. **Gate 0 remains open.** Foundation audit and executable prototype exist; Bengali model comparison and actual Windows quality/resource/privacy tests do not.

## Foundation decision

Handy candidate inspected at commit `48a3d8e9ee456c1c6a3b7223779271c457890b4d`. Its existing product contains downloader/updater/network-backed postprocessing pathways and history/storage/UI infrastructure that require a larger removal and runtime audit. It was not adopted as a product dependency. This is a scope/maintenance decision, not a claim that Handy is unsafe or that its own offline features fail.

The PRD-permitted fallback was selected for this alpha: shared Rust core, small native Win32 UI and a local inference adapter. This eliminates a WebView bootstrap and product network client while leaving browser/general-editor insertion as separate adapter work. No Handy code is included in the new app. Corresponding source and vendored Rust/whisper.cpp notices are provided.

## Model and build baseline

One candidate, multilingual Whisper Small Q5_1, is bundled: 190,085,487 bytes, SHA256 in `model-manifest.json`. It is **not an evaluated final model selection**. Large-v3-turbo comparison has not been run. Runtime: whisper-rs 0.15.1, whisper-rs-sys 0.14.1, whisper.cpp 1.7.6, CPU only, final beam size 5, previews greedy, threads capped at 6. No translation, temperature fallback or LLM rewriting.

Windows x86_64 GNU release was cross-compiled on Linux using Rust 1.85.1, GCC 13-posix and CMake. Windows PE import inspection found only Windows system libraries, with C++/pthread runtime statically linked. Build dependencies are vendored and the final builds ran with Cargo `--offline --locked`. Toolchains must be preinstalled; source archive does not contain Rust/CMake/compilers.

Linux worker/CLI smoke used an AMD EPYC 9V74 virtual environment with 9 exposed CPUs. The upstream 16kHz mono `jfk.wav` sample produced nonempty English text, real timed segments, 500 silent requests produced no text, malformed prompt returned a recoverable error, and kill/restart recovered. **This is not Bengali quality evidence or Windows performance evidence.** Timings appear in the raw smoke report; shared virtual CPU load and synthetic IPC requests make them unsuitable for product latency claims. This environment's `/proc` descriptor access was restricted and reported implausible RSS; no valid RAM/network runtime measurement is published.

## Remaining gate inputs

- Rights-cleared, speaker-separated Bengali/English/mixed corpus and transcripts; initial 200 utterances/12 speakers, then 1000/50 holdout with clean/noisy/accent/name/number/negative/silence slices.
- Same corpus/hardware/configuration comparison of Small and Large-v3-turbo; freeze thresholds and choose one default only after results.
- Real Windows 11 device/microphone and required supported textboxes; nominal insertion and focus/selection/password/lock/sleep guard suite.
- App-plus-child-process network capture, fresh disconnected install and permissions audit.
- All-in app+worker CPU/RAM, cold/warm latency, draft backlog, 500 desktop cycles/cancellation loops and extended resource run.

`scripts/benchmark.py` is an offline preliminary runner, not a completed corpus evaluation. It reports raw whitespace WER, punctuation-normalized WER and Unicode-scalar CER separately per slice, plus cold timings. It does not implement the full critical-entity/correction-effort/grapheme scorebook. The manual test register and requirements status record those gaps.
