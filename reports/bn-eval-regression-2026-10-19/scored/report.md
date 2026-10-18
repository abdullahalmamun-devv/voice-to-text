# Bengali ASR evaluation

Evaluation role: **regression**. Text field: **text**.

These are corpus error rates, not 100-minus-WER application accuracy.
Null/unannotated metrics are not measured, never perfect scores.

| Model | N | Norm WER | Raw WER | Scalar CER | Variant WER | Errors |
|---|---:|---:|---:|---:|---:|---:|
| zipformer | 30 | 17.74% | 30.31% | 5.38% | 17.74% | 0 |
| native_windows | 30 | 17.74% | 30.31% | 5.38% | 17.74% | 0 |
| native_linux | 30 | 17.74% | 30.31% | 5.38% | 17.74% | 0 |

## Confidence and paired comparisons

Bootstrap unit: utterance; units: 30; resamples: 2000; seed: 0.

Unknown speakers: utterance resampling does not establish speaker independence.
- zipformer normalized-WER 95% CI: 13.56% to 22.05%.
- native_windows normalized-WER 95% CI: 13.56% to 22.05%.
  Paired delta vs zipformer: 0.00 percentage points; 95% CI 0.00 to 0.00 points.
- native_linux normalized-WER 95% CI: 13.56% to 22.05%.
  Paired delta vs zipformer: 0.00 percentage points; 95% CI 0.00 to 0.00 points.

## Critical and Bengali-specific metrics

| Model | BN-script WER proxy | Number | Money | Negation | Name | Latin recall | Non-speech FAR |
|---|---:|---:|---:|---:|---:|---:|---:|
| zipformer | 18.03% | not measured | not measured | not measured | not measured | 0.00% | not measured |

zipformer annotation denominators: number=0, money=0, negation=0, name=0. Non-speech valid=0, failed=0. Wrong-script=0, invalid-UTF8=0, empty speech=0, runaway=0.
| native_windows | 18.03% | not measured | not measured | not measured | not measured | 0.00% | not measured |

native_windows annotation denominators: number=0, money=0, negation=0, name=0. Non-speech valid=0, failed=0. Wrong-script=0, invalid-UTF8=0, empty speech=0, runaway=0.
| native_linux | 18.03% | not measured | not measured | not measured | not measured | 0.00% | not measured |

native_linux annotation denominators: number=0, money=0, negation=0, name=0. Non-speech valid=0, failed=0. Wrong-script=0, invalid-UTF8=0, empty speech=0, runaway=0.

## Definitions and limitations

- Primary normalization: NFC, Unicode punctuation to spaces, whitespace collapse. No digit, case, spelling, name or negation rewriting.
- WER/CER are corpus micro-averages. WER can exceed 100%. CER counts Unicode scalar values including normalized spaces, not graphemes.
- BN-only WER is a Bengali-script token filter proxy, not annotated linguistic segmentation. Mixed-script tokens and numerals are excluded from that secondary proxy only.
- Variant WER is secondary and applies the same frozen one-token mapping to both sides. It never replaces primary WER or critical-span exact match.
- Critical metrics require human-annotated normalized-token spans. No annotations means null. A span fails on any mismatch or insertion at/inside its boundaries; adjacent insertions can conservatively fail a span. This is lexical exact match, not a semantic number parser.
- Negation annotations must cover polarity-bearing phrases for affirmative AND negative cases. Unannotated positive sentences cannot establish negation preservation.
- Latin recall uses per-utterance, case-folded Latin-token multiset overlap; it does not detect all spoken English or transliteration.
- Non-speech FAR requires explicitly labeled empty-reference non-speech. Speech with noise metadata is not non-speech. Failed non-speech requests are reported separately and excluded from valid FAR denominator, never treated as successful silence.
- Wrong-script permits Bengali and Latin; it cannot detect all transliteration/omission errors. Runaway flag: hypothesis tokens > max(50, 4 * max(1, reference tokens)).
- Missing/error/invalid-UTF8 responses retain the reference and are scored as empty hypotheses. Corrupt bytes are not repaired into credited words; unidentifiable JSON rows fail the run.
- latency_ms is supplied end-to-end request timing; decode_ms is separate. The scorer measures neither and makes no cross-host speed claim.
- Bootstrap is paired across models and speaker-clustered when all speakers are known. Small, reused development data and unknown upstream training overlap cannot prove generalization.
- Metadata groups are marginal and joint descriptive summaries without per-group confidence intervals.

## Metadata groups

Per-field and joint groups, including unknown metadata, are in report.json.
Group estimates have no separate confidence/independence claim.
Per-utterance errors, edit counts, metadata and timings are in per_utt.csv.
