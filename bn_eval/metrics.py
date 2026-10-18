"""Edit alignment, explicit-annotation critical metrics, paired cluster bootstrap."""
from collections import Counter, defaultdict
import math
import random
import statistics
import unicodedata

from .normalize import bengali, latin, normalize, wrong_script

RATE_FIELDS = {
    "normalized_wer": ("word_errors", "words"),
    "raw_wer": ("raw_word_errors", "raw_words"),
    "cer": ("character_errors", "characters"),
    "bn_only_wer": ("bn_word_errors", "bn_words"),
    "variant_wer": ("variant_word_errors", "words"),
    "number_exact_match": ("number_correct", "number_count"),
    "money_exact_match": ("money_correct", "money_count"),
    "negation_preservation": ("negation_correct", "negation_count"),
    "name_exact_match": ("name_correct", "name_count"),
    "latin_word_recall": ("latin_correct", "latin_count"),
    "non_speech_false_accept_rate": ("non_speech_false_accept", "non_speech_valid"),
}
CRITICAL_KINDS = ("number", "money", "negation", "name")


def distance(a, b):
    # Unicode scalar CER, not grapheme CER. Trim equal edges to bound common work.
    left = 0
    while left < min(len(a), len(b)) and a[left] == b[left]:
        left += 1
    a, b = a[left:], b[left:]
    while a and b and a[-1] == b[-1]:
        a, b = a[:-1], b[:-1]
    if len(a) > len(b):
        a, b = b, a
    if len(a) * len(b) > 25_000_000:
        raise ValueError("CER alignment budget exceeded; split oversized utterance")
    row = list(range(len(a) + 1))
    for j, right in enumerate(b, 1):
        current = [j]
        for i, left in enumerate(a, 1):
            current.append(min(current[-1] + 1, row[i] + 1,
                               row[i - 1] + (left != right)))
        row = current
    return row[-1]


def align(reference, hypothesis):
    """Return (S,D,I, operations). Stable tie order: equal, substitute, delete, insert."""
    n, m = len(reference), len(hypothesis)
    if (n + 1) * (m + 1) > 4_000_000:
        raise ValueError("Word alignment budget exceeded; split oversized utterance")
    back = [bytearray(m + 1) for _ in range(n + 1)]
    row = list(range(m + 1))
    for j in range(1, m + 1):
        back[0][j] = 3
    for i, left in enumerate(reference, 1):
        current = [i]
        back[i][0] = 2
        for j, right in enumerate(hypothesis, 1):
            candidates = (row[j - 1] + (left != right), row[j] + 1,
                          current[j - 1] + 1)
            best = min(candidates)
            current.append(best)
            choice = candidates.index(best)
            back[i][j] = (0 if left == right else 1) if choice == 0 else choice + 1
        row = current
    i, j, operations = n, m, []
    while i or j:
        operation = back[i][j]
        if operation in (0, 1):
            operations.append(("equal" if operation == 0 else "substitute", i - 1, j - 1))
            i, j = i - 1, j - 1
        elif operation == 2:
            operations.append(("delete", i - 1, j))
            i -= 1
        else:
            operations.append(("insert", i, j - 1))
            j -= 1
    operations.reverse()
    counts = Counter(op[0] for op in operations)
    return counts["substitute"], counts["delete"], counts["insert"], operations


def validate_entities(reference, tokens):
    entities = reference.get("entities")
    if entities is None:
        return []
    if not isinstance(entities, list):
        raise ValueError("entities must be a list")
    seen = set()
    for entity in entities:
        if not isinstance(entity, dict) or entity.get("kind") not in CRITICAL_KINDS:
            raise ValueError("Unknown critical entity kind")
        start, end = entity.get("start"), entity.get("end")
        if type(start) is not int or type(end) is not int or not 0 <= start < end <= len(tokens):
            raise ValueError("Entity span must index normalized reference tokens")
        key = (entity["kind"], start, end)
        if key in seen:
            raise ValueError("Duplicate entity span")
        seen.add(key)
        if "text" in entity and entity["text"] != " ".join(tokens[start:end]):
            raise ValueError("Entity text does not match its reference span")
    return entities


def per_utterance(reference, hypothesis, variants, text_field="text"):
    raw_reference = reference.get(text_field)
    if not isinstance(raw_reference, str):
        raise ValueError("Every reference needs a string " + text_field)
    raw_hypothesis = hypothesis.get("text", "")
    if not isinstance(raw_hypothesis, str):
        raise ValueError("Hypothesis text must be a string")
    error = hypothesis.get("error")
    invalid = bool(hypothesis.get("invalid_utf8"))
    if error or invalid:
        raw_hypothesis = ""
    ref, hyp = normalize(raw_reference), normalize(raw_hypothesis)
    rt, ht = ref.split(), hyp.split()
    s, d, i, operations = align(rt, ht)
    non_speech = reference.get("non_speech") is True or reference.get("kind") in (
        "silence", "noise_only", "non_speech")
    if non_speech and raw_reference.strip():
        raise ValueError("Non-speech reference must have empty actual-spoken text")
    row = {
        "id": reference["id"], "reference": raw_reference, "hypothesis": raw_hypothesis,
        "error": error or ("invalid_utf8" if invalid else ""),
        "invalid_utf8": int(invalid), "word_errors": s + d + i,
        "words": len(rt), "substitutions": s, "deletions": d, "insertions": i,
        "raw_word_errors": distance(raw_reference.split(), raw_hypothesis.split()),
        "raw_words": len(raw_reference.split()),
        "character_errors": distance(ref, hyp), "characters": len(ref),
        "bn_word_errors": distance([t for t in rt if bengali(t) and not latin(t)],
                                   [t for t in ht if bengali(t) and not latin(t)]),
        "bn_words": sum(bengali(t) and not latin(t) for t in rt),
        "variant_word_errors": distance([variants.get(t, t) for t in rt],
                                       [variants.get(t, t) for t in ht]),
        "wrong_script": int(wrong_script(raw_hypothesis)),
        "empty_hypothesis": int(not hyp),
        "empty_speech_hypothesis": int(bool(rt) and not hyp),
        "runaway_output": int(len(ht) > max(50, 4 * max(1, len(rt)))),
        "non_speech_count": int(non_speech),
        "non_speech_valid": int(non_speech and not error and not invalid),
        "non_speech_failed": int(non_speech and bool(error or invalid)),
        "non_speech_false_accept": int(non_speech and not error and not invalid
                                      and bool(raw_hypothesis.strip())),
    }
    for kind in CRITICAL_KINDS:
        row[kind + "_correct"], row[kind + "_count"] = 0, 0
    # Written references have independent token spans. Never reuse spoken spans.
    annotation_reference = dict(reference)
    if text_field != "text":
        annotation_reference["entities"] = reference.get("entities_written")
    entities = validate_entities(annotation_reference, rt)
    equal = {r for op, r, _ in operations if op == "equal"}
    insertion_gaps = {r for op, r, _ in operations if op == "insert"}
    for entity in entities:
        start, end, kind = entity["start"], entity["end"], entity["kind"]
        # Conservative: insertions at either boundary also fail exact match,
        # including added negation after an annotated affirmative verb.
        correct = all(k in equal for k in range(start, end)) and not any(
            start <= gap <= end for gap in insertion_gaps)
        row[kind + "_count"] += 1
        row[kind + "_correct"] += int(correct)
    expected = Counter(unicodedata.normalize("NFC", t).casefold() for t in rt if latin(t))
    observed = Counter(unicodedata.normalize("NFC", t).casefold() for t in ht if latin(t))
    row["latin_count"] = sum(expected.values())
    row["latin_correct"] = sum((expected & observed).values())
    for field in ("region", "speaker", "device", "noise", "kind", "banglish", "gender"):
        row[field] = reference.get(field)
    for field in ("latency_ms", "decode_ms"):
        value = hypothesis.get(field)
        if value is not None and (isinstance(value, bool) or not isinstance(value, (int, float))
                                  or not math.isfinite(value) or value < 0):
            raise ValueError("Invalid timing: " + field)
        row[field] = value
    return row


def totals(rows):
    fields = set(field for pair in RATE_FIELDS.values() for field in pair)
    fields.update(("substitutions", "deletions", "insertions", "wrong_script",
                   "invalid_utf8", "empty_hypothesis", "empty_speech_hypothesis",
                   "runaway_output", "non_speech_count", "non_speech_failed"))
    return {field: sum(row[field] for row in rows) for field in sorted(fields)}


def ratio(numerator, denominator):
    return numerator / denominator if denominator else None


def summarize(rows):
    counts = totals(rows)
    result = {name: ratio(counts[num], counts[den])
              for name, (num, den) in RATE_FIELDS.items()}
    result.update({"count": len(rows), "errors": sum(bool(r["error"]) for r in rows),
                   "counts": counts})
    for field in ("latency_ms", "decode_ms"):
        values = sorted(r[field] for r in rows if not r["error"] and r[field] is not None)
        result[field] = {"n": len(values),
                         "p50": statistics.median(values) if values else None,
                         "p95": values[max(0, math.ceil(len(values) * .95) - 1)] if values else None}
    return result


def percentile(values, fraction):
    if not values:
        return None
    ordered = sorted(values)
    point = (len(ordered) - 1) * fraction
    low, high = math.floor(point), math.ceil(point)
    return ordered[low] + (ordered[high] - ordered[low]) * (point - low)


def bootstrap(models, repeats=2000, seed=0, unit="auto"):
    if not 100 <= repeats <= 100_000:
        raise ValueError("bootstrap repeats must be between 100 and 100000")
    first = next(iter(models.values()))
    speakers_known = all(isinstance(r["speaker"], str) and r["speaker"].strip() for r in first)
    if unit == "speaker" and not speakers_known:
        raise ValueError("Speaker bootstrap requires a speaker ID for every utterance")
    chosen = "speaker" if (unit == "speaker" or (unit == "auto" and speakers_known)) else "utterance"
    members = defaultdict(list)
    for index, row in enumerate(first):
        members[row["speaker"] if chosen == "speaker" else row["id"]].append(index)
    keys = sorted(members)
    units = {label: [totals([rows[i] for i in members[key]]) for key in keys]
             for label, rows in models.items()}
    distributions = {label: {metric: [] for metric in RATE_FIELDS} for label in models}
    baseline = next(iter(models))
    deltas = {label: [] for label in models if label != baseline}
    rng = random.Random(seed)
    for _ in range(repeats):
        sampled = [rng.randrange(len(keys)) for _ in keys]
        sampled_wer = {}
        for label in models:
            for metric, (num, den) in RATE_FIELDS.items():
                value = ratio(sum(units[label][i][num] for i in sampled),
                              sum(units[label][i][den] for i in sampled))
                if value is not None:
                    distributions[label][metric].append(value)
                if metric == "normalized_wer":
                    sampled_wer[label] = value
        for label in deltas:
            if sampled_wer[label] is not None and sampled_wer[baseline] is not None:
                deltas[label].append(sampled_wer[label] - sampled_wer[baseline])
    def interval(values):
        return {"low": percentile(values, .025), "high": percentile(values, .975),
                "defined_resamples": len(values)}
    return {
        "method": "paired percentile cluster bootstrap; corpus micro-averaged ratios",
        "unit": chosen, "units": len(keys), "resamples": repeats, "seed": seed,
        "warning": ("Too few independent units for a meaningful interval" if len(keys) < 2
                    else "Unknown speakers: utterance resampling does not establish speaker independence"
                    if chosen == "utterance" and not speakers_known else None),
        "models": {label: {metric: interval(values) for metric, values in metrics.items()}
                   for label, metrics in distributions.items()},
        "paired_normalized_wer_delta": {
            label: {"baseline": baseline, "direction": "candidate minus baseline; negative is better",
                    **interval(values)}
            for label, values in deltas.items()},
    }
