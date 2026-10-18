# Bengali model investigation — 18 October 2026

Investigation in progress. No stable model is selected. This diagnostic set is
not an independent holdout and cannot certify overall accuracy.

| Complete run | Normalized WER | Scalar CER | Worker errors | Unexpected-script flags |
| --- | ---: | ---: | ---: | ---: |
| Original Small Q5_1, timestamp mode | 108.71% | 97.78% | 2 / 30 | 23 |
| Bengali-fine-tuned Small Q5_1, timestamp mode | 77.58% | 68.99% | 1 / 30 | 0 |

Both complete runs are rejected as evidence for a stable Bengali default.
Zero script flags does not imply correct Bengali: omissions, spelling mistakes,
punctuation-only responses and Latin transliteration require separate checks.
Failed responses remain empty hypotheses in WER/CER. Baseline app source is
`d2db7abbb339fea6137573cecdee8e95a8fcb0cd`; exact binary/model hashes are in each
run's configuration. These two runs used the historical timestamp-enabled final
decoder (beam 5, temperature 0 with fallback increment 0.2), forced `bn`, no
translation, no prompt and no previous context.

The Turbo run is being completed. A parallel no-timestamp three-clip diagnostic
and a raw UTF-8 segment diagnostic caused severe CPU contention. All three
no-timestamp clips timed out. That experiment is **confounded**, not evidence
that disabling timestamps helps or harms accuracy. The interrupted diagnostic
did not return segment data. The Turbo run's affected timeout rows will remain
visible, with this limitation. Timing is exploratory on a shared Linux CPU,
not a Windows latency gate. The runner now rejects concurrent benchmark copies.

Next: repeat decoder diagnostics sequentially and compare a compact Bengali
Zipformer using the same audio as an external research engine. It is not yet
integrated into the app and cannot be shipped as an accepted engine from a
development smoke test.

Bangladesh conversational/mixed/noisy speech, silence hallucination, numeric
entities, names, correction burden and Windows insertion remain unmeasured.
`benchmarks/bn-target/` contains prompts and 204 planned takes from 12 planned
development speakers, with **zero collected recordings or consents**. The
PRD's 200-utterance/12-speaker gate and separate 1,000/50 holdout remain open.
