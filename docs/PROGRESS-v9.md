# Progress, v9: human motivations

Implements [PROPOSAL-motivation.md](PROPOSAL-motivation.md) (4d301dc) in its build order. No
release is tagged. Every step is checked with the `headless-sims` workflow (10 seeds x 500 years,
standard map) against v0.8.0 ([results-v8-D7-seeds1-10-500y.txt](results-v8-D7-seeds1-10-500y.txt))
and, from step 1b on, against the honest-measurement baseline.

Targets: real boat-borne crossings to a real continent in about 4-6 of 10 worlds, at different
times, none before Y100; population and war near v0.8.0 (war deaths 0.25-0.45 %/yr); no runaway
invention; lineage dominance falling.

## Step 1a: honest measurement (no change to the simulation)

- The continent cutoff scales with land area: `Map::continent_cells()` = max(400, land/100). On
  the standard map (about 12k land cells) it stays 400; on 1200x1200 it rises.
- Each landfall is recorded (`World::landfalls`, additive `serde(default)`, so no format bump):
  landmass, tick, band, whether it came by sea, the landmass it sailed from, and the open-water gap
  from any landmass already settled (`Map::sea_gaps`, a multi-source sea BFS). A band is "by sea"
  only if it put to sea this season (`Derived::sailed_from`: a voyage, a sea displacement, a
  daughter that sailed, or drift). The Landfall event now says "on foot", "by <boat>" or "adrift".
- The headless report tags each continent `[sea, N from settled]`, `[foot]` or (old saves) `[?]`,
  and adds a boat-borne crossing line, plus effective lineages (1/sum p^2) overall, per landmass
  and per century, with the top lineage per landmass.
- `tools/agg_results.py` parses the new fields and still reads older result files.
- Verified byte-identical with v0.8.0 on seed 1 x 500y (only the new annotations differ): there
  the second continent (#2, gap 3, Y143) was in fact reached **on foot**; #3 (Y172) by sea.
- Format-8 fixture (v0.8.0, seed 4, Y150, 96x60) and a load test added.

CI (run 40239005648, [results-v9-step1a-seeds1-10-500y.txt](results-v9-step1a-seeds1-10-500y.txt)):
identical to v0.8.0 on all 10 seeds (pop 891377, war 0.356 %/yr, disease 0.111 %/yr, items 66.4,
tau 0.14). Honest reading of v0.8.0: boat-borne continent crossings in 4/10 worlds (s1 Y172 gap 5,
s2 Y195 gap 10, s7 Y302 gap 14, s8 Y188 gap 1); s1's Y143 "crossing" was on foot. Effective
lineages 5.39, largest lineage 25.8 %.
