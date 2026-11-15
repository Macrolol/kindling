# PERF-apply

Profile of `skills-step4` (`3521366`) and the order the optimisations will be applied in. No gameplay, constant, or save-format change. Each later row is one commit, bit-identical at any thread count, or it is not committed.

The phase line is `KINDLING_PROF` (`pa-prof`) on the baseline binary. Belief sites are not their own bucket, so a second binary carried temporary clocks around inheritance, testimony, payout, yearly fade, `make_kit_beliefs`, the partner walk, closure keys, the teach scan, `track_knowledge`, `year_stats`, and `home_range`. Those clocks were removed before this commit. The instrumented runs printed the same summary as the baseline runs (late world pop 1310699 / 72682 bands; seed 5 pop 144312 / 7345 bands).

Sub-buckets inside a parallel phase are a sum of per-band CPU, added when that band's effects are applied. They are not wall time, and they sit inside the parent wall bucket. `movement` includes the `refresh_toolkit` immediately before `maybe_move`. Payout runs at the start of phase 3, before that phase's clock, so it is in the band-seasons wall and not in a sub-bucket.

## Phase line

Late world: load `main_s499246296_y750_800x800.civsave`, 3 years, 4 threads. Wall 39.1 s, of which load is 5.0 s. Twelve ticks.

| bucket | seconds | kind |
|---|---:|---|
| regrow+occ | 0.1 | wall |
| band seasons | 14.4 | wall |
| contacts | 16.6 | wall |
| demography | 0.3 | wall |
| stats | 2.7 | wall |
| economy | 4.8 | wall, serial harvest |
| farming | 1.2 | CPU sum |
| experiments | 3.4 | CPU sum |
| movement | 14.6 | CPU sum, includes a toolkit refresh |
| other | 2.8 | CPU sum |
| finding neighbours | 1.3 | wall |
| meetings | 6.4 | wall |
| partner visits | 8.8 | wall |
| prep (toolkits) | 3.9 | CPU sum |
| harvest log | 0.3 | wall |
| merge | 1.3 | wall |

Per late year the wall is about 11 s of sim after load. Partner visits are 2.9 s of that, meetings 2.1 s, economy 1.6 s, stats 0.9 s.

Seed 5, 300 years, 1 thread. Wall 101.8 s (the brief's 118.5 s was another machine).

| bucket | seconds | kind |
|---|---:|---|
| regrow+occ | 0.6 | wall |
| band seasons | 57.2 | wall |
| contacts | 39.0 | wall |
| demography | 0.6 | wall |
| stats | 4.3 | wall |
| economy | 7.8 | wall |
| farming | 0.8 | CPU sum |
| experiments | 6.7 | CPU sum |
| movement | 23.2 | CPU sum, includes a toolkit refresh |
| other | 4.6 | CPU sum |
| finding neighbours | 3.2 | wall |
| meetings | 25.6 | wall |
| partner visits | 10.2 | wall |
| prep (toolkits) | 4.5 | CPU sum |
| harvest log | 0.4 | wall |
| merge | 1.4 | wall |

## Belief sites

`make_kit_beliefs` is the whole kit chooser (the old ranking plus the belief food scores), timed at every refit. The other rows are only the step-4 function named.

| site | late 3 y, 4 threads | seed 5, 300 y, 1 thread |
|---|---|---|
| kit beliefs (`make_kit_beliefs`) | 13.22 s CPU, 2.19e6 calls | 26.42 s, 4.72e6 calls |
| payout (`note_belief_payout`) | 1.48 s CPU, 8.69e5 calls | 3.96 s, 2.32e6 calls |
| yearly fade | 0.11 s, 3 calls | 0.50 s, 300 calls |
| testimony (`pass_belief`) | 0.08 s, 7.3e4 calls | 0.12 s, 1.81e5 calls |
| belief year-means | 0.03 s | 0.08 s |
| inherit on split | 0.002 s, 3278 daughters | 0.003 s, 10148 daughters |
| inherit on a drift landing | ~0 | ~0 |

Per band-year (band-seasons / 4): kit beliefs cost about 61 µs CPU on the late world and 46 µs on seed 5, at 8–10 calls per band-year. Payout is 6.8 µs CPU per band-year on both. Inheritance and testimony are noise. Fade is half a second across the whole seed-5 run.

On seed 5 the kit chooser is a quarter of the run. That is the step-4 cost that matters. It is not additive with the phase line: those calls sit inside prep, the serial harvest refresh, and the refreshes counted under experiments and movement.

## Other sites the list asks about

Temporary clocks. Partner and `land_cost` rows are inflated: the late partner-visits bucket rose from 8.8 s to 11.3 s, and seed 5's from 10.2 s to 13.0 s, because every `land_cost` and every surviving `rng.chance` took its own clock (15.5e6 and 10.7e6 calls on the late run).

| site | late 3 y, 4 threads | seed 5, 300 y, 1 thread |
|---|---|---|
| partner list walk, before `rng.chance` | 9.79 s wall (see note) | 8.99 s wall (see note) |
| of which `land_cost` | 1.41 s, 15.5e6 calls | 2.49 s, 28.3e6 calls |
| partner `rng.chance` | 0.28 s, 10.7e6 rolls, 4.03e5 visits | 0.43 s, 16.6e6 rolls, 6.17e5 visits |
| far `run_meetings` | 1.17 s wall | 3.50 s wall |
| links considered | 39.5e6 | 71.8e6 |
| teach-list scan in `exchange` | 5.58 s CPU, 3.17e6 scans | 3.66 s, 5.80e6 scans |
| `closure` on a toolkit miss | 7.15 s CPU, 1.15e6 misses | 6.62 s, 1.73e6 misses |
| toolkit hit / prev-hit / miss | 5.67e6 / 1.80e5 / 1.15e6 | 12.9e6 / 3.41e5 / 1.73e6 |
| third key would have hit | 7830 of 1.15e6 misses (0.7%) | 23159 of 1.73e6 (1.3%) |
| `track_knowledge` | 1.28 s wall, 12 ticks | 1.53 s, 1200 ticks |
| `year_stats` | 1.46 s wall, 3 years | 2.60 s, 300 years |
| `home_range` | 0.21 s | 0.40 s |

The uninstrumented partner-visits bucket, minus the far meetings, is the walk budget: about 7.6 s over the late 3 years (2.5 s/year) and about 6.7 s over seed 5. Almost all of it is before `rng.chance`.

## What will be applied

1. **Partner phase: parallel pre-filter, serial roll.** Landed. The far-partner walk runs per band through `par_collect`; survivors are rolled in band and partner-list order. See the commit table.
2. **Cheap reach bound before `land_cost`.** Skipped. The late 3-year sim fell from 40.8 s to 34.1 s, which is almost the whole uninstrumented walk (~7.6 s, meetings included in the old bucket). `land_cost` is inside that parallel walk. A bound that skips a fraction of it, then splits the rest by four threads, is under the noise.
3. **Third closure slot.** Skip. A third per-band key would have hit 0.7% of late misses and 1.3% of seed-5 misses. An extra toolkit at ~72k bands is a large allocation for a fraction of a percent of rebuilds.
4. **Incremental closure with a full-closure fallback** when the new item can feed an earlier recipe whose output is already producible. Closure is 2.4 s CPU per late year and 6.6 s on seed 5. Real, but only with the fallback. Not next.
5. **Teach-list scan.** 5.6 s CPU over the late 3 years (inside parallel meetings) and 3.7 s on seed 5. Worth doing after the partner walk.
6. **Seasonal refit / kit belief reads.** The kit chooser is the step-4 hotspot: 13.2 s CPU over the late 3 years, 26.4 s on seed 5, 8–10 calls per band-year. Skip a refit only when the chosen items cannot move. This is also the belief optimisation; inheritance, testimony, and year-means are not.
7. **One-pass `via`.** Safe half only. Small next to 1 and 6. Do it if the closure time is still there after 4.
8. **Stats tail.** Both halves are real and similar. Late year: `track_knowledge` 0.43 s, `year_stats` 0.49 s. Seed 5: 1.53 s and 2.60 s. `year_stats` is the larger one on the long run. Fix that first.
9. **`home_range` allocation.** Skip. 0.21 s over the late 3 years, 0.40 s on seed 5.
10. **Further belief work.** Payout is the only other belief site above noise (4.0 s on seed 5, 1.5 s CPU over the late 3 years, and it is inside the parallel season). Revisit only if 6 does not already shrink the refits that dominate. Do not touch inheritance or testimony.

The `movement` bucket is large on the phase line and is not scheduled. It includes a toolkit refresh, so it is not evidence for a movement patch.

## Commits

| commit | change | files | identity | s5 t1 | s5 t4 | late t4 |
|---|---|---|---|---|---|---|
| 6cb1da8 | profile and this plan | `docs/PERF-apply.md` | n/a, no sim change | n/a | n/a | n/a |
| (this) | parallel partner pre-filter, serial roll | `civ-core/src/world.rs` | yes, pa-idcheck threads 1 and 4 | 107.4 s → 105.9 s (−1.5 s) | 48.8 s → 41.3 s (−7.5 s) | 40.8 s → 34.1 s (−6.7 s) |

Times are `pa-bench` sim seconds, one rep, A = skills-step4 binary, B = this commit. Late is the Y750 800×800 save plus 3 years at 4 threads. The 1-thread seed-5 delta is small; the 4-thread deltas are the walk.

## Not applied: changes results

None yet.
