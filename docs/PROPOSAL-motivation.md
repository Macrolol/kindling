# Proposal: human motivations for Kindling's bands

Status: research and proposal only. No sim code is changed by this document.
Base: `main` at 1771b31 (v0.8.0).
Data:
- The v0.8.0 long run: 8 seeds × 1500 y on a 1200×1200 map (`docs/results-v8-longrun-1200-1500y.txt`).
- Its saves, read with scratch diagnostics (not committed) for seeds 5, 7 and 8.
- The v8 D7 10-seed × 500 y runs (`docs/results-v8-D7-seeds1-10-500y.txt`).

Michael's question was whether the invention stall, the missing long crossings and other problems come from
a deeper behavioural problem: the bands act only on food, while real people also act on fun,
curiosity, novelty and more. The short answer is **partly**:

- **Crossings and lineage dominance** are about behaviour and information. Better motives would help them
  directly.
- **The late invention stall is not motivational.** The physics tree is *exhausted*. Every item that
  can be made from the 13 raw materials has been found by about Y900 in every long-run seed. No motive
  can invent what does not exist. Motives can change *when* and *where* things are found, and they can
  open new kinds of progress (refinement and quality). Truly new items need a larger possibility space.
- **Some of what looks like a problem is measurement.** Examples are the lineage share and continent
  counting on big maps. One "early crossing" was a walk across a strait.

---

## 1. How a band decides today (map)

All in `civ-core/src`. One season is one tick, 4 per year.

| Decision | Where | Driven by |
|---|---|---|
| What it wants (food, tools, warmth, ornaments) | `world.rs` `want` (398) | needs, cold, the kit it lacks; ornaments are a weak want |
| Work split (forage, fish, hunt, farm, craft) | `world.rs` `economy` (1069), `band_season` (1200) | food return per labour, digestion, kit |
| What to tinker with | `world.rs` `attention` (1367) | needed or missing items, newly seen materials, curiosity |
| Whether and how often to experiment | `world.rs` `experiment` (1478) | `lam = invent_rate·curiosity·(0.15 + spare + stress_invent·stress)·sqrt(pop/25)` |
| Where to live | `world.rs` `maybe_move` (1974), `cell_score` (1642) | food potential of the cell and its neighbours, crowding, cold, needed deposits; inertia 1.12, ×(1+1.5·settled·farm_share) |
| Push to leave | `world.rs` `push_out` (1695) | `stress + 0.5·crowd + pressed` |
| Go to sea | `sea_range`/`boat_range` (1718/1732), `voyage` (1740), `sea_season` (1809) | push, a known `far_shore`, boat quality and seacraft; sea move must beat home by about (1.15+0.06·sea)(1−0.3·push) |
| Meet, trade and teach | `contacts` (2085), `trade` (2305), `exchange` (2428) | proximity, kinship, surplus, teach_rate |
| Fight, flee or pay off | `conflict.rs` `confront` (51), `fight` (120), `absorb` (220), `displace` (242) | grudges, strength, aggression, hunger |
| Split | `world.rs` `demography` (2493) | **size only**: pop > split_size (45)·U(0.8, 1.2) |
| Farm | `farming.rs` (adoption ~290, `share_farming` 544) | pot > 0.25 and a digging stick; adoption is pushed only by stress or crowding (0.03·ssow·dpot·push) |

### Motives present
- **Hunger and need.** These dominate work, movement and want.
- **Stress-driven invention.** `stress_invent` is 0.8.
- **Leisure.** Spare labour raises the experiment rate.
- **Curiosity.** A heritable band trait that drifts (range ≈ 0.3–2.5; mean by lineage 0.73–1.53 in the
  long run). It scales the experiment rate and attention, but **nothing else**. It has no effect on
  where a band goes, whether it goes to sea or how far it looks.
- **Fear, grudges and kin loyalty.** These act in conflict and in teaching.
- **Crowding.** It acts only as a food-share divisor and as part of `push_out`.

### Motives missing
- **Novelty and wanderlust in movement.** `cell_score` has no term for unknown or unvisited land. A band
  also already knows the true food value of cells it has never seen (`seen4` falls back to the real
  stock), so the unknown is never uncertain or tempting.
- **Curiosity about the horizon.** Sighting requires `far_shore.is_none()`, so a band that has heard
  any rumour never looks out to sea again (see §3.2).
- **Prestige and status.** There are ornaments, but no status, display, feasting, gifts or
  copying of the successful.
- **Play.** There is no playful tinkering that refines existing tools.
- **Crowding friction and dissent.** Bands split only by size, and daughters are random copies.
  Quarrels, factions and a "leave and found our own" impulse do not exist.
- **Individual ambition.** There is no trait for risk-taking, glory or leadership. Long voyages and
  raids therefore come only from push or hunger.
- **Marriage and exchange of people.** Lineages never mix (see §2).

---

## 2. Do lineages interbreed or merge? (Q2, exact answer)

**No. Lineages never interbreed or mix. They only replace one another by relabelling.**

- **How lineage is tracked.** `Band.lineage` is the id of the founding band (1–6 with the default 6
  founders, `world.rs` ~500). It is a single label, like a surname. It is copied unchanged when a band
  splits (`demography`, ~2555) and when drift founds a band (~1953). Only 6 lineage ids ever exist;
  every long-run seed reports `lineages 6`. Kinship for teaching uses the parent and ancestor chain
  (`band.rs` `kinship`, 410), which is separate from the label.
- **Merge, absorption and captives.** `conflict.rs` `absorb` (220) moves people from the losing band
  into the winner. They take the **winner's** label, and only the loser's know-how is partly carried
  across. Remnants and captives (8–20% of survivors in about 35% of fights) are relabelled the
  same way. In the long run that adds up to 0.35–0.93 M absorption events and 0.56–1.38 M captives
  per seed.
- **Marriage.** None. The code comment that people "marry out" is not backed by any movement of
  people or ancestry between bands.
- **Refugees.** Displaced bands move whole (`displace`, 242) and keep their label.
- **Conquest.** It is the absorption described above. There is no takeover of territory with a mixed
  population.
- **How the metric is computed.** `year_stats` (2798–2801):
  `top_lineage_pct = 100 · max over lineages of (summed pop) / total pop`.
  With 6 founders the even baseline is 16.7%.
- **Why it can reach 66%.** Six labels, no mixing and 1500 years. A label can only gain share through
  differential growth or relabelling, never through mixing. Three effects drive it:
  1. **Wave-front ("surfing") expansion.**
     - Seed 5 climbs steadily from 29% to 66% (`by century [29, 31, 35, 39, 46, 51, 57, 60, 64, …, 66]`),
       with 97–98% of people on one landmass.
     - Seed 8 jumps from 25% to 49% in the 2nd century. Its founders started on a 148-cell island
       3 cells from a 141,729-cell continent, so whichever families got across first filled it.
  2. **Conquest relabelling.** Seed 7 sat flat at about 20% until Y600, then climbed to 64% as fights
     reached about 300–550 k per century.
  3. **Drift.** Curiosity is not the reason. The winning lineages have mean curiosity 1.18 (seed 5),
     0.74 (seed 7, the *lowest* there) and 0.99 (seed 8).
- **Does it overstate dominance?** **Yes, strongly.** It measures one surname surviving, not one
  people replacing all others. With even modest intermarriage, everyone alive after 1500 years would
  descend from all six founders. The effective number of lineages (1/Σp²) at Y1500 is still 2.2
  (seed 5), 2.3 (seed 7) and 2.85 (seed 8). Seed 8's two big landmasses each have their own top
  lineage at about 58%. In the 500-year standard runs the top share is 19–34% (D7 mean 25–26%), which
  is not alarming.

---

## 3. Root causes (with numbers)

### 3.1 The invention stall: the tree is exhausted
- **The tree is finite.** From all 13 raws, the physics in `physics.rs` (`apply`) can make **64
  non-raw items**. The long-run worlds found 64/64 (seed 5) and 63/64 (seeds 7 and 8; the one never
  found is the outrigger canoe).
- **World-firsts per century fall to zero.** Seed 5: 21, 15, 16, 14, 9, 10, 5, 2, 1, then 0 from Y900.
  Seed 7: 19, 17, 16, 25, 3, 9, 2, 0, 1, 0, …
- **There is nothing new left to find.** In 400 sampled bands per seed, a band has about 43 materials
  at hand, can make about 34 of them, and has about 18–20 k possible input combinations.
  - P(random combination yields something new *to the band*) ≈ 0.00017–0.00021.
  - About 3 items are new to an average band, and 9–11% of bands have none.
  - P(something new *to the world*) is exactly 0.
- **What remains is churn.** Experiments keep running at 1.3–2.0 per band-year. "Inventions" become
  re-inventions of locally forgotten items: they are 2% of known recipes and 0.1% of acquisitions,
  against about 300 k learned and 300 k forgotten per year late in seed 5.
- **The early game is slow for a different reason: few bands.** World inventions per year in the first
  two centuries are 0.2–0.7, and inv/exp falls about 10× after the first century (0.003–0.007 to
  0.0005–0.0007). In seed 8, centuries 2 and 3 add only 8 and 4 world-firsts.

So more curiosity or leisure would only speed up the early sweep through the same 64 items. The
plateau then arrives sooner. Ending the stall needs (a) an **open-ended axis**, such as refinement and
quality of known recipes, and/or (b) **more physics**: forms and uses such as paddle and oar, mast,
cordage, net, harpoon, loom and cloth, quern, brick and lime, pigment, glaze and so on. Motives then
decide who explores that space.

### 3.2 No long crossings: information, skill and motive, plus a leak
1. **Stale rumours block new sighting.**
   - 202,752 of 202,772 bands in seed 5 (> 99.9%) carry a `far_shore`.
   - 98–99% of these point to landmasses under 400 cells, at a median of 188–248 cells away (up to
     825). That is far beyond any boat's range.
   - Because the look-out (`sea_season` ~1826) and scouting (~1865) only run when
     `far_shore.is_none()`, almost no band ever sights new land.
   - `far_shore` is copied in 30% of exchanges (2442) and at splits, so the stale rumour spreads to
     everyone. Aimed voyages head for impossible targets.
2. **Seacraft barely grows.**
   - It grows only from fishing from a boat (+0.01·fish_share·q per season) and decays ×0.998 per
     season.
   - Long run: p50 is 0.08–0.09, p90 0.34–0.38 and max 0.81–0.84.
   - Sails are *known* by 50–65% of bands with boats, but *usable* (xp > 0.4) by only 2.5–7 k bands.
3. **Range against gap.**
   - Boat range is `2 + 10q + 12·xp·q` (×1.6 with a usable sail): p50 ≈ 12.6, p90 ≈ 15.6, max 32–35
     cells.
   - Unreached continents sit at gaps of 48–573 cells: seed 5 #2 at 217; seed 7 #3 at 48 and #2 at
     203; seed 8 #1 at 260, #4 at 160 and #5 at 573.
   - The sea-move penalty means a 30-cell trip needs a destination about 2.95× better than home.
   - On the standard 256×160 map (D7), unreached gaps are 17–128 cells.
4. **No motive to go.**
   - Moves are purely food-relative. A well-fed coastal band never has a reason to look beyond the
     horizon. Push comes only from hunger, crowding or being menaced.
   - Real seafaring cultures (Lapita and Polynesian, Norse, Austronesian) were driven by status,
     land-hunger of younger sons, fission after disputes and deliberate exploration.
5. **The strait leak (a bug-like effect).**
   - The scout candidate in `maybe_move` is ±3 cells, or ±6 for mounted bands. Daughter placement in `demography` is ±2 cells.
   - Neither checks the cells in between. `terrain` `land_cost`/`step_cost` see only slope and height,
     and `cell_score` rejects only water *destinations*.
   - So bands walk across straits up to 3–6 cells wide. Seed 8 "landed" on the 141,729-cell continent
     at Y43 with no boat; its first boat was a dugout at Y152.5.

### 3.3 Measurement problems
- `Map::CONTINENT_CELLS = 400` is fixed. On a 1200×1200 map that is 0.03% of the area. "Continents"
  of 400–700 cells sitting 2–13 cells offshore count as crossings, while real continents 48–260 cells
  away stay unreached.
- The gap (`analysis.rs` ~300–335) is a BFS from the *home* landmass only. Seed 8's #3 shows gap 449
  but was reached at Y956, almost certainly from #2.
- `track_landfall` (2060) counts any arrival, including walking across a strait.
- D7 standard-map check: of the reported second-continent landfalls, gaps 1 and 3 (s8 Y188 and s1
  Y143) are within walking or riding reach. The real boat crossings are s1 (gap 17, Y172), s2 (gap 10,
  Y195; gap 37, Y398) and s7 (gap 14, Y302). That is about 3/10 worlds, close to target but below 4–6.

---

## 4. Proposals

Effort: S ≈ under a day, M ≈ 1–3 days, L ≈ a week or more. Every behaviour proposal is meant to be
checked on 10 seeds × 500 y on the standard map first (early game), then on one small long run.

### P0. Measure honestly (do first)
- **Mechanism.**
  1. Scale the continent cutoff with the map, e.g. `max(400, land_cells / 100)`. That leaves the
     standard map at about 400 and gives about 6 k on 1200×1200.
  2. Measure the gap from *settled* land at the time of landfall, not from home.
  3. Tag every landfall as boat-borne or on foot. Count only boat-borne landfalls on continents as
     crossings.
  4. **Start placement:** reject homelands on landmasses below the cutoff, and reject start cells
     within about 6 cells of another landmass (strait-adjacent).
  5. **Close the strait leak:** walking moves (scout candidate, daughter placement, displacement by
     land) must not cross a water cell. Rasterise the line and reject it if any cell is sea. Sea
     crossings go only through `voyage`.
  6. Report the effective number of lineages (1/Σp²), and the top lineage per landmass, next to
     `top_lineage_pct`.
- **Target.** Crossings and dominance metrics. Seed 8's Y43 "crossing" would no longer happen.
- **Expected effect.** Items 1–3 and 6 change no behaviour. Items 4–5 remove free early crossings. The
  standard-map crossing count probably falls from 4 to about 3 worlds, which the later proposals then
  have to earn back honestly.
- **Risks.** Start rejection changes which worlds a seed produces, so old results are not comparable
  seed-for-seed. The leak fix slightly slows riders on coasts.
- **Saves.** Items 1–3 and 6 are analysis-only. Item 5 changes no data. Item 4 affects only new worlds.
  No format change.
- **Measure.** Crossing table with gap-from-settled, on-foot versus boat, and year.
- **Effort.** S (1–3, 6) plus S (4–5).

### P1. Rumours age, and people keep looking ("news fades, horizons tempt")
- **Mechanism.**
  - Store the year a `far_shore` was heard, and how far it is.
  - Rumours fade, e.g. drop after about 50 years without someone confirming them.
  - Rumours spread in `exchange` only if they are within about 2× the receiver's range.
  - The look-out and scouting run **even when a rumour is held**, and keep the nearer or fresher
    shore.
  - Weight the look-out chance by curiosity. Curious bands climb the headland more often.
- **Target.** Crossings.
- **Expected effect.** Bands aim at land they can actually reach. Island-hopping stepping stones become
  visible. Scouting (D7: 1587 runs with 0 finds in 5 seeds) starts finding things. Crossings should
  come at varied times, tied to boats and local geography.
- **Risks.**
  - More landings on small islands, and more sea deaths. That is realistic, but it raises the
    lost-at-sea count.
  - Could push crossings before Y100 if boats come early. P0's start rule and the boat requirement
    guard against this; no date cap.
- **Saves.** New `far_shore_year: u16` with a serde default. Migration: treat existing rumours as old,
  so they fade in a decade. Add a v8 fixture.
- **Measure.** Share of bands with a reachable rumour; sightings and scout finds per century; crossing
  year and gap distribution.
- **Effort.** S–M.

### P2. Seafaring as a lived, taught craft
- **Mechanism.** Seacraft grows from *every* sea trip and from coastal life, not only from boat
  fishing:
  - voyages, especially completed ones, which give a big step;
  - trading by sea;
  - sea-mammal hunting.
  Seacraft is also taught at meetings like recipes, with kin and prestige bias.
- **Target.** Crossings (sail usability, range).
- **Expected effect.** Communities that live with the sea become distinct maritime cultures with
  usable sails and ranges of 25–40+ cells. Inland peoples stay landlubbers, which gives diversity.
- **Risks.** It could make crossings common everywhere. Mitigations:
  - keep `sea_survival` risk;
  - keep decay;
  - tie gains to actually going out.
  Tune with the 4–6/10 target.
- **Saves.** No new field.
- **Measure.** Seacraft p50, p90 and max; sail-usable bands; range distribution among coastal bands.
- **Effort.** S.

### P3. Wanderlust and novelty in where to go
- **Mechanism.**
  1. Make the unknown uncertain. Unvisited cells are valued at the band's *expectation* (the mean of
     its known biome), not their true stock.
  2. Add a novelty bonus to `cell_score` for cells never visited by the band or its parent, scaled by
     curiosity and by the young-adult share. That share is approximated by recent births or
     pop/split_size.
  3. A visible `far_shore` adds an "unknown land" value that a curious, confident (seacraft)
     band will weigh against risk.
- **Target.** Crossings, the spread of lineages, early-game expansion.
- **Expected effect.** A faster, more varied early frontier; bands probing coasts and passes;
  curiosity gains a fitness role: explorers find good land, but some die. Crossings come from bold
  bands rather than from hunger.
- **Risks.**
  - Faster expansion can speed up everything, including the plateau and war.
  - Needs a cap on the bonus *size* (not on dates or counts).
  - Performance: a per-band visited set is costly. A cheaper option is a coarse lineage map or a
    "camp layer" age.
- **Saves.** If a visited map is added, a serde default (empty) plus a fixture.
- **Measure.** Frontier speed (cells per century), first-boat to first-crossing lag, curiosity of the
  frontier against the core.
- **Effort.** M.

### P4. Dissent and crowding fission ("we will found our own")
- **Mechanism.**
  - Splits are also triggered by *friction*: crowding relative to food, stress and size, plus a
    per-band "disagreement" accumulator that rises with crowd and stress and falls with surplus and
    feasts.
  - A daughter that leaves after dissent leaves *farther*. With boats and a reachable rumour, it may
    sail.
  - It takes the more curious and ambitious tail of the band: daughter curiosity and ambition are
    drawn from the upper part of the parent distribution, the parent keeps the rest.
  - Add a heritable **ambition** trait, drifting like curiosity. It raises tolerance for risk in
    voyages and raids, and it raises fission.
- **Target.** Crossings (founder voyages), diversity, dominance (fission breaks up big blocs).
- **Expected effect.** Frontiers become more curious and bolder than cores, as observed in real
  expansions. Long voyages come from land-hungry or quarrelling groups, at varied times.
- **Risks.**
  - Runaway selection for curiosity at frontiers (the existing clamp 0.3–2.5 helps).
  - More, smaller bands: performance and stronger small-band forgetting.
- **Saves.** `ambition: f32` and `dissent: f32` with serde defaults (1.0, 0.0), plus a migration and
  fixture.
- **Measure.** Split causes; curiosity and ambition at the frontier against the core; crossing count
  and timing; number of bands.
- **Effort.** M.

### P5. Marriage and mixed ancestry
- **Mechanism.**
  - When friendly or kin bands meet in `contacts` or `exchange`, a few people move each way (spouse
    exchange). That also reduces tension, because marriage alliances cut grudges.
  - Track ancestry as a small vector of founder shares, e.g. `[f16; 8]`, mixed by migrants. Captives
    and absorbed people mix their ancestry into the winner instead of being relabelled.
  - Keep `lineage` as the patrilineal or "surname" label, and report both.
- **Target.** Q2 realism, the dominance metric, conflict (alliances).
- **Expected effect.** The ancestry metric shows mixing; the top *ancestry* share after 1500 y is
  probably near-even within a landmass. The surname metric still shows founder success. Fewer fights
  between intermarried neighbours.
- **Risks.** Memory (8×2 bytes × 200 k bands is fine) and save size. The alliance effect could reduce
  war too much; tune with aggression.
- **Saves.** New field with a serde default computed from `lineage` (one-hot), plus a fixture.
- **Measure.** Ancestry diversity per landmass; marriages per contact; fights between allied bands.
- **Effort.** M.

### P6. Play and refinement: an open-ended axis
- **Mechanism.**
  - Spare labour plus curiosity is spent partly on *improving* known recipes, not only on new
    combinations. A recipe's quality rises slowly with practice and play, with diminishing returns,
    and is partly lost when a band shrinks.
  - Quality feeds existing numbers: boat_q, cut_q, protect and yields.
  - Quality diffuses partly at teaching, so better practice spreads.
- **Target.** Invention stall: progress continues as refinement, and regional styles diverge.
- **Expected effect.** Late-game tech keeps creeping up and differs by region. Boats in maritime
  cultures get better, which feeds crossings.
- **Risks.** Unbounded growth (use diminishing returns toward a physics ceiling); balance with war;
  performance (one float per Known).
- **Saves.** `quality` per Known with a serde default of the base value, plus a fixture.
- **Measure.** Mean and spread of recipe quality over time; "improvement events" per century as a
  second invention series.
- **Effort.** M.
- **Note.** For *new* items, extend the physics (forms and uses listed in §3.1). That is content work
  (L), and motives then decide who finds it.

### P7. Prestige and copying the successful
- **Mechanism.**
  - A band gains prestige from surplus, fine items (ornaments, quality from P6), feasts, gifts and
    successful voyages.
  - Prestige makes others more likely to learn from it (prestige-biased learning), to marry into it
    (P5) and to ally with it.
  - Status competition gives a reason to make ornaments and to go on long voyages for exotic goods.
- **Target.** Stall (directed effort), crossings (glory and exotic trade), dominance (alliances
  instead of conquest).
- **Expected effect.** Hubs of craft and trade emerge; long-distance trade networks form.
- **Risks.** Rich-get-richer could *increase* dominance. Measure it against P5. Tuning is complex.
- **Saves.** `prestige: f32` with a serde default, plus a fixture.
- **Measure.** Prestige Gini, trade distance, learning-source bias.
- **Effort.** M–L.

---

## 5. Build order

1. **P0: measure honestly.** Ship it, rerun 10 × 500 y on the standard map, and take that as the new
   baseline. Expect about 3/10 crossings.
2. **P1: rumours age, look-outs keep looking.** The cheapest crossing lever. Check that there are
   none before Y100, and that crossings are varied.
3. **P2: seacraft from seafaring.** Tune together with P1 toward 4–6/10.
4. **P4: dissent fission with curious and ambitious emigrants.** Early-game diversity and frontiers.
5. **P3: wanderlust and novelty in movement.** Tune after P4, because the two interact.
6. **P5: marriage and ancestry.** The metric plus alliances. It needs a save migration.
7. **P6: play and refinement** (plus, separately, a physics content pass for new items).
8. **P7: prestige.** It builds on P5 and P6.

After each step: 10 seeds × 500 y on CI, then one small long run (for example 2 seeds × 1000 y on
600×600). Track:
- crossings (boat-borne, gap from settled land, year);
- effective lineages and ancestry;
- world-firsts per century and quality steps;
- population, fights and sea deaths.

## 6. Small first step

**P0 items 1–3 and 6 (analysis only, no behaviour change):**
- a continent cutoff scaled with land area;
- the gap from settled land;
- landfalls tagged boat or foot;
- effective lineages.

Re-read the existing v8 D7 and long-run outputs or saves with it, so the baseline is honest before any
behaviour moves.

Then **P0 items 4–5** as a separate commit:
- the strait-adjacent start rule;
- the no-walking-over-water rule.

That commit is followed by one 10 × 500 y CI run.
