"""Aggregate diffusion_hist.py output per step. Usage: python3 tools/diffusion_hist_agg.py <sha8> [hist.json]"""
import json,statistics as st,collections
d=json.load(open(__import__('sys').argv[2] if len(__import__('sys').argv)>2 else 'hist.json'));names={int(k):v for k,v in d['names'].items()}
def nm(i): return names.get(int(i),str(i))
def agg(r):
    out={}
    S=r['seeds']
    items=set(i for s in S.values() for i in s['items'])
    for i in items:
        rows=[s['items'][str(i)] if str(i) in s['items'] else s['items'].get(i) for s in S.values()]
        rows=[x for x in rows if x]
        if not rows: continue
        n=len(S)
        out[i]=dict(seeds=len(rows)/n,first=st.median(x['first'] for x in rows),peak=st.mean(x['peak_share'] for x in rows),final=st.mean(x['final_share'] for x in rows),
            fl=sum(x['forgot'] for x in rows)/max(1,sum(x['learned'] for x in rows)),learned=sum(x['learned'] for x in rows)/len(rows),
            decl=sum(1 for x in rows if x['final_share']<0.5*x['peak_share'])/len(rows),lost=sum(1 for x in rows if x.get('lost'))/len(rows),dead=sum(1 for x in rows if x['final']==0)/len(rows))
    return out
import sys
for r in d['runs']:
    if r['sha']==sys.argv[1] if len(sys.argv)>1 else r['sha']=='85f48480':
        a=agg(r)
        print('##',r['run'],r['map'],r['years'],r['subj'][:60])
        print('item                     seeds first  peak  final  forgot/learned learned/seed declined lost dead')
        for i,x in sorted(a.items(),key=lambda kv:kv[1]['first']):
            print(f"{nm(i)[:24]:24} {x['seeds']:.1f} {x['first']:6.0f} {x['peak']:.2f} {x['final']:.2f}  {x['fl']:.2f}  {x['learned']:8.0f}   {x['decl']:.1f}  {x['lost']:.1f} {x['dead']:.1f}")
