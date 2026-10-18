# Native Bengali integration evidence — 18 October 2026

[GitHub Actions run 39151584968](https://github.com/abdullahalmamun-devv/voice-to-text/actions/runs/39151584968)
passed both jobs for PR head `4c9d0681dd0851ab03446bbf747c711f5e69f592`.
The checked-out PR merge revision was `d155094b2f85fa82078fa1fa9bbf118ee86cdd2e`.

This is native app-worker inference through the pinned sherpa-onnx 1.13.8 C ABI,
not the external Python research engine. Setup/benchmark scripts are development
tooling; Python is not shipped as an app runtime.

| Check | Linux x64 | Windows x64 |
|---|---|---|
| Rust library tests | 39 passed | 39 passed |
| Pinned C ABI probe / release build | Passed | Passed with MSVC |
| Native real-WAV, silence/recovery, unsupported-mode guards | Passed | Passed |
| Busy worker cancellation, no late result | 15 ms | 13 ms |
| Frozen FLEURS clips / decoder errors | 30 / 0 | 30 / 0 |
| Exact hypothesis agreement with research baseline | 30 / 30 | 30 / 30 |
| Normalized WER / scalar CER | 17.74% / 5.38% | 17.74% / 5.38% |
| Raw WER | 30.31% | 30.31% |
| Warm request median / P95 | 449 / 806 ms | 852 / 1579 ms |

Linux formatting, Clippy all targets and ten Python tests also passed. Windows
compiled the GUI test target; no desktop interaction or microphone test is implied.
Two inference threads, CPU, four reported runner CPUs. Timings depend on host
scheduling and do not establish minimum-device latency or a cross-host speed ranking.
Cancellation is one synthetic busy-worker smoke per platform, not 100 PRD trials.

## Scope and provenance

The same [frozen development corpus](../../fleurs-bn-smoke-30/provenance.json) is
Google FLEURS bn_in/test, CC-BY-4.0, revision
`70bb2e84b976b7e960aa89f1c648e09c59f894dd`. Attribution and selection details
remain with that study. It contains read Indian Bengali, 30 clips/28 prompt IDs,
no speaker-count claim and earlier debug exposure. This is not an independent
holdout, Bangladesh conversational, mixed-English, noise or entity acceptance.
No raw audio or model weights are committed.

The original locally recorded corpus CSV hash is
`70c16cf52dd141c4e6d1e8aad4f0b0caa709c1a478acd963f469c5de6f14b82c`
(CRLF). The repository/these restored manifests use LF, hash
`3c8b0b412fd9251fb09e8e7671b6c7ef382c3b379a971781e82cdff0a0578a9c`.
The CRLF-to-LF transformation was independently checked; references, IDs and
individual WAV hashes are unchanged. Follow-up setup pins canonical LF bytes
before restoring the individually SHA-verified WAVs. The full source archive is
not claimed downloaded/verified.

Linux inference ran in a disconnected network namespace. Windows used an explicit
outbound program rule. Neither is a comprehensive Windows process-tree packet
capture or privacy certification. Both loaded the model from a Bengali-named folder.

## Saved evidence

Each platform directory preserves results.csv, report.json, run-config.json,
integration-parity.json and smoke.json extracted without altering measured values
from the job output. Individual raw IPC responses remain in the workflow's
verification artifacts while retained by GitHub. Artifact digests, IDs and job
steps are in run-info.json. The Linux config's source_revision is null because
sudo removed that environment variable; the tested checkout is identified above,
and the follow-up workflow preserves it.

The [Windows experimental artifact](https://github.com/abdullahalmamun-devv/voice-to-text/actions/runs/39151584968/artifacts/12145631778)
contains the executable, pinned model/runtime, notices, guide and manifest. It is
unsigned, needs the Microsoft v14 runtime and is not clean-machine installer QA.
GitHub artifact access/retention rules apply.

## Release decision

**Experimental only.** The proposed clean normalized-WER target is 10%, so 17.74%
does not qualify. No 82.26% general "accuracy" conversion is made. The model's
English-letter vocabulary, automatic punctuation, decoder dictionary hints and
reliable segment timing are unsupported. It requires Bengali mode, explicit
review and TXT/Markdown rather than SRT. All stable PRD data, UI/hardware, entity,
insertion, resource and extended-run gates remain open.
