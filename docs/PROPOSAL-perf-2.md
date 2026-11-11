# Perf, second look: where a large world spends its time

Nothing in this change alters the sim. It is a measurement note and a ranked list of changes. The late world is now measured, not scaled: load of `opt-saves/main_s499246296_y750_800x800.civsave` (seed 499246296, 800×800, Y750, pop 1,301,977, 72,169 bands), then three years at 4 threads on the clean binary and one year at 1 thread with temporary counters. The counters are not in the commit.

## 1. How this was measured

Local runs, one at a time, release binaries, `KINDLING_PROF=1`.

- Clean phase times: `/workspace/bots/build-bot/step3-logs/kindling-head` (commit 632dcca, the same sim as main `3b2a515`).
- Call counts and per-call times: a throwaway build of this branch with relaxed atomic counters and `Instant` timers around closure, chain fill, replay, `fit_kit`, `refresh_toolkit`, `maybe_move`, `meeting` and `exchange`. That build was about 10% slower than the clean binary on the same stretch (4.8 s vs 4.4 s). The counters are not in the commit. Per-call times below are from the 1-thread instrumented run, where the atomics were not bouncing between cores.
- `--load` plus `--years N` runs N more years from the loaded tick (`native/src/headless.rs` always steps `years * 4` ticks; it does not restart at year 0). The "seed" printed on a progress line is the `--seed` argument, not a field read back from the file. The world inside the file is what ran.
- The box is shared. The four successive years of the populated stretch agreed within about 0.2 s, and the instrumented vs clean pair agreed within 10%. The 4-thread figure is one sample of the instrumented binary. Treat the second digit as noise.
- No perf, samply, valgrind or gdb. Kernel `perf_event_paranoid` is 2.

Runs:

| run | what | threads | wall |
|---|---|---|---|
| seed 5, years 0–40, default map | early, small | 1, instrumented | 0.2 s, of which 0.1 s is map generation |
| load `step3-saves/main_seed6_y350_256x160.civsave`, +4 years | steel-age, 256×160, ~3,400 bands, ~68k people | 1, clean | 4.4 s including 0.3 s load, then 0.9 / 1.1 / 1.1 / 1.0 s per year |
| same load, +4 years | same world | 1, instrumented | 4.8 s (1.0 / 1.2 / 1.2 / 1.1 s per year after load) |
| same load, +4 years | same world | 4, instrumented | 2.5 s (about 0.55 s/year after load) |
| load the Y750 800×800 save, `--years 0` | pop 1,301,977, 72,169 bands, 101 items, mean 52.6 items/band | 4, clean | 10.4 s cold. Profile all zeros. |
| same load, +3 years | Y750→Y753, pop 1,309,652, 72,440 bands | 4, clean | 53.3 s wall, of which 6.4 s is a warm load. Years 16.2 / 15.6 / 15.1 s |
| same load, +1 year | Y750→Y751, pop 1,308,981, 72,328 bands | 1, instrumented | 41.2 s wall, of which 6.0 s is a warm load. The year is 35.2 s |

The progress lines of the loaded 256×160 world match across all three populated runs. After one year: pop 68795, bands 3393. After four: pop 67213, bands 3441. That is Y350→Y354 of the seed-6 save (3,354 bands and pop 67,836 at the moment of the save).

The Y750 progress line at Y751 is the same on 1 thread and on 4 threads, apart from the seconds: pop 1308981, bands 72328, items 101, mean 52.6, max 89, fire 71736, exp 66722, inv 29, learn 122701, forget 104692. Warm load is 6.0–6.4 s and includes `after_load`, which rebuilds all 72,169 toolkits once (about 1.9 s of that). Cold load, nothing in the page cache, was 10.4 s. The one-thread year carries the counters; its phase buckets sit within a few tenths of a second of the clean run's per-thread sums, so the overhead on one thread is small. Treat the second digit of a single year as noise: the three clean years were 16.2, then 15.6, then 15.1.

`bench.yml` has no `--load`, so that short stretch cannot be repeated on GitHub without a workflow change. The full-run numbers cited below are the ones already published in `docs/PROGRESS-skills.md` (GitHub run 41569769916 and the local seed-5 benches).

The profile line's "pairs in range" and "meetings" are counters since the start of the save, not since the load. On this stretch they still say 30.0M pairs and 2.6M meetings after four years. The meetings that actually ran in those four years are the 86,991 calls counted below.

## 2. Early / small worlds

Seed 5, default 256×160, years 0–40, 1 thread. At Y40: pop 493, 19 bands, pottery, mean 3.9 items known per band.

Every profile bucket prints 0.0 s. The chain machinery is awake but tiny:

| what | calls | time |
|---|---|---|
| `refresh_toolkit` hit / swap / rebuild | 7,330 / 8 / 212 | 5.6 ms total |
| closure | 218 | 0.2 ms |
| fresh chain fill | 454 | 0.3 ms |
| chain replay | 1,630 (0 skipped) | 0.4 ms |
| `chain_m` | 6,996 | (inside the callers) |
| `fit_kit` | 2,122 | 3.3 ms |
| `meeting` | 201 | 0.7 ms |

About two known recipes per fill, and 82 chain walks in forty years. Map generation is half of the 0.2 s. Step 3 does not matter here. A year costs a few milliseconds once the map exists, and almost none of it is maker skill.

The published seed-5 bench (300 years, 1 thread) is a different thing: it spends most of its time after the world has filled up. Locally that was 57.0 s on step-2 main (`f8d80dd`, pop 140,788), 61.8 s on fixes-only (`dfc7d7e`, pop 99,479) and 79.2 s on step-3 head (pop 104,860). GitHub, same case: 67.16 s vs 91.95 s (1.37×), and 58.88 s / 48.94 s at 2 / 4 threads. Forcing chain skill to 1.0 put the per-band cost back on the fixes-only line (`docs/PROGRESS-skills.md`, Step 3, Open problems). That full-run profile is not re-measured here. A profile file sitting next to the reference binary does not add up to the 79.2 s wall (its phase totals are far higher), so it is not used.

## 3. Late / large worlds

### What was actually timed

The populated 256×160 world at Y350 is already a steel-age world: 81 items exist, a band knows about 77 recipes, and about 63 items are available to it. It is the right shape of band. It is not the right number of bands, and the map is 16× smaller than 800×800.

Clean 1-thread profile for the four years (seconds). Sub-rows are inside "band seasons" or "contacts"; they are not extra.

| phase | seconds | per year | scales to 4 threads? |
|---|---|---|---|
| regrow + occupancy | 0.0 | <0.02 | parallel on big maps, invisible here |
| band seasons | 2.3 | 0.58 | wall fell 2.5→1.2 on the instrumented pair (~2×) |
| economy (inside seasons) | 0.3 | 0.08 | no. 0.4 s stayed 0.4 s |
| farming | 0.1 | 0.03 | CPU sum, not wall |
| experiments | 0.3 | 0.08 | CPU sum. Includes a toolkit refresh (below) |
| movement | 0.9 | 0.23 | CPU sum. Includes a toolkit refresh (below) |
| other (materials, births, practice) | 0.2 | 0.05 | CPU sum |
| prep (toolkits) | 0.2 | 0.05 | parallel |
| contacts | 1.7 | 0.43 | wall fell 1.8→0.8 (~2.3×) |
| finding neighbours | 0.1 | 0.03 | yes, already small |
| meetings | 1.2 | 0.30 | wall fell 1.4→0.5 (~2.8×) |
| partner visits | 0.3 | 0.08 | partial |
| demography | 0.0 | | |
| stats | 0.1 | 0.03 | no |

Whole year, 1 thread: about 1.02 s. At 4 threads: about 0.55 s. That is a 2.0–2.1× speedup, not 4×. The serial piece you can see is `take_harvest` (the economy row), which walks bands in order because an earlier parallel plan changed population and war. Stats and the tail of a meeting round are serial too.

### The profile blames the wrong functions

`band_season` laps the clock like this:

```rust
fx.lap(&mut tk, 9);          // end of "other"
self.refresh_toolkit(b);     // counted as experiments
// ... experiments, sea, practise ...
b.d.kit_dirty = true;
b.d.skill_ver = b.d.skill_ver.wrapping_add(1);
fx.lap(&mut tk, 9);
self.refresh_toolkit(b);     // counted as movement
self.maybe_move(b, rng, fx);
fx.lap(&mut tk, 8);
```

So the 0.9 s "movement" bucket is mostly the post-practice chain refill and kit refit. `maybe_move` itself is 54,731 calls and 201 ms in the instrumented four years, about 3.7 µs per band per season. The 0.3 s "experiments" bucket includes another `refresh_toolkit`, the one that sees this season's stock changes.

`refresh_toolkit` also runs from prep (parallel), from `take_harvest` (serial, once per band, after earlier bands have eaten), from trade, from teaching (`exchange`), and for a new band. In these four years: **316,140 cache hits, 10,586 swaps onto the one saved toolkit, 62,258 full rebuilds**. About seven calls per band per season, one of them a rebuild.

### Call counts on that stretch (1 thread, instrumented)

A band-season is one band for one season. Four years × four seasons × ~3,420 bands ≈ 55,000 band-seasons.

| function | calls | time | per call | how it grows |
|---|---|---|---|---|
| `closure` | 62,258 | 447 ms | 7.2 µs | one per rebuild. Scans ~77 known recipes, up to 16 passes |
| fresh `fill_chain` | 62,258 | 744 ms | 12 µs | same count. 16 walks + 28 adopted ranks, ~5.4 steps each |
| `replay_chain` | 45,441 entered, **0 skipped**, 45,207 full, 234 partial | 153 ms | 3.4 µs | about one per band per season, because `skill_ver` bumps every season |
| `apply_stored` (the geometric mean) | 4.64M | (inside replay and fresh) | ~50 ns | one per read rank on a full replay |
| `fit_kit` → `make_kit_for_view` | 129,649 | 744 ms | 5.7 µs | once a season after practice, plus every swap and rebuild |
| `chain_m` | 5.48M | small | a rank lookup | ~42 per `fit_kit`, plus a handful in `economy` |
| `via` | 34.4M | mostly inside closure and `recipe_info` | | |
| `exchange` | 156,188 | 740 ms | 4.7 µs | ~2 per meeting, scans the teacher's known list |
| `meeting` | 86,991 | 1,479 ms | 17 µs | includes `exchange` and the refreshes teaching and trade do |
| `maybe_move` | 54,731 | 201 ms | 3.7 µs | one per band per season |

Chain arithmetic, not the lookup, is the step-3 cost. Closure + fresh fill + replay is about 1.34 s of the 4.8 s instrumented run, roughly a quarter to a third of a populated year on one thread. `fit_kit` is another 0.74 s. `chain_m` by itself (5.5M calls) is not the problem.

Why step 3 made this hot, in the code that is actually running:

1. Practice runs every season and then `skill_ver` increments and `kit_dirty` is set. `replay_chain` returns immediately only when every raw skill bit is unchanged. That happened **zero** times in 45,441 refills. The replay is cheap (3.4 µs) because the walk was recorded. It is not cheap to do the surrounding refit.
2. A rebuild throws the recorded walk away and pays closure (7.2 µs) plus a fresh fill (12 µs). The fresh fill already adopts a stored walk when `via` is unchanged: 28 of 44 read ranks were copies, 16 were new walks. `via_stable` rejects the **whole** copy if any shared item is made a different way, so one changed recipe rewalks every read item.
3. The two-entry cache is documented as the fix for game or plants crossing a threshold. In this world it catches 10.6k returns and misses 62.3k changes: about **15%** of the times the raw set actually changed. `raws_at` folds in every carried stock above 0.05 (hide, bone, each metal, grain, flax, wool, salt) and meat / plants / fish above a `seen` threshold. Those bits flip independently. The second slot is usually the wrong set.
4. The kit is refit on every rebuild, every swap, and whenever `kit_dirty` is set. Step 3's `make_kit_scored` calls `chain_m` once per payoff candidate (`effects.rs`, `make_kit_among_view`). That is 42 lookups per fit here. The lookups are cheap. The scoring around them is the 5.7 µs, and it now depends on chain skill, so a skill tick cannot keep the old kit and stay bit-identical.

The lazy per-item chain tried during step 3 is not a better version of this. On the seed-5 bench it did 147M chain reads, 2.5M walks, 73M per-item replays and 44.7M adopts, and took 80.8 s (1.33× that session's fixes-only run), no better than the committed head at 79.2 s. Replaying every recorded walk in one batch took 90.6 s. Both were bit-identical and neither was committed. Do not go back to them. The committed design already replays a recorded walk in one pass. The remaining cost is **rebuilding the closure**, not replaying it.

### The late year, measured

This world is 21× the Y350 sample in bands (72,400 vs 3,441) and 19× in people. The map is 15.6× the cells. A band knows about 91 recipes and the closure has about 64 entries (`known_sum` / fills, `via_sum` / fills). The history line "mean 52.6" is items available, not recipes known. Do not scale from 52.6.

Clean 4-thread profile, three years, divided by three. Sub-rows of "band seasons" are CPU sums from the parallel workers except economy, the harvest log and merge, which are wall time on the main thread. Contact sub-rows add up to the contact wall.

| phase | 4-thread wall, s/year | 1-thread, s/year | what 4 threads did |
|---|---|---|---|
| whole year, after load | 15.6 | 35.2 | 2.2× |
| regrow + occupancy | 0.03 | 0.1 | not the surprise. 640k cells are still nothing |
| band seasons | 6.1 | 15.6 | 2.6× |
| economy (`take_harvest`, serial) | 2.2 | 2.2 | none |
| prep (toolkits, CPU sum) | 1.7 CPU | 1.7 | parallel; the wall is a fraction of 1.7 |
| experiments (CPU, includes one refresh) | 1.5 CPU | 1.5 | parallel |
| movement (CPU, includes the post-practice refresh) | 6.3 CPU | 6.5 | parallel. `maybe_move` itself is 2.0 s of the 6.5 |
| other (CPU) | 1.3 CPU | 1.3 | parallel |
| farming (CPU) | 0.7 CPU | 0.6 | parallel |
| harvest log + merge | 0.4 | 0.4 | serial, small |
| contacts | 8.2 | 17.9 | 2.2× |
| finding neighbours | 0.67 | 2.1 | 3.1× |
| meetings | 2.9 | 10.3 | 3.6× |
| partner visits | 4.7 | 5.5 | 1.2×. Almost serial |
| demography | 0.13 | 0.1 | serial, small |
| stats | 1.2 | 1.3 | serial |

The 18× scaling guessed 10 s/year at 4 threads and 18 s at 1 thread. The measured year is 15.6 s and 35 s. Bands were 21×, not 18×, and two costs grew faster than the bands: partner visits (73× the Y350 per-year cost) and the stats tail (about 50×). Regrow did not. Neighbour search did grow faster than the bands (the radius is fixed, the map is wider, density is 0.11 bands/cell against 0.08), and it is still 0.7 s of the 15.6.

### Where the 35 s goes, per call

Counters on the 1-thread year. The 72,169 rebuilds inside `after_load` are part of the warm load, not of the year, and are subtracted here. Nested the same way as the Y350 note: `refresh_toolkit` includes closure, fill and `fit_kit`; `meeting` includes `exchange`.

| function | calls in the year | time | per call | against the Y350 year |
|---|---|---|---|---|
| `closure` | 384,496 | 3.23 s | 8.4 µs | 7.2 µs, 24× the calls (bands were 21×) |
| fresh `fill_chain` | 384,496 | 4.84 s | 12.6 µs | 12.0 µs. 19 walks + 27 adopts, ~6 steps each |
| `replay_chain` | 216,245 entered, **0 skipped**, 215,369 full | 0.82 s | 3.8 µs | 3.4 µs, and still never skipped |
| `fit_kit` | 727,863 | 3.82 s | 5.3 µs | 5.7 µs, 22× the calls |
| `exchange` | 1,053,366 | 6.22 s | 5.9 µs | 4.7 µs, 27× the calls. Includes the rebuild when a recipe is actually learned |
| `meeting` | 602,633 (469,008 near, 133,625 far) | 11.4 s | 18.9 µs | 17 µs, 28× the calls |
| `maybe_move` | 288,867 | 2.04 s | 7.0 µs | 3.7 µs. One per band per season, and the call itself got slower |
| partner scan (`land_cost` and the loop around it) | 6.62M pairs considered | 3.90 s | | was inside the 0.08 s visit bucket. Now its own line |
| far `run_meetings` | 133,625 queued | 1.58 s | | the rest of the 5.5 s visit wall |

`refresh_toolkit` in the year: 1,886,186 hits, 56,851 swaps, 384,496 rebuilds. Of the times the raw set changed, the second slot caught 13% and missed 87%. Same failure as the 15% hit on the small map.

Rebuilds by call site, steady year (the load's 72,169 are not in this):

| site | hits | swaps | rebuilds | parallel? |
|---|---|---|---|---|
| prep | 232,865 | 6,744 | 49,258 | yes |
| harvest | 234,968 | 32,407 | 21,492 | no |
| before experiments | 249,551 | 8,646 | 30,670 | yes |
| before movement | 204,118 | 0 | 84,749 | yes |
| trade | 964,684 | 9,054 | 75,488 | yes, inside meeting rounds |
| teaching | 0 | 0 | 121,743 | yes, inside `exchange` |
| new band | 0 | 0 | 1,067 | serial, noise |
| invention | 0 | 0 | 29 | noise |

Harvest holds 5.6% of the rebuilds, not most of them. The chain tax parallelizes. On 4 threads it is a couple of seconds of the 15.6, not the 1–4 s serial bracket from the scaling note. That bracket was wrong.

The partner scan does not parallelize. Of the 5.14M `land_cost` walks (55.1M cell steps, 3.90 s):

- 1.48M other pairs were inside the teach radius and never walked.
- 3.51M walks are within `reach` (21.0M steps). Terrain can still refuse them, so the line has to be walked.
- 1.12M walks are past `reach` but within twice `reach` (17.8M steps). A shared river charges half the distance (`RIVER_VISIT` is 0.5), so some of these can still succeed. How many share a river was not counted.
- 0.51M walks (16.3M steps, 30% of every step walked) are past twice `reach`. `step_cost` is at least 1, so `land_cost` is at least the chebyshev distance, and half of that is still past `reach`. They cannot be visited. 134k visits are queued out of all of this.

`maybe_move` at 2.0 s, neighbour search at 0.7 s on 4 threads, regrow at 0.03 s and demography at 0.1 s are not the year. The stats tail is 1.2 s and was not split. `year_stats` walks every band once a year; `track_knowledge` writes a per-item grid when a band's cell or producible set changes. Which of those two is the 1.2 s is still open.

## 4. Proposed changes

Ranked by the gain expected on the measured late year (4 threads, 15.6 s). None of these are implemented. "Bit-identical" means seed 5 at 300 years and the short checks in §7 stay on the same progress lines, and the golden hashes do not move. No `KIT_UPKEEP`, axe term, labour price or `MAKER_*` constant changes in any of them.

### 1. Do not walk a partner line that cannot be travelled

Expected gain on the late year: **about 1.2 s, at 1 thread and at 4.** The scan is serial, so the threads do not eat it. Gain on the Y350 map: noise (the whole visit bucket was 0.08 s/year). Early world: none.

`land_cost` steps one cell at a time along the straight line. Each step costs at least 1, so the total is at least the chebyshev distance `d`. A boat on a shared river then takes the minimum of that and `0.5 * d`. Nothing later in the visit test goes lower. A pair with `d > 2 * reach` loses either way, and this year spent 30% of its walked steps on those pairs (0.51M of 5.14M walks, 16.3M of 55.1M steps, inside a 3.90 s scan).

```rust
let reach = 2.0 + 4.0 * pt.strength + 8.0 * boat + if frame { 1.0 } else { 0.0 };
// land_cost >= d, and a river charges half of d. Twice `reach` is already impossible.
if d as f32 > reach * 2.0 {
    continue;
}
let mut eff = self.land_cost(ca, cc);
```

Bit-identical: yes. The continue happens only when every value `eff` can still take is already above `reach`. The queued visits do not change.

Risk: low, as long as the 2× stays tied to `RIVER_VISIT` (0.5). Using `d > reach` without the river case would drop real river visits. A follow-up, not counted yet: call `same_river` (two loads) and skip when the cheapest mode that is actually available still loses. That is the other 17.8M steps, worth up to about another second if almost none of them share a river, and worth nothing if they all do.

### 2. Keep a closure when the raw set comes back

Expected gain on the late year: **up to about 2 s if a third key hits often. About 0 if it does not.** This is still conditional. It is no longer the only change that can move the year by a second, and it is not the one to write first. Gain on the small early world: none (212 rebuilds in 40 years).

Today, of the times the raw set changed, 15% found the previous toolkit and 85% built a new closure:

```rust
fn refresh_toolkit(&self, b: &mut Band) {
    let raws = self.raws_at(b);
    let key = (raws, b.flame, b.d.known_ver);
    if b.d.tk_key == Some(key) {
        self.ensure_chain(b);
        if b.d.fin_dirty || b.d.kit_dirty {
            b.d.kit = self.fit_kit(b, &b.d.tk);
            b.d.fin_dirty = false;
            b.d.kit_dirty = false;
        }
        return;
    }
    // one previous (raws, flame, known_ver)
    if b.d.tk_prev.as_ref().map(|p| p.0) == Some(key) {
        // swap toolkit and kit back, then ensure_chain + fit_kit
        return;
    }
    let mut tk = closure(&self.reg, &b.known, &raws, b.flame);
    fill_chain_adopting(&self.reg, &b.known, &mut tk, Some(&b.d.tk));
    // ...
}
```

Measured on the Y350 stretch: 316,140 hits, 10,586 swaps, 62,258 rebuilds. Each rebuild is ~19 µs of closure plus fresh fill before `fit_kit`. On the late year, after taking out the one-time load: 1,886,186 hits, 56,851 swaps, 384,496 rebuilds, and the miss rate is 87%. A rebuild is 8.4 + 12.6 µs before `fit_kit`. Turning every one of those into the swap path (replay 3.8 µs plus the fit that the swap already does) is about 6.5 s of CPU. Almost all of it sits in prep, phase 3 and meeting rounds, which took a 3.5× speedup, and 6% of it sits in the serial harvest. That is a ceiling of about 2 s on the 4-thread year, not an expectation.

Proposed, only after a one-year histogram of how many distinct keys a band actually uses:

```rust
// Sketch. Same key, same ensure_chain + fit_kit as the swap path above.
// Do not store four full Toolkit+Kit copies (see memory).
fn refresh_toolkit(&self, b: &mut Band) {
    let key = (self.raws_at(b), b.flame, b.d.known_ver);
    if let Some(slot) = b.d.tk_cache.iter_mut().find(|e| e.key == key) {
        // bring this closure back; refill chain skills; refit the kit
        // exactly as the current swap arm does
        return;
    }
    // build closure + fill_chain_adopting, push the slot, drop the oldest
}
```

The histogram is the patch that decides this one: count, for one late year, how often the third and fourth most recent keys would have hit. If three keys cover most misses, one extra slot is worth it. If the raw mask almost never repeats, stop. The one-pass closure (proposal 3) is the local fallback, and it is only a few tenths of a second.

Bit-identical: yes, if a cached closure is used only when `(raws, flame, known_ver)` matches and the current swap path's `ensure_chain` + `fit_kit` still run. The floats come out of the same replay.

Risk: medium. A wrong key (forgetting `known_ver`, or reusing a closure across a forget that shifted known-list indexes) silently changes every later choice. Memory: one extra full toolkit is a few KB to ~10 KB (via, chain, and the recorded walk of ~46 read ranks × ~6 steps). At 72k bands that is on the order of **0.5–1 GB per extra copy**, on top of the previous toolkit the sim already keeps. Do not keep four full copies. If the histogram says a third key hits, store one extra closure (avail, via, replay), not a fourth kit, and still call `fit_kit`.

Small-world gain: under a percent of a seed-5 300-year run's early centuries, and a similar fraction of its late centuries as on the Y350 map (the chain tax there is real; the absolute time is the published +22 s over step 2, of which this is only the rebuild part).

### 3. Build the closure in one pass, and adopt per item

Expected gain on the late year: **about 0.3 s**. Small early world: noise. Worth doing because it is local, not because it moves the 15.6 s year. The late closure is 8.4 µs and 3.2 s of CPU; a third off that is about a second of CPU and a few tenths of wall once the parallel phases are counted. The fresh fill is 12.6 µs with 19 walks left per rebuild. Halving those walks is the same size, and it parallelizes.

`set_via` drops the rank index and inserts in the middle of `via` on every new item, and `via()` during the closure then does a popcount rank:

```rust
fn set_via(&mut self, i: Item, v: (u16, u32, u8)) {
    self.rank_of.clear();
    self.replay.ready = false;
    if self.avail.has(i) {
        let r = self.rank(i);
        self.via[r] = v;
    } else {
        self.avail.add(i);
        let r = self.rank(i);
        self.via.insert(r, v);
    }
}
```

`closure` calls that from a loop of up to 16 passes over every known recipe. The final `via` is parallel to `avail` in item order. Proposed: while the closure is running, record `(item, recipe index, mask, depth)` in a side list and build `via` once, in `avail` order, after the loop. Then `rebuild_rank` as now. Same tuples, same order.

Separately, `via_stable` gives up on the whole recorded walk if any one item's `via` differs. In this sample that left 16 walks per rebuild next to 28 adoptions. Proposed: adopt a rank when that item's `via` tuple matches and every input named in the stored walk still has the same `via`. Rewalk only the rest. The comment in `via_stable` is right that an input whose `via` moved must not stay in a stored walk. Check that per item instead of rejecting all items.

Bit-identical: yes if the final `via` order matches `avail.iter()` and a stored walk is copied only when its own steps would have pushed the same recipe indexes. The geometric mean then uses the same weights.

Risk: higher than it looks. A wrong adopt changes `chain_m`, which changes upkeep, scrap, failure rolls and the kit, and the world diverges. The one-pass `via` build is the safe half. Land that first and check the hashes before touching `via_stable`.

Gain math on the Y350 sample, kept so the two worlds can be compared: closure is 7.2 µs. Cutting a third of it is ~0.04 s/year there. The per-item adopt bites into the 12 µs fresh fill. Not the main prize on either world. Do it because it is contained in `knowledge.rs`.

### 4. Do not refit the kit when the chain tick cannot change it

Expected gain on the late year: **about 0.5 s for the seasonal refit, and the other fits ride along with proposal 2.** `fit_kit` is 5.3 µs × 728k calls, 3.8 s of CPU. The pre-movement refresh is one fit per band per season (~289k, about 1.5 s of CPU) because practice always sets `kit_dirty`. That part is inside the parallel phase that sped up ~3.5×, so it is about half a second of the 4-thread year. Fits on a rebuild are not free, but they go away only if the rebuild does. Early world: 3 ms in 40 years, ignore.

This is step 4's edit, not this branch's. See §5. The current trigger is unconditional:

```rust
practise_use(&mut b.use_skill, ec.used_forms, floss, b.gen);
b.d.kit_dirty = true;
// ...
b.d.skill_ver = b.d.skill_ver.wrapping_add(1);
```

and the swap arm of `refresh_toolkit` always calls `fit_kit` ("the cached kit was chosen at older skills").

A bit-identical version has to prove the chosen items would not move. A cheaper `slot_score` is a behaviour-preserving speedup only if the comparisons stay on the same side. Either way it is the kit decision, which step 4 is rewriting. The number to hand them: **5.3 µs per fit, 728k times in a late year, of which one per band per season is the unconditional refit.**

### 5. Do not rebuild the teach-list on every exchange

Expected gain on the late year: **about 0.8 s.** `exchange` is 5.9 µs × 1.05M calls, 6.2 s of CPU, but 122k of those calls are the ones that learn a recipe and then rebuild the closure (3-ish seconds of that 6.2, and that cost is proposal 2). The other ~930k calls scan the teacher's ~91 recipes and usually return without teaching. That scan is about 3 s of CPU inside meetings, which sped up 3.6×, so about 0.8 s of the 4-thread year. The Y350 figure was 4.7 µs × 39k calls.

Also step 4 (the close-tie exchange). A bit-identical cache would key off `known_ver` and the listener's available set, and produce the same weighted draw. Do not write it on this branch while step 4 is in that function. The number to hand them: **5.9 µs per call, 1.05M calls in a late year, and only 122k of those learn anything.**

### 6. Leave the harvest serial

Expected gain of parallelizing it: unknown, and a previous parallel plan was not bit-identical (the comment on `take_harvest`: +23% people and +40% war deaths).

The late harvest is 2.2 s and does not shrink with threads. Of the year's rebuilds, 21k (6%) happen inside it, so a cheaper closure takes a few tenths off this 2.2 and leaves the rest. Do not reorder bands. The stats tail is the other serial second (1.2 s) and is not proposed: it was not split between `track_knowledge` and `year_stats`.

### Not proposed

- Lazy per-item `chain_m` that walks on first read. Already measured, 80.8 s, no better than head.
- One batched replay of every recorded rank. Already measured, 90.6 s, worse.
- Skipping the skill replay when the fade is "tiny". `replay_chain` skipped zero times because practice changes the bits. Skipping anyway would change `chain_m` and is not bit-identical. The replay is only 3.4 µs, so there is little to win.
- A denser `chain_m` table. 5.5M lookups are not the hot part.
- Neighbour-grid work. 0.67 s/year on the late map at 4 threads (2.1 s at 1 thread). Faster than the bands, still a small slice.
- Regrow work. 0.03 s/year on 640k cells.
- Anything that touches `KIT_UPKEEP`, the axe term, labour pricing, or a `MAKER_*` constant.

A one-line measurement fix, with no gain: lap the clock **around** the two `refresh_toolkit` calls in `band_season`, so the next profile stops charging chain fill to movement and experiments. Do it whenever the next person adds a counter. It is not in this commit.

## 5. Overlap with skills step 4

Step 4 rewrites the kit decision and valuation: `slot_score`, `choose_food` and `make_kit_with_view` in `effects.rs`; the decision side of `refresh_toolkit` in `world.rs`; band beliefs; the season economy's payout handling; splits; and the close-tie exchange.

| proposal | conflicts with step 4? |
|---|---|
| 1. skip impossible partner walks | No. The test is in `contacts`, before `land_cost`. Step 4 does not own that loop. |
| 2. closure cache inside `refresh_toolkit` | The function is shared. The cache lookup is not the decision. The `fit_kit` call on a hit must stay whatever step 4 turns it into. Land the cache as "find a matching closure, then do exactly the current hit/swap tail", and expect a merge against step 4's edit of the same function. |
| 3. one-pass `closure` and per-item adopt | No. Both live in `knowledge.rs` under the chain walk. Step 4 reads `chain_m`; it should not need to rewrite the walk. |
| 4. refit less often, or score the kit faster | Yes. This is `fit_kit`, `kit_dirty`, and `effects.rs`. Leave it. |
| 5. cache the teach list | Yes. `exchange` and the close-tie branch in `trade` (`CLOSE_TIE`). Leave it. |
| 6. harvest order | Do not touch. Splits are also step 4's; the new-band `refresh_toolkit` at the end of demography is 1,067 rebuilds in the late year. |
| economy's `chain_m` multipliers (upkeep, failure, scrap) | Those are payout-adjacent. Do not "clean them up" while step 4 is in the economy. They are not hot as lookups. |

Beliefs are not on this hot path. Nothing above proposes storing or scoring a belief.

## 6. Recommended order

1. Land the partner-line skip (proposal 1) on its own commit. It is bit-identical and it is the measured serial second. Re-time the Y750 stretch at 4 threads: the visit bucket should drop by about a second and nothing else on the progress line should move.
2. Add a key histogram for one late year (third and fourth most recent raw key). The harvest share is already measured: 21k of 384k rebuilds. The histogram decides whether proposal 2 is real. Count, while there, how many of the 1.12M mid-distance partner walks share a river.
3. If the third key hits often, land one extra closure slot, bit-identical, on its own commit, and re-time the Y750 stretch and seed 5. Watch memory at 72k bands before trying it on (b). One extra closure is on the order of 0.5–1 GB.
4. Land the one-pass `closure` (proposal 3, safe half) on its own commit. It is a few tenths of a second, not the reason to delay 1.
5. Hand step 4 the late numbers: `fit_kit` 5.3 µs, 728k times a year, ~0.5 s of wall in the unconditional seasonal refit; `exchange` 5.9 µs, 1.05M times, ~0.8 s of wall in the scans that do not teach. Do not edit those functions here.
6. Do not retry the lazy chain or the batched replay. Do not parallelize the harvest. Do not chase regrow or the neighbour grid.

## 7. How to verify

No sim change is in this commit, so there is nothing new to hash. For any later commit that does take proposal 1 or 2:

- `cargo test --release --workspace`. Every golden hash stays put, including the fixture hashes.
- `cargo build --release -p kindling` prints no warnings.
- Bit-identity, 1 thread: `kindling --headless --seed 5 --years 300 --threads 1 --progress-every 50`. Progress lines match main except the trailing seconds. Final population on main is 104,860 for step-3 head.
- Bit-identity, short: `kindling --headless --seed 1 --map 128x80 --years 200 --threads 1 --progress-every 50`. The final line matches main.
- Populated stretch, local, both thread counts: `kindling --headless --load <y350 save> --seed 6 --years 4 --threads 1` and the same with `--threads 4`. The progress lines from this note (Y351 pop 68795 bands 3393, through Y354 pop 67213 bands 3441) should match apart from the seconds. `--threads` must not change a single other field. They did not, in the runs above.
- `bench.yml` (workflow_dispatch): seed 5, years 300, map blank (default 256×160), threads `1 2 4`, baseline the step-2 ref or blank. It already sets `KINDLING_PROF=1` and records the profile. It cannot `--load`. Use it for the seed-5 wall, not for the late world.
- Large worlds: `headless-sims` (b), seeds `499246292..301`, years 750, map `800x800`, threads 4. Compare per-seed wall time and the Y750 population line to run 41569759385 (step 3: 17,346 s, the same populations). A bit-identical chain change should keep every population and move only the time. The slow seeds are the ones that matter (292 at 3063 s, 296 at 2649 s on main). Hosted runners vary; compare inside one dispatch, test against baseline, the way `bench.yml` does.
- Late-year slice, already run for this note: `--load` the Y750 save, `--years 0` (load), `--years 3 --threads 4` on the clean binary, `--years 1 --threads 1` with the counters. A change to proposal 1 or 2 should keep the Y751 line (pop 1308981, bands 72328, and the rest of that line) and move the seconds.
