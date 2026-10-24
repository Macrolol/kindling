# v5 progress (farming + sea + lineage)

Updated 2026-10-24 16:55 EDT.

## Done
- Farming stage with wild progenitor patches (grain/herd), sowing persistence, herding from wild herds (4cd3d0a).
- Conflict: meeting cap 4/season, settled-defender shelter.
- Yearly lineage stats (largest lineage %, lineages alive) in YearStats, stats panel chart, longrun_report.

## Current numbers (q12 = 4cd3d0a, 10 seeds x 500y, 256x160) docs/results-v5-q12-seeds1-10-500y.txt
- Farming: 9/10 seeds farm, s7 never, s8 only 1% at Y500; domestic grain Y217..Y483; foragers persist everywhere (45-99%).
- Pop vs v4: s2 43k (v4 33k), s3 133k (100k), s5 222k (110k); ceiling rises with farming.
- Conflict 0.336%/yr (target 0.3-0.45), disease 0.08-0.20%/yr.
- Sea: second continent in 3/10 (s1 Y152, s8 Y160, s2 Y457); none before Y100; lost 47k vs 19.6k landings on other landmasses (2 per landing).
- Perf: 733s total vs v4 531s (box load 8-10 from other users' processes; pop is larger).

## Next
1. Sea: slightly easier (target 4-6/10) and fewer deaths per voyage: range grows more with experience, capsizes near shore mostly survive.
2. DONE: beta fixture + test; fixed save round-trip drift (bands[].carry).
3. Final 10x500 + 2x1500, tests, builds, Page, screenshot.
