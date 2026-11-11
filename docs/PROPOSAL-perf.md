# Where the time goes, on small worlds and on large ones

Status: proposal only. No sim change in this commit. Written for Michael, against `main` 3b2a515 (skills step 3). The late 800×800 profile is in section 3. Rankings below are by expected seconds saved on that world, at 4 threads.

Nothing here changes behaviour, `KIT_UPKEEP`, the axe term, labour pricing, or any `MAKER_*` constant.

## 1. How this was measured

Local runs only, one at a time, release binary, `KINDLING_PROF=1`. That flag is off unless the variable is set: the timers are not created, so normal play and CI do not pay for them.

| run | what | threads | clock | world at the end |
|---|---|---:|---:|---|
| early | seed 5, 300 years from scratch | 1 | 80.1 s | Y300, pop 104860, bands 6050, 71 items |
| mid | load the seed-6 year-350 save, then 25 years | 1 | 15.7 s | Y375, pop 51365, bands 2835, 81 items |
| mid | the same stretch | 4 | 8.4 s | the same world, line for line |
| late | load the seed-499246296 year-750 save, then 3 years | 4 | 61.4 s (6.0 s of that is the load) | Y753, pop 1309652, bands 72440, 101 items |
| late | the same save, then 2 years | 1 | 83.9 s (6.3 s load) | Y752, pop 1311074, bands 72406, 101 items |

The 80.1 s figure is the uninstrumented phase profile. A second checkout of the same commit matched it apart from clock noise. The fine counters on seed 5 and on the seed-6 stretch (how many times the toolkit was rebuilt, and where) come from a temporary instrumented build of the same commit. That build took 82.2 s on seed 5, about 3% slower, and has been reverted. It is not zero-cost when the flag is off, which is why it is not in the tree.

The late stretch used a second, smaller set of counters, also reverted before this commit: time and counts inside `refresh_toolkit` (hit / swap / rebuild, closure, fit, chain fill) and, in the partner loop, how many far pairs fail a cheap reach bound and how long `land_cost` takes. Those counters do not change the world. One thread and four threads produced the same year-751 and year-752 lines (pop 1308981 / 1311074, bands 72328 / 72406, same experiments, inventions, learned, forgot). Per-call costs below are the one-thread numbers. At four threads the same counts cost about 1.2× the CPU, which is overhead, not more work.

The mid save is `main_seed6_y350_256x160.civsave`: year 350, pop 67836, 3354 bands, 81 items, steel. The late save is `main_s499246296_y750_800x800.civsave`: year 750, pop 1301977, 72169 bands, 101 items, mean 52.5 items on the band, steel, 239775 cells in range. Both are CI saves, not user saves. Loading the late save took 6–9 s. `--load` plus `--years 3` means three more years, not "run until year 3".

The printed `pairs in range` and `meetings` are cumulative since year 0, so they include the save. The late stretch itself added 13.1 million pairs and 0.47 million meetings per year (2379.5 M → 2418.7 M pairs and 115.7 M → 117.1 M meetings over the three four-thread years).

How to read the phase line. The five top-level numbers (regrow, band seasons, contacts, demography, stats) are wall-clock and add up to the sim. Under contacts, the three sub-lines are wall-clock too. Under band seasons, economy, the harvest log and the merge are wall-clock. Prep, farming, experiments, movement and "other" are worker-seconds: each worker's time, added up. On one thread that is the clock. On four threads it is not a share of the wall, and those worker-seconds came out a little higher for the same counts.

The seed-6 slice is a decline, not a boom: pop 67836 → 51365. It is still a populated year. The late slice is flat: pop stays at about 1.31 million and bands at about 72,300. People per band are about 18.

Full-length timed comparisons stay on GitHub Actions. The 750-year run of this seed on main, 4 threads, was 2953 s (run 43021503049), which is 3.9 s per year averaged across the whole history. The year measured here is the expensive end of that run, not the average.

## 2. Early / small hotspots (seed 5)

Seed 5 is the whole growth, from six founder bands to 6,050, averaged into one 80.1 s. A year costs 0.27 s on average. The late years of this same run cost more than that; the early years cost less. There is no separate late-year profile of seed 5.

Wall-clock, one thread:

| slice | seconds | share |
|---|---:|---:|
| meetings | 21.9 | 27% |
| movement | 15.3 | 19% |
| partner visits | 9.7 | 12% |
| economy (the harvest, serial) | 6.3 | 8% |
| experiments | 5.6 | 7% |
| the rest of the band's season | 4.1 | 5% |
| finding neighbours | 4.0 | 5% |
| not in a sub-timer | 4.4 | 5% |
| fitting toolkits at the start of the season | 3.3 | 4% |
| year stats | 2.9 | 4% |
| regrow, farming, harvest log, merge, demography | 2.6 | 3% |

Band seasons are 40.5 s (51%). Contacts are 35.6 s (44%). The two together are the sim. Regrow, farming and demography are not.

The fine counters, on the slightly slower instrumented run, put one function under almost half of that: `refresh_toolkit`, 37.6 s, 13.5 million calls.

- 11.7 million (87%) are cache hits. The two-entry cache (a band's raw materials flip as stocks are eaten and regrow) is doing its job.
- 1.52 million are full rebuilds: recompute the closure, walk the production chains, fit the kit.
- 1.82 million refill the chain because recipe skill moved; 10.1 million skip that.
- Fitting the kit is 4.0 million calls and 13.0 s. Closing the toolkit is 6.3 s. Walking chains is 9.4 s inside a 13.2 s fill. Replaying a stored walk is only 3.5 s, and almost every replay recomputes every rank (1.81 million full, 0.01 million partial).

Where the 37.6 s is spent:

| call site | calls | seconds | what it is |
|---|---:|---:|---|
| movement | 1.95 M | 13.0 | once a season, after practice; always dirty |
| meet and trade | 4.07 M | 8.6 | only when goods actually moved |
| teach | 0.40 M | 5.8 | a recipe was just learned; full rebuild |
| experiments | 1.95 M | 3.3 | once a season |
| prep | 1.95 M | 3.2 | once a season, phase 1 |
| harvest | 1.95 M | 2.5 | once a season, serial |
| partner trade | 1.19 M | 1.2 | far visits that moved goods |
| newborn | 0.01 M | 0.1 | |

Same call count at prep and at movement, four times the cost at movement. Practice runs every season, bumps `skill_ver`, and sets `kit_dirty`, so the movement call always replays the chain and refits the kit. Teach is rare and expensive: about 15 µs, against about 3 µs for an ordinary call.

Beside the toolkit: planning the harvest 3.7 s, `mark_used` 3.1 s (that is the 4.4 s that no sub-timer owns), `maybe_move` 3.5 s. Movement's 15.3 s is mostly the toolkit refresh (13.0 s) plus scoring a handful of neighbouring cells (3.5 s).

Partial chain replay does not pay on this run either. A replay updates every rank when more than a quarter of the recipe skills moved. Practice moves about 43 of ~59 skills (`changed/replay` 43, `known/skills` 59), so 1.81 million of 1.82 million replays take the full pass. The inverted index is effectively dead.

## 3. Mid / large hotspots

### Seed-6 year 350, measured

Twenty-five populated years. One thread: 0.62 s per year. Four threads: 0.33 s per year.

The mix is almost the same as seed 5. Contacts are 42% instead of 44%. Band seasons are 54% instead of 51%. What changes is the cost of a year, because every year has ~3,000 bands, and each band knows more (77 recipes in the chain fill, against 59 averaged over seed 5's growth). A refresh costs 3.8 µs instead of 2.8 µs.

Per year, this stretch against seed 5's lifetime average:

| | seed 5 | this stretch | ratio |
|---|---:|---:|---:|
| meetings | 0.073 s | 0.188 s | 2.6× |
| movement | 0.051 s | 0.140 s | 2.7× |
| toolkit refresh (CPU) | 0.125 s | 0.328 s | 2.6× |
| experiments | 0.019 s | 0.048 s | 2.5× |
| economy | 0.021 s | 0.044 s | 2.1× |
| partner visits | 0.032 s | 0.052 s | 1.6× |
| stats | 0.010 s | 0.020 s | 2.0× |
| finding neighbours | 0.013 s | 0.016 s | 1.2× |

Neighbour search does not grow with the populated year. It already walks an occupancy grid, only inside `teach_radius` (further along a river, or twice as far for riders on open ground), and it parallelizes. Meeting *count* will not explode with crowding either: a band stops after 4 meetings a season. This stretch is at about 2.6 participations per band per season, so there is some headroom, and then meetings grow with bands, not with density.

On one thread the toolkit is half the sim (8.2 s of 15.6 s). It splits cleanly onto the phase lines:

- Movement, 3.5 s = refresh 2.74 s + scoring cells 0.72 s.
- Economy, 1.1 s ≈ refresh 0.47 s + planning the harvest 0.53 s.
- The unnamed 0.5 s inside band seasons = `mark_used`.
- Meetings, 4.7 s, include a 1.52 s teach rebuild and a 1.57 s trade refresh. About two thirds of meeting time is the toolkit.

Teach is 94,000 calls and 16 µs each. A cache hit at prep is 2.7 µs. The expensive calls are the ones that rebuild.

Of 2.16 million refreshes, 1.76 million hit, 0.06 million swap to the previous raw-material key, 0.34 million rebuild. Of those rebuilds, 0.23 million still had the same `via` as the toolkit they replaced (the walk could be copied) and 0.11 million did not. The closure is recomputed from scratch in both cases. Replay of a skill-only change is 0.59 s. The partial replay runs 2,000 times against 254,000 full replays, for the same reason as on seed 5: 56 of 77 skills move.

### What four threads actually speed up (seed 6)

| slice | 1 thread | 4 threads | speedup |
|---|---:|---:|---:|
| meetings | 4.7 s | 1.9 s | 2.5× |
| finding neighbours | 0.4 s | 0.2 s | 2.0× |
| band seasons (whole) | 8.4 s | 4.1 s | 2.0× |
| of which the parallel part | 7.1 s | 2.5 s | 2.8× |
| partner visits | 1.3 s | 1.2 s | 1.1× |
| economy | 1.1 s | 1.4 s | none (serial) |
| stats | 0.5 s | 0.6 s | none |
| demography | 0.1 s | 0.2 s | none |

At four threads the slices that did not speed up are 3.7 s of 8.2 s, about 45%. That is the Amdahl cap on this map. Economy is serial on purpose: planning every band against the start-of-season stock was tried, and it left stocks fuller, with about +23% people and +40% war deaths. Do not parallelize it.

On that 256-wide map, at the thread count CI uses, the order of the wall-clock was: the parallel band season (half of it still the serial harvest), then meetings, then partner visits, then year stats.

### Late 800×800, year 750, measured

Three years at 4 threads, two years at 1 thread, from the save above. About 72,300 bands the whole way. Phase time, not including the load:

| | 1 thread, per year | share | 4 threads, per year | share | speedup |
|---|---:|---:|---:|---:|---:|
| whole sim | 38.8 s | | 18.5 s | | 2.10× |
| meetings | 10.65 s | 27% | 3.70 s | 20% | 2.9× |
| partner visits | 7.15 s | 18% | 5.87 s | 32% | 1.2× |
| movement (CPU; wall only at 1 thread) | 7.00 s | 18% | 7.63 s CPU | | does not fall |
| band seasons, whole | 16.90 s | 44% | 6.90 s | 37% | 2.4× |
| of which economy (serial) | 2.30 s | 6% | 2.33 s | 13% | none |
| of which the parallel part of the season | 14.2 s | | 4.1 s | 22% | 3.4× |
| finding neighbours | 2.30 s | 6% | 0.70 s | 4% | 3.3× |
| experiments (CPU) | 1.60 s | 4% | | | inside the 3.4× |
| prep, toolkits (CPU) | 1.75 s | 5% | | | inside the 3.4× |
| year stats | 1.55 s | 4% | 1.17 s | 6% | 1.3× |
| demography, regrow, harvest log, merge | 0.7 s | 2% | 0.6 s | 3% | none |

The four-thread year is 18.5 s. The three years were 20.4 s, 16.7 s and 18.3 s, so that average is not one slow year. Regrow is under 0.1 s/year even though 800×800 is past the 65,536-cell cutoff and the regrow does run in parallel. It is not a target.

What did not speed up is 10.0 s of the 18.5 s, about 54%. On the seed-6 stretch that serial block was the harvest plus a small partner scan. Here the partner phase is most of it (5.9 s), then the harvest (2.3 s), then year stats (1.2 s).

Contacts and meetings, per year, against the earlier runs:

| | seed 5, per year | seed 6, per year | late, per year |
|---|---:|---:|---:|
| pairs in range | 0.11 M | 0.20 M | 13.1 M |
| meetings | 0.008 M | 0.016 M | 0.47 M |
| participations per band per season | | 2.6 | 3.2 |
| partner links on the band | | ~15 | 43.2 |
| known recipes | 59 (run average) | 77 | 90.9 |
| producible items | | | 52.2 |

The meeting cap is 4. This world is at 3.2, so extra neighbours mostly do not become extra meetings. Pairs per band per year are about 180, against about 60 on the seed-6 stretch, and neighbour search is still only 4% at four threads. It scales. A spatial index would be spent on a cost we can see, and the cost is small.

`refresh_toolkit`, one thread, where the CPU time is the clock. 15.0 s per year, 39% of the 38.8 s. That is 207 µs per band per year, against 106 µs on the seed-6 stretch and a run-long 125 µs average on seed 5. Eight calls per band per season, the same shape as before (prep, harvest, experiments, movement, and the meetings that actually move goods or teach).

| | seed 5 | seed 6 | late, 1 thread |
|---|---:|---:|---:|
| per refresh call | 2.8 µs | 3.8 µs | 6.4 µs |
| per band per year | 125 µs | 106 µs | 207 µs |
| calls per band per season | ~7 | ~7 | 8.1 |
| hit / swap / rebuild | 87% / 2% / 11% | 81% / 3% / 16% | 81% / 3% / 16% |
| closure, per year | 0.021 s | 0.069 s | 3.23 s |
| closure, per rebuild | | 5.1 µs | 8.4 µs |
| fit, per year | 0.043 s | 0.105 s | 4.16 s |
| fit, per call | 3.2 µs | 3.7 µs | 5.7 µs |
| fits per band per season | | | 2.5 |
| known recipes at a rebuild | 59 | 77 | 92 |

The per-call cost grew faster than the list (6.4 / 3.8 = 1.7×, lists 91 / 77 = 1.2×). Closure per rebuild grew the same way (8.4 / 5.1 = 1.6×). That is the multi-pass scan over `known`. Chain refill on a hit is 0.84 s per year, small next to closure and fit. The four-thread CPU for the same refreshes was 18.1 s per year. Do not add that to the 18.5 s wall.

Per band per year, the whole sim at four threads: about 110 µs on the seed-6 stretch (0.33 s, ~3,000 bands), about 256 µs here (18.5 s, ~72,300 bands). Bands are 24×, the year is 56×, and each band is 2.3× more expensive. The extra per band is the longer list, the 43 partner links, and meetings sitting closer to the cap. It is not the map area. Partners who are far enough to take the visit path are 10.7 cells apart on average, not hundreds.

The partner phase is the one the earlier estimate got wrong. It is 5.9 s per year at four threads, 32% of the wall, and it only sped up 1.2×, so about 5.2 s of it is serial. `land_cost` is 0.61 s of that. The other ~4.6 s is walking the partner list (about 6 million links a year after the two ends of a pair are folded together). Of the far pairs:

- 31% fail the cheap bound (distance, or half the distance on a shared river with a boat) and 99% of the pairs that fail after the walk are in that 31%. The bound is a true lower bound. It does not fire on a different landmass: zero pairs, because almost every band has a boat.
- 66% are inside the bound and then lose the random roll. They still pay for the walk, and a correct bound must not skip them.
- 3% visit. Those visits, and not the terrain walk, are the part of this phase that sped up with threads (about 2.0 s per year at one thread, about 0.7 s at four).

So rejecting before `land_cost` saves about 0.2 s per year, not the 1–2 s the scaling guess suggested, and not the 5.9 s phase. The guess assumed walks would grow with the width of the map. They did not. The list got longer (15 links → 43) and stayed local.

## 4. Proposed changes

Ranked by expected seconds saved per late year, at 4 threads, on this year-750 world (18.5 s). Gains on the small world are the measured seed-5 and seed-6 shares. Where the late number is still a range, the section says what was not split out.

### 1. Rebuild a toolkit by extending the old one, when a band only learned a recipe

Bit-identical: yes, if it is done carefully. Risk: medium. This is the one to test with a hash, not just a summary line.

Closure is 3.2 s per late year at one thread (8% of that year), 8.4 µs per rebuild, and the rebuild is 16% of refreshes. Lists are already 91 recipes, and the cost per rebuild rose 1.6× from the seed-6 stretch while the list rose only 1.2×, which is what a multi-pass scan does. Two thirds of rebuilds on the seed-6 stretch ended with the same `via` as the toolkit they replaced. That fraction was not counted again here. The closure is thrown away and redone in either case.

A rebuild always starts from nothing:

```rust
let mut tk = closure(&self.reg, &b.known, &raws, b.flame);
fill_chain_adopting(&self.reg, &b.known, &mut tk, Some(&b.d.tk));
```

`closure` rescans every known recipe, up to 16 passes, and keeps the first way to make each item:

```rust
let mut changed = true;
let mut guard = 0;
while changed && guard < 16 {
    changed = false;
    guard += 1;
    for (k, kn) in known.iter().enumerate() {
        let r = &reg.recipes[kn.recipe as usize];
        if !r.inputs.iter().all(|&i| tk.avail.has(i)) {
            continue;
        }
        if tk.producible.has(r.output) {
            continue; // first writer wins
        }
        // record via, mark producible, changed = true
    }
}
```

A learned recipe is appended. `known_ver` bumps. Nothing already producible can change its `via`, because the first writer wins and the old recipes are still first. So a learn can start from the previous closure and try only the new recipe, then anything that new output unlocks, in known-list order, with the same "already producible, skip" rule. A forget shifts known-list indexes (those indexes are stored in `via`) and still wants a full closure. A change of raw materials or of the flame still wants a full closure; the two-entry cache already covers the common flip.

```rust
fn closure_after_learn(reg: &Registry, known: &[Known], prev: &Toolkit) -> Toolkit {
    let mut tk = prev.clone();
    // `known` is `prev`'s list plus recipes pushed at the end.
    // Try those, then rescan only while a newly producible item can unlock
    // a later recipe. Same order, same first-writer rule, as `closure`.
    tk
}
```

The chain walk already copies the stored steps when `via` is unchanged. After an incremental closure that left old `via` entries alone, that path should hit. Today it hits only after the full closure has been paid. This pass did not time that walk on its own, and did not split closure time by call site, so the wall-clock saving is a range: the 3.2 s is one-thread CPU, meetings scale 2.9× and the parallel season scales 3.4×, and some rebuilds sit in the serial harvest. Call it 1–2 s per late year at four threads. That is the best saving that is both measured and a real code change. It also gets worse per band as lists pass 91, which the other proposals do not.

Do not spend time on the partial chain replay. It was built for "a few skills moved". Practice moves most of them, every season. Measured on both smaller worlds: a few thousand partial replays against a few hundred thousand full ones. Not re-counted here.

### 2. Count items in a flat array in the year-end stats

Bit-identical: yes, if the lost-item events stay in item-id order. Risk: low. `Item` is a `u16` and the `BTreeMap` already iterates in that order.

`year_stats` runs once a year, on one thread. It walked 1.55 s per late year at one thread and 1.17 s at four (6% of the wall). It does not scale. On the seed-6 stretch it was 0.6 s of the whole 25-year four-thread run.

```rust
let mut counts: BTreeMap<Item, u32> = BTreeMap::new();
for b in &self.bands {
    let mut av = b.d.tk.producible;
    // ...
    for it in av.iter() {
        *counts.entry(it).or_default() += 1;
    }
}
```

`NITEMS` is 1,024. A `[u32; NITEMS]`, cleared once a year, incremented by item id, then scanned from 0 to `NITEMS` for the "nobody knows this any more" pass, produces the same counts and the same event order.

The iteration over each band's producible set stays. The tree does not. Expect about 0.5–0.8 s per late year, because this is most of a serial 1.2 s and the tree is the part that is not the producible scan. Worth doing because it is small, serial, and bit-identical. Not because it is the hotspot.

### 3. Leave the kit score alone; do not call it more often in step 4

Bit-identical: a rewrite of the score is not, and this proposal does not do one. Risk of a behaviour change: none, if we only follow the constraint below. Seconds saved on today's code: about zero. The reason it is this high is the size of the path it protects.

Fitting the kit is 4.2 s per late year at one thread, 5.7 µs a call, 2.5 fits per band per season. The refresh around it is 8.1 calls per band per season, 2.3 million calls a year, 81% of them cache hits, 6.4 µs each, 15 s per year. The movement site is the standing one: every season, `band_season` sets `kit_dirty` and bumps `skill_ver` unconditionally, after practice, and then refreshes.

```rust
b.d.kit_dirty = true;
// ...
b.d.skill_ver = b.d.skill_ver.wrapping_add(1);
// ...
self.refresh_toolkit(b); // movement: replay the chain, then fit_kit
self.maybe_move(b, rng, fx);
```

`fit_kit` builds a `BandView` and calls `make_kit_for_view`. Skills step 4 throws this scoring away and replaces it with beliefs (section 5). A cache of today's food-minus-labour score will be deleted. Do not start that cache.

If a belief update, or a scan of the belief table, is placed inside `refresh_toolkit`, it runs on every hit. Eight times a season, not once. At the current fit's cost, one extra pass over every refresh is about 4 s of CPU per late year, and on the order of 1–2 s of the four-thread wall, because most of those calls sit in the parallel season and in meetings. A heavier pass costs more than that. The belief update wants to run once a season, next to the harvest payout, not here.

### 4. Do not walk the map for a partner who is already too far

Bit-identical: yes. Risk: low. Expected gain on this late world: about 0.2 s per year. It was ranked first on the scaling guess. The measurement puts it here.

The partner phase is real: 5.9 s per late year at four threads, 32% of the wall, 1.2× from one thread. The patch only removes `land_cost` for pairs the existing test would reject. That walk is 0.61 s per year. The cheap bound (half the distance on a shared river with a boat, otherwise the distance; and a different landmass with no boat) matches 99% of the pairs that fail after the walk, and it fires on 31% of far pairs. 0.3 × 0.61 s is about 0.2 s. The other 66% of far pairs are in reach and then lose a coin flip; skipping them would change who visits. Mean distance of a far pair is 10.7 cells. Almost every band has a boat, so the landmass reject fired zero times.

The remaining ~4.6 s is the serial walk of the partner list, about 43 links on the band. This patch does not touch that. What inside the list walk costs the 4.6 s (the id map, the linear `partner()` lookup, the reach arithmetic) was not split. Do not rewrite that loop on a hunch. A later profile can time those pieces; until then the 5.9 s is mostly not this change.

The loop stays serial. It uses one shared random stream. Parallelising it by splitting that stream would change who visits.

Current (`world.rs`, the partner loop):

```rust
let d = (b.x as i32 - o.x as i32).abs().max((b.y as i32 - o.y as i32).abs());
if d <= r {
    continue; // already handled as an ordinary meeting
}
let boat = b.d.kit.boat_q.max(o.d.kit.boat_q);
let frame = b.d.kit.frame != NONE || o.d.kit.frame != NONE;
let reach = 2.0 + 4.0 * pt.strength + 8.0 * boat + if frame { 1.0 } else { 0.0 };
let (ca, cc) = (b.cell(w as usize), o.cell(w as usize));
let mut eff = self.land_cost(ca, cc);
if boat > 0.0 {
    if self.same_river(ca, cc) {
        eff = eff.min(terrain::RIVER_VISIT * d as f32);
    }
    // ... sea ...
}
if eff > reach || (lm_a != lm_c && boat <= 0.0) {
    continue;
}
```

Proposed: reject on the cheap bounds, then do exactly the same walk and the same later tests.

```rust
let (ca, cc) = (b.cell(w as usize), o.cell(w as usize));
let (lm_a, lm_c) = (self.map.landmass_of(ca), self.map.landmass_of(cc));
if lm_a != lm_c && boat <= 0.0 {
    continue;
}
let cheapest = if boat > 0.0 && self.same_river(ca, cc) {
    terrain::RIVER_VISIT * d as f32
} else {
    d as f32
};
if cheapest > reach {
    continue;
}
let mut eff = self.land_cost(ca, cc);
```

Any pair rejected here would have been rejected by the existing `eff > reach` test, or by the existing different-landmass test. Pairs that are close enough still walk the line, and still roll the same random number. On the small world this is a modest cut of a 1.2 s serial slice. On the late world it is the 0.2 s above. Still bit-identical, still fine to land whenever someone is in that function. It is not what moves the late year.

### 5. Do not parallelize the harvest; only stop allocating inside it

Bit-identical: yes for the allocation. Parallelizing the harvest is not bit-identical, and it has already been measured to change the world (+23% people). Leave the band order alone.

The serial harvest is 2.3 s per late year at four threads, the same at one thread (13% of the wall). On the seed-6 stretch about half of the one-thread economy line was a toolkit refresh that usually hits, and half was `economy`. `home_range` allocates a `Vec` of cells for every band, every season.

```rust
pub fn home_range(&self, b: &Band, kit: &Kit) -> Vec<(usize, f32)> {
    let mut cells = Vec::with_capacity(16);
    cells.push((ci, 1.0));
    // eight neighbours, then an allocated sea list if there is a boat
}
```

A small array on the stack covers the land cells. The sea list is the only part that is sometimes long. The late profile did not split the 2.3 s into refresh versus `home_range`, so the saving is still the small-world estimate scaled up: under 0.3 s per late year. Do it after 1 and 2, or skip it.

### Not proposed

- Fewer calls to `refresh_toolkit` on the hit path, on today's score. Hits are 6.4 µs. The call count looks alarming and is not the problem. Step 4 is the case that changes this, and that is proposal 3.
- Waking up the partial chain replay. Measured dead on both smaller worlds.
- A spatial index for neighbour search. It is 4% of the late year at four threads, 13 million pairs a year, and it scales 3.3×. Meetings are already near the cap of 4, so the extra pairs are skipped.
- Anything that changes who wins a tie, what a tool costs, or which recipe is tried. That is step 4 and step 5, not a performance patch.

## 5. Overlap with skills step 4

Step 4 (beliefs) rewrites the kit decision and the way a band values a tool. The code it replaces:

- `effects.rs`: `slot_score` / `slot_score_m`, `choose_food`, `make_kit_with_view`, `make_kit_scored`. The current decision is food minus labour from the true effects, one pass that fills payoff slots, and a second pass only when a food slot's inputs moved.
- `world.rs`: `fit_kit` and `decision_view`, the decision side of `refresh_toolkit`. The closure, the chain walk and the cache key are not the decision. They feed it.
- The belief table on the band, empty until this step.
- The season economy's payout (food, labour, materials actually used), which is what a belief is allowed to learn from.
- Splits, where a daughter copies beliefs with less weight, and the close-tie exchange, where testimony is weaker than experience.

What that means for the list above:

- Proposals 1, 2 and 4 do not touch the decision. They stay valid after step 4. A slower closure makes every belief-driven refit slower too, because the chain is still an input to layer 1 even when layer 2 stops reading `tool_effects`.
- Proposal 3's scoring formula is step 4's job. Do not pre-optimize `choose_food`. Do write step 4 so that the new decision, and the belief update from the payout, run once per band per season. The close-tie exchange already refreshes both toolkits when goods move; that refresh must not also walk the belief table. The late world refreshes 8 times per band per season and 2.3 million times a year. That is the multiplier on a mistake here.
- Proposal 5's harvest loop is where the payout is computed. Step 4 will add an update at the end of it. Keep that update O(tools the band used), not O(every belief the band has ever held), or the serial 2.3 s grows by the size of the belief table.
- Do not "tidy" `KIT_UPKEEP`, the axe term, or `MAKER_*` while doing any of this. Step 4 and step 5 change valuation on purpose. A performance patch that changes them will be indistinguishable from a balance change, and the guard rails will not tell you which one you shipped.

## 6. Recommended order

1. Incremental closure on learn, plus the existing walk-copy. About 1–2 s per late year at four threads. The measured cost that gets worse per band as the list grows past 91. Bit-identical only if a learn does not change old `via`. Test with `World::state_hash`, not just the summary line.
2. Flat arrays in `year_stats`. About 0.5–0.8 s per late year, serial, low risk.
3. Step 4, with the once-a-season rule in section 5. Not a performance patch. Skipping the rule is how a late year gains a second or two it does not have today.
4. The partner reach check, whenever someone is next in that function. About 0.2 s per late year. Do not expect it to move the 5.9 s partner phase. The rest of that phase is the serial list walk, and it is not yet split finely enough to patch.
5. Stack storage in `home_range`, only if a later look still shows the harvest's non-toolkit half as real time. Under 0.3 s on the numbers here.

Neighbour search stays as it is.

## 7. How to verify

Behaviour, for anything in section 4 that claims to be bit-identical:

- The final summary line must match on seed 5, 300 years, 1 thread and 4 threads: pop, bands, items, experiments, inventions, learned, forgot, splits, deaths. The seed-6 stretch already shows that 1 thread and 4 threads match each other on today's code (Y375, pop 51365, bands 2835, and the same counter block). The late stretch does too, through the years both thread counts ran: Y751 pop 1308981 bands 72328, Y752 pop 1311074 bands 72406.
- Prefer `World::state_hash` on a short run from the seed-6 year-350 save (`--load`, `--years` 25 or less) at 1 and 4 threads, against the same stretch on `main`. The summary line can match while a toolkit `via` is quietly different; the hash will not. The late save is the wrong place to iterate on that: loading it is several seconds and a year is 18 s, and the hash over 72,000 bands is a heavier comparison than the seed-6 stretch.
- `cargo test` for the closure and the kit. A learn that does not change old `via`, and a forget that still rebuilds, both want a unit test.

Speed:

- `bench.yml` is the right workflow for the small world. It already exports `KINDLING_PROF=1` and takes `seed`, `years`, `map` and `threads`. Dispatch it on the perf branch and on `main`, same runner family, seed 5, 300 years, map left blank (default 256×160), threads `1 4`. The phase line is in the artifact. It has no `--load` input, so it cannot time a save stretch. A 800×800, 750-year bench does not fit its 60-minute job; don't point it at one.
- The populated stretch stays a local check, or a one-off Actions run that is allowed to `--load`: `--threads 1` and `--threads 4`, `--years` 25, the seed-6 year-350 save. Compare the phase line, not a single total.
- The late stretch is the same shape, and it has now been run: `--years` 2 or 3, not 20. A change aimed at section 4 should move the phase it names (closure inside meetings and the parallel season; stats; the partner line only for proposal 4, and only by a couple of tenths). Another full 750 years is the wrong test for a tenth of a second.
- `headless-sims` (b) is the behaviour check for a large world, not a profile: seeds `499246292..499246301`, 750 years, map `800x800`, 4 threads, against the guard rails in `docs/PROPOSAL-skills.md` (crossings, war rate, no quiet population collapse). Run it when a change is ready to keep, not while tuning. The (a) run, seeds 1–10, 500 years, 256×160, catches a small-world drift the bench's one seed would miss.

Done means: the phase line moved in the slice the proposal named, the hash matched, and (b) did not move a guard rail. A lower total with a different pop is a balance change, and it does not count.
