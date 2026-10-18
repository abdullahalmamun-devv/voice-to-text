"""Score frozen JSONL references and one or more offline hypotheses."""
import argparse
import csv
import json
import math
from pathlib import Path
import platform
import re
import sys

from . import __version__
from .metrics import RATE_FIELDS, bootstrap, summarize, per_utterance
from .normalize import VERSION, load_variants, sha256

METADATA = ("region", "speaker", "device", "noise", "kind", "banglish", "gender")


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON key: " + key)
        result[key] = value
    return result


def _reject_constant(value):
    raise ValueError("Non-finite JSON constant: " + value)


def read_jsonl(path, reference=False):
    result = {}
    with Path(path).open("rb") as stream:
        for line_number, raw in enumerate(stream, 1):
            if not raw.strip():
                continue
            if len(raw) > 262_144:
                raise ValueError("Oversized JSONL line")
            invalid = False
            try:
                line = raw.decode("utf-8-sig" if line_number == 1 else "utf-8")
            except UnicodeDecodeError:
                if reference:
                    raise ValueError("Invalid UTF-8 in reference line " + str(line_number))
                # Replacement is ONLY for locating an intact JSON id/envelope.
                # Corrupt transcript text is never scored as a valid hypothesis.
                line, invalid = raw.decode("utf-8", errors="replace"), True
            item = json.loads(line, object_pairs_hook=_object, parse_constant=_reject_constant)
            if not isinstance(item, dict):
                raise ValueError("JSONL row must be an object")
            identifier = item.get("id")
            if not isinstance(identifier, str) or not identifier.strip() or len(identifier) > 200:
                raise ValueError("Missing/invalid id on line " + str(line_number))
            if any(char.isspace() or ord(char) < 32 or char == "\ufffd"
                   or 0xD800 <= ord(char) <= 0xDFFF for char in identifier):
                raise ValueError("Invalid or unrecoverable id")
            if identifier in result:
                raise ValueError("Duplicate id: " + identifier)
            text = item.get("text")
            if not isinstance(text, str):
                raise ValueError("Missing string text for " + identifier)
            if reference:
                for field in ("text", "text_written"):
                    if field in item:
                        if not isinstance(item[field], str):
                            raise ValueError("Reference text fields must be strings")
                        try:
                            item[field].encode("utf-8")
                        except UnicodeEncodeError:
                            raise ValueError("Invalid Unicode in reference " + identifier)
                for field in METADATA:
                    value = item.get(field)
                    if value is not None and not isinstance(value, (str, bool, int)):
                        raise ValueError("Metadata must be scalar: " + field)
                if "non_speech" in item and type(item["non_speech"]) is not bool:
                    raise ValueError("non_speech must be boolean")
            else:
                if item.get("error") is not None and not isinstance(item["error"], str):
                    raise ValueError("error must be a string or null")
                if type(item.get("invalid_utf8", False)) is not bool:
                    raise ValueError("invalid_utf8 must be boolean")
                if invalid:
                    item.update(text="", invalid_utf8=True, error="invalid_utf8")
                else:
                    try:
                        text.encode("utf-8")
                    except UnicodeEncodeError:
                        item.update(text="", error="invalid_unicode")
            result[identifier] = item
    if not result and reference:
        raise ValueError("Empty reference manifest")
    return result


def _group_key(value):
    if value is None or value == "":
        return "<unknown>"
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def _display(value):
    return "not measured" if value is None else "{:.2f}%".format(100 * value)


def render_markdown(report):
    lines = [
        "# Bengali ASR evaluation",
        "",
        "Evaluation role: **{}**. Text field: **{}**.".format(
            report["configuration"]["role"], report["configuration"]["text_field"]),
        "",
        "These are corpus error rates, not 100-minus-WER application accuracy.",
        "Null/unannotated metrics are not measured, never perfect scores.",
        "",
        "| Model | N | Norm WER | Raw WER | Scalar CER | Variant WER | Errors |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for label, result in report["models"].items():
        lines.append("| {} | {} | {} | {} | {} | {} | {} |".format(
            label, result["count"], _display(result["normalized_wer"]),
            _display(result["raw_wer"]), _display(result["cer"]),
            _display(result["variant_wer"]), result["errors"]))
    lines.extend(["", "## Confidence and paired comparisons", "",
                  "Bootstrap unit: {}; units: {}; resamples: {}; seed: {}.".format(
                      report["bootstrap"]["unit"], report["bootstrap"]["units"],
                      report["bootstrap"]["resamples"], report["bootstrap"]["seed"])])
    if report["bootstrap"]["warning"]:
        lines.extend(["", report["bootstrap"]["warning"] + "."])
    for label, result in report["models"].items():
        interval = report["bootstrap"]["models"][label]["normalized_wer"]
        lines.append("- {} normalized-WER 95% CI: {} to {}.".format(
            label, _display(interval["low"]), _display(interval["high"])))
        paired = report["bootstrap"]["paired_normalized_wer_delta"].get(label)
        if paired:
            lines.append("  Paired delta vs {}: {} percentage points; 95% CI {} to {} points.".format(
                paired["baseline"],
                "undefined" if paired["estimate"] is None else "{:.2f}".format(100 * paired["estimate"]),
                "undefined" if paired["low"] is None else "{:.2f}".format(100 * paired["low"]),
                "undefined" if paired["high"] is None else "{:.2f}".format(100 * paired["high"])))
    lines.extend(["", "## Critical and Bengali-specific metrics", "",
                  "| Model | BN-script WER proxy | Number | Money | Negation | Name | Latin recall | Non-speech FAR |",
                  "|---|---:|---:|---:|---:|---:|---:|---:|"])
    for label, result in report["models"].items():
        lines.append("| {} | {} | {} | {} | {} | {} | {} | {} |".format(label, *[
            _display(result[name]) for name in (
                "bn_only_wer", "number_exact_match", "money_exact_match",
                "negation_preservation", "name_exact_match", "latin_word_recall",
                "non_speech_false_accept_rate")]))
        counts = result["counts"]
        lines.extend(["", "{} annotation denominators: number={}, money={}, negation={}, name={}. "
                      "Non-speech valid={}, failed={}. Wrong-script={}, invalid-UTF8={}, "
                      "empty speech={}, runaway={}.".format(
                          label, counts["number_count"], counts["money_count"],
                          counts["negation_count"], counts["name_count"],
                          counts["non_speech_valid"], counts["non_speech_failed"],
                          counts["wrong_script"], counts["invalid_utf8"],
                          counts["empty_speech_hypothesis"], counts["runaway_output"])])
    lines.extend(["", "## Definitions and limitations", ""])
    lines.extend("- " + entry for entry in report["definitions"])
    lines.extend(["", "## Metadata groups", "",
                  "Per-field and joint groups, including unknown metadata, are in report.json.",
                  "Group estimates have no separate confidence/independence claim.",
                  "Per-utterance errors, edit counts, metadata and timings are in per_utt.csv.", ""])
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", required=True, type=Path)
    parser.add_argument("--hyp", required=True, action="append", metavar="LABEL=JSONL")
    parser.add_argument("--variants", type=Path, default=Path(__file__).with_name("variants_v1.tsv"))
    parser.add_argument("--group-by", default="region,device,kind,noise")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--text-field", choices=("text", "text_written"), default="text")
    parser.add_argument("--role", choices=("training", "development", "regression", "holdout"),
                        default="development")
    parser.add_argument("--bootstrap", type=int, default=2000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--bootstrap-unit", choices=("auto", "speaker", "utterance"), default="auto")
    args = parser.parse_args(argv)
    if args.out.exists():
        raise ValueError("Output already exists; use a new run directory")
    group_fields = args.group_by.split(",") if args.group_by else []
    if len(set(group_fields)) != len(group_fields) or any(f not in METADATA for f in group_fields):
        raise ValueError("Unknown or duplicate metadata grouping field")
    references = read_jsonl(args.ref, reference=True)
    variants = load_variants(args.variants)
    models, sources = {}, {}
    for specification in args.hyp:
        label, separator, filename = specification.partition("=")
        if not separator or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,63}", label):
            raise ValueError("Expected safe LABEL=JSONL")
        if label in models:
            raise ValueError("Duplicate model label")
        path = Path(filename)
        hypotheses = read_jsonl(path)
        if set(hypotheses) - set(references):
            raise ValueError("Hypothesis has IDs absent from references")
        rows = [
            per_utterance(references[key], hypotheses.get(key, {
                "id": key, "text": "", "error": "missing_hypothesis"}), variants, args.text_field)
            for key in sorted(references)
        ]
        models[label] = rows
        sources[label] = {"path": str(path), "sha256": sha256(path),
                          "missing_ids": sorted(set(references) - set(hypotheses))}
    summaries = {label: summarize(rows) for label, rows in models.items()}
    confidence = bootstrap(models, args.bootstrap, args.seed, args.bootstrap_unit)
    baseline = next(iter(models))
    for label, delta in confidence["paired_normalized_wer_delta"].items():
        left, right = summaries[baseline]["normalized_wer"], summaries[label]["normalized_wer"]
        delta["estimate"] = None if left is None or right is None else right - left
    groups = {}
    for label, rows in models.items():
        groups[label] = {}
        for field in group_fields:
            keys = sorted({_group_key(r[field]) for r in rows})
            groups[label][field] = {key: summarize([r for r in rows if _group_key(r[field]) == key])
                                    for key in keys}
        if len(group_fields) > 1:
            keys = sorted({tuple(_group_key(r[f]) for f in group_fields) for r in rows})
            groups[label]["+".join(group_fields)] = {
                json.dumps(key, ensure_ascii=False): summarize([
                    r for r in rows if tuple(_group_key(r[f]) for f in group_fields) == key])
                for key in keys}
    root = Path(__file__).parent
    report = {
        "schema_version": 1,
        "configuration": {
            "bn_eval_version": __version__, "normalization_version": VERSION,
            "python_version": platform.python_version(), "role": args.role,
            "text_field": args.text_field, "group_by": group_fields,
            "reference": {"path": str(args.ref), "sha256": sha256(args.ref)},
            "hypotheses": sources,
            "variants": {"path": str(args.variants), "sha256": sha256(args.variants),
                         "entries": len(variants)},
            "source_hashes": {p.name: sha256(p) for p in sorted(root.glob("*.py"))},
        },
        "definitions": [
            "Primary normalization: NFC, Unicode punctuation to spaces, whitespace collapse. "
            "No digit, case, spelling, name or negation rewriting.",
            "WER/CER are corpus micro-averages. WER can exceed 100%. "
            "CER counts Unicode scalar values including normalized spaces, not graphemes.",
            "BN-only WER is a Bengali-script token filter proxy, not annotated linguistic segmentation. "
            "Mixed-script tokens and numerals are excluded from that secondary proxy only.",
            "Variant WER is secondary and applies the same frozen one-token mapping to both sides. "
            "It never replaces primary WER or critical-span exact match.",
            "Critical metrics require human-annotated normalized-token spans. No annotations means null. "
            "A span fails on any mismatch or insertion at/inside its boundaries; adjacent insertions "
            "can conservatively fail a span. This is lexical exact match, not a semantic number parser.",
            "Negation annotations must cover polarity-bearing phrases for affirmative AND negative cases. "
            "Unannotated positive sentences cannot establish negation preservation.",
            "Latin recall uses per-utterance, case-folded Latin-token multiset overlap; "
            "it does not detect all spoken English or transliteration.",
            "Non-speech FAR requires explicitly labeled empty-reference non-speech. "
            "Speech with noise metadata is not non-speech. Failed non-speech requests are reported "
            "separately and excluded from valid FAR denominator, never treated as successful silence.",
            "Wrong-script permits Bengali and Latin; it cannot detect all transliteration/omission errors. "
            "Runaway flag: hypothesis tokens > max(50, 4 * max(1, reference tokens)).",
            "Missing/error/invalid-UTF8 responses retain the reference and are scored as empty hypotheses. "
            "Corrupt bytes are not repaired into credited words; unidentifiable JSON rows fail the run.",
            "latency_ms is supplied end-to-end request timing; decode_ms is separate. "
            "The scorer measures neither and makes no cross-host speed claim.",
            "Bootstrap is paired across models and speaker-clustered when all speakers are known. "
            "Small, reused development data and unknown upstream training overlap cannot prove generalization.",
            "Metadata groups are marginal and joint descriptive summaries without per-group confidence intervals.",
        ],
        "models": summaries, "bootstrap": confidence, "groups": groups,
    }
    args.out.mkdir(parents=True)
    (args.out / "report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8")
    (args.out / "report.md").write_text(render_markdown(report), encoding="utf-8")
    all_rows = [{"model": label, **row} for label, rows in models.items() for row in rows]
    with (args.out / "per_utt.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(all_rows[0]))
        writer.writeheader()
        writer.writerows(all_rows)
    print(json.dumps({"role": args.role, "models": list(models), "utterances": len(references),
                      "output": str(args.out)}, sort_keys=True))
    return report


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as error:
        print("bn_eval: " + str(error), file=sys.stderr)
        sys.exit(2)
