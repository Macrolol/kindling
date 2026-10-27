# Proposal: new inventions (after iron and sail)

Status: **accepted (Oct 26 2026), being built step by step.** Michael's answers to the open
questions are in section 6 ("Decisions"); progress per step is in the step table (section 5)
and in `docs/PROGRESS-inventions.md`. The proposal was written against `main` 2b7005c, the code
that produced `docs/results-v10-750y-800x800.txt`.

Later themes, not covered here: culture, language, politics, and the live event stream.
This proposal leaves hooks for each of them (section 4 and the "hook" notes).

Effort scale (same as earlier proposals): **S** = under a day, **M** = 1 to 3 days,
**L** = a week or more. Each estimate covers code, tests, the save migration and the CI balance
check.

---

## 1. How the technology system works today (plain version)

- **Things are a shape plus a material.** Every item is a *form* (raw, edge, point, shaft, cord,
  vessel, axe, boat, sail, plank, ornament ...) made of a *material* (wood, flint, clay, copper,
  bronze, iron, salt ...). There are 25 materials and 25 forms.
- **Nobody hands out inventions.** Bands tinker. Each season a band picks up things it already
  has and does one of three things with them: **shape** (one thing, worked with a tool),
  **combine** (two or three things fixed together) or **heat** (put in a fire or kiln). Fixed
  physics rules in `physics.rs` decide whether that makes anything. For example, flint struck
  with a hard hammer gives an edge, clay heated hot enough gives ceramic, and iron ore at
  about 1400 °C in a charcoal kiln with bellows gives a spongy bloom that still has to be
  forged. Water at hand and dry air also matter.
- **What a band tinkers with depends on its life.** Hunters notice bone, hide and flint.
  Fishers notice cord and wood. People with cold winters notice hides and pots. Ores catch the
  eye only where they crop out. Habits build up: bands that keep working clay get better at
  noticing clay. That is why worlds and regions end up with different technologies.
- **Knowing is not owning.** A recipe is known with some *skill*. It is taught at meetings,
  passed to daughter bands, and **forgotten** if it is not used. Since v9 each recipe also has a
  *quality* that improves slowly with practice.
- **Tools change life through a "kit".** Each season the band takes the best item it can make
  for each job: hunting weapon, cutting edge, digging stick, pot, axe, clothes, fire, kiln,
  cord, hammer, boat, net, trap, bow, shield, outrigger, sail, plank hull, frame, ornament,
  cooked foods. From that kit it gets plain multipliers: more game, fish or plants, better
  winter protection, slower food spoilage, boat seaworthiness, and so on.
- **The sea is its own system.** Boat range is `2 + 10·q + 12·xp·q` cells, where q is how good
  the boat is and xp is the band's seamanship. A usable sail multiplies range by 1.6, and an
  ambitious band reaches about 25% further. Bands can only aim for land they have **seen**:
  from a hilltop or from a fishing boat, at most about 13 cells out. Scouts with good boats
  sometimes sail out on a random heading. Rumours of a far shore fade after 50 years and spread
  only to neighbours whose boats could plausibly use them.
- **Where it stands now (v10 run, 800×800, 750 years, 11 surviving worlds):**
  - The tree is used up by around Y550–650. All worlds together found 87 different items, and a
    typical world finds 81.
  - Only **1 of 11** worlds reached another continent by boat. The other continents are 38 to
    171 cells away, while the best boats reach 20 to 38 cells.
  - Since v9 the binding limit has been **seeing and remembering**, not boat range. Every
    landfall by sea was a single hop of 18 cells or less.

## 2. Status of the approved plan of Oct 25

The plan Michael approved on Oct 25 is the **"technologies for activities"** proposal (v8, the
"D" steps). It is in the design-plan Page, which still labels it "Approved, being built", and in
`docs/PROGRESS-v8.md`. **All seven steps are built and released** (v0.8.0):

| step | what | commits |
|---|---|---|
| D1 | groundwork: room for more tools (7 forms, 2 materials, save v8) | 935aa5d |
| D2 | terrain and rivers (slope, river travel) | 635f603 |
| D3 | fighting tools (bow, shield, palisade) | 0f9510a, 1e51930 |
| D4 | storage, salt, dairy, manure | 917d79a, a031258 |
| D5 | seafaring (outrigger, sail, plank hull, wayfinding) | 6afafca |
| D6 | frames, ornaments, herbal medicine | 4a01c50 |
| D7 | riding | 323f194 |

Since then, v9 (motivations: honest crossing measurement, fading rumours, taught seamanship,
dissent, wanderlust, marriage, refinement, prestige) and v10 (compact saves, event log, history
store, structured run data) have also been built.

**Not built from the Oct 25 plan's tables.** These are reused below where they fit, marked
*(leftover)*:
- **Boats and water:** pitch caulking, sealed jars, rafts for heavy loads, fish weirs, bone fish
  hooks.
- **Land and farming:** pack animals, dogs, iron axe and ploughshare (farming forest and heavy
  soil), irrigation ditches, seasonal herd moves.
- **Cold, terrain and hunting:** layered furs and sewn clothing, snowshoes and sledges, pass
  lore, fire drives.
- **Buildings:** pit houses and longhouses, mud brick, fired brick and lime.
- **Smaller follow-ups:** wood used up by palisades, a walls statistic, ornaments in
  tribute, wary bands meeting less.
- **Prospecting:** ore still never draws people to go and fetch it.

## 3. The new inventions

### Ground rules this design keeps
- **No free reach.** No existing boat gets more range or safety. Long crossings have to come
  from new knowledge and new things: reading signs at sea, star and season lore, remembered
  routes, food and water that keep, and bigger caulked hulls.
- **No dates, no caps, no nudges.** Every invention needs materials that really lie in that
  band's country, knowledge it already has, and a use for it in its way of life. If there is no
  use, it gets forgotten, as now. Worlds without tin, iron, pine or a reason to count will
  differ.
- **Crossings stay hard.** Target: about **4–6 of 10** worlds reach another continent by boat,
  **none before Y100**, and never all 10.
- **The format has room.** Items are packed as `form·32 + material`, so there are **7 free
  material slots** and **7 free form slots** before anything has to be repacked. This proposal
  uses **3 materials** (pitch, steel, gold) and **3 forms** (jar, mark, ship). The rest stay free
  for culture work (pigments, dyes, instruments). Know-how that is not a thing, such as
  seamanship or herbal lore today, is a number from 0 to 1 per band (a *practice*).

### Tier 1: keeping things (needs pottery, a fire, wood)

**1.1 Pitch (tar)**: new material *(leftover: caulking)*
- **Needs:** resinous wood heated slowly in a closed pot or kiln, about 350–450 °C, in short
  supply of air. Physics rule: heat wood together with a ceramic vessel → pitch.
- **Does:**
  - caulks hulls, cutting boat upkeep labour by about 40%;
  - glues hafts (spear and axe durability a little higher);
  - seals jars (1.2).
- **Why it's realistic:** birch and pine tar is among the oldest made substances. Tarred hulls
  are universal in wooden-boat cultures.
- **Balance:** a little more spare labour for boat people. This is also the first new item
  after the plateau.

**1.2 Sealed jar**: new form *(leftover)*
- **Needs:** a fired pot plus pitch (combine). Later, clay or lime stoppers too.
- **Does:**
  - stored food spoils 20% slower, on top of salt and pits;
  - the store can be a little larger;
  - jars carry **fresh water** and preserved food on voyages (2.1);
  - a jar of grain left with water ferments into ale (4.3).
- **Why:** storage jars are central to every early storage economy and to long sea trade.
- **Balance:** bigger winter stores, which means a bit more growth and a bit more crowd disease.
  To watch.

**1.3 Drying and cold stores**: practices, no new item
- **Drying racks:** an edge plus cord in dry or windy, cold country. Fish and meat dried
  in quantity, like stockfish or jerky.
- **Cold cellars:** settled people with a digging stick where winters are well below freezing
  (ice pits, permafrost cellars).
- **Does:** fresh food spoils less in those places only. Dried food counts as **voyage
  provisions** (2.1). Fishing peoples of the cold north are then likely navigators, which gives
  geographic diversity.
- **Balance:** small; it depends on the place.

### Tier 2: long crossings (needs boats, sail, and the Tier 1 stores)

The four pieces below are four kinds of knowledge that real seafarers combined. Each one alone
does little; all together make a deliberate long crossing possible. That spread is what makes
outcomes differ between worlds.

**2.1 Provisions and endurance at sea**: replaces the yes/no "prepared" rule
- **Today:** a band counts as "prepared" if it has half a season of stores or a plank boat.
  That cuts the sea toll and nothing else.
- **Proposed:** a crossing has an **endurance**: how many open-water days the crew can last
  on the water and preserved food they carry. Preserved food is salted, smoked, dried, cheese,
  grain, jars. Past their endurance the toll rises steeply (thirst, hunger).
- **The floor is set high enough that today's short hops (≤ 18 cells) are unchanged.** Only
  long legs need jars and dried or salted food.
- **Why:** Polynesian, Norse and Austronesian voyaging was limited by water and food as much as
  by hulls.
- **Balance:** no effect on short hops. Long legs become possible only for bands with
  preservation tech. This is a *link*, not a boost.

**2.2 Pilotage (sign reading)**: new practice `pilot`
- **Grows from:** time out of sight of land (scouting, long voyages, surviving a drift). It is
  taught like seamanship and fades when unused.
- **Does:** out on the water, land can be "sighted" up to about **+10 cells** beyond eyesight
  from signs: homing birds, clouds piled over islands, swells bent by land, drifting
  vegetation. It does **not** help from the shore. Scouts with pilotage turn back toward signs
  instead of sailing a random heading to the end.
- **Why:** documented Pacific wayfinding detects islands 30–50 km beyond sight.
- **Balance:** this is the main lever on the "can't see it, can't aim for it" wall. It only acts
  where boats already go out, and it needs years of seagoing first, so it cannot happen before
  Y100.

**2.3 Sky lore (star paths and calendar)**: new practice `sky`, shared by farmers and sailors
- **Grows from:**
  - farmers timing sowing by the stars and the sun's rising points (settled, fields);
  - sailing at night (pilotage above about 0.3).
- **Does:**
  - aimed voyages to a known but unseen shore lose much less heading (error 0.15 → as low
    as about 0.06);
  - farmers get a small sowing bonus (+3–5% crop).
- **Why:** star compasses and agricultural calendars are the same skill. Coastal farmers
  becoming navigators is a well-documented path (Austronesian expansion).
- **Hook:** sky lore is a natural seed for **calendar, ritual and shared names for stars**
  (culture and language later).
- **Balance:** a small farm bonus everywhere; the sea effect only matters together with 2.1
  and 2.2.

**2.4 Remembered routes**: changes how rumours live
- **Today:** a far-shore rumour fades in 50 years.
- **Proposed:** when the band that holds the rumour has pilotage or records (3.1), the rumour
  is kept as a *route* (sung or recited sailing directions) and fades slower, up to about
  3× with good lore.
- A route can be passed to neighbours even if their own boats are a bit short, because the
  teller knows the stepping stones. The receiving band's range still decides whether anyone
  sails it.
- **Why:** oral sailing directions persisted for generations (Pacific, Norse *landnám*
  accounts).
- **Balance:** more aimed voyages over 20–40-cell gaps, but only in seafaring regions.

**2.5 Ship (caulked, nailed plank hull)**: new form *(builds on the leftover plank and caulking
items)*
- **Needs:** a plank hull, metal points used as nails or rivets (copper, bronze or iron), and
  pitch, put together at the water (combine).
- **Does:**
  - **Not** more seaworthiness: that is already pinned at 1.0, and leaving it alone keeps the
    no-free-reach rule.
  - A ship carries a bigger party and much more provisions: endurance about 1.5×.
  - With a sail, a ship holds course and goes further on a long leg: range about 1.25×.
  - Its *quality* (v9 refinement) improves endurance. That gives refinement a job again instead
    of hitting the boat_q ceiling.
- **Why:** riveted clinker hulls and sewn-plank ocean ships carried founding parties and
  provisions.
- **Balance:** with a sail and good pilotage, the best ship's realistic one-leg reach is about
  45–55 cells. That reaches the nearer unreached continents (38+ cells), or farther ones by way
  of islands, but not the 100–170-cell gaps. Those stay unreached, which is good for diversity.

### Tier 3: metal beyond wrought iron (needs bloomery iron)

**3.1 Iron in the fields**: no new item *(leftover: iron axe and ploughshare)*
- **Needs:** an iron axe or iron cutting edge in the kit.
- **Does:**
  - forest cells become worth clearing;
  - heavy wet soils become worth tilling;
  - field work goes faster.
- **Why:** iron axes and shares opened the forests of temperate Europe and India to farming.
- **Balance:** more farmland in wet and forested country, so more people and more disease.
  The disease link is already in the sim.

**3.2 Steel**: new material
- **Needs:** wrought iron kept long in a charcoal kiln with bellows (carburising), then
  **quenched in water**. That means a water ctx, a hammer, and more than one heat. The chance per
  try is low (about 0.03), so it comes late and patchily.
- **Does:** steel edges, axes, points and spears. These are existing forms, so they are new
  items without any new form. They give sharper, longer-lasting tools: lower upkeep, better
  cutting and better weapons.
- **Why:** carburised and quenched iron appears within centuries of bloomery iron, wherever
  smiths had good charcoal.
- **Balance:**
  - more lethal fights where steel is known (open question 3);
  - steel raises the refinement ceiling for edge tools (5.3).

**3.3 Native gold**: new material and one new deposit
- **Where:** placer gold in stream gravels, using the last free deposit bit. Map generator
  v5; older maps get it added on load, the same way iron ore was added in v5.
- **Needs:** nothing but finding it and a hammer. Gold is soft, is worked cold, and never
  tarnishes.
- **Does:**
  - gold ornaments: fine quality, so prestige and trade value (4.2);
  - no tool use at all.
- **Why:** worked gold is among the earliest metalwork (Varna, about 4500 BCE). It is prized
  because it is rare and lasts.
- **Hook:** gold and ornaments as **wealth and status goods** for politics later.
- **Balance:** small and local.

*Considered and left for later:* casting in moulds, lead and silver, cast iron, lime and mortar,
glaze. Each needs more geology or heat physics than it gives right now. Lime would make a good
later "buildings" step together with mud brick and walls *(leftover)*.

### Tier 4: counting and keeping track (precursors of writing)

**4.1 Tallies and tokens**: new form `Mark` plus new practice `records`
- **Needs:** something to count, and an incised mark.
  - **Physics:** cut notches into a stick or bone with an edge (a tally); press clay with a
    point (a token), which can then be fired (a ceramic token).
  - **Use:** the item counts as *used* (and so is not forgotten) only where there is something
    to keep track of: a big store, three or more trade partners, a herd, or debts of tribute.
    Elsewhere it is invented now and then and forgotten again. That follows naturally from
    today's forgetting rules.
- **Does (`records` grows with use):**
  - store losses fall a little (about 5%; better management);
  - **trade ties are remembered better**: bonds fade slower, and the number of partners a band
    can keep grows with records instead of always being 8 (open question 1);
  - recipes are forgotten somewhat slower (skill decay up to about −25%);
  - routes are kept as **charts** (4.4).
- **Why:** tally sticks, knotted cords (quipu) and Near-Eastern clay tokens are the documented
  precursors of writing, and they appear with storage and exchange economies.
- **Hook:** *the* seed for writing, numbers and administration (culture, language, politics).
  Every token style can carry its lineage, which later becomes a cultural marker.

**4.2 Feasts**: no new item, a prestige change
- **Today:** prestige from surplus is based on the *share* of spare labour, so a band of 20 and
  a band of 200 earn the same.
- **Proposed:** a band with stored surplus hosts neighbours at a feast. The prestige it earns
  depends on how much food it actually gives (roughly the square root of the amount), so larger,
  richer bands stand out. Ale (4.3) and gold or other fine ornaments make a feast count for more.
- **Why:** competitive feasting is the classic route to status in storage societies
  (Northwest Coast, Neolithic Near East).
- **Balance:** this fixes "prestige doesn't track band size" (correlation is 0.08–0.28 today).
  Prestige draws ambitious bands and copying, so watch for runaway hubs.
- **Hook:** feasts are where politics (leaders, obligations) will plug in.

**4.3 Ale**: a jar of grain, so a new item with no new form
- **Needs:** grain, a sealed jar and water, in warm enough country.
- **Does:**
  - a little extra food from grain;
  - doubles a feast's draw.
- **Why:** brewing is as old as grain storage, and safer than bad water.

**4.4 Charts**: no new item, records × pilotage
- **Needs:** records and pilotage both above about 0.4.
- **Does:**
  - routes no longer fade while the chart is used;
  - routes can be taught to partners (like stick charts or recited directions with tally
    aids).
- **Balance:** the slowest and last piece of the crossing chain. It should arrive only in a
  minority of worlds.

### 3.5 What happens to the "saturation" findings

| finding (v9/v10 analysis) | what changes it |
|---|---|
| tree used up at about Y550–650 | about 25–35 new items (pitch, jar and its contents, ship variants, steel items, gold items, marks and tokens) plus 4 practices. They come late and depend on place, so the plateau moves out and differs between worlds. |
| partners pinned at 7.6 of 8 | partner capacity grows with records (4.1) instead of being a flat 8 (open question 1) |
| prestige barely tracks band size | feasts scale with the absolute amount given (4.2) |
| recipe quality flat at about 0.33, boat_q pinned at 1.0 | ship quality improves endurance (2.5); steel raises the ceiling for edges (3.2); quality also counts for weapons (a REFACTOR-NOTES item) |
| long crossings limited by sight and memory | pilotage, sky lore, routes, charts, provisions (Tier 2) |
| disease rises with farming and never levels off | not addressed here. Wells and drains, with lime, are a candidate for the later buildings step (open question 6). |

## 4. Data and events: hooks for the future live stream

Everything goes through the existing seams: `World::push_event` for events, the `counters!`
registry for counters, `YearStats` for yearly figures, and `world.landfalls` for crossings.
So `metrics.csv`, `frames.ndjson`, `chronicle.ndjson` and a future stream all get them without
any new plumbing. Adding columns does not change the data schema (still v1). New event kinds are
appended after `Married` (35), so older ids never move. An older build shows unknown kinds as its
last kind, so the stream should carry the kind *name*, as `event_record` already does.

**New event kinds**
- **Major:**
  - `FirstShip`, `FirstSteel`, `FirstRecords` (records ≥ 0.3 anywhere), `FirstChart`, `FirstFeast`;
  - `LongestCrossing`: a sea landfall whose leg beats the world's record (recipe = leg length).
- **Minor (life ring):**
  - `SignSighted` (land found from signs, not eyes; recipe = distance);
  - `Expedition` (an aimed or scouting voyage set out; recipe = planned reach);
  - `RouteKept` and `RouteLost` (a route outlived its 50 years, or faded at last);
  - `Feast` (other = guest band, recipe = food given).
- Inventions themselves already report as `FirstInvention` / `Invention` with the item, so pitch,
  jars, steel and so on need no new kinds.

**New counters** (auto-added to `metrics.csv` and frames)
- **Sea:** `sightings_eye`, `sightings_signs`, `expeditions`, `expedition_lost`,
  `voyages_provisioned`, `endurance_deaths`, `routes_kept`, `routes_faded`.
- **Records and prestige:** `feasts`, `marks_used`.
- **Metal:** `steel_made`.

**New `YearStats` fields:** mean `pilot`, `sky` and `records` (over seafaring, farming and all
bands), bands with a ship, bands with jars, the longest leg so far, and mean endurance of
seagoing bands.

**Landfall ledger:** each `LandfallRec` gets the leg length and a small tech bit-set (sail,
ship, pilot, sky, chart, jars) as additive `serde(default)` fields. The summary can then say
*how* each crossing was made, and a stream can show it live.

**Summary and report:** a "crossings" block (worlds, legs, tech used) and a "late tech" block
(first year of each new item or practice per world, to check spread and diversity).

## 5. Build order (small steps, each checked before the next)

**Every step's check is two GitHub Actions runs** of `headless-sims.yml`:
- **(a) Early game:** 10 seeds × 500 years on the standard 256×160 map (`seeds 1..10`). Nothing
  may change before the step's new physics can appear. No crossing before Y100. Population,
  farming and disease within ±10% of the previous step unless the step means to change them.
- **(b) Late game,** for steps from 2 onward: 10 seeds × 750 years on 800×800 (seeds
  499246292..499246301, the v10 set). Crossings, item spread and first years are compared with
  `docs/results-v10-750y-800x800.txt`.

Each step also gets:
- `cargo test` (save fixtures included);
- a load test of a v10 save made before the step;
- `./build-web.sh`.

Results go to `docs/PROGRESS-inventions.md` (built and checked, like `PROGRESS-v8.md`).

| # | step | contents | pass check | effort | status |
|---|---|---|---|---|---|
| 0 | **Measure first** | new counters, YearStats fields, landfall leg and tech fields, event kinds (unused yet), report blocks. No behaviour change. Also fix a v10 fixture save. | report and save byte-identical to now apart from new columns; baselines (a)+(b) recorded | S–M | done (see PROGRESS) |
| 1 | **Pitch and sealed jars** | material Pitch, form Jar, caulking upkeep, jar spoilage. Save v11 (stock 25→26, craft leaning inserted; same pattern as v7→v8). | (a) pop within ±10%, jars appear only after pottery; no crossing change | M | done (see PROGRESS) |
| 2 | **Provisions and endurance** | continuous provisions; drying racks; cold cellars; endurance toll on long legs | (a) unchanged crossings; (b) no more than baseline yet (no reach to use it); lost-at-sea not up on short hops | M | done (see PROGRESS) |
| 3 | **Pilotage and remembered routes** | `pilot` practice, sign sighting at sea, routes fade slower | (b) more aimed voyages and sightings; crossings 1→2–3 of 10; none before Y100 | M | done; crossings stayed 1/10, geography (see PROGRESS) |
| 4 | **Sky lore** | `sky` practice from farming and night sailing; heading error; sowing bonus | (a) farm yield +≤5%; (b) crossings about 3–4 of 10 | M | done; sowing +1–1.5%, (b) crossings 0/10, geography (see PROGRESS) |
| 5 | **Ships** | form Ship, endurance and range with sail, quality → endurance | (b) **target 4–6 of 10 by boat, not 10/10, none before Y100**; refinement no longer pinned for hulls | M–L | done; (b) crossings 1/10 (target not met: range, not endurance, is the limit; see PROGRESS) |
| 5b | **Expeditions, mat sails, deep-water seamanship** (Michael approved) | provisioned ship crews on planned voyages reach as far as endurance; plaited mat sails on plank hulls and ships only; faster seacraft and pilotage on ship legs out of sight of land | (b) **target 4–6 of 10 (4 acceptable), none before Y100, not 10/10**; (a) no big rise above 6/10 | S | done: (a) 5/10, (b) 1/10 (Y673), below target; endurance-bound, see PROGRESS |
| 6 | **Iron in the fields** | iron axe and edge open forest and heavy-soil farming | (b) more farmland in wet and forest worlds; disease within reason | S–M | done: (b) fields +5%, population +8%, disease +8%, war same |
| 7 | **Steel** | material Steel, carburise and quench recipe, steel tools and weapons | (b) steel in some but not all iron worlds; fight deaths not more than about 1.3× | M | to do |
| 8 | **Tallies, tokens and records** | form Mark, `records` practice, slower forgetting, store keeping, partner capacity | (b) records only where storage or trade is heavy; partners no longer pinned; items per band not runaway | M | to do |
| 9 | **Charts** | records × pilotage keeps and teaches routes | (b) crossings still within 4–6/10 (charts must not push it to 9–10) | S | to do |
| 10 | **Feasts and ale** | absolute-surplus prestige, ale | (b) prestige ~ size correlation rises (target 0.4–0.6); no single hub takes over | M | to do |
| 11 | **Native gold** | deposit, map generator v5, gold ornaments | (a)+(b) other map layers unchanged (gold placed from its own random stream); local prestige | M | to do |

If a step overshoots the crossing target, tune that step's own strength rather than add a cap.
The rough order matters: preservation first (it is useful on land too), then the sea chain,
then metal, then counting, which ties into the sea chain through charts.

**Total:** about 3–4 weeks of work across 12 steps. Steps 0–5 (about 1.5–2 weeks) deliver the
long-crossing goal.

## 6. Risks and open questions for Michael

**Decisions (Michael, Oct 26 2026)** — these override the text above where they differ:
1. **Partner cap:** drop the fixed 8 altogether. Ties fade unless kept up by visits, trade or
   marriage; records slow the fading (step 8). Old saves' partner lists migrate and load.
2. **Crossing target:** 4–6 of 10 worlds crossing to another continent by boat, none before
   Y100, never all — judged on the 800×800, 750-year runs (check b). The 500-year standard map
   (check a) is a sanity check only.
3. **Steel:** may make fights deadlier, but only through emergent edge and armour mechanics
   (no flat multiplier). The war rate is watched.
4. **Gold:** yes, as a small optional map-generator addition (rare river gravel or quartz
   veins); a new resource with defaults so old saves load.
5. **Ale and feasts:** build now, as a small step (prestige from giving).
6. **Disease:** leave as it is. Wells and drains come later with towns and politics (noted in
   `docs/IDEAS-later.md`).
7. **Sky lore:** one shared practice that helps farming and sailing. Its sea effect is gated
   on pilot / sea practice, so sky lore alone gives no crossings.

**Open questions (as asked)**
1. **Partner cap.** Should the flat 8 trade partners become "8 plus what records let you keep
   track of", or should it be replaced by ties that simply fade unless kept up (no number at
   all)? The second is purer but harder to balance.
2. **Crossing target.** Is "4–6 of 10 worlds, none before Y100" meant for the 750-year 800×800
   runs, the 500-year standard map, or both? Today the standard map is at about 3–4/10 and the
   big map at 1/11. Tier 2 mostly helps the big map.
3. **Steel and war.** Steel makes fights deadlier. OK, or should steel stay mostly tools until
   politics exists?
4. **Gold needs a map-generator change** (new worlds only; old saves get gold added on load).
   Acceptable, or leave gold until a geology pass?
5. **Ale and feasts:** fine to include drink and feasting now, or keep it for the culture phase?
6. **Disease:** add wells and drains (with lime and brick) as a later "buildings" step, or keep
   disease as it is (it rising with crowding is realistic)?
7. **Sky lore helping farms:** OK that one practice touches both farming and sailing, or keep
   them as separate lores?

**Risks**
- **Runaway knowledge.** Records slow forgetting. If that is too strong, every band keeps
  everything and worlds converge. Step 8 checks items per band and the spread between worlds.
- **Over-reach at sea.** Pilotage, routes and charts together could make crossings routine.
  Each step is checked against the target, and the strengths are tuned per step.
- **Disease and population** grow with better storage and forest farming. They are watched in
  every check.
- **Save format.** Steps 1, 7 and 8 add materials or forms. They follow the v7→v8 pattern
  (stock padded, craft leaning inserted, migration plus binary route, fixture test). New `Band`
  fields must be appended at the end of the struct (v10 positional band rows). A new
  `FORMAT_VERSION` (11) is needed once, at step 1; later additions are additive.
- **Web build:** all changes are in `civ-core` with no new dependencies, so the wasm build should
  be unaffected. `./build-web.sh` is checked every step.
- **CI time.** The late-game check (b) is the expensive one, so it runs once per step and not for
  every tuning try. Tuning uses 3–4 seeds first.

## 7. Effort summary

| step | effort |
|---|---|
| 0 measure | S–M |
| 1 pitch and jars | M |
| 2 provisions | M |
| 3 pilotage and routes | M |
| 4 sky lore | M |
| 5 ships | M–L |
| 6 iron in fields | S–M |
| 7 steel | M |
| 8 records | M |
| 9 charts | S |
| 10 feasts and ale | M |
| 11 gold | M |

Optional later (not in this plan):
- the leftover land items: pack animals, dogs, snowshoes and sledges, pass lore, rafts, fish
  weirs, hooks;
- buildings: pit houses and longhouses, mud brick, lime, walls;
- prospecting trips to ore.
