#!/usr/bin/env python3
"""Compare probe_diffusion outputs of two runs (old_dir new_dir): columns present in the old
items/series/forgets CSVs must match exactly (forgets as a multiset: HashMap order), and the
probe log must match apart from timings and lines the new build adds."""
import sys, os, re, csv, glob, gzip, collections
old, new = sys.argv[1], sys.argv[2]
op = lambda p: gzip.open(p, 'rt') if p.endswith('.gz') else open(p)
def files(root, kind):
    return {re.sub(r'\.csv(\.gz)?$', '', os.path.basename(p)).split(kind + '-')[1]: p
            for p in glob.glob(f'{root}/**/{kind}-*.csv*', recursive=True)}
issues = []; n = collections.Counter()
for kind in ('items', 'series', 'forgets'):
    fo, fn = files(old, kind), files(new, kind)
    if set(fo) != set(fn): issues.append(f'{kind}: seed sets differ {sorted(set(fo) ^ set(fn))}')
    for tag in sorted(set(fo) & set(fn)):
        ro = list(csv.DictReader(op(fo[tag]))); rn = list(csv.DictReader(op(fn[tag])))
        cols = list(ro[0].keys()) if ro else []
        miss = [c for c in cols if rn and c not in rn[0]]
        if miss: issues.append(f'{kind}-{tag}: columns gone {miss}')
        cols = [c for c in cols if c not in miss]
        a = [tuple(r[c] for c in cols) for r in ro]; b = [tuple(r[c] for c in cols) for r in rn]
        if kind == 'forgets': a, b = sorted(a), sorted(b)
        if a != b:
            d = sum(1 for x, y in zip(a, b) if x != y) + abs(len(a) - len(b))
            issues.append(f'{kind}-{tag}: {d} of {len(a)} rows differ')
        n[kind] += len(a)
strip = lambda s: re.sub(r'\(\d+(\.\d+)?s\)|\b\d+(\.\d+)?\s*(s|ms)\b', '', s).rstrip()
lo = {os.path.basename(p): p for p in glob.glob(f'{old}/**/probe-*.log', recursive=True)}
ln = {os.path.basename(p): p for p in glob.glob(f'{new}/**/probe-*.log', recursive=True)}
for k in sorted(set(lo) & set(ln)):
    a = [strip(l) for l in open(lo[k], errors='replace') if not l.startswith(('test ', 'running', '   Compiling', '    Finished', '     Running'))]
    b = set(strip(l) for l in open(ln[k], errors='replace'))
    gone = [l for l in a if l and l not in b and not re.search(r'test result|finished in|Running|elapsed', l)]
    if gone: issues.append(f'{k}: {len(gone)} old log lines not in new, e.g. {gone[:3]}')
    n['logs'] += 1
hashes = lambda root: sorted(set(m for p in glob.glob(f'{root}/**/probe-*.log', recursive=True) for m in re.findall(r'state_hash\s*[:=]?\s*([0-9a-f]{8,})', open(p, errors='replace').read())))
ho, hn = hashes(old), hashes(new)
print(f"compared rows: {dict(n)}; state hashes old {len(ho)} new {len(hn)} {'equal' if ho == hn else 'DIFFER'}")
print(f'mismatches: {len(issues)}')
for i in issues[:30]: print('  ', i)
