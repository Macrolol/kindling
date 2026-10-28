"""Spread speed from probe series CSVs: years from first appearance to 50%/90% of bands knowing,
and known / makeable / in-kit shares at the end. Usage: diffusion_series.py <dir> [item names...]"""
import csv,glob,gzip,sys,collections,statistics as st
def op(p): return gzip.open(p,'rt') if p.endswith('.gz') else open(p)
D=collections.defaultdict(dict)
for p in glob.glob(f'{sys.argv[1]}/**/series-*.csv*',recursive=True):
    tag=p.split('series-')[1].split('.csv')[0]
    s=collections.defaultdict(list)
    for r in csv.DictReader(op(p)):
        s[r['name']].append((float(r['year']),int(r['bands']),int(r['known']),int(r['make']),int(r['kit']),int(r['used'])))
    for nm,v in s.items(): D[nm][tag]=v
want=sys.argv[2:] or sorted(D)
print(f"{'item':26} seeds  t50  t90 | known make  kit (mean over 2nd half of each run)")
for nm in want:
    if nm not in D: continue
    t50=[];t90=[];kn=[];mk=[];kt=[]
    for tag,v in D[nm].items():
        y0=v[0][0]
        a=[y-y0 for y,b,k,*_ in v if b and k/b>=0.5]; t50.append(a[0] if a else None)
        a=[y-y0 for y,b,k,*_ in v if b and k/b>=0.9]; t90.append(a[0] if a else None)
        end=v[-1][0]; h=[x for x in v if x[0]>=end/2 and x[1]]
        if h:
            kn.append(st.mean(x[2]/x[1] for x in h)); mk.append(st.mean(x[3]/x[1] for x in h)); kt.append(st.mean(x[4]/x[1] for x in h))
    f=lambda xs: f"{st.median([x for x in xs if x is not None]):4.0f}" if any(x is not None for x in xs) else "   -"
    g=lambda xs: f"{st.mean(xs):.2f}" if xs else "  - "
    print(f"{nm[:26]:26} {len(D[nm][list(D[nm])[0]]) and len(D[nm]):5} {f(t50)} {f(t90)} | {g(kn)} {g(mk)} {g(kt)}  (reached 50%: {sum(x is not None for x in t50)}/{len(t50)})")
