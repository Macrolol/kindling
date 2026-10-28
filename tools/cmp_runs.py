#!/usr/bin/env python3
"""Compare two headless-sims artifact trees (old, new): old fields must match exactly;
fields only in new must be zero. Timings/git sha ignored."""
import sys, os, json, csv, re, glob
old, new = sys.argv[1], sys.argv[2]
IGN = {"git_sha", "t_gen", "t_run", "t_save", "started_unix", "elapsed", "wall", "secs"}
issues = []; newnz = {}; stats = {"seeds":0,"frames":0,"rows":0,"chron":0}
def walk(a, b, path):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a:
            if k in IGN or k.startswith("t_"): continue
            if k not in b: issues.append(f"{path}.{k} missing in new"); continue
            walk(a[k], b[k], f"{path}.{k}")
        for k in b:
            if k not in a and not (k in IGN or k.startswith("t_")): zero(b[k], f"{path}.{k}")
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b): issues.append(f"{path} len {len(a)} vs {len(b)}"); return
        for i,(x,y) in enumerate(zip(a,b)): walk(x,y,f"{path}[{i}]")
    elif a != b: issues.append(f"{path}: {a!r} vs {b!r}")
def zero(v, path):
    if isinstance(v, dict):
        for k,x in v.items(): zero(x, f"{path}.{k}")
    elif isinstance(v, list):
        for x in v: zero(x, path)
    elif v not in (0, 0.0, None, False, "", "0", "0.0", "0.000"):
        try:
            if float(v) == 0: return
        except Exception: pass
        p = re.sub(r"\[\d+\]", "", path); newnz[p] = newnz.get(p, 0) + 1
def seeds(root):
    out = {}
    for m in glob.glob(f"{root}/results-*/data/seed-*"):
        out[os.path.basename(m)] = m
    return out
so, sn = seeds(old), seeds(new)
if set(so) != set(sn): issues.append(f"seed sets differ {sorted(set(so)^set(sn))}")
for s in sorted(so):
    if s not in sn: continue
    a, b = so[s], sn[s]; stats["seeds"] += 1
    for f in ("manifest.json", "summary.json"):
        walk(json.load(open(f"{a}/{f}")), json.load(open(f"{b}/{f}")), f"{s}/{f}")
    ca = open(f"{a}/chronicle.ndjson","rb").read(); cb = open(f"{b}/chronicle.ndjson","rb").read()
    if ca != cb:
        la, lb = ca.splitlines(), cb.splitlines()
        if len(la) != len(lb): issues.append(f"{s} chronicle lines {len(la)} vs {len(lb)}")
        for i,(x,y) in enumerate(zip(la,lb)): walk(json.loads(x), json.loads(y), f"{s}/chronicle[{i}]")
    stats["chron"] += ca.count(b"\n")
    fa = open(f"{a}/frames.ndjson").read().splitlines(); fb = open(f"{b}/frames.ndjson").read().splitlines()
    if len(fa) != len(fb): issues.append(f"{s} frames {len(fa)} vs {len(fb)}")
    for i,(x,y) in enumerate(zip(fa,fb)): walk(json.loads(x), json.loads(y), f"{s}/frames"); stats["frames"] += 1
    ra = list(csv.DictReader(open(f"{a}/metrics.csv"))); rb = list(csv.DictReader(open(f"{b}/metrics.csv")))
    if len(ra) != len(rb): issues.append(f"{s} metrics rows {len(ra)} vs {len(rb)}")
    for x,y in zip(ra,rb):
        stats["rows"] += 1
        for k in x:
            if k.startswith("t_") or k in IGN: continue
            if x[k] != y.get(k): issues.append(f"{s}/metrics y{x.get('year')} {k}: {x[k]} vs {y.get(k)}")
        for k in y:
            if k not in x: zero(y[k], f"metrics.{k}")
# text results: strip timings
def txt(root):
    t = {}
    for f in glob.glob(f"{root}/results-*/results-*.txt"):
        L = open(f).read()
        L = re.sub(r"\|\s*[\d.]+s( \(gen [\d.]+s\))?", "|", L)
        t[os.path.basename(f)] = L
    return t
ta, tb = txt(old), txt(new)
for k in ta:
    if k in tb and ta[k] != tb[k]:
        import difflib
        d = [l for l in difflib.unified_diff(ta[k].splitlines(), tb[k].splitlines(), lineterm="", n=0) if l[:1] in "+-" and l[:3] not in ("+++","---")]
        issues.append(f"{k}: {len(d)} differing text lines, e.g. {d[:4]}")
print(f"compared {stats['seeds']} seeds, {stats['frames']} frames, {stats['rows']} metrics rows, {stats['chron']} chronicle events")
print(f"mismatches in old fields: {len(issues)}")
for i in issues[:40]: print("  ", i)
print(f"new-only fields with nonzero values: {len(newnz)}")
for k,v in sorted(newnz.items())[:60]: print(f"   {k}: {v} nonzero")
