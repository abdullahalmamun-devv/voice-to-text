#!/usr/bin/env python3
"""Restore the frozen 30 public FLEURS WAVs. Developer/CI setup, not app code."""
import argparse
import csv
import hashlib
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
URL='https://huggingface.co/datasets/google/fleurs/resolve/70bb2e84b976b7e960aa89f1c648e09c59f894dd/data/bn_in/audio/test.tar.gz'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    if args.output.exists():raise SystemExit('Select a new output directory')
    source=ROOT/'benchmarks/fleurs-bn-smoke-30'
    # Git checkouts can change CSV CRLF/LF bytes. Pin the canonical LF form;
    # references, IDs and per-WAV hashes must remain exactly the frozen corpus.
    corpus_bytes=(source/'corpus.csv').read_bytes().replace(b'\r\n',b'\n')
    if hashlib.sha256(corpus_bytes).hexdigest()!='3c8b0b412fd9251fb09e8e7671b6c7ef382c3b379a971781e82cdff0a0578a9c':
        raise SystemExit('Frozen corpus manifest checksum mismatch')
    rows=list(csv.DictReader(corpus_bytes.decode('utf-8-sig').splitlines()))
    wanted={Path(row['audio']).name:row for row in rows}
    if len(wanted)!=30:raise SystemExit('Unexpected frozen corpus')
    for row in rows:
        path=PurePosixPath(row['audio'])
        if path.is_absolute() or '..' in path.parts:raise SystemExit('Unsafe corpus path')
    args.output.mkdir(parents=True)
    with urllib.request.urlopen(URL,timeout=90) as response:
        class Limited:
            count=0
            def read(self,size=-1):
                data=response.read(size)
                self.count+=len(data)
                if self.count>96_000_000:raise ValueError('Selected corpus exceeded stream limit')
                return data
        with tarfile.open(fileobj=Limited(),mode='r|gz') as archive:
            for member in archive:
                name=Path(member.name).name
                if not member.isfile() or name not in wanted:continue
                row=wanted[name]
                if member.size>10_000_000:raise ValueError('Unexpected audio member size')
                with archive.extractfile(member) as stream:data=stream.read()
                if hashlib.sha256(data).hexdigest()!=row['sha256']:
                    raise ValueError('Frozen audio checksum mismatch')
                target=args.output/row['audio'];target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(data);del wanted[name]
                if not wanted:break
    if wanted:raise SystemExit('Frozen corpus members missing')
    (args.output/'corpus.csv').write_bytes(corpus_bytes)
    shutil.copyfile(source/'provenance.json',args.output/'provenance.json')
    print('Restored all 30 individually verified public WAVs; full archive not verified')

if __name__=='__main__':main()
