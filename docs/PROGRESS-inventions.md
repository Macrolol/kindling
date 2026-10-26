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

## Step 0. Measure first (in progress)
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

## Step 1. Pitch and sealed jars (in progress)
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

## Step 2. Provisions and endurance (in progress)
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

