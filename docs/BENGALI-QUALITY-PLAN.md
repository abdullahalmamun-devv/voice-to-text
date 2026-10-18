# Bengali recognition first

Status: 18 October 2026. Bengali recognition is the primary product gate. UI expansion is secondary until local speech quality is demonstrated. No current model has passed stable acceptance.

Measured update: the external Bengali Zipformer candidate completed all 30 frozen clips with 17.74% normalized WER, 5.38% scalar CER and zero decoder errors. It is the next native integration experiment, not an accepted app engine. The clean 10% normalized-WER target is still unmet. The Small baselines failed; the Turbo run is confounded, and timestamp ablation did not establish a fix. See [full evidence and decision](../benchmarks/fleurs-bn-smoke-30/DECISION.md).

## Immediate comparison

Run the same final app decoder, CPU profile and frozen audio manifest against:

1. Original multilingual Whisper Small Q5_1: compact baseline.
2. Multilingual Large-v3-turbo Q5_0: stronger general candidate.
3. bangla-speech-processing/BanglaASR: existing Bengali-fine-tuned Whisper Small, converted from safetensors to GGML and Q5_1 for the same app runtime. This is conversion of existing weights, not new training.

The initial manifest contains the first 30 WAV members of the pinned Google FLEURS bn_in test archive (28 distinct prompt IDs). Selection precedes this run, with no outcome-based exclusion. It is a diagnostic smoke set: read Bengali/India, speaker IDs unavailable, earlier clips used for debugging. It cannot establish Bangladesh dialect coverage, English code-switching, noise robustness, independent holdout performance or the PRD's 200-utterance/12-speaker gate.

Record model/source/binary/audio hashes, exact decoding settings, raw and NFC/punctuation-normalized WER, scalar CER, errors, wrong-script flags, and cold-first versus warm-worker timings. Retain failed utterances as errors/deletions. A WER is not converted into a blanket application accuracy percentage. Check punctuation, negation, names and amounts independently; no spelling/number rewriting to hide recognition errors.

## Decision rules

- A model producing wrong scripts or invalid output cannot be accepted merely to meet a package-size target.
- Compare the Bengali fine-tune before committing to a different runtime or training. Test English/mixed speech separately before making it the sole default.
- Prefer quality over an unmeasured lightweight claim. Publish actual bytes and indicative CPU times; qualify Windows hardware independently.
- Review remains on; uncertainty/script flags cannot certify correctness. No remote fallback or automatic model download.
- If all candidates fail, record failure and evaluate another local candidate; do not mark the core feature complete.

## Next acceptance set

Freeze a rights-cleared Bangladesh-focused set of at least 200 utterances/12 speakers: natural colloquial speech, Bangla-English mixed terms, negation, names/amounts/digits, quiet speech and laptop noise. Keep development speakers separate from the PRD's 1,000-utterance/50-speaker holdout; add at least 200 silence/non-speech clips and labelled critical entities. Public read speech is a supplement, not a substitute.

Use actual Windows microphone recordings and correction effort, then optimize the accepted model's local execution. Warm model reuse, measured CPU/SIMD configuration and optional GPU paths can improve speed without silently changing the quality gate. Trained VAD and dictionary hints require controlled A/B tests, especially for quiet speech and names.

## Deliverable for this work session

Publish the frozen manifest/provenance, reusable local benchmark and conversion tools, complete results for the compared models, and a model decision with remaining gaps. Keep audio and large weights out of git; provide pinned sources/hashes. Stable Bengali support and 100% recognition are not claimed from a small smoke set.
