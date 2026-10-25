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

CI (run 40240330260, [results-v9-step3-seeds1-10-500y.txt](results-v9-step3-seeds1-10-500y.txt)):
pop 1037037 (step 2 1120438, step 1b 993655), war 0.371 %/yr, disease 0.131 %/yr, items 69.7,
tau 0.14; iron in 7 worlds (4). Crossings in the same 4/10 worlds, later than in step 2 in three
of them: s1 Y157, s2 Y308, s7 Y294, s8 Y190. Seacraft p90 0.41-0.69 (mean 0.53) and coastal boat
range p90 17-20 cells in every world; only s5 invented a sail (466 sail-usable bands). The
uncrossed gaps from settled land are 21-59 cells: beyond every world's range, and in s9, s10 and
s6 nobody has even seen the far shore. Seamanship is now lived and taught, but boats, not skill,
cap the range. Effective lineages 5.33, largest 26.1 %.

## Step 4 (P4): dissent, crowding splits, the bold emigrate

- `Band::ambition` (heritable, log scale, clamped to +-1; founders sd 0.3, drift sd 0.12 a split)
  and `Band::dissent` (additive `serde(default)` fields; old saves load with 0 = ordinary, calm).
- Dissent follows friction at `DISSENT_RATE` 0.1 a season: push-out (hunger, crowding, pressure)
  x size relative to the split size (up to 1.5) x e^ambition. A band of 60 %+ of split size with
  dissent above 0.5 may break up early (chance 0.5 x the excess a season). Those who leave in
  anger look for land up to 4 cells away (not 2), take ship more often (0.8 vs 0.5) and aim for a
  known far shore more often (0.9 vs 0.7). They carry the bold tail: the daughter's curiosity and
  ambition move up by `EMIGRANT_TAIL` x |gauss|, the parent's down in proportion to its size;
  the quarrel leaves with them (parent dissent x0.3, daughter 0).
- Ambition widens a voyage's reach (range x e^(0.25 x ambition); the sea toll then charges for
  it) and stiffens resolve in a confrontation (x e^(0.15 x ambition)). Raids do not yet use it.
- Measurement: `Counters::dissent_splits`; headless line `motives (v9)`: splits by cause, mean
  dissent, curiosity and ambition overall and off the home landmass.
- Local seed 1 x 500y: 24 % of splits after dissent; mean ambition drifted to -0.13 (the bold
  die more at sea and in fights), overseas bands no bolder than at home (-0.13, curiosity 0.99 vs
  1.03): the tail is diluted by later local splits. Crossings #2 Y134 (gap 3), #3 Y202 (gap 5),
  pop 132747 (109085 in step 3), war 0.336 %/yr, disease 0.124, effective lineages 5.51 (26 %).

CI (run 40241143469, [results-v9-step4-seeds1-10-500y.txt](results-v9-step4-seeds1-10-500y.txt)):
pop 1053876 (step 3 1037037), war 0.370 %/yr, disease 0.122 %/yr, items 65.4 (69.7), tau 0.15.
Crossings in the same 4/10 worlds: s1 Y134, s2 Y197, s7 Y327, s8 Y184. 22-43 % of splits now
follow dissent. Mean ambition drifts below 0 in 8 worlds (the bold die more), but where people
crossed the sea the overseas bands are bolder than at home (s2 +0.40 vs +0.19, s7 +0.09 vs -0.17,
s8 +0.01 vs -0.05) and in s7 more curious (1.22 vs 1.03): founder voyages select the bold, as
intended, without a runaway. Effective lineages 5.41 (5.33), largest 25.2 % (26.1).
