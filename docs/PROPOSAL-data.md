# Proposal: the data side of Kindling (live streaming, remote viewing, and the rest)

Status: research and proposal only. No sim code is changed by this document.
Base: `main` at 70260e9 (v0.8.0, save format 9).
Measurements: a scratch spike (`civ-core/examples/stream_spike.rs`, kept on the local branch
`spike/data-stream`, not on `main`) run on the CI 750-year save of seed 295068869. That save is
an 800×800 map at Y750, with 1.17 M people in 68.7 k bands. It was run on the box with 4 threads.

Michael's question: as the project grows, the data side matters more. Can events be streamed
live while a run is in progress, for example to watch a headless run remotely? And what else on
the data side should improve?

**Short answer: yes, it is feasible and cheap.**
- The sim already produces everything a stream needs, in a deterministic order: the typed events,
  per-year stats, counters, bands and per-cell layers.
- Building a compact frame of the whole world costs about **1.7 % of a tick** at 1 M people, even
  if done every tick.
- A full frame is under 1 MB compressed. Done once a year, that is about 70 KB/s, which any LAN or
  home connection carries easily.
- The streaming does not need to touch the save format or the RNG, so **saves and determinism are
  safe**.

The real work is elsewhere:
- the viewer, because the renderer reads `World` directly;
- the plumbing around runs (manifest, structured outputs, a results database), which pays off
  sooner than live viewing does.

---

## 1. What the data side looks like today

### What a run emits

| Output | Produced by | Form | Notes |
|---|---|---|---|
| Per-seed report | `native/src/headless.rs` | ~20 lines of free text per seed (headline, firsts, continents, sea, seafaring, lineages, ancestry, motives, refinement, prestige, trade, losses, discovery order, conflict, farming), plus an aggregate block with Kendall tau for multi-seed runs | Only at the **end** of each seed |
| Progress lines | `--report K` | One stderr line every K years (pop, bands, items, exp/inv/learn/forget) | The only thing available while a run is going |
| `--json` | `analysis::summarize` → `Summary` | One JSON object per seed **instead of** the text | Not the same content as the text (the text has more) |
| `.civsave` | `World::save_bytes` | MessagePack (named) + raw deflate, format 9; 43–147 MB at Y750 | Written only at the end (`--save` overwrites per seed). Saving 147 MB took 18.6 s; loading took 3.9 s |
| Profile | `KINDLING_PROF` | One stderr line | |
| CI artifacts | `.github/workflows/headless-sims.yml` | results txt and stderr (and saves, if asked) per matrix job, retained 14 days; binary 1 day; summary 14 days | The summary job concatenates the text and runs the parsers |
| Parsers | `tools/sum_results.py`, `agg_results.py`, `v7_metrics.py` | Regex over the text report | Every new report line needs a new regex |
| Diagnostics | 13 `civ-core/examples/*` (inspect_save, lineage_diag, longrun_report, popdiag, sea_gaps, …) and the local `kanal` tool | Load a full save, compute, print | Every question costs a 147 MB load |

### What lives inside `World`

- **Events.** `Event { tick, kind: u8, band, other, cell, item, recipe }` covers 34 `EventKind`s.
  - Each one goes through a single function, `World::push_event` (world.rs:675).
  - Major kinds (15 of them, such as firsts, landfall, bloodiest fight) are also kept forever in
    `chronicle`.
  - Everything goes into `events`, a rolling buffer of `EVENT_CAP = 40 000`.
- **`history: Vec<YearStats>`** holds one record per year with about 35 fields. For 751 years it
  is 95 KB as MessagePack.
- **`counters: Counters`** holds about 50 running totals (u64). They are summed from per-band
  effect buffers by a hand-written `add_fields!` list in `fx::add_counters`.
- **Firsts and lineage data:** `firsts`, `landfall`/`landfalls`, `cell_first`, `item_bands`,
  `item_peak`, `item_lost`, `farm_starts`.
- **Per-cell layers:** `plants`, `game`, `fish`, `small`, `crop`, `tend`, `dom`, `fert`, `camp`,
  `trade_flow`, `conflict_flow`, `camped`.
- **Bands:** a `Vec<Band>`, each with about 50 saved fields and the derived `d` (toolkit, kit,
  economy).

### Pain points

1. **Text is the interface.** Results are free text, read back with regexes in three Python
   scripts. A new metric means changes in three places: the report line, the parser and the
   aggregator. Small wording changes break the parsers silently. We hit this with tau, the
   continent lines and the "not made anywhere now" line.
2. **Nothing is available until the end.** If a run times out or crashes, its result is lost. The
   CI sim jobs have a 150-minute cap.
3. **Diagnostics need a full save.** A question like "how deep are fish stocks near camps?" means
   uploading, downloading and loading 147 MB, then writing a scratch example.
4. **Counters are added by hand.** A new `Counters` field must also go in `add_fields!`, or it
   silently stays 0 (REFACTOR-NOTES). Counters are totals only, with no per-year series. So
   anything not copied into `YearStats` by hand (refinements, marriages, crossings…) has no
   per-century curve.
5. **The rolling event log overflows within one season at scale (new finding).** The spike
   measured about **52 000 events per tick** at Y750:
   - Learned: about 25.6 k;
   - Forgot: about 25.6 k;
   - Fight, Fled, Split, Merged, Absorbed, Epidemic and the rest: about 1.7 k together.

   So `events` (cap 40 k) holds **less than one season**, almost all of it Learned/Forgot. This
   affects:
   - the UI's "all events" and "selected band" logs (`ui.rs` 236–240);
   - `longrun_report`'s fight list;
   - `lineage_diag`.

   These quietly see almost nothing on big worlds. The chronicle is fine because it is separate.
6. **No run metadata.** Results files do not record the commit, params, map, threads or
   timings in a machine-readable way. We reconstruct them from file names and workflow inputs.
7. **CI artifacts are bulky and short-lived.** One 147 MB save per seed. Results disappear after
   14 days unless copied into `docs/`.

---

## 2. Live streaming and remote viewing

### 2.1 Measured cost and size (spike, 1.17 M people, 800×800)

| Item | Measured |
|---|---|
| One tick (season) | 2.9–3.05 s |
| Building and encoding a full frame (all bands as compact rows + 3 u8 cell layers, MessagePack + deflate level 3) | 0.051 s, **1.7 % of a tick** |
| Band rows (id, x, y, pop, lineage, flags, items) for 68.7 k bands | 596 KB compressed |
| Three u8 cell layers (people, tended ground, trade flow) | 1.92 MB raw → 273 KB compressed |
| Delta over a year | 97 % of bands changed (mostly pop), so a band delta saves almost nothing (578 KB). The cell XOR delta saves about 10 % (243 KB). **Deltas are not worth it at first**; send keyframes |
| Events | ~52 k/tick, about 1 MB/tick as raw 20-byte records; 97 % Learned/Forgot |
| `history` (751 years) | 95 KB MessagePack, 69 KB compressed |
| Full save | 146 MB, 18.6 s |

**Scaling to a 1650×1650 map** (2.72 M cells, 4.25× the cells):
- Cell layers grow to about 1.2 MB compressed per keyframe (about 8 MB raw for 3 layers).
- Band rows scale with the number of bands, not with area. At ~1 M people they stay at about
  0.6 MB.
- So a full frame is about **2 MB**:
  - one per year → about 0.15 MB/s at 3 s/tick;
  - one per tick → about 0.6 MB/s (5 Mbit/s).
- Both are fine on a LAN. Once a year is fine over the internet.

**Bandwidth is a non-issue. The thing to control is the event volume.** Learned and Forgot
should go out as per-item counts per year, not as individual events.

### 2.2 Event stream from the core

- **Typed events already exist.** `push_event` is the one choke point, and it runs on the main
  thread in band order: the parallel phases buffer effects in `Fx` and apply them in order. So
  the event order is already deterministic.
- **Minimal civ-core change: an event tap.**

  ```rust
  /// When Some, every event is also appended here (drained by the caller after each step).
  #[serde(skip)] pub tap: Option<Vec<Event>>,
  ```

  - `push_event` adds one `if let Some(t) = &mut self.tap { t.push(e) }`.
  - It does nothing when off, so the GUI, the tests and determinism are unaffected.
  - It is `serde(skip)` and so never in a save. **There is no save format change.**
  - A `Vec` keeps `World: Clone + Default`, which a `Box<dyn Sink>` would break.
- **Snapshots are taken outside the step**, from `&World`, after `step()` returns. They read
  `bands`, the cell layers, `history.last()` and `counters`. They never touch the RNG and never
  write to the world. So a run with streaming on gives the same `state_hash` as one without; a
  test should assert this.
- **Counter deltas** come free by diffing `Counters` between frames (the spike does this).
  Per-year counter series follow directly, with no new `YearStats` fields.
- **Threading.** The sim thread builds the frame (about 50 ms) and hands the bytes to a writer or
  server thread through a bounded channel. If the channel is full, the frame is **dropped, never
  waited for**. The sim is never blocked by a slow viewer or a slow disk.

### 2.3 Transport

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| **Append to a stream file** (`run.kstream`) | No network code in the sim. Survives viewer disconnects. Doubles as the **replay file**. Works on CI (upload as artifact), on the box, and on Michael's machine | Not "live" across machines by itself | **Primary.** Every other transport reads from this |
| **SSE over `tiny_http`** (sync, small, no async runtime) | One-way push is all a viewer needs. Works in every browser with `EventSource`. Can also serve the static dashboard. A few hundred lines | Text framing (base64 or JSON for binary frames, or fetch binary frames by URL) | **Phase 2** live server, behind a `serve` feature in `native` only |
| WebSocket (`tungstenite` on a thread) | Two-way: inspect a band, pause, change overlay | More code; not needed for watching | Phase 3, only when the viewer gets interactive |
| `axum` / `tokio` | Full-featured | Pulls an async runtime and ~100 crates into a sim binary; longer CI builds | Not worth it at this scale |

### 2.4 Serialization

- **MessagePack (`rmp-serde`)**: already a dependency, self-describing, used by saves, and there
  are small JS decoders.
  - Use named maps for the header and the metrics records, so fields can be added freely.
  - Use tuples for band rows, for compactness.
- **Bincode or postcard**: a little smaller and faster, but not self-describing, so every schema
  change breaks old files. That is wrong for a replay format meant to last.
- **JSON / NDJSON**: for low-volume records (manifest, per-year metrics, major events), so they
  can be read with `jq`, Python or DuckDB without a decoder.
- **Compression:** deflate (`flate2`, already a dependency) per frame. zstd would be about 2× faster
  at similar size, but it is not worth a new C dependency yet.

**Proposed stream file layout:**
- A magic and version line, then a manifest record.
- Then length-prefixed records `[u32 len][u8 type][payload]`, with these types:
  - `Manifest`;
  - `Map` (static layers, once);
  - `Year` (YearStats + counter deltas + per-item learned/forgot counts);
  - `Events` (non-Learned/Forgot events for the period);
  - `Bands` (keyframe rows);
  - `Cells` (selected u8 layers);
  - `End` (summary JSON).
- Each record carries its own `schema` number, and readers skip unknown types. That gives
  forward and backward compatibility on the same rules as `SAVE_FORMAT.md`.

**Size of a replay:**
- Yearly keyframes for 750 years on 800×800 come to roughly 200–400 MB. Late years cost about
  0.9 MB; early years almost nothing.
- Keyframes every 10 years plus yearly metrics and events come to about 40–80 MB.
- Both are smaller than one Y750 save, and they hold the whole history instead of one moment.

### 2.5 Viewer

- **The renderer is tied to `World`.**
  - `render.rs` (`map_pixels`, `draw_world`, `pick_band`) and `ui.rs` take `&World`.
  - They read `map`, `bands` (including `d.tk.producible`), `tend`, `trade_flow`,
    `conflict_flow` and more.
  - The UI has about 40 `World` accesses, and the inspector reads band internals.
- A remote client therefore needs one of these:
  - **(a) A "shadow World"** filled from frames: map from the `Map` record, bands as thin
    `Band`s. It is quick to hack, but fragile, and the inspector would show defaults.
  - **(b) A `ViewModel` layer**: per-cell RGBA inputs and band rows. Both the local `World` and a
    remote or replay stream would produce it. This is the clean route, and it also lets the web
    build open a replay file. Estimate 3–5 days, with the band inspector fetched on demand over a
    WebSocket.
- **Lighter first viewer (recommended for phase 2): a small dashboard page** served by the same
  `tiny_http` thread. It shows:
  - the map as an image, rendered in the headless binary with the existing `map_pixels`. That
    function only returns RGBA bytes and needs no window. Encoded as PNG, it is about 0.1–0.3 MB a
    year;
  - charts of the per-year metrics;
  - a ticker of major and social events;
  - a run header (manifest, ETA).

  It needs no wasm and no renderer refactor, and it works the same for a live run or a replay
  file. It is about 1.5–2 days of work. The full wasm client can follow later if the dashboard
  proves useful.

### 2.6 Where runs happen, and how to watch them remotely

| Where | Live viewing | Notes |
|---|---|---|
| **Michael's machine or a LAN server** | Yes: `kindling --headless … --serve 0.0.0.0:8080`, open from any device on the LAN | The natural home for long runs. A self-hosted Actions runner here gets the same thing from CI |
| **The box** | Viewable in the box's own browser (I can watch). No public inbound port | For Michael to watch from outside, a tunnel (e.g. Tailscale) would be his call. Otherwise, use the replay file |
| **GitHub-hosted Actions** | **No live serving.** Actions runners are meant for building and testing the project. Exposing a server from a runner through a tunnel is a terms-of-service grey area, and jobs end within 6 h anyway | Use these instead: (1) richer `--report` lines, which already stream live in the Actions log; (2) the stream or replay file and a PNG timelapse uploaded as artifacts; (3) the per-run job summary. View them afterwards in the dashboard with `--replay run.kstream --serve` |
| Cloudflare or other hosting | On hold | The same SSE and static page would deploy later unchanged; R2-style object storage would be the natural home for replays |

### 2.7 Recommended architecture

```
           ┌───────────── civ-core (no I/O) ──────────────┐
 step() →  │ push_event ──► tap: Option<Vec<Event>>       │   (serde skip; no save change)
           │ history / counters / bands / layers (as now) │
           └──────────────┬───────────────────────────────┘
                          │ after each step (read-only):  civ_core::stream::Frame builders
                          ▼
   native headless: frame every N ticks ─► bounded channel (drop if full) ─► writer thread
                                                                │
                                         run.kstream (+ metrics.csv, events.ndjson, manifest.json)
                                                                │
                              optional `serve` feature: tiny_http thread tails the file
                                    ├── GET /            dashboard (static HTML + JS)
                                    ├── GET /events      SSE: Year / Events / "frame N ready"
                                    └── GET /frame/N     PNG map or MessagePack frame
   `kindling --replay run.kstream --serve` = the same server over a finished run
```

**Changes needed in civ-core:**
- the `tap` field and the one line in `push_event`;
- a new `stream` module with frame record types and builders (band rows, quantized layers,
  per-item learned/forgot counts);
- the `Counters` registry (section 3), so counter deltas and CSV headers are generated;
- a determinism test: same seed with and without the tap gives the same `state_hash`.

Nothing in `save.rs` changes.

### 2.8 Phases and effort

Effort is in focused agent-days, including tests and a short CI run.

| Phase | Content | Effort |
|---|---|---|
| **0. Foundations** | `--out DIR` per seed: `manifest.json`, `metrics.csv` (YearStats + counters per year), `summary.json` (with the fields that are only in the text today), `events-major.ndjson`. Counter registry macro. `docs/DATA_SCHEMA.md`. Event-buffer fix (section 3) | 1–1.5 |
| **1. Stream and replay file** | `tap`, `civ_core::stream`, writer thread, `run.kstream`, a Python reader in `tools/` | 1.5–2 |
| **2. Live dashboard** | `serve` feature (tiny_http + SSE), dashboard page, `--replay` mode, PNG map frames, CI uploads replay and timelapse | 1.5–2 |
| **3. Full remote viewer** (optional) | `ViewModel` refactor of render/ui, web build opens replay files or a live stream, WebSocket band inspector | 3–5 |
| **4. Results DB** | DuckDB ingest of manifests, metrics and summaries; SQL replacing `agg_results.py` / `v7_metrics.py`; CI summary from SQL; a notebook | 2–3 |

Phases 0–2 come to about **4–5.5 days** and deliver remote viewing of headless runs on the LAN, and
replay viewing of CI runs. Phase 0 is worth doing even if streaming is never built.

### 2.9 Risks

- **Overhead.** The measured cost is 1.7 % at 1 M people with a frame every tick; with yearly
  frames it is under 0.5 %. PNG encoding (phase 2) adds roughly 20–60 ms per frame. Use the
  channel's drop-if-full rule, and default to yearly frames.
- **Determinism.** The tap and the frame builders only read. They must not call anything that
  advances the RNG, or that recomputes and stores derived state (`after_load`,
  `refresh_toolkit`). The `state_hash` test guards this.
- **Save compatibility.** No saved field changes. The event-buffer fix changes what is *in* the
  saved `events` buffer, not its shape, so it is fully compatible both ways.
- **Stream format churn.** Per-record schema numbers and skip-unknown readers protect it, and a
  `DATA_SCHEMA.md` with additive-only rules like `SAVE_FORMAT.md`.
- **Event volume.** Learned/Forgot must be aggregated, or streams balloon to about 1 MB per
  tick.
- **Scope creep in the viewer.** Start with the dashboard and only refactor the renderer if it
  earns it.
- **Security** of `--serve 0.0.0.0`. It is read-only, has no file access beyond the run directory,
  and binds to localhost by default.

---

## 3. Other data improvements, by value against effort

| Rank | Improvement | Value | Effort | Notes |
|---|---|---|---|---|
| 1 | **Run manifest** (`manifest.json`: commit, version, save and schema versions, params, config, map, seeds, years, threads, host, start/end time, per-phase timings) | High: every result becomes self-describing and reproducible | S (hours) | Written at start and end, so a killed run still has one |
| 2 | **Structured per-year metrics** (`metrics.csv` or NDJSON per seed from `history` + counters, and `summary.json` with everything the text has) | Very high: ends regex parsing; per-century series for every counter; tau and discovery order become data | S–M (1 day) | CSV or NDJSON first. Parquet via the Rust `arrow`/`parquet` crates is too heavy a dependency; let DuckDB convert (`COPY … TO 'x.parquet'`) |
| 3 | **Fix the event buffer** (Learned/Forgot into per-year per-item counts, and a separate buffer of a few thousand recent social events, so the UI band log and diagnostics work on big worlds) | High; it fixes a silent UI and diagnostic gap | S | Content change only; save-compatible |
| 4 | **Counter registry** (one `counters! { … }` macro that declares the struct, the parallel `add`, names and iteration) | Medium–high: no more silent zeros; CSV headers and deltas come free | S–M | Settles the REFACTOR-NOTES `add_fields!` item |
| 5 | **Stable, versioned output schema** (`docs/DATA_SCHEMA.md`, `schema` in every file, additive-only rules) | High over time | S | Same discipline as `SAVE_FORMAT.md` |
| 6 | **Results DB** (DuckDB file built by `tools/ingest.py` from all run directories; SQL views for the comparisons `agg_results.py` does) | High once there are many runs; cross-version comparisons become one query | M (2 days) | DuckDB reads CSV and NDJSON directly. SQLite is fine too, but weaker for analytics |
| 7 | **Light analysis snapshots** (`--snapshot-every N`: band table + chosen cell layers, a few MB, in the stream or as Parquet via DuckDB) | High for diagnostics: most of the scratch examples and `kanal` questions would no longer need a 147 MB save | S–M (with phase 1) | The full save stays the way to *resume* |
| 8 | **Periodic checkpoints** (`--checkpoint-every Y`: a full save every Y years, overwriting) | Medium: protects long CI runs from the 150-minute cap | S | 18.6 s per save; every 100 years costs about 1.5 % at the Y750 scale |
| 9 | **Lineage and genealogy tables** (band birth/split/merge/death with parent, lineage, cell and tick, from the tap) | High for the culture, language and politics phases: trees of descent, diffusion maps, "who learned what from whom" | M | Needs the tap. Without it, the rolling buffer loses this |
| 10 | **CI artifacts** (separate artifacts for results, saves and streams; results kept 90 days, saves 3–7 days; manifest and summary always; the job summary built from SQL) | Medium; less loss and less storage | S | Saves stay opt-in (no user saves published) |
| 11 | **Notebook or dashboard over the DB** (pop, items, lineage and conflict curves across versions) | Medium | M | After 6; the phase 2 dashboard can share its charts |

**Recommended first slice:** items 1–5 together (phase 0, about 1.5 days). They remove most of
today's friction, and they are the base the stream builds on.

---

## 4. Open questions for Michael

1. Where will long runs and live viewing mostly happen: your machine, a LAN server, or the box?
   That decides whether `--serve` binds to the LAN by default, and whether a tunnel is wanted.
2. Is the light dashboard (map image, charts, event ticker) enough to start with, or is the full
   interactive viewer the goal?
3. Should replay files be uploaded by default from CI runs? They are smaller than saves, but not
   tiny: about 40–80 MB per 750-year seed with decade keyframes.
