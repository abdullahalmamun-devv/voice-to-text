#!/usr/bin/env python3
"""Native worker compatibility smoke, not speech-quality or Windows UI acceptance."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import threading
import time
import wave

spec=importlib.util.spec_from_file_location('benchmark_worker',Path(__file__).with_name('benchmark-worker.py'))
bench=importlib.util.module_from_spec(spec);spec.loader.exec_module(bench)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','test_wavs','output']:parser.add_argument(name,type=Path)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    report={'scope':'Native compatibility smoke; not UI/hardware or ASR acceptance',
        'binary_sha256':bench.digest(args.binary),'profile_sha256':bench.digest(args.profile),'checks':[]}
    with (args.output/'worker.log').open('w') as log:
        worker=bench.Worker([str(args.binary.resolve()),'--worker'],log)
        def request(session,**extra):
            payload={'session_id':session,'model':str(args.profile.resolve()),'samples':[],
                'wav_path':str((args.test_wavs/'0.wav').resolve()),'language':'bn','prompt':'',
                'preview':False,'no_timestamps':True}
            payload.update(extra)
            return worker.request(payload,60)
        try:
            for index in range(2):
                response=request(index+1,wav_path=str((args.test_wavs/f'{index}.wav').resolve()))
                transcript=response.get('transcript') or {}
                assert not response.get('error') and any('\u0980'<=c<='\u09ff' for c in transcript.get('text',''))
                assert transcript['experimental_model'] and transcript['no_timestamps']
                assert not transcript['timing_reliable'] and transcript['segments']==[]
                bench.checkpoint_json(args.output/f'response-{index+1}.json',response)
            report['checks'].append('two real Bengali WAVs decode with experimental/no-SRT metadata')
            response=request(3,language='en');assert response.get('error') and response['transcript'] is None
            response=request(4,prompt='ঢাকা');assert response.get('error') and response['transcript'] is None
            report['checks'].append('unsupported English and vocabulary hints return explicit errors')
            response=request(5,wav_path=None,samples=[0.0]*32000)
            assert not response.get('error') and response['transcript']['text']==''
            response=request(6);assert not response.get('error') and response['transcript']['text']
            report['checks'].append('silence produces no text and worker recovers after rejected requests')
            finished=[]
            with tempfile.TemporaryDirectory() as folder:
                audio=Path(folder)/'synthetic-120s.wav'
                with wave.open(str(audio),'wb') as wav:
                    wav.setparams((1,2,16000,0,'NONE','not compressed'))
                    wav.writeframes(b'\x48\x01'*1_920_000)
                def long_request():
                    try:request(7,wav_path=str(audio))
                    except (RuntimeError,OSError,ValueError,TimeoutError):finished.append('cancelled')
                thread=threading.Thread(target=long_request);thread.start();time.sleep(.2)
                start=time.perf_counter();worker.close();thread.join(timeout=5)
                assert not thread.is_alive() and finished==['cancelled']
                report['cancel_wall_ms']=round((time.perf_counter()-start)*1000)
            report['checks'].append('busy native worker terminates without a late result')
        finally:worker.close()
    report['passed']=True
    bench.checkpoint_json(args.output/'smoke-report.json',report)
    print(json.dumps(report))

if __name__=='__main__':main()
