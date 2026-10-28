#!/usr/bin/env python3
"""Compare probe_diffusion outputs of two runs (old_dir new_dir): columns present in the old
items/series/forgets CSVs must match exactly (forgets as a multiset: HashMap order), and the
probe log must match apart from timings and lines the new build adds.

Streams every CSV (the forgets tables of the bigger maps are several GB once parsed): items and
series row by row, forgets as an order-free multiset digest (row count plus two 64-bit sums of
per-row blake2b hashes), so memory stays flat."""
import sys, os, re, csv, glob, gzip, collections, hashlib, itertools
old, new = sys.argv[1], sys.argv[2]
csv.field_size_limit(1 << 30)
op = lambda p: gzip.open(p, 'rt', newline='') if p.endswith('.gz') else open(p, newline='')
def files(root, kind):
    return {re.sub(r'\.csv(\.gz)?$', '', os.path.basename(p)).split(kind + '-')[1]: p
            for p in glob.glob(f'{root}/**/{kind}-*.csv*', recursive=True)}
M = (1 << 64) - 1
def digest(rows):
    n = s1 = s2 = 0
    for t in rows:
        h = hashlib.blake2b('\x1f'.join(t).encode(), digest_size=16).digest()
        n += 1; s1 = (s1 + int.from_bytes(h[:8], 'little')) & M; s2 = (s2 + int.from_bytes(h[8:], 'little')) & M
    return n, s1, s2
issues = []; n = collections.Counter()
for kind in ('items', 'series', 'forgets'):
    fo, fn = files(old, kind), files(new, kind)
    if set(fo) != set(fn): issues.append(f'{kind}: seed sets differ {sorted(set(fo) ^ set(fn))}')
    for tag in sorted(set(fo) & set(fn)):
        ro = csv.reader(op(fo[tag])); rn = csv.reader(op(fn[tag]))
        ho = next(ro, []); hn = next(rn, [])
        miss = [c for c in ho if c not in hn]
        if miss: issues.append(f'{kind}-{tag}: columns gone {miss}')
        io = [i for i, c in enumerate(ho) if c in hn]; inn = [hn.index(ho[i]) for i in io]
        a = (tuple(r[i] for i in io) for r in ro); b = (tuple(r[i] for i in inn) for r in rn)
        if kind == 'forgets':
            da, db = digest(a), digest(b)
            if da != db: issues.append(f'{kind}-{tag}: multisets differ (rows {da[0]} old, {db[0]} new)')
            n[kind] += da[0]
        else:
            d = rows = 0
            for x, y in itertools.zip_longest(a, b):
                d += x != y; rows += x is not None
            if d: issues.append(f'{kind}-{tag}: {d} of {rows} rows differ')
            n[kind] += rows
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
