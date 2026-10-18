"""Versioned normalization. Never repair spelling, numbers, names or negation."""
import csv
import hashlib
from pathlib import Path
import unicodedata

VERSION = "bn-nfc-punctuation-whitespace-v1"


def normalize(text):
    text = unicodedata.normalize("NFC", text)
    return " ".join("".join(
        " " if unicodedata.category(char).startswith("P") else char
        for char in text
    ).split())


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_variants(path):
    variants = {}
    if path is None:
        return variants
    with Path(path).open(encoding="utf-8-sig", newline="") as stream:
        lines = (line for line in stream if line.strip() and not line.startswith("#"))
        reader = csv.DictReader(lines, delimiter="\t")
        if reader.fieldnames != ["variant", "canonical"]:
            raise ValueError("Variant TSV must have variant and canonical columns")
        for row in reader:
            source, target = normalize(row["variant"]), normalize(row["canonical"])
            if len(source.split()) != 1 or len(target.split()) != 1 or source == target:
                raise ValueError("Variants must map distinct single tokens")
            if source in variants:
                raise ValueError("Duplicate variant: " + source)
            if any(char.isdigit() for char in source + target):
                raise ValueError("Number variants are forbidden")
            variants[source] = target
    if set(variants) & set(variants.values()):
        raise ValueError("Variant chains/cycles are forbidden")
    return variants


def latin(token):
    return any(unicodedata.name(char, "").startswith("LATIN ") for char in token)


def bengali(token):
    return any("\u0980" <= char <= "\u09ff" and
               unicodedata.category(char).startswith("L") for char in token)


def wrong_script(text):
    """Allow Bengali, Latin, ASCII digits and punctuation in mixed speech."""
    for char in text:
        category = unicodedata.category(char)
        if category[0] not in "LMN":
            continue
        if "\u0980" <= char <= "\u09ff" or char in "0123456789":
            continue
        if unicodedata.name(char, "").startswith("LATIN "):
            continue
        return True
    return False
