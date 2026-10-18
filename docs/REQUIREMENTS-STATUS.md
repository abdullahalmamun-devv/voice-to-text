# PRD implementation and release status

Status recorded 2026-10-11 for 0.2.0-alpha.1. “Implemented” means present in source and, where indicated, core-tested; it does not mean Windows runtime acceptance. The supplied PRD remains authoritative. V1 and V2 feature groups share one build; V2 opt-ins default off.

| PRD area | Present in this alpha | Remaining before a stable claim |
|---|---|---|
| Offline/free/source | Native UI, local CPU worker, no product HTTP/server/account/updater, corresponding vendored source and notices | Real Windows process-tree network capture; public source/tag publication and distribution/signing decision |
| Offline package | Windows GUI PE, local model+manifest, system DLL imports only, portable/per-user installer recipes | Fresh disconnected Windows installation, install/update/rollback and unsigned-binary UX tests |
| Mic/shortcut/session | Selection, hold/toggle, 120s cap, meter, cancel, stale-result IDs, lock/suspend handlers | Device/permission-loss/sleep/lock QA; callback fault paths and 500 actual desktop cycles |
| ASR/model selection | Multilingual Small Q5_1 candidate; CPU decoder with bn/en/auto and vocabulary context | Gate 0 Small vs Large-v3-turbo comparison; chosen default acceptance; regional Bangla/mixed/quiet/noisy corpus evaluation |
| Text processing | Exact dictionary, conservative supported integer amounts/digits, protected terms, explicit formatting; core regression tests | PRD full normalization/critical-entity corpus and correction-effort evaluation; unsupported number/date patterns remain literal |
| Result recovery | Review/edit/copy/retry/discard, memory-only audio, held results | Real interrupted-flow and UI task testing |
| Native insertion | Focus/process/control/caret/content checks, selection/password/read-only blocks; Unicode delivery and verified owned Undo | 500 nominal insertion trials and 50 guard scenarios; race/UIPI/password coverage, native control variants |
| Browser/general editor insertion | Safe Copy/manual-paste fallback | **Implementation gap:** UI Automation/browser/custom-editor adapters; browser direct insertion is not implemented |
| Clipboard | Only explicit Copy writes clipboard; automatic insertion avoids clipboard | Unicode/formats/contention and OS clipboard-history QA; no temporary clipboard restore mechanism is used |
| Tray/UX/theme/DPI | Four native pages, visible recording surface, Nirmala UI font, DPI scaling, themes and first-use instructions | **Implementation gaps:** complete onboarding wizard and independent fully translated UI-language selector; visual/accessibility/keyboard/contrast/screen-reader tests |
| Settings/recovery | Schema validation, atomic JSON, previous-file backup, manual imports/exports, opt-in startup | Windows recovery/upgrade persistence and startup path/multi-account testing |
| V2 drafts | In-app preview every 5s when worker free; never commits drafts to an external field | Stable phrase policy and measured preview quality/latency; microphone and cancellation interplay QA |
| V2 correction/snippets/editing | Explicit confirmed exact phrase correction; snippets; two app-owned delete commands | Broader bounded command set if required; real utterance recognition and correction UX tests |
| V2 history | Opt-in transcript-only local JSON, 7/30/manual retention, capped 500 entries, Copy/export/Delete all | At-rest protection decision and OS permission QA; no encryption/secure-erasure guarantee |
| V2 WAV/export | PCM/float WAV ≤10min/250MB, TXT/Markdown/raw timed SRT | File reading currently synchronous; linear resampling lacks a high quality anti-alias filter; wider formats excluded; timing validation with real corpus |
| Silence/VAD | Conservative RMS gate and decoder no-speech segment rejection | **Implementation gap:** trained VAD/end-point policy and real noise/quiet speech/silence hallucination evaluation |
| Platform expansion | Windows desktop; Linux CLI/worker only | macOS/Linux desktop adapters/packages/permissions/display-server matrices not implemented (PRD V1.1) |
| Benchmark/QA handoff | Offline runner, 29 passing core tests, worker smoke report, manual test register | Rights-cleared Bengali corpus; 200 utterance/12 speaker prototype set and 1000 utterance/50 speaker holdout; all required stable metrics |

Unverified targets include the proposed WER/CER, critical numeric/name accuracy, ≥99% nominal insertion success, cancellation/latency/memory limits and zero destructive wrong-target insertion rate. There is no evidence to mark PRD §31 as passed. A release owner must execute the remaining gates and record their results; this alpha is a concrete implementation and developer handoff, not that sign-off.
