#!/usr/bin/env python3
"""Build a pinned experimental model pack from LOCAL files. Never downloads.

Python is a development tool, not a shipped application dependency.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent.parent


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=['linux-x64','windows-x64'], required=True)
    for name in ['models','runtime','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    pins=json.loads((ROOT/'native/bengali-zipformer-pins.json').read_text())
    runtime=pins['runtimes'][args.platform]
    if args.output.exists():raise SystemExit('Output already exists; select a new directory')
    selected=[(args.models,args.output,pin) for pin in pins['model_files']]
    selected += [(args.runtime,args.output/'runtime',pin) for pin in runtime['files']]
    for source,_,pin in selected:
        path=source/pin['name']
        if path.is_symlink() or not path.is_file() or path.stat().st_size!=pin['size_bytes'] or digest(path)!=pin['sha256']:
            raise SystemExit('Pinned source file missing/corrupt: '+pin['name'])
    args.output.mkdir(parents=True)
    for source,target,pin in selected:
        target.mkdir(exist_ok=True)
        path=target/pin['name'];shutil.copyfile(source/pin['name'],path)
        if digest(path)!=pin['sha256']:raise SystemExit('Copied file failed verification')
    profile={'schema':1,'engine':pins['engine'],'runtime_version':pins['runtime_version'],
             'platform':args.platform,'experimental':True}
    (args.output/'bengali.vtt-model.json').write_text(json.dumps(profile,indent=2))
    shutil.copyfile(ROOT/'licenses/sherpa-onnx-APACHE-2.0.txt',args.output/'APACHE-2.0.txt')
    (args.output/'NOTICE.txt').write_text('Experimental Bengali speech model pack. Not a qualified stable release.\n'
        'Model: Alpha Cephei Vosk Bengali streaming Zipformer, Apache-2.0.\n'
        'Source: https://huggingface.co/alphacep/vosk-model-small-streaming-bn\n'
        'ONNX export: '+pins['model_source']+'\n'
        'sherpa-onnx: Copyright (c) 2023 Xiaomi Corporation, Apache-2.0.\n'
        'Runtime: '+runtime['source']+'\n'
        'ONNX Runtime: Copyright Microsoft Corporation, MIT; see ONNX-RUNTIME-MIT.txt.\n'
        'No automatic punctuation, Latin English preservation or vocabulary hints are qualified.\n')
    shutil.copyfile(ROOT/'licenses/onnxruntime-MIT.txt',args.output/'ONNX-RUNTIME-MIT.txt')
    shutil.copyfile(ROOT/'licenses/onnxruntime-ThirdPartyNotices.txt',args.output/'ONNX-RUNTIME-ThirdPartyNotices.txt')
    (args.output/'pack-provenance.json').write_text(json.dumps(pins,indent=2))
    print(json.dumps({'profile':str(args.output/'bengali.vtt-model.json'),'platform':args.platform,
                      'model_bytes':sum(p['size_bytes'] for p in pins['model_files']),
                      'runtime_bytes':sum(p['size_bytes'] for p in runtime['files']),'experimental':True}))


if __name__=='__main__':main()
