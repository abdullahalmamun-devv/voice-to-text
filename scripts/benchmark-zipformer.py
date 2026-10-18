#!/usr/bin/env python3
"""Research-only local Bengali Zipformer comparison; NOT an app engine.

Requires numpy and sherpa-onnx==1.13.8. No model download or cloud inference.
Accepts the frozen mono/16kHz IEEE float32 or PCM16 WAV corpus only.
"""
import argparse
import csv
import importlib.util
import json
from pathlib import Path
import struct
import time
from benchmark import edits, norm

spec = importlib.util.spec_from_file_location('worker_benchmark', Path(__file__).with_name('benchmark-worker.py'))
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


def read_audio(path):
    import numpy as np
    data = path.read_bytes()
    if data[:4] != b'RIFF' or data[8:12] != b'WAVE':
        raise ValueError('Expected RIFF WAV')
    offset, fmt, audio = 12, None, None
    while offset + 8 <= len(data):
        tag, size = struct.unpack_from('<4sI', data, offset)
        chunk = data[offset + 8:offset + 8 + size]
        if len(chunk) != size:
            raise ValueError('Truncated WAV')
        if tag == b'fmt ': fmt = struct.unpack_from('<HHIIHH', chunk)
        if tag == b'data': audio = chunk
        offset += 8 + size + size % 2
    if fmt is None or audio is None or fmt[1] != 1 or fmt[2] != 16000:
        raise ValueError('Benchmark requires mono/16kHz WAV')
    if (fmt[0], fmt[5]) == (3, 32):
        samples = np.frombuffer(audio, dtype='<f4').copy()
    elif (fmt[0], fmt[5]) == (1, 16):
        samples = np.frombuffer(audio, dtype='<i2').astype(np.float32) / 32768
    else:
        raise ValueError('Benchmark requires float32 or PCM16')
    if not np.isfinite(samples).all():
        raise ValueError('Non-finite samples')
    return samples


def unexpected_script(text):
    # Same ranges as app text::unexpected_script; Latin-only output is not detected.
    return any(c.isalnum() and not (c.isascii() or 0xc0 <= ord(c) <= 0x24f
               or 0x1e00 <= ord(c) <= 0x1eff or 0x980 <= ord(c) <= 0x9ff) for c in text)


def run():
    import numpy as np
    import sherpa_onnx
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('model_folder', 'corpus', 'output'): parser.add_argument(name, type=Path)
    args = parser.parse_args()
    if sherpa_onnx.__version__ != '1.13.8': raise SystemExit('Use pinned sherpa-onnx==1.13.8')
    args.output.mkdir(parents=True, exist_ok=True)
    if (args.output / 'rows.jsonl').exists(): raise SystemExit('Use a new output directory')
    manifest = json.loads((args.model_folder / 'manifest.json').read_text())
    for name, expected in manifest['files'].items():
        if bench.digest(args.model_folder / name) != expected['sha256']:
            raise SystemExit('Model checksum mismatch: ' + name)
    items = list(csv.DictReader(args.corpus.open(encoding='utf-8-sig', newline='')))
    ids = [item['id'] for item in items]
    if any(not item for item in ids) or len(set(ids)) != len(ids): raise SystemExit('Invalid corpus IDs')
    config = {'mode':'external research engine, not integrated into the desktop app',
              'runtime':'sherpa-onnx 1.13.8 CPU; 2 threads; streaming zipformer2',
              'decoder':'modified_beam_search, max_active_paths=4, endpoint detection OFF, no hotwords/LM',
              'audio':'original samples plus fixed 0.5s zero tail; one fresh stream per clip; reusable recognizer',
              'corpus_sha256':bench.digest(args.corpus),
              'model_manifest_sha256':bench.digest(args.model_folder/'manifest.json'),
              'runner_sha256':bench.digest(Path(__file__)), 'numpy_version':np.__version__,
              'normalization':'NFC + Unicode punctuation to spaces + whitespace; no digit/spelling rewriting',
              'qualified_default':False}
    (args.output/'run-config.json').write_text(json.dumps(config, indent=2))
    load_start = time.perf_counter()
    recognizer = sherpa_onnx.OnlineRecognizer.from_transducer(
        **{name:str(args.model_folder/(name+'.onnx')) for name in ('encoder','decoder','joiner')},
        tokens=str(args.model_folder/'tokens.txt'), num_threads=2, sample_rate=16000, feature_dim=80,
        decoding_method='modified_beam_search', max_active_paths=4, enable_endpoint_detection=False,
        model_type='zipformer2', provider='cpu')
    config['recognizer_load_ms'] = round((time.perf_counter()-load_start)*1000)
    rows = []
    for index, item in enumerate(items, 1):
        path = (args.corpus.parent/item['audio']).resolve()
        if bench.digest(path) != item['sha256']: raise SystemExit('Audio checksum mismatch')
        start, error, text, decode = time.perf_counter(), None, '', None
        try:
            samples = read_audio(path)
            if len(samples) != int(item['num_samples']): raise ValueError('Audio length mismatch')
            stream = recognizer.create_stream()
            decoding_start = time.perf_counter()
            stream.accept_waveform(16000, samples)
            stream.accept_waveform(16000, np.zeros(8000, dtype=np.float32))
            stream.input_finished()
            while recognizer.is_ready(stream): recognizer.decode_stream(stream)
            text = recognizer.get_result_all(stream).text.strip()
            decode = round((time.perf_counter()-decoding_start)*1000)
        except (ValueError, RuntimeError, OSError) as failure:
            error = type(failure).__name__
        reference = item['reference']; nref, nhyp = norm(reference), norm(text)
        seconds = int(item['num_samples'])/16000
        row = {'id':item['id'],'slice':item['slice'],'error':error,'cold':index==1,
               'request_wall_ms':round((time.perf_counter()-start)*1000),'audio_seconds':seconds,
               'decode_ms':decode,'decode_rtf':decode/1000/seconds if decode is not None else None,
               'raw_word_edits':edits(reference.split(),text.split()),'raw_words':len(reference.split()),
               'normalized_word_edits':edits(nref.split(),nhyp.split()),'normalized_words':len(nref.split()),
               'character_edits':edits(list(nref),list(nhyp)),'characters':len(nref),
               'review_required':unexpected_script(text) if not error else None,
               'recognized_language':None,'reference':reference,'hypothesis':text}
        rows.append(row)
        with (args.output/'rows.jsonl').open('a',encoding='utf-8') as output:
            output.write(json.dumps(row,ensure_ascii=False)+'\n')
        report = bench.summarize(rows); report['configuration'] = config
        (args.output/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
        print(json.dumps({'done':index,'total':len(items),'error':error,'decode_ms':decode}),flush=True)
    with (args.output/'results.csv').open('w',encoding='utf-8-sig',newline='') as output:
        writer=csv.DictWriter(output,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)


if __name__ == '__main__':
    with bench.benchmark_lock(): run()
