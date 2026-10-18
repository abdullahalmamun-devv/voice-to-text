#!/usr/bin/env python3
"""Explicit developer/CI download; never called by the application."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import urllib.request

ROOT=Path(__file__).resolve().parent.parent

def fetch(url,path,sha,size=None):
    part=path.with_suffix(path.suffix+'.part')
    total=0
    with urllib.request.urlopen(url,timeout=90) as response,part.open('wb') as output:
        while data:=response.read(1024*1024):
            total+=len(data)
            if total>(size if size is not None else 100_000_000):
                raise SystemExit('Unexpected download size')
            output.write(data)
        output.flush();os.fsync(output.fileno())
    with part.open('rb') as stream:actual=hashlib.file_digest(stream,'sha256').hexdigest()
    if actual!=sha or (size is not None and total!=size):
        raise SystemExit('Pinned checksum mismatch: '+path.name)
    os.replace(part,path)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform',choices=['linux-x64','windows-x64'],required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.output.exists():raise SystemExit('Output already exists')
    pins=json.loads((ROOT/'native/bengali-zipformer-pins.json').read_text())
    work=args.output.parent/(args.output.name+'-downloads')
    work.mkdir(parents=True,exist_ok=True)
    models=work/'models';models.mkdir(exist_ok=True)
    base=pins['model_source'].replace('/tree/','/resolve/')+'/'
    for pin in pins['model_files']:
        fetch(base+pin['name'],models/pin['name'],pin['sha256'],pin['size_bytes'])
    runtime=pins['runtimes'][args.platform];archive=work/'runtime.tar.bz2'
    fetch(runtime['source'],archive,runtime['archive_sha256'])
    folder=work/'runtime';folder.mkdir(exist_ok=True)
    with tarfile.open(archive) as source:
        members=list(source)
        for pin in runtime['files']:
            selected=[m for m in members if m.isfile() and Path(m.name).name==pin['name']]
            if len(selected)!=1:raise SystemExit('Ambiguous/missing pinned runtime file')
            with source.extractfile(selected[0]) as stream,(folder/pin['name']).open('wb') as output:
                output.write(stream.read());output.flush();os.fsync(output.fileno())
    subprocess.run([sys.executable,str(ROOT/'scripts/prepare-bengali-pack.py'),
        '--platform',args.platform,'--models',str(models),'--runtime',str(folder),
        '--output',str(args.output)],check=True)
    audio=work/'test-wavs';audio.mkdir(exist_ok=True)
    for name,sha in [
        ('0.wav','db02c9e036f842a7125d953870022846963d95a7ed03f98b629f336e5b18c666'),
        ('1.wav','007a7346cbfb519db087c592bb84226525e679b1f11a392d589eb68aff6ed373')]:
        fetch(base+'test_wavs/'+name,audio/name,sha)
    print(json.dumps({'pack':str(args.output),'test_wavs':str(audio),'scope':'Developer/CI only'}))

if __name__=='__main__':main()
