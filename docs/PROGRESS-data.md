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
- [ ] v10 compact encodings (bands, cell_first, map)
- [ ] history store (tiered) + genealogy
- [ ] event-log fix (per-class retention, Learned/Forgot aggregated)
- [ ] counter registry
- [ ] run manifest + `--out DIR` (metrics.csv, summary.json, events ndjson) + DATA_SCHEMA.md
- [ ] tools/ + CI use structured outputs
- [ ] determinism check, before/after sizes and times
