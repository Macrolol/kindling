# Refactor notes (v9)

Refactors considered while implementing the v9 motivation proposal. Done ones are listed with
their commit; skipped ones with the reason and a rough effort, so they can be picked up later.

## Done

- `Map::sea_gaps` replaces the private sea BFS in `analysis.rs`; the same helper now measures the
  gap at each landfall (step 1a).
- `Map::continent_cells` / `Map::is_continent` replace bare comparisons with `CONTINENT_CELLS`
  (step 1a).
- `analysis::effective_number` is shared by the summary and the per-year stats (step 1a).
- Fixed the misleading "marry out" wording in `Band::kinship` (step 1a).

## Skipped

_(filled in as steps go)_
