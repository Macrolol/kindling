# Review: is the architecture still the right path?

Proposal only. No code in this change. Read on this branch, which is main at `3b2a515`. The open drafts were read on their own worktrees and not edited: beliefs (PR #12, `kindling-step4`), the steel-axe note (PR #11, `kindling-axe`), replay (PR #7, `kindling-replay`), the step 5 plan (PR #8, `kindling-step5plan`), and the two perf notes (PR #9 `kindling-opt`, PR #10 `kindling-opt2`).

## 1. Summary

No rework. The shape of the sim is still the right one, and the last few years of features have been hanging off it, not fighting it.

Keep a deterministic seasonal step, bands as the people (not a person record each), materials with physical properties, recipes found by trying those properties, and a save that old files still load. That is the model Michael asked for, and the code still is that model. An entity-component rewrite, or a struct-of-arrays rewrite of every band, would spend months, move every world, and would not fix the two real problems.

Those problems are smaller, and they are real.

**Wear is computed in too many places, and the formula is the wrong physics.** Durability is `(toughness + 0.5 × malleability) / 12`, then clamped to 1. Copper, bronze and iron all sit on that cap. Steel, less malleable, comes out *less* durable (0.979 against 1.0). For a working edge, being easier to hammer does not make the edge last longer. This is part of why a steel axe loses to a copper one (PR #11). Fix it as one function that every scorer calls. Do that before step 5, in its own change, not buried inside beliefs or risk.

**"Is this tool better?" is asked five ways.** The kit choice, the economy, the fight formula, the diffusion probe, and (on the step 4 branch) the belief table each combine sharpness, wear and labour with different weights. The skills plan is right to keep **two** of those: what the tool really does, and what the band believes. Do not collapse those into one number. Do stop the probe and the belief prior from inventing a third set of weights.

**Step 4 is slower, and the slowdown is not a layout problem.** On the ten-seed checks the worlds are about the same size and the runs are about 60% slower (256×160, 500 years) and about 28% slower (800×800, 750 years). The single-thread bench, seed 5, 300 years, went from 85.4 s to 118.5 s. That one is partly a bigger world (+38% people, +21% bands) and still about +14% per band. The hot spots already measured on step 3 — rebuilding a toolkit from scratch, walking partner lines that cannot be travelled, a harvest that has to stay serial — are still the hot spots. Land the bit-identical patches from the perf notes before or alongside step 5. Do not rewrite storage to chase them.

**Headless running plus a recording is worth doing, and it is not a sim speedup.** Almost none of a step is drawing. The win is running a big world on a native multi-thread machine and letting the web page watch a picture, instead of running that world single-threaded in the browser. Keep interactive play on the desktop. Keep a small-map sim in the web build. PR #7 is the design for the recording. It does not have to land before step 5.

### Order

1. Durability formula, one function, its own change. Rebase PR #12 onto it if #12 has not merged, so belief priors call the new function. `FIXTURE_V10_HASH.0` stays; the run-on hash (`.1`) moves, and the usual guard rails are the check.
2. The bit-identical perf patches from #9 and #10 that do not touch scoring: skip a partner line that cannot be travelled, extend a toolkit when a band only learned a recipe, count items in a flat array at year end. Hashes do not move.
3. Step 5 (PR #8) on top of beliefs. Point the probe at the same two numbers the sim uses (true effect, belief). Do not add another yardstick. Step 5's "+5% versus step 4" budget hides the 28–60% already paid; say so in the progress note.
4. Replay / remote viewing (PR #7) when you want to watch runs. Not a gate.

Do not start an ECS, a struct-of-arrays rewrite of bands, a project whose only job is to split `world.rs`, or a move of every balance constant into the saved `Params`.

## 2. The architecture as it is

### Crates

The workspace (`Cargo.toml`) has two members. The web build is not a third crate.

| Piece | Path | What it is |
|---|---|---|
| `civ-core` | `civ-core/` | The sim. Native, headless and web all call this. No window, no clock in the result. |
| `kindling` (native) | `native/` | Desktop app, headless runner, and the wasm target. Macroquad + egui. |
| web shell | `web/index.html` (11 lines), `web/civ_io.js` (26 lines) | Page that loads `kindling.wasm`. Save download/upload. Not a sim. |
| tools | `tools/*.py` | CI result tables. Not in the sim. |
| tests | `civ-core/src/tests*.rs`, `civ-core/tests/fixtures/` | Unit tests and old saves (v1–v13 and v15; no v14 or v16 file). Format 16 is the current writer. |

`native/Cargo.toml` is the cross-platform split. Off wasm, `civ-core` is built with the `parallel` feature (rayon) and mimalloc. On wasm, `civ-core` is built **without** `parallel` and without mimalloc (`native/Cargo.toml` lines 18–24). One step function, two schedulers.

`civ-core/src/lib.rs` lines 1–2 say the step has no threads and no clocks. That comment is a bit behind the code. With the `parallel` feature, rayon runs inside `World::step`. A profiler clock (`Tick`, `world.rs` around 4911) runs only when `KINDLING_PROF` is set. Neither is allowed to change the world. The rule that still holds: same seed, same result at any thread count.

### Modules, and how big they are

Lines are `wc -l` on this branch.

| File | Lines | Job |
|---|---:|---|
| `civ-core/src/world.rs` | 4972 | The season. Bands' year, meetings, movement, demography, year stats, the event log's kinds. |
| `civ-core/src/effects.rs` | 1371 | What a tool does to food and labour, and the kit choice that reads it. |
| `civ-core/src/knowledge.rs` | 1077 | Recipes, the per-band known list, the toolkit (what can be made right now). |
| `civ-core/src/probe_diffusion.rs` | 994 | Test-only. The diffusion probe. Not in a release build (`lib.rs` line 18, `#[cfg(test)]`). |
| `civ-core/src/map.rs` | 969 | Terrain, climate, deposits. One struct of arrays, not a cell struct. |
| `civ-core/src/analysis.rs` | 1187 | End-of-run summary. Not on the step. |
| `civ-core/src/codec.rs` | 805 | Binary columns for bands, the map, events. |
| `civ-core/src/band.rs` | 759 | `Band`, `Kit`, `Derived`, weapon quality. |
| `civ-core/src/physics.rs` | 647 | `apply`: shape / combine / heat → an item, from properties only. |
| `civ-core/src/materials.rs` | 577 | Materials, forms, and the quality formulas. |
| `civ-core/src/save.rs` | 579 | Envelope, version, the migration list. |
| `civ-core/src/archive.rs` | 324 | Tiered history, three event rings. |
| `civ-core/src/params.rs` | 175 | `Config` (fixed) and `Params` (live, saved). |
| `civ-core/src/data.rs` | 100 | `metrics.csv`, event records. Schema version 1. |
| `civ-core/src/rng.rs` | 85 | SplitMix64, and the stateless map hash. |
| `civ-core/src/world/farming.rs` | 728 | Fields, herds, crop potential. |
| `civ-core/src/world/voyaging.rs` | 447 | Crossing-ledger bit flags and sea helpers. The voyage itself is still in `world.rs`. |
| `civ-core/src/world/conflict.rs` | 380 | Confront, fight, flight. |
| `civ-core/src/world/fx.rs` | 331 | Per-band effect buffer, applied later in band order. `par_map`. |
| `civ-core/src/world/feasting.rs` | 160 | Feasts, ale. |
| `civ-core/src/world/cell_first.rs` | 150 | Sparse "first tick this cell could make this item". |
| `civ-core/src/world/terrain.rs` | 133 | Step cost, rivers. |
| `native/src/ui.rs` | 823 | egui. Reads the world, does not step it. |
| `native/src/headless.rs` | 730 | `--headless`. Steps, then writes the report. |
| `native/src/main.rs` | 502 | Frame loop. Steps between draws. |
| `native/src/render.rs` | 414 | Map texture and band discs. |
| `native/src/cli.rs` | 187 | Args, thread pool. |
| `native/src/saveio.rs` | 96 | Files. Wasm uses download/upload. |

Tests, not the sim: `tests.rs` 3026, `tests_skills.rs` 1945, `tests_diag.rs` 539, `tests_diffusion.rs` 273.

On the step 4 branch, not on main: `beliefs.rs` is 623 lines, `effects.rs` grows to 1673, `world.rs` to 5291. The new module is the right place. The cost is what it does per band, not where the file sits.

`world.rs` is large, but it is not a junk drawer. The anchors:

| Lines (approx.) | What |
|---|---|
| 1–306 | Constants and `EventKind` |
| 307–716 | `Event`, `YearStats`, the `counters!` registry |
| 718–916 | `World` |
| 917–1288 | `new`, `after_load`, recipe cleanup on old saves |
| 1290–1452 | `step`, `bands_season` |
| 1454–1850 | Harvest, regrow, toolkit refresh |
| 1852–2076 | `economy` |
| 2077–2430 | The rest of one band's season |
| 2450–2687 | Experiments |
| 2688–3344 | Movement and the sea |
| 3346–3952 | Meetings, trade, marriage |
| 3953–4198 | Demography (splits, deaths) |
| 4199–4972 | Knowledge tracking, year stats, archive, `state_hash` |

Farming, fights, feast rules and the effect buffer already live under `world/`. What is still in the big file is the season loop and the social core (meetings, trade, movement). `docs/REFACTOR-NOTES.md` already lists "split `sea_season`" and "a table of meeting effects" as skipped, on purpose, so a balance change would not be mixed with a move.

### The season

One tick is one season. Four seasons make a year (`world.rs` `step`, line 1290).

```text
regrow wild stocks and fields
decay trade_flow (×0.85) and conflict_flow (×0.93)     // overlay trails
build the occupancy grid

bands, three phases:
  1. in parallel: refit each band's toolkit
  2. in band order, serial: plan the economy and take the harvest
  3. in parallel: farming, experiments, sea, forgetting, movement
     each band writes an Fx buffer; buffers apply in band order

contacts (neighbours, meetings, partner visits)
demography (births, deaths, splits)
landfall / knowledge tracking
if tick % 4 == 0: year stats and the archive
```

Phase 2 is serial on purpose. Planning every band against the start-of-season stock was tried. It left stocks fuller, with about +23% people and +40% war deaths (perf notes). Do not parallelize it.

Phase 3 does not share one random stream across threads. The world stream draws one key, then each band builds its own stream from its id and the tick (`world.rs` around 1380):

```rust
let key = rng.next_u64();
let mut r = Rng::new(key ^ (b.id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (tick << 40));
```

Results come back in band order either way (`fx.rs` `par_map`, line 275). Below 32 bands, or on wasm (no rayon), it is an ordinary loop. The test `same_result_on_any_thread_count` (`tests.rs` line 344) locks this.

### Map and cells

There is no `Cell` struct. `Map` (`map.rs` line 94) is a struct of arrays: height, temperature, rain, flow, biome, deposits, and a carrying capacity per resource, each a `Vec` of one value per cell. Landmass ids are derived on load and not saved. The generator is versioned (`GEN_VERSION` 5). Gold uses its own salt (`GOLD_SALT`, `map.rs` line 146) so adding gold did not reshuffle older layers. A saved map is the map that was generated, not "seed it again"; legacy maps stay legacy.

Renewable stocks (plants, game, fish, small game) and the farming layers (tend, dom, fertility, crop, opened) are more arrays on `World`, one float per cell (`world.rs` from line 726).

### Bands, and why there are no people

`Band` (`band.rs` line 47) is the agent. `pop` is a `u32` count. Births and deaths move the count. Marriage mixes ancestry shares (`mix_ancestry`), it does not move person records. There is no `Person` struct. Skills, recipes, beliefs and craft leanings are per band.

That is the right grain. A late 800×800 world is about 1.3 million people and about 72,000 bands. A person record each would be a different program: saves, determinism and the meeting loop would all change, and nothing in the skills plan needs it. User skill is already "how this band handles an axe", not "how Ash handles an axe". The skills proposal chose that, and the struct matches it.

A band carries a lot of small lists: known recipes, stock and origin per material, partners, craft leanings, grudges (at most 6), ancestry (at most 8), and, since format 16, per-form user skill plus an empty belief list waiting for step 4 (`band.rs` lines 26–43 and 167–197). Daughters copy those lists, discounted.

`Derived` (`band.rs` line 468) is rebuilt every season and is not what the save stores as the live world. It holds the toolkit, the kit, last season's harvest, food, stress, demand heard from partners, and the two-entry toolkit cache. It is large. See section 4.

### Items, materials, properties

`materials.rs` lines 1–4 state the rule, and it is the right rule: every material has physical properties, and every useful quality of an artefact is computed from those properties. Nothing in that file says what a band can do. `physics::apply` (`physics.rs` line 77) says what happens when shape, combine or heat is tried. A recipe is an (action, inputs) pair a band has seen work (`knowledge.rs` `Recipe`, line 58).

29 materials (`NMAT`), 13 of them gatherable, the rest made (charcoal, ceramic, metals, pitch, steel, salt). 32 forms (`NFORM`). An item id is `form * 32 + mat`, a `u16` (`Item`). `ItemSet` is a bitset, 16 × `u64`, 128 bytes, never saved, so it can grow.

`Props` (`materials.rs` line 104) is the chemistry: hardness, toughness, fracture, sharpness, workability, plasticity, fibre, malleability, fuel temperature, food, reduction and firing temperatures, melt, refractory, float, seal, and later prized / woody / resin. New materials are appended so old indexes stay put.

The quality functions at the bottom of `materials.rs` (from line 530) are where the rule starts to fray. They are the right *place*. They are not all the right *formula*, and they are not the only place a formula lives. Section 3 counts the copies. The two that matter for the audit:

```rust
// materials.rs:532
pub fn cut_q(m: Mat) -> f32 {
    let p = m.props();
    ((0.6 * p.sharp + 0.4 * p.hard) / 10.0).clamp(0.0, 1.0)
}

// materials.rs:537
pub fn durability(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + 0.5 * p.malleable) / 12.0).clamp(0.05, 1.0)
}
```

`cut_q` is "how keen an edge". The clamp to 1 hides nothing important today (steel is 0.78). `durability` is "how long it lasts", and it adds malleability, then clamps to 1. Copper (tough 9, malleable 9), bronze (9 and 6) and iron (9 and 6) all come out 1.0. Steel (tough 9, malleable 5.5) comes out 0.979. The comment says "tough or re-sharpenable materials last longer". For a working edge that addition is the bug: a soft metal is easier to reshape, and it loses its edge sooner. Vessel, boat, mesh and garment each have their own formula beside these two (`vessel_q` is a match on the material, not on the properties).

### Knowledge, the toolkit, the kit

A band's `known: Vec<Known>` is the recipes it can try. `Known` (`knowledge.rs` line 338) is 28 bytes: recipe id, skill, quality, when it was learned, who from, last time it was usable. The world's `Registry` is the list of every recipe anyone has found. Ids are stable. Old recipes that physics no longer allows are marked retired on load (`clean_recipes`, `world.rs` line 1255), not deleted, so ids in old saves still mean the same recipe.

The toolkit (`knowledge.rs` line 384) is derived. It is the closure: which items this band can make from what it knows, what is at hand, and whether it holds a flame. It remembers, per item, which known-list index produced it, which raw materials the chain needs, how deep the chain is, and (since step 3) the chain's maker skill. A two-entry cache on `Derived` covers the common case of a raw material flipping across a threshold. The recipe index is a small deterministic hasher (`FastHasher`, `knowledge.rs` line 11) because SipHash was about 6% of a single-thread run. Nothing iterates that map, so hash order never enters the sim.

The kit (`band.rs` line 232, 168 bytes) is the band's chosen tool for each job: hunt, cut, dig, axe, boat, and so on, plus the yield multipliers those choices produce. It is derived, not saved. `make_kit_*` in `effects.rs` fills it. `world.rs` `fit_kit` (line 1686) is the call from the season.

### Economy, labour, food

`economy` (`world.rs` line 1852) turns the kit into a season of food. Yields come from `effects` (`food_mults` / `yields_base`): a digger and a vessel help plants, a cut helps game and fields, an axe helps plants a little (weight 0.1) and fields a little more (weight 0.2). The band takes that harvest in order, against the stocks earlier bands left. Upkeep is labour, not a second food:

```rust
// effects.rs:366 — fraction of the labour pool, before × LABOUR_SHARE × pop
sealed * rate * (1.15 - durability(mat)) * (1.0 + 0.3 * depth)
```

`rate` is `Params::upkeep` (0.02). Depth is the recipe chain, not years of use. Ornaments cost nothing. A fire is a flat 0.01. The kit choice prices that labour as food the band did not grow (`price_of`, `effects.rs` around 787): last season's food per unit of working labour, spare hands not included. `LABOUR_SHARE` is 0.55, `WORK_SHARE` is 0.85.

So the economy and the kit choice share `tool_upkeep` and the yield function. That part of "one path" already happened, in skills step 1. What they do not share is documented in section 3.

### Conflict, voyaging, farming

These are band-level rules, each with its own file under `world/` plus a caller in the season.

- **Farming** (`world/farming.rs`). Repeated camping raises `tend`. Above a threshold the cell is a field. `sow`, `strain`, `herd`, `tame`, `settled` live on the band. Soil drains (`Params::soil_drain`). Iron tools raise `opened`. Crowd disease is `sick` / `immune` on the band, scaled by `Params::disease`.
- **Conflict** (`world/conflict.rs`). A tense meeting can become a confrontation (`Params::aggression`). Losses scale with `weapon_q` (`band.rs` line 701) and with `Params::lethality`. Weapons are the same spears and axes the band hunts with. Nothing is made "for war". Grudges are a short list on the band.
- **Voyaging.** Range and survival are functions of boat quality, seacraft, food put by, pilotage and sky lore (`world.rs` `sea_survival`, `boat_range`, `voyage`, from about line 2755). `world/voyaging.rs` holds the tech-bit flags written into the crossing ledger. Drift, scouting and landfall are counters, not a separate agent.

### Beliefs (step 4, not on this branch)

Format 16 already reserved the fields (`band.rs` lines 167–197): `use_skill`, `beliefs`, `learn_curve`, `fight_calib`, `last_fight_switch`, raid tallies. On main they are filled with defaults and the kit choice does not read them. `Belief` is 36 bytes: item, slot, flags, and eight floats (`n`, mean, variance, failure, labour, and three fight fields left at 0 until step 6).

On `skills-step4`, `beliefs.rs` owns the table. Priors come from a visible material term (`material_term`), not from `tool_effects`. The season records what the tool paid, fades unused rows, copies a weaker table to a daughter, and lets a close tie pass one belief. Food slots are then chosen from the table. The economy still pays the true effect. That split — truth in the economy, belief in the choice — is what the skills proposal asked for. It fits the structure. The copies of the material weights inside `material_term` do not (section 3).

### Events, counters, year stats

Three ways a fact leaves the sim, and a new one has to use the one that matches:

| Outlet | Where | When to use it |
|---|---|---|
| `Event` | `world.rs` line 307. A small record: tick, kind, band, other, cell, item, recipe. | Something happened once (a first, a fight, a landfall). Goes through `push_event`. Three rings in `archive.rs` (lore / conflict / life). |
| `Counters` | The `counters!` macro, `world.rs` line 510. | A running total. One line in the macro generates the struct, the sum, the csv names and the per-year delta. Forgetting a line used to leave a counter at 0 forever. |
| `YearStats` | `world.rs` line 330. One row per closed year, 448 bytes. | A stock or a mean at year end (population, bands, mean user skill). Not a sum of events. |

`docs/DATA_SCHEMA.md` and `data.rs` (`DATA_SCHEMA` 1) are the contract for `metrics.csv` and the event records. Step 0 of the skills work pre-declared the step 2–8 counters and year-stat fields so later steps fill numbers instead of growing the file format. That is the structure working.

`state_hash` (`world.rs` line 4579) is not a full checksum. It mixes tick, rng, each band's id, pop, x, y, known-list length, and the recipe count. It does **not** mix kit contents, stocks, or durability. `FIXTURE_V10_HASH.0` (`tests.rs` line 1116) is this hash just after a v10 fixture loads. It must not move: a load-time change that alters pop, position or known-list length would move it. `FIXTURE_V10_HASH.1` (line 1122) is the same world after 40 more steps. Rule changes move `.1`. That is expected, and the constant's comment lists every step that has moved it.

### Saves and the version shims

`FORMAT_VERSION` is 16 (`save.rs` line 22). `SUPPORT_FLOOR` is 1. The file is magic, then raw deflate, then a MessagePack envelope (JSON of the same envelope also loads). Rules, from `docs/SAVE_FORMAT.md`:

1. A new field gets `#[serde(default)]`, so an old file loads.
2. A breaking change bumps the version and **appends** a function to `MIGRATIONS` (`save.rs` line 27, fifteen steps, v1→v2 through v15→v16). Never delete or reorder one.
3. A file from a newer build is refused with a clear message.

Bands are not stored as one MessagePack object per field in the hot path. `codec.rs` writes rows, then column blobs for the ragged lists (`BANDS_V` 3, line 225). New band fields are appended at the end of `Band` and default when missing. Derived data (toolkits, kits, most of `Derived`) is not saved. `after_load` (`world.rs` line 1112) rebuilds it, pads stock vectors when a new material appears, fills `use_skill` for a pre-v16 save, and retires recipes physics no longer allows.

`ViewState` (`save.rs` line 394) is camera, selection and overlay. It rides in the envelope beside the world. `step` does not read it.

This shim pile is expensive to *extend carelessly* and cheap to *obey*. Format 16 already has the belief columns. Step 4 did not bump the version. Step 5's plan does not either. A durability-formula change does not need a new version: it changes future steps, not the bytes of an old file.

### RNG

`Rng` (`rng.rs` line 4) is SplitMix64. The world owns one stream (`rng_state` on `World`) for the serial phases: harvest order, meetings, demography, the key handed to the parallel season. Map noise uses `hash3` (`rng.rs` line 78), a pure function of seed and coordinates, so terrain does not consume the world stream. Maker failure is another pure roll, `maker_fail_roll(seed, band, tick, slot)` (`effects.rs` line 101), so a failed tool does not shift every later draw. Gold placement has its own salt. The rule: a new random choice in the parallel season must come from the per-band stream (or a pure hash of band, tick and slot), never from a shared `Rng` that threads would race.

### Threading

`par_map` (`fx.rs` line 275) is the only door. Parallel when the feature is on, the slice is at least 32 long, and the pool has more than one thread. `par_scale` (line 287) decays big overlays. `par_collect` (line 300) builds big arrays. Callers that need a deterministic merge write an `Fx` and apply it in band order afterwards.

What is parallel today: toolkit prep, the post-harvest season, neighbour search, meeting rounds, regrow on big maps, field regrow, and (in the renderer only) map pixels. What is serial, and must stay serial: the harvest, demography, the tail of year stats, and the walk of the partner list (it barely speeds up; see section 4).

Wasm never takes the parallel branch. A world that is comfortable on four native threads is a different machine in the browser. Section 5 is about that, not about putting rayon in wasm.

## 3. Where new work fits, and where it fights

The skills steps did not outgrow the model. They filled holes the model already had: a derived kit, a counter registry, columns on the band, `serde(default)`. Step 4 adds a module and reads fields format 16 already reserved. That is the structure working.

What fights is local: the same physical fact restated with different weights, a clamp that erases an ordering, and a per-band cost that grew faster than the world.

### What fitted cleanly

| Change | Why it fitted |
|---|---|
| Skills steps 1–3 (on main) | Layer 1 (true food and labour) went into `effects`, which the economy already called. Maker skill went into the toolkit, which was already derived and not saved. |
| Format 16 | New band columns appended. `BANDS_V` went to 3. Old files load. `FIXTURE_V10_HASH.0` did not move. |
| Counters and `YearStats` for steps 2–8 | Declared in step 0, left at 0 until the step that fills them. One line in `counters!`, one field on `YearStats`. |
| Farming, conflict, feasts, crossings | Each got a file under `world/` and counters. The season loop grew call sites, not a second kind of agent. |
| Diffusion D1–D6 | Kit-by-payoff, teaching, forgetting. Same `Band`, same step. |
| Step 4 beliefs (branch) | New `beliefs.rs`. Choice reads the table. Economy still pays `tool_effects`. That split is the proposal, and the files can hold it. |

### Five yardsticks

"Is this tool better?" is computed in five places. Two of them should exist. The other three are drift.

| Yardstick | Where | What it adds up | Used for a decision? |
|---|---|---|---|
| Kit choice | `effects.rs` `slot_score_m` (line 1011), `make_kit_scored` | This band's food, minus labour priced at last season's food per worker. Food slots use `tool_upkeep`. | Yes. This is the live choice on main. |
| Economy | `world.rs` `economy` (line 1852), yields from `effects` | The same upkeep and the same yield function, applied to the stocks. | Yes. It pays the season. It does not pick the tool. |
| Fight | `band.rs` `edge_q` (line 740), `weapon_q` (line 701) | `edge_q` is `0.55 × cut + 0.45 × durability`. The axe term inside `weapon_q` is a different mix, `0.5 × cut + 0.5 × durability`. | Yes, for losses in `world/conflict.rs`. The food choice ignores `Effects.fight` on purpose. |
| Probe, "true value" | `probe_diffusion.rs` `true_value` (line 182) | Unweighted sum of the five food multipliers, minus `KIT_UPKEEP` (0.06, `band.rs` line 734) times wear times depth, plus `0.1 ×` fight. | No. It is the report's "worse held" column. Test-only. |
| Probe, "own value" | `probe_diffusion.rs` `own_value` (line 210) | Sum of `slot_score` over the twelve slots. | It is supposed to be the band's own ranking. Since step 2 it is not quite: the choice scores food at the band's own skill, and the sum of marginals is not the marginal. |
| Beliefs (step 4 only) | `kindling-step4` `beliefs.rs` `material_term` | A fresh set of weights. Hunt uses `cut_q` and ignores durability. The axe uses `0.5 × cut + 0.5 × durability` again. The choice then does **not** call `tool_effects`. | Yes, on that branch, for food slots. The economy still pays the truth. |

The economy and the kit choice sharing `tool_upkeep` is the part that already got unified, in step 1. Keep that.

The fight formula is allowed to differ from the food formula. A battle cares about an edge that chips. A harvest cares about food per hour. They should both call the same `cut` and the same `wear`, and then weight them for the job. Today `edge_q` and `axe_term` (`effects.rs` line 511) disagree with each other about the weights, and both call `durability` which has already thrown the ordering away.

The probe is the one that actively misleads. `KIT_UPKEEP` is not `tool_upkeep`. The probe adds the farm multiplier in full for a band that does not farm, and it adds fight, which the band does not read. PR #11 measured the result on seed 6, year 350: of 772 bands that can make a steel axe and hold copper, every one ranks copper higher on the real score, and the probe still prefers steel for 94% of them. That is not a stale kit. It is a third yardstick. Step 4 points "own value" at the belief price, which is the right fix for that column. "True value" still needs to use the economy's units, or the report should say it is unweighted. Step 5's plan already retargets the probe. Do it there. Do not invent a sixth formula.

### The durability formula, and how many times it is restated

One definition, `materials.rs` lines 537–540:

```rust
((p.tough + 0.5 * p.malleable) / 12.0).clamp(0.05, 1.0)
```

Keenness, `cut_q`, lines 532–535, is `(0.6 × sharp + 0.4 × hard) / 10`, clamped to 0..1. Steel is 0.78. That clamp is not hiding an order. The durability clamp is.

| | cut | durability | axe score (half and half) | wear `1.15 − durability` |
|---|---:|---:|---:|---:|
| copper | 0.480 | 1.000 (raw 1.125, clamped) | 0.740 | 0.150 |
| bronze | 0.650 | 1.000 (raw 1.000) | 0.825 | 0.150 |
| iron | 0.640 | 1.000 (raw 1.000) | 0.820 | 0.150 |
| steel | 0.780 | 0.979 | 0.880 | 0.171 |

Copper, bronze and iron are the same durability. Steel lasts *less* long than copper, because copper is more malleable and the sum is then capped. Taking the cap off is not the fix. Uncapped, copper is 1.125, still "tougher" than steel, for the same bad reason. Michael's point is the reason: for a working edge, malleability is not durability. A soft edge dulls. Being easy to hammer is how a smith puts an edge *back*, which is a different number.

Call sites of `durability(` on this branch, from a count: `effects.rs` 7, `band.rs` 2, `probe_diffusion.rs` 1, plus the definition, plus tests (`tests_skills` 3, `tests_diag` 4, `tests` 1). Call sites of `cut_q(`: `physics.rs` 11, `effects.rs` 8, `band.rs` 2, plus tests. The physics calls are recipe *chances* (can I knap this, can I carve a shaft). Those should keep using keenness. They are not a second wear formula.

The wear expression `1.15 − durability` is written out again, not only inside `tool_upkeep`:

- `effects.rs` line 373, the real upkeep
- `effects.rs` around line 1050, the non-food score
- `probe_diffusion.rs` line 159, times `KIT_UPKEEP` instead of `Params::upkeep`
- the same expression copied into `tests_skills.rs` and `tests_diag.rs`

The axe mix `0.5 × cut + 0.5 × durability` is written in `axe_term`, again in the fight term inside `effects.rs` (around lines 836 and 864), again in `weapon_q`, and again in step 4's `material_term`. `edge_q` is a fourth mix (`0.55 / 0.45`) for the same two inputs.

`vessel_q` does not use `Props` at all. It is a match: ceramic 1, hide 0.6, fibre 0.5, anything else 0.3 (`materials.rs` line 542). `garment_q` special-cases flax at 0.55 and otherwise uses the same shape as mesh. Those are the same habit, a table beside the chemistry, and they are not what is distorting the axe. Leave them until someone is changing containers. Do not let the durability fix become a rewrite of every quality function in one go.

Other clamps are a different kind. Skill, chance and prestige fading into 0..1 are bounds on a probability, not an ordering. `ambition` is clamped to ±`AMBITION_MAX` because it is a log multiplier. Those can stay. The ones to remove are the ones that make two materials compare equal when they are not.

### What step 4 costs, and what it does not break

Step 4 did not break the save rules or the load hash. `FIXTURE_V10_HASH.0` stayed `0x1845e2690330d59`. `.1` moved, because beliefs change the next forty steps. Format stays 16.

The slowdown is real, and on the big checks it is **not** "more people".

| Run | Step 3 | Step 4 | Time | People | Bands |
|---|---:|---:|---|---|---|
| Ten seeds, 500 y, 256×160, 4 threads (CI notes on PR #12) | 1,352 s | 2,158 s | **+60%** | −2.4% | −1.3% |
| Ten seeds, 750 y, 800×800, 4 threads | 17,339 s | 22,225 s | **+28%** | +0.5% | −0.5% |
| Bench, seed 5, 300 y, 1 thread (the figure in the ask) | 85.4 s | 118.5 s | **+39%** | +38% | +21%, and about **+14% per band** |
| Same bench, local, noisy box (PR #12 body) | 120.0 s | 150.2 s | +25% | +38% (104,860 → 144,312) | +21% (6,050 → 7,345), about +3% per band |

Read the first two rows as the architecture fact. Same number of bands, a third to a half again as long. The seed-5 bench is a different world (beliefs changed who ate well, so the world grew) plus some per-band work. The local pair was run while other builds were on the box; do not quote its +3% as the overhead.

The sim jobs finished. The probe jobs did not. On the 256×160 run, probe seeds 1–5 were cancelled at the 150-minute job limit. On the 800×800 run both probe jobs were cancelled at 330 minutes, still on the first seed. Step 5 wants more probe columns. That gets harder, not easier.

Nothing in those profiles says the `Band` struct is the wrong shape. Section 4 says what the time is actually spent on. It was measured on step 3, which is this branch's sim. Step 4 sits on top of the same toolkit refresh. A belief update that runs inside every refresh multiplies whatever the refresh already costs. The late world refreshes a toolkit about eight times per band per season (perf note #9). That is the multiplier on a sloppy belief update. Step 4's own write-up says the choice does not call `tool_effects`. Good. The closure still runs, because the economy still needs the chain.

### Hot spots the structure already knows about

Both perf notes profile the same year-750 save (seed 499246296, 800×800, about 1.30 million people, about 72,000 bands, about 91 known recipes). #10 is the sharper one. A late year is about 16 s at 4 threads and about 35 s at 1 thread. #9 measured the same stretch a little higher (about 18.5 s at 4 threads). The second digit moves with the box. The shape does not.

Where a late year goes, 4 threads, from #10:

| Phase | Seconds per year | Speeds up on 4 threads? |
|---|---:|---|
| Whole year, after load | 15.6 | 2.2× against the 35 s one-thread year |
| Partner visits | 4.7 | 1.2×. Almost serial. |
| Meetings | 2.9 | 3.6× |
| Harvest (`take_harvest`) | 2.2 | No |
| Year stats | 1.2 | No |
| Regrow, occupancy, demography | under 0.2 | Not the year |

The one-thread call counts, same year, from #10:

| Call | Time | Note |
|---|---:|---|
| `closure` | 3.23 s | 8.4 µs, 384k rebuilds. Scans the known list in passes. |
| fresh `fill_chain` | 4.84 s | 12.6 µs. Thrown away and redone even when one recipe was appended. |
| `replay_chain` | 0.82 s | Entered 216k times, **skipped 0**. Practice bumps `skill_ver` every season, so the "skip if nothing changed" path never hits. |
| `fit_kit` | 3.82 s | 5.3 µs, 2-plus fits per band per season. |
| `meeting` (includes `exchange`) | 11.4 s | Teaching walks the teacher's known list. |
| `land_cost` inside the partner scan | 3.90 s | 30% of the steps walked are pairs already too far to visit. |

Proposed patches that do not change a single band's choice, from those notes:

1. Do not walk a partner line once the straight-line distance already exceeds what a river discount could save. About 1.2 s on the late year, serial, so threads do not eat it. Bit-identical.
2. When a band only *learns* a recipe, extend the old closure instead of rebuilding it. On the order of 1–2 s per late year at 4 threads. A forget still wants a full rebuild. Bit-identical if the first-writer-wins rule stays.
3. Count "how many bands can make this item" in a flat array at year end, not a `BTreeMap` insert per band. A slice of the serial 1.2 s. Bit-identical.
4. Do not refit the kit when the chain tick cannot change it. Real gain, easy to get wrong, and step 4 replaces the score being cached. Do not land a cache of today's food-minus-labour number. It would be deleted.

Not proposed, and still not proposed: parallel harvest, a spatial index for neighbours (already 3× and only about 0.7 s), fewer `refresh_toolkit` calls on the hit path (a hit is a few microseconds; the miss is the cost).

### Layout, hash maps, allocation

`Vec<Band>` is an array of 1,624-byte structs (section 4). Walking it for "what is pop and where is the camp" pulls the toolkit along. That hurts the serial year-stats pass. It does not explain closure, meetings, or the partner walk. Those chase pointers *inside* one band: `Vec<Known>`, the toolkit's own vecs, a linear scan of `partners` (`Band::partner` is a find, not a hash).

Hash maps in the hot loop were a real bug and are mostly already gone. SipHash on the recipe index was about 6% of a single-thread run; `FastHasher` replaced it (`knowledge.rs` lines 11–13). The toolkit's "which recipe makes this item" is a dense `Vec` plus a flat `rank_of`, not a map. What remains is `BTreeMap` for per-item tallies at year end (`item_bands`, `item_known`, and friends on `World`). Those are the flat-array patch above. The diffusion probe uses `HashMap`, and the probe is not in the release step.

Per-step allocation is real and, on the measured year, small. `home_range` (`world.rs` line 1787) builds a `Vec` of cells for every band every season. `par_map` collects a `Vec` of `Fx`. The harvest returns a `Vec` of economy results. mimalloc is linked on native for exactly this (`native/src/main.rs` lines 13–16). The perf note's estimate for stopping the `home_range` heap allocation is under 0.3 s of a 16 s year. Worth doing when someone is in that function. Not a reason to change the data model.

### Saves, constants, fixtures

The migration list is fifteen functions and must stay (`save.rs` line 27). That is not debt. Deleting an old reader is how a year-3 save dies. The cost is discipline: append fields, default them, do not reorder `Band`. Step 4 and the step 5 plan both obey it. A durability change does not need a version bump, because it does not change the bytes of an old file. The next step uses the new wear.

`Params` (`params.rs` line 58) is the right size: the knobs the settings window edits, saved with the world, every change logged in `settings_log`. Beside that, `pub const` is scattered on purpose (`docs/REFACTOR-NOTES.md`). A count on this branch: `world.rs` 67, `world/voyaging.rs` 50, `effects.rs` 21, `map.rs` 14, `world/feasting.rs` 11, `band.rs` 10, `archive.rs` 10, and smaller files behind those. On the order of 200 named constants. They are not in `Params` because a constant change is a new rule for every world, including one just loaded. A `Params` field would freeze the old number inside an old save. For a bugfix of physics, a constant is what you want.

The fixtures and `tests_skills.rs` (1,945 lines) are why a casual refactor feels expensive. Most of that cost is honest. A pure move of a function does not touch them. A formula change updates the tests that pin copper-at-1.0, and updates `FIXTURE_V10_HASH.1`. It must not update `.0`. The v10 fixture file itself stays byte-for-byte.

## 4. Measurements

No new simulation was run for this note. The profiles below are the ones in PR #9 and PR #10, on step 3, which is this branch's sim. The step 4 times are from that PR's CI notes and from the bench in the ask. Struct sizes are `size_of` on a release build of this tree. The scratch test that printed them was not committed.

### How big the structs are

There is no `Person` and no `Cell`. `Item` is a `u16`. These are the inline sizes, not the heap behind a `Vec`:

| Struct | Bytes | What the bytes are |
|---|---:|---|
| `Band` | 1624 | The agent. About three quarters of it is `Derived`. |
| `Derived` | 1208 | Toolkit, kit, last season's food and harvest. Rebuilt. Not the saved row. |
| `Toolkit` | 536 | Closure, chain skill, the recorded walk. Inside `Derived`. |
| `World` | 3904 | Headers. The cell arrays and the band vec live on the heap. |
| `Counters` | 720 | About ninety `u64`s. One copy on the world, plus per-year deltas. |
| `YearStats` | 448 | One saved row per year. |
| `Map` | 424 | Headers only. Cells are parallel `Vec`s. |
| `Kit` | 168 | Chosen tools and the yield multipliers. |
| `BandView` | 168 | What the kit choice is allowed to see. |
| `ItemSet` | 128 | Bitset, 16 × `u64`. |
| `Effects` | 52 | Food, labour, materials, failure, fight for one candidate. |
| `Belief` | 36 | One tool in one slot. Empty on main. Cap 24 on the step 4 branch. |
| `Recipe` | 36 | One discovered action. |
| `Known` | 28 | One recipe a band holds. |
| `Partner` | 20 | One trade tie. |
| `Event` | 24 | One log record. |

`Derived` is 1,208 of `Band`'s 1,624 bytes. A loop that only wants population and camp still touches the toolkit.

Heap, estimated, not measured. The year-750 save has 72,169 bands, about 91 known recipes, about 43 partner links (perf notes):

| Piece | Order of magnitude |
|---|---|
| The `Band` array, inline only | 72,169 × 1,624 ≈ **112 MB** |
| Known lists | 91 × 28 × 72,169 ≈ **175 MB** |
| Partner lists | 43 × 20 × 72,169 ≈ **60 MB** |
| Map layers | about 15 float arrays × 640,000 cells × 4 ≈ **40 MB** |

A late world is a few hundred megabytes before stocks, events and the archive. That matches the replay note's compressed save of 80–150 MB at that year. It is also why an 800×800 world is a bad citizen of a wasm linear memory, and why a recording of a few hundred kilobytes a year is a different object (section 5).

### Lines per concern

Section 2 has the file table. The season concerns inside `world.rs`, from the function anchors: economy about 200 lines (`economy` at 1852), one band's season about 350 (`band_season` at 2077), experiments about 240, movement and sea about 650, meetings and trade about 600, demography about 250, year stats and the archive about 400. Farming (728), conflict (380) and voyage helpers (447) are already outside the file. The 4,972 lines are mostly the social season, not unused scaffolding.

### Formula sites

Counted on this branch. Definitions included in the file that owns them.

| Function | Production call sites, besides tests |
|---|---|
| `durability` | 10 (`effects` 7, `band` 2, probe 1) |
| `cut_q` | 21 (`physics` 11, `effects` 8, `band` 2) |
| `1.15 − durability` written out | 3 production (`tool_upkeep`, the non-food score, the probe), plus two test copies |
| `0.5 × cut + 0.5 × durability` | 4 production on main (`axe_term`, two fight-term copies in `effects`, `weapon_q`), plus step 4's `material_term` |

`physics.rs` is right to call `cut_q`. It is asking "is this edge keen enough to shape that". It should not grow a wear formula of its own.

## 5. Headless running, and watching from somewhere else

Leaning on headless runs plus a recording does **not** make a step faster. It moves the drawing off the machine that is stepping, and it stops the browser from being that machine for a large world. That is worth doing. It is not a substitute for the perf patches, and it is not a reason to drop interactive play.

### What the sim does for the picture

`civ-core` does not know about frames, egui, or macroquad. `World::step` never calls the renderer. The frame loop in `native/src/main.rs` (from line 401) is the other way around: it calls `step` between draws, and it caps that work so the window stays up. Normal speed runs seasons while they fit in about 30 ms of the frame. Fast mode runs as many as fit in about 250 ms. The sim is not doing view work. The view is waiting on the sim.

Two fields on `World` exist only to be drawn:

- `trade_flow` (`world.rs` line 781), decayed ×0.85 every season (line 1299), added to when goods move (`fx.rs`)
- `conflict_flow` (line 783), decayed ×0.93, added to on a fight

Nothing in the step reads them back. The renderer does (`render.rs` around lines 236 and 246). They are saved, so the overlay survives a load. On the late-year profile, regrow plus occupancy plus these two decays is about 0.03 s of a 16 s year, under 1%. The scatter-adds during trade and combat were not split out of meetings; they are not free, and they are not the meeting. Leaving the trails in the world is cheap. Moving them out is optional, and it is not a speedup.

`ViewState` (camera, selection, overlay, `save.rs` line 394) is stored beside the world and never read by `step`.

Derived fields (`kit`, harvest, food, the toolkit cache) are for the next season, not for drawing. The know-how overlay calls `n_items()` at draw time, which reads the toolkit. That is view code walking sim data. It is not work inside `step`.

The map texture is already throttled (`render.rs` line 287). Static overlays (terrain, deposits, temperature) are drawn once. A live overlay is redrawn at most every half second, or every two seconds in fast mode, and the gap grows with how long the last rebuild took. The sim does not keep a per-frame cache. The renderer already chose "coarser than every season" without any help from `civ-core`.

So: a step is not 10% drawing with 90% sim. Drawing is outside the step. The overlay trails inside the step are well under 1% of a late year. Making the core "more headless" does not claw back the 16 seconds.

### What a recording changes

PR #7 (`docs/PROPOSAL-replay.md` on `kindling-replay`) is the right design, and this note does not redo it. The part that matters for architecture:

- A save answers "resume this world, bit-exact". A recording answers "what did it look like, year by year". They do not share a version number. Format 16 stays. A recording cannot continue a world.
- The picture is taken after `step` returns. It must not touch the rng. `state_hash` and `.0` stay put.
- One picture a year costs about half a percent of the run. The spike measured 1.7–2% of a season to build and compress a full picture; a year is four seasons and the picture is taken once.
- A late 800×800 picture in that spike was about 596 KB of band rows and 273 KB of three byte layers. A save of that world is 80–150 MB. Copying the save format into the recording would make a movie as big as a save, and it would break every time `Band` grows.
- The file is the transport. Tailing it is the live view. A small read-only server is optional and off unless asked. GitHub-hosted runners run the sim; they do not host a viewer. Watch a runner live only on a self-hosted machine. Download the file when the job ends.
- The web page today can pick a `.civsave` (`web/civ_io.js`). It cannot pick a recording, because there is no such file.

### The web build as a viewer

Today the web build **is** the sim. `build-web.sh` compiles `native/` to wasm. That binary includes `World::step`, physics, the toolkit closure, experiments, meetings. It does not include rayon or mimalloc. One thread, and a linear memory that has to hold the whole world.

A late year is about 35 s on one native thread, and about 16 s on four. Wasm is slower than native one-thread, not faster. An 800×800 year in the browser is tens of seconds of frozen page, plus a few hundred megabytes of bands (section 4). A small map (the default 256×160 early on, or a 128×80) is a fine interactive toy, and the start picker only needs a map. Those should keep running in the page.

A viewer-only mode drops, from the browser:

- `World::step` and everything under it (physics, closure, experiments, the meeting loop, demography)
- the stocks, the known lists, the rng, and the 1,624-byte bands
- the need to ship that code in the wasm module

It keeps map decode (`codec::decode_map` is already how a save stores terrain), a short row per band (x, y, pop, lineage, a handful of overlay bytes), `YearStats`, and the events. That is the picture PR #7 already specified. The inspector that shows recipes, stocks and grudges (`ui.rs`) needs a full `Band`. A short row cannot feed it. The honest UI is: scrub the year, see the map, read the charts; open a save when you want to poke one band and continue. The proposal says the viewer should admit what it cannot show. That is the right trade.

What you give up if the web build stops running the sim entirely: a stranger cannot open the page and watch a civilisation grow from a seed without a recording to point at. That is a real loss for a small map, and no loss for 800×800, which the page cannot run well anyway. So do not delete the wasm sim. Add a path that opens a recording and never calls `step`. Native keeps both: interactive play, and the same viewer.

Hosting cost is the file, not a stream of saves. At one picture a year the disk sees well under 1 MB/s (the proposal's figure). Decade pictures are the right default for an 800×800 CI batch, so a normal run does not grow an artifact. No new server, no Tokio, no axum. MessagePack and deflate are already dependencies.

Data layout does not need to change to serve this. The sim's `Vec<Band>` is the right layout for a band walking its own known list. The renderer wants six numbers. Snapshot those six. Do not store bands as a struct of arrays so the GPU is happier. The GPU is not the 16 seconds.

## 6. Options

Each row is a thing we could do. The recommendation in section 7 picks among them.

| | What | Effort | Risk | Unlocks | Hashes and saves |
|---|---|---|---|---|---|
| (a) | One edge/wear function. Every scorer calls it. | Small. A few days, plus the audit's coefficients. | Medium for *worlds* (they move). Low for the code. | The durability bug, at the root. Steps 5 and 6 stop copying a bad mix. | No version bump. `.0` stays. `.1` moves. Old saves load. |
| (b) | One vocabulary, two decisions. Not one number. | Small, mostly on the step 4/5 branches. | High if it collapses truth and belief. Low if it only deletes the extra copies. | Probe stops lying. Beliefs stay beliefs. | Probe-only edits do not move the sim hash. Chooser edits move `.1`. |
| (c) | ECS, or struct-of-arrays for bands and people. | Large. Months. | High. Easy to change who meets whom. | Not the measured hot spots. A thinner year-stats scan, at best. | A bit-identical move keeps both hashes and the save. A real ECS will not be bit-identical. |
| (d) | Split `world.rs`. | Medium, and it collides with #12 and #8. | Low if it is a pure move. High as a merge. | Easier reading. Nothing in the sim. | Pure move: hashes stay, saves stay. |
| (e) | Sim / view split, as PR #7. | Medium. The proposal's own steps. | Low for the sim, if capture sits after `step`. | Watch a big run. Web memory. Not a faster step. | New file, new version, not format 16. `.0` stays if rng is untouched. |
| (f) | Pull constants into one place. | Small to medium, huge diff. | Low for behaviour, high for every open PR. | A table of contents. Not a faster or truer sim. | Pure move: hashes stay. Do not put them in `Params`. |

### (a) One material edge

The root fix is a function, not a new crate. `materials.rs` already claims to be the chemistry. Make that claim true for wear.

```rust
/// Working-edge facts. No clamp that makes two metals compare equal.
/// `cut` is keenness. `wear` is how fast a working edge is lost.
/// `repair` is how readily a smith puts an edge back (malleability lives here, not in `wear`).
pub struct Edge { pub cut: f32, pub wear: f32, pub repair: f32 }

pub fn edge(m: Mat) -> Edge { /* the audit owns the coefficients */ }

pub fn tool_upkeep(rate: f32, it: Item, depth: u8, sealed: f32) -> f32 {
    // wear is edge(mat(it)).wear, not a handwritten `1.15 - durability(...)`
}
```

`axe_term`, `edge_q`, `weapon_q`, the probe's upkeep, and step 4's `material_term` call `edge`. They may still weight cut against wear differently: food, fight and a prior are different questions. They may not add malleability back into wear, and they may not clamp the result to 1.

`physics::apply` keeps calling keenness for "can this edge shape that". That is `edge(m).cut`, or today's `cut_q` if the audit leaves keenness alone.

Chain depth stays a separate argument of `tool_upkeep`. It is an accounting choice (PR #11), not a material property. Do not hide it inside `edge`.

Migration. No save version, no migration function. `after_load` refits kits and does not change pop, position or known-list length, which is all `.0` mixes, so `FIXTURE_V10_HASH.0` stays. `.1` moves, because forty steps diverge. Update the tests that assert copper durability is 1.0. Check with the usual guard rails (no crossing before year 100, crossing counts, war rate, no population collapse), not by freezing the old worlds. If PR #12 has not merged, rebase it so `material_term` calls `edge`. If it has, priors for a new tool pick up the new wear immediately, and a band's stored belief is wrong until a few seasons of use rewrite it. That is survivable. Starting step 5 on the old wear is not, because step 5 prices risk on top of that belief.

### (b) Two decisions, one vocabulary

Do not build this:

```rust
fn value(it: Item) -> f32 { /* kit, economy, probe and beliefs all call it */ }
```

A band that is wrong about steel is the point of beliefs. The economy must keep charging the true labour while the band believes something else. Step 4 already drew that line (the choice does not call `tool_effects`). Hold the line.

Do build this:

```rust
// Truth. The economy, and the probe's "true" column, call this.
pub fn tool_effects(...) -> Effects;

// What the band thinks. The kit choice, and the probe's "own" column, call this.
// Step 4's belief price. Step 5 adds risk on top of this function, not beside it.
pub fn belief_price(...) -> f32;
```

`belief_price` turns `edge()` into a prior. It does not retype `0.5 * cut_q + 0.5 * durability`. The probe's true column uses the band's own harvest weights and `tool_upkeep`, or it is labelled unweighted and stops being called "worse". `KIT_UPKEEP` remains only where there is no labour price (boats and the like), which is what its comment already says.

Effort is small because step 4 and the step 5 plan are already walking there. The risk is someone "cleaning up" by deleting `tool_effects` from the economy, or by feeding the belief back into the payout. Don't. Migration: a probe-only change moves no hash. A chooser change moves `.1` only, and step 4 has already accepted that.

### (c) Data-oriented storage

A full ECS, or a person record, is the wrong rework.

There are no people to transpose. The agent is the band, and the hot data of a band is its own known list and its own partners, not column 7 of every band. Closure walks one band's recipes. Meetings take two bands. A struct of arrays helps a loop that reads one field of all bands. The only such loop that shows up in the profile is year stats, about 1.2 s of a 16 s year.

The measured costs are a multi-pass scan of `Vec<Known>`, a chain walk, and a serial partner list. SoA does not make those smaller.

The one layout change that matches the sizes: `Derived` is 1,208 of the band's 1,624 inline bytes, and it is not saved. A side `Vec<Derived>`, same index as `bands`, would let a pop scan touch a few dozen bytes instead of the toolkit. It is a pure move if every `b.d` becomes `derived[i]`. Both hashes stay. The save stays. It is also easy to get wrong (a split band, a pending drift landing, a parallel borrow). It is not worth doing before step 5, and it may never be worth doing for 1 second of a 16 second year.

Do not invent `Person`. At 1.3 million people the meeting loop, the save and the determinism test all change, and no planned step needs a named individual.

### (d) Splitting `world.rs`

Worth doing the day someone is already editing meetings, or the sea, and not before. `docs/REFACTOR-NOTES.md` skipped "split `sea_season`" for this reason: a move mixed into a balance change hides the diff. PR #12 and PR #8 both edit `world.rs` (`fit_kit`, `economy`, `band_season`, `year_stats`). A split now is a merge tax on the skills work, for no change in behaviour.

When it happens: move `economy`, the meeting/`exchange` block, and the voyage functions out, the way `farming` and `conflict` already moved. Pure move. Run the thread-count test. `.0` and `.1` both stay. No save change.

### (e) Sim and view

Do PR #7, on its own, when you want to watch a run that is still going or one that finished overnight. Capture after `step` returns. Do not read the rng. Do not write the overlay trails into the recording by running a second sim.

Keep interactive play on the desktop: pause, inspect a band, change a `Params` knob, save, continue. The recording is not a save. The web build keeps `step` for a small map and gains "open a recording" for a large one.

Version the recording from 1. Do not borrow `FORMAT_VERSION`. Old recordings stay readable by the same rule as saves: a version field, old readers kept, a newer file refused with a clear message. `.0` is untouched because the capture is not a load and not a step.

This does not unlock step 5. It does not pay back the 28–60%. It unlocks looking at a world the browser cannot run.

### (f) Constants

Do not consolidate them as a project. A single `consts.rs` would be a pure move and the hashes would stay, and the diff would touch every open PR for a navigation win. A generated index, or a heading in `docs/DATA_SCHEMA.md`, is enough if a number is hard to find.

Do not move balance constants into `Params`. A loaded save would keep the old number, so a physics fix would silently not apply to the worlds you care about continuing. The settings window would grow a hundred knobs nobody should drag during a run.

## 7. Recommendation, against the open PRs

Same order as section 1, named against the drafts.

**Before step 5**

1. **Durability, its own change, before PR #8 and coordinated with PR #12.** The audit owns the coefficients. The architecture rule is: one `edge`, malleability is repair and not wear, no clamp that flattens copper, bronze and iron. Rebase #12 onto it if #12 has not merged, so `material_term` is not a handwritten copy. Do not put this inside the step 5 PR, and do not put the chain-depth question (PR #11) inside it. Depth can stay. The axe note showed depth, not the durability cap, is most of why a forager keeps copper. Both can be true. Fix the physics now; decide the accounting separately.
2. **Bit-identical perf from PR #9 and PR #10.** Skip a partner line that is already too far. Extend a closure when a band only learned a recipe. Flat array for the year-end item counts. Do not cache today's food score (#9's "leave the kit score alone" is right, and step 4 deletes that score). Do not parallelize the harvest. These do not move `.0` or `.1`. Land them on main, or on a branch that #12 and #8 can rebase onto. They are safe under beliefs because they do not touch the chooser. They matter more after step 4, not less: the (a) worlds got 60% slower with no extra bands, and the probe jobs stopped finishing.
3. **Then step 5 (PR #8), on beliefs.** Its own plan says: if the choice still calls `tool_effects`, stop; if `.0` moved, stop. Hold it to that. Add risk on `belief_price`, not on a new function. Point the probe's two columns at `tool_effects` and `belief_price`. Write down that the bench budget is +5% against step 4, and that step 4 is already +28% and +60% against step 3 on the same-size worlds. A pass that meets "+5% versus step 4" can still be the wrong time to add more.

**Can wait**

- **PR #7, the recording.** Do it when you want to watch runs, including CI. It does not block skills, and skills should not block it, as long as it does not edit `push_event` while step 4 is in that function. The proposal already says to skip the event tap in the first cut. Good.
- **Splitting `world.rs`.** After step 5, or inside a step that is already rewriting one of those functions, as a separate commit that does not change behaviour.
- **A side `Vec<Derived>`.** Only if a later profile shows year stats or a band scan dominating after the patches above. Not on a hunch from the 1,624-byte figure.
- **`vessel_q` and the other match-tables.** When containers are the subject. Not this fix.
- **Overlay trails.** They can stay on `World`. Moving them is a tidy, not a speedup.

**Do not do**

- An ECS, a person record, or a struct-of-arrays rewrite of `Band`.
- One `value()` shared by the economy and the beliefs.
- A project to gather every `pub const` into `Params` or into one file.
- Parallel harvest.
- Turning the web build into a viewer only, with no sim left in the page.
- Streaming saves as the way to watch a run. A save is a resume point. A picture a year is the movie.
- Anything that moves `FIXTURE_V10_HASH.0`. If a load-time refill changes pop, position or known-list length, stop.

PR #11 stays an analysis. It is right that the kit choice matches food-minus-labour, and right that the probe's yardstick is the thing calling copper "worse". It treats "malleable copper wears less" as acceptable physics. This note does not. That disagreement is a decision for Michael, listed below, not a second architecture.

## 8. Decisions needed

1. **Wear.** Confirm that malleability must not increase the durability of a working edge, and that the 1.0 cap goes because it flattens copper, bronze and iron. The audit picks the coefficients. If you would rather keep "soft metals are re-sharpened, so they last", say so and the formula stays; the single function is still worth having, but it is then a tidy and not a physics fix.
2. **When, relative to PR #12.** Rebase beliefs onto the new `edge` before #12 merges, or merge #12 first and accept a few seasons of stale beliefs after the formula lands? The recommendation is rebase-first.
3. **Chain depth as a seasonal cost.** Leave it (PR #11's option 1), or stop charging the whole chain every season (option 3)? This review does not fold that into the durability change. It does change who holds a steel axe, and it moves `.1`.
4. **Is +28% / +60% on step 4 acceptable** until the bit-identical patches land, or do those patches have to land before step 5 starts? The recommendation is: patches first, or in the same queue, ahead of step 5.
5. **Web.** Keep a small-map sim in the wasm build and add a recording viewer (recommended), or stop shipping `step` to the browser at all?
6. **Interactive play.** Confirm the desktop app keeps pause, inspect, live `Params`, and save-and-continue. The recording does not replace that.
7. **`trade_flow` / `conflict_flow`.** Leave them on `World` (recommended; they are under 1% and they make old saves' overlays still work), or drop them from the step and let only a live viewer accumulate them?

