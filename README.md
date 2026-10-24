# Kindling (working name)

A living-civilisation simulator in Rust. No tech tree is written down anywhere: items, their properties and what can be made from them come out of a small material-physics model (hardness, fracture, sharpness, fuel temperature, refractoriness, ore reduction temperature…). Bands of foragers experiment with what is around them, keep what helps, teach neighbours and children, and forget what they stop using.

Stage 1 prototype: map + resources, foraging bands that move, split and die, fire, emergent recipe discovery, teaching/copying/forgetting, a native viewer, a headless multi-seed runner and versioned saves.

## Layout
- `civ-core/`: the simulation (no graphics). Includes rng, materials, physics (discovery rules), knowledge, map, params, band, world, save, analysis.
- `native/`: the `kindling` binary. It has the macroquad + egui viewer and the headless mode, and builds for native and wasm.
- `build-web.sh`: wasm build into `web-dist/` (+ zip).
- `dist/`: prebuilt Linux / Windows binaries, web zip, screenshots.
- `docs/SAVE_FORMAT.md`: save format and migration rules. `docs/results-*.txt`: raw multi-seed output.

## Running
```
kindling                                   # viewer, random seed
kindling --seed 7                          # viewer, fixed seed
kindling --load saves/x.civsave            # resume a save
kindling --headless --seeds 1..8 --years 500   # multi-seed stats, no window
kindling --seed 1 --screenshot out.png --years 300 [--overlay 0-6] [--map WxH (default 256x160)] [--ui none|inspector,log,stats,settings]
```
Viewer keys: Space pause, +/- speed, F follow, 1–7 overlays (terrain, know-how, item spread, population, deposits, temperature, trade), Ctrl+S save, Ctrl+O load. Wheel zooms, drag pans, click selects a band.

Settings can be changed live in the Settings window. Each change is recorded in the world's settings log, which is saved with the world.

## Building
```
cargo build --release -p kindling                                       # Linux
cargo build --release -p kindling --target x86_64-pc-windows-gnu        # Windows
./build-web.sh                                                          # Web
cargo test -p civ-core
```
