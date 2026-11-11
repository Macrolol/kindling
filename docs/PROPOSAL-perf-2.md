# Perf, second look: where a large world spends its time

Nothing in this change alters the sim. It is a measurement note and a ranked list of changes. The late 800×800 save was not on disk, so every large-world number below is either from the GitHub runs already in `docs/PROGRESS-skills.md` or a scaling of the populated 256×160 world. Those scaled numbers are marked as such.

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

The progress lines of the loaded world match across all three populated runs. After one year: pop 68795, bands 3393. After four: pop 67213, bands 3441. That is Y350→Y354 of the seed-6 save (3,354 bands and pop 67,836 at the moment of the save).

`/workspace/bots/build-bot/opt-saves/` was not there. The 800×800 Y750 save is **pending**. A follow-up should load it and repeat the +4 year stretch at 1 and 4 threads. Do not wait on it for this note.

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

### What a late 800×800 year probably looks like (not yet measured)

GitHub, main code, 4 threads, 750 years, 800×800 (`docs/PROGRESS-skills.md` and the brief):

| seed | people at the end | total sim time | note |
|---|---|---|---|
| 499246292 | 1.68M | 3063 s | |
| 499246296 | 1.30M | 2649 s | the save that was supposed to be local |
| 499246295 | 340k | 871 s | a thin world, much less than a third of the time of the big ones |
| late year at Y700 | | ~12 s/year on 4 threads | early years are well under 1 s |

Step 3 made the whole 750-year (b) run 17,346 s against step 2's 15,991 s (+8.5%). That is a smaller fraction than the seed-5 +37% because the early centuries have almost no chain work (§2). The late years still pay the chain tax measured above.

Scaling from this sample, as an estimate only:

- People are not the loop. `economy` uses population as a float. Experiments draw a Poisson from `sqrt(pop)`. The loops are bands, known recipes, and meeting pairs.
- Band density is similar. 3,441 bands on 256×160 is 0.08 per cell. 40–80k bands on 800×800 is 0.06–0.13 per cell. Neighbour finding was 0.03 s/year here and will stay a small slice if the radius stays a fixed number of cells. It is not where the 12 s goes.
- 60k bands is about 18× this world. 18 × 1.02 s ≈ 18 s/year on 1 thread, and 18 × 0.55 s ≈ 10 s/year on 4 threads. The reported 12 s at Y700 sits next to that (more than 60k bands on the heaviest seeds, plus whatever regrow does on 640k cells). Regrow was under the 0.05 s printing threshold for four years on 41k cells. Sixteen times the cells can still be hidden inside a second per year, or it can be a surprise. **Pending** until the big save is loaded.
- Chain rebuild + fresh fill is ~0.30 s/year here on 1 thread (447+744 ms over 4 years). ×18 bands ≈ 5 s/year on 1 thread. On 4 threads that falls only for the part that runs inside prep, phase 3 and meeting rounds. The part inside `take_harvest` does not fall. I did not tag rebuilds by call site, so the serial share is unknown. A reasonable bracket for the chain-rebuild piece of a 12 s year is **1 to 4 seconds**. The skill replay on top is smaller (~0.04 s/year here, under a second even at 18×, and it parallelizes).
- `fit_kit` is 0.19 s/year here. ×18 ≈ 3 s on 1 thread, on the order of 1–2 s of a 4-thread late year if it stays in the parallel phases.
- `exchange` is 0.19 s/year here, inside meetings, which got a 2.8× speedup at 4 threads. On the order of 1–1.5 s of a 12 s year.
- Serial `take_harvest` is 0.08 s/year here and does not shrink with threads. ×18 ≈ 1.5 s of a late 4-thread year, and more if the rebuilds land in it.

Meetings, the chain rebuild, and the kit refit are the three large pieces. Neighbour search, regrow (probably), demography and `maybe_move` are not.

## 4. Proposed changes

Ranked by the gain expected on a late large world (4 threads, a year that costs about 12 s). None of these are implemented. "Bit-identical" means seed 5 at 300 years and the short checks in §7 stay on the same progress lines, and the golden hashes do not move. No `KIT_UPKEEP`, axe term, labour price or `MAKER_*` constant changes in any of them.

### 1. Keep a closure when the raw set comes back

Expected gain on a large year: **about 1–3 s, if keys repeat. About 0 if they do not.** This is the one change that can move the 12 s year by more than a second, and it is conditional. Gain on the small early world: none (212 rebuilds in 40 years).

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

Measured on the Y350 stretch: 316,140 hits, 10,586 swaps, 62,258 rebuilds. Each rebuild is ~19 µs of closure plus fresh fill before `fit_kit`.

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

The histogram is the real first patch: count, for one populated year, how often the third and fourth most recent keys would have hit. If three keys cover most misses, one extra slot is worth it. If the raw mask almost never repeats, stop and spend the effort on proposal 2 instead.

Bit-identical: yes, if a cached closure is used only when `(raws, flame, known_ver)` matches and the current swap path's `ensure_chain` + `fit_kit` still run. The floats come out of the same replay.

Risk: medium. A wrong key (forgetting `known_ver`, or reusing a closure across a forget that shifted known-list indexes) silently changes every later choice. Memory: one extra full toolkit is a few KB to ~10 KB (via, chain, and the recorded walk of ~40 read ranks × ~5 steps). At 80k bands that is on the order of **0.5–1 GB per extra copy**, on top of the previous toolkit the sim already keeps. Do not keep four full copies. If the histogram says a third key hits, store one extra closure (avail, via, replay), not a fourth kit, and still call `fit_kit`.

Small-world gain: under a percent of a seed-5 300-year run's early centuries, and a similar fraction of its late centuries as on the Y350 map (the chain tax there is real; the absolute time is the published +22 s over step 2, of which this is only the rebuild part).

### 2. Build the closure in one pass, and adopt per item

Expected gain on a large year: **a fraction of a second up to about 1 s**. Small early world: noise. This is the fallback if proposal 1's histogram says keys do not repeat, and it is worth doing either way because it is local.

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

Gain math: closure is 7.2 µs. Cutting a third of it is ~0.04 s/year here, under a second at 18× bands. The per-item adopt bites into the 12 µs fresh fill. Halving the remaining 16 walks might be a few microseconds per rebuild, another ~0.05–0.10 s/year here, around 1 s on a 1-thread large year and less on 4 threads. Not the main prize. Do it because it is contained in `knowledge.rs`.

### 3. Do not refit the kit when the chain tick cannot change it

Expected gain on a large year: **about 1–2 s**. On this sample `fit_kit` is 5.7 µs × 32k calls per year. Early world: 3 ms in 40 years, ignore.

This is step 4's edit, not this branch's. See §5. The current trigger is unconditional:

```rust
practise_use(&mut b.use_skill, ec.used_forms, floss, b.gen);
b.d.kit_dirty = true;
// ...
b.d.skill_ver = b.d.skill_ver.wrapping_add(1);
```

and the swap arm of `refresh_toolkit` always calls `fit_kit` ("the cached kit was chosen at older skills").

A bit-identical version has to prove the chosen items would not move. A cheaper `slot_score` is a behaviour-preserving speedup only if the comparisons stay on the same side. Either way it is the kit decision, which step 4 is rewriting. The number to hand them: **5.7 µs per fit, once a season per band, plus once per raw-set change.**

### 4. Do not rebuild the teach-list on every exchange

Expected gain on a large year: **about 1–1.5 s**. `exchange` is 4.7 µs × 39k calls per year here, dominated by allocating a candidate list and a weight list over the teacher's ~77 recipes. Meetings already parallelize (~2.8× at 4 threads), so this does not get a free 4×.

Also step 4 (the close-tie exchange). A bit-identical cache would key off `known_ver` and the listener's available set, and produce the same weighted draw. Do not write it on this branch while step 4 is in that function. The number to hand them: **4.7 µs per exchange, about two exchanges per meeting, ~22k meetings per year per 3,400 bands.**

### 5. Leave the harvest serial

Expected gain of parallelizing it: unknown, and a previous parallel plan was not bit-identical (the comment on `take_harvest`: +23% people and +40% war deaths). 

The serial year-cost at 18× bands is about 1.5 s plus whatever share of proposal 1 sits inside the harvest loop. Making the refresh cheaper (proposal 1) is the way to shrink this. Do not reorder bands.

### Not proposed

- Lazy per-item `chain_m` that walks on first read. Already measured, 80.8 s, no better than head.
- One batched replay of every recorded rank. Already measured, 90.6 s, worse.
- Skipping the skill replay when the fade is "tiny". `replay_chain` skipped zero times because practice changes the bits. Skipping anyway would change `chain_m` and is not bit-identical. The replay is only 3.4 µs, so there is little to win.
- A denser `chain_m` table. 5.5M lookups are not the hot part.
- Neighbour-grid work. 0.03 s/year here, and density does not jump on the big map.
- Anything that touches `KIT_UPKEEP`, the axe term, labour pricing, or a `MAKER_*` constant.

A one-line measurement fix, with no gain: lap the clock **around** the two `refresh_toolkit` calls in `band_season`, so the next profile stops charging chain fill to movement and experiments. Do it whenever the next person adds a counter. It is not in this commit.

## 5. Overlap with skills step 4

Step 4 rewrites the kit decision and valuation: `slot_score`, `choose_food` and `make_kit_with_view` in `effects.rs`; the decision side of `refresh_toolkit` in `world.rs`; band beliefs; the season economy's payout handling; splits; and the close-tie exchange.

| proposal | conflicts with step 4? |
|---|---|
| 1. closure cache inside `refresh_toolkit` | The function is shared. The cache lookup is not the decision. The `fit_kit` call on a hit must stay whatever step 4 turns it into. Land the cache as "find a matching closure, then do exactly the current hit/swap tail", and expect a merge against step 4's edit of the same function. |
| 2. one-pass `closure` and per-item adopt | No. Both live in `knowledge.rs` under the chain walk. Step 4 reads `chain_m`; it should not need to rewrite the walk. |
| 3. refit less often, or score the kit faster | Yes. This is `fit_kit`, `kit_dirty`, and `effects.rs`. Leave it. |
| 4. cache the teach list | Yes. `exchange` and the close-tie branch in `trade` (`CLOSE_TIE`). Leave it. |
| 5. harvest order | Do not touch. Splits are also step 4's; the new-band `refresh_toolkit` at the end of demography is cheap and was not a visible cost. |
| economy's `chain_m` multipliers (upkeep, failure, scrap) | Those are payout-adjacent. Do not "clean them up" while step 4 is in the economy. They are not hot as lookups. |

Beliefs are not on this hot path. Nothing above proposes storing or scoring a belief.

## 6. Recommended order

1. When `opt-saves/main_s499246296_y750_800x800.civsave` exists, repeat the +4 year stretch at 1 and 4 threads with `KINDLING_PROF=1`. That confirms or kills the 18× scaling. Until then, do not treat the 1–4 s chain bracket as a measurement.
2. Add a key histogram for one populated year (third and fourth most recent raw key, and how many of the 62k rebuilds happen inside `take_harvest`). That is a counter, not a behaviour change. It decides whether proposal 1 is real.
3. If the third key hits often, land one extra closure slot, bit-identical, on its own commit, and re-time the Y350 stretch and seed 5. Watch memory at 80k bands before trying it on (b).
4. Land the one-pass `closure` (proposal 2, safe half) on its own commit. Consider the per-item adopt only if the hashes survive the `via` build and the fresh-fill time is still visible.
5. Hand step 4 the two numbers: `fit_kit` 5.7 µs once a season per band, `exchange` 4.7 µs about twice a meeting. Let them spend the 1–2 s and the 1–1.5 s. Do not edit those functions here.
6. Do not retry the lazy chain or the batched replay. Do not parallelize the harvest.

## 7. How to verify

No sim change is in this commit, so there is nothing new to hash. For any later commit that does take proposal 1 or 2:

- `cargo test --release --workspace`. Every golden hash stays put, including the fixture hashes.
- `cargo build --release -p kindling` prints no warnings.
- Bit-identity, 1 thread: `kindling --headless --seed 5 --years 300 --threads 1 --progress-every 50`. Progress lines match main except the trailing seconds. Final population on main is 104,860 for step-3 head.
- Bit-identity, short: `kindling --headless --seed 1 --map 128x80 --years 200 --threads 1 --progress-every 50`. The final line matches main.
- Populated stretch, local, both thread counts: `kindling --headless --load <y350 save> --seed 6 --years 4 --threads 1` and the same with `--threads 4`. The progress lines from this note (Y351 pop 68795 bands 3393, through Y354 pop 67213 bands 3441) should match apart from the seconds. `--threads` must not change a single other field. They did not, in the runs above.
- `bench.yml` (workflow_dispatch): seed 5, years 300, map blank (default 256×160), threads `1 2 4`, baseline the step-2 ref or blank. It already sets `KINDLING_PROF=1` and records the profile. It cannot `--load`. Use it for the seed-5 wall, not for the late world.
- Large worlds: `headless-sims` (b), seeds `499246292..301`, years 750, map `800x800`, threads 4. Compare per-seed wall time and the Y750 population line to run 41569759385 (step 3: 17,346 s, the same populations). A bit-identical chain change should keep every population and move only the time. The slow seeds are the ones that matter (292 at 3063 s, 296 at 2649 s on main). Hosted runners vary; compare inside one dispatch, test against baseline, the way `bench.yml` does.
- Late-year slice, once the save is in: `--load` that save, `--years 4`, `--threads 1` and `--threads 4`, and read the profile. That is the number this note is missing.
