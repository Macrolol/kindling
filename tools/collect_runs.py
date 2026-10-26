#!/usr/bin/env python3
"""Collect `kindling --headless --out DIR` structured outputs (docs/DATA_SCHEMA.md) into tables.

Usage: collect_runs.py DIR [OUTDIR]
  DIR     searched recursively for seed-*/summary.json (+ manifest.json, metrics.csv), e.g. a
          folder of downloaded CI artifacts
  OUTDIR  (default: DIR) gets
            runs.csv         one row per seed: manifest + headline + conflict numbers
            metrics-all.csv  every seed's metrics.csv with a leading `seed` column
Prints a one-line overview per seed. Exits non-zero on an unknown schema version.
"""
import csv
import glob
import json
import os
import sys

SCHEMAS = {1}

RUN_COLS = ['seed', 'git_sha', 'version', 'map_w', 'map_h', 'start_year', 'end_year', 'threads', 'save_format',
            'save_bytes', 't_gen', 't_run', 't_save', 'loaded_from']
SUM_COLS = ['pop', 'bands', 'items_now', 'items_mean', 'stage_now', 'stage_ever', 'inventions', 'experiments',
            'transfers', 'forgets', 'splits', 'deaths', 'range_cells']
CONF_COLS = ['confrontations', 'avoided', 'tributes', 'fled', 'fights', 'fights_per_century', 'deaths',
             'death_rate_pct', 'displaced', 'fled_by_sea', 'absorbed', 'captives', 'lineages', 'top_lineage_pct',
             'disease_rate_pct']


def runs(root):
    for f in sorted(glob.glob(os.path.join(root, '**', 'summary.json'), recursive=True)):
        d = os.path.dirname(f)
        sm = json.load(open(f))
        if sm.get('schema') not in SCHEMAS:
            sys.exit(f"{f}: data schema {sm.get('schema')} not understood (know {sorted(SCHEMAS)})")
        mf = os.path.join(d, 'manifest.json')
        man = json.load(open(mf)) if os.path.exists(mf) else {}
        yield d, sm, man


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    root = sys.argv[1]
    out = sys.argv[2] if len(sys.argv) > 2 else root
    os.makedirs(out, exist_ok=True)
    seen = {}
    for d, sm, man in runs(root):
        seen.setdefault(sm['seed'], (d, sm, man))  # first copy of a seed wins
    if not seen:
        sys.exit(f"no seed-*/summary.json under {root}")
    with open(os.path.join(out, 'runs.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(RUN_COLS + SUM_COLS + ['conflict_' + c for c in CONF_COLS])
        for seed in sorted(seen):
            d, sm, man = seen[seed]
            row = [man.get(c, sm['seed'] if c == 'seed' else '') for c in RUN_COLS]
            row += [sm['summary'].get(c, '') for c in SUM_COLS]
            row += [sm['conflict'].get(c, '') for c in CONF_COLS]
            w.writerow(['' if v is None else v for v in row])
            print(f"s{seed}: Y{sm['summary']['years']:.0f} pop {sm['summary']['pop']} stage {sm['summary']['stage_now']} "
                  f"fights/c {sm['conflict']['fights_per_century']:.1f} | sha {man.get('git_sha', '?')[:12]} "
                  f"run {man.get('t_run', 0):.1f}s save {man.get('save_bytes') or '-'} B")
    header = None
    with open(os.path.join(out, 'metrics-all.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        for seed in sorted(seen):
            p = os.path.join(seen[seed][0], 'metrics.csv')
            if not os.path.exists(p):
                continue
            rows = list(csv.reader(open(p)))
            if header is None:
                header = rows[0]
                w.writerow(['seed'] + header)
            elif rows[0] != header:
                sys.exit(f"{p}: columns differ from the first seed's metrics.csv")
            for r in rows[1:]:
                w.writerow([seed] + r)
    print(f"{len(seen)} seeds -> {out}/runs.csv, {out}/metrics-all.csv")


if __name__ == '__main__':
    main()
