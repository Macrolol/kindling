# Save format

- File: `MAGIC` (`b"CIVSAVE\x01"`) followed by raw-deflate compressed MessagePack of an `Envelope`.
- `Envelope { format_version, app_version, saved_at, world }`. Plain JSON of the same envelope also loads, which helps with debugging.
- Maps and hash maps are stored as lists of pairs, so keys don't have to be strings.
- Derived data (toolkits, caches) is never saved. `World::after_load` rebuilds it.

## Compatibility rules
1. Any new field gets `#[serde(default)]`, so old saves still load.
2. A breaking change bumps `FORMAT_VERSION` and appends a function to `MIGRATIONS`. That function rewrites the envelope's generic value tree from version N to N+1. The loader runs every step from the file's version up to the current one.
3. Never delete or reorder a migration. Saves from newer versions are refused with a clear message.

Current: `FORMAT_VERSION = 2`.

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
- Fixtures `civ-core/tests/fixtures/v1_seed7_y60.civsave` and `v2_seed7_y60.civsave` (written by
  the v1 and v2 builds) are loaded by `v1_fixture_loads_and_runs` and `v2_fixture_loads_and_runs`.
