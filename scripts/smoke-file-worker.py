#!/usr/bin/env python3
"""Local synthetic-silence WAV/recovery smoke. Not Bengali ASR or GUI acceptance."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import wave

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('binary',type=Path);parser.add_argument('model',type=Path)
    args=parser.parse_args()
    with tempfile.TemporaryDirectory() as folder:
        path=Path(folder)/'বাংলা-silence.wav'
        with wave.open(str(path),'wb') as audio:
            audio.setnchannels(1);audio.setsampwidth(2);audio.setframerate(16000)
            audio.writeframes(bytes(32000))
        process=subprocess.Popen([str(args.binary.resolve()),'--worker'],stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,encoding='utf-8')
        def request(identifier,wav,samples):
            payload={'session_id':identifier,'model':str(args.model.resolve()),'language':'bn',
                     'prompt':'','preview':False,'samples':samples,'wav_path':wav}
            process.stdin.write(json.dumps(payload,ensure_ascii=False)+'\n');process.stdin.flush()
            result=json.loads(process.stdout.readline())
            assert result['session_id']==identifier
            return result
        try:
            assert request(1,str(path),[])['transcript']['text']==''
            assert request(2,str(path),[0.])['error']
            assert request(3,str(path.with_name('missing.wav')),[])['error']
            assert request(4,str(path),[])['transcript']['text']==''
        finally:
            process.stdin.close()
            try:process.wait(timeout=10)
            except subprocess.TimeoutExpired:process.kill();process.wait()
        assert process.returncode==0
    print(json.dumps({'checks':4,'passed':True,'scope':'synthetic silence WAV and worker error recovery only'}))

if __name__=='__main__':main()
