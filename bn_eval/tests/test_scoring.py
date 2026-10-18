import csv
import itertools
import json
from pathlib import Path
import random

import pytest

from bn_eval.metrics import align, bootstrap, distance, per_utterance, summarize
from bn_eval.normalize import load_variants, normalize, wrong_script
from bn_eval.score import main, read_jsonl


def row(ref, hyp, **extra):
    return per_utterance({"id": "u1", "text": ref, **extra},
                         {"id": "u1", "text": hyp}, {})


def write(path, records):
    path.write_text("".join(json.dumps(record, ensure_ascii=False) + "\n"
                            for record in records), encoding="utf-8")
    return path


def oracle(a, b):
    matrix = [[0] * (len(b) + 1) for _ in range(len(a) + 1)]
    for i in range(len(a) + 1):
        matrix[i][0] = i
    for j in range(len(b) + 1):
        matrix[0][j] = j
    for i, x in enumerate(a, 1):
        for j, y in enumerate(b, 1):
            matrix[i][j] = min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1,
                               matrix[i - 1][j - 1] + (x != y))
    return matrix[-1][-1]


def test_alignment_against_independent_full_matrix_oracle():
    rng = random.Random(184)
    for _ in range(250):
        a = [rng.choice("abc") for _ in range(rng.randrange(12))]
        b = [rng.choice("abc") for _ in range(rng.randrange(12))]
        s, d, i, operations = align(a, b)
        assert s + d + i == distance(a, b) == oracle(a, b)
        assert [a[r] for op, r, _ in operations if op != "insert"] == a
        assert [b[h] for op, _, h in operations if op != "delete"] == b


@pytest.mark.parametrize("reference,hypothesis,counts", [
    ("আমি যাব", "আমি যাব", (0, 0, 0)),
    ("আমি যাব", "আমি আসব", (1, 0, 0)),
    ("আমি যাব না", "আমি যাব", (0, 1, 0)),
    ("আমি যাব", "আমি যাব না", (0, 0, 1)),
    ("", "ভুল শব্দ", (0, 0, 2)),
    ("আমি যাব", "", (0, 2, 0)),
])
def test_edit_error_taxonomy(reference, hypothesis, counts):
    actual = row(reference, hypothesis)
    assert tuple(actual[key] for key in ("substitutions", "deletions", "insertions")) == counts


def test_wer_can_exceed_one_hundred_percent():
    assert summarize([row("আমি", "আমি কালকে এখানে আবার আসব")])["normalized_wer"] == 4


def test_normalization_never_rewrites_entities_or_negation():
    assert normalize("  আমি,  ১৫০০ টাকা দেব না। ") == "আমি ১৫০০ টাকা দেব না"
    assert normalize("১৫০০") != normalize("1500")
    assert normalize("দশ হাজার") != normalize("১০০০০")
    assert normalize("করবো") != normalize("করব")
    assert normalize("আমি যাব না") != normalize("আমি যাব")


def test_cer_is_unicode_scalar_not_grapheme():
    result = row("কি", "ক")
    assert result["characters"] == 2
    assert result["character_errors"] == 1


def test_raw_and_normalized_scores_separate():
    result = summarize([row("আমি যাব।", "আমি যাব")])
    assert result["raw_wer"] == .5
    assert result["normalized_wer"] == 0


def test_variants_secondary_only():
    result = per_utterance({"id": "u", "text": "আমি যাবো"}, {"text": "আমি যাব"},
                           {"যাবো": "যাব"})
    assert result["word_errors"] == 1
    assert result["variant_word_errors"] == 0


@pytest.mark.parametrize("contents", [
    "variant\tcanonical\n১৫০০\t1500\n",
    "variant\tcanonical\na\tb\nb\tc\n",
    "variant\tcanonical\na\tb\na\tc\n",
    "variant\tcanonical\na b\tc\n",
])
def test_reject_unsafe_variant_maps(tmp_path, contents):
    path = tmp_path / "variants.tsv"
    path.write_text(contents, encoding="utf-8")
    with pytest.raises(ValueError):
        load_variants(path)


def test_wrong_script_allows_bengali_and_latin_but_detects_devanagari():
    assert not wrong_script("আজ GitHub meeting ১০টায়।")
    assert wrong_script("मैं যাব")
    assert not wrong_script("আমি যাব।")


def test_bn_only_is_explicit_script_proxy():
    result = row("আমি Zoom খুলব", "আমি Teams খুলব")
    assert result["word_errors"] == 1
    assert result["bn_word_errors"] == 0
    assert result["latin_count"] == 1 and result["latin_correct"] == 0


def test_latin_recall_is_multiset_local_and_case_folded():
    result = row("GitHub GitHub Zoom", "github Zoom Teams")
    assert result["latin_count"] == 3
    assert result["latin_correct"] == 2


def test_critical_metrics_absent_annotations_are_null():
    result = summarize([row("আমি ১০০ টাকা দেব না", "আমি ১০০ টাকা দেব না")])
    for key in ("number_exact_match", "money_exact_match", "negation_preservation",
                "name_exact_match"):
        assert result[key] is None


def test_number_at_wrong_position_does_not_pass_substring_check():
    result = row("রহিম ১০০ করিম ২০০", "রহিম ২০০ করিম ১০০",
                 entities=[{"kind": "number", "start": 1, "end": 2},
                           {"kind": "number", "start": 3, "end": 4}])
    assert result["number_count"] == 2
    assert result["number_correct"] == 0


@pytest.mark.parametrize("reference,hypothesis,span", [
    ("আমি যাব না", "আমি যাব", (1, 3)),
    ("আমি যাব", "আমি যাব না", (1, 2)),
    ("আমি যাব না আজ", "আমি যাব আজ না", (1, 3)),
])
def test_negation_omission_addition_and_relocation_fail(reference, hypothesis, span):
    result = row(reference, hypothesis, entities=[
        {"kind": "negation", "start": span[0], "end": span[1]}])
    assert result["negation_count"] == 1
    assert result["negation_correct"] == 0


def test_money_span_rejects_extra_numeric_word():
    result = row("দশ হাজার টাকা পাঠাব", "দশ হাজার পাঁচ টাকা পাঠাব",
                 entities=[{"kind": "money", "start": 0, "end": 3}])
    assert result["money_correct"] == 0


def test_name_spelling_not_variant_tolerant():
    result = per_utterance(
        {"id": "u", "text": "রহিম যাবে",
         "entities": [{"kind": "name", "start": 0, "end": 1}]},
        {"text": "রাহিম যাবে"}, {"রাহিম": "রহিম"})
    assert result["variant_word_errors"] == 0
    assert result["name_correct"] == 0


@pytest.mark.parametrize("entity", [
    {"kind": "number", "start": -1, "end": 1},
    {"kind": "name", "start": True, "end": 1},
    {"kind": "number", "start": 0, "end": 0},
    {"kind": "money", "start": 0, "end": 1, "text": "অন্য"},
    {"kind": "unknown", "start": 0, "end": 1},
])
def test_bad_entity_annotations_fail(entity):
    with pytest.raises(ValueError):
        row("এক টাকা", "এক টাকা", entities=[entity])


def test_written_form_has_independent_annotations():
    result = per_utterance(
        {"id": "u", "text": "দশ হাজার টাকা", "text_written": "১০,০০০ টাকা।",
         "entities": [{"kind": "money", "start": 0, "end": 3}]},
        {"text": "১০,০০০ টাকা।"}, {}, text_field="text_written")
    assert result["word_errors"] == 0 and result["money_count"] == 0


def test_noise_metadata_does_not_mean_non_speech():
    assert row("আমি যাব", "আমি যাব", noise="fan")["non_speech_count"] == 0


def test_non_speech_false_accept_and_failed_coverage():
    silence = row("", "আমি যাব", kind="silence")
    failed = per_utterance({"id": "u2", "text": "", "kind": "noise_only"},
                           {"text": "", "error": "timeout"}, {})
    result = summarize([silence, failed])
    assert result["non_speech_false_accept_rate"] == 1
    assert result["counts"]["non_speech_valid"] == 1
    assert result["counts"]["non_speech_failed"] == 1
    assert result["normalized_wer"] is None


def test_punctuation_on_silence_is_false_accept():
    assert row("", "...", kind="silence")["non_speech_false_accept"] == 1


def test_non_speech_cannot_have_spoken_reference():
    with pytest.raises(ValueError):
        row("আমি", "", kind="silence")


def test_runaway_output_keeps_full_error_penalty():
    result = row("হ্যাঁ", "না " * 60)
    assert result["runaway_output"] == 1
    assert result["word_errors"] == 60


def test_invalid_utf8_is_an_error_not_repaired_credit(tmp_path):
    path = tmp_path / "hyp.jsonl"
    path.write_bytes(b'{"id":"u","text":"\xff"}\n')
    hyp = read_jsonl(path)["u"]
    result = per_utterance({"id": "u", "text": "আমি যাব"}, hyp, {})
    assert result["invalid_utf8"] == 1
    assert result["hypothesis"] == ""
    assert result["deletions"] == 2
    with pytest.raises(ValueError):
        read_jsonl(path, reference=True)


def test_unrecoverable_utf8_id_is_rejected(tmp_path):
    path = tmp_path / "hyp.jsonl"
    path.write_bytes(b'{"id":"\xff","text":"a"}\n')
    with pytest.raises(ValueError):
        read_jsonl(path)


def test_escaped_surrogate_is_error(tmp_path):
    path = tmp_path / "hyp.jsonl"
    path.write_text('{"id":"u","text":"\\ud800"}\n', encoding="utf-8")
    assert read_jsonl(path)["u"]["error"] == "invalid_unicode"


@pytest.mark.parametrize("contents", [
    '{"id":"u","text":"a"}\n{"id":"u","text":"b"}\n',
    '{"id":"u","id":"v","text":"a"}\n',
    '{"id":"u","text":"a","latency_ms":NaN}\n',
    '{"text":"a"}\n',
])
def test_bad_jsonl_is_not_silently_dropped(tmp_path, contents):
    path = tmp_path / "input.jsonl"
    path.write_text(contents, encoding="utf-8")
    with pytest.raises(ValueError):
        read_jsonl(path)


def test_speaker_cluster_bootstrap_is_paired_and_deterministic():
    first, second = [], []
    for i in range(6):
        ref = {"id": str(i), "text": "আমি যাব", "speaker": "s" + str(i // 2)}
        first.append(per_utterance(ref, {"text": "আমি যাব"}, {}))
        second.append(per_utterance(ref, {"text": "আমি আসব"}, {}))
    models = {"baseline": first, "candidate": second}
    report = bootstrap(models, repeats=100, seed=7)
    assert report == bootstrap(models, repeats=100, seed=7)
    assert report["unit"] == "speaker" and report["units"] == 3
    delta = report["paired_normalized_wer_delta"]["candidate"]
    assert delta["low"] == delta["high"] == .5


def test_unknown_speakers_warn_and_forced_cluster_refuses():
    rows = [row("আমি", "আমি")]
    report = bootstrap({"x": rows}, repeats=100)
    assert report["unit"] == "utterance" and report["warning"]
    with pytest.raises(ValueError):
        bootstrap({"x": rows}, repeats=100, unit="speaker")


def test_frozen_native_results_reproduce_recorded_wer_and_cer():
    path = Path(__file__).resolve().parents[2] / "benchmarks/native-bengali-ci/2026-10-18/windows/results.csv"
    with path.open(encoding="utf-8-sig", newline="") as stream:
        rows = [row(item["reference"], item["hypothesis"]) for item in csv.DictReader(stream)]
    result = summarize(rows)
    assert result["count"] == 30
    assert result["normalized_wer"] == pytest.approx(.1774193548387097)
    assert result["raw_wer"] == pytest.approx(.30307941653160453)
    assert result["cer"] == pytest.approx(.05375808860129418)


def test_cli_determinism_missing_hypotheses_and_groups(tmp_path):
    refs = write(tmp_path / "refs.jsonl", [
        {"id": "u1", "text": "আমি যাব", "speaker": "s1", "noise": "fan"},
        {"id": "u2", "text": "আমি আসব", "speaker": "s2", "noise": "quiet"},
    ])
    hyp = write(tmp_path / "hyp.jsonl", [{"id": "u1", "text": "আমি যাব"}])
    for suffix in ("one", "two"):
        main(["--ref", str(refs), "--hyp", "model=" + str(hyp),
              "--out", str(tmp_path / suffix), "--bootstrap", "100"])
    for name in ("report.json", "report.md", "per_utt.csv"):
        assert (tmp_path / "one" / name).read_bytes() == (tmp_path / "two" / name).read_bytes()
    report = json.loads((tmp_path / "one/report.json").read_text())
    assert report["models"]["model"]["normalized_wer"] == .5
    assert report["models"]["model"]["errors"] == 1
    assert "<unknown>" in report["groups"]["model"]["region"]
    assert report["configuration"]["hypotheses"]["model"]["missing_ids"] == ["u2"]
    with pytest.raises(ValueError, match="already exists"):
        main(["--ref", str(refs), "--hyp", "model=" + str(hyp),
              "--out", str(tmp_path / "one")])


def test_silence_only_cli_has_undefined_wer_and_paired_delta(tmp_path):
    ref = write(tmp_path / "ref.jsonl", [{"id": "s", "text": "", "kind": "silence"}])
    hyp = write(tmp_path / "hyp.jsonl", [{"id": "s", "text": ""}])
    report = main(["--ref", str(ref), "--hyp", "one=" + str(hyp), "--hyp", "two=" + str(hyp),
                   "--out", str(tmp_path / "out"), "--bootstrap", "100"])
    assert report["models"]["one"]["normalized_wer"] is None
    assert report["bootstrap"]["paired_normalized_wer_delta"]["two"]["estimate"] is None


def test_unknown_hypothesis_id_refused(tmp_path):
    ref = write(tmp_path / "ref.jsonl", [{"id": "a", "text": "আমি"}])
    hyp = write(tmp_path / "hyp.jsonl", [{"id": "b", "text": "আমি"}])
    with pytest.raises(ValueError, match="absent"):
        main(["--ref", str(ref), "--hyp", "model=" + str(hyp), "--out", str(tmp_path / "out")])
