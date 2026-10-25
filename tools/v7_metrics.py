#!/usr/bin/env python3
"""v7 metrics from Kindling headless output (any concatenation of per-seed runs).

Per seed: first iron bloom and bronze (discovery years), largest lineage share at the end and its
maximum over the century samples. Across seeds: Kendall-tau distance of discovery order over all
seed pairs (same definition as civ-core analysis::order_distance).
Usage: v7_metrics.py results.txt [baseline.txt]
"""
import re
import sys
from itertools import combinations

ITEM = re.compile(r'^\s+Y\s*([\d.]+) (.+?)\s{2,}inv ')


def parse(path):
    seeds, cur = {}, None
    for line in open(path):
        m = re.match(r'seed (\d+): Y\d+ pop (\d+)', line)
        if m:
            s = int(m.group(1))
            if s in seeds and seeds[s].get('done'):
                cur = None
                continue
            cur = seeds.setdefault(s, {'order': [], 'year': {}, 'pop': int(m.group(2))})
            continue
        if cur is None:
            continue
        mi = ITEM.match(line)
        if mi:
            name = mi.group(2).strip()
            k, key = 1, name
            while key in cur['year']:
                k += 1
                key = f'{name}#{k}'
            cur['order'].append(key)
            cur['year'][key] = float(mi.group(1))
            continue
        ml = re.search(r'lineages \d+ \(largest (\d+)% of people; by century \[([\d, ]+)\]', line)
        if ml:
            cur['lin'] = int(ml.group(1))
            cur['lin_c'] = [int(x) for x in ml.group(2).split(',')]
            cur['done'] = True
            cur = None
    return {s: v for s, v in seeds.items() if v.get('done')}


def tau(a, b):
    common = [i for i in a if i in b]
    if len(common) < 3:
        return None
    pa = {x: i for i, x in enumerate(a)}
    pb = {x: i for i, x in enumerate(b)}
    d = t = 0
    for x, y in combinations(common, 2):
        t += 1
        d += (pa[x] < pa[y]) != (pb[x] < pb[y])
    return d / t


def first(y, pred):
    v = [t for k, t in y.items() if pred(k)]
    return min(v) if v else None


def summary(seeds):
    # v7+ (bellows exist): "iron bloom" is the unforged bloom and usable iron is "wrought iron";
    # before v7 "iron bloom" was the usable metal
    v7 = any('bellows' in k for v in seeds.values() for k in v['year'])
    rows, extra = [], {}
    for s in sorted(seeds):
        v = seeds[s]
        y = v['year']
        iron = y.get('wrought iron') if v7 else y.get('iron bloom')
        extra[s] = (first(y, lambda k: 'bellows' in k), y.get('iron bloom') if v7 else None)
        rows.append((s, iron, y.get('bronze (metal)'), v['lin'], max(v['lin_c']), v['lin_c']))
    summary.extra = extra
    ts = [tau(seeds[a]['order'], seeds[b]['order']) for a, b in combinations(sorted(seeds), 2)]
    ts = [t for t in ts if t is not None]
    return rows, sum(ts) / max(1, len(ts)), len(ts)


def fmt(y):
    return f'Y{y:.0f}' if y is not None else 'never'


def main():
    cur = summary(parse(sys.argv[1]))
    base = summary(parse(sys.argv[2])) if len(sys.argv) > 2 else None
    ex = summary(parse(sys.argv[1])) and summary.extra
    for name, (rows, t, n) in [('this run', cur)] + ([('baseline', base)] if base else []):
        print(f'== {name}: tau {t:.3f} over {n} pairs')
        print('| seed | usable iron | bellows | bloom | bronze | iron first? | lineage Y500 | lineage max (centuries) |')
        print('|---|---|---|---|---|---|---|---|')
        for s, i, b, l, lm, lc in rows:
            first = 'iron' if i is not None and (b is None or i < b) else ('bronze' if b is not None else '-')
            bl, bm = ex.get(s, (None, None)) if name == 'this run' else (None, None)
            print(f'| {s} | {fmt(i)} | {fmt(bl)} | {fmt(bm)} | {fmt(b)} | {first} | {l}% | {lm}% {lc} |')
        irons = [r[1] for r in rows if r[1] is not None]
        brs = [r[2] for r in rows if r[2] is not None]
        med = lambda v: sorted(v)[len(v) // 2] if v else None
        nf = sum(1 for r in rows if r[1] is not None and (r[2] is None or r[1] < r[2]))
        print(f'iron seeds {len(irons)} (median {fmt(med(irons))}), bronze seeds {len(brs)} (median {fmt(med(brs))}), iron before bronze in {nf}; lineage max share mean {sum(r[4] for r in rows)/len(rows):.1f}%, worst {max(r[4] for r in rows)}%')


if __name__ == '__main__':
    main()
