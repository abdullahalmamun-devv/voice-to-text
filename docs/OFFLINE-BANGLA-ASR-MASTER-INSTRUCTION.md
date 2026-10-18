# MASTER INSTRUCTION — OFFLINE BANGLA ASR IMPROVEMENT

## Project Goal

আমাদের `VoiceToText` project-এর প্রধান লক্ষ্য হলো:

**সম্পূর্ণ offline, local, Windows-compatible Bangla Speech-to-Text system তৈরি করা**, যা বিশেষভাবে বাংলাদেশের বাস্তব কথ্য বাংলা, regional accent, Bangla-English mixed speech, নাম, সংখ্যা, টাকার অঙ্ক, negation এবং noisy environment-এ যতটা সম্ভব নির্ভুলভাবে কাজ করবে।

কোনো cloud transcription API ব্যবহার করা যাবে না।

বর্তমান Bengali Zipformer experimental baseline:

- Normalized WER: প্রায় `17.74%`
- CER: প্রায় `5.38%`
- বর্তমান 30-audio benchmark: Google FLEURS `bn_in/test`
- এই corpus Bangladesh conversational benchmark নয়।
- Current model experimental; production-ready নয়।

আমাদের লক্ষ্য কেবল existing model test করা নয়।

আমাদের কাজ হবে:

**Dataset → Evaluation Framework → Train/Fine-tune → Evaluate → Error Analysis → Improve → Final Holdout → Windows Integration**

---

# PHASE 1 — Existing Work Preserve করো

বর্তমান repository-এর existing benchmark, reports, model integrations এবং test artifacts delete/overwrite করবে না।

বিশেষভাবে preserve করো:

```text
benchmarks/fleurs-bn-smoke-30/
benchmarks/native-bengali-ci/
benchmarks/bn-target/
```

বর্তমানে ব্যবহৃত 30টি FLEURS audio development/regression dataset হিসেবে থাকবে।

এগুলো:

- training-এ যাবে না
- final holdout-এ যাবে না
- নতুন model-এর regression comparison-এ ব্যবহার করা যাবে

Zipformer-এর existing demo:

```text
test_wavs/0.wav
test_wavs/1.wav
```

এগুলোকেও unseen test data হিসেবে ব্যবহার করা যাবে না।

---

# PHASE 2 — `bn_eval` Evaluation Framework Implement করো

Fine-tuning শুরু করার আগে reproducible evaluation framework project-এ থাকতে হবে।

Implement:

```text
bn_eval/
    score.py
    normalize.py
    variants_v1.tsv
    REFERENCE_STYLE_GUIDE.md
    tests/
```

Requirements:

- Python 3.9+
- standard library দিয়ে core scoring সম্ভব হলে সেটাই ব্যবহার করো
- `pytest` tests
- deterministic output
- normalization versioned হবে

Example command:

```bash
python -m bn_eval.score \
  --ref data/dev/ref.jsonl \
  --hyp zipformer=out/zipformer.jsonl \
  --hyp candidate=out/candidate.jsonl \
  --group-by region,device,kind,noise \
  --variants bn_eval/variants_v1.tsv \
  --out reports/<date>
```

Test:

```bash
python -m pytest bn_eval/tests -q
```

Output:

```text
report.md
report.json
per_utt.csv
```

---

# PHASE 3 — Evaluation Metrics

শুধু একটি WER number দিয়ে model বিচার করবে না।

Report করতে হবে:

### Core Recognition

- Normalized WER
- Raw WER
- CER
- bootstrap 95% CI
- paired delta between models

### Bengali-specific

- Bangla-only WER
- variant-tolerant WER
- wrong-script detection
- invalid UTF-8
- empty hypotheses
- runaway output

### Critical Product Metrics

- Number exact-match accuracy
- Money/amount accuracy
- Negation preservation
- Name/entity accuracy
- Latin/English word recall
- Silence/noise false-accept rate

### Metadata Groups

যেখানে metadata পাওয়া যায়:

```text
region
speaker
device
noise
kind
banglish
gender
```

এসব অনুযায়ী breakdown করো।

---

# PHASE 4 — Reference Format

Reference JSONL:

```json
{
  "id": "u0007",
  "text": "কাল দশ হাজার টাকা পাঠাব",
  "text_written": "কাল ১০,০০০ টাকা পাঠাব।",
  "speaker": "s03",
  "region": "sylhet",
  "device": "laptop_mic",
  "noise": "fan",
  "kind": "spont",
  "banglish": false
}
```

### Important

`text` হবে বক্তা **বাস্তবে যা উচ্চারণ করেছে**।

Prompt যা ছিল সেটাকে automatically ground truth ধরা যাবে না।

উদাহরণ:

Prompt:

```text
আমি আগামীকাল ঢাকা যাব।
```

Speaker যদি বলে:

```text
আমি কালকে ঢাকা যাব।
```

Ground truth হবে:

```text
আমি কালকে ঢাকা যাব।
```

Model output দেখে reference কখনো modify করা যাবে না।

---

# PHASE 5 — Bengali Dataset Collection

Training/fine-tuning-এর জন্য একাধিক licensed Bengali dataset সংগ্রহ করো।

Priority:

## A. OpenSLR SLR53

Bangladeshi Bengali হওয়ায় highest priority।

Audio + transcript সংগ্রহ করো।

আগে:

- dataset license verify
- provenance save
- original manifest preserve
- hash generate

---

## B. Bangla-10K

Full dataset অপ্রয়োজনে download করবে না।

যদি streaming/filtering supported হয়:

```text
locale == bn-BD
```

অর্থাৎ Bangladesh Bengali prioritize করো।

যেসব field পাওয়া যায়:

```text
speaker
region
locale
speech type
audio
transcript
```

সব preserve করো।

Natural/conversational এবং Bangla-English mixed samples বিশেষভাবে useful।

---

## C. Mozilla Common Voice Bengali

Bengali audio + transcript সংগ্রহ করো।

Speaker IDs থাকলে অবশ্যই preserve করবে।

Speaker-wise split করার জন্য এটি গুরুত্বপূর্ণ।

---

## D. AI4Bharat IndicVoices Bengali

Bengali config/subset সংগ্রহ করো।

Natural/spontaneous speech থাকলে priority দাও।

---

## E. Google FLEURS Bengali

FLEURS ব্যবহার করলে:

```text
bn_in
```

কিন্তু আমাদের আগে ব্যবহৃত 30টি audio এবং তাদের duplicate/derived copy training বা final test-এ যাবে না।

FLEURS মূলত read speech; Bangladesh conversational evaluation-এর replacement নয়।

---

# PHASE 6 — License Verification

কোনো dataset training-এ ব্যবহারের আগে তার license machine-readable manifest-এ record করতে হবে।

উদাহরণ:

```json
{
  "dataset": "dataset-name",
  "source": "...",
  "revision": "...",
  "license": "...",
  "download_date": "...",
  "sha256": "...",
  "commercial_use_allowed": true
}
```

যদি কোনো dataset:

- non-commercial only
- research-only
- unclear redistribution rights

হয়, production model training-এ automatically mix করবে না।

আগে flag করবে।

---

# PHASE 7 — Dataset Normalization

সব dataset এক format-এ convert করো।

Preferred training audio:

```text
WAV
mono
16 kHz
PCM
```

কিন্তু original audio delete করবে না।

Structure:

```text
data/

  sources/
      openslr53/
      commonvoice/
      bangla10k/
      indicvoices/
      fleurs/

  processed/
      audio/

  manifests/
      master.jsonl
      train.jsonl
      dev.jsonl
      holdout.jsonl
```

Master manifest example:

```json
{
  "id": "bd_000001",
  "audio": "processed/audio/bd_000001.wav",
  "text": "আজকে আমি অফিসে যাব না",
  "speaker": "speaker_0021",
  "region": "dhaka",
  "source_dataset": "openslr53",
  "language": "bn-BD",
  "kind": "read",
  "duration_ms": 4210,
  "sha256": "..."
}
```

---

# PHASE 8 — Deduplication

একই audio যেন multiple dataset/source থেকে training এবং test দুই জায়গায় না চলে যায়।

Deduplicate using:

- SHA-256
- audio duration
- normalized transcript
- acoustic fingerprint if practical

Exact duplicate অবশ্যই remove করো।

Near duplicate flag করো।

---

# PHASE 9 — Train / Dev / Holdout Split

এটি অত্যন্ত গুরুত্বপূর্ণ।

Split **speaker-wise** হবে।

কখনো utterance-wise random split করবে না যদি একই speaker multiple split-এ চলে যেতে পারে।

Rules:

```text
TRAIN speakers
      ≠
DEV speakers
      ≠
HOLDOUT speakers
```

Suggested concept:

```text
Train       → model learning

Dev         → model selection
              decoder settings
              hyperparameters
              checkpoints
              punctuation experiments

Holdout     → final unbiased evaluation only
```

Holdout repeatedly দেখে model tune করা যাবে না।

---

# PHASE 10 — Bangladesh-Owned Dataset

Online dataset যথেষ্ট নয়।

নিজেদের Bangladesh-focused dataset তৈরি করতে হবে।

Existing plan:

```text
12 development speakers
×
17 takes
≈
204 utterances
```

এটি সংগ্রহ করো।

Include:

- Dhaka/general Bangla
- Rajshahi
- Sylhet
- Chattogram
- Barishal
- Rangpur
- Khulna

যতটা practical coverage সম্ভব।

Speech types:

```text
normal conversation
fast speech
slow speech
read speech
spontaneous speech
Bangla-English mixed
```

Environment:

```text
quiet
fan
AC
street
TV/background speech
laptop microphone
phone microphone
USB microphone
```

---

# PHASE 11 — Critical Bengali Test Categories

Dataset-এ আলাদা coverage রাখো।

### Numbers

```text
পনেরো
একশো পঞ্চাশ
দশ হাজার
এক লাখ সাতচল্লিশ হাজার
```

### Money

```text
পাঁচশ টাকা
১৫০০ টাকা
দুই লাখ টাকা
```

### Negation

```text
আমি যাব
আমি যাব না

এটা করব
এটা করব না

সে এসেছে
সে আসেনি
```

Negation error critical failure হিসেবে count করো।

### Names

Bangladesh:

```text
মানুষের নাম
জেলার নাম
উপজেলার নাম
প্রতিষ্ঠানের নাম
বাংলা brand/business names
```

### Bangla-English

Realistic sentences:

```text
আজকে Zoom meeting আছে।

GitHub repository update করো।

Adobe Illustrator open করো।

Facebook page-এ post দাও।
```

---

# PHASE 12 — Base Model Decision

অত্যন্ত গুরুত্বপূর্ণ:

বর্তমানে app-এ থাকা final/inference Zipformer artifact-কে fine-tunable ধরে নেবে না।

প্রথমে determine করো:

```text
Do we have:

training checkpoint?
training recipe?
tokenizer?
vocabulary?
training config?
optimizer-compatible checkpoint?
```

যদি শুধু ONNX/inference model থাকে, সেটিকে সরাসরি fine-tune করার চেষ্টা করবে না।

তখন trainable base model নির্বাচন করতে হবে।

Compare candidate families that:

1. Bengali training support করে
2. offline inference সম্ভব
3. Windows deployment সম্ভব
4. fine-tuning recipe available
5. Bangla-English output support করতে পারে
6. reasonable CPU inference দিতে পারে

---

# PHASE 13 — Current Zipformer Vocabulary Limitation

Current Bengali Zipformer vocabulary-তে Latin English support সীমিত।

তাই:

```text
Facebook
AI
GitHub
Adobe
Windows
```

এর মতো শব্দ সঠিক Latin spelling-এ output দেওয়ার capability বর্তমান model-এ নাও থাকতে পারে।

এটা decoder tuning দিয়ে সবসময় solve হবে না।

Fine-tuning architecture/tokenizer নির্বাচন করার সময় Bangla + Latin vocabulary support নিশ্চিত করো।

---

# PHASE 14 — Fine-Tuning Pipeline

Dataset প্রস্তুত এবং base model নির্বাচনের পর fine-tuning শুরু করো।

প্রতিটি run reproducible হবে।

Record:

```text
base model
base revision/hash
dataset revision/hash
train speakers
dev speakers
tokenizer
learning rate
batch size
epochs
optimizer
scheduler
augmentation
seed
hardware
training duration
checkpoint
```

Example directory:

```text
experiments/

    exp_001/
        config.json
        dataset.json
        train.log
        checkpoints/
        eval/
```

---

# PHASE 15 — Do Not Optimize on Holdout

Training:

```text
TRAIN
```

Decision making:

```text
DEV
```

Only release validation:

```text
HOLDOUT
```

Never:

```text
look at holdout
↓
change model
↓
rerun holdout
↓
change model again
```

এতে holdout dev-set হয়ে যাবে।

---

# PHASE 16 — Fine-Tuning Iteration

প্রতিটি checkpoint-এ:

```text
Fine-tune
↓
Run DEV inference
↓
bn_eval
↓
Error taxonomy
↓
Identify weakness
↓
Next experiment
```

Blindly epochs বাড়াবে না।

Determine:

- substitutions
- deletions
- insertions
- dialect errors
- English errors
- number errors
- entity errors
- negation errors
- noise failures

তারপর targeted improvement করো।

---

# PHASE 17 — Audio Augmentation

Training-এর সময় controlled augmentation experiment করা যেতে পারে:

```text
background noise
room noise
volume variation
speed variation
reverb
```

কিন্তু clean audio destroy করবে না।

Original audio preserve থাকবে।

Augmentation configuration record করতে হবে।

---

# PHASE 18 — VAD Evaluation

Recognition model এবং Voice Activity Detection আলাদা বিষয়।

Test:

```text
silence
fan
AC
keyboard
TV
road noise
breathing
long pause
low voice
```

Measure:

```text
false speech start
false speech end
first-word clipping
last-word clipping
silence hallucination
```

---

# PHASE 19 — Punctuation Separate রাখো

ASR recognition এবং text beautification এক metric-এ মিশাবে না।

Pipeline:

```text
Audio
 ↓
VAD
 ↓
ASR
 ↓
Raw spoken transcript
 ↓
Optional punctuation/formatting
 ↓
Final written text
```

Raw reference:

```text
text
```

Written-form reference:

```text
text_written
```

দুটিকে আলাদা evaluation run দিয়ে score করো।

Punctuation model যেন কখনো:

- নাম বদলায়
- সংখ্যা বদলায়
- negation বাদ দেয়
- hallucinated word যোগ করে

এটা allow করবে না।

---

# PHASE 20 — Regression Suite

প্রতিটি model/runtime change-এর পরে automatic regression চালাও।

Include:

```text
existing FLEURS 30
Bangladesh dev set
silence tests
critical entity tests
number tests
negation tests
Banglish tests
worker cancellation
invalid UTF-8
model load failure
```

আগের benchmark result থেকে degradation হলে report করো।

Silently accept করবে না।

---

# PHASE 21 — Report Every Experiment

প্রতিটি experiment-এর জন্য generate:

```text
reports/
    exp_001/
        report.md
        report.json
        per_utt.csv
        run-config.json
```

Report-এ লিখবে:

```text
Model
Dataset
Dataset hash
Checkpoint
WER
CER
WER bn-only
Variant WER
Number accuracy
Negation accuracy
Latin recall
Silence false accepts
Latency
Errors
Known limitations
Decision
```

---

# PHASE 22 — Initial Quality Gate

এগুলো provisional engineering targets।

Final target dataset পাওয়ার পরে revise করা যাবে।

Start with:

```text
Bangladesh clean conversational normalized WER:
≤ 10%
```

তারপর objective হবে এটিকে যতটা সম্ভব আরও কমানো।

Critical fields:

```text
number/amount accuracy → extremely high
negation preservation → extremely high
wrong-script errors → near zero
invalid UTF-8 → zero
silence hallucination → near zero
```

শুধু:

```text
100 - WER = accuracy
```

এভাবে marketing accuracy বানাবে না।

---

# PHASE 23 — Final Holdout

Model architecture, tokenizer, decoder এবং settings freeze হওয়ার পরে completely new Bangladesh holdout সংগ্রহ করো।

Target concept:

```text
50 new speakers
~1000 utterances
```

Development speakers final holdout-এ থাকবে না।

Prefer নতুন sentences/prompts।

Include:

- normal Bangla
- regional Bangla
- Banglish
- names
- numbers
- money
- negation
- noise
- multiple devices

এটাই release-level truth set হবে।

---

# PHASE 24 — Windows Product Validation

Offline model final হওয়ার পরে actual Windows app-এ test করো।

CI build pass যথেষ্ট নয়।

Test:

```text
Windows 10
Windows 11

8 GB RAM
16 GB RAM

Laptop microphone
USB microphone
Bluetooth microphone
```

Measure:

```text
microphone capture
permission
VAD
inference
punctuation
textbox insertion
undo
device switch
sleep/wake
long session
CPU
RAM
end-to-end latency
```

Most important latency metric:

```text
User stops speaking
→
Final text appears
```

---

# PHASE 25 — Privacy

Final application:

```text
audio stays local
model stays local
inference stays local
```

No cloud fallback without explicit future product decision.

Do not silently upload user speech.

---

# PHASE 26 — Immediate Execution Order

Do the work in this exact priority:

### Step 1
Implement and test `bn_eval`.

### Step 2
Create dataset management/provenance structure.

### Step 3
Download and inspect licensed Bengali datasets.

Priority:

```text
OpenSLR SLR53 Bangladeshi Bengali
Bangla-10K bn-BD
Common Voice Bengali
IndicVoices Bengali
FLEURS Bengali where appropriate
```

### Step 4
Normalize audio/manifests and deduplicate.

### Step 5
Create strict speaker-separated:

```text
train
dev
```

Do NOT finalize external holdout yet.

### Step 6
Collect Bangladesh-owned 204 development recordings.

### Step 7
Benchmark current Zipformer and other viable trainable baselines on the same DEV set.

### Step 8
Determine the best trainable base model.

Do not assume the current inference-only Zipformer artifact is fine-tunable.

### Step 9
Fine-tune the selected base model.

### Step 10
Run `bn_eval` after every meaningful checkpoint.

### Step 11
Perform error analysis and improve deliberately.

### Step 12
Freeze model/settings.

### Step 13
Collect/use independent Bangladesh final holdout.

### Step 14
Evaluate once as release candidate.

### Step 15
Integrate optimized offline model into Windows VoiceToText application and run real-device QA.

---

# Required Agent Behaviour

Do not report something as successful merely because:

- build passed
- model loaded
- a few sentences looked correct
- WER decreased on training data

Always distinguish:

```text
training result
development result
regression result
holdout result
Windows product result
```

Never hide failed runs.

Never delete evidence because the result is poor.

Every important experiment must be reproducible from repository instructions and recorded configs.

If a dataset, checkpoint, license, split or metric is uncertain, investigate it and document the uncertainty rather than guessing.

---

# Final Objective

আমাদের উদ্দেশ্য কোনো benchmark-এ শুধু ভালো score দেখানো নয়।

আমাদের উদ্দেশ্য:

> বাংলাদেশের একজন বাস্তব ব্যবহারকারী তার স্বাভাবিক কণ্ঠে, স্বাভাবিক মাইক্রোফোনে, বাংলা ও প্রয়োজনমতো English মিশিয়ে কথা বললে—সম্পূর্ণ offline অবস্থায় দ্রুত, নির্ভরযোগ্য এবং অর্থ না বদলে যথাসম্ভব নির্ভুল transcription পাওয়া।

এই উদ্দেশ্যের বিপরীতে প্রতিটি model, dataset, training decision এবং benchmark evaluate করবে।