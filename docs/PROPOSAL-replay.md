# Proposal: recording headless runs, and watching them live or as replays

Status: design, plus one measurement example the sim does not call (`civ-core/examples/replay_codec_experiment.rs`).
Base: `replay-proposal`, from `main` at 3b2a515.
Builds on [PROPOSAL-data.md](PROPOSAL-data.md) section 2 (live streaming and remote viewing) and section 4 (open questions). Sizes in §2–§7 are from the 26 Oct spike (`civ-core/examples/stream_spike.rs` on branch `spike/data-stream`, not re-run here): an 800×800 world at Y750, 1.17 M people, 68.7 k bands, 4 threads. Population figures for the two CI cases are from the skills step 3 reports (`docs/results-skills-step3-seeds1-10-500y.txt`, `docs/results-skills-step3-750y-800x800.txt`). §9 measures codecs on two bot saves: `main_seed6_y350_256x160.civsave` (256×160, Y350, 67,836 people) and `main_s499246296_y750_800x800.civsave` (800×800, Y750, 1,301,977 people, 72,169 bands).

Michael's question: watch a headless run while it is going (including one on another computer, or on GitHub Actions) and watch it again afterwards, in the native app and in the web build.

**Short answer.** Record a viewing file as the run goes. Open that file in the app you already have, native or web. Do not try to stream a save, and do not serve from a GitHub-hosted runner.

- One picture a year, plus that year's numbers and events, costs about **half a percent** of the run. The spike measured 1.7–2 % of a season to build and compress a full picture; a year is four seasons, and we take the picture once.
- The file is the replay. Tailing the same file is the live view. A small read-only server is optional and off unless asked.
- Saves stay the only way to resume. Format 16 is untouched (`civ-core/src/save.rs:22`). A recording cannot continue a world.
- On Actions, write the file inside the sim job that already exists. Decade pictures for an 800×800 batch; a picture every year is fine for 256×160. Download it when the job ends. To watch a runner live, use a self-hosted runner on your own machine.

---

## What this keeps, changes and adds

[PROPOSAL-data.md](PROPOSAL-data.md) §2 is the starting point. Its phase 0 has since landed (manifest, `metrics.csv`, `summary.json`, chronicle, history-store frames, the three event rings, the counter registry, `docs/DATA_SCHEMA.md`). This document does not redo that.

**Kept**

- The spike's costs and sizes. Building a picture was 1.7–2 % of a season. A late 800×800 picture was 596 KB of band rows and 273 KB of three `u8` layers. A whole-row band delta was 578 KB, because 97 % of bands changed, and a cell XOR saved about 10 %. §9 keeps that result: a whole-row delta is still the wrong codec. It does not keep the conclusion that every delta is the wrong codec.
- Learned and Forgot go out as per-item counts, never as one record per person. They were 97 % of ~52 k events per tick.
- Capture reads the world after `step()` returns. It does not touch the RNG. Streaming must not change `state_hash` or the save.
- The recording file is the transport. Everything else (a local watcher, a browser, another computer) reads that file. No Tokio, no axum.
- MessagePack (`rmp-serde`) and raw deflate (`flate2`, already a dependency, Rust backend, no new C library). Named maps for headers, tuples for band rows. JSON only for the manifest, so a person can see seed and commit without a decoder.
- GitHub-hosted runners are for the run, not for hosting a server. The Actions log already shows progress (`--progress-every`). The recording is how you watch afterwards.
- User saves are private and are never put in a workflow, an artifact or a release (`docs/CI-ARCHIVES.md`).

**Changed**

- The viewer is the existing map, not a separate dashboard. §2.5 recommended a chart page first and a real viewer later (its phase 3, "only if the dashboard earns it"). The thing you asked for is the native app and the web build, so that is the design. A chart page is not a step.
- A recording does not drop pictures when the disk is slow. §2.2 said drop a frame rather than wait. That is right for an optional live preview, and wrong for the file you will scrub later. The writer may stall the loop *between* steps. At one picture a year the stall does not happen: the disk sees well under 1 MB/s.
- No event tap in the first cut. §2.2's `tap: Option<Vec<Event>>` on `push_event` (`civ-core/src/world.rs:1084`) is still the right fix if a single year ever overflows the rings. It is not needed for a yearly recording, and it edits the same function the skills work is in. See §3 and §8.
- The picture is not a save blob. The resume format (`civ-core/src/codec.rs`, `BANDS_V` 3) is large, it changes when bands change, and copying it would make a recording about as big as a save. Skills step 4 does not change that save layout: the v16 belief block is already on the band row from step 2 (`BANDS_V` 3), and step 4 only fills it. Whether a recording's picture should carry that block is open question 6, not a settled change. New counters from step 4 still show up either way, because the year's numbers are copied by counter name.
- CI does not upload a yearly 800×800 recording by default. §2.4 guessed 40–80 MB per seed with decade pictures and suggested uploading them by default. Decade pictures are the right *opt-in* for the big batch. The default stays off, so a normal CI run does not grow.
- Band rows between keyframes are a per-field delta, not a fresh list every year. The data proposal rejected deltas because a whole-row delta did not shrink. §9 measures that again (it still does not) and measures a field delta that does, with no error in position or population. Layers stay whole pictures. Charts stay exact `YearStats`.

**Added**

- A versioned, append-only, seekable file (`.kndrec`), flushed every year, so a killed run still plays up to the last finished year.
- The same compatibility rule as saves: a version field, and old readers stay.
- Sizes for the two CI cases and for 1650×1650, and a viewer that says what it cannot show.
- A measured picture codec (§9), on both saves. Per-field band deltas in version 1. Layers left as deflated `u8` planes. No population quantisation in version 1: two significant figures are a no-op at this population, and an error-bounded quantiser that does drop small changes saves too little to be worth the error.
- How the web build opens a file, and how Actions stays inside the free plan's 20 concurrent jobs.

---

## 1. What exists today, and what is missing

### What a headless run already writes

`kindling --headless` (`native/src/headless.rs:9`) steps the world, then at the **end** of the seed writes the report. With `--out DIR` it also writes `DIR/seed-S/` (`native/src/headless.rs:587`, schema in `docs/DATA_SCHEMA.md`):

| file | useful for watching? |
|---|---|
| `manifest.json` | yes, as the run header. Written only at the end |
| `metrics.csv` | yes, the charts. One row per year, exact `YearStats` plus counter increases. End of run |
| `summary.json` | end-of-run totals. Not a timeline |
| `chronicle.ndjson` | the major events, in full. End of run |
| `frames.ndjson` | the history store (`civ-core/src/archive.rs`). Coarse: an 8×8 population grid, per-lineage and per-landmass rows, per-item learned/forgotten counts. **Not a map.** Years older than 100 are folded into decades (`RECENT_YEARS`, `archive.rs:105`) |
| `genealogy.csv` | only with `--out-genealogy` |
| `*.civsave` | only with `--save`. One moment, full fidelity, the way to resume. 80–150 MB at Y750 on 800×800. Not a movie |
| stdout progress | one line every 50 years (`--progress-every`). The only live signal. On Actions this is the log |

Nothing in that list is a picture of the map, nothing is flushed as the run goes, and nothing the app can scrub.

### What the app can draw

The desktop app and the web build are the same crate (`native/`, compiled to wasm by `build-web.sh`). The map is `map_pixels` / `draw_world` / `pick_band` (`native/src/render.rs:141`, `:340`, `:398`). All three take `&World`. They read:

- the map (biome, height, rivers, deposits, temperature) for terrain, deposits and temperature;
- band `x`, `y`, `pop`, `lineage`, `flame`, and `n_items()` for the discs and the know-how overlay;
- `tend`, `dom`, `fert`, `crop_pot` and a herd flag for farming;
- `trade_flow` and `conflict_flow` for those overlays;
- `cell_first` and `b.d.tk.producible` for "where this item has spread" (overlay 2).

The inspector (`native/src/ui.rs:339`) reads the band's recipes, stocks, kit, grudges, curiosity, stress and the rest. The charts read `w.history`.

The web page can download or pick a `.civsave` (`web/civ_io.js:15`, polled from `native/src/main.rs` around the wasm file load). It cannot pick a recording, because there is no such file. `quad-url.js` is copied into the web build and nothing reads it.

There is no `--record`, no `--replay`, no `--serve`. `native/src/cli.rs` stops at `--out`.

### What is missing

1. A picture per year (band discs + the cell layers the overlays need), written as the year closes, not at the end.
2. A container that can be tailed live, seeked later, and still opened in five years.
3. A path in the renderer that draws from that file as well as from a live `World`.
4. A way to point the web build at a file or a URL.
5. An Actions switch that turns (1) on without a new job, and without publishing a user save.

---

## 2. The recording

A recording is for **looking**. A save is for **continuing**. They do not share a version number or a reader.

| | save (`.civsave`) | recording (`.kndrec`) |
|---|---|---|
| question it answers | resume this world, bit-exact | what did this run look like, year by year |
| version | `save::FORMAT_VERSION` = 16 (`save.rs:22`) | `replay::REPLAY_VERSION`, starting at 1 |
| contains | every band field, stocks, recipes, RNG, full-precision layers | map once; a short row per band; a few quantized layers; the year's numbers; events |
| old files | always load (`docs/SAVE_FORMAT.md` rules 1–3; migrations in `save.rs:27` are never removed) | same rule: version field, old readers kept, a newer file than this build is refused with a clear message |
| user saves | private. Never an Actions input or a release asset | a recording of a CI seed is public, like the report. A recording made after `--load` of your save stays on your machine |

`frames.ndjson` is neither of these. It stays the analysis table it is.

### What goes in

**Once, at the start**

- Manifest, JSON, same facts as `civ_core::data::Manifest` (`civ-core/src/data.rs:15`): schema, commit, seed, map size, years asked, params, config, command line. Plus `replay_version`, `record_every` (years between pictures) and `loaded_from`. If `loaded_from` is set, this run continued a save. The file is still only a viewing file, and CI must not be given one of these (see §4).
- The map, once, as `codec::encode_map` (`civ-core/src/codec.rs:626`). That is the same blob the save uses, so terrain, rivers, deposits, height and temperature match the live app, including a legacy map that would not come back from "generate this seed again".

**Every year** (whether or not this year has a picture)

- The year's `YearStats` (exact, the same numbers as `metrics.csv`).
- That year's history-store `Frame` (`archive.rs:163`): counter increases, event counts by kind, lineages, landmasses, the 8×8 grid, per-item learned/forgotten, weddings between lineages. Copied out **before** the store folds the year into a decade. The save folds after 100 years; the recording does not. This is why a recording is a better timeline than a late save.
- Events that are not Learned or Forgot, for that year only. Learned and Forgot are the two counts on the `Frame`'s lore row (`LoreRow`, `archive.rs:122`: times learned, times forgotten, per item).
- The chronicle is not a second copy. It is those events whose kind is major (`EventKind::major`, `world.rs:279`), in order. The chronicle panel filters for them.

**On picture years** (default: every year)

- One row per band, and nothing else about the band:

```rust
/// A band as drawn. Not a save row: no recipes, stocks, beliefs, grudges or RNG.
#[derive(Clone, Copy, serde::Serialize)]
pub struct BandDot {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub pop: u32,
    pub lineage: u32,
    /// bit0 flame, bit1 sowing, bit2 has a boat, bit3 herding. Same bits the spike used.
    pub flags: u8,
    /// distinct things the band can make, saturating at 255 (`Band::n_items`)
    pub items: u8,
}
```

- Five `u8` cell layers, full map, named: `tend`, `dom`, `fert`, `trade`, `conflict`. Population and know-how are rebuilt from the dots, which is what `map_pixels` already does for overlays 1 and 3 (`render.rs:149`). The people layer the spike stored is left out on purpose.

Quantization is the spike's, so the colours match the ramps in `render.rs`:

```rust
fn q(v: f32, scale: f32) -> u8 { (v * scale).clamp(0.0, 255.0) as u8 }
// tend, dom, fert: q(value, 255.0)          // 0..1
// trade, conflict: q((1.0 + flow).ln(), 40.0)
```

The viewer divides back by the same scale. The picture matches; the float underneath does not. Charts and totals stay exact, because they come from `YearStats`, not from the layer.

**Not in the recording**

- Individual Learned / Forgot events (~25 k of each per tick at the spike).
- Recipes, stocks, kit contents, beliefs, grudges, who taught what. The inspector needs a `World` or a save for all of these. Beliefs are the open case: [PROPOSAL-skills.md](PROPOSAL-skills.md) §4 says the belief block should be in the stream frame layout from the start, and this draft leaves it out (question 6).
- `cell_first` / "this item's spread" (overlay 2). A producible bitset per band would be another ~0.4 MB a picture at late 800×800 (68 k bands × ~200 items). Later, if you want it, as its own record type. Not in version 1.
- Fish, game, plants, the RNG, and anything required to call `step()`.

### Container

Append-only. A reader can tail it, and a file cut off at a year boundary still plays.

```
offset 0
  8 bytes   magic  b"KNDREC\x01"
  u32 le    REPLAY_VERSION (1)
  u32 le    header bytes after this field (so a later header can grow)
then records, each:

  u32 le    payload_len
  u16 le    type
  u16 le    schema of this type (starts at 1; extra trailing fields do not bump it)
  u32 le    year (0 for manifest and map)
  u32 le    reserved
  [payload_len bytes]
```

| type | name | payload |
|---|---|---|
| 1 | Manifest | JSON, uncompressed, so `strings` shows the seed |
| 2 | Map | `encode_map` bytes, once |
| 3 | Year | deflate(MessagePack) of `YearStats` + `Frame` |
| 4 | Events | deflate(`codec::encode_events`) of this year's non-Learned/Forgot events, weddings included |
| 5 | Bands | picture years only. Deflate(MessagePack) of `[BandDot]` on the first picture, on every 10th picture, and whenever the previous picture is more than a year ago (so a CI decade file is all keyframes). Otherwise deflate of an id-sorted field delta from the previous picture (§9). Exact cells, exact population |
| 6 | Cells | deflate of `layers: u8`, then `w*h*layers` bytes. Not a delta and not a lossy transform (§9) |
| 7 | Index | one entry, appended when that year is flushed: `year u32`, `flags u16` (bit0 = has a picture), `offset u64` of the Year record |
| 8 | End | JSON one-liner: last year, bytes written. Absent while the run is alive |

The writer builds a record in memory and writes it with one `write_all` + `flush` per record. A reader that hits a short tail ignores that partial record and stops at the previous year. That is the crash story: kill the process, the file plays through the last flushed year.

Seek: on open, walk the 16-byte headers (skip `payload_len` bytes each time) and keep every Index entry. A 750-year file is a few thousand records. Scrubbing does not scan again. A live tail remembers the file position and reads only the new bytes.

Unknown type: skip `payload_len` bytes. That is how a new record is added without breaking old viewers, within a version they understand. A file whose header version is newer than this build is refused, with the version in the message, the same way a newer save is refused (`docs/SAVE_FORMAT.md` rule 3). Old readers stay in the tree under the version they implement, the same way `MIGRATIONS` is never deleted. A trailing field on a tuple (the `LoreRow` pattern, `archive.rs:127`) does not bump the version; changing what an existing field means does.

Compression is deflate level 3, per record, which is what the spike timed. zstd is already used on the CI *archive* (`zstd -19`). Putting zstd inside `.kndrec` as well would add a C dependency for almost no saving on bytes that are already deflated. Not in version 1.

```rust
// civ-core/src/replay.rs  (new; no I/O, no threads)
pub const REPLAY_VERSION: u32 = 1;
pub struct YearRec { pub stats: YearStats, pub frame: archive::Frame }
pub struct Capture {
    pub year: u32,
    pub year_rec: YearRec,
    pub events: Vec<Event>,   // Learned / Forgot already removed
    pub bands: Option<Vec<BandDot>>,
    pub cells: Option<CellPic>, // five u8 layers
}
/// Read-only. Does not call step, after_load, or anything that uses the RNG.
pub fn capture(w: &World, picture: bool) -> Capture { /* ... */ }
```

File I/O, the writer thread and the HTTP server live in `native/`, not in `civ-core`. The wasm build reads the same types.

### How big

The spike, late 800×800, one picture: **596 KB** of band rows + **273 KB** for three `u8` layers (1.92 MB raw), deflate level 3. This design drops the people layer and adds `dom`, `fert` and `conflict`. Same compression ratio gives about **0.40 MB** of cells instead of 0.27, so a late picture is about **1.0 MB** at 1.2 M people (0.60 bands + 0.40 cells). The year's numbers and the non-Learned/Forgot events add roughly **20–60 KB** at that scale (the spike's event mass was 97 % Learned/Forgot, which we do not store).

Band bytes follow population, not the map. Cell bytes follow the area. Early years are small; the file is dominated by the last two centuries. The averages below use that shape (from the century population rows in the step 3 reports) and round up.

| run | late picture | average year, picture every year | whole run, every year | whole run, picture every 10 years |
|---|---|---|---|---|
| **(a)** 256×160, 500 y, ~130 k people (seeds range ~56–258 k) | ~0.10 MB estimate at the fat end of that range. **Measured, seed 6 Y351, 67,836 people, 3,393 bands: 38,271 B** (23,343 B MessagePack bands + 14,928 B layers). **24,874 B** with the §9 field delta | ~40 KB estimate. The measured year is 38 KB, or 25 KB with the field delta | **about 15–40 MB** per seed, still an estimate. This one year sits under that pace; field deltas take about a third off the measured picture | **about 5–12 MB** per seed, estimate. Decade files are keyframes, so §9 does not shrink them |
| **(b)** 800×800, 750 y, ~1.3 M on the measured seed (reports span ~0.2–1.7 M) | **922 KB measured** at Y751 (639,927 B MessagePack bands + 281,603 B layers). **504 KB** with the §9 field delta (222,089 + 281,603) | the ten late years average **899 KB** for a full picture and about **504 KB** with the delta. A run-average over 750 years was not summed; early years are smaller | **200–350 MB** per seed, the planning range, not a 750-year sum. A year as fat as Y751 for all 750 years would be a ceiling of about 690 MB of keyframes or 380 MB of field-delta pictures | **40–80 MB** per seed. Decade files are keyframes, so the delta does not shrink them. 75 pictures at the measured 922 KB would be 69 MB; early pictures are smaller |
| 1650×1650, ~1 M people, as in the spike's scaling note | ~2 MB | — | ~2 MB × years × ~0.4 | decade pictures stay in the low hundreds of MB for 750 y |
| 1650×1650 if it ever fills to the same density as the spike (about 4× the people) | ~4–4.5 MB | — | roughly double the line above | still a viewing file, not a save |

Ten seeds of (a) at a picture every year: **about 0.2–0.4 GB**, an estimate. The measured seed-6 picture is 38 KB (25 KB with a field delta) at 68 k people, so a small seed pulls a ten-seed archive toward the bottom of that range, and field deltas pull it further. That is one year of one seed, not ten full runs. Ten seeds of (b) at a picture every year: **about 2–3.5 GB**, which is over the 2 GB release-asset cap and would be split (`headless-sims.yml` already splits at 2 GB). Ten seeds of (b) with decade pictures: **about 0.4–0.8 GB**, one asset, no split. Those two are still planning ranges, not sums of ten full runs. §9 now has the late picture: 922 KB as a keyframe, 504 KB with a field delta. Only the yearly file shrinks, and only in its band half. A decade file is keyframes.

A year's stats without a picture are a few kilobytes on (a) and around 10–20 KB on (b). Leaving them in on the decade setting is what makes the charts still move every year while the map holds still for ten.

Bandwidth the spike already stated, and it still holds: a 1650×1650 picture of about 2 MB, once a year, at about 3 s per season (12 s per year), is **about 0.15 MB/s**. One picture every season is **about 0.6 MB/s**. Both are comfortable on a LAN. Once a year is comfortable on a home uplink. Every season is the one that hurts a slow uplink, and it is not the default.

---

## 3. Where the picture is taken

`World::step` (`world.rs:1290`) is the season. On `tick % 4 == 0` it calls `year_stats`, which ends in `archive_year` (`world.rs:4539`) and `Archive::close_year` (`archive.rs:286`). That is the moment the year's `Frame`, the learned/forgotten counts and `history.last()` all exist. The headless loop is the right caller, after `step()` returns:

```rust
// native/src/headless.rs, the loop at line 33
w.step();
if recording && w.tick % 4 == 0 {
    let picture = w.year() as u32 % record_every == 0;
    let snap = civ_core::replay::capture(&w, picture); // &World, read-only
    tx.send(snap).expect("recorder");                   // stall here, never inside step
}
```

`capture` reads `bands`, the five layers, `history.last()`, `archive.frames.last()` (the year just closed) and the event rings. It does not call `after_load` or rebuild a toolkit.

**Learned / Forgot.** Already counted, per item, by `Archive::note` (`archive.rs:256`) into `tally.learned` / `tally.forgot`, then stored on the lore row in `close_year`. `capture` copies that row. It does not walk those events.

**Other events.** At the spike, the other 3 % was about 1.7 k events per tick, about 7 k per year. The conflict ring and the life ring each hold 40 000 (`RING_CAPS`, `archive.rs:31`), which is several years at that rate, so the year just closed is still in the rings. Weddings are not in the rings; they are in `archive.marriages` (`archive.rs:243`). `capture` copies both, for ticks in that year, and skips Learned / Forgot. If a future world exceeds 40 k non-learning events in one year, the year's *counts* in the `Frame` stay correct and the event *list* loses the oldest. That is the point where the tap from §2.2 of the data proposal is worth adding. Not before, and not while skills step 4 is editing `push_event`.

**Threading.** One bounded channel, capacity 2, to a writer thread that deflates and appends. The sim thread only builds the plain structs (the spike's 50 ms included deflate; building without deflate is less). The writer thread does the deflate, so the next season starts while last year's bytes hit the disk. Because we `send` rather than `try_send`, a stuck disk shows up as a pause between years, and the file never has a hole. A live preview of *extra* frames (if we ever take them) may use `try_send` and drop. The yearly picture may not.

**Determinism.** `capture` takes `&World`. The channel carries owned data. Nothing writes back. `state_hash` (`world.rs:4579`) is a weak check on its own: it mixes tick, RNG, band id, population, position, `known.len()` and the recipe count, and it ignores counters, layers and events. The test that matters compares a run that captures with a run that does not, on **both** `state_hash` and `save_bytes`. Those two saves must be byte-identical. Recording is not a save-format change.

**Default rate.** One picture per year (`--record-every 1`). That is one captured season in four, so about 2 % / 4 ≈ **0.5 %** of the run at the spike's scale, and less earlier, when a picture is small. `--record-every 10` is the CI setting for 800×800. A hidden `--record-seasons` (a picture every season, ~2 % and about four times the pictures) exists for a short clip where you want bands to move. It is not the default and not a CI option.

The desktop app can record too, from the same `capture` call after its own `step`. Same file. Not required for the first step.

---

## 4. Watching a run that is still going

The file is the stream. A watcher that reads only complete records (a header whose payload is fully present) never sees a torn year.

| where the run is | how you watch | live? |
|---|---|---|
| the machine you are sitting at | the app opens the path and tails it | yes, a year at a time |
| another computer you can SSH to | `ssh -L 8080:127.0.0.1:8080` and the optional local server, or copy the growing file | yes |
| a self-hosted Actions runner on that machine | same as a local run: `--record` and, if you want, `--serve` | yes |
| a GitHub-hosted runner | the log's progress line while it runs; download the `.kndrec` when the job ends (or when it is killed — the file is flushed) | no, and it should stay that way |
| the box this agent runs on | the box's own browser can open a finished file. No inbound port | after the run |

### The optional server

Off unless you pass `--serve`. Default bind is `127.0.0.1:8080`, not `0.0.0.0`. Binding every interface is a separate flag, `--serve-lan`, so a typo cannot open the port.

`tiny_http` on a thread, behind a Cargo feature so the wasm build does not link it. No async runtime. Read-only: GET only, no POST, no path that reads anything except this one recording.

```
GET /manifest
GET /map
GET /index              last flushed year, and each picture's offset
GET /year/750           Year record + events
GET /keyframe/750       bands + cells for the latest picture at or before that year
GET /events             Server-Sent Events: {"year": 750} when a year is flushed
```

A viewer that joins halfway fetches the manifest, the map and the latest picture (about 1 MB late in an 800×800 run, about 2 MB on 1650×1650), then follows `/events`. It does not download the whole history just to watch the live edge. Scrubbing backwards pulls older pictures on demand.

`--replay path.kndrec --serve` is the same server over a finished file, for when you want the page or the app on another device without copying the file first.

There is no relay in this design. A tunnel you start yourself (SSH, or Tailscale if you already use it) is enough, and it keeps the token out of the sim. A `--relay URL` that pushes bytes to a host you run is a later option; it needs a token, and the token must not be written into the manifest or the log. Not a step here.

### Security

- Nothing listens unless you ask. The default run writes a file and exits.
- The server cannot open a save, list a directory, change params, or pause the sim. Pausing would be a control channel. Leave it out, so a leaked port is a window, not a steering wheel.
- The recording's manifest stores the command line. Do not put secrets on that command line. When a relay exists later, strip the token before the manifest is written.
- **Your saves stay private.** CI never grows an input for a save path. `archive_saves` remains "CI-generated saves only" (`docs/CI-ARCHIVES.md`). A recording is a view of whatever world was simulated: if you `--load` a personal save and pass `--record`, that file describes your world. The headless runner should refuse `--record` together with `--load` when stdout is not a terminal and an env var `KINDLING_RECORD_LOADED=1` is unset, and print why. CI does not set that variable. On your own machine you can set it, and the manifest's `loaded_from` is the reminder not to upload the file.
- `--serve-lan` plus `loaded_from` prints a one-line warning. It still serves, because you are on your own LAN, but it should be obvious.

### Bandwidth

Following the live edge sends one new picture a year plus a few kilobytes of events. From §2, that is about 0.08 MB/s late in an 800×800 run (1 MB / 12 s) and about 0.15 MB/s for the 1650×1650 planning figure. Catch-up is one picture, not the whole file. The whole file is the replay download: 15–40 MB for a 256×160 seed, 40–80 MB for an 800×800 seed with decade pictures, 200–350 MB for an 800×800 seed with a picture every year.

---

## 5. The viewer

One draw path. The live world and a recording both fill it. `draw_world` stops taking `&World`.

```rust
// native/src/render.rs
pub struct BandDot { /* same fields as civ_core::replay::BandDot */ }

pub struct DrawIn<'a> {
    pub map: &'a civ_core::map::Map,
    pub tick: u32,
    pub tend: &'a [f32],
    pub dom: &'a [f32],
    pub fert: &'a [f32],
    pub trade_flow: &'a [f32],
    pub conflict_flow: &'a [f32],
    pub crop_pot: &'a [f32],          // derived from the map, once
    pub dots: &'a [BandDot],
    /// overlay 2 only. None in a recording: the UI says so, it does not draw a blank lie.
    pub item_spread: Option<ItemSpread<'a>>,
}

pub fn map_pixels(d: &DrawIn, overlay: u8, item: Option<Item>) -> Vec<u8>;
pub fn draw_world(d: &DrawIn, v: &View, mt: &MapTex, overlay: u8, item: Option<Item>, selected: Option<u32>, bands: bool);
pub fn pick_band(d: &DrawIn, v: &View, p: Vec2) -> Option<u32>;
```

A live `World` fills `DrawIn` with the slices it already owns. That change has to leave the picture identical; it is a refactor, not a new renderer. A recording fills `DrawIn` from the latest picture at or before the playhead, dequantizing the five layers with the scales in §2.

`MapTex` (`render.rs:291`) keeps its cache. The cache key becomes the playhead year plus the overlay, instead of `w.tick`.

### What you can do with a recording

| control | behaviour |
|---|---|
| play / pause | Space, as now (`main.rs` around the Space key). Replay mode does not call `step` |
| speed | the existing slider (`ui.rs:170`) is seasons/s for a live world. In replay it is **years/s**: 1, 4, 20, 100. Charts follow the cursor exactly. If inflate cannot keep up, the map keeps the last decoded picture and the cursor does not wait |
| scrub | a year slider from the manifest's start year to the last flushed year. Left/right move one year, Shift moves ten. Seek = the Index entry, then load that picture |
| follow live | if the playhead is on the last year, a new Index entry advances it. If you have scrubbed back, it stays |
| overlays 1, 3, 4, 5, 6, 7, 8 | work. 4 and 5 are the static map. 1 and 3 are rebuilt from the dots. 6, 7 and 8 use the quantized layers |
| overlay 2 (item spread) | disabled, with the line "item spread needs a save". No grey map pretending to be data |
| click a disc | a short card: id, lineage, population, cell, biome, fire, sowing, herd, boat, item count, and that band's events this year. Not the recipe list |
| charts | `YearStats` for every year, including years the map is holding a decade picture |
| chronicle | major events, in order, up to the playhead |

### What needs a full World

The inspector's recipe list, stocks, kit, grudges, beliefs, curiosity and stress (`ui.rs:339` onwards). Who taught a recipe. Continuing the run. Any edit to params. Overlay 2. Fish and game. Those stay on a live world or a loaded save. The card says so, in one line: "Recording. Recipes and beliefs are in a save, not here."

We do not build a shadow `World` and hope the inspector's defaults look harmless. A half-filled `Band` would lie.

### Web

Same binary. Two ways in, both end at the same bytes the desktop `--replay` path reads:

1. **File picker.** `web/civ_io.js:15` accepts `.civsave,.json`. Add `.kndrec`. `main.rs` already polls a picked file on wasm. Branch on the magic: `CIVSAVE\x01` loads a world, `KNDREC\x01` starts replay. The picker copies the whole file into wasm memory, which is what a save already does. A decade recording or a 256×160 yearly file (under about 80 MB) is fine. One 800×800 yearly file (200–350 MB) is heavy but within what a desktop browser will hold. Do not open all ten seeds at once; the archive is one file per seed.
2. **URL.** `quad-url.js` is already on the page (`web/index.html`, `build-web.sh`) and unused. Read `?replay=`. Fetch it in `civ_io.js` into the same buffer as the picker. This works for `--serve` on localhost, and for a host that sends CORS headers. A GitHub release asset does not. For a CI recording, download the file (or the `run-<id>` archive, `docs/CI-ARCHIVES.md`) and use the picker. Do not promise that a release URL will play inside the page.

There is no second renderer for the web.

---

## 6. GitHub Actions

Recording runs **inside the sim job**. It does not add a matrix entry. `sim` and `probe` are each capped at `max-parallel: 20` (`.github/workflows/headless-sims.yml:285` and the probe job). The free plan allows 20 concurrent jobs. A run with the probe on already queues behind that cap; recording must not add a third set of jobs. The plan job, the build, the summary and the archive are a handful more, and they do not overlap the whole matrix for long. Leave `max-parallel` where it is.

### Input

```yaml
record:
  description: "Viewing file per seed: off (default), decade, or year"
  required: false
  default: "off"
  type: string
```

`off` is the default, so current runs do not change size. The plan job rejects anything else with the same style of error it already uses for a bad seed range.

In the sim step, next to the existing `extra=(--years … --out out/data)`:

```yaml
# record = decade | year. Directory form: one file per seed, beside manifest.json.
if [ "$RECORD" = "decade" ]; then extra+=(--record out/data --record-every 10); fi
if [ "$RECORD" = "year" ]; then extra+=(--record out/data --record-every 1); fi
```

That writes `out/data/seed-S/recording.kndrec`. The artifact path `out/data/**` already uploads it (`headless-sims.yml` around the `results-*` upload, retention 30 days).

Which seeds, when you do turn it on:

| you want | inputs | recording |
|---|---|---|
| the usual early-game check | seeds `1..10`, 500 y, map `256x160` (or blank, the default) | `year`. About 0.2–0.4 GB for all ten, an estimate from §2. The measured seed-6 year is 38 KB (25 KB with the §9 delta), so expect the low side, lower again with field deltas. Still not a measured ten-seed archive |
| the usual late-game check | seeds `499246292..499246301`, 750 y, map `800x800` | `decade`. About 0.4–0.8 GB for all ten. Decade files are keyframes. The measured late keyframe is 922 KB (§9), so this range is no longer a spike guess; it is still not a sum of 75 pictures times ten seeds |
| one late world, properly | one of those seeds, 750 y, `800x800` | `year`. About 200–350 MB, one file you can scrub. At Y751 density a field delta cuts that year's picture from 922 KB to 504 KB. Do not multiply 504 KB by 750: early years are smaller, and every 10th picture is still a keyframe |

Do not record all ten late seeds at `year`. That is 2–3.5 GB, the release asset splits at 2 GB, and it crowds the storage below.

`seeds_per_job` stays 5. That is two sim jobs for a ten-seed batch, not ten.

### Minutes and storage

The picture costs about 0.5 % of sim time (§3). The step 3 late batch was about 290 minutes of sim across ten seeds (the slowest seed, 499246292, was 3063 s). Recording adds a couple of minutes to that batch, not a new job. Public repositories do not pay for GitHub-hosted minutes. The free plan's 2 000 private minutes are not the limit that matters here.

Storage is the limit. The release asset cap in this repo is 2 GB (`docs/CI-ARCHIVES.md`, and the split at `headless-sims.yml` near the `2000000000` check). GitHub's billing docs quote 500 MB of included Actions storage on the Free plan, then paid storage, but those docs describe billing for private repositories, and Kindling is public. Whether that 500 MB figure applies here still needs checking before it drives retention rules such as the 7-day cut. The rules below are sensible either way; they should not rest on an unchecked allowance.

Rules so a recorded run does not sit in two places at full size for a month:

- The `.kndrec` is in the job artifact, because the archive job downloads those artifacts, and because a cancelled job should still leave a file (`if: always()` is already on the upload).
- When `record=year` and the map is 800×800 or larger, that job's artifact retention drops from 30 days to 7. Decade recordings and 256×160 yearly recordings keep 30 days, like the reports.
- The pre-release `run-<id>` (default `archive: true`) is the long-lived copy. It already packs `out/data/**`. Deflate inside the recording plus `zstd -19` outside will not shrink the pictures much further; plan on the sizes in §2. Delete a test archive the way `CI-ARCHIVES.md` already says.
- Do not combine `record=year` on the late batch with `archive_saves`. Saves of those seeds are tens to hundreds of megabytes each, on top of the recordings.
- A recording is not a user save. The workflow grows no save-path input. `archive_saves` stays off unless you ask, and only for saves the sim just wrote.

### Watching an Actions run

Hosted runners have no inbound port, and standing a server up on one is not something this workflow should do. Two real options:

1. **After, or when the job stops.** The file is flushed every year, and the upload uses `if: always()`, so a timeout or a cancel still publishes the years that finished. Download `results-<seeds>` or the `run-<id>` release and open `seed-S/recording.kndrec` in the app. That is the path to use for GitHub-hosted runs.
2. **Live.** A self-hosted runner on your machine, `runner` input set to its label (the workflow already has that input). Then it is a local run: tail the file, or `--serve` on localhost. The 20-job cap still applies to hosted jobs; a self-hosted runner does not consume a hosted slot the same way, but do not raise `max-parallel`.

A mid-job re-upload of the growing file (`upload-artifact` with overwrite every N years) is not in the first cut. It doubles storage traffic for a lag of "however often we upload", and the flushed file at the end already covers the case where you want to see how far it got. The progress line gains the recording's size, so the log shows it is writing:

```
seed 499246292 Y 400 pop … | 1234.0s | recording 180 MB
```

---

## 7. Costs, risks, open questions

### Costs

| | every year (default on your machine) | every 10 years (CI late batch) | every season (opt-in clip) |
|---|---|---|---|
| CPU | ~0.5 % (one season in four, at the spike's 1.7–2 % per season) | ~0.05 % | ~2 % |
| memory | one picture in flight, twice (channel of 2). Under ~15 MB at 800×800, under ~40 MB at 1650×1650 | same, less often | same peak |
| disk, one (a) seed | 15–40 MB | 5–12 MB | ~4× the yearly file |
| disk, one (b) seed | 200–350 MB | 40–80 MB | ~4× |
| disk, ten (b) seeds | 2–3.5 GB. Do not do this on CI | 0.4–0.8 GB | no |
| bandwidth, following live | ~0.08 MB/s late (b), ~0.15 MB/s at 1650×1650 / ~1 M people | a picture one tenth as often; the year's numbers are tiny | ~0.6 MB/s at 1650×1650. Not the default |

PNG timelapse frames are not part of recording. `map_pixels` can make them later, in the viewer, from a picture it has already decoded. Encoding PNG on the sim thread (the data proposal's 20–60 ms) is wasted if the app is going to draw the picture anyway.

§9 is the codec measurement. It lowers the band half of a *yearly* picture (38,271 B → 24,874 B on the 256×160 year; 921,530 B → 503,692 B on the 800×800 year) and does not lower decade files, which store keyframes. The 800×800 disk cells above are that measurement, not the spike's guess. The 200–350 MB and 40–80 MB figures are still planning ranges for a whole run, not a sum.

### Risks

- **A recording that changes the run.** Mitigated by `&World`, no work inside `step`, and a test that `save_bytes` matches a run with recording off. `state_hash` alone is not enough (`world.rs:4579`).
- **The event list loses the start of a huge year.** Counts in the `Frame` do not. The tap is the fix, and it waits until skills step 4 is not editing `push_event` (§8, last step).
- **File size on CI.** Default off. Decade for the late batch. Retention cut for a large yearly file. The 2 GB asset split already exists; we should not need it for a recording.
- **Private saves.** Refusing `--record` after `--load` unless you override it. No new workflow input for a save. The server is read-only and localhost unless you ask.
- **The renderer refactor changes the live picture.** `DrawIn` for a live world is the same slices `map_pixels` reads today. Check with a screenshot of a known seed before and after, overlays 0, 1, 3, 6, 7 and 8.
- **Quantized layers.** Trade and conflict colours match the ramp; a tooltip that showed the raw float would be off. The recording UI should not show a fake precise float. Charts use `YearStats`.
- **Wasm and a 300 MB file.** Accepted for one seed. Not accepted as "open the whole ten-seed archive in the page".
- **Skills step 4** (beliefs, the kit decision, new counters; `docs/PROPOSAL-skills.md` step 4) edits `band.rs` and `world.rs`. The recording module only reads public fields that already exist (`id`, `x`, `y`, `pop`, `lineage`, `flame`, `sow`, `herd`, `n_items`, the layers, `Frame`, `YearStats`). New counters appear by name with no version bump. Step 4 does not change the save layout: the v16 belief block (`BANDS_V` 3) already exists from step 2 and is only filled, so there is no save-layout change that could invalidate recordings. This draft leaves the belief block out of the picture; that departs from PROPOSAL-skills §4 and is open question 6 for Michael. Do not merge the recording work into the skills PR, and do not add the `push_event` tap in that PR.
- **Scope.** The viewer is the app. No dashboard, no results database, no relay, in this design.

### Open questions

These replace the three in [PROPOSAL-data.md](PROPOSAL-data.md) §4. The first two of those are answered here unless you disagree: the viewer is the real app, and CI uploads a recording only when you set `record`.

1. **Live, day to day.** Is "the file on this machine, tail it" the common case, or do you want the localhost server (and an SSH tunnel) in the first slice? The file works either way. The server is a step of its own (§8, step 7).
2. **CI default, when you start using it.** This design says `off` until you pass `decade` or `year`, and `decade` for the 800×800 ten-seed batch. If you would rather have decade pictures on every late-game run without remembering the flag, say so. I would not turn `year` on for that batch.
3. **How smooth.** A picture a year means discs jump once a year. Is that enough to watch? `--record-seasons` is there if a short run needs smoother motion, at about 2 % and four times the file. I would not make it the default. §9 measured the other way to smooth a *decade* file (interpolate between keyframes, store a correction when the interpolation is outside a budget). Watchable corrections cost most of a yearly picture, so I would not do that in version 1 either.
4. **Item spread.** Overlay 2 stays save-only in version 1. Is that acceptable, or do you want a later record type with a producible bitset (roughly +0.4 MB on a late 800×800 picture)?
5. **Hosted-runner "near live".** Is download-when-the-job-finishes enough, with a self-hosted runner when you want true live? I recommend yes. A periodic re-upload from the hosted job is extra storage and still lags by whatever interval we pick.
6. **Beliefs in the picture.** [PROPOSAL-skills.md](PROPOSAL-skills.md) §4 says the belief block should be in the stream frame layout from the start (band layout 3, with the `use_skill` column and the belief block). This draft leaves beliefs out: the viewer does not draw them, they are large, and the save stays the resume format. That is a departure, not a consequence of step 4. The v16 belief block already exists (`BANDS_V` 3, from skills step 2); step 4 fills it and does not change the save layout. Do you want beliefs in recording version 1, or only in the save?

---

## 8. Steps

Each step is checkable on its own. Effort is S (a day or less), M (a few days), L (a week or more). None of these is L. Civ-core steps are read-only additions: they do not edit `step`, `Band`'s layout, or `push_event`, so they can land beside skills step 4 but should not be mixed into that PR.

| # | step | where | effort | done when |
|---|---|---|---|---|
| 1 | **Format.** `civ-core/src/replay.rs`: version, record header, writer/reader over an in-memory buffer, skip-unknown, refuse a newer version, old reader kept as `read_v1`. A synthetic recording round-trips. No world, no I/O | civ-core, new file plus `pub mod replay` in `lib.rs`. Does not touch the sim | S | a test builds a buffer, reads it back, and a second test shows an unknown type is skipped |
| 2 | **Capture.** `replay::capture(&World, picture)` as in §3. Test: 40 ticks of a tiny map, with and without `capture` after each year, `save_bytes` identical and `state_hash` identical | civ-core, read-only. Recompiles if step 4 adds fields; does not need to change unless a field it names is removed | S | the test passes, and a debug print of one tiny picture's byte size is in the test output |
| 3 | **Headless file.** `--record DIR`, `--record-every N`, writer thread, flush every year, size on the progress line. One seed, small map, local | native only (`cli.rs`, `headless.rs`, new `replay_io.rs`) | S–M | `seed-S/recording.kndrec` exists after a short run, plays back its manifest, and the run's save matches a run without `--record` |
| 4 | **DrawIn.** `map_pixels` / `draw_world` / `pick_band` take `DrawIn`. The live app fills it from `World`. No replay yet | native only (`render.rs`). The web build picks this up because it is the same crate | M | the live app looks the same on overlays 0, 1, 3, 6, 7, 8 (screenshot against a known seed) |
| 5 | **Replay in the app.** `--replay file`, scrub, speed, pause, short band card, charts, chronicle, "item spread needs a save". Tail a growing file | native (`main.rs`, `ui.rs`). No civ-core | M | open the file from step 3, scrub to a year, see discs and one overlay; Space pauses; the card does not show recipes |
| 6 | **Web.** `.kndrec` on the picker; magic branch; `?replay=` for a CORS or localhost URL. Note in `DATA_SCHEMA.md` or a short `docs/REPLAY.md` that release assets are opened via the picker | `web/civ_io.js`, the wasm branch in `main.rs` | S | the web build opens the same file as step 5 and scrubs |
| 7 | **CI switch.** the `record` input in §6, retention, the progress size already from step 3, a paragraph in `docs/CI-ARCHIVES.md`. No new job | workflow + docs | S | a dispatch with `record=year` on one small seed uploads `recording.kndrec`; a dispatch with `record` omitted uploads nothing new |
| 8 | **Local server.** `tiny_http`, feature-gated, localhost, the GET list in §4, `--replay --serve` for a finished file | native, feature off for wasm | M | from another device, an SSH tunnel and a browser (or the app pointed at the URL) follow a run that is still stepping. Nothing listens if the flag is absent |
| 9 | **Event tap, only if a year overflows the rings.** the `serde(skip)` buffer from the data proposal, drained at year close, still not saved | civ-core `world.rs` `push_event`. **Do this after skills step 4**, not during it | S | a test with the tap on and off has identical `save_bytes`. Not scheduled until the rings actually lose events |
| 10 | **Band field deltas.** The §9 codec, in the same change as step 1. Id-sorted per-field deltas for `BandDot` when the previous picture is last year. A MessagePack keyframe on the first picture, every 10 pictures, and whenever the gap is longer than a year (a decade recording is all keyframes). Layers stay deflate of the five `u8` planes. No population quantisation, no Haar or quadtree, no interpolated decade playback in version 1 | civ-core `replay.rs` | S | a one-year delta round-trips onto the previous dots with 0 cells of error and 0 population error, and the deflated delta is smaller than that year's keyframe. The example stays an example |

Steps 1–3 give you a file. Steps 4–6 let you watch it in the app and on the web. Step 7 is the Actions switch. Step 8 is live viewing from another device. Step 9 is deliberately last and maybe never. Step 10 is part of step 1's format, not a later pass: version 1 of the file should not freeze the "every picture is a MessagePack list" layout and then change it.

Suggested order against the skills work: 1, 2 and 10 can merge any time (they only add a module). 4, 5, 6 and 7 do not touch civ-core. 3 touches the headless runner only. None of 1–8 or 10 should be committed on the skills branch.

---

## 9. A Kindling-specific lossy codec for recordings

Viewing only. Charts and totals stay the year's `YearStats`. Saves are not touched, and a recording stays a versioned file of its own. The question is whether the picture itself can be smaller than "deflate the rows and the five `u8` planes", and what that does to what you see.

The codec was measured, not estimated, by `civ-core/examples/replay_codec_experiment.rs` (header comment: experiment for this section, not used by the sim). It loads a save, takes picture A, steps one year (4 ticks) for picture B, then nine more years for a decade gap. Compression is deflate level 3, the same setting §2 uses. Dependencies are the ones civ-core already has (`flate2`, `rmp-serde`). No new crate.

**Saves.** Both are bot saves, not Michael's. Format v16.

- `main_seed6_y350_256x160.civsave`. Map 256×160. Picture A is year 350: 3,354 bands, 67,836 people. Picture B is year 351: 3,393 bands, 68,795 people. One year of sim was 1,212 ms; the nine years after that were 8,745 ms. A later run for the quantisers in §9.5 reproduced every byte count below; that run's year was 1,033 ms and the nine after it were 9,612 ms.
- `main_s499246296_y750_800x800.civsave` (CI run 43021503049 on main). Map 800×800. Picture A is year 750: 72,169 bands, 1,301,977 people. Picture B is year 751: 72,328 bands, 1,308,981 people. One year of sim was 38,576 ms; the nine years after that were 369,605 ms (about 41 s a year). A full decade was cheap enough to step. The 800×800 tables are in the subsection after the 256×160 ones, and again as one block so the ratios are not copied across.

What changed from A to B, among the 3,344 bands in both pictures: position 64.9 %, population 65.6 %, lineage 0 %, flags 1.2 %, item-count 68.3 %. Births 49, deaths 10. Over the ten year-steps, 98.1 % of bands differed in some field. Mean population is about 20.

### 1. Bands as tracked entities

Positions are already cells (`u16`). Population quantisation is two significant figures, exact below 100, so a small change in a small band should round to nothing. Births and deaths are add and remove lists. Streams are id-sorted. Two codings of "delta":

- **Whole row**, which is what the spike tried. If anything in the row changed, store the whole row.
- **Per field.** Store only the fields that changed, as a mask plus those values. Adds and removes stay lists.

| method (picture B, deflate level 3) | bytes | raw | max position (cells, Chebyshev) | mean position (Euclidean) | max relative pop | mean relative pop | encode |
|---|---:|---:|---:|---:|---:|---:|---:|
| keyframe, packed 18 B/row | 27,162 | 61,074 | 0 | 0 | 0 | 0 | 1.8 ms |
| keyframe, MessagePack (the §2 layout) | 23,343 | 33,587 | 0 | 0 | 0 | 0 | 1.2 ms |
| whole-row delta, exact | 26,056 | 58,570 | 0 | 0 | 0 | 0 | 1.3 ms |
| whole-row delta, population quantised | 26,056 | 58,570 | 0 | 0 | 0 | 0 | 1.6 ms |
| field delta, exact | 9,946 | 15,102 | 0 | 0 | 0 | 0 | 0.5 ms |
| field delta, population quantised | 9,946 | 15,102 | 0 | 0 | 0 | 0 | 0.5 ms |

Mean over the ten year-steps: packed keyframe 27,013 B, whole-row delta 26,181 B, whole-row with quantised population 26,181 B, field delta 10,215 B. The share of bands that changed at all was 98.1 % exact and 98.1 % after quantisation.

Quantisation to two significant figures does **not** fix the spike. The spike's whole-row delta was 578 KB against a 596 KB keyframe, because almost every band's population moved. Here the same shape shows up at a smaller scale: 26,056 B against 27,162 B, about 4 % off. Two significant figures changed nothing we could measure. Every band in picture B was a fixed point of the quantiser (max relative error 0 on the quantised keyframe). The quantiser is exact below 100, and this world's bands sit there (mean population about 20, maximum 44), so a change of one person is still a change. Even if population had frozen, position moved on 65 % of bands and the item count on 68 %, so a whole-row rule would still resend the row. The 800×800 save is the same shape, measured, not guessed: mean population 18.0, maximum 47, and the quantised whole-row is byte-identical to the exact one (599,742 B). §9.5 measures quantisers that do drop a one-person change.

The field delta is the one that works, and it is lossless. 9,946 B, which is 43 % of the 23,343 B MessagePack keyframe and 37 % of the 27,162 B packed keyframe. A move of one cell is a zigzag integer, not a new row. Encode is half a millisecond. Error is 0 cells and 0 population, because nothing was quantised away. This is a delta from last year. A delta across a decade was not measured. With 98 % of bands changing every year, a ten-year gap should be treated as a keyframe until someone measures it.

### 2. Layers as smooth fields

Picture B, five planes (`tend`, `dom`, `fert`, `trade`, `conflict`), 256×160, raw 204,800 B. The baseline is those bytes deflated: **14,928 B**, max error 0, mean error 0, encode 0.7 ms. The planes are sparse or flat, which is why deflate is already this small: `tend` 9.5 % nonzero, `dom` 5.6 %, `trade` 11.7 %, `conflict` 1.9 %. `fert` is the odd one: 100 % nonzero, mean 247.5 out of 255, a nearly full field with a few lower cells, not a sparse mask.

| method | bytes | raw | max error (u8 steps) | mean error (u8 steps) | encode |
|---|---:|---:|---:|---:|---:|
| baseline, plain `u8` + deflate | 14,928 | 204,800 | 0 | 0 | 0.7 ms |
| 4×4 tile mean | 1,655 | 12,810 | 238 | 2.21 | 0.1 ms |
| 8×8 tile mean | 626 | 3,210 | 250 | 3.54 | 0.05 ms |
| 16×16 tile mean | 239 | 810 | 251 | 5.74 | <0.1 ms |
| quadtree, tolerance 0 (lossless) | 19,233 | 39,923 | 0 | 0 | 1.2 ms |
| quadtree, tolerance 4 | 16,959 | 33,661 | 4 | 0.017 | 1.0 ms |
| quadtree, tolerance 8 | 15,429 | 30,205 | 8 | 0.043 | 1.0 ms |
| quadtree, tolerance 16 | 13,145 | 25,535 | 14 | 0.108 | 0.7 ms |
| 8×8 Haar, Q=1 (lossless) | 28,042 | 41,664 | 0 | 0 | 1.2 ms |
| 8×8 Haar, Q=4 | 20,660 | 36,397 | 8 | 0.130 | 1.0 ms |
| 8×8 Haar, Q=8 | 16,438 | 31,839 | 15 | 0.207 | 0.6 ms |
| 8×8 Haar, Q=16 | 12,247 | 26,565 | 26 | 0.368 | 0.5 ms |

Haar is an integer Haar on 8×8 blocks (reversible at Q=1; the example checks that), then a uniform quantiser and a list of the nonzero coefficients. No extra dependency. Q=2 was also run: 25,033 B, max error 9, mean 0.091. It does not beat the rows above.

Coarse tiles are tiny and not usable. Max error near 250 is a hot cell and a zero cell averaged into one shade, painted across the whole tile. The mean looks small only because most cells are already the common value. That is the wrong picture of a camp, a road or a coast.

Within a max error of 8 levels, the smallest coding per layer was:

| layer | baseline | best at max ≤ 8 | max | mean | which |
|---|---:|---:|---:|---:|---|
| tend | 4,095 | 3,368 | 7 | 0.102 | quadtree T=8 |
| dom | 680 | 575 | 3 | 0.001 | quadtree T=8 |
| fert | 2,658 | 2,658 | 0 | 0 | baseline |
| trade | 5,363 | 5,363 | 0 | 0 | baseline |
| conflict | 1,569 | 1,532 | 7 | 0.018 | quadtree T=8 |

A few hundred bytes on the sparse planes, and the two planes that hold the detail (`fert`, `trade`) prefer plain deflate. Lossless quadtree and lossless Haar are both *larger* than the baseline (19.2 KB and 28.0 KB against 14.9 KB): the block or tree header costs more than LZ77 spends on a flat interior. The 800×800 planes are the same shape at a larger size (baseline 281,603 B). A uniform quadtree or Haar does not beat it inside a max error of 8. Picking the best coding per layer saves 20,438 B, almost all of it on `tend`. That is the measurement the expectation was waiting on.

### 3. Bits where the activity is

Tiles compared with the previous picture. Unchanged tiles (or tiles within a skip tolerance) are not sent; the viewer keeps the old tile. A tile that does change is sent whole, or as a mean, or as four quadrant means, if that stays inside a per-cell budget, otherwise whole. 16×16 over 256×160 is 160 tiles a layer, 800 tiles across five layers.

| method (B against A) | bytes | max u8 | mean u8 | encode | tiles sent | tiles skipped |
|---|---:|---:|---:|---:|---:|---:|
| baseline, whole planes | 14,928 | 0 | 0 | 0.7 ms | — | — |
| 16×16, exact, resend any change | 13,952 | 0 | 0 | 1.2 ms | 128 | 672 |
| 16×16, exact skip, LOD budget 8 | 13,839 | 7 | 0.001 | 1.0 ms | 128 | 672 |
| 16×16, skip if change ≤ 8, LOD budget 8 | 11,960 | 8 | 0.010 | 1.2 ms | 93 | 707 |
| 8×8, exact | 13,678 | 0 | 0 | 1.1 ms | 353 | 2,847 |

Across the decade gap (year 360 against year 350), exact 16×16 was 14,952 B and the tolerant one was 14,022 B, against a year-360 baseline of 15,904 B. Mean of the ten year-steps: baseline 15,530 B, exact 16×16 14,539 B, tolerant 13,148 B.

Skipping the quiet tiles does not beat deflate by enough to matter. 672 of 800 tiles were unchanged and the lossless coding only fell from 14,928 B to 13,952 B (6 %). Deflate was already not spending bits on the zeros. Allowing 8 levels of error on a quiet tile gets to 11,960 B, about 20 % off, for a codec the viewer has to keep in sync with the previous picture. Same story as the spike's cell XOR. On 800×800 the lossless 16×16 coding is 265,451 B against a 281,603 B baseline (6 % again; 10,353 of 12,500 tiles skipped). Not a second code path.

### 4. Interpolate between decade keyframes

Pictures at year 350 and year 360 are stored. Years 351–359 are predicted: position and population linearly interpolated for a band present at both ends, other fields taken from the nearer keyframe, births held at the end where they appear and deaths held at the start until they vanish. Layers are interpolated per cell. A correction is stored when the prediction misses the budget. Two budgets:

- **Viewing.** Position within 1 cell (Chebyshev), relative population within 10 %, each layer within 8 u8 steps.
- **Tight.** Position exact, relative population within 5 %, layers within 2.

Interpolation alone, with no corrections, is not watchable. On bands matched by id: max position error 17 cells in year 351, rising to 35 in year 359; mean position a few cells; max relative population from 2.0 to 7.5 (a band shown at several times its real size). Layers: max error 95–171 u8, mean about 0.5–0.9. Ghost bands (in the prediction, not in the year) 373–534. Missing bands 19–85.

Corrections bring it back inside the budget. The worst corrected year at the viewing budget was exactly on the budget: 1.0 cell, 0.100 relative population, 8 u8. Mean of the nine correction years:

| | band bytes | layer bytes | sum |
|---|---:|---:|---:|
| viewing budget, mean correction | 10,784 | 13,493 | 24,278 |
| tight budget, mean correction | — | — | 25,850 |
| one packed keyframe (year 360) | 25,862 | 15,904 | 41,766 |
| mean full picture, years 351–360 (packed bands + layers) | | | 42,543 |

Amortised, one keyframe plus nine viewing corrections, over the ten years: **26,027 B/year**. That is 61 % of storing every picture (42,543 B), and about **6×** a decade file that just holds the map still (the keyframe's 41,766 B spread over ten years, about 4.2 KB/year of picture). The tight budget barely costs more (25,850 B against 24,278 B): once you are correcting at all, most of the bytes are the bands and tiles that moved for real, not the ones sitting just outside a loose budget. Encode stays in the same 1 ms band as the other layer codings.

The band half of a correction (10.8 KB) is about the same size as an ordinary year-to-year field delta (10.2 KB). Interpolation is not buying band bytes. The layer half (13.5 KB) is most of a full layer picture (15.5 KB), because a linear blend of a coast is outside an 8-level budget on enough tiles that those tiles are sent anyway.

### 800×800, seed 499246296, year 750

Same example, same deflate level 3. Picture A is year 750 (72,169 bands, 1,301,977 people). Picture B is year 751 (72,328 bands, 1,308,981 people). One year was 38,576 ms; nine more were 369,605 ms. Among the 71,261 bands in both pictures: position 71.3 %, population 63.7 %, lineage 0 %, flags 2.1 %, item-count 66.2 %. Births 1,067, deaths 908. Over the ten year-steps, 98.1 % of bands differed in some field, before and after two-significant-figure population quantisation. Mean population is 18.0. The largest band in picture B has 47 people. Every band is under 100, so two significant figures are exact.

**Bands.** A MessagePack keyframe is slightly *larger* than the packed 18-byte row here (639,927 B against 618,165 B). On the 256×160 year it was the other way round. Three percent is not a reason to leave the §2 tuple layout. The field delta is 222,089 B, which is 35 % of that MessagePack keyframe and 36 % of the packed one. Encode of the deflate is 13 ms for the field delta and 32 ms for the keyframe.

| method (picture B, deflate level 3) | bytes | raw | max position (cells) | mean position | max relative pop | mean relative pop | encode |
|---|---:|---:|---:|---:|---:|---:|---:|
| keyframe, packed 18 B/row | 618,165 | 1,301,904 | 0 | 0 | 0 | 0 | 32.9 ms |
| keyframe, MessagePack | 639,927 | 1,108,351 | 0 | 0 | 0 | 0 | 32.4 ms |
| whole-row delta, exact | 599,742 | 1,261,034 | 0 | 0 | 0 | 0 | 31.1 ms |
| whole-row delta, population quantised | 599,742 | 1,261,034 | 0 | 0 | 0 | 0 | 31.3 ms |
| field delta, exact | 222,089 | 330,751 | 0 | 0 | 0 | 0 | 12.8 ms |
| field delta, population quantised | 222,089 | 330,751 | 0 | 0 | 0 | 0 | 14.1 ms |

Mean over the ten year-steps: packed keyframe 617,107 B, whole-row 598,567 B, whole-row quantised 598,567 B, field delta 222,143 B. Whole-row is 3 % under the packed keyframe. Same spike, larger numbers.

**Layers.** Five planes, 800×800, raw 3,200,000 B. Baseline **281,603 B**, error 0, encode 15.5 ms. Fill: `tend` 14.9 % nonzero, `dom` 6.4 %, `trade` 14.3 %, `conflict` 3.2 %, `fert` 100 % nonzero and mean 246.6. Same coast-and-ocean shape as the small map.

| method | bytes | raw | max error (u8) | mean error (u8) | encode |
|---|---:|---:|---:|---:|---:|
| baseline, plain `u8` + deflate | 281,603 | 3,200,000 | 0 | 0 | 15.5 ms |
| 4×4 tile mean | 27,561 | 200,015 | 238 | 2.87 | 1.2 ms |
| 8×8 tile mean | 8,710 | 50,010 | 250 | 4.46 | 0.4 ms |
| 16×16 tile mean | 2,784 | 12,510 | 253 | 6.85 | 0.2 ms |
| quadtree, tolerance 0 | 375,741 | 831,204 | 0 | 0 | 20.6 ms |
| quadtree, tolerance 4 | 322,685 | 667,101 | 4 | 0.024 | 17.5 ms |
| quadtree, tolerance 8 | 293,026 | 595,836 | 8 | 0.056 | 15.9 ms |
| quadtree, tolerance 16 | 248,250 | 496,381 | 16 | 0.141 | 15.5 ms |
| 8×8 Haar, Q=1 | 534,213 | 817,646 | 0 | 0 | 24.1 ms |
| 8×8 Haar, Q=4 | 378,733 | 693,828 | 10 | 0.165 | 19.3 ms |
| 8×8 Haar, Q=8 | 293,348 | 593,488 | 15 | 0.255 | 15.0 ms |
| 8×8 Haar, Q=16 | 214,915 | 485,538 | 29 | 0.463 | 11.2 ms |

Inside a max error of 8, applied to one layer at a time:

| layer | baseline | best at max ≤ 8 | max | mean | which |
|---|---:|---:|---:|---:|---|
| tend | 85,072 | 69,010 | 8 | 0.145 | quadtree T=8 |
| dom | 10,965 | 9,269 | 7 | 0.001 | quadtree T=8 |
| fert | 48,118 | 48,118 | 0 | 0 | baseline |
| trade | 100,536 | 100,536 | 0 | 0 | baseline |
| conflict | 34,232 | 34,232 | 0 | 0 | baseline |

The uniform quadtree at tolerance 8 is *larger* than the baseline (293,026 B against 281,603 B), because `fert` and `trade` get more expensive and they are most of the bytes. Per-layer selection saves 20,438 B, and 16,062 B of that is `tend`. Lossless quadtree and Haar are larger (376 KB and 534 KB). Not a second code path.

**Activity.** 16×16 over 800×800 is 2,500 tiles a layer, 12,500 across five layers.

| method (B against A) | bytes | max u8 | mean u8 | encode | tiles sent | tiles skipped |
|---|---:|---:|---:|---:|---:|---:|
| baseline, whole planes | 281,603 | 0 | 0 | 15.5 ms | — | — |
| 16×16, exact, resend any change | 265,451 | 0 | 0 | 27.3 ms | 2,147 | 10,353 |
| 16×16, exact skip, LOD budget 8 | 263,007 | 8 | 0.002 | 25.6 ms | 2,147 | 10,353 |
| 16×16, skip if change ≤ 8, LOD budget 8 | 230,298 | 8 | 0.004 | 22.8 ms | 1,636 | 10,864 |
| 8×8, exact | 261,685 | 0 | 0 | 27.0 ms | 6,802 | 43,198 |

Across the decade gap (year 760 against year 750), exact 16×16 was 267,131 B and the tolerant one was 237,592 B, against a year-760 layer baseline of 281,831 B. Mean of the ten year-steps: baseline 281,691 B, exact 16×16 265,660 B, tolerant 228,684 B. Lossless saving is 6 %, the same as the small map. The tolerant coding saves 18 % and needs the viewer to keep the previous picture. Not worth it.

**Decade interpolation.** Pictures at year 750 and year 760. Same two budgets as the small map. Interpolation alone is not watchable: on matched bands, max position error 37 cells in year 751 and 58 in year 759; max relative population 4.5 rising to 8.3; layers max 126–172 u8. Ghost bands about 10,000. Missing bands 281–916.

Corrections land on the budget (worst corrected year: 1.0 cell, 0.100 relative population, 8 u8). Mean of the nine correction years:

| | band bytes | layer bytes | sum |
|---|---:|---:|---:|
| viewing budget, mean correction | 231,261 | 233,090 | 464,351 |
| tight budget, mean correction | — | — | 498,301 |
| one packed keyframe (year 760) | 611,490 | 281,831 | 893,321 |
| mean full picture, years 751–760 (packed bands + layers) | | | 898,798 |

Amortised, one keyframe plus nine viewing corrections: **507,248 B/year**, 56 % of storing every picture, and about **5.7×** a decade file that holds the map still (893,321 B / 10 ≈ 89 KB/year). The band half of a correction (231 KB) is the whole of a year-to-year field delta (222 KB). Interpolation is still not buying band bytes. The tight budget is 498,301 B/year.

A late keyframe picture is 639,927 + 281,603 = **921,530 B**. The version-1 picture, field delta plus layers, is 222,089 + 281,603 = **503,692 B**. Encode of those two deflates is about 48 ms. The year of sim was 38,576 ms, so the picture is **0.12 %** of the year. The spike's half a percent still covers it.

### 5. Population quantisers that drop a small change

Two significant figures are exact below 100, and both saves live there (maximum band 44 and 47 in the two picture-B snapshots; 99.6 % and 99.7 % of bands are under 40). Build Bot's review is right: that quantiser was not tested. §9.5 adds two that do zero out a small change, and checks the error bound on every band. Both are still only in the example.

**Log buckets.** Relative step 5 % or 10 %, absolute floor ±2 people. Centers are spaced by twice `max(2, ceil(rel × center))`, so a reconstructed population is at most that far from the truth. A midpoint rounds down. The output is a fixed point (`q(q(p)) = q(p)`). Under about 40 people at 5 %, and under about 20 at 10 %, the floor wins and the grid is multiples of 4. That is almost every band in these saves. The 5 % and 10 % steps are only visible on the handful of bands above that.

**Dead-band.** The keyframe sends the exact population. A later year writes a band's population only when it differs from the **last sent** value by more than `max(2, ceil(rel × truth))`, and the value it writes is the exact population, not a bucket. The error cannot walk, because a year that would leave the ball around the last sent value sends again and the error goes back to zero. Comparing against last year's truth instead would let a change of one person a year drift with no bound. The example does not do that.

"Pop sent, survivors" is the share of bands present in both pictures whose population field is written. Births are always sent. "Pop sent, all" counts births as well, over the bands in the new picture. Whole-row bytes are the spike coding on the same shown populations: a row is still sent when position or the item count changes. Max absolute and max relative are the worst year. The other columns are the mean of the ten steps, and the send shares are totals over those steps.

256×160, mean of ten year-steps. One year of sim on the re-run was 1,033 ms. Encode of a field delta, including the round trip, is about 1 ms.

| method | field bytes | whole-row bytes | max abs (people) | mean abs | max rel | mean rel | pop sent, survivors | pop sent, all |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| exact | 10,215 | 26,181 | 0 | 0 | 0 | 0 | 67.8 % | 68.4 % |
| two significant figures | 10,215 | 26,181 | 0 | 0 | 0 | 0 | 67.8 % | 68.4 % |
| log 5 %, floor ±2 | 8,876 | 23,636 | 3 | 1.00 | 0.333 | 0.067 | 32.7 % | 33.8 % |
| log 10 %, floor ±2 | 8,776 | 23,279 | 4 | 1.15 | 0.333 | 0.072 | 29.6 % | 30.8 % |
| dead-band 5 %, floor ±2 | 8,776 | 24,551 | 3 | 0.72 | 0.500 | 0.047 | 24.3 % | 25.6 % |
| dead-band 10 %, floor ±2 | 8,577 | 24,259 | 5 | 0.95 | 0.500 | 0.055 | 19.9 % | 21.2 % |

800×800, mean of ten year-steps. Encode is about 37 ms, which is 0.10 % of the 38,576 ms year.

| method | field bytes | whole-row bytes | max abs (people) | mean abs | max rel | mean rel | pop sent, survivors | pop sent, all |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| exact | 222,143 | 598,567 | 0 | 0 | 0 | 0 | 64.4 % | 65.0 % |
| two significant figures | 222,143 | 598,567 | 0 | 0 | 0 | 0 | 64.4 % | 65.0 % |
| log 5 %, floor ±2 | 193,220 | 558,800 | 4 | 1.00 | 0.333 | 0.070 | 28.5 % | 29.6 % |
| log 10 %, floor ±2 | 191,233 | 552,836 | 6 | 1.12 | 0.333 | 0.074 | 25.9 % | 27.1 % |
| dead-band 5 %, floor ±2 | 190,924 | 567,164 | 3 | 0.75 | 0.500 | 0.052 | 20.1 % | 21.4 % |
| dead-band 10 %, floor ±2 | 187,414 | 562,328 | 5 | 0.94 | 0.500 | 0.059 | 16.5 % | 17.8 % |

The first dead-band year sends less, because nothing has drifted out to the threshold yet. On 800×800, year 751 against 750: the 5 % dead-band writes 12.3 % of surviving populations (184,241 B, max abs 3, mean abs 0.65, max rel 0.50, mean rel 0.041) and the 10 % dead-band writes 8.5 % (180,005 B, max abs 5, mean abs 0.77). By the later years the share has settled at the mean above. The worst absolute error does not rise from year to year. The run aborts if any band leaves `max(2, ceil(rel × truth))`. None did, on either save, for ten years.

The relative error of 0.33 or 0.50 is the floor, not a leak. An error of two people on a band of four to six is a third or a half. The 5 % and 10 % steps barely differ, because almost every band is in the ±2 regime. Mean absolute error is about one person.

What the bytes do to a picture, 800×800, using the ten-year means: the field delta falls from 222 KB to 187 KB at the looser dead-band (16 % off the delta). Layers stay 282 KB. The picture falls from about 504 KB to about 469 KB, which is 7 %. The whole-row coding falls from 599 KB to 562 KB and is still within 9 % of the packed keyframe. Dropping small population changes does not rescue a whole-row delta while position and the item count keep moving on about 70 % of bands.

### Recommendation

**Version 1, adopt**

- Per-field band deltas, id-sorted, with add and remove lists, from the previous *year's* picture. Lossless: 0 cells, 0 population error. A MessagePack keyframe on the first picture, every 10 pictures, and any time the previous picture is more than a year back. A CI decade file (`--record-every 10`) is therefore all keyframes. The 35 % and 43 % figures are year-to-year measurements; a single delta spanning a decade was not measured, and with 98 % of bands changing every year it should stay a keyframe. Scrubbing to a keyframe does not apply a chain of deltas. Measured 9,946 B against 23,343 B on the 256×160 year, and 222,089 B against 639,927 B on the 800×800 year.
- Layers unchanged: five `u8` planes, deflate level 3, error 0 beyond the quantisation §2 already specifies (the ramp the viewer divides back by). Charts stay `YearStats`.

**Version 1, do not adopt**

- Population quantised to two significant figures. Measured saving: none, on both saves. Max population in the picture-B snapshots is 44 and 47, so the quantiser never moved a value.
- Log buckets and a dead-band tracker, at 5 % and 10 % with a ±2 floor. These do drop small changes. The dead-band is the better of the two: it sends the exact population, the error stays inside the bound for all ten years (it does not drift), and about 16–20 % of surviving bands send a population in the steady state, against 64 % exact. The saving is about 35 KB on a 222 KB field delta, and about 35 KB on a 504 KB picture, because the layers do not shrink. Max relative error is 0.5 on a tiny band, which is the floor doing what it was asked. Not worth a wrong disc in version 1. If a later version wants it, use the dead-band, not two significant figures and not the log buckets.
- Whole-row deltas. 26,056 B against 27,162 B on the small map. 599,742 B against 618,165 B packed, and against 639,927 B MessagePack, on 800×800. The spike's 578 KB against 596 KB, again. Quantising population does not close that gap.
- Tile means, quadtree, Haar. A max error of 8 is not enough to beat plain deflate on either map. On 800×800 a uniform quadtree at tolerance 8 is larger than the baseline, and the best per-layer choice saves 20,438 B of 281,603 B. The cheap tiles (max error ~250) are the wrong coastline.
- Activity refresh. Lossless saving 6 % on both maps (13,952 B against 14,928 B; 265,451 B against 281,603 B). With an 8-level budget, 11,960 B and 230,298 B. Not worth the state the viewer would have to keep.
- Decade interpolation. Same answer at both sizes. Watchable corrections cost 24,278 B/year on the small map and 464,351 B/year on 800×800, which is 56–61 % of storing every picture and about 6× a still decade picture. The tight budget barely costs more.

**Already measured, and still not a change of plan**

- Haar or a quadtree, now that the 800×800 layers exist. They are still a flat interior and a sharp coast. Not in version 1.
- A coarser population step. It was measured (§9.5). The bands are small enough that two significant figures are a no-op, and the step that does bite (±2, or 5–10 % once a band is large enough) saves about 7 % of a late picture. I would spend that budget on nothing: the lossless field delta is the codec.

**What this does to the CI sizes**

- **(a) yearly, 256×160.** Measured picture, this seed and year: 38,271 B baseline (MessagePack bands + layers), 24,874 B with the version-1 delta. The §2 whole-run **15–40 MB** and the ten-seed **0.2–0.4 GB** stay estimates. This year is under the pace those ranges assumed, and the delta takes about a third off the picture, so a recorded (a) batch should come in at the low end, somewhat lower with the delta. Not a substitute for summing 500 years.
- **(a) decade.** Unchanged by the codec. Keyframes only. The §2 **5–12 MB** per seed stays an estimate; the measured keyframe is 38 KB, so a seed this size is toward the bottom of that range or under it.
- **(b) yearly, 800×800.** The late picture is measured: **922 KB** as a MessagePack keyframe plus layers, **504 KB** with the field delta. The §2 **200–350 MB** per seed stays the planning range for a whole run. It is not 504 KB × 750. Early years are smaller, and every 10th picture is a keyframe. A year as fat as Y751 for all 750 years would be a ceiling of about 690 MB of keyframes or 380 MB of field-delta pictures.
- **(b) decade.** The codec does not move it. Keyframes only. The measured late keyframe is 922 KB, so 75 of those would be 69 MB and the real file is less. The §2 **40–80 MB** per seed, **0.4–0.8 GB** for ten, stays the number to plan on.

CPU is not the constraint on either map. The slowest record deflate on the 800×800 picture is the MessagePack keyframe at 32 ms, plus 16 ms for the layers, about 48 ms all together. The year of sim was 38,576 ms. That is 0.12 % of the year. The spike's 0.5 % of the run (§3) still covers version 1, and a field delta is less work than deflating a keyframe, not more.
