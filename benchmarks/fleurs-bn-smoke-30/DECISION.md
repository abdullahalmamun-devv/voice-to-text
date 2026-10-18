# Bengali model investigation — 18 October 2026

**Decision: prioritize a native Zipformer integration experiment for Bengali. No model is qualified for stable release or 100% accuracy.** The 94,119,939-byte Zipformer pack is the most promising measured candidate here. It is currently an external research engine, not part of the desktop app.

## Complete 30-utterance observations

| Run | Normalized WER | Scalar CER | Errors | Script flags | Decision |
| --- | ---: | ---: | ---: | ---: | --- |
| Original Small Q5_1, app timestamp decoder | 108.71% | 97.78% | 2 / 30 | 23 | Reject as Bengali default |
| Bengali-fine-tuned Small Q5_1, app timestamp decoder | 77.58% | 68.99% | 1 / 30 | 0 | Reject as Bengali default |
| Large-v3-turbo Q5_0, confounded run | 89.52% | 64.46% | 12 / 30 | 0 | Inconclusive; do not rank from this run |
| Bengali Zipformer, sequential external engine | **17.74%** | **5.38%** | **0 / 30** | **0** | Advance to integration experiment |

Failed responses remain empty hypotheses in WER/CER. Raw WER, transcripts, references, per-clip timings and model/audio hashes are retained. Zipformer's raw WER is 30.31%; it emits no automatic punctuation. Normalization changes punctuation/whitespace/NFC only, not spelling, digits or word choices. Script flags permit Latin and miss transliteration and omissions. Zero flags does not imply correct Bengali.

This is a development smoke set of read Indian Bengali: 30 clips, 28 prompt IDs, unknown speaker count, earlier clips used for debugging. It is not an independent holdout or Bangladesh conversational/mixed/noisy acceptance set. The proposed clean-speech normalized WER target is 10%; **17.74% does not pass that target**. There is no blanket application accuracy claim.

## Settings and diagnostic findings

The initial app source is d2db7abbb339fea6137573cecdee8e95a8fcb0cd. Whisper runs use forced bn, final beam 5, temperature 0 with fallback increment 0.2, no translation, prompt or previous context, six CPU threads at most, and the build options in environment.json. The two Small baselines use one unchanged binary.

The Turbo run contains 9 invalid-text responses and 3 timeouts. Rows 15–17 overlapped oversubscribed diagnostics; timeouts cannot be attributed to model quality. After row 15's timeout, restarted workers loaded a rebuilt binary:
- Rows 1–15: SHA-256 9b57b2689fe4cb23c4c2636e4570dbb434ee6869794ad5461f8f391b58412169.
- Rows 16–30: SHA-256 977c7592d0d1b30eaedc0c53f1073af827c4b205ec9f47c0b737a5e495ded87f.

The initial run-config describes the initial binary only. Both decoder profiles use timestamps, but this mixed-binary run is not a controlled comparison. All observations are preserved. The runner now rejects concurrent copies and refuses to restart with a changed binary.

The fine-tune's source generation config has return_timestamps:false. A first parallel three-clip test timed out and is confounded. Repeating those same three clips sequentially (bn-small-no-ts-3-serial) finished without timeouts, but two returned invalid text. No timestamp setting is promoted from that result. Valid no-timestamp responses explicitly mark timing unreliable; the desktop default is unchanged.

A separate sequential raw-byte diagnostic on original Small/first clip returned one invalid UTF-8 segment; concatenating its raw bytes remained invalid. This case is not fixed by joining segment fragments. utf8-diagnostic-serial.json retains that evidence. Reproduce with:

```sh
cargo run --release --example inspect-segments -- MODEL WAV
```

The app continues to reject invalid UTF-8 rather than replace bytes and present corrupted text as valid recognition.

Zipformer uses sherpa-onnx 1.13.8, CPU/2 threads, modified beam search/4 paths, endpoint detection disabled, no LM/hotwords, one new stream per original WAV, and a fixed 0.5s silent tail. Runtime/model hashes are pinned. On this shared Linux host its warm decoding requests have median 516 ms and P95 955 ms; median decode RTF is 0.0402. These are indicative external-engine timings, not Windows microphone-to-insertion latency or a controlled cross-engine speedup. Recognizer loading is reported separately. All 30 final rows/report counts were checked after completion. Atomic, flushed checkpoints protect future runs.

## Concrete next implementation

1. Add an experimental native sherpa-onnx C API worker behind an explicit local model profile. Keep Python out of the shipped app. Pin and verify every model and runtime file, package Apache-2.0 notices, preserve process cancellation, and make the benchmark exercise the actual app worker. No runtime download.
2. Re-run this identical corpus through that worker and verify the integration preserves transcripts and errors. Measure Windows memory, microphone latency, cancellation and actual insertion on supported hardware/apps.
3. Collect/freeze the PRD's Bangladesh-focused 200-utterance/12-speaker development set, then keep the independent 1,000/50 holdout separate. Test natural mixed English, negation, 200 numeric entities and 200 names/terms, 200 silence clips, noise and correction burden. Assess punctuation separately; do not silently rewrite names, amounts or negation to improve a score.

The candidate's 500-token vocabulary has no Latin letter tokens except special tokens. English terms may be transliterated or missed; preserving Latin English needs another model/profile and mixed-language testing. It cannot yet serve as the sole bilingual default.

benchmarks/bn-target/ contains prompts and 204 planned takes from 12 planned speakers, with **zero collected recordings or consents**. These gates remain open. Stable V1/V2, a complete offline Windows installer, and other OS desktop support are not delivered or certified by this investigation.
