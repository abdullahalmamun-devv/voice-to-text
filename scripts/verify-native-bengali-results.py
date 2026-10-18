#!/usr/bin/env python3
"""Check native integration parity, not release-quality acceptance."""
import argparse
import csv
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parent.parent

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('results',type=Path)
    args=parser.parse_args()
    expected=list(csv.DictReader((ROOT/'benchmarks/fleurs-bn-smoke-30/runs/zipformer-30-serial/results.csv').open(encoding='utf-8-sig')))
    actual=list(csv.DictReader((args.results/'results.csv').open(encoding='utf-8-sig')))
    assert len(actual)==len(expected)==30
    changed=[]
    for index,(reference,row) in enumerate(zip(expected,actual),1):
        assert row['id']==reference['id'] and not row['error']
        transcript=json.loads((args.results/f'result-{index:04}.json').read_text(encoding='utf-8'))['transcript']
        assert transcript['experimental_model'] and transcript['no_timestamps']
        assert not transcript['timing_reliable'] and transcript['segments']==[]
        if row['hypothesis']!=reference['hypothesis']:changed.append(row['id'])
    report=json.loads((args.results/'report.json').read_text(encoding='utf-8'))
    assert report['count']==30 and report['errors']==0
    summary={'scope':'Native/research transcript parity, NOT stable release acceptance',
             'count':30,'changed_transcripts':changed,'quality':report['slices']}
    (args.results/'integration-parity.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps(summary))
    if changed:raise SystemExit('Native integration output differs; inspect recorded results before promotion')

if __name__=='__main__':main()
