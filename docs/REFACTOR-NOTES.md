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
- Every new tunable is a named, documented `pub const` next to its siblings (`SEA_TEACH_*`,
  `DISSENT_*`, `MARRIAGE_P`, `REFINE_*`/`QUALITY_*`, `PRESTIGE_*`) instead of an inline literal
  (steps 2-8).
- `Counters` and `fx::add_counters` documented: a new counter field must be added to the
  `add_fields!` list or it silently stays zero in parallel phases (step 4). (Superseded by the
  `counters!` registry, see below.)
- Moved the displaced doc comment of `Band::bond` back onto its field (step 6).
- `tools/agg_results.py` labels its columns "baseline" / "new" from the last two path parts, and
  parses each v9 summary line, so step-to-step comparisons need no hand editing (steps 1-8).
- `Derived::fin_dirty`: a quality-only change rebuilds just the kit (`make_kit` +
  `refine_kit`), not the toolkit closure, so refinement costs almost nothing per season (step 7).
- `note_crossing` is the single place a crossing is counted (and now rewarded, step 8);
  `analysis::gini` is a reusable helper for any inequality measure (step 8).

## Skipped

- **Split `sea_season`** (~250 lines: lure, destination choice, toll, landing, word of shores).
  Splitting into `choose_voyage` / `make_voyage` / `come_ashore` would make sea changes easier.
  Medium effort; skipped because each step was being balanced against CI and a big move would
  have mixed refactor noise into the balance diffs.
- **Derived `Default` for `Band` / `Kit`.** Several literals spell out every field; the P6
  invention literal now uses `..Default::default()`, but the band constructors (founding,
  splitting, drift landings) still list fields by hand. Small effort; a `Band::founder(..)`
  constructor would remove most of it.
- **Meeting / exchange structure.** `meeting` -> `exchange` (teaching, seacraft, far-shore word,
  quality, trade, food gifts) is one long function. A table of "meeting effects" each taking
  `(bands, src, dst, rng, fx)` would let new social effects (P7-style) plug in. Medium effort.
- **Hunting sea mammals** (needed for a richer P2 coastal economy) - content, not refactor;
  needs new resources and recipes.
- **Raids ignore ambition.** P4's ambition shapes splitting and wandering but not raiding; wiring
  it into `confront` touches war balance, so left for a dedicated balance pass.
- **P3 expectation-valuing** (bands valuing a destination by remembered return rather than the
  current estimate) - skipped as a model change with unclear balance effects.
- ~~**`add_fields!` manual list.**~~ Done (data groundwork): `Counters` is declared through the
  `counters!` macro in `world.rs`, which generates the struct, `Counters::add` (what
  `fx::add_counters` calls), `NAMES` / `values()` for the metrics output and `since()` for the
  per-year deltas in the history store. A new counter is one line in that list and is summed,
  written to `metrics.csv` and archived automatically; test `counter_registry_covers_every_field`
  checks the generated code against the serialized struct. `YearStats` per-year sums still use
  the short `add_fields!` list in `fx::add_acc` (most of its fields are set at year end).
- **P6 per-century improvement series** - only the total refinement count is kept; adding a
  `YearStats` field is easy but enlarges every save.
- **Quality does not touch weapons** - deliberate (war balance), but the hook would sit in
  `refine_kit` next to `protect`.
- **P7 ornament-seeking** - status-hungry bands do not aim their experiments at ornaments, and
  prestige goods have no trade value of their own. Would need an experiment-attention hook and
  a trade-good value table.
