#!/usr/bin/env python3
"""Package one explicit local native pack. No downloads or model guessing."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tomllib

ROOT=Path(__file__).resolve().parent.parent
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']

def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','model_pack','output']:parser.add_argument(name,type=Path)
    parser.add_argument('--verification',type=Path)
    parser.add_argument('--imports',type=Path)
    args=parser.parse_args()
    if args.output.exists():raise SystemExit('Output already exists')
    profile=json.loads((args.model_pack/'bengali.vtt-model.json').read_text())
    assert profile['platform']=='windows-x64' and profile['experimental']
    pins=json.loads((ROOT/'native/bengali-zipformer-pins.json').read_text())
    for folder,files in [(args.model_pack,pins['model_files']),
                         (args.model_pack/'runtime',pins['runtimes']['windows-x64']['files'])]:
        assert set(p.name for p in folder.iterdir() if p.is_file()).issuperset(pin['name'] for pin in files)
        for pin in files:
            path=folder/pin['name']
            assert not path.is_symlink() and path.stat().st_size==pin['size_bytes'] and digest(path)==pin['sha256']
    assert set(p.name for p in (args.model_pack/'runtime').iterdir())==set(p['name'] for p in pins['runtimes']['windows-x64']['files'])
    args.output.mkdir(parents=True)
    shutil.copyfile(args.binary,args.output/'VoiceToText.exe')
    shutil.copytree(args.model_pack,args.output/'bengali-model')
    shutil.copytree(ROOT/'licenses',args.output/'licenses')
    shutil.copyfile(ROOT/'LICENSE',args.output/'LICENSE')
    shutil.copyfile(ROOT/'docs/EXPERIMENTAL-BENGALI.md',args.output/'START-HERE.md')
    if args.verification:shutil.copytree(args.verification,args.output/'verification')
    if args.imports:shutil.copyfile(args.imports,args.output/'windows-imports.txt')
    (args.output/'scripts').mkdir()
    shutil.copyfile(ROOT/'scripts/Verify-Package.ps1',args.output/'scripts/Verify-Package.ps1')
    (args.output/'Verify.cmd').write_bytes(b'@echo off\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\\Verify-Package.ps1"\r\npause\r\n')
    files=[{'path':str(p.relative_to(args.output)).replace('\\','/'),'size_bytes':p.stat().st_size,'sha256':digest(p)}
           for p in sorted(args.output.rglob('*')) if p.is_file()]
    (args.output/'package-manifest.json').write_text(json.dumps({'version':VERSION,'experimental':True,'files':files},indent=2))
    print(json.dumps({'output':str(args.output),'files':len(files),'experimental':True}))

if __name__=='__main__':main()
