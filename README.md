# VoiceToText — Offline Bangla Dictation

Version: **0.2.0-alpha.3 (development)** · Windows 11 x64 desktop · MIT application source.

This build combines the PRD's V1 core with optional V2 features in one native Windows application. It is an engineering alpha, **not a PRD-complete stable release**. No Bengali accuracy percentage or universal application compatibility is claimed. Read [START-HERE.md](START-HERE.md) for installation and [docs/REQUIREMENTS-STATUS.md](docs/REQUIREMENTS-STATUS.md) for remaining implementation and validation work.

## Current checkpoint

Source is public at https://github.com/abdullahalmamun-devv/voice-to-text. See CHANGELOG.md, docs/PROJECT-STATUS.md and the exact commit's workflow status. Alpha.2 restores cancellable WAV reads, anti-alias filtering, script-mismatch review, audio-bound SRT timing and targeted native insertion/Undo. Browser/custom controls still require Copy.

Alpha.3 adds an explicitly experimental native Bengali Zipformer profile with pinned model/runtime bytes and required review. Python is not a shipped runtime. See [Bengali setup](docs/EXPERIMENTAL-BENGALI.md) and [measured investigation](benchmarks/fleurs-bn-smoke-30/DECISION.md).

No Bengali model has qualified for stable release. The external 30-clip normalized WER is 17.74%, above the proposed 10% clean target. Mixed English, punctuation, critical entities and Bangladesh conversational quality remain open. Small from alpha.1 failed this diagnostic and is not recommended as a production default.

## Included

- Local multilingual Whisper inference, a bundled Small Q5_1 model in the Windows package, no account, HTTP client, updater, remote fallback, browser/WebView or server.
- Microphone selection, configurable global shortcut, toggle/hold recording, tray and recording surface, explicit cancellation, lock/suspend cancellation and 120-second microphone limit.
- Bengali / English / automatic language detection; conservative number formatting; preferred terms and exact alias dictionary; result review, edit, copy, retry and discard.
- Guarded Unicode insertion into native Edit/RichEdit controls. Changed focus/caret/content, password fields and selections block insertion. Undo is limited to an unchanged, verified app insertion.
- Optional V2 draft preview, transcript-only local history, confirmed exact-phrase corrections, explicit snippets, bounded editing of app result text, PCM/float WAV import and TXT/Markdown/raw-segment SRT export.
- Per-user settings with a previous-file backup; manual settings/dictionary import/export; light/dark/system themes, DPI scaling, opt-in startup and text-free diagnostics.

History, live draft, voice commands and startup are off by default. Review is on. Microphone audio is retained in RAM for Retry until the next recording, Discard/Cancel or Exit; it is never automatically written to an audio file.

## Packages

CI app-only artifacts contain no models. The separate **Windows-experimental-Bengali-alpha3** artifact includes the explicitly selected Bengali pack, native CPU libraries, guide, licenses and checksums. It requires Microsoft Visual C++ v14 and remains an unsigned experimental package, not a clean-machine installer or stable release.

The Windows archive contains the executable, multilingual model, notices, guide and checksum manifest. Extract the whole archive before running `VoiceToText.exe`; Python, Rust and CMake are not user runtime requirements. The source archive contains vendored build dependencies but excludes the model weights; copy `models/` from the Windows package when developing.

The Linux binary supplied under `bin/` in the source archive is a tested local CLI/worker, **not a Linux desktop application**. macOS and Linux desktop adapters are not implemented. No stable or signed binaries are published.

## Development

See [docs/BUILD.md](docs/BUILD.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/DEPENDENCY-PATCHES.md](docs/DEPENDENCY-PATCHES.md) and [docs/QA-REPORT.md](docs/QA-REPORT.md). The original requirements are preserved in [docs/PRODUCT-REQUIREMENTS.md](docs/PRODUCT-REQUIREMENTS.md).

Application code is MIT. Third-party components and model weights retain their own licenses; see `licenses/` and `docs/THIRD-PARTY.md`.
