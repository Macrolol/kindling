# Kindling v8: technologies for activities (the approved D proposal). Progress log

Started 25 Oct 2026, 11:50 EDT. Same rules as v7: one local sim at a time, RAYON_NUM_THREADS=4,
cargo -j4, every run under `timeout`. Each step is checked with 10 seeds x 500 years on GitHub
Actions (`headless-sims.yml`, 256x160 map) against `docs/results-v7-seeds1-10-500y.txt`.
Balance targets: pop within about 15% of 928,522; war 0.25-0.45%/yr; about 9 farming seeds;
crossings in 4-6 of 10 seeds and none before Y100.

Build order (from the design Page): 1 groundwork, 2 terrain and rivers, 3 fighting,
4 storage/salt/dairying/manuring, 5 seafaring, 6 trade carriers/ornaments/medicine, 7 riding.

## D1. Groundwork (done, commit 935aa5d, run 40208663447)
- Item set widened: 25 materials (new: brine, salt) and 25 forms (new: bow, shield, outrigger,
  sail, plank boat, carrying frame, ornament). `ItemSet` is now 16 words (up to 1024 items).
- Toolkit (`Kit`) gains slots for bow, shield, outrigger, sail, plank, frame and ornament. They
  are filled by `make_kit` but have no effects yet; later steps give them effects.
- Craft vectors: new materials get a neutral leaning of 1.0 (`band::fit_craft`), inserted before
  the three action leanings. World creation keeps the founders' random draws unchanged.
- Save format 8 with a frozen `v7_to_v8` migration (pads stock and origin to 25 materials, inserts
  neutral leanings). Format 5, 6 and 7 binaries still decode. New fixture
  `civ-core/tests/fixtures/v7_0_seed4_y150_96x60.civsave` (written by the v0.7.0 binary) and test
  `v7_0_fixture_gains_new_materials_in_place`.
- Results: `docs/results-v8-D1-seeds1-10-500y.txt`.

| metric | v7 | D1 |
|---|---|---|
| pop (10 seeds, Y500) | 918,614 | 876,123 |
| war %/yr | 0.363 | 0.363 |
| disease %/yr | 0.116 | 0.116 |
| farming seeds | 9 | 7 |
| crossings | 4/10 | 4/10 (Y125, Y133, Y226, Y261) |
| iron seeds (median) | 10 (Y253) | 6 (Y299) |
| bronze seeds (median) | 4 (Y339) | 6 (Y307) |
| tau | 0.137 | 0.139 |
| lineage max share, mean / worst | 27.7% / 42% | 29.1% / 45% |

The step adds no behaviour, but the extra neutral entries shift the craft-leaning normalisation
slightly, and the sim is chaotic, so individual seeds diverge (iron and farming are the seed-level
outcomes most sensitive to this). The war and disease rates are unchanged and all targets but
farming seeds are met. I'm tracking farming seeds over the next steps rather than tuning for it.

## D2. Terrain cost and river travel (done, commit 635f603, run 40209240376)
- `civ-core/src/world/terrain.rs`: the cost of a step between neighbouring cells is
  `1 + max(|climb| - 150 m, 0) / 400 m + max(height - 1500 m, 0) / 1000 m`, summed along the line
  between two cells. Ranges and high plateaus become barriers and passes matter.
- River systems (8-connected river cells) are labelled on load. A band with a boat on a river
  travels along the same river for a third of the land cost (at least 1), and can scout 2-6 cells
  along it.
- The cost shows up in moves (a mild effort penalty on candidate cells), meetings and teaching
  (`teach_rate x kin / (1 + cost)`), and partner visits (effective distance compared to reach;
  rivers and sea routes shorten it for bands with boats).
- Not built from the river section: rafts for heavy loads, fish weirs.
- Test `terrain_cost_and_rivers`.

| metric | v7 | D1 | D2 |
|---|---|---|---|
| pop | 918,614 | 876,123 | 840,996 (-9.4% vs target) |
| war %/yr | 0.363 | 0.363 | 0.353 |
| disease %/yr | 0.116 | 0.116 | 0.109 |
| farming seeds | 9 | 7 | 9 |
| crossings | 4/10 | 4/10 | 4/10 (Y132, Y138, Y205, Y371) |
| iron seeds (median) | 10 (Y253) | 6 (Y299) | 8 (Y230) |
| bronze seeds (median) | 4 (Y339) | 6 (Y307) | 6 (Y360) |
| tau | 0.137 | 0.139 | 0.138 |
| lineage max share, mean / worst | 27.7% / 42% | 29.1% / 45% | 28.5% / 42% |

Farming seeds are back to 9, which supports reading D1's dip as seed-level noise. The tau of
discovery order is unchanged: terrain slows contact but every world still finds things in a
similar order.
