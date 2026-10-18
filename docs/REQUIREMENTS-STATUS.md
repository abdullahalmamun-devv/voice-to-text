# PRD implementation and release status — alpha.3

Updated 18 October 2026. "Present" describes source or the specific automated
evidence below; it does not sign off the PRD. V1 and optional V2 features share one
Windows application. History, drafts, commands and startup remain off by default.

| PRD area | Present / verified | Remaining for stable release |
|---|---|---|
| Offline source and package | Native Win32/Rust worker; vendored source; no product network client, server, account or updater. Experimental Bengali package contains pinned weights and native CPU libraries | Disconnected clean-machine installation, signing/distribution decision and actual Windows process-tree traffic capture |
| Bengali recognition | Pinned native Zipformer candidate; 30 development clips exactly match research outputs; normalized WER 17.74%, scalar CER 5.38%, zero decoder errors | Clean WER target 10% unmet; Bangladesh conversational, mixed/noisy speech, numeric/name sets and independent holdout not qualified |
| Language/model selection | Multilingual Whisper bn/en/auto retained. Experimental native profile is explicitly selected, Bengali-only and always reviewed | No stable default qualified; native English-letter vocabulary, automatic punctuation and decoder hints unsupported |
| Audio/session | Anti-alias resampling, cancellable WAV reads, microphone choice, hold/toggle shortcut, cancel/stale-result guards, 120s mic cap | Real microphone/device/permission/sleep/lock tests, measured endpoint/VAD policy and desktop cycles |
| Native runtime | Hash/version-pinned C API, independently checked ABI, exact dependency allowlist and restricted Windows library loading | Broader device/runtime coverage; CI runner success is not fresh-machine acceptance |
| Text processing | Exact aliases, preferred terms for Whisper, conservative integer formatting, explicit formatting/snippets and confirmed corrections | Full entity/normalization corpus and correction-effort evaluation. Native profile rejects unsupported vocabulary prompts explicitly |
| Result recovery | Review/edit/copy/retry/discard; RAM-only captured audio. Experimental results cannot automatically insert | Interrupted UI flows and hardware/resource checks |
| Native insertion/Undo | Focus/process/control/caret/content guards; supported Unicode Edit/RichEdit delivery and owned Undo | 500 nominal insertion trials, 50 guard cases, race/UIPI/password/control-variant validation |
| Browser/custom editors | Manual Copy and paste fallback | UI Automation/browser/custom-editor adapters remain unimplemented |
| Clipboard | Explicit Copy only; automatic insertion avoids clipboard | Format/contention and OS clipboard-history QA |
| UI/onboarding | Native pages, recording indicator, fonts, theme/DPI and setup guides | Complete independent Bengali/English UI selector, onboarding wizard, screen-reader/keyboard/contrast tests |
| Settings/history | Atomic settings + backup; manual import/export; transcript-only opt-in capped history and retention | Windows recovery/startup/upgrade checks and at-rest protection decision; no encryption/secure-erasure guarantee |
| V2 drafts/commands | Experimental in-app draft, snippets and bounded app-result deletion; no external draft insertion | Recognition and latency/preview-policy qualification |
| WAV/export | PCM/float WAV within limits, cancellable reads, TXT/Markdown and audio-bound raw timing checks | Native Bengali has no verified segment timing and refuses SRT; broader formats excluded |
| Silence/VAD | Conservative energy gate, no-speech handling and synthetic silence/recovery smoke | Trained/evaluated VAD and 200 real non-speech/noise/quiet clips |
| Automated checks | 39 Rust tests on Linux and Windows; Linux formatting/Clippy and 10 Python tests; MSVC/native ABI and release builds | Does not cover Windows UI, mic, actual target insertion or the full PRD performance/resource matrix |
| Other desktops | Linux CLI/worker | Linux and macOS desktop adapters, packages and permissions/display-server QA remain unimplemented |

See [PROJECT-STATUS.md](PROJECT-STATUS.md), [EXPERIMENTAL-BENGALI.md](EXPERIMENTAL-BENGALI.md)
and the frozen [investigation](../benchmarks/fleurs-bn-smoke-30/DECISION.md) for exact
evidence and limitations. The planned bn-target recording takes contain no collected
speaker recordings/consents.

PRD gates remain open: 200 utterances/12 speakers, independent 1000/50 holdout,
200 non-speech clips, numeric/name evaluation sets, 500 insertion trials, 50 guard
cases, 100 cancellations and an eight-hour run. Neither 100% ASR accuracy nor
universal textbox compatibility nor completed stable V1/V2 is claimed.
