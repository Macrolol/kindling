#!/usr/bin/env python3
"""Aggregate Kindling headless results across seeds.

Works on any concatenation of per-seed output (one `kindling --headless` process per seed or
one multi-seed process), unlike the cross-seed lines that only a multi-seed run prints.
Usage: agg_results.py results.txt [baseline.txt]   (baseline: print a side-by-side table)
"""
import re
import sys


def parse(path):
    seeds = {}
    cur = None
    for line in open(path):
        m = re.match(r'seed (\d+): Y(\d+) pop (\d+) .*?\| ([\d.]+)s \(gen', line)
        if m:
            s = int(m.group(1))
            if s in seeds and 'war' in seeds[s]:
                cur = None  # repeated headline (multi-seed summary block): skip
                continue
            cur = seeds.setdefault(s, {})
            cur.update(seed=s, year=int(m.group(2)), pop=int(m.group(3)), t=float(m.group(4)),
                       stage=re.search(r'stage now: (\w+)', line).group(1))
            continue
        if cur is None:
            continue
        if 'continents settled' in line:
            ys = [int(y) for y in re.findall(r'gap \d+ Y(\d+) pop', line)]
            cur['cross'] = sorted(y for y in ys if y > 0)
            cur['cont'] = re.search(r'settled (\d+/\d+)', line).group(1)
        elif re.match(r'\s+sea:', line):
            cur['lost'] = int(re.search(r'lost (\d+) people', line).group(1))
            cur['voy'] = int(re.search(r'voyages (\d+)', line).group(1))
            cur['land'] = int(re.search(r'landed on another landmass (\d+)', line).group(1))
        elif re.match(r'\s+trade:', line):
            f = re.search(r'first: (.*)', line)
            f = f.group(1) if f else ''
            for metal in ('iron', 'bronze', 'copper'):
                mm = re.search(r'(?:^|, )%s Y(\d+)' % metal, f)
                cur[metal] = int(mm.group(1)) if mm else None
        elif re.match(r'\s+conflict:', line):
            cur['war'] = float(re.search(r'killed = ([\d.]+)%', line).group(1))
            cur['dis'] = float(re.search(r'disease deaths ([\d.]+)%', line).group(1))
            cur['fights'] = float(re.search(r'fights \(([\d.]+)/century\)', line).group(1))
        elif re.match(r'\s+farming:', line):
            cur['orig'] = int(re.search(r'independent origins (\d+)', line).group(1))
            d = re.search(r'domestic grain (\S+)', line).group(1)
            cur['dom'] = int(d[1:]) if d.startswith('Y') else None
        elif re.match(r'\s+farming now:', line):
            m = re.search(r'\((\d+) people\) vs \d+ foraging bands \((\d+) people\)', line)
            fp, gp = int(m.group(1)), int(m.group(2))
            cur['forag'] = gp / max(1, fp + gp)
            cur['ffood'] = int(re.search(r'food from farming (\d+)%', line).group(1))
            cur = None
    return [seeds[k] for k in sorted(seeds) if 'forag' in seeds[k]]


def agg(rows):
    n = len(rows)
    mean = lambda k: sum(r[k] for r in rows) / n
    cross = [(r['seed'], r['cross'][0]) for r in rows if r.get('cross')]
    dom = [r['dom'] for r in rows if r['dom']]
    return {
        'seeds': n,
        'total pop': sum(r['pop'] for r in rows),
        'killed in war %/yr (mean)': round(mean('war'), 3),
        'disease %/yr (mean)': round(mean('dis'), 3),
        'fights/century (mean)': round(mean('fights'), 0),
        'farming (dom. grain) seeds': len(dom),
        'first domestic grain': f"Y{min(dom)}-Y{max(dom)}" if dom else '-',
        'farming origins (mean)': round(mean('orig'), 1),
        'foragers % of people': f"{min(r['forag'] for r in rows)*100:.0f}-{max(r['forag'] for r in rows)*100:.0f}% (mean {mean('forag')*100:.0f}%)",
        'second continent seeds': f"{len(cross)}/{n}: " + ', '.join(f"s{s} Y{y}" for s, y in cross),
        'crossings before Y100': sum(1 for _, y in cross if y < 100),
        'voyages / landings': f"{sum(r['voy'] for r in rows)} / {sum(r['land'] for r in rows)}",
        'lost at sea': sum(r['lost'] for r in rows),
        'iron seeds': sum(1 for r in rows if r.get('iron')),
        'bronze seeds': sum(1 for r in rows if r.get('bronze')),
        'sim seconds (sum)': round(sum(r['t'] for r in rows), 1),
    }


def main():
    a = agg(parse(sys.argv[1]))
    if len(sys.argv) > 2:
        b = agg(parse(sys.argv[2]))
        w = max(len(k) for k in a)
        print(f"| metric | {sys.argv[2].split('/')[-1]} | {sys.argv[1].split('/')[-1]} |")
        print("|---|---|---|")
        for k in a:
            print(f"| {k} | {b[k]} | {a[k]} |")
    else:
        for k, v in a.items():
            print(f"{k:30} {v}")
    print()
    for r in parse(sys.argv[1]):
        print(f"s{r['seed']:<3} pop {r['pop']:>7} war {r['war']:.3f}% dis {r['dis']:.3f}% fights/c {r['fights']:>7.1f} orig {r['orig']:>2} dom {('Y%d' % r['dom']) if r['dom'] else 'never':>5} "
              f"forag {r['forag']*100:3.0f}% cross {r.get('cross')} lost {r['lost']} iron {r.get('iron')} bronze {r.get('bronze')} {r['t']}s")


if __name__ == '__main__':
    main()
