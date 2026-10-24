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

## Final 500y candidate (q13, sea change) docs/results-v5-seeds1-10-500y.txt
- Sea: second continent in 4/10 (s1 Y152, s8 Y162, s2 Y226, s7 Y307); none before Y100; lost at sea 38.5k (v4 57.1k).
- Farming: 9/10 seeds; s7 never; domestic grain Y241..Y400; farmers 16-56% of people at Y500, foragers persist.
- Conflict 0.332%/yr, disease 0.065-0.199%/yr; largest lineage 30% mean (s7 55%); tau 0.13.
- Seed 2: flat ~35k Y100-Y300 (small landmass, no farming), then farming + crossing -> 79k.
- Time 730s total vs v4 531s under load ~10 (pop 1.57x v4).

## Next
1. 2 seeds x 1500y (s1, s7), sequential.
2. Tests, builds (linux/windows/web), screenshot, Page, report.
