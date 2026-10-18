#!/usr/bin/env python3
"""Offline held-out corpus runner. Never downloads or uploads audio.
CSV columns: id,audio,reference,slice,language. Paths relative to the CSV.
Only consenting/rights-cleared data belongs in this manifest.
Raw and punctuation-normalized scores remain separate.
"""
import argparse,csv,json,re,subprocess,time,statistics
from pathlib import Path

def edits(a,b):
    row=list(range(len(b)+1))
    for i,x in enumerate(a,1):
        nxt=[i]
        for j,y in enumerate(b,1):nxt.append(min(nxt[-1]+1,row[j]+1,row[j-1]+(x!=y)))
        row=nxt
    return row[-1]

def norm(s):return re.sub(r'[।.,!?;:—–"“”()]',' ',s).strip()

def main():
    p=argparse.ArgumentParser();p.add_argument('binary',type=Path);p.add_argument('model',type=Path);p.add_argument('corpus',type=Path);p.add_argument('output',type=Path);args=p.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    rows=[];by_slice={}
    with args.corpus.open(encoding='utf-8-sig',newline='') as f:
        for i,item in enumerate(csv.DictReader(f)):
            audio=(args.corpus.parent/item['audio']).resolve();target=args.output/f'result-{i:06}.json'
            start=time.perf_counter()
            subprocess.run([str(args.binary.resolve()),'--transcribe',str(args.model.resolve()),str(audio),str(target),item.get('language')or'bn'],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            result=json.loads(target.read_text(encoding='utf-8'));ref=item['reference'];hyp=result['text'];tag=item.get('slice')or'unspecified'
            raw=edits(ref.split(),hyp.split());nref=norm(ref);nhyp=norm(hyp);nw=edits(nref.split(),nhyp.split());cer=edits(list(nref),list(nhyp))
            row={'id':item['id'],'slice':tag,'raw_word_edits':raw,'raw_words':len(ref.split()),'normalized_word_edits':nw,'normalized_words':len(nref.split()),'character_edits':cer,'characters':len(nref),'decode_ms':result['decode_ms'],'load_ms':result['load_ms'],'cold_wall_ms':round((time.perf_counter()-start)*1000),'reference':ref,'hypothesis':hyp}
            rows.append(row);by_slice.setdefault(tag,[]).append(row)
    report={'method':'CPU local, cold CLI per utterance; CER uses Unicode scalar values, not grapheme clusters','model':str(args.model.name),'count':len(rows),'slices':{}}
    for tag,group in by_slice.items():
        def score(errors,total):
            denom=sum(r[total]for r in group);return sum(r[errors]for r in group)/denom if denom else None
        timing=sorted(r['cold_wall_ms']for r in group)
        report['slices'][tag]={'n':len(group),'raw_wer':score('raw_word_edits','raw_words'),'normalized_wer':score('normalized_word_edits','normalized_words'),'cer':score('character_edits','characters'),'cold_p50_ms':statistics.median(timing),'cold_p95_ms':timing[min(len(timing)-1,int(len(timing)*.95))]}
    (args.output/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    with (args.output/'results.csv').open('w',encoding='utf-8-sig',newline='')as f:
        if rows:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    print(json.dumps({'count':len(rows),'output':str(args.output)}))
if __name__=='__main__':main()
