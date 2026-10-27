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

## Step 2. Provisions and endurance (done, dedfd2d)
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
- Check (b), run [40417390032](https://github.com/Macrolol/kindling/actions/runs/40417390032),
  `docs/results-inv-step2-750y-800x800.txt`: 1/10 crossed (seed 499246293, Y723, leg 20, gap 14,
  outrigger+plank+provisioned+jar; step 0 Y649), no more than the baseline, pass. Seed 499246298
  dies out early as before. War 0.296 %/y (step 0 0.280 over all 10), items 76.2 (73.3), lost at
  sea per voyage 0.109 (0.105).

## Step 3. Pilotage and remembered routes (done, c5b9207; crossing target not met, see check (b))
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
- Check (a), run [40419063313](https://github.com/Macrolol/kindling/actions/runs/40419063313),
  `docs/results-inv-step3-seeds1-10-500y.txt`: continents crossed 4/10 (Y118, Y179, Y205, Y270;
  step 2 Y118, Y179, Y205, Y276), none before Y100. Aimed voyages 30-51%, steered by signs
  8-14%; pilot p90 0.20-0.22 in the two seeds that cross most, 0.01-0.09 elsewhere. War 0.407
  %/y (0.402), items 68.1 (72.7), pop 1 035 270 (-9.6% on step 2). Watch: the drop is mostly
  seeds 3 and 4 and two seeds (9, 10) whose grain was not domesticated by Y500 (it was at
  Y363/Y395 in step 2); pilotage does not touch farming, so this reads as discovery-timing
  chance, and lost at sea is down (39 391 vs 40 411).
- Check (b), run [40419072634](https://github.com/Macrolol/kindling/actions/runs/40419072634),
  `docs/results-inv-step3-750y-800x800.txt`: 1/10 crossed (seed 499246293, Y723, as in step 2);
  the plan's 2–3 is not met. Pilotage works as meant: 41–60% of voyages aimed, 7–17% steered by
  signs, landings up 6% (370 746 vs 349 247) on slightly fewer voyages, lost at sea down
  (226 578 vs 230 181). War 0.299 %/y (0.296), items 75.1 (76.2), pop 7.31M (7.20M).
- Why no more crossings: geography. `civ-core/examples/hop_chain.rs` (new, world generation only)
  finds, for each unreached continent, the narrowest open-water hop on the best chain of
  islands. On the (b) worlds that is 47 cells (seed 297), 50 (295), 54 (301), 66 (294), 85
  (292), 97 (296), 160 (299), 171 (300); seed 293's is 14 (crossed) and 298's 13 (dies out in
  Y39). The longest leg made in any (b) run is 26–35 cells. Pilotage and sky lore only steer
  within range, so on (b) they can change when and how often, not whether; the 4–6/10 target
  rests on ships (step 5) making 47–54-cell legs. On the default map the needed hops are
  3–48 cells (seed 10's continent is 21 cells away by island chain and is not yet reached).

## Step 4. Sky lore (done)
- Band field `sky` (0..1; appended after `pilot`, older rows load as 0; no format bump): a
  calendar and star paths, one practice for farming and sailing (decision 7).
- Learned by settled farmers (0.003 x sow x settled per season, once both are above 0.3; at full
  farming it settles near 0.6 after about 50 years) and by crews out of sight of land on a leg
  survived (0.003 per open-water cell up to 25, times pilot / 0.3 up to 1, so a novice learns
  little). Fades 0.998 per season, passes to daughters and drift parties, and is taught to
  neighbours who farm or have some pilotage (pilot > 0.02), so farmers' calendars reach coastal
  seafarers.
- Farming: sowing effort (the plough factor) x (1 + 0.05 x sky), at most +5%.
- At sea it counts only as far as the crew has pilotage: `sky_at_sea` = sky x min(pilot / 0.3,
  1), and an aimed heading's error shrinks by 0.6 x that (0.15 -> 0.06 for a seasoned crew).
  Sky lore alone steers nobody and adds no range. Ledger bit `sky` when `sky_at_sea` >= 0.2.
- YearStats `sky_bands`, `sky_farm_mean`, `sky_sea_mean`, `sky_at_sea_mean`; counter
  `sky_voyages`; `summary.json` `sea.sky_*`; a "sky lore" line in the multi-seed report.
- Tests: `sky_lore_heading_and_save` (no heading change without pilotage, 0.15 -> 0.06 with it,
  more hits on an island 16 cells out with no extra range, ledger bit, learning bounds, binary
  and JSON round trip), `farmers_learn_sky_lore`; v10 fixture bands load with sky 0.
- Local look: seed 3, 300 years: no farming to speak of by Y300 and pilot p90 0.07, so no sky
  lore at all (a first version let only pilots above 0.3 learn at sea; nobody ever got there,
  so sea learning was made proportional instead). Seed 5, 400 years: sky lore from Y250 with
  farming, 7122 bands above 0.1 by Y400, farmers' mean 0.23 (+1.2% sowing effort), boat bands'
  mean 0.14 but only 0.008 usable at sea (pilotage is low on that map).
- Check (a), run [40421935783](https://github.com/Macrolol/kindling/actions/runs/40421935783),
  `docs/results-inv-step4-seeds1-10-500y.txt`: continents crossed 4/10 at the same years as step
  3 (Y118, Y179, Y205, Y270), none before Y100. Farmers' sky lore mean 0.19-0.28 where grain is
  farmed (about +1-1.4% sowing effort, inside the +5% bound); usable at sea only on the two
  seeds with real pilotage (s1 0.04, s8 0.05), steering 0.4-0.8% of voyages. War 0.402 %/y
  (0.407), items 68.0 (68.1), pop 1 149 650 (+11% on step 3, back above step 2's 1 145 k): most
  of the gain is seeds 1, 3, 4 and 9 where grain was domesticated earlier this time (seed 1's
  farmed food 6% -> 44%), which is discovery timing rather than the 1% sowing bonus.
- Check (b), run [40421945252](https://github.com/Macrolol/kindling/actions/runs/40421945252),
  `docs/results-inv-step4-750y-800x800.txt`: 0/10 crossed. Seed 499246293, which crossed at
  Y723 in steps 2 and 3 (a 14-cell hop at the very end of the run), did not get across by Y750
  this time; its longest leg was 32 cells, so the hop was in reach and this is late-run chance,
  not a lost ability. The plan's 3-4/10 is not met for the geographic reason found in step 3:
  sky lore only narrows the heading error and adds no range. Sky lore at sea stays small
  (`sky_at_sea` mean 0.01-0.06) because pilotage is low on these maps (p90 0.06-0.18). War
  0.298 %/y (0.299), items 73.9 (75.1), pop 7.34M (7.31M), farmed food share within a few
  points per seed, lost at sea 228 568 (226 578).

## Step 5. Ships (done, 7b1f3d5; crossing target not met: 1/10 in check (b), see below)
- Form `Ship` (id 26, appended; "nailed plank ship" for wood). Recipe, by the water only:
  plank boat + raw pitch + a metal point (copper, bronze or iron) -> ship of the plank's
  material, success 0.2. New kit slot `ship` and `ship_q`; daughters inherit them; ships need
  caulking upkeep like other craft.
- Effects: endurance x 1.4 x (1 + 0.2 x ship quality) (about 1.5x at quality 0.3, so 32 -> 45-48
  cells); boat range x 1.25 only with a usable sail (sail in kit, sail experience > 0.4). No
  extra seaworthiness and no bigger crews. Ledger bit `ship` (TECH_SHIP), event `FirstShip`
  (id 36, major), counter `ship_voyages`, YearStats `ship_bands`, `ship_q_mean`, summary
  `sea.ship_*` and `sea.first_ship_year`, and a "ships" line in the multi-seed report.
- Tuning (before any check): the first build made no ships on seed 5 in 500 years although
  planks (Y334), pitch (Y139) and copper/iron points (Y253-270) were all there. An experiment
  draws 3 things weighted by attention and need, so plank + pitch + metal point came up about
  4e-7 of the time. Fix, in the style of the existing need weights: a band that already has a
  plank hull and lives by the water weighs planks, pitch and metal points by 1 + 3 x seacraft
  (shipwrights tinker with ship parts), and the recipe's success went 0.06 -> 0.2. No cap,
  date or range boost.
- Tests: `ships_recipe_and_reach` (recipe and its negatives, name, form list, unchanged
  boat_q, endurance factor, range x1.25 only with a usable sail, provisions scaling, ledger bit,
  `FirstShip` once and only for ships, event round trip). 45 pass, 1 ignored; fixtures load.
- Local look, seed 5, 500 years (after tuning): first ship Y405 (pitch + copper point + plank
  boat), 3 646 ship bands by Y500 (quality mean 0.15), endurance p90 32 -> 46 cells, longest leg
  25 -> 35 cells; first crossing by ship Y440 (18 cells, to landmass #3); 1 527 ship voyages.
  Seed 5's second continent is 31 cells from settled land and was still not reached.
- Check (a), run [40424609567](https://github.com/Macrolol/kindling/actions/runs/40424609567),
  `docs/results-inv-step5-seeds1-10-500y.txt`: continents crossed 4/10 at the same years as
  steps 3 and 4 (Y118, Y179, Y205, Y270), none before Y100, pass. Ships were built in 6/10
  seeds, first at Y297-Y472 (never before a plank boat, pitch and a metal point were all there);
  first crossing to another landmass by ship Y440-Y480 in seeds 1, 2, 3, 5. Endurance p90 rose
  32 -> 45-48 cells where there are ships, but only seed 5 has a usable sail (967 sail bands,
  range p90 27.8 vs 18-21 elsewhere), so the ships' range bonus barely acts; 2.5% of voyages
  by ship (seeds 1-5). Ship quality mean 0.02-0.24, so hull refinement has room again. Pop
  1 117 446 (-2.8% on step 4), war 0.408 %/y (0.402), items 68.2 (68.0).
- Check (b), run [40424612153](https://github.com/Macrolol/kindling/actions/runs/40424612153),
  `docs/results-inv-step5-750y-800x800.txt`: **1/10 crossed** (seed 499246293, Y692, leg 22,
  gap 14, outrigger+plank+provisioned+jar: not by ship); step 4 0/10, v10 baseline 1/10. None
  before Y100, not all. The 4-6/10 target is **not met**. Ships built in 4/10 seeds (292 Y504,
  293 Y552, 294 Y432, 297 Y506), 9k-29k ship bands by Y750, quality mean 0.13-0.22. War 0.294
  %/y (0.298), items 74.4 (73.9), pop 7.22M (7.34M, -1.5%), lost at sea 225 851 (228 568).
- Why crossings still do not happen (section 5 of the results file has the per-seed table):
  - **Range, not endurance, is the wall.** Endurance p90 is 46-47 cells wherever there are ships,
    but range p90 is only 20-21 cells without a sail and 29-30 with one. A leg ends when the
    boat's range runs out, so the step 2 and step 5 endurance never binds. Range = (2 + 10 q +
    12 xp q) x 1.6 with a usable sail x 1.25 for a ship x e^(0.25 ambition). For a typical good
    band (q 0.9, seacraft 0.6, ambition 0.3) that is about 38 cells with ship and sail, 61 at
    the very most (q, seacraft and ambition all at 1). Seacraft p90 sits at 0.52-0.66 all run.
  - **Geography.** The narrowest hop to an unreached continent (step 3, `hop_chain`) is 47 (297),
    50 (295), 54 (301), 66 (294), 85 (292), 97 (296), 160 (299), 171 (300) cells; 293's is 14
    (crossed) and 298 dies out in Y39. Only 297, 295 and 301 are within ship-and-sail reach at
    all.
  - **Ship + sail come together late, and only where both exist.** The parts are not late:
    pitch Y94-Y222 and a metal point Y191-Y390 in every seed with metal. The slow gates are the
    plank boat (needs a metal axe: Y233-Y648, never in 295 and 301) and cloth sails (need linen
    or wool cloth: Y512-Y722, never in 295, 298 and 301), then the three-part combine 34-273
    years after the plank boat. Seed 297, the one in reach, had ship + sail only from Y584; its
    record leg went 26 -> 34 (Y650) -> 38.5 (Y750), still short of 47. Seed 293's record went
    27 -> 45.5 cells within 40 years of its first sail (Y709), so ship + sail do lengthen legs
    as designed.
  - **Aiming.** An unreached continent 47+ cells off is never seen and so never rumoured, and
    pilot p90 0.04-0.17 means signs reach only 0.4-1.7 cells: these legs sail a random heading.
- Candidate fixes for Michael (none implemented; all emergent, no caps, dates or extra reach for
  primitive boats):
  1. **Planned expeditions bounded by provisions.** A fully provisioned ship crew on a deliberate
     voyage sails until its food and water (endurance) run out, not the coastal day-range, so
     the step 2 and 5 work (jars, dried and salted food, ship quality) sets the reach of long
     voyages: about 45-54 cells for ships only. Within reach: 297 (47), and 295/301 only if they
     ever get ships.
  2. **Mat sails stepped on plank hulls.** A sail of woven plant-fibre matting (as Austronesian
     pandanus sails) on a plank hull or ship, so sail use no longer waits for linen or wool
     cloth (Y512-Y722 or never). Restricted to plank hulls so dugouts and canoes gain nothing.
  3. **Deep-water seamanship from long legs.** Seacraft and pilotage grow faster on legs out of
     sight of land in a ship (as step 3 does for pilotage), so ship crews push range (12 x xp x
     q) and sign radius up over generations instead of sitting at seacraft 0.55 and pilot 0.1.
  - Caveat: even with all three, 292, 296, 299 and 300 need 85-171-cell legs and 298 dies out,
    so this seed set allows at most about 4-5 of 10 (293, 297, 295, 301, maybe 294); 295 and 301
    also need metal axes for plank boats first. Reaching 4-6/10 may mean accepting the top of that
    or reviewing the (b) seed set and map generator with Michael.
- Checkpoint: steps 0-5 milestone reached; step 6 not started (waiting for Michael).
- Michael's answer (Oct 27): all three fixes approved as step 5b; then step 6 onward.

## Step 5b: expeditions, mat sails, deep-water seamanship (done; (b) below target)

- Added (all emergent; nothing for dugouts, canoes or plank boats without a ship except mat sails
  on plank hulls):
  1. Planned expeditions: on a planned move or daughter voyage, a ship crew with stores for all
     (store >= half the people) and food that keeps (preserved >= 0.8) sails as far as its
     endurance (about 36 cells with skins, 45-54 with jars) when that is beyond its usual reach.
     Scouting, flight and drift keep the usual reach. Counter `expeditions`, ledger bit
     `expedition`.
  2. Mat sails: plank boat or ship + shaft + woven basket by the water makes a "woven mat sail"
     (p 0.05); usable only with a plank hull or ship (`sail_usable`), kept after a cloth sail.
     YearStats `mat_sail_bands`, summary `first_mat_sail_year`.
  3. Deep-water seamanship: ship legs >= 6 cells teach seacraft 0.004 per cell past 6 and
     pilotage at twice the per-cell rate over the whole leg (no 25-cell limit). Counter
     `deep_water_legs`.
- Tests: `expeditions_mat_sails_and_deep_water` (recipe, kit preference, no use on a dugout,
  same range as linen on a plank hull, expedition reach only for stocked ship crews, ship legs
  out of sight of land teach more). All 46 civ-core tests pass. No save format change.
- Local smoke (seeds 1-2, 500y): mat sails Y419/Y471; 4 expeditions, 152 deep-water ship legs.
- Check (a), run 40478614457 (commit d74ae59), docs/results-inv-step5b-seeds1-10-500y.txt:
  crossings 5/10 (Y118, Y179, Y205, Y270, Y476), none before Y100 (step 5: 4/10). The new one is
  seed 5 at Y476 on an expedition ship (44-cell leg). War 0.401%/y, items 68.4, population
  1 177 335. Expeditions 15, deep-water ship legs 539, mat sails in 8/10 seeds (Y289-Y484).
- Check (b), run 40478624024, docs/results-inv-step5b-750y-800x800.txt: crossings **1/10**
  (499246293 at Y673, plank boat + sail, not a ship; step 5 had it at Y692), none before Y100.
  War 0.297%/y, items 75.2, population 7 053 596, disease 0.170%/y. Ships in 7 seeds, 417
  expeditions, 57 296 deep-water ship legs, seacraft p90 0.53-0.66, pilot p90 0.09-0.18.
- Reading: no overshoot (no pre-Y100 crossing, not 7+/10, (a) only +1), so no tuning under the
  agreed rule. But (b) is below the 4-6/10 target. The binding limit is endurance: longest legs
  top out at 44-46 cells, right at the endurance p90 of 46-47 (jars, ship x1.4 x (1+0.2q), ship
  quality only 0.13-0.19), while the narrowest hops are 47 (297), 50 (295), 54 (301), 66 (294),
  85-171 (292, 296, 299, 300). 295 and 301 never get plank boats or ships. Expeditions now sail
  as far as their stores go, so more crossings need longer endurance (e.g. better ship quality
  or new stores) or a different seed set / map - a design question for Michael.

## Step 6: iron in the fields (done)

- Added: iron axes and edges open forest and heavy wet ground for farming (PROPOSAL 3.1).
  - New saved layer `opened` (0..1 per cell, defaults empty; no format bump) and derived
    `open_pot` (forest penalty mostly lifted; up to +25% on heavy wet soil).
  - Bands whose toolkit makes an iron axe or edge farm against the opened potential, open
    0.06 x sow x weather of the gap a season, and plough 1.15x. Everyone else benefits from
    ground already opened. Opened ground fades slowly where fields are abandoned.
  - Iron is checked in the toolkit, not the kit slot (bronze cut 0.65 beats iron 0.64, so the
    kit keeps bronze axes).
  - YearStats `iron_farm_bands`, `opened_fields`, `forest_fields`, `wet_fields`; counter
    `iron_clearing`; farm summary fields and an "iron in the fields" headless line.
- Test `iron_opens_forest_and_heavy_ground` (potentials, iron vs bronze, crop cap, plough,
  clearing via fx, fade, save round trip, old save with no layer). All civ-core tests pass.
- Local smoke (seeds 1-2, 500y): iron reached ever in both but no band holds iron at Y500
  (stage now copper), so no iron farming yet; fields 5480/3649, forest 2149/1347, wet 599/384
  as baseline. Effects should show in the 750y (b) runs.
- Check (a), run 40479800575 (commit 7b3f2c2), docs/results-inv-step6-seeds1-10-500y.txt:
  crossings 5/10 (Y118, Y179, Y205, Y270, Y466), none before Y100. Iron farming only in seed 5
  (from Y245) and seed 7 (Y404) within 500 years, so little change: war 0.400%/y, disease
  0.122%/y, items 68.4, population 1 159 092 (5b: 1 177 335, seed 5 diverging), fields 33 377.
- Check (b), run 40479809575, docs/results-inv-step6-750y-800x800.txt: iron farming in 8 of 9
  surviving worlds (Y280-Y583). Field cells 215 990 vs 204 858 in 5b (+5%), 35 019 opened by
  iron, 67 834 in forest and 19 958 on wet ground. Farmers +16%, population 7 648 642 (+8%).
  Disease 0.183%/y (5b 0.170, +8%), war 0.297%/y (same), items 74.9. Crossings 1/10
  (499246293 at Y670), none before Y100.
- Reading: the step does what PROPOSAL 3.1 asked - more farmland in forest and wet worlds with
  disease within reason - so no tuning.

## Decision (Oct 27): crossing rate accepted

- Michael accepts the crossing rate after steps 5b-6: about 1/10 on the 800x800 750-year runs
  (b) and 5/10 on the standard map (a). The 4-6/10 target for (b) is dropped.
- From step 7 on, crossings are a guard rail only: stop and report if any world crosses before
  Y100, if (b) reaches 7+/10, or if (a) rises above 6/10. No more crossing tuning (this also
  applies to step 9, charts).

## Step 7: steel (done)

- Material `Steel` (26, save format v12, migration `v11_to_v12`, fixture
  `v11_seed9_y160_64x40.civsave` from the step 6 build). Recipe: Heat of wrought iron with
  bellows in a closed charcoal kiln, by water for the quench, p 0.03 a try (`STEEL_P`). Shaped
  with a hammerstone like iron into knives, points, spears and axes; traded and carried like
  iron; steel tools count as iron in the fields. Props hard 6, tough 9, sharp 9, malleable 5.5
  (cut 0.78 vs iron 0.64, bronze 0.65), so the kit takes steel over bronze and iron.
- Deadlier only through the edge (Michael's decision): no new multiplier. Fight losses already
  scale with `0.6 + weapon_q`; a steel spear and axe add about 0.07 to `weapon_q`, i.e. about
  +5% to losses inflicted. Armour (`loss_guard`: bows, shields) unchanged; no metal armour yet.
- New stage `steel`. YearStats `steel_bands`, `weapon_q_mean`; counter `steel_made`; summary
  `metal.*` (first steel year, steel bands, peak, forging seasons, weapon quality, deaths per
  fight); a "steel" headless line. The world first is the usual `FirstInvention` of the steel
  bar (no separate `FirstSteel` kind, which would duplicate it).
- Tests `steel_recipe_kit_and_weapons`, `v11_fixture_gains_steel`; `v10_fixture_loads_and_runs`
  run-on hash updated (one more material); `v9_fixture_loads_exactly` skips the new leaning. All
  civ-core tests pass (48, 1 ignored).
- Local smoke (seeds 1-2, 500y): neither world holds iron at Y500, so no steel; nothing broken.
- Check (a), run [40486753199](https://github.com/Macrolol/kindling/actions/runs/40486753199)
  (`docs/results-inv-step7-seeds1-10-500y.txt`): crossings 4/10 (first Y135), war 0.400%/yr
  (step 6: 0.400), disease 0.129 (0.122), items 70.5 (68.4), population 1,171,697 (1,159,092).
  Iron in 5 worlds, steel in 2 (seed 4 Y397, seed 7 Y376). Killed per fight 3.01 (2.99).
- Check (b), run [40486756302](https://github.com/Macrolol/kindling/actions/runs/40486756302)
  (`docs/results-inv-step7-750y-800x800.txt`): crossings 1/10 (Y622), war 0.303%/yr (0.297),
  disease 0.175 (0.183), items 77.3 (74.9), population 8,107,740 (7,648,642). Steel made in 7
  of the 9 worlds that ever reached iron (Y295-Y731), held at the end in 6. Killed per fight,
  living worlds: 2.84 (2.67), x1.06; largest per-seed rise x1.12. Targets met; no tuning.

## Step 8: tallies, tokens and records (in progress)

- Form `Mark` (id 27, no format bump): tally (good edge + wooden stick or bone), clay token
  (point pressed into soft clay), fired clay token (fired like a pot). Kit slot `mark`.
- Band `records` (0..1, saved after `sky`): grows while a band with a mark has something to
  count (store of 2 seasons' need, 4 ties of 0.5+, a herd, or tribute owed: new saved field
  `owed`, set to 1 on both sides by tribute); fades 3% a season. Effects at full records: 5%
  less spoilage, 25% slower forgetting of unused recipes, trade ties fade 0.9 instead of 0.8
  a year and 1.5x as many are kept up. Taught to bands with something to count.
- Partner cap removed (Michael's decision). First smoke with plain fading: partners ran away
  (mean 105, max 281 per band; 4x slower; items and learning way up). Tie fade now depends on how
  thinly a band's care is spread: `keep^((partners/span)^2)` past `TIE_SPAN` = 2 partners
  (records widen it). No fixed cap. Records first made too easy (100% of bands); tightened to
  the thresholds above and a faster fade.
- Event `FirstRecords` (major), counter `marks_used`, YearStats `mark_bands`, `records_bands`,
  `records_mean`, `partners_mean`, `partners_max`; summary `records.*`; headless "records" line.
- Tests `tallies_and_tokens`, `records_need_something_to_count` (incl. no cap: 24 partners),
  `records_fade_event_and_save` (incl. v11 fixture loads records 0); v10 run-on hash updated.
  All civ-core tests pass (51, 1 ignored).
- Local smoke (seeds 1-2, 500y), against step 7's smoke: population 243k (253k), war 0.409%/yr
  (0.423), disease 0.094 (0.121), items per band 33-38 (33-37), run time about the same.
  Records in 54-62% of bands at Y500, 88-91% of them with something to count now; first
  records Y99-Y106. Partners per band mean 22 (max 61); before, about 7.7 against the cap of 8.
