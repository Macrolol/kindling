# Kindling v6.1: zoom-out, picker fix, single-thread speed, sparse first-reach table. Progress log

25 Oct 2026, 08:29 to about 09:30 EDT. Rules: one local sim at a time, RAYON_NUM_THREADS=4,
cargo -j4, every run under `timeout`. Multi-seed balance and speed runs go on GitHub Actions
(`headless-sims.yml`, `bench.yml`), and speed is only compared within one bench run.

## Commits (main)
- bd242f5 / b4f2c07: viewer zoom-out and fit-map, start markers drawn above the windows
  (b4f2c07 also undoes an accidental rustfmt of native; 8e124a7 does the same for civ-core)
- 10b178f, 4923dc5, 0371844: single-thread speed-ups (output identical)
- f6b783c: sparse `cell_first` table, save format 6, `v5_to_v6` migration, v0.6.0 fixture
- f261fac: version 0.6.1

## 1. Zoom out to the whole map (done)
- Before: the wheel zoom was clamped to 1 px/cell at minimum, so a 1650x1650 map could never be
  seen whole on a normal screen.
- Now: the minimum zoom is half of the "fit map" zoom (`View::min_zoom`), so any map up to
  2048x2048 fits with room to spare. The **Fit map** button, `Home` or `0` resets to the whole map.
- Under 1 px/cell the map texture uses linear filtering (no aliasing), and band dots are drawn
  cheaply when zoomed far out.
- The same code runs on the web build (checked with a headless Chrome screenshot of the
  0.6.1 web-dist).
- Screenshot: `docs/images/screenshot-zoomout-1650.png` (1650x1650 map, whole map in view).

## 2. Start-picker legend over lineage 1's marker (done)
- The start markers are now drawn after (above) the egui windows, so the Legend window no longer
  hides lineage 1's start marker.
- Screenshot: `docs/images/screenshot-start-picker.png`.

## 3. Single-thread overhead (done; output byte-identical)
Changes, all pure speed-ups:
- A fast deterministic hasher (FxHash-style `FastMap`) for the toolkit/registry/contacts maps and
  the merge grid.
- Allocation-free `mark_used`.
- Word-wise `ItemSet::iter`.
- A 1024-bit "may know" filter in `exchange`.
- A dense rank-indexed toolkit `via` table instead of a hash map.

Check: the `--save` files of the old and new binaries are byte-identical (seed 5 200 y 1 thread;
seed 2 150 y 4 threads), so balance cannot change.

`bench.yml`, seed 5, 300 years, GitHub runner. Each pair is from one bench run:

| run | binary | threads | wall s |
|---|---|---|---|
| 40195072042 | v0.6.0 | 1 | 42.80 |
| 40195072042 | v0.6.1 (0371844) | 1 | **32.08 (-25.0%)** |
| 40195067916 | v0.5.0 | 1 | 34.03 |
| 40195067916 | v0.6.1 (0371844) | 1 | 38.98 (+14.5% over v5; was +38 to 49% in v0.6.0) |
| 40195067916 | v0.6.1 | 2 | 25.89 (1.31x faster than v5) |
| 40195067916 | v0.6.1 | 4 | 21.87 (1.56x faster than v5) |

1-thread profile, v0.6.0 to v0.6.1 (run 40195072042):

| stage | v0.6.0 s | v0.6.1 s |
|---|---|---|
| band seasons | 26.9 | 18.9 |
| contacts | 12.1 | 10.7 |
| economy | 5.2 | 4.1 |
| experiments | 5.3 | 3.0 |
| movement | 4.7 | 4.0 |
| stats | 2.4 | 1.3 |

Both binaries ended at the same state (Y300 pop 148736, bands 7689).

## 4. Sparse first-reach (`cell_first`) table (done; save format 6)
- Before: one dense `u32` per map cell for every item ever held (10.9 MB per item at
  1650x1650, about 60 items).
- Now: `world::CellFirst` keeps 64x64 tiles, allocated only where the item has arrived. Saves
  store `cell_first_sparse` as `[item, {w, h, cells: [[cell, tick]]}]`.
- Format 6, with migration `v5_to_v6`. Binary format-5 saves load directly; the legacy dense
  field is converted in `after_load`. Details are in `docs/SAVE_FORMAT.md`.
- Tests:
  - New fixture `civ-core/tests/fixtures/v6_0_seed4_y150_64x40.civsave` (written by the v0.6.0
    release binary).
  - `v6_0_fixture_loads_sparse_cell_first` checks the table against the dense one in the file,
    checks that the JSON migration route gives the same table and state hash, and does a
    format-6 round trip.
  - `cell_first_sparse_basics`.
  - The v1 to v5beta fixtures still load.
- Memory, seed 3, 1650x1650, 100 years, 4 threads, on this box:

| | v0.6.0 | v0.6.1 |
|---|---|---|
| peak RSS | 1826 MB | **1022 MB (-44%)** |
| wall | 66.1 s | 63.1 s |
| save file | 67.45 MB | 66.99 MB |

- Loading that v0.6.0 save in v0.6.1 peaks at 1781 MB, because the old dense table is decoded
  once. Re-saving converts the file to format 6.

## Balance (headless-sims run 40196311987, f261fac, seeds 1-10 x 500 y)
Identical to v0.6.0, as expected for byte-identical output:

| metric | v6 | v6.1 |
|---|---|---|
| total pop Y500 | 928522 | 928522 |
| war %/yr | 0.347 | 0.347 |
| farming seeds | 9 | 9 |
| crossings | 4/10, none before Y100 | 4/10, none before Y100 |
| sim seconds (sum, 4 threads) | 448.5 | 391.5 |

Results: `docs/results-v6.1-seeds1-10-500y.txt`.

## Tests and release
- `cargo test --release -p civ-core` with and without `--features parallel`: all pass (13 and 12
  tests, 1 ignored as before). `-p kindling`: passes.
- Builds: Linux x86_64, Windows x86_64-pc-windows-gnu, web (`./build-web.sh`).
- Release: https://github.com/Macrolol/kindling/releases/tag/v0.6.1

## Open items
- One thread is still about 15% slower than v5. What is left:
  - movement's `cell_score` / `seen_logged`;
  - `regrow_cell` (the same arithmetic as v5);
  - `refresh_toolkit`;
  - contacts, whose cost grows with how tightly bands cluster.
  
  Each of these would need a balance re-check if it changed the output.
- Loading a v5 or v0.6.0 save of a huge map still briefly needs the dense table's memory.
