#!/usr/bin/env python3
"""Offline held-out corpus runner. Never downloads or uploads audio.
CSV columns: id,audio,reference,slice,language. Paths relative to the CSV.
Only consenting/rights-cleared data belongs in this manifest.
Raw and punctuation-normalized scores remain separate.
"""
import argparse,csv,json,re,subprocess,time,statistics,unicodedata,math,hashlib
from pathlib import Path

def edits(a,b):
    row=list(range(len(b)+1))
    for i,x in enumerate(a,1):
        nxt=[i]
        for j,y in enumerate(b,1):nxt.append(min(nxt[-1]+1,row[j]+1,row[j-1]+(x!=y)))
        row=nxt
    return row[-1]

def norm(s):
    return ' '.join(''.join(' ' if unicodedata.category(c).startswith('P') else c for c in unicodedata.normalize('NFC',s)).split())

def percentile95(values):
    values=sorted(values)
    return values[max(0,math.ceil(len(values)*.95)-1)] if values else None

def digest(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
    p=argparse.ArgumentParser();p.add_argument('binary',type=Path);p.add_argument('model',type=Path);p.add_argument('corpus',type=Path);p.add_argument('output',type=Path);args=p.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    config={'binary_sha256':digest(args.binary),'model_sha256':digest(args.model),'corpus_sha256':digest(args.corpus),'mode':'cold CLI per utterance','normalization':'NFC + Unicode punctuation to spaces + whitespace'}
    config_path=args.output/'run-config.json'
    if config_path.exists() and json.loads(config_path.read_text())!=config:raise SystemExit('Output directory belongs to a different run configuration')
    config_path.write_text(json.dumps(config,indent=2))
    rows=[];by_slice={};seen=set()
    with args.corpus.open(encoding='utf-8-sig',newline='') as f:
        for i,item in enumerate(csv.DictReader(f)):
            audio=(args.corpus.parent/item['audio']).resolve();target=args.output/f'result-{i:06}.json'
            if not item['id'] or item['id'] in seen:raise SystemExit('Corpus IDs must be present and unique')
            seen.add(item['id']);target.unlink(missing_ok=True)
            start=time.perf_counter();error=None
            try:
                subprocess.run([str(args.binary.resolve()),'--transcribe',str(args.model.resolve()),str(audio),str(target),item.get('language')or'bn'],check=True,timeout=600,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
                result=json.loads(target.read_text(encoding='utf-8'))
            except (OSError,ValueError,subprocess.SubprocessError) as failure:
                error=type(failure).__name__;result={'text':'','decode_ms':None,'load_ms':None}
            ref=item['reference'];hyp=result['text'];tag=item.get('slice')or'unspecified'
            raw=edits(ref.split(),hyp.split());nref=norm(ref);nhyp=norm(hyp);nw=edits(nref.split(),nhyp.split());cer=edits(list(nref),list(nhyp))
            row={'error':error,'id':item['id'],'slice':tag,'raw_word_edits':raw,'raw_words':len(ref.split()),'normalized_word_edits':nw,'normalized_words':len(nref.split()),'character_edits':cer,'characters':len(nref),'decode_ms':result['decode_ms'],'load_ms':result['load_ms'],'cold_wall_ms':round((time.perf_counter()-start)*1000),'reference':ref,'hypothesis':hyp}
            rows.append(row);by_slice.setdefault(tag,[]).append(row)
    report={'method':'CPU local, cold CLI per utterance; CER uses Unicode scalar values, not grapheme clusters','configuration':config,'errors':sum(r['error'] is not None for r in rows),'critical_entities':None,'correction_burden':None,'actual_false_insertions':None,'model':str(args.model.name),'count':len(rows),'slices':{}}
    for tag,group in by_slice.items():
        def score(errors,total):
            denom=sum(r[total]for r in group);return sum(r[errors]for r in group)/denom if denom else None
        timing=sorted(r['cold_wall_ms']for r in group if r['error'] is None)
        report['slices'][tag]={'n':len(group),'errors':sum(r['error'] is not None for r in group),'raw_wer':score('raw_word_edits','raw_words'),'normalized_wer':score('normalized_word_edits','normalized_words'),'cer':score('character_edits','characters'),'cold_p50_ms':statistics.median(timing) if timing else None,'cold_p95_ms':percentile95(timing)}
    (args.output/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    with (args.output/'results.csv').open('w',encoding='utf-8-sig',newline='')as f:
        if rows:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    print(json.dumps({'count':len(rows),'output':str(args.output)}))
if __name__=='__main__':main()
