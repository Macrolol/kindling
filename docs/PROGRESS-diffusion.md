# Diffusion investigation: progress log

Goal (Michael, 2026-10-27): find which inventions are dropped quickly or fail to spread,
and why ("new technologies have a hard time overcoming the prevalence of old ones"). This
is diagnosis only. There are no balance changes; the findings and proposals go in
docs/REPORT-diffusion.md.

## 1. Mechanics read (v0.9.0, 0868efd)

- Kit choice (`band.rs make_kit`) ranks items per job by **material score only**. A
  recipe's quality (skill refinement) is ignored, so a newly producible better-material
  item always takes the job. Quality only scales effects afterwards (`refine_kit`: at most
  +10% yields, boats 0.3, protection 0.06).
- Practice (`world.rs economy`): the recipes in the production chain of the kit items
  actually used are marked `used`. Context gates skip some: craft away from water, nets
  with fish yield < 1, ornaments always, pitch unless needed, tallies unless the band
  keeps count.
- Forgetting (`band_season`): an unused recipe loses
  `0.003*(1+3/sqrt(pop))*(1-0.25*records)` skill per season. It is forgotten below 0.04,
  and quality fades too.
- Teaching (`exchange`) picks one random teachable recipe per meeting. Chance is
  `skill*(0.8 if the learner has the inputs at hand else 0.25)*prestige`. The learner
  starts at skill 0.3, so an unpractised taught recipe is gone in about 13 years
  (pop 25).
- Materials are those of the current cell plus carried stock, so producibility flips as
  bands move.

## 2. Instrumentation

- `civ-core/src/probe_diffusion.rs`: an ignored test (`probe_diffusion`), read-only.
  After every season it recomputes each band's toolkit and kit and classifies every
  known recipe as used, or idle for one of these reasons: no_input / redundant /
  idle_slot / displaced / unneeded. It tracks (band, recipe) lifetimes and writes
  items/series/forgets CSVs. Env: PROBE_SEEDS, PROBE_YEARS, PROBE_W, PROBE_H,
  PROBE_EVERY, PROBE_OUT.
- It is compiled only under `cfg(test)`. Check (seed 3, 128x80, 200 y, `--out` +
  `--save`): before and after the change, the `.civsave` (sha256 803d68d9…) and
  frames/chronicle are byte-identical; the manifest differs only in git_sha/timing/paths.
  The probe's world matches the CLI run: 1644 bands, pop 34874 at year 200.
- Workflow: new `probe` input on headless-sims.yml. The `probe` job runs on the same seed
  matrix and uploads `results-probe-<chunk>`, which goes into the run-<id> archive.

## 3. Historical data (CI archives + docs/results-inv-step*.txt)

- Tools: `tools/diffusion_hist.py` reads the archived run-<id> releases (frames `lore` +
  chronicle), and `tools/diffusion_hist_agg.py` aggregates them per step.
- Churn: across steps 0-13 forgot/learned stays at 0.86-0.88 on the default map (seeds
  1-10, 500 y) and 0.89-0.92 on 800x800 (750 y). That is 1.4-1.8M learn events per seed.
- The animal-product family (hide thong/bag/net/clothing, rawhide snare/bow, bone
  point/beads, bone-tipped spear, cooked meat/fish) peaks at 0.4-0.98 of bands and ends at
  0.10-0.34, with the same shape in every step and on both map sizes.
- **Caveat found:** frames `lore[1]` and `LostWorldwide` count bands *able to make* an
  item now (`World::item_bands` = closure of what is available this season), not bands
  that *know* it. The historical "collapse" therefore mixes input loss with knowledge
  loss. The probe separates the two (see 4).
- Pitch/sealed jar dropped from 0.92 to 0.17 at step 13.2. That is the step-12
  resinous-wood change, a design change and not diffusion.

## 4. Local probe (seed 1, 128x80, 600 y, RAYON_NUM_THREADS=4)

- `tools/diffusion_probe_agg.py <dir>` aggregates the probe CSVs.
- Forgets by reason: no_input 58%, displaced 19%, unneeded 16%, idle_slot 4%, redundant 3%.
  71% of forgotten recipes were never practised after the season they were learned in.
- Taught recipes arrive producible only 48% of the time; 70% of taught recipes are
  later forgotten (vs 25% of those inherited at a split).
- Hide thong is *known* by 0.85-0.98 of bands throughout but *makeable* by only
  0.07-0.18 after year 180. Cooked meat is makeable by 0.99 → 0.07 by year 600. Most of the
  family's collapse is game/hide input scarcity, not lost know-how.
- Kit choice ignores quality. New hide net / rawhide snare replace plant-fibre ones
  although refined payoff is worse in ~80% of comparisons.
- Ornaments are never practised (`economy` skips `kit.ornament`), so bone beads are
  forgotten (40% idle_slot) and lost (peak 0.62 → 0).
- CI runs (probe=true): 40606688049 (128x80, 8 seeds, 1500 y), 40606696457 (192x120,
  8 seeds, 1200 y), 40606704688 (256x160, 5 seeds, 1000 y).

## 5. Hide/bone supply check (`probe_hide`, seed 1, 128x80, 600 y, local)

- Hide/bone stock gains `hunted*0.01` per season and loses 0.02, plus 0.06 when `needed` and
  not `present`. For hide/bone `present` falls to the DEPOSITS branch and is always false,
  so a band using hide items needs a big-game harvest of >= 8 per season to break even
  (>= 2 without the penalty).
- Hide-needing bands hunt 3.1-4.5 on average; only 3-14% reach 8. That holds even at year
  100, when 96% of game remains (9%). Game left is 0.61 at y200, 0.40 at y300 and 0.13 at y600.
- So the hide/bone family is starved structurally, by the stock budget, well before game
  depletion; depletion adds to it later.

## 6. CI probe results across map sizes

Runs 40606688049 (128x80, 8 seeds, 1500 y) and 40606696457 (192x120, 8 seeds, 1200 y).
Aggregates: `tools/diffusion_probe_agg.py`, `tools/diffusion_series.py`.

| metric | 128x80 | 192x120 |
|---|---|---|
| forgets: no_input / displaced / unneeded / redundant / idle_slot % | 58.7 / 22.9 / 11.2 / 4.6 / 2.7 | 58.6 / 24.6 / 9.3 / 4.3 / 3.1 |
| taught recipes arriving producible % | 44.5 | 44.0 |
| taught / inherited / invented gains later forgotten % | 73.6 / 28.9 / 78.4 | 70.8 / 28.5 / 75.8 |
| forgotten without practice after the learning season % | 75.5 | 77.9 |

So the local seed-1 picture holds across seeds and both sizes.

Metal and other "better" tech (2nd-half means of known / make / kit):
- copper knife 0.76/0.38/0.14 and 0.76/0.42/0.13: makeable but in the kit only a third of the time.
  `make_kit` ranks edges by `cut_q*(0.75+0.25*dur)`: flint 0.625, obsidian 0.641, copper 0.480,
  bronze 0.650, iron 0.640, steel 0.776. Copper loses to flint, and iron loses to obsidian. But
  `weapon_q` (0.55 cut + 0.45 dur) and upkeep (`1.15 - dur`) both rate metal well above flint.
- copper-tipped spear 0.68/0.36/0.17 and 0.54/0.33/0.11: flint spear 0.695 vs copper 0.662.
- copper axe 0.71-0.76 / 0.38-0.40 / 0.24-0.36: copper wins the axe slot (0.740 vs stone 0.449).
- plank boat 0.74-0.81 / 0.39-0.55 / same; woven mat sail 0.66-0.72 / 0.38-0.48; sealed jar
  0.81-0.88 / 0.21-0.27 (100% of jar forgets are no_input: pitch); fired clay token kit 0.59-0.72.
  Wherever these can be made they are used, so their limit is inputs, not incumbency.
- nailed plank ship: 1 seed (192x120), peak 0.30, 95% of forgets no_input (nails + pitch).
- There are no drying-rack or chart items in the item table. Smoking and sky lore are flags
  or skills and are not tracked by the per-recipe probe.

### 6b. 256x160 (40606704688, all 5 seeds)

- Probe artifacts were downloaded with `gh run download -n results-probe-<c>`.
- Forget reasons: no_input 60.8, displaced 23.5, unneeded 7.4, redundant 5.0, idle_slot 3.3%.
- Taught recipes arriving producible: 42.1%. 77.6% of forgotten recipes were never
  practised.
- Copper knife 0.81/0.46/0.12, plank boat make = kit 0.51, bone beads 0.62 → 0.12.
  Same picture as the other sizes.
- Charts (summary.json, 128x80 + 192x120): first chart in 10/16 seeds, peak <= 17 bands,
  0 at end in 14/16.
- Report written: docs/REPORT-diffusion.md.

## D. Diffusion step D1-D6 (approved 2026-10-27, decisions in REPORT-diffusion.md section 8)

- D0 (in progress): decisions recorded. Baselines are probe runs 40606688049 (128x80),
  40606696457 (192x120) and 40606704688 (256x160), plus step 13 checks 40588153931 (a) and
  40588163981 (b).
- D1 hide/bone at hand for hunters (`World::mat_at_hand`: `Mat::Hide | Mat::Bone` present when
  this season's big-game harvest `h[1]+0.3h[3] > 1`, so the 0.06 missing-input drain stops while
  hunting goes on). Tests `d1_hide_and_bone_at_hand_for_hunters`, `d1_hide_stock_holds_for_hunters`;
  v10 fixture run-on hash updated. Smoke `probe_hide` seed 1 128x80 600 y (81 s): has_hide now
  tracks need_hide (0.52/0.51 y200, 0.33/0.35 y600). Hunting
  pressure about unchanged: game left 0.55 y200 (was 0.61), 0.37 y300 (0.40), 0.19 y600 (0.13).
  Full suite 66 passed.
- D2 worn ornaments practised (`economy`: the kit's ornament goes into `used`, so `mark_used`
  walks its chain, with no upkeep and no `needed` bits). Test `d2_ornament_practised_without_wear`;
  v10 run-on hash updated. Smoke `probe_diffusion` seed 1 128x80 600 y (111 s), share of bands
  able to make it at y600 (D1+D2): hide thong 0.64, bone beads 0.64, cooked meat 0.27 (pre-D probe
  8-seed means: 0.12, 0.09 and 0.28). Full suite 67 passed.
- D3 kit by expected payoff, with trials (`band::make_kit_for`; `make_kit(avail)` is the fully
  practised view). Payoff slots (hunt, cut, point, axe, boat, net, trap, bow, shield,
  outrigger, plank, ship) rank by `q x (0.5+0.5 skill) x (1+QUALITY_YIELD quality) -
  KIT_UPKEEP x (1.15-dur) x (1+0.3 depth)`, blades by `edge_q` (0.55 cut + 0.45 dur). Edge
  values: flint 0.55, obsidian 0.51, copper 0.71, bronze 0.81, iron 0.80, steel 0.87. A fresh
  copper knife (skill 0.3) loses to honed flint and takes over at skill ~0.4.
  `Kit::trial` holds each slot's challenger; `economy` tries it with `KIT_TRIAL_P` 0.2 a season
  (hash of seed, band, tick, slot) in seasons the slot is used, adding its wear and stock draw.
  The kit is re-chosen once a season after skills move (`kit_dirty`) and on toolkit-cache
  swaps. The probe now reads the real kit choice (`make_kit_for` / `make_kit_among`).
  Tests `d3_metal_blades_outrank_stone_when_practised`, `d3_newcomer_is_tried_then_takes_over`,
  `d3_trials_about_one_season_in_five`; v10 run-on hash updated; full suite 70 passed.
  Smoke: headless seed 1 128x80 600 y in 53 s (2832 bands, pop 54k at y600; D2 probe run had
  1807 bands, 31.5k). Probe at y600 (one seed, D2 -> D3): steel knife known 0.14 -> 0.99,
  kit 0.13 -> 0.65; iron knife known 0.26 -> 0.93 (used 0.37, kit 0.03: kept in trial); copper
  knife gone (displaced by steel); flint-tipped spear kit 0.31 -> 0.05; cooked meat make
  0.27 -> 0.06 and hide thong 0.64 -> 0.23 (more people on the same game). To watch in the CI checks.
- D4 teaching what the learner can use (`exchange`: the recipe shown is picked with weight
  `teach_weight` = 1 + `TEACH_USABLE` (3) x the share of its inputs the learner has, instead of
  uniformly; listening odds unchanged). Test `d4_teaching_weighted_to_usable_recipes`.
  `refined_recipes_make_better_kits` now compares only bands whose chosen kit stays the same,
  because since D3 a finer recipe can also change which tool is relied on. v10 run-on hash
  updated; full suite 71 passed. Smoke probe seed 1 128x80 600 y (275 s), D3 -> D4: taught
  recipes producible on arrival 49.9% -> 55.4% (D2 run: 64.9%); taught-and-forgotten 66.9% ->
  68.5%; forget reasons no_input 55.4 -> 50.5%, displaced 25.8 -> 31.5%.
