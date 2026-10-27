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

## Step 8: tallies, tokens and records (done; guard rails fine, side effects of no cap noted)

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
- Check (a), run [40490126875](https://github.com/Macrolol/kindling/actions/runs/40490126875)
  (`docs/results-inv-step8-seeds1-10-500y.txt`): crossings 4/10 (Y118, Y127, Y177, Y303; none
  before Y100), war 0.387%/yr (step 7: 0.400), disease 0.113 (0.129), items 69.7 (70.5), items per
  band 35.3 (35.8), population 1,111,541 (1,171,697, -5%). Records in 23-92% of bands at Y500,
  76-99% of them with something to count. Partners per band mean 18.7-24.7, max 49-66 (step 7:
  7.8 of 8). Sim time 674 s (694 s).
- Check (b), run [40490129626](https://github.com/Macrolol/kindling/actions/runs/40490129626)
  (`docs/results-inv-step8-750y-800x800.txt`): crossings 1/10 (499246293 at Y636; step 7 Y622),
  war 0.299%/yr (0.303), disease 0.174 (0.175), items 75.9 (77.3), items per band 35.2 (36.5),
  population 7,327,512 (8,107,740, -10%; step 6 7,648,642). Killed per fight 2.63 (2.73). Records
  in 64-91% of bands at Y750, 92-97% of them with something to count. Partners per band mean
  14.0-24.4, max 52-71 (step 7: 7.7 of 8).
- Guard rails: no crossing before Y100, (b) 1/10, (a) 4/10, war not up, no world collapses
  (499246298 dies out, as in steps 6 and 7). So no tuning.
- What removing the cap does at scale (both checks agree):
  - Partners are no longer pinned (about 2.7x as many, wide spread), and items per band are
    not runaway (slightly down). Run time: (a) the same; (b) +15% summed (6726 -> 7745 s), the
    slowest seed 1249 -> 1781 s (499246297; 499246296 949 -> 1650 s) while 292, 299 and 300 ran
    faster, so the cost is uneven but bounded.
  - Ties are many but weak: with care spread over ~20 partners few reach the 0.3 strength that
    counts as "close", so food sharing to the hungry fell by about 90% (b: 4.05M -> 0.45M;
    a: 304k -> 28k). Trades -27% in both checks; mean trade distance 2.4-2.6 -> 1.5-2.0
    cells in (b) (2.1 -> 1.4 in (a)); metal carried from source shorter.
  - Learning slightly down: recipes learned -7% (a) / -11% (b), forgotten -9% / -12%.
  - Knock-on: metal and farming slower. (b) iron traded in 6 worlds (9), steel made in only 2
    (7: 499246296, 499246297); (a) wrought iron in 1 world (5), steel in none (2); farmers 59%
    of people in (b) (67%) and 47% in (a) (62%). This is the main price of the change and the
    likely cause of the population drop.
  - For Michael (not done, no guard rail broken): if the weaker trade and slower metallurgy are
    unwanted, the levers are the spread penalty (`TIE_SPAN` 2, the squared exponent) or the
    0.3 tie strength that counts as close for food sharing (kin of 0.2+ already count).
- Checkpoint: step 8 done; step 9 (charts) not started.

## Step 8b: close ties (done; guard rails fine)

Fix for step 8's side effects (weak ties, food sharing -90%, less metal trade). Real people keep
a few close ties and many loose ones, so a band's yearly tie upkeep now goes mostly to the ties
that already mean most.

- Design (`Band::fade_ties`, yearly in `year_stats`): a care budget `TIE_CARE` x (1 +
  `RECORDS_SPAN` 0.5 x records) is shared out by each tie's weight `w` = trade strength +
  marriage tie + kinship with that partner (a parallel kinship pass). A tie gets
  `care x w / sum(w^2)` of full upkeep (at most 1); its strength is multiplied by `LOOSE_KEEP`
  + (`full` - `LOOSE_KEEP`) x that share, `full` = 0.8 + 0.1 x records (as in step 8). So strong
  ties reinforce themselves, loose ones fade towards `LOOSE_KEEP` a year but survive a couple of
  years, and a band with few ties keeps them all up. No cap and no set number of close ties;
  records widen the budget. Replaces step 8's `keep^(spread^2)` rule (`TIE_SPAN` removed).
  `CLOSE_TIE` 0.3 (unchanged value) is now a named constant.
- New YearStats `close_ties_mean`, `close_ties_max` (ties of 0.3+ per band after the fade), in
  metrics.csv, summary `records.close_ties_*` and the headless records line (`k<mean>/<max>`).
  No save format bump (trailing serde-default fields; DATA_SCHEMA.md updated).
- Test `close_ties_get_the_care` (few ties kept at x0.8; 2 strong among 30 loose: strong kept,
  loose fade but stay at least `LOOSE_KEEP`, all 32 kept; kin and marriage raise care; records
  widen it; YearStats and summary match a recount). v10 run-on hash updated. All civ-core
  tests pass (52, 1 ignored; 53 with `--features parallel`).
- Local smoke (seeds 1-2, 500y), first attempt `TIE_CARE` 2, `LOOSE_KEEP` 0.4, against the
  same seeds in step 7 / step 8 check (a):

  | | step 7 | step 8 | 8b first (2 / 0.4) | 8b tuned (4 / 0.3) |
  |---|---|---|---|---|
  | food shared s1 / s2 | 20,566 / 49,945 | 1,929 / 2,207 | 7,530 / 13,270 | 10,478 / 20,406 |
  | trades s1 / s2 (M) | 14.2 / 13.6 | 11.1 / 8.6 | 13.5 / 12.2 | 12.9 / 12.4 |
  | mean trade distance | 1.94 / 2.13 | 1.31 / 1.44 | 1.74 / 1.90 | 1.75 / 1.89 |
  | close ties per band mean/max | - | - | 1.1/5, 1.1/5 | 2.1/8, 2.3/7 |
  | partners per band mean/max | 7.7 of 8 | 21.8/61, 22.5/62 | 29.2/74, 30.8/64 | 29.6/68, 30.5/67 |
  | population s1 / s2 | 116,466 / 136,564 | 118,285 / 136,213 | 121,878 / 123,053 | 116,282 / 119,870 |
  | run time s1 / s2 (local) | 58.6 / 54.3 s | 67.9 / 54.9 s | 82.4 / 62.0 s | 77.5 / 68.8 s |

  (Step 7/8 columns from the GitHub check (a) summaries for seeds 1-2; local smoke population
  for those versions was 253k / 243k summed, so seed-level population is noisy.)
- Tuning (the one allowed round): the first attempt left only about one close tie per band and
  food sharing at about a quarter to a third of step 7, so it clearly missed. Raised `TIE_CARE`
  2 -> 4 (a band keeps up about four ties in full, the size of a "support clique" of close
  ties) and lowered `LOOSE_KEEP` 0.4 -> 0.3 so the larger budget does not keep extra loose ties
  alive. Close ties doubled (about 2.2 per band, max 8), food sharing up a further 40-50%
  (now about half of step 7), trades and trade distance back near step 7; partners unchanged.
- GitHub checks (commit bb2595f): (a) run 40501565886
  (https://github.com/Macrolol/kindling/actions/runs/40501565886,
  docs/results-inv-step8b-seeds1-10-500y.txt), (b) run 40501575101
  (https://github.com/Macrolol/kindling/actions/runs/40501575101,
  docs/results-inv-step8b-750y-800x800.txt). Totals, step 7 / step 8 / step 8b:

  | | (a) s7 | (a) s8 | (a) s8b | (b) s7 | (b) s8 | (b) s8b |
  |---|---|---|---|---|---|---|
  | food shared | 303,586 | 27,600 | 159,134 | 4,052,823 | 454,207 | 2,071,843 |
  | trades | 141.2M | 103.3M | 132.5M | 1200.5M | 879.8M | 1095.5M |
  | mean trade distance | 2.12 | 1.43 | 1.98 | 2.21 | 1.63 | 2.13 |
  | worlds with iron / steel | 5 / 2 | 3 / 0 | 7 / 0 | 9 / 7 | 9 / 2 | 9 / 4 |
  | bands farming with iron (end) | 3,896 | 5,336 | 5,949 | 136,703 | 56,422 | 68,115 |
  | farmer share of population | 0.62 | 0.47 | 0.52 | 0.67 | 0.59 | 0.62 |
  | population | 1,171,697 | 1,111,541 | 1,147,334 | 8,107,740 | 7,327,512 | 7,348,876 |
  | partners per band mean / max | (capped 8) | 21.5 / 66 | 30.8 / 74 | (capped 8) | 18.1 / 71 | 26.8 / 85 |
  | close ties per band mean / max | - | - | 2.18 / 8 | - | - | 2.07 / 10 |
  | run time sum (max seed) | 693 s (155) | 674 s (153) | 938 s (247) | 6726 s (1249) | 7745 s (1781) | 8616 s (1764) |
  | war deaths % | 0.400 | 0.387 | 0.374 | 0.303 | 0.299 | 0.290 |
  | continent crossings | 4/10 | 4/10 | 4/10 | 1/10 | 1/10 | 1/10 |
  | first crossings | Y135-274 | Y118-303 | Y106-255 | Y622 | Y636 | Y634 |

- Reading: food sharing is back to about half of step 7 (5-6x step 8), trades to 90-94% of
  step 7 and trade distance nearly back. Steel recovers on the big map (4 worlds, step 7 had 7,
  step 8 2) but not on the small one (0, step 7 had 2); iron is at or above step 7. Farmer
  share and population sit between step 8 and step 7. Partners stay uncapped and varied
  (more than step 8, since strong ties now last), with about two close ties per band. Run time
  is up about 40% on (a) and 11% on (b) against step 8 (more live partnerships to visit; the
  kinship pass is parallel). Guard rails fine: (a) 4/10, (b) 1/10, none before Y100; war a
  little lower.

## Step 9: charts (done; guard rails fine)

Plan 4.4: no new item; records x pilotage keeps and teaches routes.

- Design: a band keeps charts (`World::charts`) when records and pilotage are both at least
  `CHART_MIN` 0.4 and it has a boat (the chart is kept while it is used). Its far-shore route
  no longer fades: each sea season the route is renewed instead of being forgotten after
  `rumour_keep` years. When it passes word of the far shore to a partner or neighbour
  (`exchange`), the reach is x`CHART_TELL` 1.5 and the chance `CHART_TEACH_P` 0.6 (plain word
  0.3), and the taught route arrives fresh. Pilotage (p90 0.02-0.19 in step 8) and records
  (mean 0.1-0.5) are rarely both 0.4+, so charts should be a minority thing.
- New: event `FirstChart` (kind 38, major); counters `routes_kept`, `routes_faded`,
  `routes_taught`; YearStats `chart_bands`, `charted_routes`; summary `charts.*`; a "charts"
  line in the headless report. No kept/faded route events in the chronicle (counters only, to
  keep it quiet). No save format bump; DATA_SCHEMA.md updated.
- Test `charts_keep_and_teach_routes`: a charted band keeps a 1500-year-old route, the same band
  without records forgets it, no boat means no chart; teaching trials (near: charts taught
  more than 1.5x as often as plain word; far: only a chart carries); `routes_taught` counts;
  `FirstChart` fires once; YearStats and summary match. All civ-core tests pass (53, 1 ignored).
- Local smoke (seeds 1-2, 500y): s1 first chart Y414, peak 2 chart bands, 7 routes taught;
  s2 no charts (its run is identical to step 8b: pop 119,870). s1 pop 103,489 (8b 116,282),
  which is run-to-run divergence after Y414 rather than a real effect of two chart bands.
  Crossings unchanged (2/2, first Y106 / Y187, none before Y100). Run time 77.5 / 68.7 s.
- GitHub checks dispatched (commit 0a96dd3): (a) run 40504455280, (b) run 40504465166.
- Check (a), run [40504455280](https://github.com/Macrolol/kindling/actions/runs/40504455280)
  (`docs/results-inv-step9-seeds1-10-500y.txt`): charts in 2/10 worlds (seed 1 Y414, peak 2
  bands, gone by Y492; seed 5 Y466, 5 bands at Y500 with 4 routes on charts), routes taught
  from a chart 15, kept by a chart 0, faded 232,548. Eight seeds are identical to step 8b (no
  chart ever, same numbers). Crossings 4/10 (Y106, Y176, Y187, Y255; same as 8b, none before
  Y100), war 0.373%/yr (8b 0.374), disease 0.119 (0.119), items 73.8 (73.6), population
  1,136,998 (1,147,334, -0.9%: seed 1 diverges after its first chart, 116k -> 103k, as in the
  local smoke; seed 5 +1%). Sim time 914 s (938 s).
- Check (b), run [40504465166](https://github.com/Macrolol/kindling/actions/runs/40504465166)
  (`docs/results-inv-step9-750y-800x800.txt`): charts in 8/10 worlds (all but the dead
  499246298 and 499246299), first Y447-Y706 (447, 459, 506, 569, 635, 657, 703, 706); peak
  chart bands 1-10 per world, 15 at Y750 summed; routes taught from a chart 343 (499246297:
  170, 499246294: 85), kept by a chart 1, faded 2,655,926. Crossings 1/10 (499246293 at Y637;
  8b Y634), war 0.289%/yr (0.290), disease 0.171 (0.171), items 77.6 (78.3), population
  7,370,782 (7,348,876, +0.3%), steel worlds 3 (4: 499246293 diverges after its chart at Y569
  and makes no steel). Sim time 7599 s (8616 s; slowest seed 1627 s, 1764 s; runner noise,
  charts add no real work).
- Totals, step 8b -> step 9:

  | | (a) s8b | (a) s9 | (b) s8b | (b) s9 |
  |---|---|---|---|---|
  | worlds with charts | - | 2/10 | - | 8/10 |
  | first chart year | - | Y414, Y466 | - | Y447-Y706 |
  | chart bands peak (max world) | - | 5 | - | 10 |
  | routes kept / taught (chart) | - | 0 / 15 | - | 1 / 343 |
  | routes faded | (not counted) | 232,548 | (not counted) | 2,655,926 |
  | continent crossings | 4/10 | 4/10 | 1/10 | 1/10 |
  | first crossings | Y106-255 | Y106-255 | Y634 | Y637 |
  | war deaths % | 0.374 | 0.373 | 0.290 | 0.289 |
  | disease % | 0.119 | 0.119 | 0.171 | 0.171 |
  | items (mean, end) | 73.6 | 73.8 | 78.3 | 77.6 |
  | population | 1,147,334 | 1,136,998 | 7,348,876 | 7,370,782 |
  | run time sum (max seed) | 938 s (247) | 914 s (246) | 8616 s (1764) | 7599 s (1627) |

- Reading: charts are the minority thing the design aimed for: rare and late on the small map,
  common but small on the big one (a handful of bands per world, first in the last 300
  years). They teach routes (hundreds on (b)) but almost never have to keep one: a band with
  pilotage 0.4+ already remembers a route for 90+ years (`rumour_keep`), and chart bands
  mostly last a few decades, so `routes_kept` stays at 0-1. Crossings do not move (they come
  before charts or, on (b), only once), so the over-reach worry in the plan did not happen.
  Everything else is within run-to-run divergence of step 8b.
- Guard rails: no crossing before Y100, (b) 1/10, (a) 4/10, war not up, no population
  collapse (499246298 dies out at Y31, as in every step). No tuning.
- Checkpoint: step 9 done; step 10 (feasts and ale) not started.

## Step 10: feasts and ale (done; prestige ~ size goal missed, guard rails fine)

- Plan (approved Oct 27: ale and feasts now, a small step, prestige in proportion to giving):
  a band with food stored beyond what it needs hosts a neighbour at a feast and gives part of
  that surplus; prestige earned ~ sqrt(food given), x2 when it pours ale. This replaces the old
  share-of-spare-labour prestige term. Ale = a sealed jar + grain at the water (item
  jar-of-grain, no new form or material); brewed only by grain farmers where the warm season is
  warm enough; a little more food from grain and a bigger feast draw. Events FirstFeast (major)
  and Feast (minor, history store only); counters; YearStats incl. prestige ~ size correlation
  and the top band's share of prestige. No caps, set dates or boosts.
- Code (civ-core `world/feasting.rs`; band.rs `Kit::ale`; physics ale recipe; materials name
  "ale"; world.rs events/counters/YearStats/meeting hook; fx `Op::Feast`; analysis
  `FeastsSummary`; headless "feasts" line; DATA_SCHEMA section). Prestige: spare-labour term and
  flat gift removed; giving (feasts and food to the hungry) earns 0.004 x sqrt(food) (x2 ale).
- Tests: `ale_is_brewed_from_grain_in_a_jar_at_the_water`,
  `feasts_give_food_and_earn_standing_by_the_gift` (transfer, sqrt standing, ale x2, guest cap,
  no feast when lean, FirstFeast once, YearStats, summary). `prestige_biases_...` now runs 600
  seasons (no standing before the first food stores). v10 fixture run-on hash updated. All 55
  pass, also with `--features parallel`.
- Local smoke, first attempt (keep 1 season, give 25% of the surplus, prestige 0.004 x
  sqrt(food)): s1 first feast Y90, 173,500 feasts, ale Y477 (2,080 bands, 2,135 ale feasts);
  s2 first feast Y116, 212,395 feasts, no ale. Prestige ~ size r only 0.01 / 0.13 (last-100y
  mean), top band 0.2-0.3%, Gini 0.70 / 0.53. A clear miss: only ~0.6% of bands had a store
  past a full season, so feasts were too rare to shape standing.
- Diagnosis (seed 1, 256x160, probes): most fed bands hold 0.25-0.5 season in store; absolute
  store tracks band size (r 0.3-0.7) but store/need does not. Keeping back 0.1 season raised r
  (0.43 / 0.31 / 0.23 for Y100-200 / -300 / -400) but halved the v5-beta fixture's population
  in 20 years (hosts gave away their lean-season store); 0.25 cost a fifth. Passing prestige
  to daughter bands in proportion to the people who leave did not help (tried, dropped).
- Tuning round (the one allowed): `FEAST_KEEP` 0.5 season, `FEAST_SHARE` 0.25, `FEAST_MIN` 0.5,
  prestige in proportion to the food given (`PRESTIGE_GIVE` 0.01 x food, x2 with ale), the
  same for food to the hungry. Probe seed 1: r 0.39 (Y100-200), 0.21, 0.14; top band <1%;
  Gini ~0.48. Tests updated (linear standing); v10 run-on hash updated again.
- Local smoke, tuned (8c1f050, seeds 1-2, 500y, 256x160): first feast Y44 / Y62; 1,850,353 /
  1,760,697 feasts (10x the first attempt), no ale yet in 500 y on either seed. Prestige ~ size r
  0.16 / 0.30 at Y500 (last-100y mean 0.14 / 0.26); top band 0.2% / 0.1% (max from Y100 2.6% /
  6.8%, no hub); Gini 0.53 / 0.43. Crossings Y126 / Y188 (step 9: Y106 / Y187), none before
  Y100. Pop 120,751 / 123,856 (step 9: 103,489 / 119,870). Killed 0.412 / 0.467 %/yr (step 9
  0.370 / 0.452); disease 0.084 / 0.109 %/yr. Guard rails fine; r below the 0.4 goal on 256x160.
- GitHub checks dispatched (commit e248dc8): (a) run 40514735908, (b) run 40514750187.
- Check (a), run [40514735908](https://github.com/Macrolol/kindling/actions/runs/40514735908)
  (`docs/results-inv-step10-seeds1-10-500y.txt`, commit e248dc8): feasts in 10/10 worlds, first
  Y22-Y62; 16,420,842 feasts giving 27.5M person-seasons of food; food to the hungry 272,859
  (step 9 158,354). Ale in 2/10 worlds (seed 5 Y436, seed 3 Y456; peak 15,817 / 5,664 bands;
  1,126,881 ale feasts). Prestige ~ size r 0.19 (last-100y mean; per seed 0.07-0.34; end 0.21);
  by century it is 0.35-0.5 in the first two centuries and falls to 0.1-0.3 from Y200 on (section
  8). Prestige Gini 0.37 -> 0.53, top-10% share 26% -> 37%; top band 0.24% of all prestige at the
  end, most from Y100 18.1% (seed 7 at Y100 with 58 bands, under 8% by Y113). Crossings 4/10 (Y126,
  Y178, Y188, Y246; step 9 Y106, Y176, Y187, Y255), none before Y100. War 0.414%/yr (0.373, +11%),
  disease 0.130 (0.119), items 74.8 (73.8), population 1,150,261 (1,136,998, +1.2%; seed 10
  56k -> 89k, seed 9 84k -> 65k). Sim time 903 s (914 s).
- Check (b), run [40514750187](https://github.com/Macrolol/kindling/actions/runs/40514750187)
  (`docs/results-inv-step10-750y-800x800.txt`): feasts in all 9 living worlds, first Y28-Y66;
  118,837,820 feasts giving 221.0M person-seasons; food to the hungry 3,079,803 (2,059,908). Ale
  in 7/10 worlds, first Y369-Y668 (369, 380, 450, 538, 594, 658, 668; never in 295, 301 and the
  dead 298); peak ale bands 10,728-64,494 per world; 40.5M ale feasts. **Prestige ~ size r 0.25**
  (last-100y mean over the 9 living worlds; per seed 0.09-0.37; end 0.26): below the 0.4-0.6
  goal. Prestige Gini 0.44 -> 0.63 (living worlds), top-10% share 32% -> 48%; top band 0.05% at
  the end, most from Y100 in a world of 100+ bands 8.5% (499246299 at Y107); no hub. (499246301's
  "max 100%" is the world's 2-3 bands at Y66-Y134.) Crossings 1/10 (499246293 at Y663; step 9
  Y637), none before Y100. War 0.305%/yr (0.289, +6%), disease 0.185 (0.171, +8%), items 78.8
  (77.6), population 7,536,822 (7,370,782, +2.3%), steel worlds 2 (3). Sim time 9943 s (7599 s,
  +31%; slowest seed 2695 s, 1627 s): the biggest rises are the two worlds with the most ale
  feasts (499246297 1523 -> 2695 s, 499246299 449 -> 819 s), the rest within runner noise.
- Totals, step 9 -> step 10 (prestige figures over living worlds; step 9 did not measure prestige
  ~ size, the v9 analysis found 0.08-0.28 per seed):

  | | (a) s9 | (a) s10 | (b) s9 | (b) s10 |
  |---|---|---|---|---|
  | prestige ~ size r (last 100 y) | - | 0.19 (0.07-0.34) | - | 0.25 (0.09-0.37) |
  | prestige Gini (end) | 0.37 | 0.53 | 0.44 | 0.63 |
  | top-10% share of prestige | 26% | 37% | 32% | 48% |
  | top band share, end (most from Y100) | - | 0.24% (18.1%) | - | 0.05% (8.5%) |
  | worlds with feasts / first | - | 10/10 / Y22-62 | - | 9/9 / Y28-66 |
  | feasts | - | 16.4M | - | 118.8M |
  | food given at feasts (person-seasons) | - | 27.5M | - | 221.0M |
  | food to the hungry | 158,354 | 272,859 | 2,059,908 | 3,079,803 |
  | ale worlds / first | - | 2/10 / Y436, Y456 | - | 7/10 / Y369-668 |
  | ale bands peak (max world) / ale feasts | - | 15,817 / 1.1M | - | 64,494 / 40.5M |
  | continent crossings | 4/10 | 4/10 | 1/10 | 1/10 |
  | first crossings | Y106-255 | Y126-246 | Y637 | Y663 |
  | war deaths % | 0.373 | 0.414 | 0.289 | 0.305 |
  | disease % | 0.119 | 0.130 | 0.171 | 0.185 |
  | items (mean, end) | 73.8 | 74.8 | 77.6 | 78.8 |
  | population | 1,136,998 | 1,150,261 | 7,370,782 | 7,536,822 |
  | run time sum (max seed) | 914 s (246) | 903 s (265) | 7599 s (1627) | 9943 s (2695) |

- Reading: feasting is everywhere and early (every world by Y66), ale comes late and only in warm
  grain country (7 of 9 living worlds on (b), 2 of 10 on (a)), and it is a big flow: food given at
  feasts is about 90x the food given to the hungry. Standing is more unequal than in step 9 (Gini
  +0.16 to +0.19) but spread over many bands: no hub (top band well under 1% from Y200 on).
  Food to the hungry rose by half. Population, items, crossings, disease and war are near step 9
  (war +6% to +11%, within the guard rail).
- **Goal missed: prestige ~ size r 0.25 on (b), 0.19 on (a), against 0.4-0.6.** Diagnosis (from
  the by-century figures and the code; not probed further, as no more tuning was allowed):
  - r is in or near the goal (0.35-0.5) while worlds have tens to hundreds of bands, in the first
    one or two centuries, and falls to 0.1-0.3 from about Y200 on, as bands number in the
    thousands, and stays there.
  - The fall comes as bone beads spread (first Y44-Y202). Ornament standing
    (`PRESTIGE_FINE` 0.01 x (0.5 + fine) a season, fading x0.99) settles at 0.5-1.5 per
    ornament band whatever its size, about the size of all the standing (mean 0.58 on (a), 1.00
    on (b)), so it adds size-free noise to every band.
  - A gift is capped at one season of the guest's need (`FEAST_EAT`), so a big host gives no
    more to a small neighbour than a small host would; and standing fades in ~17 years while
    bands keep splitting, so a band's size and what it gave lately drift apart.
- Options (for Michael; not done):
  1. Accept the miss and keep step 10 as built: giving now drives standing, no hub, guard rails
     fine; but r 0.19-0.25 is inside the pre-feast range (v9: 0.08-0.28), so little gained there.
  2. One more, separately approved tuning round aimed at the cause: scale ornament standing with
     the band's size and lift the guest cap so a big
     host can feast more than one neighbour's worth; then rerun (a) and (b).
- Guard rails: no crossing before Y100, (b) 1/10, (a) 4/10, war +6% / +11% (not doubling), no
  population collapse (499246298 dies out at Y31, as in every step). No further tuning.
- Checkpoint: step 10 done (goal missed, reported); step 11 (native gold) not started.

## Step 10b: feasts and ornaments scale with band size (in progress)

- Plan (approved Oct 27, one more tuning round for step 10): (1) a feast is as big as its host:
  the flat cap of one season of the guest's need becomes one season of food for a crowd the
  size of the host or the guest, whichever is bigger (`feast_cap`), so a big band with surplus
  gives more; standing stays in proportion to the food given. (2) Ornament standing grows with
  how many people see it: x sqrt(pop / 25) (`ornament_prestige`; a 25-person band earns what
  every band did before). Feast tallies go to the history store once a year
  (`Archive::note_many`) instead of an effect per feast (cheaper). No caps or boosts.
- Tests: `feasts_give_food_and_earn_standing_by_the_gift` extended (big host gives up to its own
  need, `feast_cap`, sqrt ornament standing, no feast effect after the first, yearly Feast tally).
  v10 fixture run-on hash updated (the v10 save still loads with the same load hash). All 56
  pass, also with `--features parallel`. DATA_SCHEMA section added (no format bump).
- Probes (seed 1, 256x160, 400 y; r by 50-year mark Y200/250/300/350/400 or by century):
  - 10b as built: C3-C5 r 0.29 / 0.14 / 0.15 (step 10 0.21 / 0.14 / 0.14); seed 2 0.25 / 0.27 /
    0.31 (0.28 / 0.17 / 0.26). Little change.
  - Per-size breakdown (debug, seed 1): bands are 0-50 people, mostly 10-30; mean prestige rises
    only 0.42 -> 0.79 from size 5 to 40. Small bands hold more store per person (store/need ~0.5
    vs 0.17-0.3) and far more often have a surplus (16-50% vs 2-8% for 30-50 people): stores are
    per head and self-limited by the effort target, a growing band eats into its store, and a
    split leaves remnant and daughter with stores but few mouths. So the feast flow per head
    falls with size, and the bigger cap rarely binds. Ornaments are held by 12-40% of bands in
    every size bin (binary, size-free noise).
  - Ornament standing ~ size^1 instead of sqrt: 0.39 / 0.23 / 0.18 / 0.05 / 0.09 (no help).
  - Daughter takes standing in proportion to the people who leave: 0.14 / 0.08 / 0.13 / 0.14 /
    -0.05 (worse). Both together: worse.
  - Labour-backed feast (host adds food its spare labour could fetch, 0.5 or 0.2 x spare x
    need): C3/C4 r 0.31 / 0.30 and 0.33 / 0.17, but it is free food per meeting (farming bands
    1,315 vs 21 at Y400 at 0.5): not realistic as is, dropped.
  - None reaches 0.4 past Y200; kept the plain design (1) + (2).
- Local smoke (seeds 1-2, 500 y, 256x160, against the tuned step 10 smoke): pop 267,064
  (244,607); war 0.425 %/yr (0.439); disease 0.130 (0.097); crossings Y162 / Y183 (Y126 / Y188),
  none before Y100; prestige ~ size r last-100y 0.15 / 0.31, mean 0.23 (0.20); Gini (100y) 0.53
  (0.48); top band max from Y100 2.6% / 6.8% (same); 3.77M feasts (3.61M). Run time 189 s
  (153 s; local, one sim at a time).
- GitHub checks on commit 2fd2579, dispatched 11:08 EDT Oct 27:
  (a) run 40528043525 https://github.com/Macrolol/kindling/actions/runs/40528043525 (done,
  `docs/results-inv-step10b-seeds1-10-500y.txt`); (b) run 40528062469
  https://github.com/Macrolol/kindling/actions/runs/40528062469 (running).
- (a) seeds 1-10, 500 y, against step 10: pop 1,184,877 (1,150,261); crossings 4/10, first
  Y162 / Y183 / Y183 / Y248, none before Y100 (4/10, Y126 first); war 0.420 %/yr (0.414);
  disease 0.138 (0.130); prestige ~ size r last-100y mean 0.261 (0.186), range 0.11-0.41; Gini
  (100y) 0.479 (0.496); top band max from Y100 18.1% (same: seed 7 at Y121, a world of ~100
  bands); run 796 s total (903 s).

## Step 11: native gold (in progress)

- Plan (approved Oct 27): gold as a rare optional map resource; old saves load, old maps get no
  gold unless regenerated; gold from its own random stream so every other map layer stays
  byte-identical.
- Built: map generator 5 (`D_GOLD` = 128): a few quartz veins in the hills (max(2, land/2500)
  veins of 1-4 cells, 600-3000 m) and placer gold in river gravels below them (chance 0.35 a
  river cell, up to 40 steps downstream), from `seed ^ GOLD_SALT`. Material `Gold` (27, save
  format v13, migration `v12_to_v13`) with the new prop `prized` (1.0; 0 elsewhere): soft
  (hard 2.5), no tools; a hammerstone beats a nugget into a gold ornament ("gold ring", p 0.4).
  The kit wears gold before any other ornament; standing `ornament_prestige(fine + prized,
  pop)` (2-3x a plain ornament), the wearer's own (gold is not traded). Rare in wants, wears
  like metal, strong attention once seen. Old maps keep generator 4 (no gold) and their binary
  saves decode against the generator-4 map (`Map::generate_layers(.., false)`).
- Stats: YearStats `gold_bands`, `gold_held`, `gold_prestige_x`; summary `gold.*`; a "gold"
  headless line. DATA_SCHEMA and SAVE_FORMAT (v13) sections.
- Tests (all 59 pass, also with `--features parallel`): `gold_leaves_every_other_map_layer_identical`
  (gold bit masked, generator-5 maps hash exactly as the generator-4 maps recorded with the step
  10b build, for 256x160 seeds 1 and 7 and 200x120 seed 499246292; the no-gold map is the
  generator-4 map; gold is rare, on rivers or hills only), `gold_ornaments_are_prized`,
  `v12_fixture_gains_gold_vectors_but_no_gold` (new fixture `v12_seed9_y170_64x40.civsave`
  from the step 10b build; JSON route, re-save, no gold appears). The v1 fixture test now
  expects generator 4 after the upgrade (no gold on load). v10 fixture run-on hash updated (one
  more material; load hash unchanged).
- Local smoke (seeds 1-2, 500 y, 256x160, against the 10b smoke): gold cells 44 / 36 (0.1% of
  the map); first gold ring Y186 / Y193; bands wearing gold peak 292 / 223 (of ~7,000; 62 / 156
  at Y500), holding gold 284 / 363; gold wearers' prestige 1.28x / 2.18x the mean (last 100
  years). Pop 257,796 (267,064); war 0.369 %/yr (0.425); disease 0.122 (0.130); crossings
  Y132, Y180 / Y212, Y320 (Y162 / Y183), none before Y100; prestige ~ size r last-100y 0.15 /
  0.27 (0.15 / 0.31); Gini (100y) 0.585 (0.533) and top 10% share 50% (40%): gold adds to the
  spread of standing, but no hub (top band max from Y100 4.1% / 8.1%, both at Y100 before any
  gold; 0.25% on average over the last 100 years). Run 165 s (189 s).
- GitHub checks on commit 7032723, dispatched 11:28 EDT Oct 27: (a) run 40531284922
  https://github.com/Macrolol/kindling/actions/runs/40531284922, (b) run 40531289571
  https://github.com/Macrolol/kindling/actions/runs/40531289571 (running).
