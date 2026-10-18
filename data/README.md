# Local Bengali dataset workspace

Original downloads belong in sources/{openslr53,bangla10k,commonvoice,indicvoices,fleurs}/.
Keep original manifests and audio; normalization writes new processed/audio files.
Manifests/master.jsonl, train.jsonl and dev.jsonl are generated records, not fabricated samples.
No holdout is finalized during development. No collected Bangladesh-owned recordings exist yet.

Raw audio, speaker records, intermediate manifests and checkpoints are ignored by Git.
Track source URLs/revisions, license evidence, original file hashes, transformation versions,
exclusions and split audit summaries in datasets/ and experiment configs instead.
Previously used FLEURS-30 and Zipformer demos are regression-only, forbidden in training/holdout.
Unknown speakers and unresolved licenses are quarantined; do not random-split utterances.
