#!/usr/bin/env python3
"""Aggregate Kindling headless results across seeds.

Works on any concatenation of per-seed output (one `kindling --headless` process per seed or
one multi-seed process), unlike the cross-seed lines that only a multi-seed run prints.
Usage: agg_results.py RESULTS [BASELINE]   (baseline: print a side-by-side table)
RESULTS / BASELINE: a text report (results.txt) or a structured-output directory written by
`kindling --headless --out DIR` (summary.json + manifest.json per seed, docs/DATA_SCHEMA.md);
both give the same rows, so old text baselines compare against new structured runs.
"""
import glob
import json
import os
import re
import sys

# the part of a continent entry between "#id" and its year, old or new format
CONT = r'\d+c (?:gap \d+ )?(?:home )?'


def r0(x):
    return int(round(x)) if x is not None else None


def row_from_json(sm, man):
    """One seed's row (same keys as the text parser) from summary.json (+ manifest.json)."""
    s, cf = sm['summary'], sm['conflict']
    r = dict(seed=sm['seed'], year=r0(s['years']), pop=s['pop'], stage=s['stage_now'], items=s['items_now'],
             t=round((man or {}).get('t_gen', 0) + (man or {}).get('t_run', 0), 1))
    cs = s['continents']
    r['cross'] = sorted(r0(c['landfall_year']) for c in cs if c['landfall_year'] is not None and r0(c['landfall_year']) > 0)
    r['cont'] = f"{sum(1 for c in cs if c['landfall_year'] is not None)}/{len(cs)}"
    if any(c['landfall_year'] is not None and not (c['landfall_year'] == 0 and c.get('by_sea') is None) for c in cs):
        r['boat'] = sorted((r0(c['landfall_year']), str(c['gap_settled']) if c['gap_settled'] is not None else '-')
                           for c in cs if c.get('by_sea') is True and c['landfall_year'] is not None and r0(c['landfall_year']) > 0)
        r['foot'] = sorted(r0(c['landfall_year']) for c in cs if c.get('by_sea') is False and c['landfall_year'] is not None and r0(c['landfall_year']) > 0)
    r['order'] = [f['item'] for f in s['firsts']]
    l = s['lineages']
    r['eff'] = round(l['effective'], 2)
    r['top_lm'] = r0(l['by_landmass'][0]['top_pct']) if l['by_landmass'] else None
    q, mo = s['sea'], s['motives']
    r.update(lost=q['lost_at_sea'], voy=q['voyages'], land=q['crossings'], xp90=round(q['seacraft_p90'], 2),
             sail=q['sail_bands'], rng90=round(q['range_p90'], 1), anc_top=r0(l['ancestry_top_pct']),
             anc_eff=round(l['ancestry_effective'], 2), wed=mo['marriages'], inlaw=mo['fights_in_law'],
             refine=mo['refinements'], quality=round(mo['quality_mean'], 2), boatq=round(mo['boat_q_mean'], 2),
             pmean=round(mo['prestige_mean'], 2), pgini=round(mo['prestige_gini'], 2), ptop=r0(mo['prestige_top10_pct']),
             ptaught=r0(mo['prestige_taught_pct']))
    for metal in ('iron', 'bronze', 'copper'):
        ys = [y for y, m in s['trade']['first_traded'] if m == metal]
        r[metal] = r0(ys[0]) if ys else None
    r.update(war=round(cf['death_rate_pct'], 3), dis=round(cf['disease_rate_pct'], 3), fights=round(cf['fights_per_century'], 1),
             top=r0(cf['top_lineage_pct']) if cf['lineages'] > 0 else None)
    g = s['farming']
    r['orig'] = len(g['origins'])
    r['dom'] = r0(g['first_domestic_grain'])
    r['forag'] = g['forager_pop'] / max(1, g['farmer_pop'] + g['forager_pop'])
    r['ffood'] = r0(100 * g['farm_food_share'])
    return r


def parse_dir(path):
    rows = {}
    for f in sorted(glob.glob(os.path.join(path, '**', 'summary.json'), recursive=True)):
        sm = json.load(open(f))
        mf = os.path.join(os.path.dirname(f), 'manifest.json')
        man = json.load(open(mf)) if os.path.exists(mf) else None
        rows[sm['seed']] = row_from_json(sm, man)
    return [rows[k] for k in sorted(rows)]


def parse(path):
    if os.path.isdir(path):
        return parse_dir(path)
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
                       stage=re.search(r'stage now: (\w+)', line).group(1),
                       items=int(re.search(r'items now (\d+)', line).group(1)))
            continue
        if cur is None:
            continue
        if 'continents settled' in line:
            # old lines: "#1 1234c gap 12 Y356 pop ..."; newer: "#1 1234c Y356 pop ..." and
            # "#2 999c home Y0 pop ..." (gaps now come from settled land, in the [sea, ..] tag)
            ys = [int(y) for y in re.findall(CONT + r'Y(\d+) pop', line)]
            cur['cross'] = sorted(y for y in ys if y > 0)
            cur['cont'] = re.search(r'settled (\d+/\d+)', line).group(1)
            # v9: how each continent was first reached ("[sea, G from settled]" / "[foot]")
            if '[sea' in line or '[foot]' in line or '[?]' in line:
                cur['boat'] = sorted((int(y), g) for y, g in re.findall(CONT + r'Y(\d+) pop \d+ \[sea, (\S+) from settled\]', line) if int(y) > 0)
                cur['foot'] = sorted(int(y) for y in re.findall(CONT + r'Y(\d+) pop \d+ \[foot\]', line) if int(y) > 0)
        elif re.match(r'\s+discovery order:', line):
            cur['order'] = [int(x) for x in line.split(':', 1)[1].split()]
        elif re.match(r'\s+lineages:', line):
            cur['eff'] = float(re.search(r'effective ([\d.]+)', line).group(1))
            t = re.search(r'top L\d+ (\d+)%', line)
            cur['top_lm'] = int(t.group(1)) if t else None
        elif re.match(r'\s+sea:', line):
            cur['lost'] = int(re.search(r'lost (\d+) people', line).group(1))
            cur['voy'] = int(re.search(r'voyages (\d+)', line).group(1))
            cur['land'] = int(re.search(r'landed on another landmass (\d+)', line).group(1))
        elif re.match(r'\s+seafaring \(v9\):', line):
            cur['xp90'] = float(re.search(r'p90 ([\d.]+)', line).group(1))
            cur['sail'] = int(re.search(r'sail-usable bands (\d+)', line).group(1))
            cur['rng90'] = float(re.search(r'range p90 ([\d.]+)', line).group(1))
        elif re.match(r'\s+ancestry \(v9\):', line):
            cur['anc_top'] = int(re.search(r'largest founder (\d+)%', line).group(1))
            cur['anc_eff'] = float(re.search(r'effective founders ([\d.]+)', line).group(1))
            cur['wed'] = int(re.search(r'(\d+) weddings', line).group(1))
            cur['inlaw'] = int(re.search(r'(\d+) fights between in-laws', line).group(1))
        elif re.match(r'\s+refinement \(v9\):', line):
            cur['refine'] = int(re.search(r'(\d+) improvements', line).group(1))
            cur['quality'] = float(re.search(r'mean recipe quality ([\d.]+)', line).group(1))
            cur['boatq'] = float(re.search(r'boat seaworthiness ([\d.]+)', line).group(1))
        elif re.match(r'\s+prestige \(v9\):', line):
            cur['pmean'] = float(re.search(r'mean ([\d.]+)', line).group(1))
            cur['pgini'] = float(re.search(r'Gini ([\d.]+)', line).group(1))
            cur['ptop'] = int(re.search(r'hold (\d+)%', line).group(1))
            cur['ptaught'] = int(re.search(r'(\d+)% of recipes taught', line).group(1))
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
            t = re.search(r'largest (\d+)% of people', line)
            cur['top'] = int(t.group(1)) if t else None
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


def order_distance(a, b):
    """Share of item pairs found in opposite order (same as civ_core::analysis::order_distance)."""
    common = [i for i in a if i in b]
    if len(common) < 3:
        return None
    pa = {x: k for k, x in enumerate(a)}
    pb = {x: k for k, x in enumerate(b)}
    disc = tot = 0
    for i in range(len(common)):
        for j in range(i + 1, len(common)):
            tot += 1
            if (pa[common[i]] < pa[common[j]]) != (pb[common[i]] < pb[common[j]]):
                disc += 1
    return disc / tot


def tau(path, rows=None):
    """Kendall-tau discovery-order distance: from a multi-seed run's summary block, or else
    computed from the per-seed "discovery order:" lines (one job per seed). None if neither."""
    for line in ([] if os.path.isdir(path) else open(path)):
        m = re.search(r'Kendall tau, 0 same .. 1 reversed\): mean ([\d.]+)', line)
        if m:
            return float(m.group(1))
    orders = [r['order'] for r in (rows or []) if r.get('order')]
    ds = [d for i in range(len(orders)) for j in range(i + 1, len(orders))
          if (d := order_distance(orders[i], orders[j])) is not None]
    return round(sum(ds) / len(ds), 2) if ds else None


def agg(rows, tau_v=None):
    n = len(rows)
    mean = lambda k: sum(r[k] for r in rows) / n
    have = lambda k: [r[k] for r in rows if r.get(k) is not None]
    boat = [(r['seed'], r['boat'][0]) for r in rows if r.get('boat')]
    v9 = any('boat' in r for r in rows)
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
        'boat-borne continent crossing seeds (v9)': (f"{len(boat)}/{n}: " + ', '.join(f"s{s} Y{y} (gap {g})" for s, (y, g) in boat)) if v9 else '-',
        'continents reached on foot (v9)': (', '.join(f"s{r['seed']} Y{r['foot'][0]}" for r in rows if r.get('foot')) or 'none') if v9 else '-',
        'effective lineages (mean)': round(sum(have('eff')) / len(have('eff')), 2) if have('eff') else '-',
        'largest lineage % (mean)': round(sum(have('top')) / len(have('top')), 1) if have('top') else '-',
        'top lineage on main landmass % (mean)': round(sum(have('top_lm')) / len(have('top_lm')), 1) if have('top_lm') else '-',
        'ancestry: effective founders / largest % (mean, v9)': (f"{sum(have('anc_eff'))/len(have('anc_eff')):.2f} / {sum(have('anc_top'))/len(have('anc_top')):.1f}" if have('anc_eff') else '-'),
        'weddings / in-law fights (sum, v9)': (f"{sum(have('wed'))} / {sum(have('inlaw'))}" if have('wed') else '-'),
        'recipe quality / boat seaworthiness (mean, v9)': (f"{sum(have('quality'))/len(have('quality')):.2f} / {sum(have('boatq'))/len(have('boatq')):.2f}" if have('quality') else '-'),
        'refinements (mean, v9)': (round(sum(have('refine')) / len(have('refine'))) if have('refine') else '-'),
        'prestige mean / Gini / top-10% share (mean, v9)': (f"{sum(have('pmean'))/len(have('pmean')):.2f} / {sum(have('pgini'))/len(have('pgini')):.2f} / {sum(have('ptop'))/len(have('ptop')):.0f}%" if have('pmean') else '-'),
        'taught by the more prestigious % (mean, v9)': (round(sum(have('ptaught')) / len(have('ptaught'))) if have('ptaught') else '-'),
        'items now (mean)': round(mean('items'), 1),
        'discovery-order tau': tau_v if tau_v is not None else '-',
        'voyages / landings': f"{sum(r['voy'] for r in rows)} / {sum(r['land'] for r in rows)}",
        'lost at sea': sum(r['lost'] for r in rows),
        'seacraft p90 / coastal range p90 (mean, v9)': (f"{sum(have('xp90'))/len(have('xp90')):.2f} / {sum(have('rng90'))/len(have('rng90')):.1f}" if have('xp90') else '-'),
        'sail-usable bands (sum, v9)': (sum(have('sail')) if have('sail') else '-'),
        'iron seeds': sum(1 for r in rows if r.get('iron')),
        'bronze seeds': sum(1 for r in rows if r.get('bronze')),
        'sim seconds (sum)': round(sum(r['t'] for r in rows), 1),
    }


def main():
    ra = parse(sys.argv[1])
    a = agg(ra, tau(sys.argv[1], ra))
    if len(sys.argv) > 2:
        rb = parse(sys.argv[2])
        b = agg(rb, tau(sys.argv[2], rb))
        w = max(len(k) for k in a)
        # columns: baseline (second argument) first, then the new results; label by
        # the last two path parts so two results-all.txt downloads are told apart
        name = lambda p: '/'.join(p.rstrip('/').split('/')[-2:])
        print(f"| metric | baseline: {name(sys.argv[2])} | new: {name(sys.argv[1])} |")
        print("|---|---|---|")
        for k in a:
            print(f"| {k} | {b[k]} | {a[k]} |")
    else:
        for k, v in a.items():
            print(f"{k:30} {v}")
    print()
    for r in parse(sys.argv[1]):
        print(f"s{r['seed']:<3} pop {r['pop']:>7} war {r['war']:.3f}% dis {r['dis']:.3f}% fights/c {r['fights']:>7.1f} orig {r['orig']:>2} dom {('Y%d' % r['dom']) if r['dom'] else 'never':>5} "
              f"forag {r['forag']*100:3.0f}% cross {r.get('cross')} boat {r.get('boat')} lost {r['lost']} iron {r.get('iron')} bronze {r.get('bronze')} {r['t']}s")


if __name__ == '__main__':
    main()
