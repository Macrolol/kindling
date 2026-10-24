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

### 2. Benchmarks (in progress, /workspace/bench/bench2.log)
800x800, loaded from big800_y600.civsave, +20 years (Y600 to Y620, about 43k to 45k bands, pop 0.84 to 0.91M):

| build | threads | wall s | user CPU s | pop Y620 |
|---|---|---|---|---|
| v5 | 1 | 77.1 | 66.7 | 838677 |
| v6 (par4) | 1 | 97.3 | 82.5 | 907079 |
| v6 (par4) | 2 | 103.4 | 85.6 | 907079 |
| v6 (par4) | 4 | (running) | | |

- The t1 and t2 results are identical (same pop), so the run is deterministic across thread
  counts even though that is no longer required.
- Single-thread CPU cost is about 24% above v5.
- The t2 run used no more CPU than t1, so on this box the second thread got no real core.

Profile (wall-clock phases; sub-entries are thread-seconds):
- v5: band seasons 36.9 (economy 6.9, experiments 10.6, movement 6.5, other 7.0), contacts 29.1 (meetings 21.5), stats 6.6
- v6 t1: band seasons 52.1 (economy incl. toolkit refresh 18.8, experiments 15.4, movement 6.7, other 3.2), contacts 34.3 (meetings 25.5), stats 5.1

Next: the default-map seed 5 500y runs (v5, then t1, t2 and t4), then a 10-seed balance check.

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
