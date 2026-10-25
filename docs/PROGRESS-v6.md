# Kindling v6: multi-core step + start picker. Progress log

Rules: one sim at a time, RAYON_NUM_THREADS=4, every run under `timeout`, no parallel batches.
Box: 8 cores, but 8 to 9 other users' `pyRing` processes keep every core at about 90% (load 9 to 12),
so a 4-thread run gets roughly one core of real CPU. Wall times here therefore **cannot** show
parallel scaling. They do show the single-thread CPU cost. CPU (user) seconds are the honest
metric on this box.

## Commits
- 616855f: CLI map clamp raised from 600x400 to 2048x2048.
- 47769fc: core parallel season, contacts and regrow (rayon, feature `parallel`), Fx effect
  buffers, chosen starts (`Config.starts`, `start_check`, `parse_starts`), sanity test.
- b819286: viewer `--threads`, `--start`/`--starts`, thread setting, fast mode (X), throttled
  parallel map redraw, start picker in Load / new world.
- 95e56c0: fair-share harvest of contested stocks, meetings on band pairs by reference, round
  planner, mimalloc on native.

## Step log
### 1. Tests and cross-build (done, 95e56c0)
- `cargo test -p civ-core --features parallel`: 11 passed, 1 ignored (farm_diag).
- `cargo test -p civ-core` (no rayon): 10 passed, 1 ignored.
  Covered: v1, v2, v3, v4 and v5beta fixtures load and run; save round trip; sanity and
  conservation; chosen starts; same result on any thread count.
- The windows-gnu cross-build with mimalloc compiles.

### 2. Benchmarks (done, /workspace/bench/bench2.log, 19:31-19:55 EDT)
800x800, loaded from big800_y600.civsave, +20 years (Y600 to Y620, about 43k to 45k bands, pop 0.84 to 0.91M):

| build | threads | wall s | user CPU s | pop Y620 |
|---|---|---|---|---|
| v5 | 1 | 77.1 | 66.7 | 838677 |
| v6 (par4) | 1 | 97.3 | 82.5 | 907079 |
| v6 (par4) | 2 | 103.4 | 85.6 | 907079 |
| v6 (par4) | 4 | 82.0 | 80.9 | 907079 |

- The t1 and t2 results are identical (same pop), so the run is deterministic across thread
  counts even though that is no longer required.
- Single-thread CPU cost is about 24% above v5.
- The t2 run used no more CPU than t1, so on this box the second thread got no real core.

Profile (wall-clock phases; sub-entries are thread-seconds):
- v5: band seasons 36.9 (economy 6.9, experiments 10.6, movement 6.5, other 7.0), contacts 29.1 (meetings 21.5), stats 6.6
- v6 t1: band seasons 52.1 (economy incl. toolkit refresh 18.8, experiments 15.4, movement 6.7, other 3.2), contacts 34.3 (meetings 25.5), stats 5.1

Default map (300x200), seed 5, fresh world, 500 years (ends at about 11k to 15k bands):

| build | threads | wall s | user CPU s | pop Y500 |
|---|---|---|---|---|
| v5 | 1 | 180.2 | 155.1 | 203374 |
| v6 (par4) | 1 | 232.6 | 189.3 | 275635 |
| v6 (par4) | 2 | 313.7 | 207.1 | 275635 |
| v6 (par4) | 4 | 350.5 | 218.7 | 275635 |

- All thread counts give the identical world again (deterministic).
- Single-thread CPU cost on the default map: +22% over v5.
- t2/t4 are *slower* in wall time here: with every core already taken by other processes, the
  worker threads are descheduled and each of the ~8000 parallel regions per run waits at its
  barrier for the slowest one. This is box contention, not a property of the code; it cannot be
  measured away here.
- On 800x800 the t4 wall time (82.0 s) beat t2 (103.4 s), but still not t1 (97.3 vs 82.0: t4 is
  16% faster than t1, about what one extra part-core gives).

Laptop estimate (an estimate, not measured): about 80% of v6 single-thread time is in the
parallel phases (band seasons, neighbour search, meetings, regrow). Amdahl with that fraction gives
about 2.5x at 4 free cores and 3.3x at 8 over v6 t1; minus barrier/merge overhead, roughly 2 to
2.5x at 4 cores, i.e. about 1.6 to 2x faster than v5 single-thread, and roughly 2.5x over v5 at 8
cores. Large maps benefit most (more bands per parallel region).

### 3. Balance check, first try (done, 20:01-20:20 EDT): **off, not accepted**
`kindling-par4 --threads 1 --headless --seeds 1..10 --years 500` (build 95e56c0, fair-share harvest),
docs/results-v6a-sharedharvest-seeds1-10-500y.txt (+ -summary.txt), vs v5:

| metric (10 seeds) | v5 | v6a fair share |
|---|---|---|
| total pop Y500 | 909k | 1121k (+23%) |
| fights per century (mean) | 4480 | 7116 (+59%) |
| killed in war, %/yr | 0.332 | 0.466 |
| farming origins (mean) | 16.5 | 11.7 |
| farming share of people Y500 (mean) | 35% | 29% |
| voyages / landings / lost at sea | 188k / 19.7k / 38.5k | 204k / 14.0k / 33.5k |
| scouting trips | 2229 | 404 |

Pop is higher on 9 of 10 seeds and war deaths on 10 of 10, so this is systematic, not noise.
Cause: when home ranges overlap, v5 has each later band see what earlier bands left and work
harder for its food; planning all bands on the start stock and then sharing out the take leaves
the stocks fuller and the bands working less, which is a balance change.

Fix in progress (step 4): phase 2 now takes harvests in band order exactly like v5; a band whose
cells an earlier band already took from this season re-plans its economy against the live stock
(sequentially). Uncontested bands keep their parallel plan, which is then exact.

## Known findings
- **Map clamp:** until 616855f the CLI silently clamped `--map` to 600x400, so a "1650x1650" CLI
  world was really 600x400. The viewer's own size fields allow 2048.
- **cell_first** (item -> per-cell first tick) is dense: 4 bytes x cells x items. At 1650x1650
  with about 70 items that is about 760 MB of RAM. It is saved too (deflate shrinks it, because
  it is mostly u32::MAX). This is the main memory/save-size cost on huge maps. A sparse form
  would need a save-format bump; not done.
- **Balance-relevant changes from parallelising** (seeds differ from v5):
  - every band plans its harvest against the start-of-season stocks, and contested cells are
    shared fairly (`share_harvest`): +8% pop at 800x800 Y620 (907k vs 839k). Without the sharing
    it was +44%;
  - farm layers and world-firsts are read as at the start of the season;
  - an invention is not usable in the same season;
  - meetings run in band-disjoint rounds.

### 4. Sequential harvest (done, c365b39) and balance check 2 (20:21-20:38 EDT): closer, **not yet accepted**
Phase 2 plans each band's economy and takes its harvest in band order, exactly as v5 (only the
toolkit refresh stays parallel). Measured with the speculative variant (parallel plan, re-plan if
an earlier band already took from a shared cell: 89-94% of plans had to be redone, so c365b39 just
plans sequentially; the two give identical worlds). docs/results-v6b-seqharvest-seeds1-10-500y*.txt.

| metric (10 seeds) | v5 | v6a fair share | v6b seq. harvest |
|---|---|---|---|
| total pop Y500 | 909k | 1121k | 934k (+3%; higher on 4 of 10 seeds) |
| fights per century (mean) | 4480 | 7116 | 3564 (-20%) |
| killed in war, %/yr | 0.332 | 0.466 | 0.272 (lower on 10 of 10) |
| farming origins (mean) | 16.5 | 11.7 | 8.3 |
| farming share of people Y500 (mean) | 35% | 29% | 32% |
| voyages / landings / lost at sea | 188k / 19.7k / 38.5k | 204k / 14.0k / 33.5k | 181k / 17.9k / 36.5k |

Pop and sea are now within noise. Farming origins (half of v5) and war deaths (-18%, all seeds)
are still off.

Cause found for farming origins: farm-layer changes were buffered as deltas computed from the
start-of-season value and summed. Where several bands tend the same cell, v5 compounds them
(each step `t += k(0.4 - t)` sees the previous one, so foraging alone never pushes tending past 0.4);
summed deltas overshoot past 0.45, which counts as "fields nearby", so new starts were logged as
"took up farming" instead of independent origins. Fix (step 5): buffer each step as "move toward
target by k" and apply it to the live value in band order, the exact v5 result.

Next: balance check 3 with the farm-layer fix; then look at war deaths (candidates: movement now
sees the stocks after every band has harvested; meeting order by rounds).

## v6 finish (25 Oct 2026, EDT). Balance tests moved to GitHub Actions

### 5. Turnaround: GitHub Actions vs this box
`headless-sims.yml`: 10 seeds x 500 years, one seed per job, `--threads 4`.

| run | commit | dispatch -> summary | notes |
|---|---|---|---|
| 1 | c353bd1 | 3m36s | cold cargo cache (44 s compile) |
| 2 | dab0519 | 3m18s | warm cache (19 s compile); 3 seed jobs waited ~40 s for a runner while the test and bench jobs held slots |
| 3 | 6057dbb | 2m54s | tests off |
| 4 | 5532fac | 4m22s | ran beside a bench run on a slow runner batch (seed 5 alone took 148 s) |
| 5 | f06d614 | 2m50s | tests on, bench running too |
| 6 | 5f7eca7 (final) | 3m27s | tests on (passed, 68 s), bench running too; seed 5 job waited 26 s for a runner, then ran 113 s; table identical to run 5 |

On this box, one seed (seed 1, 500 y) took 63.8 s wall at load 6 to 8 and **253.9 s** at load 9 to 13
(user 107.7 s; the same seed takes 60.7 s on a runner). Ten sequential seeds come to 10 to 25+ min,
plus a 3 to 4 min build. **Actions takes about 3 min for the whole check, 3 to 8x faster, and puts
no load on the box.**

Workflow fixes (dab0519, 5532fac):
- `ref` can be a short SHA, branch or tag. The plan job resolves it to a full SHA once, and every
  job checks out that SHA. A short SHA used to break `actions/checkout`, and later jobs could also
  pick up a moved branch.
- `run_tests` (default on) runs `cargo test` on its own runner. Set it off for pure balance runs
  to free a runner slot. `threads` (default 4) sets the sim threads.
- The summary job runs `tools/agg_results.py` against `baseline` (default the v5 results) and
  writes the table into the run summary and `aggregate.md`, so a balance verdict needs no download.
- The concurrency group is now keyed by ref + seeds + years. It used to be one group per branch,
  which serialised experiments on different commits, and a newer pending run silently cancelled an
  older pending one.
- `bench.yml` builds the ref and a baseline (default v0.5.0) on **one** runner and times both,
  so timings are comparable. It now also captures the `KINDLING_PROF` per-phase profile.

### 6. Balance: fixed (5532fac)
The cause of the remaining gap (war -18%, disease -16%, farming origins half of v5) was
**movement**. In phase 3, `cell_score` and `push_out` read the wild stocks after *every* band
had harvested, so all cells looked emptier than in v5, where band k saw only its own harvest and
the earlier bands'. Bands therefore left shared ground sooner and spread thinner. That meant less
crowding, and so less tension, less disease and fewer farming starts (push = stress + crowd).
v6 bench seed 5 had 6.2k bands at Y300 against 7.6k in v5.

Fix: phase 2 now logs, for each cell a band harvests, the stocks just before it takes its share.
In phase 3, `World::seen(r, c, band)` returns the value logged by the first *later* band to
harvest that cell, or the live value if none did. That is exactly what that band saw in v5. It is
used in movement (`cell_score`, sea fish), raw materials for the toolkit (`raws_at`), the
experiments' "at hand" test and the herding check in farming. It is not a tuning knob: v6 now
reproduces v5's view. After the harvest the log is regrouped by cell, with a per-cell
(start, len) index that is reset only on the cells touched, so it is O(bands) per season on any
map size. On the bench, seed 5 at Y300 now has 148.7k people and 7689 bands (v5: 149.4k, 7635).

Final, 10 seeds x 500 y (docs/results-v6-final-seeds1-10-500y.txt):

| metric | v5 | v6 final | diff |
|---|---|---|---|
| total pop Y500 | 909196 | 928522 | +2.1% |
| killed in war %/yr | 0.332 | 0.347 | +4.5% |
| disease %/yr | 0.113 | 0.122 | +8% |
| fights/century | 4480 | 4583 | +2.3% |
| domestic-grain seeds | 9 (Y241-400) | 9 (Y252-476) | |
| farming origins (mean) | 16.5 | 15.0 | -9% |
| foragers, mean % of people | 46% | 51% | farming does not take over |
| second continent | 4/10: Y152, 226, 307, 162 | 4/10: s1 Y156, s2 Y208, s7 Y258, s8 Y176 | none before Y100 |
| voyages / landings | 188052 / 19688 | 190900 / 19709 | |
| lost at sea | 38547 | 39305 | +2% |
| iron / bronze seeds | 8 / 6 | 8 / 4 | |

There are no artificial interventions. The only remaining differences from v5 are deliberate
consequences of parallel phases: farm layers are read as at the start of the season, an invention
is first usable the next season, and meetings run in band-disjoint rounds. Bronze shows up in 4
seeds instead of 6, which is within seed noise (the iron seeds are unchanged).
`v5beta_fixture_loads_and_runs` passes again: the population collapse it caught was the same
dispersal effect.

### 7. Speed (bench.yml, seed 5, 300 y, same runner for every row)

| build | base v5 1t | v6 1t | v6 2t | v6 4t |
|---|---|---|---|---|
| dab0519 (before) | 31.49 s | 37.23 s | 24.23 s | 20.18 s |
| 6057dbb toolkit cache | 32.29 s | 36.34 s | 23.73 s | 20.58 s |
| 5532fac v5 view, sorted log | 31.77 s | 66.67 s | 41.11 s | 32.93 s |
| f06d614 per-cell chains | 30.94 s | 49.67 s | 31.46 s | 26.23 s |
| d994b2b regrouped by cell | 32.13 s | 44.25 s | 29.33 s | 24.78 s |
| 5f7eca7 final (run 40191926979, slower runner) | 33.15 s | 49.33 s | 32.00 s | 26.54 s |

Caveat: before the balance fix, the v6 world at Y300 was ~20% smaller (fewer bands), so the
+12 to 18% single-thread overhead of the first two rows understates the per-band cost. At an
equal world (last row), v6 1t is +38% over v5 and 4 threads give 1.79x over v6 1t, i.e. 1.30x
faster than v5. Per-phase profile, 1t (v5 -> v6, s): band seasons 13.1 -> 21.5 (economy
2.8 -> 4.1, experiments 3.8 -> 4.3, movement 1.8 -> 3.8, farming 0.3 -> 0.9, prep 1.2, log 0.3,
merge 0.4, ~5 s per-band phase-3 overhead: Fx buffers, Econ hand-over, mark_used), contacts
7.9 -> 9.8 (27.1M vs 22.3M pairs in range: more clustered world, not code), regrow 0.3 -> 0.6.
The toolkit cache removed the duplicated toolkit closure work. The rest is the price of the
three-phase split and is not recovered yet (see open items).

Final bench (5f7eca7, run 40191926979): v6 1t is +49% over v5 on that runner (+38% at d994b2b,
same code apart from profiling; runner noise). 2 threads: 1.54x over v6 1t, 1.04x faster than v5.
4 threads: 1.86x over v6 1t, 1.25x faster than v5. 1t profile v5 -> v6 (s): band seasons
19.0 -> 30.4 (economy 3.7 -> 6.0, movement 2.9 -> 6.0, experiments 5.6 -> 6.2, farming 0.4 -> 1.4,
prep 1.6, log 0.5, merge 0.4), contacts 11.0 -> 14.0, regrow+occ 0.5 -> 1.6.

**Single-thread overhead: settled, not recovered in v6.** The remaining cost is spread across the
phase split (economy hand-over, the v5-view lookups in movement, the Fx buffers, farm-layer ops) with
no single hot spot left. Each remaining fix touches the balance-critical path and would need another
balance re-check, so it is deferred to v6.1. Use `--threads 2` or more to run faster than v5.

### 8. Release v0.6.0
- Version 0.6.0 (native, civ-core, Cargo.lock). `build-web.sh` now looks in every
  `~/.cargo/registry/src/*/` dir for each JS file, takes the newest version, and fails clearly
  if one is missing.
- Start picker screenshot: docs/images/screenshot-start-picker.png
  (`kindling --screenshot out.png --ui picker --seed 8`).
- Final check on 5f7eca7: headless-sims run 40191925053 (10 x 500 y), aggregate identical to run 5;
  its cargo test job passed. Local `cargo test --release -p civ-core --features parallel`: 11 passed,
  1 ignored (farm_diag); without `parallel`: 10 passed, 1 ignored.
- Builds (box, -j4): Linux `cargo build --release -p kindling`, Windows `--target
  x86_64-pc-windows-gnu`, web `./build-web.sh` -> web-dist.zip.
- Release: https://github.com/Macrolol/kindling/releases/tag/v0.6.0 (assets kindling-linux-x86_64,
  kindling-windows-x86_64.exe, kindling-web.zip, SHA256SUMS.txt).

### Open items
- Single-thread cost is +38 to 49% over v5 at an equal world (settled for v6, see section 7). Candidates: the per-band phase-3
  hand-over (Fx/Econ allocation, ~5 s on the bench), the harvest-view lookups in movement (2 s),
  and farm-layer Toward ops (0.6 s). Multi-thread runs are faster than v5 from 2 threads up.
- Hosted runners vary by ~40% between batches (base v5 took 23 s to 32 s on the same bench), so
  compare speed only within one bench run.
