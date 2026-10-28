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
