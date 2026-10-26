# Kindling inventions (after iron and sail): progress log

Started 26 Oct 2026. Plan: `docs/PROPOSAL-inventions.md` (steps 0–11, with Michael's decisions in
section 6). Rules: one local sim at a time, RAYON_NUM_THREADS=4, every run under `timeout`;
batches run on GitHub Actions (`headless-sims.yml`, archive=true, archive_saves=false).

Checks:
- **(a)** 10 seeds × 500 years, default 256×160 map (seeds 1..10); sanity check, baseline
  `docs/results-v5-seeds1-10-500y.txt` era / latest v10 code.
- **(b)** 10 seeds × 750 years, 800×800 (seeds 499246292..499246301); the crossing target
  (4–6 of 10 by boat, none before Y100, not all) is judged here. Baseline
  `docs/results-v10-750y-800x800.txt` (1/10 crossed: seed 293 at Y649, gap 14).

## Step 0. Measure first (done, fcaeb50)
- New: crossing ledger. Each crossing to another landmass records its open-water leg (cells)
  and the sea tech used (`TECH_*` bits in `civ_core::world::voyaging`: outrigger, sail, plank,
  provisioned, …; later bits reserved for jars, ships, pilotage, sky lore, routes; `adrift`).
- YearStats: `boat_bands`, `sail_bands`, `seacraft_mean`, `seacraft_p90`, `rumour_share`,
  `leg_record`. Counters: `crossing_cells`, `voyages_provisioned`, `longest_leg` (max).
  Event `LongestCrossing` (id 35). `summary.crossings[]`, `summary.sea.longest_leg/mean_leg/
  voyages_provisioned`, continents' `leg`/`tech`. A "crossing ledger" line in the multi-seed text
  report. All documented in `docs/DATA_SCHEMA.md`.
- New fixture `v10_seed9_y150_64x40.civsave` (written by the v10 build before this work) and
  test `v10_fixture_loads_and_runs`; test `longest_crossing_sets_the_record`.
- Deviation from the plan: the event kinds and counters for later features (jars, pilotage, …)
  are added in their own steps, not reserved now.
- Local check: seed 3, 300 years: the text report is byte-identical to the v10 build and
  `metrics.csv` is identical on all shared columns (behaviour unchanged); the v10 fixture
  loads and continues identically.
- Check (a), run 40415208414 (`docs/results-inv-step0-seeds1-10-500y.txt`): pop 1191364, war
  0.363%/y, items 69.3, continent crossed 4/10 (Y142, Y172, Y191, Y234). Baseline for step 1+.
- Check (b), run 40415219305 (`docs/results-inv-step0-750y-800x800.txt`): 1/10 crossed (seed
  499246293, Y649, leg 20, gap 14), identical to the v10 baseline; seed 499246298 dies out in
  Y39 as before. War 0.311%/y over the 9 live seeds, items 81.4 (live seeds).

## Step 1. Pitch and sealed jars (done, 4b9e8b7)
- New material `Pitch` (25: seal 9, no fuel value, never carried, made where it is used) and
  form `Jar` (25). Recipes (physics): heat wood + a fired pot over any fire (>= 350 C) gives
  pitch (p 0.2); combine a fired pot + pitch gives a sealed jar (p 0.25). Unfired or woven pots
  do neither.
- Effects (economy, constants in `world.rs`): pitch is boiled up only when there is a hull by
  the water to caulk or a jar to seal (its upkeep is then paid). While it is about, boats,
  outriggers, sails and planks cost 0.6× upkeep (`PITCH_CAULK`) and hafted weapons and axes
  0.9× (`PITCH_GLUE`). Jars: stored food spoils 0.8× (`JAR_SPOIL`) and the store holds 0.2 more
  per person (`JAR_STORE`).
- Save format v11 (`v10_to_v11`; binary v10 decoded directly and padded by `fit_vectors`).
- YearStats `pitch_bands`, `jar_bands`; crossing ledger `jar` bit.
- Tests: `pitch_and_sealed_jars`, `jars_only_after_pottery`; fixture tests updated (v10
  fixture's 10-year checksum is this build's own: the new material shifts the craft-leaning
  normalisation, as in v8).
- Local look, seed 3, 300 years, default map: pitch Y76, sealed jar Y124 (pottery first);
  by Y200 every band with pitch has jars (2091 bands).
- Check (a), run [40416321312](https://github.com/Macrolol/kindling/actions/runs/40416321312),
  `docs/results-inv-step1-seeds1-10-500y.txt`: total pop 1 163 523 (-2.3% on step 0), pass;
  pitch Y60–Y210 and jars 34–109 years after it in every seed, never before pottery, pass;
  continents crossed 4/10 (Y128, Y160 adrift, Y190, Y274; step 0 Y142, Y172, Y191, Y234), none
  before Y100, pass. Mean items 69.2 (69.3).
- Watch: deaths in fights up in all 10 seeds, 0.363 -> 0.409 %/y (+13%). Raids are lured by the
  other band's store, and jars mean more stored food; emergent, not tuned.

## Step 2. Provisions and endurance (built, dedfd2d; check (b) pending)
- Provisions are continuous: `fed` = store / (half a season's food per person), 0..1 (a plank
  boat always counts as fully fed). The sea toll's provisions factor is 1 - 0.3 x fed, so the
  old all-or-nothing rule is its two ends.
- Endurance (open-water cells before food and water run out, `World::provisions` in
  `world/voyaging.rs`): 18 for everyone (`ENDURANCE_BASE`; every leg the old boats made
  routinely is unchanged), plus up to 10 for food that keeps (salted, dried, grain 1.0;
  jars, cheese 0.8; smoked 0.6) times `fed`, but only as far as the water lasts (skins +4,
  sealed jars +10), plus 4 for a plank boat. Past it survival falls as exp(-0.03 x over^2)
  (8 cells over: 15%).
- Drying racks (an edge and cord, where the air is dry, rain < 0.35, or the winter freezes)
  cut fresh-food spoilage by 25% and count as food that keeps. Cold cellars (settled, a
  digging stick, winters below -5 C) cut it by 30% at home (they do not travel).
- Daughters founded over the sea take the parent's jars, edge, cord, pots and smoking with them.
- Scouting parties and fishing drifts are unchanged.
- YearStats `dried_bands`, `cellar_bands`, `endurance_p50`, `endurance_p90`; counter
  `past_endurance`. No save-format change.
- Tests: `provisions_and_endurance`; the v10 fixture's 10-year checksum updated.
- Local look, seed 3, 300 years: endurance p90 22 (racks + skins), 2282 bands drying food by
  Y300, no cellars on the default map; 28 of 11 755 voyages went past endurance; pop 106k.
  Pitch came late in this run (Y213), so no jars yet: discovery timing is chaotic.
- Check (a), run [40417386615](https://github.com/Macrolol/kindling/actions/runs/40417386615),
  `docs/results-inv-step2-seeds1-10-500y.txt`: continents crossed 4/10 (Y118, Y179, Y205, Y276;
  step 1 Y128, Y160, Y190, Y274), none before Y100, pass. Pop 1 144 582 (-1.6% on step 1), war
  0.402 %/y (0.409), items 72.7 (69.2). Lost at sea per voyage 0.151 (0.148), so short hops are
  not hurt; 833 of 267 882 voyages (0.3%) went past endurance. Endurance p90 32 cells in most
  seeds by Y500.
- Check (b), run [40417390032](https://github.com/Macrolol/kindling/actions/runs/40417390032):
  running (dispatched 19:06 EDT).

## Step 3. Pilotage and remembered routes (built, c5b9207; checks running)
- Band field `pilot` (0..1; appended after `carry`, so older band rows load as 0; no format
  bump). Learned out of sight of land: a planned leg of 6+ open-water cells survived (0.003 per
  cell, up to 25), a scouting party home (0.03), a drift party home or ashore (0.03), and
  fishing grounds beyond 6 cells (0.002 x fish share per season). Fades 0.998 per season (as
  seacraft), passes to daughters, is taught to coastal boat-owning neighbours like seacraft.
- Signs at sea: out on the water a crew with pilotage reads land within 10 x pilot cells (a
  map-derived nearest-land field, cached, not saved) and, once per trip, turns toward it if it is
  another landmass, with its usual heading error. Range is unchanged: signs only redirect.
- Routes: word of a far shore lasts 50 x (1 + 2 x pilot) years instead of 50, and a pilot tells
  it to neighbours up to 1.5x as far off (the hearer's own reach x (1 + 0.5 x teller's pilot)).
- Ledger bits `pilot` and `route` now set; counters `aimed_voyages`, `sign_voyages`; YearStats
  `pilot_bands`, `pilot_mean`, `pilot_p90`; a "pilotage" line in the multi-seed report.
- Tests: `pilotage_signs_and_routes` (signs field, pilots find an island 15 cells out far more
  often with no extra range, learning/fading bounds, rumour keep, save round trip); v10
  fixture's 10-year checksum updated, and its bands load with pilot 0.
- Local look, seed 3, 300 years (one continent, so no crossing test): 11 966 voyages, 56% aimed,
  8.7% steered by signs; 381 bands with pilot > 0.1, mean 0.02 / p90 0.07 over boat bands;
  lost at sea 1357 (step 2 build: 1472); pop 94k. A first version without fishing-ground
  learning and with a 43-year fade left pilotage at p90 0.02 (14 sign voyages): too rare to
  matter, so learning on the fishing grounds was added and the fade matched to seacraft's.
- Check (a), run [40419063313](https://github.com/Macrolol/kindling/actions/runs/40419063313), and
  check (b), run [40419072634](https://github.com/Macrolol/kindling/actions/runs/40419072634):
  running (dispatched 19:20 EDT).
