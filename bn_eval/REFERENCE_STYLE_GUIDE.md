# Bengali ASR reference and scoring contract — version 1

This is development tooling, not an application runtime dependency.
Python 3.9+ and its standard library run scoring. pytest is test tooling only.
The offline application never invokes dataset download, scoring or training tools.

## Reference truth

Listen to the recording and transcribe what was actually spoken in text.
A reading prompt is not automatically the transcript. Never edit a reference
after looking at model output. Preserve hesitations, dialect, repetitions,
negation and code switching according to the frozen annotation policy.
Use a second human review for critical spans and disputed transcripts.

Use text_written for independently annotated written form, including
punctuation and written numbers. Do not silently convert one into the other.
Run spoken and written evaluation separately with --text-field.
Keep original references, revisions, reviewer changes and audio hashes.

One UTF-8 JSON object per line:

    {"id":"u7","text":"কাল দশ হাজার টাকা পাঠাব না","text_written":"কাল ১০,০০০ টাকা পাঠাব না।","speaker":"s03","region":"sylhet","device":"laptop_mic","noise":"fan","kind":"spont","banglish":false,"entities":[{"kind":"number","start":1,"end":3,"text":"দশ হাজার"},{"kind":"money","start":1,"end":4,"text":"দশ হাজার টাকা"},{"kind":"negation","start":4,"end":6,"text":"পাঠাব না"}]}

Speaker IDs must identify people within the source, not filenames, utterance IDs,
gender, region or prompt IDs. Dataset management namespaces source speaker IDs.
Unknown metadata stays absent/null. Do not infer dialect or speaker identity.
Gender is optional source-provided metadata; it is not inferred from audio.

Silence/noise-only reference:

    {"id":"silence7","text":"","kind":"noise_only","noise":"fan","non_speech":true}

Speech with noise=fan is still speech, not a false-accept test.
Empty references without an explicit non-speech label do not establish silence
coverage. A failed request is not evidence of successful silence rejection.

## Hypotheses

One row per attempted recording, with matching id:

    {"id":"u7","text":"কাল দশ হাজার টাকা পাঠাব না","latency_ms":780,"decode_ms":640}
    {"id":"u8","text":"","error":"timeout"}

latency_ms is the timing supplied by the inference runner, not measured by
the scorer. Declare whether it is WAV request latency or actual user-stops-
speaking-to-final-text latency. Keep decode_ms separate. Missing timings are
null, not zero. Persist the runtime, model/checkpoint, decoder, hardware and
audio provenance alongside every inference run.

Missing hypotheses are retained as empty/error responses: speech deletions count.
Unknown IDs, duplicate IDs/JSON keys and malformed envelopes fail the run rather
than silently disappearing. Invalid UTF-8 can be attributed only if the JSON
envelope and ID remain recoverable; its text receives no repaired-word credit.
Escaped lone surrogates are invalid-Unicode errors, distinct from invalid bytes.

## Normalization

Version: bn-nfc-punctuation-whitespace-v1.

- Unicode NFC; every Unicode punctuation character becomes a space.
- Collapse whitespace; preserve case, spelling, numbers, symbols and negation.
- No automatic number parsing, digit conversion, transliteration or name fixes.
- Raw WER uses whitespace tokens without this normalization.
- CER uses normalized Unicode scalar values, including spaces, not graphemes.

The initial primary normalization intentionally matches the historical scorer.
Changing it requires a new version and rerunning all comparisons. Decimal/group
punctuation can split written numbers; therefore written numeric semantics must
not be inferred from WER or the lexical critical-span score.

variants_v1.tsv is a small, frozen, secondary one-token spelling map applied
symmetrically to both sides. It does not change primary WER, references or critical
metrics. Document rationale before adding entries. Do not add semantic synonyms,
names, negation changes or numeric equivalences to improve a model's score.
The loader rejects chains/cycles, duplicate sources, numeric and multi-token maps.
The file's SHA-256 is in every report.

## Critical annotations and limitations

entities uses half-open token indexes AFTER primary normalization, never raw
character offsets. Kinds are number, money, negation, name. An optional
text must exactly match the indexed reference tokens. Overlapping categories
are allowed: the amount may contain an independently annotated number.
entities_written supplies independent spans for written-form evaluation.

Critical exact match uses deterministic edit alignment: every reference token
must match at the corresponding position, and an insertion at or inside either
boundary fails the span. A correct name/number elsewhere does not earn credit.
This conservative metric can count a neighboring unrelated insertion as a
failure; inspect per-utterance output. It is not a semantic numerical-value
parser or a complete entity extractor. Missing annotations produce null and
zero coverage, not 100% accuracy.

For negation, annotate the polarity-bearing verb phrase in BOTH affirmative
and negative sentences, e.g. যাব and যাব না. This catches additions as well
as omissions at those spans. Include distinct positive/negative examples;
unannotated affirmative sentences cannot establish negation preservation.

The secondary bn_only_wer is explicitly a script-filter proxy: retain tokens
containing Bengali letters and no Latin letters. It cannot identify the true
linguistic language of transliterated terms. Primary WER always retains all
tokens. Latin recall uses per-utterance, case-folded token-multiset overlap;
it does not establish recognition of all spoken English.

Wrong-script flags allow Bengali and Latin. They do not detect missing words
or all transliterations. Runaway threshold is > max(50, 4 × reference-token
count, using at least one reference token). Both flags are diagnostics.
Error rates, raw transcripts, failure counts and denominators remain visible.

## Confidence and comparisons

WER/CER are corpus micro-averages, including failed speech deletions and extra
hypothesis insertions; WER can exceed 100%. For an all-empty-reference corpus,
WER/CER denominator zero is null; use explicit non-speech false-accept rates.

Bootstrap defaults to 2,000 resamples, fixed seed 0, percentile 95% intervals.
When ALL speaker IDs are known it resamples speaker clusters. Otherwise it
resamples utterances with an independence warning. --bootstrap-unit speaker
refuses incomplete speaker metadata. The same sampled units apply to every
model; paired delta is candidate minus first model, so negative is better.
Error and missing responses are not removed from the pairs.
Small/reused development corpora cannot prove release performance.
Group breakdowns are descriptive, without separate group confidence intervals.

## Commands

    python -m bn_eval.score --ref data/dev/ref.jsonl --hyp zipformer=out/zipformer.jsonl --hyp candidate=out/candidate.jsonl --group-by region,device,kind,noise --variants bn_eval/variants_v1.tsv --out reports/new-run
    python -m pytest bn_eval/tests -q

Outputs: report.md, report.json, per_utt.csv. Output directories must be
new; existing results are never overwritten. Hashes cover input files, variant
table and scorer source. No wall-clock timestamp/random seed is invented.
The role (training, development, regression, holdout) is an explicit
declaration, not automatic certification of split integrity.

The historical FLEURS 30 stay regression/development ONLY. Do not train on
them, place them in final holdout or label them newly collected Bangladesh audio.
The Zipformer demo clips are also previously observed. Synthetic unit-test
texts/signals are not human recordings or empirical model-quality results.
