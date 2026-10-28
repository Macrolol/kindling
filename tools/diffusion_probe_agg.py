"""Aggregate probe_diffusion outputs (items-*.csv[.gz], forgets-*.csv[.gz]) across seeds."""
import csv,glob,gzip,sys,collections,math,os
def op(p): return gzip.open(p,'rt') if p.endswith('.gz') else open(p)
root=sys.argv[1]
items=collections.defaultdict(list)
for p in glob.glob(f'{root}/**/items-*.csv*',recursive=True):
    tag=os.path.basename(p).split('items-')[1].split('.csv')[0]
    for r in csv.DictReader(op(p)): r['tag']=tag; items[r['name']].append(r)
nseeds=len({r['tag'] for v in items.values() for r in v})
F=lambda r,k: float(r[k]) if r[k] not in('','NaN','nan') else math.nan
def mean(xs):
    xs=[x for x in xs if not math.isnan(x)]; return sum(xs)/len(xs) if xs else math.nan
def tot(v,k): return sum(F(r,k) for r in v if not math.isnan(F(r,k)))
R=['no_input','redundant','idle_slot','displaced','unneeded']
print(f"seeds: {nseeds}")
print("item                      seeds first peak  final kit/know  taught taught_ok% forgets f/gain | reasons% "+' '.join(x[:6] for x in R)+" | yrs never% <20y_taught | inc q_new q_inc worse_ref gain extinct")
rows=[]
for nm,v in items.items():
    gained=tot(v,'gained_invented')+tot(v,'gained_inherited')+tot(v,'gained_taught')
    fr=[tot(v,'forget_'+x) for x in R]; nf=sum(fr)
    nfo=tot(v,'forgets_invented')+tot(v,'forgets_inherited')+tot(v,'forgets_taught')
    t=tot(v,'gained_taught'); tok=tot(v,'taught_ok')
    rows.append((mean([F(r,'first_year') for r in v]),nm,len(v),mean([F(r,'peak_share') for r in v]),sum(F(r,'final_known') for r in v)/max(1,sum(1 for r in v)),mean([F(r,'final_share') for r in v]),
        tot(v,'bs_kit')/max(1,tot(v,'bs_known')),t,100*tok/max(1,tot(v,'taught_ok')+tot(v,'taught_bad')),nfo,nfo/max(1,gained),[100*x/max(1,nf) for x in fr],
        sum(F(r,'mean_years_known_at_forget')*(F(r,'forgets_invented')+F(r,'forgets_inherited')+F(r,'forgets_taught')) for r in v if not math.isnan(F(r,'mean_years_known_at_forget')))/max(1,nfo),
        100*tot(v,'forget_never_used')/max(1,nfo),100*tot(v,'forget_taught_within_20y')/max(1,nfo),tot(v,'inc_n'),
        mean([F(r,'q_new_mean') for r in v]),mean([F(r,'q_inc_mean') for r in v]),mean([F(r,'worse_refined_share') for r in v]),mean([F(r,'gain_mean') for r in v]),tot(v,'extinctions')))
for x in sorted(rows):
    print(f"{x[1][:25]:25} {x[2]:3} {x[0]:5.0f} {x[3]:.2f}  {x[5]:.2f}  {x[6]:.2f}  {x[7]:8.0f}  {x[8]:5.1f}  {x[9]:8.0f} {x[10]:.2f} | "+' '.join(f"{y:6.1f}" for y in x[11])+f" | {x[12]:5.1f} {x[13]:5.1f} {x[14]:5.1f} | {x[15]:7.0f} {x[16]:.2f} {x[17]:.2f} {x[18]:.2f} {x[19]:+.3f} {x[20]:.0f}")
# global
G=collections.Counter()
for v in items.values():
    for r in v:
        for k in ['gained_invented','gained_inherited','gained_taught','gained_start','taught_ok','taught_bad','forgets_invented','forgets_inherited','forgets_taught','forgets_band_gone','forget_never_used','forget_taught_within_20y']+['forget_'+x for x in R]:
            G[k]+=F(r,k)
print('\nTOTALS', dict(G))
nf=sum(G['forget_'+x] for x in R)
print('forget reasons %:', {x: round(100*G['forget_'+x]/nf,1) for x in R})
print('taught arriving producible %:', round(100*G['taught_ok']/(G['taught_ok']+G['taught_bad']),1))
print('forgets by origin % of that origin gained:', {o: round(100*G['forgets_'+o]/max(1,G['gained_'+o]),1) for o in ['invented','inherited','taught']})
print('forgotten without ever being practised beyond the learning season %:', round(100*G['forget_never_used']/nf,1))
