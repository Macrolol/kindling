# Save format

- File: `MAGIC` (`b"CIVSAVE\x01"`) followed by raw-deflate compressed MessagePack of an `Envelope`.
- `Envelope { format_version, app_version, saved_at, world }`. Plain JSON of the same envelope also loads, which helps with debugging.
- Maps and hash maps are stored as lists of pairs, so keys don't have to be strings.
- Derived data (toolkits, caches) is never saved. `World::after_load` rebuilds it.

## Compatibility rules
1. Any new field gets `#[serde(default)]`, so old saves still load.
2. A breaking change bumps `FORMAT_VERSION` and appends a function to `MIGRATIONS`. That function rewrites the envelope's generic value tree from version N to N+1. The loader runs every step from the file's version up to the current one.
3. Never delete or reorder a migration. Saves from newer versions are refused with a clear message.

Current: `FORMAT_VERSION = 1`, no migrations yet.
