import importlib.util
import io
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest


def load(name, filename):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).parent/filename)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


bench=load('worker_benchmark','benchmark-worker.py')
converter=load('safetensors_converter','convert-bn-safetensors.py')


class WorkerBenchmarkTests(unittest.TestCase):
    def test_overlapping_benchmarks_are_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'benchmark.lock'
            with bench.benchmark_lock(path):
                with self.assertRaises(SystemExit):
                    with bench.benchmark_lock(path):
                        self.fail('Overlapping benchmark acquired the lock')
            with bench.benchmark_lock(path):
                pass

    def test_wrong_session_rejected(self):
        command=[sys.executable,'-u','-c',"import sys,json;sys.stdin.readline();print(json.dumps({'session_id':99}))"]
        worker=bench.Worker(command,subprocess.DEVNULL)
        try:
            with self.assertRaises(ValueError):worker.request({'session_id':1},2)
        finally:worker.close()

    def test_nonresponsive_worker_times_out(self):
        worker=bench.Worker([sys.executable,'-u','-c','import time;time.sleep(10)'],subprocess.DEVNULL)
        try:
            with self.assertRaises(TimeoutError):worker.request({'session_id':1},0.05)
        finally:worker.close()

    def test_failed_rows_count_and_cold_time_excluded(self):
        common={'slice':'bn','raw_words':1,'normalized_words':1,'characters':5,
                'reference':'hello','hypothesis':'','review_required':None,'decode_rtf':None}
        rows=[dict(common,error='failed',cold=True,request_wall_ms=1000,raw_word_edits=1,normalized_word_edits=1,character_edits=5),
              dict(common,error=None,cold=False,request_wall_ms=12,raw_word_edits=0,normalized_word_edits=0,character_edits=0)]
        report=bench.summarize(rows)
        self.assertEqual(report['errors'],1)
        self.assertEqual(report['slices']['bn']['normalized_wer'],0.5)
        self.assertEqual(report['slices']['bn']['warm_request_p50_ms'],12)


class ConverterTests(unittest.TestCase):
    def test_changed_tokenizer_or_architecture_is_rejected(self):
        config={'vocab_size':1,'max_source_positions':1,'d_model':1,'encoder_attention_heads':1,
                'encoder_layers':1,'max_target_positions':1,'decoder_attention_heads':1,
                'decoder_layers':1,'num_mel_bins':1}
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'template.bin'
            # One mel value and one base token; no weights are executed or required.
            path.write_bytes(struct.pack('<12i',0x67676d6c,*([1]*11))+struct.pack('<2ifii',1,1,0.,1,1)+b'A')
            prefix=converter.template_prefix(path,config,{'A':0})
            self.assertEqual(prefix[-1:],b'A')
            with self.assertRaises(ValueError):converter.template_prefix(path,config,{'B':0})
            config['d_model']=2
            with self.assertRaises(ValueError):converter.template_prefix(path,config,{'A':0})


if __name__=='__main__':unittest.main()
