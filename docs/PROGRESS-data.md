# Data groundwork + save size: progress notes (pt88u)

Resume notes for the data/save work started 2026-10-26. Scope: Phase 0 of
docs/PROPOSAL-data.md (manifest, structured metrics, counter registry, event-log fix, tools/CI)
plus smaller saves with a history store. **Not** in scope yet: the stream server, replay, dashboard
(phases 1-2) - but every format below is meant to be reused by them.

## 1. What a 147 MB save is made of (measured)

`cargo run --release -p civ-core --example save_breakdown -- FILE.civsave` (msgpack-named size and
deflate-6 size per top-level World field, per Band field, per map layer).

s869, 750 years, 800x800, 68,730 bands, 147.0 MB file (deflated sizes):

| part | MB | note |
|---|---:|---|
| bands | 93.7 | of which `known` 52.1 (3.9M entries, 57/band), craft 7.2, partners 5.8, ancestry 1.9, origin 1.8, stock 1.8, rest ~20 (mostly field names repeated per band) |
| cell_first | 32.5 | (cell, tick) pairs, 73 MB raw |
| map | 14.2 | f32 terrain layers; **bit-identical to `Map::generate(w,h,seed)`** |
| plants/game/small/fish, flows, farming layers | ~6 | per-cell f32 sim state |
| events | 0.5 | 40k rolling log = less than one season at this size |
| history | 0.08 | YearStats per year: tiny |

Prototype (`examples/zz_compact_proto.rs`, not kept): known as columns 52 -> 35 MB (skill and
quality are near-random f32s: ~2.7 and ~3.3 bytes each even under xz, so this is close to the
lossless floor); cell_first delta-varints 32.5 -> 9.8 MB (xz: 4.8); map XOR regenerated map
14.2 -> 0.03 MB; shuffling the f32 world layers buys only 4%. xz over deflate gains little except
on cell_first, so the container stays raw deflate (pure Rust, works in wasm; no zstd C build).

## 2. Decisions

- Saves stay **lossless** (save -> load -> continue must give the same run), so no quantizing.
- Format v10: binary saves (MessagePack, `is_human_readable() == false`) store the heavy parts as
  compact blobs; JSON saves and the migration path (human-readable) keep the old logical shape, so
  `save_json` output and all JSON migrations are unchanged. Binary v5-v9 saves are decoded with
  rmp's human-readable mode (old logical shape).

## 3. Status / log

- [x] breakdown measured (above)
- [x] v10 compact encodings (bands, cell_first, map) - 3edc611
- [x] counter registry (`counters!` macro; REFACTOR-NOTES) - ed17335
- [x] history store (tiered) + genealogy; event-log fix (per-class rings, Learned/Forgot as
      per-item tallies in the frames) - 119c494; codec accepts pre-blob v10 dev saves - c608d8c
- [x] run manifest + `--out DIR` (manifest.json, metrics.csv, summary.json, chronicle.ndjson,
      frames.ndjson, opt. genealogy.csv) - 91cd290; docs/DATA_SCHEMA.md - 21a880b
- [x] tools/ + CI use structured outputs (collect_runs.py; agg_results.py reads text or --out
      dirs, identical rows on the same run; workflow passes --out out/data) - 21a880b
- [x] determinism check, before/after sizes and times (below)

## 4. Results

### Save size and time, v9 -> v10 (12 CI saves, 800x800, 750 y; re-saved locally)

| seed | v9 | v10 | v10 / v9 | v9 load | v10 save | v10 load |
|---|---:|---:|---:|---:|---:|---:|
| 863 | 47.74 MB | 23.10 MB | 48.4% | 1.26 s | 2.90 s | 1.21 s |
| 864 | 122.93 MB | 68.77 MB | 55.9% | 3.09 s | 7.23 s | 2.22 s |
| 865 | 42.79 MB | 19.18 MB | 44.8% | 1.08 s | 2.52 s | 1.11 s |
| 866 | 110.13 MB | 61.21 MB | 55.6% | 2.76 s | 6.25 s | 1.87 s |
| 867 | 141.44 MB | 80.57 MB | 57.0% | 3.58 s | 7.96 s | 2.26 s |
| 868 | 71.70 MB | 38.09 MB | 53.1% | 1.88 s | 4.32 s | 1.46 s |
| 869 | 146.96 MB | 83.02 MB | 56.5% | 3.66 s | 8.29 s | 2.60 s |
| 870 | 92.45 MB | 50.31 MB | 54.4% | 2.65 s | 5.62 s | 1.79 s |
| 871 | 108.33 MB | 59.61 MB | 55.0% | 2.79 s | 6.26 s | 1.96 s |
| 872 | 135.32 MB | 76.02 MB | 56.2% | 3.41 s | 7.84 s | 2.21 s |
| 873 | 130.70 MB | 71.42 MB | 54.6% | 3.20 s | 7.32 s | 2.13 s |
| 874 | 114.25 MB | 62.41 MB | 54.6% | 2.95 s | 6.74 s | 2.03 s |

Saving with the v9 code took 18.85 s for s869 (v10: ~8.0-8.3 s). Test save (seed 11, 400 y,
256x160): 10.90 -> 6.45 MB. What is left is mostly per-band f32 skill / quality values, which
are close to incompressible; only a lossy encoding would shrink them further (not done: saves
stay exact).

### History store cost

| run | without | with archive + rings | |
|---|---:|---:|---|
| seed 11, 400 y, 256x160 | 6.45 MB | 7.11 MB | 130 frames (100 yearly + 30 decadal), genealogy 24,154, weddings kept 31,493, events 86,897 (was 40,000) |
| same, continued to 500 y | 6.20 MB* | 7.03 MB | 140 frames, genealogy 39,198 |
| s869 (800x800) + 5 y | 83.02 MB | 83.35 MB | archive 1.23 MB (mostly weddings), events 0.55 MB |

(*the base continuation save.) Event reach on s869: the old 40k ring held less than one season;
the life ring now reaches ~17 years back and the conflict ring ~8.5 years.

### Determinism

- Seed 11, 400 y, 256x160: text report identical to the pre-change build; the JSON world is
  identical except `events` (retention) and the new `archive`. Save -> load -> +100 y: same.
- `--out` changes nothing: report and .civsave byte-identical with and without it (seeds 3-4).
- cargo test --release --workspace passes (incl. the v9 fixture checksums, the save round trip
  and `data_outputs_shape`).
- wasm: the browser build compiles; a wasm32 harness (civ-core, node) loads the v9 fixture,
  runs, saves v10, reloads and runs again. Native and wasm runs of the same save already
  diverged before this work (same divergence with the v9 code: float maths differs between the
  targets), so cross-target identity was not expected and is unchanged.

## 5. For phases 1-2

See docs/DATA_SCHEMA.md "How the live stream and replay plug in": a stream frame = one
metrics.csv row + one Frame; the event feed = `codec::encode_events` / `data::event_record`
hooked at `World::push_event`; replay = history + frames + genealogy, exact replay = load a save
and re-run.
