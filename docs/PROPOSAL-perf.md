# Proposal: where Kindling's time goes

Status: measurement and proposal only. No sim code changes in this document.
Base: `main` at `3b2a515` (skills step 3).
Branch: `perf-profile`.

The short version: a crowded year costs about the same per band on the seed-5 bench and on a later save, and about half of that cost is rebuilding or refitting each band's toolkit. Four threads give just under 2×, the same as the full 300-year bench, because the harvest and the partner-visit scan do not scale. The 800×800 late game is not profiled yet. From these numbers it should be several times slower per year than this save, and the same toolkit work is the first place to look.

Nothing here changes `KIT_UPKEEP`, the axe term, labour pricing, or any `MAKER_*` constant.

---

## 1. How this was measured

Two local runs, both with `KINDLING_PROF=1` (one phase line on stderr at the end). One sim at a time. The binary is the existing release build; it still has the temporary fine counters from the previous pass compiled in. Those counters are not in the source any more (they are not free when switched off). On the seed-5 bench they added about 2.6% (80.1 s clean, 82.2 s instrumented). The phase shares below are from that instrumented binary. A later clean build should be a few percent faster and should show the same shares.

`--load` starts from the save. `--years N` then runs N more years (4 seasons each). It does not mean "stop at year N". The default, if `--years` is left off, is 300 more years. `--seed` only labels the report; the loaded world keeps its own seed.

| Run | What | Threads | Wall |
|---|---|---|---|
| Seed 5, from year 0 | Default map 256×160, 300 years. Clean timing from the previous pass. Ends at 104 860 people, 6 050 bands. | 1 | 80.1 s |
| Seed 6 save, year 350 | `main_seed6_y350_256x160.civsave`. Same map. At load: 67 836 people, 3 354 bands, 81 items, mean 52.5 per band. Then 25 years, to year 375. | 1 and 4 | 15.7 s and 8.4 s |

The 25-year window is not a healthy growing world. An epidemic runs through it (9 epidemics and 7 392 disease deaths at load; 16 and 8 860 at year 375). Population falls, then steadies:

| Year | People | Bands | 1 thread (cumulative) | 4 threads (cumulative) |
|---|---:|---:|---:|---:|
| 350 (load) | 67 836 | 3 354 | | |
| 355 | 66 367 | 3 438 | 3.7 s | 2.1 s |
| 360 | 57 413 | 3 242 | 6.9 s | 3.9 s |
| 365 | 47 068 | 2 915 | 10.0 s | 5.4 s |
| 370 | 47 053 | 2 792 | 12.9 s | 6.9 s |
| 375 | 51 365 | 2 835 | 15.7 s | 8.4 s |

About 0.2 s of each total is loading the save. The simulated stretch is about 0.62 s/year on 1 thread and 0.33 s/year on 4 threads. One thread and four threads produced the same world: same population at every report, same discovery order, same final counts (853 874 experiments, 590 inventions). Thread count is not changing results.

Phase names come from `Prof::NAMES` in `civ-core/src/world.rs`. The five top-level numbers (regrow, band seasons, contacts, demography, stats) are wall-clock. So are economy, the harvest log, the merge, finding neighbours, meetings, and partner visits. Prep, farming, experiments, movement, and "other" are sums of per-band CPU time. On one thread that matches the clock. On four threads those sums stay large or grow, even while the parent wall-clock falls. The fine-counter seconds have the same problem. Use them as a breakdown of work, not as a second clock. The four-thread fine *counts* matched the one-thread counts exactly.

`pairs in range` and `meetings` are lifetime counters stored in the save. The stretch added about 4.9 million pairs and about 0.4 million meetings (29.0 M → 33.9 M pairs, 2.5 M → 2.9 M meetings).

GitHub Actions numbers cited below are full runs on main, not profiles. Batch (b) is `headless-sims.yml`: seeds `499246292..301`, 750 years, map `800x800`, 4 threads. Three of those seeds, as a scale reference: seed 499246292 finished with 1.68 M people in 3 063 s, seed 499246296 with 1.30 M in 2 649 s, seed 499246295 with 340 k in 871 s. Late years on that map are about 12 s per simulated year around year 700, against well under 1 s early. Average over 750 years is only a few seconds per year because the early centuries are cheap.

---

## 2. Early and small worlds (seed 5)

Default map, which is already 256×160. This is the bench in `bench.yml` (seed 5, 300 years, one thread). It is a small world by population, not a small map.

Most of the 80.1 s is the crowded end, not the early years:

| Stretch | Bands at the end | Time | Seconds per year |
|---|---:|---:|---:|
| Years 0–200 | 1 936 | 6.4 s | 0.03 |
| Years 200–250 | 4 237 | 20.1 s | 0.40 |
| Years 250–300 | 6 050 | 53.6 s | 1.07 |

Years 0–200 are 8% of the runtime. There is no separate phase split for them. They are not where a speedup would show up. The phase line for the whole run is already a late-game mix for a few thousand bands.

Wall-clock, 80.1 s, one thread:

| Phase | Seconds | Share |
|---|---:|---:|
| Band seasons | 40.5 | 51% |
| Contacts | 35.6 | 44% |
| Stats | 2.9 | 4% |
| Regrow and occupancy | 0.8 | 1% |
| Demography | 0.3 | under 1% |

Inside contacts, which adds up exactly: meetings 21.9 s (27%), partner visits 9.7 s (12%), finding neighbours 4.0 s (5%).

Inside band seasons (these add to 36.1 s; the other ~4 s is overhead around them): movement 15.3 s, economy 6.3 s, experiments 5.6 s, the rest of the season 4.1 s, toolkit prep 3.3 s, farming 0.7 s, harvest log 0.4 s, merge 0.4 s.

The fine counters (instrumented run, 82.2 s, same world) put `refresh_toolkit` at 37.6 s, just under half the run. It is not a sixth phase. It sits inside prep, harvest, experiments, movement, and meetings.

| Where `refresh_toolkit` was called | Calls | Time | Per call |
|---|---:|---:|---:|
| Prep | 1.95 M | 3.16 s | 1.6 µs |
| Harvest | 1.95 M | 2.52 s | 1.3 µs |
| Experiments | 1.95 M | 3.27 s | 1.7 µs |
| Movement | 1.95 M | 12.96 s | 6.6 µs |
| Meeting trade | 4.07 M | 8.62 s | 2.1 µs |
| Teaching | 0.40 M | 5.83 s | 14.5 µs |
| Partner trade | 1.19 M | 1.17 s | 1.0 µs |
| Newborn | 0.01 M | 0.12 s | |

Of the 37.6 s: fitting the kit 13.0 s (4.05 M fits), filling chain skill 13.2 s, walking production chains 9.4 s, building the closure 6.3 s, replaying a stored walk 3.5 s. Cache behaviour: 11.7 M hits, 0.26 M swaps, 1.52 M full rebuilds. The chain fill ran 1.82 M times and was skipped 10.1 M times. Replay was almost always the full pass (1.81 M full, 0.01 M partial). Mean known recipes over the run 58.7, mean closure size at fill 56, about 5.0 steps per walked item.

Other measured pieces, all small next to refresh: economy math 3.7 s, `maybe_move` 3.5 s, `mark_used` 3.1 s.

Earlier CI on this same bench, after skills step 3: 92 s on 1 thread, 59 s on 2, 49 s on 4 (run 41569769916). Four threads are 1.9×, not 4×.

---

## 3. Mid and large worlds

### Seed 6 at year 350 (this pass)

Same 256×160 map, but every measured year is already late. Fewer bands than seed 5 at year 300 (about 3 100 against about 5 000 over seed 5's last 50 years). Deeper knowledge: mean known recipes 77 against a run-long average of 59 on seed 5, mean closure size 61 against 56, chain walk 5.3 steps against 5.0.

Per band-year the two crowded regimes match. Seed 5's last 50 years are about 210 µs per band-year (53.6 s, roughly 260 thousand band-years). This stretch is about 200 µs (15.5 s of sim, roughly 77 thousand band-years). The earlier band-years inside seed 5 are about 110 µs. So the cost per band rises as lists of recipes get longer, then levels off. It does not keep growing just because the calendar does.

One thread, phase line, 15.6 s accounted:

| Phase | Seconds | Share | Seed 5 share |
|---|---:|---:|---:|
| Band seasons | 8.4 | 54% | 51% |
| Contacts | 6.5 | 42% | 44% |
| Stats | 0.5 | 3% | 4% |
| Regrow and occupancy | 0.1 | 1% | 1% |
| Demography | 0.1 | 1% | under 1% |

Meetings 4.7 s (30% against 27%), movement 3.5 s (22% against 19%), partner visits 1.3 s (8% against 12%), economy 1.1 s (7% against 8%), experiments 1.2 s, prep 0.9 s, neighbours 0.4 s (3% against 5%).

The mix barely moves. What changes is inside `refresh_toolkit`, which is 8.2 s here, 53% of the stretch, against 46% on seed 5.

| | Seed 5, whole run | Seed 6, years 350–375 |
|---|---:|---:|
| Time per refresh call | 2.8 µs | 3.8 µs |
| Calls per band per season | about 7 | about 7 |
| Fit the kit | 13.0 s, 16% of the run | 2.62 s, 17% |
| Walk production chains | 9.4 s | 2.30 s, 15% |
| Build the closure | 6.3 s | 1.72 s, 11% |
| Replay a stored walk | 3.5 s | 0.59 s, 4% |
| Teaching refresh, per call | 14.5 µs | 16.2 µs |
| Movement refresh, per call | 6.6 µs | 8.8 µs |
| Full chain replays vs partial | 1.81 M / 0.01 M | 0.254 M / 0.002 M |
| Rebuilds that could copy the old walk | not split | 0.227 M stable, 0.109 M not |

Same seven-or-so refreshes per band per season: prep, harvest, experiments, movement, about two meeting trades, sometimes a lesson, sometimes a partner. The movement call is the expensive one because practice sets `kit_dirty` every season, so the next refresh refits the kit and refills chain skill. Teaching is the most expensive per call because a new recipe misses the cache and rebuilds the closure.

Partial chain replay almost never runs in either world. Too many recipe skills move in a season, and the code gives up and reapplies every rank.

Four threads, same stretch, wall 8.2 s of phases (1.9×, same factor as the 300-year bench):

| Piece | 1 thread | 4 threads | What happened |
|---|---:|---:|---|
| Band seasons (wall) | 8.4 s | 4.1 s | 2.0×. Prep and the parallel part of the season scale; the harvest does not. |
| Meetings (wall) | 4.7 s | 1.9 s | 2.5×. Best of the lot. Rounds run in parallel. |
| Finding neighbours (wall) | 0.4 s | 0.2 s | 2×. |
| Contacts (wall) | 6.5 s | 3.2 s | 2.0×. |
| Partner visits (wall) | 1.3 s | 1.2 s | Did not scale. The scan of standing partners is serial; the meetings inside it are a small part. |
| Economy / harvest (wall) | 1.1 s | 1.4 s | Did not scale. Band-by-band on purpose. |
| Stats, demography, regrow | 0.7 s | 0.9 s | Serial, and too small to matter on this map. |
| Movement, experiments, prep (CPU sum) | 3.5 + 1.2 + 0.9 s | 4.5 + 1.4 + 1.2 s | Not wall-clock. The sums do not fall when threads are added. |

Regrow is serial on 256×160 because the parallel path only starts at 65 536 cells (this map is 40 960). On 800×800 it already runs in parallel. That is not a lead.

What grows with band count, on this map: almost the whole run, linearly. Seed 5's last 50 years have about 1.7× the bands of this stretch and take about 1.7× the seconds per year. Meetings are not at the cap (`MAX_MEETINGS` is 4 per band per season; this stretch is about 1.3). Neighbour search is a few percent.

What grows with knowledge, not with head-count: time per refresh call (2.8 µs → 3.8 µs), closure cost, and the teaching refresh. Chain *depth* barely moved (5.0 → 5.3 steps). The lists got longer (59 → 77 known recipes), and that is enough.

What does not grow with bands: regrow (grows with the map), demography, stats, and the number of refreshes per band per season.

### Late 800×800 (pending: next pass)

Not measured here. The save `main_s499246296_y750_800x800.civsave` (seed 499246296, year 750, about 1.3 M people) was still being generated. Do not wait on it for this document. Next pass: `--load` that file, `--years 20` or `25`, `--threads 1` and `--threads 4`, `KINDLING_PROF=1`. Same rule as this pass: a short stretch, not another 750 years.

What the full runs already say, without a phase line:

- A 1.3 M-person seed takes about 2 600–3 100 s for 750 years on 4 threads. A 340 k-person seed takes 871 s. Time follows population across seeds, not just the map.
- Around year 700 a year costs about 12 s on 4 threads. This 256×160 stretch costs 0.33 s/year on 4 threads at about 3 000 bands.
- Linear in bands, at the 200 µs per band-year measured here, 60 000 bands would be about 12 s/year on one thread and about 6 s/year on four. The reported 12 s/year on four threads is about twice that. Either year 750 has more bands, or longer recipe lists, or local crowding pushes meetings toward the cap of 4 per season. The phase line on that save is what settles it. Until then the ranking below prefers work that grows with recipe lists and with meetings, not a neighbour index we have not seen dominate.

---

## 4. Proposed changes

Ranked by expected gain on the large worlds (many bands, long recipe lists, the run that actually hurts: 4 threads, late years). Gains on the small world are the seed-5 bench. None of these are implemented. "Bit-identical" means the proposal is only worth doing if a one-thread run and a four-thread run still match today's summary line and discovery order.

### 1. Build the producible set as a worklist

The closure is 6.3 s (8%) of seed 5 and 1.72 s (11%) of the late stretch. Each full rebuild scans every known recipe up to 16 times, and most of those checks fail. Rebuilds are 1.52 M on seed 5 and 0.34 M in this 25-year stretch. A new recipe (the teaching refresh, 14–16 µs, the slowest call) always misses the cache and pays this. Longer known lists make every pass more expensive, which is the late-game direction (59 → 77 recipes already).

On a large world this scales with bands and with list length, and it sits inside meetings, which are the part of contacts that does scale. Expect to remove most of that 8% on the small bench and most of the 11% here, and a larger share once lists grow past 77. Not all of the neighbouring chain walk (15% of the late stretch): 68% of rebuilds already copy the old walk when `via` is unchanged (`via_stable`). The walk that remains is the rebuilds where raws or a forgotten recipe actually change the graph.

Current (`civ-core/src/knowledge.rs`, `closure`):

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
            continue;
        }
        // set_via, mark producible, changed = true
    }
}
```

Proposed: keep the same `set_via` rules, but only look at recipes whose inputs have just become available. Within one wave, still walk `known` in list order, so the first recipe that can make an item still wins. A teach that only appends a recipe can start from the previous toolkit instead of allocating a new one; `via_stable` already says an appended recipe does not rewrite old `via`.

Bit-identical if that order is preserved. Risk: medium. A wave that runs in a different order will pick a different recipe for the same item, and the world diverges. Tests that lock `via` for a fixed known list are the check. Does not touch the kit decision, so it does not collide with skills step 4.

### 2. Reject impossible partner visits before walking the terrain

Partner visits are 9.7 s (12%) of seed 5 and 1.3 s (8%) of this late stretch on one thread. On four threads they stay 1.2 s, which is 15% of the wall. The CI runs that matter are four-thread runs, so a serial slice this size is a bigger part of the number we care about than the one-thread share suggests. It grows with standing partnerships (about 15 links per band on this save), linearly with bands, and it will not shrink when more cores are added.

The scan already skips partners inside the teach radius. Everyone else pays `land_cost`, which walks the straight line cell by cell. `step_cost` is at least 1, so that walk is at least the chebyshev distance. With a boat on a shared river the effort can be `RIVER_VISIT * d` (`RIVER_VISIT` is 0.5). The random draw happens only after the reach test, so a reject that uses a true lower bound does not consume rng and does not change who visits.

Current (`civ-core/src/world.rs`, inside `contacts`):

```rust
let mut eff = self.land_cost(ca, cc);
if boat > 0.0 {
    if self.same_river(ca, cc) {
        eff = eff.min(terrain::RIVER_VISIT * d as f32);
    }
    if lm_a != lm_c || (self.coastal(ca) && self.coastal(cc)) {
        eff = eff.min(d as f32);
    }
}
if eff > reach || (lm_a != lm_c && boat <= 0.0) {
    continue;
}
if rng.chance(0.5 * pt.strength / (1.0 + 0.15 * eff)) {
    far.push((a, c, d));
}
```

Proposed: before `land_cost`,

```rust
let bound = if boat > 0.0 && self.same_river(ca, cc) {
    terrain::RIVER_VISIT * d as f32
} else {
    d as f32
};
if bound > reach || (lm_a != lm_c && boat <= 0.0) {
    continue;
}
```

then the existing `land_cost` and the same `rng.chance`. How many partners are far enough for the bound to fire was not counted. The gain is some fraction of that 8% (one thread) or 15% (four threads), not necessarily all of it. On the large map, if partners drift further apart, the fraction rises. Bit-identical. Risk: low, as long as the bound really is a lower bound (`land_cost >= d`, and the river case uses 0.5). No overlap with step 4.

Do not parallelise this scan by splitting the world rng. That would change the sequence of visits.

### 3. Refit the kit only when a score input changed

Fitting the kit is 13 s (16%) of seed 5 and 2.62 s (17%) of the late stretch: 4.05 M fits there, 0.71 M here, about 3.7 µs each. Almost all of it is one call per band per season. Practice sets the dirty flag unconditionally, and the movement refresh then refits:

```rust
practise_use(&mut b.use_skill, ec.used_forms, floss, b.gen);
b.d.kit_dirty = true;
```

```rust
if b.d.tk_key == Some(key) {
    self.ensure_chain(b);
    if b.d.fin_dirty || b.d.kit_dirty {
        let kit = self.fit_kit(b, &b.d.tk);
        b.d.kit = kit;
        b.d.fin_dirty = false;
        b.d.kit_dirty = false;
    }
    return;
}
```

`fit_kit` calls `make_kit_for_view`. That is the decision. Recipe skill moves on purpose (the replay path reapplies the whole chain because enough skills changed), so a cache key on the exact inputs will still miss most seasons. Expected gain today: low single digits, not the whole 17%. The reason it is on this list at all: step 4 replaces that scorer with beliefs, and each fit gets more expensive, while this flag still forces one full decision per band per season. On a 60 000-band world that is a quarter of a million decisions per year.

Proposed, when step 4's inputs are known: clear the blanket assignment, and set `kit_dirty` only when something `make_kit_for_view` reads has changed (chain skill of a candidate, user skill of a form, recipe quality, the harvest view). Step 4 adds beliefs to that list. Do not rewrite `slot_score` or `choose_food` in the perf work.

```rust
// practice still updates skills and skill_ver exactly as now
if scores_can_move(b) {
    b.d.kit_dirty = true;
}
```

Bit-identical only if `scores_can_move` is complete. A missed input silently changes which tool a band holds. Risk: medium, and higher if it lands before step 4 and has to be redone. Small-world gain: a few percent. Large-world gain: that same few percent now, and a real slice of the late year once beliefs make the decision heavier.

### 4. Leave the neighbour scan alone until the 800×800 profile

Finding neighbours is 4.0 s (5%) of seed 5 and 0.4 s (3%) of this stretch, and it already scales about 2× on four threads. Each band walks a square of radius `teach_radius` (2), doubled for a rider. Pairs in range do grow faster than bands on a fixed map, but at this density the scan is not the problem, and meetings are capped at 4 per band per season so extra pairs are mostly skipped.

On 800×800 the open question is local density, not the map area. If farming country piles bands into the same cells, the inner loop (every occupant of every cell in the square) grows, and more bands hit the meeting cap. That would show up as neighbour time and meeting time jumping together. If the next profile still shows a few percent, do nothing. If it shows a large share, the fix is a coarser grid or a per-cell list, still producing the same pairs and the same per-band random draws. Not bit-identical if the draw sequence changes. Risk of doing it now: spending the change on a cost we have not seen.

No code sketch until that profile exists.

### Not proposed

- Parallel harvest. It was tried. Planning every band against the start-of-season stock gave about +23% people and +40% war deaths (`take_harvest` in `world.rs`). The serial harvest stays.
- Tuning the partial chain replay. It already exists and almost never runs (254 thousand full replays against 2 thousand partial in this stretch), because too many skills move. The cutoff is not the cost. Replay itself is 4% here.
- `raws_at` on the cache-hit path. About 2% of the late stretch.
- Any change to `KIT_UPKEEP`, the axe term, labour pricing, or `MAKER_*`.

---

## 5. Overlap with skills step 4

Step 4 (beliefs) rewrites the kit decision and the valuation behind it. That is `slot_score` / `slot_score_m`, `choose_food`, and `make_kit_with_view` / `make_kit_scored` in `civ-core/src/effects.rs`; the decision side of `refresh_toolkit` and `fit_kit` in `world.rs`; band beliefs; how the season economy turns a payout into a belief update; splits; and the close-tie exchange at a meeting.

Perf work should stay on the other side of that line:

| This proposal | Overlap with step 4 |
|---|---|
| 1. Worklist closure | None. The producible set and `via` stay as they are. Step 4 reads them and does not build them. |
| 2. Partner-visit bound | None. Who is in range does not change, and exchange is untouched. |
| 3. When to refit | Direct. The skip key has to list every input of the decision. Beliefs are new inputs. Doing this before step 4 means doing it twice. The scorer itself is step 4's file. |
| 4. Neighbour index | None, and it waits on a profile anyway. |

The harvest loop, split decisions, and close-tie exchange stay as they are. A faster `refresh_toolkit` helps step 4 (beliefs will call the same refresh, or something shaped like it) as long as the cache key grows a field for "beliefs changed" instead of being bypassed.

---

## 6. Recommended order

1. Worklist closure, bit-identical, with a test that `via` matches the old fixpoint on a fixed known list. Re-time seed 5 (300 years, 1 thread) and a 25-year stretch from the year-350 save (1 and 4 threads).
2. Partner-visit lower bound. Small patch. Same two timings. This is the one that should move the four-thread number more than the one-thread number.
3. Profile the year-750 800×800 save (short stretch, 1 and 4 threads) before any spatial index. Fold the result into section 3.
4. Kit-dirty flag together with step 4, not before. The key has to include beliefs, and the scorer is step 4's.

If the year-750 profile shows meetings or the neighbour scan far above the shares in section 3, item 3 jumps the queue and item 1 still stays first: closure cost per band only gets worse as lists get longer.

---

## 7. How to verify

No behaviour change is intended. A change that is not bit-identical needs an explicit decision; it should not ride along with these patches.

**Bench, small world.** `bench.yml` is manual. Inputs that match section 2: `seed` 5, `years` 300, `map` blank (default 256×160), `threads` `1 4`, `ref` the branch, `baseline_ref` `main` or blank if the baseline is not wanted. The workflow already sets `KINDLING_PROF=1` and records the phase line. Compare wall time and the phase line to section 2 (80 s class on one thread, about 1.9× on four). It has no `--load` input, so it cannot run the year-350 stretch. That stretch stays a local check until someone adds a load input.

**Short stretch, local, both thread counts.** From the year-350 save, `--years 25 --threads 1` and `--threads 4`. The summary line and the discovery order must match each other and, until behaviour changes on purpose, must match this pass: year 375, pop 51 365, bands 2 835, the same discovery-order tail. Loading is `--load <file> --years 25`. Do not pass `--years 375`; that would run 375 more years.

**Batch (b), large world.** `headless-sims.yml`: `seeds` `499246292..301`, `years` 750, `map` `800x800`, `threads` 4. Compare per-seed seconds to the main-code times (the slow seeds are about 2 600–3 100 s, the small seed about 870 s). Guard rails stay the ones in `docs/PROGRESS-skills.md`: crossings, war deaths per year, no population collapse. A bit-identical patch should match main's discovery orders on (a) as well (`seeds` `1..10`, `years` 500, `map` `256x160`).

**Phase line on the late save.** Once `main_s499246296_y750_800x800.civsave` is in place, one short stretch with `KINDLING_PROF=1` confirms or corrects the ranking in section 4. Full 750-year timings stay on GitHub Actions.
