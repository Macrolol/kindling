"""diffusion_knives.py DIR: cut-slot kit shares (metal vs stone) and metal-knife spread from probe series CSVs."""
import csv,glob,gzip,sys,collections,statistics as st
def op(p): return gzip.open(p,'rt') if p.endswith('.gz') else open(p)
METAL=['copper knife','bronze knife','iron knife','steel knife']; STONE=['stone blade','flint blade','obsidian blade']
CHK=[100,200,300,400,500,600,800,1000,1200,1500]
runs={}
for p in sorted(glob.glob(f'{sys.argv[1]}/**/series-*.csv*',recursive=True)):
    tag=p.split('series-')[1].split('.csv')[0]
    by=collections.defaultdict(lambda: dict(b=0,it={}))
    for r in csv.DictReader(op(p)):
        y=int(float(r['year'])); by[y]['b']=int(r['bands']); by[y]['it'][r['name']]=(int(r['known']),int(r['make']),int(r['kit']))
    runs[tag]=by
def share(by,y,names,col):
    b=by[y]['b'];  return sum(by[y]['it'].get(n,(0,0,0))[col] for n in names)/b if b else None
print(f"seeds {len(runs)}")
print("year  | metal kit  stone kit | copper bronze iron steel (kit) | steel known | iron known  (means over seeds alive at that year)")
for c in CHK:
    rows=[]
    for tag,by in runs.items():
        ys=[y for y in by if y<=c]
        if not ys or max(by)<c: continue
        y=max(ys); 
        if not by[y]['b']: continue
        rows.append([share(by,y,METAL,2),share(by,y,STONE,2)]+[share(by,y,[m],2) for m in METAL]+[share(by,y,['steel knife'],0),share(by,y,['iron knife'],0)])
    if rows:
        m=[st.mean(r[i] for r in rows) for i in range(8)]
        print(f"Y{c:<4} | {m[0]:.2f}       {m[1]:.2f}     | {m[2]:.2f}   {m[3]:.2f}   {m[4]:.2f} {m[5]:.2f}       | {m[6]:.2f}        | {m[7]:.2f}   (n={len(rows)})")
print("\nper seed: first year known, years from first to 25/50/75% of bands with it in kit, kit share at Y600 and end")
for n in ['copper knife','iron knife','steel knife']:
    print(f"-- {n}")
    for tag,by in runs.items():
        ys=sorted(y for y in by if by[y]['it'].get(n,(0,0,0))[0]>0)
        if not ys: continue
        y0=ys[0]
        def t(f):
            a=[y for y in sorted(by) if y>=y0 and by[y]['b'] and by[y]['it'].get(n,(0,0,0))[2]/by[y]['b']>=f]; return f"{a[0]-y0}" if a else '-'
        y6=max([y for y in by if y<=600] or [0]); end=max(by)
        k6=share(by,y6,[n],2) if y6 and by[y6]['b'] else None; ke=share(by,end,[n],2) if by[end]['b'] else None
        print(f"   {tag:22} first Y{y0:<5} t25 {t(.25):>4} t50 {t(.5):>4} t75 {t(.75):>4} | kit Y600 {'-' if k6 is None else '%.2f'%k6}  end(Y{end}) {'-' if ke is None else '%.2f'%ke}")
