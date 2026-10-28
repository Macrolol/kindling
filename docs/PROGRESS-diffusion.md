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
