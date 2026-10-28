"""Per-run/seed/item spread history from archived CI run-<id> releases (frames lore + chronicle).
Usage: python3 tools/diffusion_hist.py <dir with run-*/kindling-run-*> [out.json]"""
import json,glob,os,re,subprocess,collections,sys
names={}
runs=[]
for rd in sorted([d for d in glob.glob(sys.argv[1]+'/run-*/kindling-run-*') if os.path.isdir(d)]):
    m=json.load(open(rd+'/run-manifest.json'))
    sha=m['ref_sha']
    subj=subprocess.run(['git','-C',os.path.dirname(os.path.dirname(os.path.abspath(__file__))),'log','-1','--format=%cs %s',sha],capture_output=True,text=True).stdout.strip()
    seeds={}
    for sd in sorted(glob.glob(rd+'/jobs/results-*/data/seed-*')):
        s=int(sd.split('seed-')[1])
        lost=collections.Counter()
        for line in open(sd+'/chronicle.ndjson'):
            e=json.loads(line)
            if e['kind']=='FirstInvention':
                mm=re.search(r'first to make (.+?) \(',e['text'])
                if mm: names[e['item']]=mm.group(1)
            elif e['kind']=='LostWorldwide':
                lost[e['item']]+=1
                mm=re.search(r'can make (.+?) any more',e['text'])
                if mm: names.setdefault(e['item'],mm.group(1))
        it={}
        last_year=0
        for line in open(sd+'/frames.ndjson'):
            f=json.loads(line)
            nb=sum(l[1] for l in f['lineages']) or 1
            y=f['year'];last_year=y
            for item,able,lr,fg in f['lore']:
                d=it.setdefault(item,dict(first=y,peak=0,peak_share=0,peak_year=y,final=0,final_share=0,learned=0,forgot=0,zero_after=0))
                share=able/nb
                if share>d['peak_share']: d['peak_share']=share;d['peak_year']=y
                d['peak']=max(d['peak'],able)
                d['learned']+=lr;d['forgot']+=fg
            seen={l[0]:l[1] for l in f['lore']}
            for item,d in it.items():
                a=seen.get(item,0)
                d['final']=a;d['final_share']=a/nb
        for k,c in lost.items():
            if k in it: it[k]['lost']=c
        seeds[s]=dict(items=it,years=last_year)
    runs.append(dict(run=os.path.basename(rd),sha=sha[:8],subj=subj,map=m['inputs'].get('map') or 'default',years=m['inputs']['years'],seeds=seeds))
json.dump(dict(names={int(k):v for k,v in names.items()},runs=runs),open(sys.argv[2] if len(sys.argv)>2 else 'hist.json','w'))
for r in runs: print(r['run'],r['sha'],r['map'],r['years'],len(r['seeds']),r['subj'][:70])
