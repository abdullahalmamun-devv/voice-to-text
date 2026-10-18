#!/usr/bin/env python3
"""Compile the upstream-header ABI probe with the host C compiler, no ASR load."""
import os
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent
EXPECTED='272 136 16 8 144 176 192 232 248 64 128'


def main():
    metadata=json.loads((ROOT/'native/source-integrity.json').read_text())
    if hashlib.sha256((ROOT/'native'/metadata['header']).read_bytes()).hexdigest()!=metadata['sha256']:
        raise SystemExit('Pinned upstream C header checksum mismatch')
    with tempfile.TemporaryDirectory() as folder:
        output=Path(folder)/('abi.exe' if os.name=='nt' else 'abi')
        if os.name=='nt':
            command=['cl','/nologo','/TC','/I'+str(ROOT/'native'),str(ROOT/'native/check-abi.c'),'/Fe:'+str(output)]
        else:command=['cc','-I',str(ROOT/'native'),str(ROOT/'native/check-abi.c'),'-o',str(output)]
        subprocess.run(command,cwd=folder,check=True,stdout=subprocess.DEVNULL)
        actual=subprocess.check_output([str(output)],text=True).strip()
        if actual!=EXPECTED:raise SystemExit('Unsupported C ABI: '+actual)
        print('Pinned sherpa-onnx C ABI verified')


if __name__=='__main__':main()
