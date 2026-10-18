> Historical alpha.1 report. Current alpha.3 evidence and remaining gates are in [PROJECT-STATUS.md](PROJECT-STATUS.md) and [REQUIREMENTS-STATUS.md](REQUIREMENTS-STATUS.md); original observations below are retained as dated evidence.

# QA report — 0.2.0-alpha.1

Recorded 2026-10-11. This is an alpha evidence report. Windows GUI/microphone and Bengali accuracy acceptance are pending.

| Check | Actual outcome | Scope |
|---|---|---|
| Core unit tests | 29 passed, 0 failed | Linux execution of shared session, text, model, audio, storage and timestamp code |
| Core static checks | Rustfmt; Clippy all targets/no-deps with warnings denied passed | Linux core/CLI; upstream dependency warnings retained |
| Windows release build | Successful x86_64-pc-windows-gnu cross-build | Compiled native UI, WaveIn, insertion/Undo and manifest; no Windows execution |
| Windows import inspection | Only Windows system DLLs; no compiler-runtime DLL | PE inspection, not disconnected installation or traffic capture |
| Local model inference | 7 speech requests completed, including preview/final and fresh worker after kill | Upstream English JFK sample; no Bangla accuracy score |
| Silent audio | 500 worker requests returned empty text | Synthetic zeros; not 500 desktop sessions or noisy-silence corpus |
| Invalid prompt | Error then successful next request | Pipe worker rejects embedded NUL; no decoder panic |
| Cancellation/recovery | In-flight child killed; replacement child transcribed | Worker process test plus unit-tested stale-result rejection; no mic/GUI runtime proof |
| Default persistence | History-off append left no files; worker working directory empty | Unit/integration evidence, not comprehensive Windows filesystem trace |
| Memory/network resource inspection | Inconclusive/unavailable | Descriptor readlink denied; /proc RSS implausible; no valid RAM or packet-capture claim |
| Source/dependency/model package integrity | SHA256 manifest and archive checks supplied | Integrity checks, not publisher signature or legal certification |

Raw evidence is in `docs/validation/`. Worker request wall times include serialization/model verification and virtual host scheduling. They are not product cold/warm latency or benchmark measurements. Model load_ms excludes the preceding integrity verification. The speech sample is not redistributed in the packages; developers can provide a local rights-cleared 16kHz mono PCM WAV to rerun the smoke script. The tested sample came from the upstream whisper.cpp samples/jfk.wav development fixture.

## Known implementation gaps and limitations

Browser/custom editor UI Automation insertion, trained VAD/end-point handling, complete onboarding/UI-language translation, high quality anti-alias resampling and other desktop platform adapters remain unimplemented. Unknown editors have explicit Copy fallback. WAV read is synchronous. Live preview is experimental and may increase final-result delay on slow CPUs. History is plaintext, opt-in, capped at 500 entries and has no app encryption. The bundled Small model is a candidate, not a benchmark-selected final default. GNU Windows loading temporarily buffers the model to support Unicode paths.

All Windows device/permission/callback behavior, focus races, SendInput/UIPI, owned Undo, DPI/theme/screen-reader layout, installer/startup/upgrade/rollback, resource limits, long runs, network capture and Bengali recognition quality require actual execution. Cross-compilation does not validate them. No known destructive insertion is claimed absent testing; the code blocks unverified targets, but a universal safety guarantee cannot be inferred from source alone.

See REQUIREMENTS-STATUS.md and WINDOWS-QA.md for concrete completion work. The PRD stable-release checklist is **not signed off**. No claim of 100% recognition, universal textbox support or complete V1/V2 stable delivery is made.
