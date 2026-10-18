# Bengali diagnostic corpus

This is a **30-utterance development smoke test, not a release acceptance set**.
Audio is not stored in git. `corpus.csv` freezes references, relative audio paths,
individual SHA-256 hashes, sample counts and prompt IDs before this run. See
`provenance.json` for the exact dataset revision and selection method. References
and derived evaluation material are attributed to Google FLEURS, CC BY 4.0:
https://huggingface.co/datasets/google/fleurs

The first 30 WAV archive members contain 28 distinct prompts. Speaker IDs are
unavailable. These are read Indian Bengali, with earlier first clips used in
debugging. They do not establish Bangladesh conversational, mixed-language,
noise, names, numbers, silence or independent holdout performance. The complete
upstream archive was not downloaded or verified; individual selected WAVs were.

## Reproduce

Restore the WAVs from the pinned `bn_in/test` archive to the paths listed in
`corpus.csv`. Verify every audio hash before inference. Build the app with the
recorded compiler/CPU options and obtain the exact local model files in `models/`.
The app and benchmark perform no download and no cloud recognition.

```sh
python3 scripts/benchmark-worker.py /absolute/path/to/voicetotext \
  /absolute/path/to/model.bin benchmarks/fleurs-bn-smoke-30/corpus.csv \
  /absolute/path/to/new-output --cpu-profile 'record your actual build and CPU'
```

Run one ASR process at a time. The current runner locks out concurrent copies.
Each run uses a reusable worker/model, with the first request marked cold. WAV
I/O, inference and IPC are inside request wall time; this is not Windows
microphone-to-insertion latency. Errors stay in the denominator as empty output.
Raw WER and NFC/punctuation-normalized WER remain separate. CER counts Unicode
scalar values, including normalized spaces, not grapheme clusters. WER can exceed
100%; it is not an application accuracy percentage. The script flag allows Latin
text and does not detect Latin-only transliteration, omissions or spelling errors.

## Bengali fine-tune conversion

Source: `bangla-speech-processing/BanglaASR`, pinned revision and file hashes in
`models/ggml-bangla-asr-small-q5_1.manifest.json`. Upstream describes an MIT-licensed
Whisper Small fine-tune. Its published scores are not our measurements.

The conversion needs local `config.json`, `vocab.json`, `model.safetensors`, NumPy
and `safetensors==0.5.3`. It runs no pickle or model-supplied Python. The original
Small GGML file supplies mel filters and vocabulary; dimensions and all 50,257
base token IDs must match. 479 tensors are mapped using whisper.cpp v1.7.6's
official conversion layout, followed by its Q5_1 quantizer. No new training.

```sh
python3 scripts/convert-bn-safetensors.py /local/pinned-hf-folder \
  /local/ggml-small-q5_1.bin /local/bangla-f16.bin
/local/whisper-quantize /local/bangla-f16.bin /local/bangla-q5_1.bin q5_1
```

The fine-tune's pinned `generation_config.json` specifies `return_timestamps:false`.
The app's historical decoder uses timestamps. The explicit `--no-timestamps`
benchmark option tests this difference and marks segment timing unreliable; it
does not change the Windows app's default. A setting is not accepted from its
model card alone.

## Runtime and qualification

Weights and ONNX research models are not shipped by this commit. Manifests with
`evaluated:false`/`qualified_default:false` are source and integrity records,
not accepted model packs. `benchmark-zipformer.py`, if used, is an external
research comparison with sherpa-onnx 1.13.8; it is not the app's Whisper worker.
No candidate is qualified for stable Bengali dictation by this smoke test.

Complete results and experiment limitations are recorded in `DECISION.md`.
