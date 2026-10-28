# Kindling users and makers (skill, trials, beliefs, tool choice): progress log

Started 28 Oct 2026. Plan: `docs/PROPOSAL-skills.md` (revision 4, approved by Michael on Oct 28
with decisions 1-18 in section 7; steps 0-8 in section 5). Written against `main` 7231103 (the
D1-D6 diffusion build, `docs/REPORT-diffusion.md` section 9).

Rules: one local sim at a time, RAYON_NUM_THREADS=4, every local command under `timeout`;
batches run on GitHub Actions (`headless-sims.yml`, archive=true, archive_saves=false); only short
`gh run view` polls; push after each sub-step; no tags or releases.

Checks for every step (proposal section 5):
- **(a)** 10 seeds x 500 years, default 256x160 map (seeds 1..10). D1-D6 baseline run
  40620593419 (`docs/results-diff-seeds1-10-500y.txt`): war 0.320 %/y, crossings 5/10.
- **(b)** 10 seeds x 750 years, 800x800 (seeds 499246292..499246301). D1-D6 baseline run
  40620602590 (`docs/results-diff-750y-800x800.txt`): war 0.271 %/y, crossings 1/10; seed
  499246298 dies early, as expected.
- **Probes** (`probe: true`): 128x80 (seeds 1..8, 1500 y), 192x120 (seeds 1..8, 1200 y), 256x160
  (seeds 1..5, 1000 y). D1-D6 baselines 40620611442, 40620619645, 40620627632
  (`docs/results-diff-probes.txt`).

Guard rails: no continent crossing before Y100; (b) under 7/10, (a) at most 6/10; war rate not
doubled against D1-D6; from step 6 no single arms race over 25% of bands or half the map width,
and races in half the (b) seeds; from step 8 raider bands under 10% of bands, one region at most;
no population collapse.

| # | step | status |
|---|---|---|
| 0 | Measure first: probe fixes (3.8), new counters and YearStats fields (zero yet) | in progress |
| 1 | Layer 1 and blade scoring | not started |
| 2 | User skill by form, save 16 | not started |
| 3 | Maker skill | not started |
| 4 | Beliefs | not started |
| 5 | Layer 2: risk and trials | not started |
| 6 | Layer 2: materials, fight need, fight beliefs, arms races | not started |
| 7 | Calibration | not started |
| 8 | Raiding as a way of life (decision 17) | not started |

## Decisions recorded (Oct 28 2026)
- 17: raiding ships with this proposal, as step 8.
- 18: grief dissent (`DISSENT_GRIEF`) accepted as the one new cost mechanism.

## Step 0. Measure first (in progress)

### 0a. Code (Oct 28)

- `civ-core/src/probe_diffusion.rs` (proposal 3.8): kit choices scored by a layer-1-like true
  value (`true_value`: food multipliers + boat + protection + fine work, less the upkeep of the
  payoff-slot tools, plus `PROBE_FIGHT_W` 0.1 x (weapon_q + 1 - loss_guard)), by its food part
  (`food`) and by the band's own valuation (`own_value`, the slot pay `make_kit_with` ranks by);
  `PROBE_TIE` 0.02. Adoption is measured when a slot's pick changes, against the kit with the
  replaced item forced back (forced = the old item cannot be made any more); held is yearly,
  `worse_held_share`. "Believed wrong" reads NA until beliefs (step 4). New `skills-*.csv` and
  `pairs-*.csv`; the old `worse_refined_share` / `gain_mean` columns are kept on the D1-D6
  payoff for comparison. `tools/diffusion_probe_agg.py` prints the split.
- 27 new counters and 26 new `YearStats` fields, all 0 until the step that fills them
  (`docs/DATA_SCHEMA.md`, "Skills step 0"); test `skills_step0_fields_stay_zero`.
- Local checks (rustc 1.99.1, RAYON_NUM_THREADS=4): `cargo test --release --workspace` passes
  (78 civ-core tests, golden hashes unchanged). Probe seed 1, 128x80, 200 years: `state_hash
  6df29787462b70ce` before and after; `series` identical, `forgets` identical as a set of rows
  (HashMap order), the old `items` columns identical. Headless seed 3, 128x80, 300 years
  (`--out`, `--save`), old build vs new: text report identical but for timings; chronicle
  identical; `summary.json` and every frame identical apart from the 27 new zero counter keys;
  `metrics.csv` 300 rows, all 149 old columns identical, 53 new columns all 0; manifest differs
  only in paths and timings; the save is 4.7 kB larger (the new fields), as expected.
- Local probe (seed 1, 128x80, 200 y): 10 766 adoptions (5 244 forced); of the 5 522 with a
  choice, worse in true food 7.3 % (tie 37.5 %), worse in true value 7.3 % (tie 35.2 %), worse by
  own valuation 0.0 %; worse_held_share 9.3 % (food 8.9 %). The worst pairs: sharpened-stick
  spear held over bone- or flint-tipped spears (skill-ranked), stone blade over flint blade, iron
  knife over flint blade (cause 2).

### 0b. GitHub check (dispatched Oct 28, 11:05 AM EDT, build 960dad8)

- (a) seeds 1-10, 500 y, 256x160: run 40712451626 (compare 40620593419)
- (b) seeds 499246292-301, 750 y, 800x800: run 40712466759 (compare 40620602590)
- probes: 128x80 seeds 1-8 1500 y run 40712480983; 192x120 seeds 1-8 1200 y run 40712496067;
  256x160 seeds 1-5 1000 y run 40712511412 (sim_timeout 300)
- The three probe runs were cancelled at 11:3x AM EDT: with (a)/(b) they held all 20 free-plan
  job slots and starved the others. Re-dispatched one at a time (below).
- (a) and (b) identical to the pre-step-0 runs (79d5fd4) apart from the new zero fields and
  timings: `tools/cmp_runs.py OLD NEW` checks every manifest/summary.json/frames/metrics/chronicle
  field and the per-job text; 0 mismatches, 0 nonzero new fields on both.
  docs/results-skills-step0-seeds1-10-500y.txt, docs/results-skills-step0-750y-800x800.txt.
- Probe re-dispatch: 128x80 seeds 1-8 1500 y run 40725221814 (12:21 PM EDT); once (a)/(b) had
  finished, 192x120 seeds 1-8 1200 y run 40732114919 and 256x160 seeds 1-5 1000 y run 40732132799
  (sim_timeout 300) alongside it (1:04 PM EDT).
