#!/usr/bin/env python3
"""Offline warm-worker benchmark. No downloads; audio stays in the local worker.

Uses the app decoder, one model context, and manifest WAV paths. Cold first request
and warm later requests are separate. This is not microphone or insertion latency.
"""
import argparse
from contextlib import contextmanager
import csv
import hashlib
import json
import os
from pathlib import Path
import queue
import statistics
import subprocess
import tempfile
import threading
import time
from benchmark import edits, norm, percentile95


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def checkpoint_json(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    with temporary.open('w', encoding='utf-8') as output:
        json.dump(value, output, ensure_ascii=False, indent=2)
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, path)


@contextmanager
def benchmark_lock(path=None):
    """Avoid oversubscribed GGML workers invalidating timeout/speed comparisons."""
    path = path or Path(tempfile.gettempdir()) / 'voicetotext-asr-benchmark.lock'
    with path.open('a+b') as stream:
        stream.seek(0)
        if not stream.read(1):
            stream.write(b'0')
            stream.flush()
        stream.seek(0)
        try:
            if os.name == 'nt':
                import msvcrt
                msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            raise SystemExit('Another local ASR benchmark is running; run models sequentially') from None
        yield


class Worker:
    def __init__(self, command, stderr):
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=stderr, text=True, encoding='utf-8')
        self.responses = queue.Queue()
        def read():
            try:
                for line in self.process.stdout:
                    self.responses.put(line)
            finally:
                self.responses.put(None)
        self.reader = threading.Thread(target=read, daemon=True)
        self.reader.start()

    def request(self, payload, timeout):
        self.process.stdin.write(json.dumps(payload, ensure_ascii=False) + '\n')
        self.process.stdin.flush()
        try:
            line = self.responses.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError('Local worker timeout') from None
        if line is None:
            raise RuntimeError('Local worker exited')
        result = json.loads(line)
        if result.get('session_id') != payload['session_id']:
            raise ValueError('Mismatched worker session')
        return result

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait(timeout=10)
        self.reader.join(timeout=1)
        self.process.stdin.close()
        self.process.stdout.close()


def summarize(rows):
    result = {'count': len(rows), 'errors': sum(row['error'] is not None for row in rows), 'slices': {}}
    for tag in sorted({row['slice'] for row in rows}):
        group = [row for row in rows if row['slice'] == tag]
        def score(errors, total):
            denominator = sum(row[total] for row in group)
            return sum(row[errors] for row in group) / denominator if denominator else None
        warm = [row['request_wall_ms'] for row in group if not row['cold'] and not row['error']]
        rtfs = [row['decode_rtf'] for row in group if row['decode_rtf'] is not None]
        result['slices'][tag] = {
            'n': len(group), 'errors': sum(bool(row['error']) for row in group),
            'raw_wer': score('raw_word_edits', 'raw_words'),
            'normalized_wer': score('normalized_word_edits', 'normalized_words'),
            'normalized_scalar_cer': score('character_edits', 'characters'),
            'unexpected_script': sum(row['review_required'] is True for row in group),
            'silence_generated_text': sum(not row['reference'].strip() and bool(row['hypothesis'].strip()) for row in group),
            'warm_request_p50_ms': statistics.median(warm) if warm else None,
            'warm_request_p95_ms': percentile95(warm),
            'median_decode_rtf': statistics.median(rtfs) if rtfs else None,
        }
    result.update({'critical_entities': None, 'correction_burden': None, 'actual_insertions': None})
    return result


def run():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'model', 'corpus', 'output'):
        parser.add_argument(name, type=Path)
    parser.add_argument('--timeout', type=float, default=180)
    parser.add_argument('--cpu-profile', required=True)
    parser.add_argument('--no-timestamps', action='store_true')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with args.corpus.open(encoding='utf-8-sig', newline='') as stream:
        items = list(csv.DictReader(stream))
    ids = [item['id'] for item in items]
    if any(not identifier for identifier in ids) or len(set(ids)) != len(ids):
        raise SystemExit('Corpus IDs must be present and unique')
    config = {'binary_sha256': digest(args.binary), 'model_sha256': digest(args.model),
              'corpus_sha256': digest(args.corpus), 'cpu_profile': args.cpu_profile,
              'model_manifest_sha256': digest(args.model.with_suffix('.manifest.json')) if args.model.with_suffix('.manifest.json').exists() else None,
              'no_timestamps': args.no_timestamps, 'runner_sha256': digest(Path(__file__)),
              'mode': 'one persistent app worker, first request cold, later requests warm unless restarted',
              'decoder': 'app final beam=5, temperature=0, fallback increment=0.2, no translation/context/prompt',
              'normalization': 'NFC + Unicode punctuation to spaces + whitespace; digits/spelling not rewritten',
              'cer': 'Unicode scalar values including normalized spaces, not graphemes',
              'timeout_seconds': args.timeout, 'available_cpu_count': len(os.sched_getaffinity(0)) if hasattr(os, 'sched_getaffinity') else os.cpu_count()}
    if (args.output/'rows.jsonl').exists():
        raise SystemExit('Output already contains rows; use a new directory for another run')
    cfg = args.output / 'run-config.json'
    if cfg.exists() and json.loads(cfg.read_text()) != config:
        raise SystemExit('Different configuration already exists in output directory')
    checkpoint_json(cfg, config)
    rows, worker, cold = [], None, True
    with (args.output / 'worker-stderr.log').open('w') as stderr:
        try:
            for index, item in enumerate(items, 1):
                audio = (args.corpus.parent / item['audio']).resolve()
                if item.get('sha256') and digest(audio) != item['sha256']:
                    raise SystemExit('Audio integrity mismatch: ' + item['id'])
                if worker is None:
                    if digest(args.binary) != config['binary_sha256']:
                        raise SystemExit('Benchmark binary changed; preserve this partial run and use a new directory')
                    worker = Worker([str(args.binary.resolve()), '--worker'], stderr)
                    cold = True
                start, error, transcript = time.perf_counter(), None, {}
                try:
                    response = worker.request({'session_id': index, 'model': str(args.model.resolve()),
                                               'wav_path': str(audio), 'samples': [], 'language': item.get('language') or 'bn',
                                               'prompt': '', 'preview': False, 'no_timestamps': args.no_timestamps}, args.timeout)
                    checkpoint_json(args.output / f'result-{index:04}.json', response)
                    if response.get('error'):
                        error = response['error']
                    else:
                        transcript = response.get('transcript') or {}
                        if not isinstance(transcript.get('text'), str):
                            raise ValueError('Invalid transcript')
                        if bool(transcript.get('no_timestamps')) != args.no_timestamps:
                            raise ValueError('Worker decoder profile mismatch')
                except (OSError, ValueError, RuntimeError, TimeoutError) as failure:
                    error = type(failure).__name__
                    worker.close()
                    worker = None
                wall = round((time.perf_counter()-start)*1000)
                ref, hyp = item['reference'], transcript.get('text', '') if not error else ''
                nref, nhyp = norm(ref), norm(hyp)
                seconds = int(item['num_samples']) / 16000 if item.get('num_samples') else None
                decode = transcript.get('decode_ms') if not error else None
                row = {'id': item['id'], 'slice': item.get('slice') or 'unspecified', 'error': error,
                       'cold': cold, 'request_wall_ms': wall, 'audio_seconds': seconds,
                       'decode_ms': decode, 'decode_rtf': decode/1000/seconds if decode is not None and seconds else None,
                       'raw_word_edits': edits(ref.split(), hyp.split()), 'raw_words': len(ref.split()),
                       'normalized_word_edits': edits(nref.split(), nhyp.split()), 'normalized_words': len(nref.split()),
                       'character_edits': edits(list(nref), list(nhyp)), 'characters': len(nref),
                       'review_required': transcript.get('review_required') if not error else None,
                       'recognized_language': transcript.get('recognized_language') if not error else None,
                       'reference': ref, 'hypothesis': hyp}
                rows.append(row)
                with (args.output / 'rows.jsonl').open('a', encoding='utf-8') as stream:
                    stream.write(json.dumps(row, ensure_ascii=False)+'\n')
                    stream.flush()
                    os.fsync(stream.fileno())
                report = summarize(rows)
                report['configuration'] = config
                checkpoint_json(args.output/'report.json', report)
                print(json.dumps({'done': index, 'total': len(items), 'id': item['id'], 'error': error, 'decode_ms': decode}), flush=True)
                cold = False
        finally:
            if worker:
                worker.close()
    with (args.output/'results.csv').open('w', encoding='utf-8-sig', newline='') as stream:
        if rows:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
            stream.flush()
            os.fsync(stream.fileno())


def main():
    with benchmark_lock():
        run()


if __name__ == '__main__':
    main()
