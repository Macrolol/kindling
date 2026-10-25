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

## Step 1b: no walking across water; starts clear of straits

- `Map::walkable(a, b)`: a straight walk is allowed only if every cell on the line is land.
  Scouting, river scouting, a daughter's placement and a displacement on land all need it, so
  nobody steps over a strait any more (in v0.8.0 seed 1, #2 was "reached" that way).
- Founders start on good land on a continent at least `START_CLEAR` (6) cells from any other
  landmass, falling back to any good land; the founders' scatter stays on the origin landmass.
  This changes which world each seed gives.
- Test `no_walking_across_water_and_clear_starts`.

CI (run 40239259556, [results-v9-step1b-seeds1-10-500y.txt](results-v9-step1b-seeds1-10-500y.txt)),
**the honest baseline for later steps**: pop 993655 (v0.8.0 891377), war 0.371 %/yr, disease
0.133 %/yr, items 67.7, tau 0.14. Boat-borne continent crossings 4/10: s1 Y118 (gap 3), s2 Y192
(gap 10), s7 Y255 (gap 14), s8 Y196 (gap 1); none on foot, none before Y100. Effective lineages
5.27, largest 27.1 %. Seed 4 now starts on the large continent (pop 32k -> 91k).

## Step 2 (P1): rumours fade, look-outs keep looking, curiosity weights them

- `Band::far_shore_year` dates a rumour. It fades after `RUMOUR_YEARS` (50) unless seen again
  (`sight_shore`): a sighting of the same landmass refreshes it; a different shore replaces it
  unless the held one is nearer and still fresh (under 25 years old).
- Coastal bands keep watching after they have seen a shore, with chance `0.25 x curiosity`
  (capped at 0.9), and scouting no longer stops once a rumour is held.
- A rumour is passed to a neighbour (chance 0.3, keeping its date) only if the neighbour has none
  and the shore lies within twice the neighbour's boat range. A band without a boat accepts
  rumours only of very near shores, so landlubbers rarely relay them.
- Save format 9: migration `v8_to_v9` dates held rumours 40 years before the save; binary
  format-5..8 saves get the same in place (`age_old_rumours`). The v8 fixture test checks both.
- Local seed 1 x 500y: pop 148301, #2 by sea Y157 (gap 3), #3 by sea Y198 (gap 5), war 0.351
  %/yr, disease 0.136 %/yr, effective lineages 5.58 (largest 25 %), items 67.

CI (run 40239687085, [results-v9-step2-seeds1-10-500y.txt](results-v9-step2-seeds1-10-500y.txt)):
pop 1120438 (step 1b 993655), war 0.362 %/yr, disease 0.127 %/yr, items 67.9, tau 0.14. Crossings
in the same 4/10 worlds, at different dates: s1 Y157 (gap 3), s2 Y222 (gap 10), s7 Y246 (gap 14),
s8 Y158 (gap 1). Landings on another landmass rose 60 % (23005 -> 36727) but no world with a gap
of 26+ was crossed: rumours alone do not make long crossings; boats and skill limit them (as the
proposal expected). Effective lineages 5.29, largest 26.8 % (top on the main landmass 26.9 -> 24.2 %).

## Step 3 (P2): seamanship from all sea life, and taught

- `World::learn_sea(b, gain)` moves seacraft `gain` of the way to 1. Sources: boat fishing (as
  before); a completed voyage, `VOYAGE_LESSON` 0.03 + 0.01 per cell of open water (up to 10; was a
  flat 0.05); a scouting party that comes home (0.03); a visit to partners on another landmass
  (0.02 for each side with boats). Decay (x0.998 a season) and sea risk are unchanged.
- Seacraft is taught at meetings like recipes: a band better at sea by more than 0.1 teaches a
  coastal band with boats (chance 0.1, 0.2 between kin) a quarter of the gap. Inland peoples and
  bands without boats learn nothing.
- Sea-mammal hunting is not modelled (no such harvest), so it gives nothing yet.
- Measurement: a new headless line `seafaring (v9)`: seacraft p50 / p90 / max of bands with
  boats, sail-usable bands, the 90th percentile of coastal boat range, and the share of bands
  holding word of a far shore (and within range). `agg_results.py` reports seacraft and range p90
  and sail-usable bands.
- Local seed 1 x 500y: seacraft mean 0.31 -> 0.51 (p90 0.69, max 0.88), coastal range p90 20.4
  cells, no sail invented in that world; crossings #2 Y157, #3 Y188 (by sea); pop 109085, war
  0.347 %/yr, effective lineages 5.61 (largest 22 %).

CI: _pending_
