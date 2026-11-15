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

Sections 3–8 (where each feature fits, the measurements, viewing, the options, and the questions) follow this map.

## 2. The architecture as it is

### Crates

The workspace (`Cargo.toml`) has two members. The web build is not a third crate.

| Piece | Path | What it is |
|---|---|---|
| `civ-core` | `civ-core/` | The sim. Native, headless and web all call this. No window, no clock in the result. |
| `kindling` (native) | `native/` | Desktop app, headless runner, and the wasm target. Macroquad + egui. |
| web shell | `web/index.html` (11 lines), `web/civ_io.js` (26 lines) | Page that loads `kindling.wasm`. Save download/upload. Not a sim. |
| tools | `tools/*.py` | CI result tables. Not in the sim. |
| tests | `civ-core/src/tests*.rs`, `civ-core/tests/fixtures/` | Unit tests and old-save fixtures, v1 through v15 (format 16 is the current writer). |

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
