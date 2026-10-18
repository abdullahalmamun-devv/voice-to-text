# Project status — 18 October 2026

Source and checkpoints are public at https://github.com/abdullahalmamun-devv/voice-to-text.
The current integration is **0.2.0-alpha.3**, an experimental Windows x64 application.
Neither V1 nor V2 has passed all PRD stable-release acceptance gates.

## Delivered and verified

- Alpha.2 reliability fixes were reconstructed and merged in PR #1: cancellable WAV reads,
  anti-alias filtering, script-mismatch review, audio-bound timing and guarded native
  insertion/Undo. Browser/custom controls still use manual Copy.
- PR #2 preserves the frozen 30-clip Bengali investigation, model/corpus hashes,
  scoring code, transcripts and failed observations. The original Small, Bengali
  fine-tuned Small and confounded Turbo runs did not qualify as stable defaults.
- PR #3 integrates the better Bengali Zipformer candidate directly through pinned
  sherpa-onnx C ABI and ONNX Runtime libraries, inside the cancellable native worker.
  Python is developer/CI tooling, not an application runtime. No cloud inference
  or application-triggered model download is introduced.
- CI run [39151584968](https://github.com/abdullahalmamun-devv/voice-to-text/actions/runs/39151584968)
  passed both Linux and Windows jobs: 39 Rust tests each, Linux formatting/Clippy
  and ten Python tests, independently compiled C ABI probes and release builds.
- Both native workers decoded all 30 frozen FLEURS clips with exact transcript
  parity to the research baseline, loaded models from Bengali-named folders and
  passed real-WAV/silence/recovery/mode-rejection/cancellation smoke. Linux inference
  ran without a network namespace connection; Windows used an outbound program
  rule. A Windows executable + model + CPU runtime package was produced.
- Reports, transcripts, hashes and exact scope are preserved in
  [native integration evidence](../benchmarks/native-bengali-ci/2026-10-18/README.md).
  The earlier Linux report-ownership failure was fixed and this run is green.

## Measured Bengali result and limits

On the same 30 development clips, the Windows native worker measured normalized
WER **17.74%**, raw WER **30.31%** and normalized Unicode-scalar CER **5.38%**,
with zero decoder errors or unexpected-script flags. Warm request median/P95 were
852/1579 ms on that CI runner, not a minimum-device performance guarantee.

These clips are read Indian Bengali with 28 distinct prompt IDs; this is not an
independent holdout, Bangladesh conversational benchmark or mixed-English test.
The proposed clean normalized-WER target is 10%, so the model remains unqualified.
See [the investigation](../benchmarks/fleurs-bn-smoke-30/DECISION.md).

The experimental profile requires Bengali mode and explicit review. It does not
support English-letter vocabulary, decoder dictionary hints, automatic punctuation
or reliable segment timing/SRT. The multilingual model option remains available;
the experimental profile is selected explicitly, never silently promoted.

## Remaining work

- Collect and consent the planned Bangladesh development recordings, annotate them,
  and run speaker-separated development and independent holdout evaluation.
  The 204 planned takes in bn-target are a recording plan, not collected audio.
- Improve punctuation, mixed English, names/numbers and conversational/noisy speech;
  qualify model and decoding choices against the PRD rather than this smoke set.
- Complete bilingual interface/onboarding, evaluated voice-activity detection,
  browser integration and real Windows microphone, focus/insertion, accessibility,
  resource/network and clean-machine installer checks.
- Execute the full gates: 200 utterances/12 speakers, independent 1000/50 holdout,
  200 non-speech clips, numeric/name entity sets, 500 insertion trials, 50 guard
  cases, 100 cancellations and an eight-hour run. CI smoke is not those gates.
- Linux CLI/worker tests do not implement a Linux desktop adapter; macOS desktop
  support is also unimplemented.

## Availability and schedule

The Windows experimental artifact contains the model and native CPU libraries.
It is unsigned and needs the Microsoft Visual C++ v14 runtime; it is not a verified
clean-machine installer. Read [the Bengali guide](EXPERIMENTAL-BENGALI.md).

A stable Windows V1/V2 candidate still requires several active engineering weeks,
speech data and real Windows devices. The earlier provisional 3–6 week range is
conditional, not an elapsed-time promise; model failures or missing evaluation
resources can extend it. No 100% ASR accuracy or final delivery date is claimed.
