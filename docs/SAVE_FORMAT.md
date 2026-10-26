# Save format

- File: `MAGIC` (`b"CIVSAVE\x01"`) followed by raw-deflate compressed MessagePack of an `Envelope`.
- `Envelope { format_version, app_version, saved_at, world }`. Plain JSON of the same envelope also loads, which helps with debugging.
- Maps and hash maps are stored as lists of pairs, so keys don't have to be strings.
- Derived data (toolkits, caches) is never saved. `World::after_load` rebuilds it.

## Compatibility rules
1. Any new field gets `#[serde(default)]`, so old saves still load.
2. A breaking change bumps `FORMAT_VERSION` and appends a function to `MIGRATIONS`. That function rewrites the envelope's generic value tree from version N to N+1. The loader runs every step from the file's version up to the current one.
3. Never delete or reorder a migration. Saves from newer versions are refused with a clear message.

Current: `FORMAT_VERSION = 10` (Kindling 0.9 development). v0.8.0 writes format 8, v0.7.0 format 7, v0.6.1 format 6, v0.5.x and v0.6.0 format 5.

## Version history
- **v1**: first stable format (stage 1: stone/fire/pottery/metal emergence on the old generator).
- **v2** (food tech, boats, continents, trade). Migration `v1_to_v2` in `civ-core/src/save.rs`:
  - `bands[].stock` padded from 13 to 18 entries (metals can now be carried and traded);
  - `bands[].origin` added (where each carried material was gathered; `u32::MAX` = unknown);
  - `map.gen_version` written as 1 for old maps, so `Map::upgrade_legacy` adds the v2 map
    layers (shelf fish, small game, landmass labels) on load.
  - Additive v2 fields that need no migration: `bands[].partners`, `world.landfall`,
    `world.item_peak`, `world.item_lost`, `world.traded_first`, `small`, the new `YearStats` trade fields, and
    the new `EventKind` values (12 Landfall, 13 LostLocal, 14 FirstTraded; unknown kinds from a
    newer build clamp to the last known one).
  - Not saved, rebuilt on load: toolkits, kits, per-season figures (food, need, stress...),
    demand signals, landmass labels (v2 only: the trade-flow overlay, saved since v3).
- **v3** (lineage knowledge, craft habits, seafaring). Migration `v2_to_v3`:
  - `bands[].known[].kin` added (found by this band or an ancestor): set to `from == u32::MAX`
    for v2 bands (v2 could not tell an ancestor's find from a teacher's);
  - `bands[].craft` added (21 attention / way-of-working weights), padded with 1.0 = no leaning;
  - `bands[].far_shore` added (`null` = no far shore known).
  - Additive v3 fields: `world.trade_flow` (the trade overlay is now saved, so it is no longer
    empty after a load), sea counters, the `EventKind` values 15 LostAtSea and 16 Drifted.
  - Map generator 3 (`map.gen_version` 3) adds placer tin in stream gravels below tin lodes.
    Older maps are stored in the save and keep their own geology (no placers are added).
- **v4** (competition and conflict). Migration `v3_to_v4`:
  - `bands[].grudges` added (`[{id, lineage, w}]`, wrongs remembered against other bands): `[]`;
  - `bands[].pressed` added (0..1, recent pressure from stronger neighbours): `0.0`.
  - Additive v4 fields: `world.conflict_flow` (conflict overlay; rebuilt empty for older saves),
    conflict `counters` (confrontations, avoided, tributes, fled, fights, conflict_deaths,
    displaced, fled_by_sea, absorbed, captives, max_fight_deaths), the matching `YearStats`
    fields, `params.aggression` / `lethality` / `grudge_memory` (defaults 1.0 / 1.0 / 0.92), and
    the `EventKind` values 17 Fight, 18 FirstFight, 19 BloodiestFight, 20 Fled, 21 Tribute,
    22 Absorbed, 23 FirstSeaRefuge (for Fight-type events `recipe` holds the number killed).
- **v5** (farming stage). Migration `v4_to_v5`:
  - five materials appended after bronze (grain, flax, wool, iron ore, iron): `bands[].stock` and
    `bands[].origin` padded from 18 to 23 entries (`0.0` / `u32::MAX`). Item ids are
    `form * 32 + mat`, so existing item ids do not move;
  - `bands[].craft` is [18 material leanings, 3 action leanings]: five neutral `1.0` material
    leanings are inserted at index 18, before the action leanings (21 -> 26 entries);
  - new band fields `sow`, `strain`, `herd`, `tame`, `settled`, `sick`, `immune`: `0.0`.
  - Additive v5 fields: the per-cell farming layers `world.camp`, `tend`, `dom`, `fert`, `crop`
    (absent in older saves and built fresh on load: fertility 1, nothing tended),
    `world.farm_starts`, `world.farm_flags`, farming / disease `counters` and `YearStats` fields,
    `params.farm_rate` / `farm_yield` / `soil_drain` / `herd_rate` / `disease` (all default 1.0),
    and the `EventKind` values 24 FirstCultivation, 25 FarmingOrigin, 26 TookUpFarming,
    27 AbandonedFarming, 28 FirstDomestic, 29 FirstHerding, 30 FirstEpidemic, 31 Epidemic,
    32 FirstPlough. `crop_pot` / `graze_pot` are derived and never saved.
  - Map generator 4 (`map.gen_version` 4) adds bog and laterite iron ore. Maps below gen 4 gain
    iron ore on load (`Map::upgrade_legacy`); no other geology changes.
  - Additive (after the farming beta): `bands[].carry`, the part of last season's derived band
    state that the next season reads first (food ratio, stress, harvest mix, farm share, trade
    demand ...). Without it a reloaded world drifted from the original after a few seasons;
    saves without it (v1-v4, and the farming beta) load with it zeroed, as before.
  - The farming beta build (`dist/kindling-farming-beta-linux-x86_64`) already writes format 5;
    its saves load unchanged (fixture `v5beta_seed4_y180_64x40.civsave`, test
    `v5beta_fixture_loads_and_runs`). The wild-progenitor layer is derived and rebuilt on load.
- **v6** (Kindling 0.6.1, sparse first-arrival table). Migration `v5_to_v6`:
  - `world.cell_first` (dense: for each item ever held, one `u32` per map cell, so 10.9 MB per
    item at 1650×1650) is replaced by `world.cell_first_sparse`: pairs
    `[item, {w, h, cells: [[cell, tick], ...]}]`, listing only the cells the item has reached,
    in ascending order. In memory it is a 64×64-tile table (`civ-core/src/world/cell_first.rs`)
    that allocates a tile only on the item's first arrival there.
  - The migration rewrites the JSON tree (dense table to cell list, `u32::MAX` = never). Binary
    format-5 saves (v0.5.x, v0.6.0) skip the JSON tree: they are decoded directly, which reads the
    legacy dense field and converts it in `World::after_load`. That route gives the same world
    as the migration path (checked by the fixture test). `migrated_from` reports 5 either way.
  - Fixture `v6_0_seed4_y150_64x40.civsave` (written by the v0.6.0 release binary) is loaded by
    `v6_0_fixture_loads_sparse_cell_first`. The test compares the result with the dense table read
    straight from the file, checks that the JSON migration route gives the same `cell_first` and
    state hash, and checks that a format-6 round trip writes no dense `cell_first`.
- **v7** (Kindling 0.7.0, iron needs bellows and forging). Migration `v6_to_v7`:
  - Two forms are appended (`Bellows` = 16, `Bloom` = 17), so every older item id is unchanged.
  - Smelting iron ore now gives an iron bloom (item 566), which has to be forged into wrought
    iron (raw iron, item 22). Every heat recipe in `world.reg.recipes` that has raw iron ore
    (item 21) among its inputs and raw iron as its output is re-pointed at the bloom, with
    `p = 0.05`. Iron already made, carried or worked is left alone.
  - Band fields `anc` (earlier ancestors) and grudge field `par` are additive and load as 0.
  - Binary format-5 and format-6 saves are decoded directly and get the same re-pointing in
    place (`iron_to_bloom`); JSON saves run the migration. `migrated_from` reports 5 or 6.
  - Fixture `v6_1_seed4_y125.civsave` (written by the v0.6.1 code, with iron already smelted)
    is loaded by `v6_1_fixture_iron_recipes_become_bloom`, which checks both routes.
- **v8** (Kindling 0.8, technologies for activities). Migration `v7_to_v8`:
  - Two materials are appended (`Brine` = 23, only where it lies and never carried, and `Salt` =
    24), so `bands[].stock` and `bands[].origin` grow from 23 to 25 entries (0.0 / u32::MAX).
  - `bands[].craft` is [material leanings, 3 action leanings]: the 2 new neutral (1.0) material
    leanings are inserted at index 23, before the action leanings.
  - Seven forms are appended (`Bow` 18, `Shield` 19, `Outrigger` 20, `Sail` 21, `Plank` 22,
    `Frame` 23, `Ornament` 24), so every older item id is unchanged.
  - Binary format-5/6/7 saves are decoded directly; `Band::fit_vectors` (via `band::fit_craft`)
    makes the same insertion on load. JSON saves run the migration. `migrated_from` reports 7.
  - Fixture `v7_0_seed4_y150_96x60.civsave` (written by the v0.7.0 release binary) is loaded by
    `v7_0_fixture_gains_new_materials_in_place`, which checks both routes agree leaning by leaning.
- **v9** (Kindling 0.9 development, human motivations). Migration `v8_to_v9`:
  - `bands[].far_shore_year` is new: the year a band's far shore was last seen or confirmed
    (rumours now fade after `RUMOUR_YEARS`, 50). A rumour already held is treated as old news,
    heard 40 years before the save, so it fades within about a decade unless confirmed.
  - Additive v9 fields: `world.landfalls` (per landmass: when, which band, by sea or on foot, from
    which landmass, the sea gap from settled land; empty for older saves, whose landfalls the
    analysis reports as reached "[?]"), `YearStats.eff_lineages`, `bands[].ambition` and
    `bands[].dissent` (0 = ordinary, calm), `counters.dissent_splits`, `bands[].ancestry` (empty = all of the band's own
    lineage), `bands[].partners[].married` (0 = no marriage ties), `counters.marriages` and
    `counters.fights_in_law`, `bands[].known[].quality` (0 = the plain recipe) and
    `counters.refinements`, `bands[].prestige` (0 = no standing) and `counters.prestige_taught`, and `world.camped` (per
    cell, 1 + the last year a band camped there; when missing, `after_load` marks land on
    settled landmasses as camped in the load year and everything else as never visited).
  - Binary format-5..8 saves are decoded directly and dated in place (`age_old_rumours`); JSON
    saves run the migration. `migrated_from` reports 8.
  - Fixture `v8_0_seed4_y150_96x60.civsave` (written by the v0.8.0 code) is loaded by
    `v8_0_fixture_loads_and_runs`, which checks both routes agree.
- **v10** (Kindling 0.9 development, compact binary saves). Migration `v9_to_v10` (identity):
  - Only the **binary** layout changes; the logical (JSON) shape is the same as v9. MessagePack's
    serde (de)serializer is not "human readable", and three parts of the world then go out as
    byte blobs (`civ-core/src/codec.rs`), each starting with its own layout number:
    - `world.bands`: the rows (every band field except the three below, as positional
      MessagePack, so **new `Band` fields must be appended at the end** and default when missing),
      then `known`, `partners` and `craft` as columns across all bands in their original order
      (varint / zigzag deltas, f32s byte-shuffled and bit-exact);
    - `world.map`: the bitwise XOR of every layer with `Map::generate(w, h, seed)` (all zeros
      for an untouched map of the current generator; maps of an older `gen_version` are stored
      against nothing, i.e. as they are);
    - `world.cell_first_sparse[]`: each table as `[1, w, h, n, cell gaps.., zigzag tick deltas..]`.
  - JSON saves and the JSON migration route keep the logical shape. Binary v5-v9 saves are decoded
    directly in rmp's human-readable mode (`load_bytes(.., logical)`), which reads the old shape.
  - Nothing is rounded: a world saved and loaded continues exactly as before (tests
    `v10_binary_is_exact`, `save_round_trip_continues_identically`).
  - A future change to a blob layout gets a new layout number and keeps the old decoder; blobs
    are not migrated through JSON. To migrate a v10+ binary save through JSON, decode it with the
    current reader and re-serialize it with `serde_json` (human readable = logical shape).
  - Fixture `v9_seed9_y150_64x40.civsave` (written by the v9 code) is loaded by
    `v9_fixture_loads_exactly`, which checks it against checksums taken from the v9 build.
  - Typical effect: a 147 MB v9 save (800×800, 750 years, 68.7k bands) re-saves as 83 MB.
  - **Event log and history store** (`civ-core/src/archive.rs`, same version, all fields default
    when missing, so v10 saves written before them still load):
    - `world.events` is an `EventLog` of three rings with their own retention (lore 20 000:
      learned / forgot; conflict 40 000; life 40 000: everything else). Older saves' single ring
      is sorted into the three on load.
    - `world.archive`: `frames` (one `Frame` a year for the last 100 years, folded into one a
      decade before that), `genealogy` (every Founded / Split / Merged / Died / Drifted /
      Absorbed event, for ever), `marriages` (single weddings of the last 10 years),
      `last_counters`, `tally` (the open year) and `begun` (tick the store started: 0, or the
      load tick for pre-v10 saves).
    - Event lists (`chronicle`, the rings, `genealogy`, `marriages`) are stored as one blob each
      (`codec::encode_events`, layout `EVENTS_V` = 1): count, then columns of tick deltas,
      kinds, bands, `other - band`, cells, `item + 1`, `recipe + 1` as varints. The reader also
      accepts a plain sequence of events. JSON keeps them as lists of event objects.
    - Neither is read by the simulation; results are unchanged.
    - Cost: about +10% on a 400-year 256×160 save (6.45 → 7.11 MB), +0.4% on the 800×800 save
      (83.02 → 83.35 MB).
- Fixtures `civ-core/tests/fixtures/v1_seed7_y60.civsave` … `v4_seed7_y60.civsave` (written by
  the v1–v4 builds) are loaded by `v1_fixture_loads_and_runs` … `v4_fixture_loads_and_runs`.
