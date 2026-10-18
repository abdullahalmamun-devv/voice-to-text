#!/usr/bin/env python3
"""Verify unmodified vendored crates against Cargo's bundled checksums."""
import hashlib
import json
from pathlib import Path

root=Path(__file__).resolve().parent.parent
patched={'whisper-rs','whisper-rs-sys'}
checked=0
failures=[]
for crate in sorted((root/'vendor').iterdir()):
    if not crate.is_dir() or crate.name in patched:
        continue
    manifest=crate/'.cargo-checksum.json'
    if not manifest.is_file():
        failures.append(str(manifest.relative_to(root))+' missing')
        continue
    for name, expected in json.loads(manifest.read_text())['files'].items():
        path=crate/name
        if not path.is_file():
            failures.append(str(path.relative_to(root))+' missing')
            continue
        with path.open('rb') as stream:
            actual=hashlib.file_digest(stream,'sha256').hexdigest()
        checked+=1
        if actual!=expected:
            failures.append(str(path.relative_to(root))+' checksum mismatch')
if failures:
    raise SystemExit('\n'.join(failures))
print(f'{checked} unmodified vendored files verified; two documented path-patched crates excluded')
