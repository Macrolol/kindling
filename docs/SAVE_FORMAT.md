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
    `world.item_peak`, `world.traded_first`, `small`, the new `YearStats` trade fields, and
    the new `EventKind` values (12 Landfall, 13 LostLocal, 14 FirstTraded; unknown kinds from a
    newer build clamp to the last known one).
  - Not saved, rebuilt on load: toolkits, kits, per-season figures (food, need, stress...),
    demand signals, the trade-flow overlay (starts empty after a load and fills as trade
    happens), landmass labels.
- Fixture `civ-core/tests/fixtures/v1_seed7_y60.civsave` (written by the v1 build) is loaded by
  the test `v1_fixture_loads_and_runs`.
