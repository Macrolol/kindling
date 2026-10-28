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
