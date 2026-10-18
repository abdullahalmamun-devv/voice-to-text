"""Convert recorded benchmark CSVs to scorer JSONL without new inference."""
import argparse
import csv
import json
from pathlib import Path

from .normalize import sha256


def load(path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        rows = list(csv.DictReader(stream))
    if not rows or len({row["id"] for row in rows}) != len(rows):
        raise ValueError("Empty/duplicate CSV ids")
    return rows


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--results", action="append", required=True, metavar="LABEL=CSV")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        raise ValueError("Output already exists")
    corpus = load(args.corpus)
    refs = {row["id"]: row for row in corpus}
    outputs = {"ref.jsonl": [
        {"id": row["id"], "text": row["reference"],
         "gender": row.get("gender"), "source_dataset": "google/fleurs",
         "kind": "read", "speaker": None}
        for row in corpus]}
    provenance = {"scope": "Recorded development/regression outputs; no new inference or training",
                  "corpus": {"path": str(args.corpus), "sha256": sha256(args.corpus)},
                  "results": {}, "timing": "Original recorded WAV worker request; not microphone stop-to-final"}
    for specification in args.results:
        label, separator, filename = specification.partition("=")
        if not separator or not label or label in ("ref", "provenance") or any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-" for c in label):
            raise ValueError("Expected safe LABEL=CSV")
        if label in provenance["results"]:
            raise ValueError("Duplicate label")
        path = Path(filename)
        rows = load(path)
        if {row["id"] for row in rows} != set(refs):
            raise ValueError("CSV corpus ID mismatch")
        result = []
        for row in rows:
            if row["reference"] != refs[row["id"]]["reference"]:
                raise ValueError("CSV reference changed")
            item = {"id": row["id"], "text": row["hypothesis"], "error": row.get("error") or None}
            for source, target in (("request_wall_ms", "latency_ms"), ("decode_ms", "decode_ms")):
                if row.get(source):
                    item[target] = float(row[source])
            # Legacy invalid-text errors may include invalid UTF-8; do not invent
            # byte-level classifications that the CSV does not establish.
            result.append(item)
        outputs[label + ".jsonl"] = result
        provenance["results"][label] = {"path": str(path), "sha256": sha256(path)}
    args.out.mkdir(parents=True)
    for name, rows in outputs.items():
        (args.out / name).write_text("".join(json.dumps(row, ensure_ascii=False) + "\n"
                                            for row in rows), encoding="utf-8")
    (args.out / "provenance.json").write_text(
        json.dumps(provenance, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
