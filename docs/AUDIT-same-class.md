# Audit: formulas that flatten or reverse a real-world ordering

No simulation behaviour was changed. The only code added is ignored diagnostic tests in `civ-core/src/tests_audit.rs`. Golden hashes are untouched. A separate stream is fixing durability; that bug is listed here as the exemplar, not re-fixed. Material sizes are flagged for a later pass and are not retuned.

## Summary

Twenty-one findings. Two are high, nine are medium, ten are low. The material findings from the first pass stand. The bounds were measured on two saved worlds, seed 6 at year 350 and seed 1 at year 200, instead of judging clamps from the formula alone. The sizing pass measured material steps from the 0..10 property through to the food, fight and survival the sim applies. The protection cap is ranked above births and push: it binds more widely on the earlier save, and the warmth finding depends on it. The hunt finding is low, by this audit's own definition. The boat hazard floor is a late-game description and stays medium.

The high one is the bug Michael already found. `durability` puts copper, bronze and iron on the same ceiling of 1, and steel just under it (0.979), because the formula pays for malleability and then divides by 12. Wear is `1.15 - durability`, so those three metals all wear at 0.150 and steel wears faster, at 0.171. Fight order is not reversed: a steel spear is still the deadliest weapon in the table (weapon quality 0.915, then bronze 0.875, iron 0.871, copper 0.814, flint 0.707). What the cap does change is upkeep, and therefore which tool a band keeps when the food gain is small. On a nominal band, forcing every axe to chain depth 8 makes bronze's score 0.319 and steel's 0.318. The axe study already measured the larger effect, which is chain depth, not this gap. On the saved world the depth term is doing that job: a copper axe sits near chain depth 3.2 and a steel axe near 7.3, so steel's labour is about 1.6 times copper's before the wear reversal.

The other high one is the size, not the order. Felling trials put a steel axe several times ahead of a stone axe, not a tenth. They do not put steel several times ahead of bronze: Mathieu and Meyer's bronze axes felled about as fast as steel at every trunk diameter, and only stone fell behind as the trees got thicker. The large step is from stone, or from soft copper, up to a good hard metal. Steel against copper in the sim is 2× on the hardness property and 1.63× on `cut_q`. A work-hardened copper edge is roughly 100–130 HV, more if the copper is arsenical, against a historical steel edge of a few hundred HV, about 3–6×. About 15× is only an outer bound, annealed copper against a modern quenched edge, and it is not the calorie target. The sim then multiplies the axe by 0.1 on plants and 0.2 on the farm. With a digging stick and a hammer already in the kit, a steel axe gathers 1.009× the plants of a copper axe, and 1.119× the farm yield. A steel spear makes a band 1.068× as strong. Chain depth 8 multiplies upkeep by 3.4 for every metal. On the nominal mix that depth swing is worth more food than swapping copper for steel. Iron against bronze stays inside 1%, which is fair for unsteeled iron and a good bronze, and should not be "fixed" into a large step. Section 18 has the pairs. Nothing was retuned.

The other material findings are the same kind of mistake.

- Woollen clothing and hide clothing both score 1.0 for warmth, so a band that can sew both always wears hide. Linen is correctly cooler (0.55). That tie matters twice, because the protection cap already holds 51.16% of bands at year 350 and 69.75% at year 200. A later warmth split is shaved back off any band that also has fire, a kiln and an axe. The warmth finding depends on clearing that cap. The cap is ranked above births and push.
- A rawhide net outscores a bast net (7.55 against 7.40 on the nominal score) and a wool net sits between them. All three have the same mesh quality, 0.800, and hide then wins on wear. Linen, once it exists, beats all three (8.79).
- A dugout and a skin boat have the same seaworthiness, 0.929. The first pass said an experienced crew then hits one survival floor, and that rounding puts every hull in one fishing radius. On the year-350 save it is not only veterans. Hull quality is already 1 for the median boat, 98.8% carry an outrigger, and 98.6% of boats sit on the hazard floor of 0.3, including 1,164 of 1,178 crews with seacraft under 0.25. Fish radius is 3 for 3,296 of 3,333 boats. That is the late game. On the year-200 save, 0 of 1,294 boats are on the hazard floor, nobody has an outrigger, and the radius splits 735 boats on radius 2 against 559 on radius 3, so the rounding is a cliff at quality 0.75 rather than one bin for everyone. The rank stays medium, and it is marked as late for the floor. How far a voyage goes is not flattened: `sea_range` on the late save still runs from about 10 to 20 cells.
- Ornaments are ranked by hardness. An obsidian pendant (6.5) beats a bronze ring (6), and bone beads (5) beat a copper ring (4). Gold is exempt, because prized goods get a flat +10.
- A hunting spear is scored on cutting quality alone. Obsidian (cut 0.820) therefore beats steel (0.780) when both chains are depth 0 (slot score 16.868 against 16.856). At depth 8 steel is ahead. The order is wrong only at depth 0, and other terms already carry the usual keeper, so this is low. On the year-350 save 3,255 hunts are steel; the only other materials with more than 20 hunts are wood (22) and copper (45). On the year-200 save the hunts are flint 551, bone 449 and wood 295, and steel is not there. The reversal is latent until a sharp stone and a shallow steel chain are both in reach.

What the measurement adds, outside materials:

- Births do not see surplus. The food term is 1 for 98.78% of bands at year 350 and 0 for 14 bands (0.42%). The ramp from 75% of need up to a full meal holds 0.81%. At year 200 the food term is 1 for 89.12% and 0 for 6.87%. That is still most bands, so the rank holds. Stored food is not destroyed (nobody is on the store cap; a person can keep about 2.2 seasons on the late save), and it still does not buy a child. Spare labour and settlement still move fertility, from about 0.86 to 1.01 across the middle 80% of the well-fed. Bands are not identical. They also do not grow faster for being richer in food.
- Leaving home is capped at 1, and the raw push is already at least 1 for 46% of bands at year 350 (median 0.78, 90th percentile 6.2, maximum 22). At year 200 it is at least 1 for 63.81% (median 1.68, maximum 22.6). That is stronger than the late save and supports medium or higher. The rank stays medium: the late median is still under the cap, and a merely crowded camp still shares push 1 with a camp at raw push 6 to 22. Stress is zero for 88% of bands on the late save, so this is crowding, not famine.
- Chance meetings stop at 4 a season. The median band has 26 neighbours inside the teach radius. That count is not a count of dropped meetings: travel cost was not resimulated, and partner visits are uncapped. From the same cell, where travel is free, 16% of bands already expect more than 4 meetings. The cap sits inside ordinary density.
- A visit moves the same bundle whether the pile is modest or large. Once stock is at least 0.3, the lot is `(stock − 0.2)` capped at `0.3 × frame`. That cap binds for 76% of steel holders who have any tradable steel, and for most copper, bronze, iron, hide and flax holders. The hard stock cap of 1.5 almost never binds.
- Farm warmth is full at 12°C, and 99.8% of farmers are there. Wetter than rain 0.5 does not help a field off a river (42% of farmer camps). Biome and the dry half of the rain range still rank land. Crop stocks sit near three quarters of field capacity (median 0.74), which is a real field, not a pile-up.
- Wool in a cool land is refilled to 0.8 and then drains to 0.78, for all 11 such bands, while their herds vary about tenfold. Flax has the same refill and is not pinned, because flax is also grown and traded.

What is not flattened, on this save. Harvest saturation is low: after one wild regrow, plants take 9% of the stock in reach, game 18%, fish 15%, farms 9%. A better tool still brings more food. The grade of metal inside the tool is a separate, small factor (rank 2): saturation is not what hides it. The fight's casualty cap of 0.45 is never reached (the high end of a loss is about 0.28). Stress hits 1 for one band. The experiment cap of 6 is never reached. `food_ratio.min(1)` in a fight is idle, because food ratio is already capped at 1. The flat 0.02 in `marginal_mats` does not tie metal stocks (gold's median is 0.30, steel's 0.79, bronze's 0.39). `1 + 0.3 × depth` is not a flat rate.

How much to trust this. The save figures are two seeds: year 350 on a 256×160 map, almost everyone well fed and boated, and year 200 on a 128×80 map (1,296 bands, population 23,118), also mostly well fed (89% at the birth ceiling) but with no outriggers and no hazard floor. A formula that binds on both will bind wherever bands are this well fed. The boat floor does not. It is the late save. The protection cap and the push cap bind on both, and they bind more widely early. Neither save says what a hungry year does. Population growth here is not a picture of local abundance: net of the baseline death rate, the well-fed gain about 0.007 people per person per season before disease, and the food term cannot make that larger. The steep harvest curve is the important negative on that save. Harvest saturation is low, so a multiplier does reach the take. What the sizing pass adds is that the material multiplier itself is close to 1. Having a spear still changes the hunt. Which metal the spear is made of does not, by more than about 14% of game and 7% of fighting strength, steel against copper. That ratio does not depend on the save. It is the formula, at user skill 1 and recipe quality 0. Real-world comparisons are ranges (the felling claim is several-fold for a hard metal against stone, and a small step from bronze to steel, not one factor to type in; the tool-relevant hardness gap is about 3–6×, with about 15× only as an outer bound, and those figures were not taken from a note of Michael's). Trust the sim ratios. Treat the "several-fold" felling claim as steel or bronze against stone, not as steel against bronze. The results in hand are weakest where the question is which hull is safer, whether a crowded camp leaves, whether surplus food grows the band, how many neighbours a season can teach, which of two similar materials is worn, knotted or displayed, and how large a steel edge should be beside a copper one.

## The class

A formula whose structure flattens or reverses a real-world ordering, erases a difference that should change a choice, or sizes a real difference far too small or far too large. A clamp that no input reaches, or a bound that is meant to be a biological or physical limit, is not in the class.

Sub-types:

- **A.** A clamp, cap, floor or saturation where several materials, techs or bands land on the same bound, so a real difference vanishes, or the bound is reached in ordinary play.
- **B.** A property used with the wrong sign, or for the wrong job. Malleability standing in for how long an edge keeps working is the example.
- **C.** An additive mix where one property can stand in for a missing essential one (a product, a minimum or a threshold would not let it), or a weight that lets an irrelevant property dominate.
- **D.** `min`, `max`, `round` or an integer conversion that hides a difference the inputs actually had.
- **E.** A constant that dominates the term, so the property barely moves the result. `1.15 - x` with `x` near 1 is the example. A large fixed offset can do the same.
- **F.** A divisor chosen so that real inputs already sit on top of each other, or past 1, before the clamp.
- **G.** A match with a default (`_ => 0.3`) that gives unrelated materials the same value, or ranks a real article wrongly.
- **H.** A difference between materials or techs that is compressed or exaggerated relative to the real world, even when the order is right. A small coefficient in front of a quality (`0.1 × axe`, `0.2 × axe`), a 0..10 property scale standing in for a much wider measured ratio, or an additive offset that shrinks a ratio toward 1, are the examples. Flagged for a later sizing pass. Not retuned here.

## Method and coverage

Workspace members are `civ-core` and `native` (`Cargo.toml`). `web/` is the shell around a wasm build and has no quality formulas. `tools/` is Python for reading result files, not the sim. `civ-core/examples/` reprints a few of the same clamps for charts.

Searches covered `civ-core/src` and `native/src` (tests included). Counts are match counts, not file counts.

| Pattern | Count |
|---|---:|
| `.clamp(` | 96 |
| `.min(` | 159 |
| `.max(` | 270 |
| `max(` not as a method | 6 |
| `min(` not as a method | 0 (the sim uses the method) |
| `saturating_` | 27 |
| `.round()` | 23 |
| `as u8` / `as u16` / `as u32` / `as u64` | 479 |
| `/ <number>.0` | 87 |
| `_ =>` | 89 |
| `props()` | 94 |

Almost all of the `as u*` casts are indices, item ids and event kinds, not a float quality being truncated. The quality path was read directly: every `props()` use in `civ-core/src` outside tests, and the functions `cut_q`, `durability`, `vessel_q`, `boat_q`, `mesh_q`, `garment_q`, `fire_temp`, `edge_q`, `weapon_q`, `axe_term`, `tool_upkeep`, the kit picker in `effects.rs` (`make_kit_scored`), `protect`, `sea_survival_x`, `sea_range` and the home-range fish radius.

The ignored test `audit_formula_caps` evaluates those formulas on all 29 materials and prints ties. It also scores hunt, cut, axe, net, trap, boat, bow and shield through `slot_score` on `BandView::nominal()` (food equals population, user skill 1, quality 0) at chain depth 0 and depth 8. Nothing is asserted.

```text
cargo test --release -p civ-core audit_formula_caps -- --ignored --nocapture
```

The run passed. The tables below are that output, cut down to the rows that matter. `CAP` means the clamp changed the value. `AT1` means the raw value was already 1.

The second ignored test, `audit_bounds_in_play`, loads only the save in `DIAG_SAVE`. If that variable is unset it prints `audit_bounds_in_play: DIAG_SAVE is unset; skipping (no save loaded)` and returns. It does not fall back to a path in the source. This run set `DIAG_SAVE` to `/workspace/bots/build-bot/step3-saves/main_seed6_y350_256x160.civsave` (format 16, year 350.00, tick 1400, 3,354 live bands, population 67,836). It does not step the world. It copies the private wild-stock regrow, applies it once to this test's plant stocks, and calls the public `economy`, so harvest saturation is the start of season 0. Crops and fields are not regrown. Saved `food_ratio`, stress and spare are the season just lived (tick 1400 is the start of the next year; the season just run is autumn). Fractions below are that one run. Nothing is asserted.

How to make the saves again. Both files in `step3-saves` are worlds written by main at `f8d80dd` ("Skills step 2: user skill by form, save format 16"). A later commit will not reproduce the bytes.

- Seed 6, year 350, map 256×160: `kindling --headless --seed 6 --map 256x160 --years 350 --threads 4 --progress-every 0 --save main_seed6_y350_256x160.civsave`, from that commit. The loaded file matches that description (format 16, year 350.00).
- Seed 1, year 200, map 128×80: the file is named `main_seed1_y200_128x80.civsave`. The name is seed 1, year 200, 128×80, and the step-3 brief records this copy as the same main `f8d80dd`. This audit did not find a separate build log beside that brief, and the commit is not stored in the filename as a hash. The matching command is `kindling --headless --seed 1 --map 128x80 --years 200 --threads 4 --progress-every 0 --save main_seed1_y200_128x80.civsave`. An earlier step-2 note describes a save of the same shape made at `ecf56ef`; the file used here is the step-3 copy, which that brief attributes to `f8d80dd`.

```text
cargo test --release -p civ-core audit_bounds -- --ignored --nocapture
```

That filter also runs `audit_bounds_in_play_early`, the same body, reading `DIAG_SAVE_EARLY`. If that variable is unset the test prints `audit_bounds_in_play_early: DIAG_SAVE_EARLY is unset; skipping (no save loaded)` and returns. This run set it to the seed-1 file (format 16, year 200.00, tick 800, 1,296 live bands, population 23,118). It does not step the world. The figures used from it, and only those, are: food term 1 for 89.12% of bands and 0 for 6.87%; raw push at least 1 for 63.81% (median 1.68, maximum 22.6); protection at least 0.919 for 69.75%; 1,294 boats, outrigger 0%, hazard floor 0%, fish radius 2 for 735 and radius 3 for 559 (43.20% of boats have quality at least 0.75, the rounding cliff); hunts flint 551, bone 449, wood 295. Per-band rows are not copied here.

The third ignored test, `audit_material_ratios`, prints the size of a gap at three layers: the 0..10 property, the quality score, and the food, fight or survival figure the sim applies. It builds one kit per material, calls `kit_yields`, `weapon_q`, `tool_upkeep`, `slot_score`, `World::sea_survival_x`, `World::sea_range` and `fire_temp`, with user skill 1 and recipe quality 0. Nothing is asserted. It does not load a save.

```text
cargo test --release -p civ-core audit_material_ratios -- --ignored --nocapture
```

The run passed. Every sim ratio in the sizing section is that output.

### Qualities of the materials that can actually be tools

| Material | cut | durability | wear `1.15−dur` | edge | axe `½ cut + ½ dur` | boat | mesh | garment |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| flint | 0.760 | 0.292 | 0.858 | 0.549 | 0.526 | 0.250 | 0.233 | 0.292 |
| obsidian | 0.820 | 0.125 | 1.025 | 0.507 | 0.472 | 0.107 | 0.100 | 0.125 |
| wood | 0.200 | 0.500 | 0.650 | 0.335 | 0.350 | 0.929 | 0.667 | 0.833 |
| bone | 0.430 | 0.417 | 0.733 | 0.424 | 0.423 | 0.357 | 0.333 | 0.417 |
| plant fibre | 0.020 | 0.250 | 0.900 | 0.123 | 0.135 | 0.643 | 0.800 | 1.000 AT1 |
| hide | 0.040 | 0.500 | 0.650 | 0.247 | 0.270 | 0.929 | 0.800 | 1.000 AT1 |
| flax | 0.020 | 0.375 | 0.775 | 0.180 | 0.198 | 0.321 | 0.933 | 0.550 |
| wool | 0.012 | 0.333 | 0.817 | 0.157 | 0.173 | 0.429 | 0.800 | 1.000 AT1 |
| copper | 0.480 | 1.000 CAP (raw 1.125) | 0.150 | 0.714 | 0.740 | 0.643 | 0.600 | 0.750 |
| bronze | 0.650 | 1.000 AT1 | 0.150 | 0.808 | 0.825 | 0.643 | 0.600 | 0.750 |
| iron | 0.640 | 1.000 AT1 | 0.150 | 0.802 | 0.820 | 0.643 | 0.600 | 0.750 |
| steel | 0.780 | 0.979 | 0.171 | 0.870 | 0.880 | 0.643 | 0.600 | 0.750 |
| gold | 0.160 | 0.833 | 0.317 | 0.463 | 0.497 | 0.357 | 0.333 | 0.417 |

`cut_q` never hits its clamp (the highest raw value is obsidian at 0.820). `boat_q` tops out at 0.929 and `mesh_q` at 0.933, so those clamps are idle. The durability clamp and the garment clamp are not.

Weapon quality of a lone spear: steel 0.915, bronze 0.875, iron 0.871, copper 0.814, flint 0.707, obsidian 0.680, stone 0.671, bone 0.626.

Nominal `slot_score` (depth 0 / depth 8), empty kit, higher is better:

| Slot | Order at depth 0 | What ties |
|---|---|---|
| Hunt, as a spear | obsidian 16.868, steel 16.856, flint 16.194, bronze 15.196, iron 15.068, copper 13.010 | — |
| Hunt, as a spear, depth 8 | steel 16.615, obsidian 15.421, flint 14.983, bronze 14.985 | bronze passes flint once both are deep |
| Cut, a knife | steel 1.947, bronze 1.618, iron 1.592, obsidian 1.550, flint 1.490, copper 1.172 | — |
| Axe | steel 0.559, bronze 0.531, iron 0.527, copper 0.467 | at depth 8: bronze 0.319, steel 0.318, iron 0.315 |
| Net | flax 8.794, hide 7.546, wool 7.448, fibre 7.399 | copper, bronze and iron tie at 5.858 (not nets) |
| Boat | wood 0.890, hide 0.890, then fibre 0.589 | wood and hide |
| Bow or shield term | copper = bronze = iron 9.991, steel 9.990, then stone, wood = hide 6.961, flax 5.454, wool 4.951 | the metal tie is not a makeable bow |

Fire, open / stone lining / clay / ceramic: wood 650 / 810 / 970 / 1010; charcoal 950 / 1110 / 1270 / 1310. The charcoal advantage stays 300 degrees. Copper ore reduces at 1150, so a stone lining is not enough and a clay or ceramic one is. Iron ore reduces at 1400, which still needs the bellows (+250). That ordering is intact. The size of the gates, which is a separate question, is section 20: charcoal and stone miss copper by 40°C, and ceramic over clay is only +40°C.

Section numbers under Findings keep the first pass's order. Sections 18–20 are the sizing pass. The rank column is the order after that pass. A "rank N" in a finding is this table, not the previous one. The axe numbers already measured in the other worktree were not repeated.

## Ranked findings

Severity. High: the formula reverses or flattens a core comparison widely. Medium: a wrong order, or a bound that binds widely and erases a difference that should change an outcome. Low: it trims the top, it hits a small population, or other terms still carry the ordering.

`durability-v2` is the other stream. It is off in this build. When it is on it addresses ranks 1 and 12, partly rank 3, and the mark slot. The note under this table says how. No other rank is what that feature retunes.

| Rank | Severity | Where | Issue | Impact | Proposed fix |
|---|---|---|---|---|---|
| 1 | High | `materials.rs:537`, `effects.rs:373` | Durability caps copper, bronze and iron at 1 and ranks steel lower, because it pays for malleability. Wear is `1.15 − durability`, so steel wears out first. `durability-v2` addresses this when on. | Every upkeep comparison among metals. Fight order stays steel first. Axe keep-or-switch moves, as the axe study measured. Depth still separates the metals (copper axes near depth 3.2, steel near 7.3). Copper's upkeep is also 0.27× a stone axe's, which exaggerates copper against stone. | Score edge life from toughness and hardness, not malleability, and do not let three metals share the ceiling. |
| 2 | High | `effects.rs:554`, `effects.rs:564`, `materials.rs:532`, `band.rs:701`, `world/conflict.rs:49` | Sub-type H. Felling trials are several-fold for a hard metal against stone, and a small step from bronze to steel. Steel/copper is 1.63× on `cut_q`. Plants then move by 1.009× (`0.1 × axe`), farm by 1.119×, spear strength by 1.068×. A tool edge is about 3–6× on Vickers, not the calorie target. Depth 8 multiplies upkeep by 3.4, which on the nominal mix outweighs the metal. | The tech ladder barely changes food or a fight. Iron against bronze is already about 1, and that part is fair. Details in section 18. | Not now. Later: size axes and blades to sustained work, not to a 10% bump, and shrink the fight offsets. Re-price depth after. |
| 3 | Medium | `materials.rs:553`, `effects.rs:873`, `world.rs:1804`, `world.rs:2761` | Dugout and skin boat both score 0.929. Late game (year 350): hull quality is at 1, 98.8% have an outrigger, 98.6% of boats sit on the hazard floor of 0.3, and 3,296 of 3,333 share fish radius 3. Year 200: floor 0%, outrigger 0%, radius 2 for 735 boats and radius 3 for 559, a cliff at quality 0.75. `durability-v2` partly addresses the wear tie (`hull_life` reads hardness), not the floor or the round. | A plank does not fish further and does not keep a crew safer once the floor binds. That is late. Early, the radius still erases the gap inside each bin. Voyage distance still spreads (`sea_range` about 10 to 20 on the late save). | Separate wood and hide. Do not round the radius. Do not floor survival so early, and do not let the outrigger pin every hull there. |
| 4 | Medium | `effects.rs:872`, `world.rs:1804`, `world.rs:2760` | Sub-type H. A plank is only `boat_q + 0.25` (1.077× a dugout). Survival on a 10-cell hop is 1.013×, fish radius is the same 3, and the outrigger's +0.15 is larger than the hull gap. | Even after wood and hide are untied, a planked hull is not a different boat. Section 19. | Let hull class move load and hazard by more than 0.15. Do not retune now. |
| 5 | Medium | `effects.rs:892` | Protection caps at 0.92. With fire, warmth 1, a kiln and an axe the raw value is 0.95. Year 350: 51.16% of bands are on the cap. Year 200: 69.75%. Prerequisite for rank 8: a warmth fix that leaves the best garment at 1 is swallowed here. | Most of the earlier world, and half the later one, share one cold-protection number. Extra garment quality cannot help them. | Put the cap above a fully equipped good garment, or drop it, in the same change as the warmth table. |
| 6 | Medium | `world.rs:2207`, `world.rs:4851` | Births use `((food_ratio − 0.75) / 0.25).clamp(0, 1)`, and `food_ratio` is already capped at 1. Year 350: the food term is 1 for 98.78% and 0 for 0.42%. Year 200: 1 for 89.12% and 0 for 6.87%. | Surplus food does not buy children. Spare and settlement still move the birth factor by about ±8% around the median on the late save. Store is not full. The rank holds: both saves are mostly on the top step. | Let births see the store, not only whether this season was eaten. |
| 7 | Medium | `world.rs:2744` | `push_out` clamps stress, crowd and pressure to 1. Year 350: raw push is at least 1 for 46% (median 0.78, max 22). Year 200: 63.81% (median 1.68, max 22.6). The early numbers support medium or higher. | A merely crowded camp and a camp with raw push 6–22 leave, split and scout at the same rate. Stress is not what drives it. Kept at medium because the late median is still under the cap. | Soft-saturate, `1 − exp(−raw)`, so the tail still ranks. |
| 8 | Medium | `materials.rs:563`, `effects.rs:1249` | Hide, wool and plant fibre all have warmth 1.0. Flax is special-cased to 0.55. The tie keeps hide. Wool/hide cold need is exactly 1. Depends on rank 5. | Wool clothes are never worn once hide clothes exist. For the bands on the protection cap, a split that stays near 1 would not change cold need either. Linen at 0.55 is a plausible size. | Rank warmth by loft: wool, then hide, then linen and bast, with no shared ceiling, and clear the protection cap first. |
| 9 | Medium | `world.rs:28`, `world.rs:3431` | Chance meetings stop at 4. Median neighbours inside the teach radius is 26. Partner visits are uncapped. | Denser country does not teach more, once a band is past 4 chance meetings. Not fully resimulated: 16% are over the cap from their own cell alone. | Let the cap grow with the band, for example `4 + pop / 40`, instead of a flat 4. |
| 10 | Medium | `materials.rs:558`, `effects.rs:671` | Mesh quality ties fibre, hide and wool at 0.800. Wear then picks hide, then wool, then bast. Flax/fibre mesh is 1.17× and fish from the net is only 1.07×. | Until linen exists, the net and the snare are rawhide, not bast cord. The tie, not the 0.9 coefficient, is what erases wool against bast. | Make mesh a product of fibre and strength, so bast beats wool and rawhide. |
| 11 | Medium | `effects.rs:1258` | Ornaments are ranked by hardness, plus 10 if the material is prized. | Obsidian beats bronze. Bone beats copper. Gold is fine. Prestige amount ignores hardness. | Rank show by prized and by being metal, not by scratch hardness. |
| 12 | Low | `effects.rs:500`, `materials.rs:532` | Hunt food uses `cut_q` and not toughness. Obsidian's edge outscores steel's at depth 0 only. `durability-v2` addresses this when on: `hunt_q_of` uses `food_edge_q`. | The order is wrong only where the chain is new, and other terms already carry the keeper. Year 350: 3,255 hunts are steel (wood 22 and copper 45 are the only other materials above 20). Year 200: flint 551, bone 449, wood 295, no steel. The size of steel against copper, once order is fixed, is rank 2. | Put durability into the hunt food term, after finding 1, or stop letting sharpness alone outrank a metal. |
| 13 | Low | `world.rs:3929`, `world.rs:163` | A relevant lesson is `skill × 0.8 × prestige_bias`, then `.min(0.95)`. The bias floor of 0.5 binds on 10% of partner edges; the ceiling of 2 binds on 0.67%. | 19% of teachable recipes (skill at least 0.25) would hit 0.95 if the lesson is relevant. Ordinary teachers, bias near 1, stay under it. Showing off never hits its 0.9 cap. | Let bias raise a chance that is already a probability, `1 − (1 − base)^bias`, so skill and prestige keep their order under 1. |
| 14 | Low | `world.rs:3698` | A visit's lot is `(stock − 0.2).min(0.3 × frame)` once stock is at least 0.3. Worth is already 0 at stock 1.3. The stock cap of 1.5 almost never binds. | A band with 0.5 steel and one with 1.2 send the same bundle. Steel: 76% of holders with stock at least 0.3. Gold never. | If a richer pile should move more, let the cap grow with stock. A load limit is still reasonable, which is why this stays low. |
| 15 | Low | `farming.rs:59`, `farming.rs:60` | Warmth is full at 12°C (99.8% of farmers). Off a river, water is full once rain is 0.5 (42% of farmer camps). | Hot and cool farmland share warmth 1. Biome, river, and the dry half of rain still rank land. Crop at field capacity is only 11% (median 0.74). | If hotter farmland should yield more, saturate warmth nearer 18°C than 12°C. Otherwise leave it: the niche is the usual shape. |
| 16 | Low | `farming.rs:547` | Cool-land wool is `stock.max(0.8)`, then ordinary use drains it to about 0.78. | All 11 such bands sit there, while herd per person runs from 0.04 to 0.58. Flax has the same refill (`farming.rs:462`) and is not pinned. | Fill wool from the herd, `(stock + 0.15 × herd).min(1.5)`, instead of a constant. |
| 17 | Low | `effects.rs:838` | A bow's fight bonus is a flat 0.3 whenever any point is carried. | Bone arrows and steel arrows fight the same once the bow exists. The spear still cares about material. | Scale the bow's fight term by the point's `edge_q`. |
| 18 | Low | `world.rs:2008` | Food effort is at least 30% of labour. Upkeep alone exceeds 55% of labour for 3.64% of bands, which is the tail that hits that floor on tools alone. | Those bands cannot spend less time feeding themselves as the kit gets dearer. `1 + 0.3 × depth` still spreads (median 2.2, max 4.9). | Leave it if the floor is "you still have to eat". It is not a flat labour price. |
| 19 | Low | `materials.rs:549` | `vessel_q` defaults to 0.3, so wood, stone, every metal, bone, pitch and meat are one vessel. | No recipe makes those vessels, so the 0.3 does not change play. | Default unknown vessels to 0. |
| 20 | Low | `materials.rs:571`, `physics.rs:476`, `materials.rs:197` | Sub-type H. Iron reduces at 1400°C, 90°C above charcoal in a ceramic kiln, so bellows are a switch. Ceramic over clay is only +40°C. Charcoal over wood (+300°C) is a fair step. | The ladder's order is right. The gate is a razor, and a good furnace cannot touch iron. Section 20. | Keep the order. If bellows should help iron rather than be required, lower 1400 into the usual bloomery range. Not now. |
| 21 | Low | `physics.rs:291` | Sub-types B and C, latent. An edge is hafted as an axe if `hard >= 3.0 && tough + malleable >= 3.0`. Malleability stands in for toughness, the same mistake as rank 1. No current material changes outcome: obsidian fails either way, and every metal that passes does so on toughness alone. | A future metal that is hard enough, not tough, and very malleable would become an axe. Nothing in the present table does. | Require `tough >= 3.0` on its own. Do not add malleability into the gate. |

### What `durability-v2` addresses when it is on

The feature is not on this branch, and this audit does not turn it on. The column is this note.

| Rank or slot | When the feature is on |
|---|---|
| 1 | Addressed. Wear no longer uses `(tough + 0.5 × malleable) / 12` capped at 1. |
| 3 | Partly. `hull_life` reads hardness, so wood and hide do not share one hull-wear number. `boat_q`, the hazard floor and `round` on the fish radius are unchanged. |
| 12 | Addressed. `hunt_q_of` calls `food_edge_q` instead of raw `cut_q`. |
| Mark slot | Addressed, though the slot was left in "Checked, and fine". `record_durability` uses `record_life`, which reads hardness. The ceramic +1 in the kit choice stays. |

Ranks 2 and 4–11 and 13–21 are not what that feature retunes. The axe-hafting gate (rank 21) is unchanged on that branch. The plants coefficient 0.1 and the farm coefficient 0.2 stay.

## Findings

### 1. Durability, malleability, and the wear residual

High. Sub-types A, B, E and F. This is the exemplar. Another stream is changing it. Nothing here does.

```537:539:civ-core/src/materials.rs
pub fn durability(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + 0.5 * p.malleable) / 12.0).clamp(0.05, 1.0)
}
```

```366:373:civ-core/src/effects.rs
pub fn tool_upkeep(rate: f32, it: Item, depth: u8, sealed: f32) -> f32 {
    if it == NONE || form(it) == Form::Ornament {
        return 0.0;
    }
    if matches!(form(it), Form::Fire | Form::Cooked) {
        return 0.01;
    }
    sealed * rate * (1.15 - durability(mat(it))) * (1.0 + 0.3 * depth as f32)
}
```

What it does. Durability is toughness plus half of malleability, divided by 12, then clamped to 0.05..1. Wear, for everything except a fire, a meal or an ornament, is 1.15 minus that number, times depth.

The order it should respect. A working edge should last longer as it gets harder and tougher, not as it gets easier to hammer. Copper is soft and the edge rolls. Bronze holds an edge better. Wrought iron is in the same class as a good bronze, or a little better. Steel holds an edge best of the four. Flint holds a sharp edge until it chips. Obsidian is sharper still and chips immediately.

Why it is wrong. Copper's raw value is (9 + 0.5×9) / 12 = 1.125, clamped to 1. Bronze is (9 + 0.5×6) / 12 = 1 exactly. Iron is the same 1. Steel is (9 + 0.5×5.5) / 12 = 0.979, because it was given less malleability on purpose (it is quench-hardened). The divisor of 12 is what piles the three softer metals on the ceiling (F). Malleability is the wrong property (B): it describes whether a smith can reshape the metal, not whether the edge survives use. The constant 1.15 then turns a capped 1.0 into wear 0.150 and steel's 0.979 into wear 0.171 (E). Being the best metal in the model makes you wear out faster than copper. Only brine hits the 0.05 floor. That floor is harmless.

`edge_q` is `0.55 * cut_q + 0.45 * durability`. The cap compresses bronze and iron (0.808 and 0.802) but does not put them over steel (0.870). `weapon_q` of a spear stays steel 0.915, bronze 0.875, iron 0.871, copper 0.814, flint 0.707. The comment in `weapon_q` says durability was added so that bronze and iron would beat flint. They do. The comment in `tests.rs` around the steel assertions allows steel's durability to be as low as 95% of iron's, so the test currently accepts this.

Impact. Upkeep of every copper, bronze and iron tool is identical per unit of depth. Steel's is about 14% higher before depth. The axe study measured wear 0.150 against 0.171 and showed it is a real part of why a forager keeps an iron or copper axe. On the nominal mix in this audit, with depth forced equal, the axe scores at depth 0 are steel 0.559, bronze 0.531, iron 0.527, copper 0.467. At depth 8 they are bronze 0.319, steel 0.318, iron 0.315, copper 0.255. Bronze passing steel is the cap, visible once the food term is small and the chain is long. It is a small cross on this particular mix. It is the wrong durability ordering on every mix.

A knife does not reverse. Cut-slot scores at depth 0: steel 1.947, bronze 1.618, iron 1.592, obsidian 1.550, flint 1.490, copper 1.172.

Proposed fix, not applied. Drop malleability. Keep the result below the ceiling for every metal, in the order steel, iron, bronze, copper, flint, obsidian. One shape that does that with the numbers already in `PROPS`:

```rust
// edge life: hard enough to hold, tough enough not to chip. Not "easy to reshape".
pub fn durability(m: Mat) -> f32 {
    let p = m.props();
    (0.65 * p.tough + 0.35 * p.hard) / 10.0
}
```

That is about steel 0.80, iron 0.78, bronze 0.76, copper 0.69, flint 0.47, obsidian 0.29, hide 0.43. No clamp is required for these inputs. Wear should not be `1.15 - d` if `d` lives near 1: a gap of 0.02 becomes a large share of a 0.15 residual. A proportional wear, for example `0.25 / durability`, keeps the same order and does not pile the best metals onto one labour cost. Any change here moves golden hashes. It should land with the durability stream, not beside it.

### 2. Warmth ties wool, hide and bast, then keeps the hide

Medium, rank 8. Sub-types A, B and F. Depends on rank 5: if the best garment stays at warmth 1, fire plus kiln plus axe is still 0.95 and the protection cap puts it back to 0.92. On the year-200 save that cap already holds 69.75% of bands.

```563:568:civ-core/src/materials.rs
pub fn garment_q(m: Mat) -> f32 {
    if m == Mat::Flax {
        return 0.55; // linen: strong and light, but not warm
    }
    let p = m.props();
    ((p.tough + p.fibrous) / 12.0).clamp(0.0, 1.0)
}
```

The kit picker takes the maximum of this and nothing else (`effects.rs:1249`). On a tie it keeps the earlier item id.

The order it should respect. In the cold: a fleece or a fur, then a hide cloak, then linen, then a bast or grass cape. Linen is cool. Bast is not warmer than wool. The code already says so for flax, in the comment.

Why it is wrong. Wool is toughness 4 plus fibrous 8, which is 12/12 = 1. Hide is 6+6 = 12/12 = 1. Plant fibre is 3+9 = 12/12 = 1. Three different clothes land on the ceiling because the divisor is 12 (F) and because toughness is being asked to mean warmth (B). The flax special case shows the author knew a plant cloth is not warm, but wild fibre goes through the formula and hits 1, and wool is not given a loft that hide lacks. Physics never sews a garment from raw fibre (`cloth_fibre` requires toughness at least 4, and fibre is 3), so the fibre 1.0 is dead. The live pair is hide and wool, both 1.0, and linen at 0.55.

Hide's material index is 8 and wool's is 19, and the picker uses a strict `>`. Hide always wins the tie.

Impact. `protect` is `0.35 * fire + 0.5 * garment_q + ...`, capped at 0.92. Hide and wool both give 0.850 with a fire and no kiln, so cold need (`cold_scale * ((8 - t) / 14) * (1 - protect)`) is the same either way. Survival is not wrong *because the scores differ*; it is wrong because they do not differ, and the world then wears hide forever. Wool stock is still produced by a tame herd in a cool land. It is not what the garment slot demands. Sails are a separate ranking and do prefer linen to wool (14.5 against 14), which is the right sail order. That path is fine.

Proposed fix, not applied. Only four fibres are clothing, and the formula cannot see loft. A table matches the physics that actually makes clothes:

```rust
pub fn garment_q(m: Mat) -> f32 {
    match m {
        Mat::Wool => 0.95,
        Mat::Hide => 0.80,
        Mat::Flax => 0.45,
        Mat::Fibre => 0.35,
        _ => 0.15,
    }
}
```

Wool then replaces hide in the cold slot, and linen stays the hot-weather cloth. If garments should also be chosen for the climate rather than one global maximum, that is a different change and not required to unstick the tie.

### 3. Nets: the same mesh score, then hide wins on wear

Medium. Sub-types C and E.

```558:560:civ-core/src/materials.rs
pub fn mesh_q(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + p.fibrous) / 15.0).clamp(0.0, 1.0)
}
```

The order it should respect. For a net: linen or flax, then bast (hemp, nettle, the sim's plant fibre), then wool a long way behind (it stretches and weakens when wet), then rawhide, which is a snare and a poor net. For a snare, rawhide is legitimately good. The sim uses one score for both slots.

Why it is wrong. Fibre is (3+9)/15 = 0.800, wool is (4+8)/15 = 0.800, hide is (6+6)/15 = 0.800. Toughness fills in for missing fibre, so a stiff hide ties a fibrous bast (C). The clamp never binds (flax is 0.933). The nominal net score then subtracts wear, and hide (durability 0.500, wear 0.650) beats wool (wear 0.817) beats fibre (wear 0.900): 7.546, 7.448, 7.399. The same order appears on the trap slot (13.332, 13.234, 13.185). Flax wins both once it exists (net 8.794, trap 15.544).

Impact. Hide cord is a real recipe (an edge cuts a hide with fibrous at least 5). Fibre cord is a real recipe (fibrous at least 8, twisted by hand). Until a band can weave flax, the picker takes the hide net and the hide snare. Bast is the historical net fibre and it loses. The tie-break is not what saves it: the scores are not equal, hide is actually ahead. After flax, the reversal is hidden, because linen wins for a good reason (it is both more fibrous and only a little less tough).

Proposed fix, not applied. Fibre should be required, and toughness should only scale it:

```rust
pub fn mesh_q(m: Mat) -> f32 {
    let p = m.props();
    (p.fibrous / 10.0) * (0.4 + 0.6 * p.tough / 10.0)
}
```

That is about flax 0.64, fibre 0.52, wool 0.51, hide 0.46. Bast then beats rawhide on the mesh term itself, so wear cannot overturn it as easily. If snares should still prefer rawhide, the trap slot needs its own term. One number cannot be the best net and the best snare.

### 4. Dugout and skin boat are the same boat, and rounding finishes the job

Medium, rank 3, and the hazard floor is the late game. Sub-types A, C, D and E. The first pass called the survival floor a veteran effect. On the year-350 save it binds for almost every boat, including novices. On the year-200 save it binds for none (0 of 1,294 boats, and no outrigger). Early, the fish-radius round is the cliff: 735 boats on radius 2 and 559 on radius 3, which is quality 0.75. The rank stays medium.

```553:555:civ-core/src/materials.rs
pub fn boat_q(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + p.float.max(p.seal)) / 14.0).clamp(0.0, 1.0)
}
```

A plank or a ship then does `(boat_q + 0.25).min(1.0)` (`effects.rs:873` and `:877`). Recipe quality multiplies and caps again at 1 (`effects.rs:902`). The fish radius is `1 + (2 * boat_q).round()` (`world.rs:1804`). Open-water survival uses `(1.4 - q - 0.25 * xp).max(0.3)` (`world.rs:2761`).

The order it should respect. A reed bundle is the worst of the boats people actually build here. A skin boat (umiak, currach) and a dugout are both real sea boats; a dugout is stiffer and does not soak the hull the way a skin does. A plank hull is sounder than either, and it should still be sounder after the crew knows the sea. Pitch is not a hull.

Why it is wrong. Wood is toughness 6 and float 7, so (6+7)/14 = 0.929. Hide is toughness 6 and seal 7, so `float.max(seal)` is also 7 and the score is 0.929. The `max` lets "floats" and "is watertight" substitute for each other, and the two hulls were given the same toughness, so they tie (C). The nominal boat score ties as well, 0.890, because wear also ties (both durability 0.500). Wood's material index is 0 and hide's is 8, so the dugout is kept whenever both recipes exist. Fibre, the reed boat, is 0.643 and scores 0.589. That part is right. Pitch scores 0.786 as a hull because seal is 9; physics will not build a pitch boat (a skin boat also requires toughness at least 5, and pitch is 2), so that value is latent.

The plank cap and the rounding then erase the plank. Wood plank quality is `min(0.929+0.25, 1) = 1`. Fish radius: dugout and skin `2 * 0.929 = 1.858`, rounds to 2, radius 3. Plank `2 * 1 = 2`, rounds to 2, radius 3 (D). Voyage *distance* still differs a little: `sea_range` at no experience is 11.29 for the dugout and 12.00 for the plank. Survival does not, once the crew is experienced. Hazard at experience 0 is 0.471 for wood or hide and 0.400 for a plank. At experience 1 all three hit the floor of 0.3 (E). A reed boat at experience 1 is still 0.507, so the floor is not so low that every hull is equal. It is low enough that a dugout, a skin boat and a ship, in the hands of people who sail, fail crossings at the same rate.

A practised dugout also hits the quality cap: `0.929 * (1 + 0.3 * quality)` reaches 1.0 once quality is about 0.25. Further practice does not make that hull sounder, and it meets the plank hull at 1.

Impact. Where there are trees and an axe (cut quality above 0.4, which a flint axe has), the skin boat is dominated and never selected. In a treeless country the dugout recipe, if it has spread, is still the one in the kit, and the skin boat the band can actually cover with hide is not. Fishing reach does not reward a plank hull. Reed boats remain worse on the formula, which is right.

Measured, seed 6, year 350, 3,333 boats. Hull quality: minimum 0.665, 10th percentile 0.979, median 1.000, maximum 1.000, mean 0.992. The `.min(1)` on a plank or a practised hull is the ordinary value, not a corner. Outrigger: 98.77%. `boat_safety` is hull quality plus 0.15 with an outrigger, so safety is about 1.1 or more and the raw hazard `(1.4 − q − 0.25 × xp)` is at most 0.3 even at experience 0. Bands on that floor: 98.62%, and 1,164 of 1,178 crews with seacraft in `[0, 0.25)`. A plank without an outrigger would not floor at experience 0 (raw hazard about 0.4). The outrigger is what puts the novices there. Fish radius `1 + round(2 × boat_q)`: 37 boats at radius 2, 3,296 at radius 3, none further. `boat_q` at least 0.75, the bin a plank shares, is 98.89%. Distance is not flattened. `sea_range`: minimum 9.82, 10th percentile 13.59, median 15.47, 90th percentile 17.99, maximum 20.47. A better-handled boat still goes further. It does not survive better, and it does not fish a wider square.

Proposed fix, not applied. Give the dugout a small edge that the props can support (wood keeps its float in the water; hide's seal is a skin over a frame), and use the unrounded quality for the radius:

```rust
pub fn boat_q(m: Mat) -> f32 {
    let p = m.props();
    let hull = if p.float >= p.seal { p.float } else { 0.85 * p.seal };
    (p.tough + hull) / 14.0
}
```

Wood stays 0.929. Hide becomes (6 + 0.85×7) / 14 = 0.854. Fibre stays 0.643. The fish radius should be the continuous `sea_range`, or at least `1 + (2.0 * q)` without `round`, so a plank at 1.0 reaches one step further than a dugout at 0.93. The survival floor of 0.3 should sit lower than a perfect hull with a veteran crew, or veterans in a reed boat and veterans in a ship will keep meeting it. For example `max(0.15)` leaves room between q = 0.93 and q = 1 at experience 1 (hazards 0.22 and 0.15). That is not enough on this save. `boat_safety` adds 0.15 for an outrigger (`world.rs:2774`) on a hull already clamped to 1, so raw hazard at experience 0 is 0.25 and the floor of 0.3 catches it. A lower floor would separate a novice (0.25) from a worse hull, and veterans with an outrigger would still meet whatever floor is left. The outrigger bonus should not be added on top of a hull that is already 1, or the hazard should be allowed to go on falling as safety passes 1.1.

### 5. Ornaments are ranked by how hard they are

Medium. Sub-type B, with an E offset that is deliberate for gold only.

```1257:1258:civ-core/src/effects.rs
// a prized material (gold) is worn before any hard one
Form::Ornament => upd(20, m.props().hard + 1.0 + 10.0 * m.props().prized, &mut k.ornament),
```

The order it should respect. Gold first, which the +10 already does. Then copper and bronze, which people smelted in part so they could wear them. Then obsidian and other bright stone. Then bone. Hard flint is a tool, not a jewel, and it is not an ornament recipe anyway.

Why it is wrong. The score is hardness plus one, plus 10 if `prized` is set. Only gold has `prized`. Makeable ornaments are bone, copper and bronze (a point, hammered), obsidian (ground), and gold (cast). Their scores: gold 13.5, obsidian 6.5, bronze 6.0, bone 5.0, copper 4.0. Scratch hardness is the wrong property for "worth showing" (B). The +1 does not dominate. The +10 does, and that part is intentional.

Impact. The worn ornament is obsidian whenever both obsidian and bronze exist, and bone whenever both bone and copper exist. Prestige gained per season is `PRESTIGE_FINE * (0.5 + fine + prized) * sqrt(pop / 25)`, and `fine` is recipe quality, not this score. Gold adds 1. The others add nothing, so the prestige *rate* is almost the same whichever non-gold ornament is worn. What changes is the want bit (`world.rs` `want`): the band asks its partners for the material of the ornament it actually wears. Copper's early value as a display good is missing, and obsidian keeps a display role it should have lost to bronze.

Proposed fix, not applied.

```rust
let show = 10.0 * p.prized + if m.is_metal() { 3.0 } else { 0.0 } + 0.1 * p.hard;
```

Gold about 13.3, bronze 3.5, copper 3.3, obsidian 0.55, bone 0.4. Copper then beats bone. Bronze beats obsidian. Gold still beats everyone. Hardness remains a trace, not the decision.

### 6. The hunt term thinks obsidian cuts better than steel

Low, rank 12. Sub-types B and C. The order is wrong only at depth 0. At depth 8 steel is already ahead, and on the year-350 save the keeper is steel (3,255 hunts; wood 22 and copper 45 are the only other materials above 20). On the year-200 save there is no steel hunt (flint 551, bone 449, wood 295). Other terms carry the ordering, which is this audit's definition of low. `durability-v2` routes `hunt_q_of` through `food_edge_q` and addresses it when on.

```494:504:civ-core/src/effects.rs
fn hunt_q_of(it: Item) -> f32 {
    if it == NONE {
        0.0
    } else {
        let m = mat(it);
        match form(it) {
            Form::Spear => 0.35 + 0.65 * cut_q(m),
            Form::Point => 0.35 * cut_q(m),
            _ => 0.2 * cut_q(m),
        }
    }
}
```

`cut_q` is `(0.6 * sharp + 0.4 * hard) / 10`. Obsidian is 0.820, steel 0.780, flint 0.760, bronze 0.650, iron 0.640, copper 0.480. Nothing is clamped.

The order it should respect. A fresh obsidian flake is sharper than a steel edge. A spear or a knife that has to last the season is not. The fight path already uses `edge_q` (cut and durability) so that steel beats obsidian as a weapon: spear weapon-quality 0.915 against 0.680. The food path does not.

Why it is wrong. Sharpness is 60% of `cut_q` and toughness is absent (C, and B if "cutting quality" was meant to be a working edge rather than a fresh one). The comment on steel in `PROPS` compares steel's 0.78 with iron and bronze and does not mention obsidian. Wear is the only place brittleness enters the food decision.

Impact, from the nominal slot score. As a spear at depth 0: obsidian 16.868, steel 16.856, flint 16.194. The cross is 0.012, because the game term is large (`1.4 * hunt_q`) and the wear gap has not been multiplied by a long chain. At depth 8: steel 16.615, obsidian 15.421, flint 14.983. An obsidian spear's chain is a knapped point on a shaft, on the order of depth 4, which scales to about 16.14. A steel spear near depth 8 or 9 stays near 16.6. So on this mix the band that already has steel does not keep the obsidian spear. The formula still says the stone cuts better, and the wear term is what saves the choice. A band whose game weight is higher than the nominal 0.3, and whose steel chain is not yet deep, can cross the other way. This audit did not re-run a world to count those bands.

As a knife the food coefficient is smaller and steel already wins at depth 0 (1.947 against obsidian's 1.550). That matches the step-2 result that iron knives gave way to steel.

Proposed fix, not applied. Use the same blend the fight uses, so food and war agree on what a working edge is:

```rust
Form::Spear => 0.35 + 0.65 * edge_q(m),
Form::Point => 0.35 * edge_q(m),
_ => 0.2 * edge_q(m),
```

`edge_q` today is pulled toward the durability cap, so this should land after finding 1, not before. Until then it would still rank steel first (0.870 against obsidian 0.507) and would also compress bronze and iron.

### 7. A bow pays the same no matter what the arrow is

Low. Sub-type D, in the form of a constant that replaces the material.

```838:844:civ-core/src/effects.rs
14 => {
    if kit.point != NONE {
        0.3
    } else {
        0.0
    }
}
```

The point slot's food multiplier is presence, not material (`effects.rs` around the `slot == 10` arm: "every point fills the slot the same way").

The order it should respect. A steel or bronze point on an arrow penetrates better than bone or flint. The bowstave in this model is not the item's material: the bow item is the string (`physics.rs`, the stave is wood, the item is the cord). Among strings that can be made (toughness at least 4 and fibrous at least 5), the score `tough + 1` is hide 7, flax 5.5, wool 5. If hide means sinew, that order is defensible. If hide means rawhide, linen should win, because rawhide stretches. The fight bonus does not use either number.

Impact. Once any point exists, the bow adds 0.3 to the fight term and the shield adds 0.25, regardless of material. The hunt weapon still scales with `edge_q`, so a steel spear is deadlier than a bone spear. Arrows are not. The string changes upkeep (hide wears less) and changes which cord is demanded. It does not change a battle. Severity stays low because the big weapon difference is on the spear, which is not flat.

Proposed fix, not applied. `0.3 * edge_q(mat(kit.point))` instead of 0.3, and leave the string ranking alone until hide is split into sinew and leather. Do not rank bowstrings by toughness if the intent is rawhide.

### 8. The vessel default

Low, rank 19. Sub-type G. The protection cap that used to sit in this section is rank 5, measured, and is written out below. Rank 8 (warmth) depends on it.

```542:550:civ-core/src/materials.rs
pub fn vessel_q(m: Mat) -> f32 {
    match m {
        Mat::Ceramic => 1.0,
        Mat::Hide => 0.6,
        Mat::Fibre | Mat::Flax => 0.5,
        Mat::Wool => 0.4,
        Mat::Clay => 0.25,
        _ => 0.3,
    }
}
```

Makeable vessels are clay (then fired to ceramic), and cord baskets or bags of fibre, flax, wool and hide. Their order, ceramic 1, hide 0.6, bast and linen 0.5, wool 0.4, raw clay 0.25, is a reasonable storage order. The default 0.3 ties wood, stone, every metal, bone, pitch, meat and the ores. Physics has no recipe that makes those into a vessel, so the tie is latent. It is still the pattern the class names: a default that gives unrelated materials one rank. A metal cauldron, if a later step added one, would store worse than a basket. Nothing in the year-350 stocks turns this default into a live tie.

Proposed fix, not applied. `_ => 0.0` in `vessel_q`, so an unexpected material is not a mediocre pot.

### 9. Births do not see a surplus

Medium, rank 6. Sub-types A and F. The year-200 save is still mostly on the top step (89.12% with the food term at 1, 6.87% at 0), so the rank holds.

```2207:2212:civ-core/src/world.rs
        let fed = ((food_ratio - 0.75) / 0.25).clamp(0.0, 1.0);
        // settled people with soft weaning foods (porridge, milk) space births closer
        let farm_share = (ec.farm_food / ec.food.max(1e-3)).clamp(0.0, 1.0);
        let sed = 1.0 + 0.6 * b.settled * farm_share;
        let births = rng.poisson(pop * p.birth_rate * fed * (0.8 + 0.4 * spare) * sed);
        let deaths = rng.poisson(pop * p.death_rate * (1.0 - 0.05 * b.herbal) + pop * p.starve_rate * (1.0 - food_ratio).powf(1.5));
```

`food_ratio` is set in `Econ::finish` (`world.rs:4851`). If the harvest covers need, the ratio is 1 and the rest goes into the store, up to `store_cap`. If it does not, the store is drawn down and the ratio is `(food + take) / need`, already at most 1. The birth clamp then divides the band of 0.75..1.0 into a ramp and sends everything above 1, which cannot occur, to 1.

The order it should respect. A band that eats and has grain in the store should raise more children than a band that only just covered this season, and a band at half rations should raise fewer than one at three quarters. Deaths already do the second half, with no clamp: starvation is `(1 − food_ratio)^1.5`. Births do not do the first half.

Why it binds. The ramp is 0.25 wide and the input is already capped at 1, so any band that ate this season is identical on the food term (F, then A). Spare labour `(0.8 + 0.4 × spare)` and settlement `sed` are outside the clamp. They are not a substitute for the store.

Measured, 3,354 live bands. `fed = 1` is 98.78% (3,313). `fed = 0` (food ratio at most 0.75) is 0.42% (14 bands; their food ratio runs from 0.506 to 0.743). The open interval is 0.81%. Saved food ratio: minimum 0.506, and the 10th percentile is already 1. Mean 0.998. Next season, from `economy` after one wild regrow, food ratio is 1 for 99.28%. Nobody is on the store cap (0.00%). Store cap per person: minimum 1.25, median 2.20, maximum 3.25, mean 2.21. The surplus is kept. Births ignore it.

The well-fed are not one fertility. Among the 3,313 with `fed = 1`, the factor `(0.8 + 0.4 × spare) × sed` is minimum 0.803, 10th percentile 0.863, median 0.927, 90th percentile 1.006, maximum 1.325. Spare there: median 0.280, 90th percentile 0.424, maximum 0.600. Settlement: median 1.006, 90th percentile 1.107, maximum 1.395. The middle 80% sits about ±8% around the median. The top is settled farmers. Baseline rates are `birth_rate` 0.014 and `death_rate` 0.006 (`params.rs`). At food ratio 1 the starve term is 0, so a typical well-fed band gains about `0.014 × 0.93 − 0.006 ≈ 0.007` people per person per season before disease. A hungrier neighbour on this map barely exists (14 bands), and a richer store does not move the 0.014.

Impact. Population differentials are not a picture of local abundance. They still track spare labour and, at the top, farming. The 14 bands under 75% of need have a birth floor of zero, which is a small clause of the same step, not a separate rank.

Proposed fix, not applied. Count store as plenty, and open the ramp so a full meal with an empty store is not already the maximum:

```rust
let plenty = (food_ratio + 0.25 * (b.store / need.max(1.0))).min(2.0);
let fed = ((plenty - 0.5) / 1.0).clamp(0.0, 1.0);
```

At food ratio 1 and an empty store, `fed` is 0.5. At a store of two seasons' need, `fed` is 1. At food ratio 0.5 and no store, `fed` is 0. Spare and settlement stay as they are.

### 10. Crowded camps all leave at the same rate

Medium, rank 7. Sub-type A. The year-200 save is stronger, not weaker: raw push is at least 1 for 63.81% of bands, median 1.68, maximum 22.6. That supports medium or higher. It stays medium because the year-350 median (0.78) is still under the cap.

```2740:2745:civ-core/src/world.rs
    /// How hard life at home pushes a band to look elsewhere: hunger, cold and crowding.
    fn push_out(&self, b: &Band) -> f32 {
        let c = b.cell(self.map.w);
        let crowd = self.crowd(c, b) as f32 / (b.pop as f32 + 10.0);
        (b.d.stress + 0.5 * crowd + b.pressed).clamp(0.0, 1.0)
    }
```

`crowd` (`world.rs:1584`) is other people on the same cell. The result is used as a sea-scout chance (`0.06 + 0.35 × push`, which is why a value above 1 was clamped), as fission friction `push × (pop / split_size).min(1.5) × ambition.exp()` (`world.rs:3964`), and in daughter-voyage scoring (`world.rs:4000`).

The order it should respect. A camp with a few neighbours should be less eager to leave than a camp with dozens on the same cell. Hunger should still push, and on this save hunger is not the term that binds.

Why it binds. The clamp is a probability bound for the scout draw. It is reached in ordinary play, and the other uses are not probabilities. They inherit the cap.

Measured. Raw push: minimum 0, 10th percentile 0, median 0.776, 90th percentile 6.16, maximum 21.57, mean 2.10. Raw at least 1: 46.15%. The median is under the cap. The upper half is pinned, so a just-crowded camp and one with raw push 6 to 22 share push 1. Saved stress is 0 for 88.46% of bands (see the fine list), so the pin is crowd, not famine. Other bands on the same cell: minimum 0, median 2, 90th percentile 19, maximum 46, mean 5.96. The typical camp is not stacked. The tail is.

Impact. Sea scouting, splitting and voyage scoring cannot tell those camps apart. A soft saturation would: raw 0.78 becomes about 0.54, raw 1 becomes 0.63, raw 6 becomes 0.998.

Proposed fix, not applied.

```rust
let raw = b.d.stress + 0.5 * crowd + b.pressed;
1.0 - (-raw).exp()
```

### 11. Protection piles half the world on 0.92

Medium, rank 5. Sub-type A. Moved out of finding 8 because the first pass treated it as a corner. Moved above births and push because the year-200 save is worse (69.75% on the cap, against 51.16% at year 350) and because rank 8 depends on this cap. A warmth change that leaves the best garment at 1 does not show up until this cap moves.

```891:892:civ-core/src/effects.rs
    let gq = q(k.garment, &garment_q);
    k.protect = (0.35 * fire + 0.5 * gq + 0.05 * kiln + 0.05 * axe * fire).min(0.92);
```

What it does. Fire is worth 0.35, a garment up to 0.50, a kiln 0.05, and an axe with a fire another 0.05. The sum is capped at 0.92. Cold need multiplies by `(1 − protect)`.

The order it should respect. A warmer garment should leave a band less cold, including a band that already has a fire, a kiln and an axe. Those are the bands that own the good clothes.

Why it binds. Warmth is 1 for hide and wool (finding 2), so fire plus garment is 0.85, plus kiln 0.90, plus axe 0.95, which becomes 0.92. The first pass said this only shaves 0.03 off a fully equipped band. That equipment is half the world.

Measured. `protect` at least 0.919: 51.16% of live bands at year 350, and 69.75% at year 200. They share one cold-protection number. Another garment, a kiln or an axe cannot move them. The earlier save is the worse one, which is why this is ranked above births and push.

Impact. Finding 2's warmth tie already makes hide and wool equal. If that tie is broken and the best garment stays at 1.0, fire + garment + kiln + axe is still 0.95 and the cap puts it back to 0.92. A split that drops hide to 0.80 (raw protection 0.85 with the same kit) would survive against a wool garment that still hits the cap. A split that leaves both near 1 would not. The cap should be raised, or removed, in the same change as the warmth table, or the warmth fix is invisible for this half.

Proposed fix, not applied. Drop the `.min(0.92)`, or set it above the best equipped sum (0.95 with today's weights, higher if wool is allowed past warmth 1). Do not leave a fully equipped good garment on the bound.

### 12. Four meetings, in a country with twenty-six neighbours

Medium, rank 9. Sub-type A. The draw itself was not resimulated. The numbers are the neighbourhood and the same-cell lower bound.

```28:28:civ-core/src/world.rs
const MAX_MEETINGS: u8 = 4;
```

Chance pairs are drawn with `teach_rate × kinship / (1 + travel_cost)` (`world.rs:3411`, `teach_rate` 0.3). A band that already has 4 such meetings is skipped (`world.rs:3431`). Same-cell travel cost is 0, so that chance is `0.3 × kinship`. Partner visits, after the cap, are not capped (`world.rs:3442`).

The order it should respect. A band in a thick settlement should hear more recipes in a season than a band with four neighbours, up to some limit of time. The limit should not sit under the ordinary number of neighbours.

Measured. Neighbours inside teach radius 2: minimum 0, 10th percentile 10, median 26, 90th percentile 50, maximum 110, mean 28.6. More than 4 neighbours: 95.74%. More than 13 neighbours, which is `0.3 × n > 4`: 82.89%. That second figure is degree, not dropped meetings. Travel cost cuts the wider draw, and this audit did not rerun it. The same-cell count is free of travel: other bands on the cell times `teach_rate` exceeds 4 for 16.46% (median cell has 2 other bands; the 90th percentile has 19). So at least that 16% are over the cap before anyone walks in from next door. The other 83% are candidates, not a measured loss.

Impact. Chance teaching in dense country saturates at 4. Standing partners still visit. The cap is inside ordinary density, which is why it is a finding, and the unsimulated travel cost is why it is not ranked above the bounds that were evaluated on every band.

Proposed fix, not applied. Grow the cap with the band instead of a flat 4, for example `4 + pop / 40`. A band of 40 then has time for five chance meetings, a band of 80 for six. Partner visits can stay uncapped.

### 13. Lessons pile up only at the top, and the prestige floor does not

Low, rank 13. Sub-type A. The first pass left prestige's 0.5..2 clamp as a social saturation. The floor binds more often than the ceiling. Neither is a material reversal.

```162:164:civ-core/src/world.rs
pub fn prestige_bias(src: &Band, dst: &Band) -> f32 {
    (1.0 + PRESTIGE_LEARN * (src.prestige - dst.prestige)).clamp(0.5, 2.0)
}
```

`PRESTIGE_LEARN` is 0.5, so the raw factor hits 2 when the teacher is at least 2 prestige ahead, and hits 0.5 when the teacher is at least 1 behind. The learn chance (`world.rs:3929`) is `(skill × 0.8 or 0.25 × bias).min(0.95)`. Showing a finer piece (`world.rs:3615`) is `(0.3 × bias).min(0.9)`.

The order it should respect. A more skilled teacher should succeed more often than a less skilled one, including when the listener already looks up to them. A much less prestigious teacher should do worse than one who is only a point behind, not the same.

Measured, 104,209 partner edges. `prestige_bias`: minimum 0.50, 10th percentile 0.500, median 0.997, 90th percentile 1.494, maximum 2.000, mean 1.016. Raw bias at least 2: 0.67%. Raw bias at most 0.5: 10.00%. The ceiling is rare. The floor piles "much less prestigious" with "one point less" on a tenth of edges. `0.3 × bias ≥ 0.9` never happens (0.00%): the show cap is a safety bound while bias stays at most 2, because `0.3 × 2 = 0.6`.

Teacher recipes with skill at least 0.25: 6,050,836. Of those, 18.95% would hit `.min(0.95)` if the lesson is relevant (the 0.8 branch). The test counted that branch, not every lesson. An irrelevant lesson is at most `0.25 × 2 = 0.5`, so it never hits 0.95. At bias 1, a relevant lesson is at most 0.8, under the cap. The cap trims the upper tail: at bias 1.5 it binds once skill is about 0.79, and at bias 2 once skill is about 0.59. A teacher at bias 2 and skill 1 would be 1.6 uncapped against 0.95 capped.

Impact. Ordinary teaching is not piled on 0.95. High-skill lessons from a admired teacher are. The prestige ceiling is fine. The floor is a small flatten of the bottom of the social range, kept in this finding rather than given its own rank.

Proposed fix, not applied. Keep the result a probability without a second clamp that erases skill:

```rust
let base = (skill * if relevant { 0.8 } else { 0.25 }).min(1.0);
let pl = 1.0 - (1.0 - base).powf(bias);
```

Bias and skill both still move `pl`, and `pl` stays at most 1. Widening the prestige floor (for example 0.25..2, or no floor) would let a large prestige gap keep ranking. Not required to fix the 0.95 pile.

### 14. A visit carries the same lot from a small pile and a large one

Low, rank 14. Sub-type A, and the cap is close to a physical load, which is why it is low.

```3696:3698:civ-core/src/world.rs
                // (v8) a carrying frame takes a bigger load, more for a better basket
                let fr = if from.d.kit.frame != NONE { 1.0 + vessel_q(mat(from.d.kit.frame)) } else { 1.0 };
                let q = (sf - 0.2).min(0.3 * fr);
```

The lot is only considered once stock is at least 0.3. Worth to the holder is already 0 at stock 1.3 (`want`), and stock itself is hard-capped at 1.5.

The order it should respect. A band with a heap of steel should be able to move more of it, per visit, than a band with a handful, up to what a person can carry. The frame already scales that load (1.0, or up to 2.0 with a ceramic basket).

Measured. "Lot-capped" is the share of holders with stock at least 0.3 whose `(stock − 0.2)` exceeds `0.3 × frame`. Steel 76.5% of 2,978, flax 90.3% of 2,383, bronze 69.4% of 399, tin 67.6% of 222, iron 65.7% of 1,990, copper 64.3% of 1,706, hide 62.8% of 1,493, wool 60.9% of 23, salt 52.6% of 1,402, fibre 44.0% of 3,263, grain 37.4% of 2,610, obsidian 23.1% of 13, flint 17.9% of 633, gold 0% of 319. Worth already 0 (stock at least 1.3): steel 6.6%, hide 15.5%, others about 0 to 1.6%. Stock at least 1.49: steel 0.6% of 3,352 holders, copper 0.2%, hide 0.3%, most metals 0%. Stocks are not tied. Copper median 0.68, maximum 1.50. Steel median 0.79, maximum 1.50. Gold median 0.30, maximum 0.394. Bronze median 0.39, maximum 0.794 (the top 10% sit on that maximum; this audit did not find a bronze refill that would explain it, and it is not claimed as one).

Impact. A band holding 0.5 steel and a band holding 1.2 send the same bundle on a visit. That is a load limit, and load limits are real, so the severity stays low. It does erase "more stock, more trade" across most metal and cloth holders. The hard cap of 1.5 is not the thing doing it.

Proposed fix, not applied, and only if a richer pile should move more than one basket. Let the cap grow with stock and still respect the frame, for example `(sf − 0.2).min(0.5 × sf.min(fr))`. A frame of 1 then moves up to half a unit, and a larger frame moves more. Leave it if the intent is one basket per visit regardless of the pile.

### 15. Farm warmth is full at 12°C

Low, rank 15. Sub-type A. Other terms still rank the land, which is why it is not medium.

```57:61:civ-core/src/world/farming.rs
    // a marked dry or cold season suits annual grasses; the ever-wet tropics less so
    let season = 0.6 + 0.4 * (m.tamp[i] / 12.0).min(1.0);
    let water = if m.river[i] { 1.0 } else { ((m.rain[i] - 0.15) / 0.35).clamp(0.0, 1.0) };
    let warm = ((m.temp[i] - 4.0) / 8.0).clamp(0.0, 1.0);
    base * season * water * warm * if m.river[i] { 1.3 } else { 1.0 }
```

Warmth is 0 at 4°C and 1 at 12°C. Off a river, water is 0 at rain 0.15 and 1 at rain 0.5. Seasonality saturates at a temperature amplitude of 12.

The order it should respect. A hotter field should outyield a cool one until the crop's real ceiling, which for wheat is well above a southern-English 12°C mean, and a wetter field should outyield a merely adequate one. Biome already does a share of this (grass 1, savanna 0.9, steppe 0.7, temperate forest 0.6, tropical forest 0.35).

Measured, 2,298 camps with `sow > 0.2`. Warmth at least 1: 99.8%. Warmth at most 0: 0%. On a river: 23.1%. Off a river and water already 1: 42.1% of all farmer camps, about 55% of the camps that are not on a river. Rain at those camps: minimum 0.165, 10th percentile 0.278, median 0.497, 90th percentile 0.807, maximum 1. The median sits on the rain-0.5 cap, so the wetter half is flat and the drier half is still graded. Amplitude at least 12: 0%. Field cells 2,283. Non-river fields with water at 1: 31.9%. Crop at field capacity: 10.7% of fields. Crop over capacity: minimum 0, 10th percentile 0.347, median 0.745, 90th percentile 0.929, maximum 1, mean 0.683.

Impact. Hot and cool farmland share warmth 1. River and biome still separate them, and dry land is still graded. The field-capacity curve is not saturated (median 0.74), so manure, seed and tools still move the crop. That part is fine. The warmth ceiling is a low agricultural ceiling reached by almost every farmer. Low, because the rest of the product still orders places.

Proposed fix, not applied, and only if hotter land should yield more. `((temp − 4) / 14).clamp(0, 1)` is 0.57 at 12°C and full only at 18°C. Water can stay a niche: rain past 0.5 is a lot of rain. If 12°C is meant to be "warm enough for this grass", leave the clamp and treat this section as the measurement, not a change.

### 16. Wool is refilled to one number, whatever the flock

Low, rank 16. Sub-type E. Eleven bands.

```544:548:civ-core/src/world/farming.rs
            // fleece: tame sheep and goats in cool lands grow wool worth spinning
            if b.tame > 0.4 && self.map.temp_winter(ci) < 10.0 {
                let wl = Mat::Wool as usize;
                b.stock[wl] = b.stock[wl].max(0.8);
```

What it does. A tame herd in a cool land sets wool stock to at least 0.8. Herd size does not enter. Non-metal use then drains about 0.02, so a refilled stock ends near 0.78. Hide and bone from the same herd do scale: `0.01 × herd.min(1)` added, then capped at 1.5 (`farming.rs:540`).

The order it should respect. A larger flock should yield more fleece. Tenfold is a real difference.

Measured. Herders: 50. `herd > 1` is 0%, so `herd.min(1)` never binds. `tame` at least 0.99 is 0%. Herd per person: minimum 0.026, median 0.195, maximum 0.853. Cool-land wool bands: 11. Their herds: minimum 0.041, median 0.181, maximum 0.580, about tenfold, and their wool stock: minimum 0.20, median 0.78, 90th percentile 0.78, maximum 0.78, mean 0.686. The constant is the stock. Flax has the same `.max(0.8)` (`farming.rs:462`). Flax stock is not piled there (median 0.72, 90th percentile 0.85, maximum 1.50), because flax is grown and traded, not only refilled. Grain's `.max(0.6)` after a real harvest (`farming.rs:453`) is a floor for that season, and grain still runs up to 1.50. Those two are noted so the lines are not missed. They are not separate findings.

Impact. Wool available to spin or trade does not track the flock. The population is 11 bands, so the rank stays low. Sails and garments that ask for wool (findings 2 and the sail note) meet a stock that cannot grow with the herd.

Proposed fix, not applied.

```rust
let hb = 0.15 * b.herd.min(2.0);
b.stock[wl] = (b.stock[wl] + hb).min(1.5);
```

A herd of 0.2 then adds 0.03 a season. A herd of 0.6 adds 0.09. The 1.5 cap is the same cap hide already uses.

### 17. The food-effort floor hits the expensive tail only

Low, rank 18. Sub-type A, small population. Not a flat labour price.

```2008:2008:civ-core/src/world.rs
        let max_e = (labour * 0.85 - upkeep_l).max(labour * 0.3);
```

Labour is `0.55 × pop`. Food effort is at most 85% of labour minus tool upkeep, and at least 30% of labour. The floor binds when upkeep, and farm labour on top of it, would otherwise push food effort under that 30%. `tool_upkeep` (`effects.rs:373`) is `sealed × rate × (1.15 − durability) × (1 + 0.3 × depth)`. `KIT_UPKEEP` is the score the kit picker subtracts. It is a scale, not a cap.

The order it should respect. A dearer kit should leave less labour for everything that is not food, which is what the depth term is for. It should not be forced back up to a food-effort floor so wide that the dearer kit gathers the same food effort as a cheap one. Depth should keep separating copper from steel.

Measured. Upkeep as a fraction of labour: minimum 0.084, 10th percentile 0.280, median 0.382, 90th percentile 0.504, maximum 0.689, mean 0.392. Upkeep above 0.55, which hits the `labour × 0.3` floor on tools alone: 3.64%. An upper bound that pretends every sower works a full field (`0.12 × sow + 0.08 × herd`) on top of upkeep is 15.06%. That 15% is not a hit rate. The depth factor on payoff tools, 31,774 of them: minimum 1.60, 10th percentile 1.60, median 2.20, 90th percentile 3.40, maximum 4.90, mean 2.38. Axe mean depth: stone 3.21 (24), copper 3.18 (806), iron 7.65 (264), steel 7.26 (2,209). Hunt: wood 3.14 (22), copper 3.42 (45), steel 8.58 (3,255). Steel axe labour from depth is about `1 + 0.3 × 7.26 ≈ 3.18`, copper about `1 + 0.3 × 3.18 ≈ 1.95`, roughly 1.6 times, before finding 1's wear reversal.

Impact. The floor erases food-effort differences for a few percent of bands, the ones whose tools are already the dearest. It does not flatten copper against steel. Left low. If the floor is "someone still has to gather", it is a biological bound and the 3.6% is the population that actually lives on it. No change is proposed unless those bands should be allowed to spend under 30% of labour on food when the kit is that expensive.

Spare's own clamp, `((labour − e_food − upkeep) / labour).clamp(0, 1)` (`world.rs:2049`), does not bind: spare is 0 for 0.03% and 1 for 0.00% (median 0.278, maximum 0.600). That one stays in the fine list.

### 21. Hafting an axe lets malleability stand in for toughness

Low, rank 21. Latent. Sub-types B and C. Same mistake as finding 1. Nothing in the current material table changes which edge becomes an axe.

```291:293:civ-core/src/physics.rs
            if em.props().hard >= 3.0 && em.props().tough + em.props().malleable >= 3.0 {
                return o(item(Form::Axe, em), 0.25 + 0.3 * cut_q(em));
            }
```

What it does. A shaft, a cord and an edge become an axe when the edge is at least 3 hard and the sum of toughness and malleability is at least 3. Malleability can fill a toughness the edge does not have.

The order it should respect. An axe head has to hold together. Toughness is that. Being easy to reshape is not a substitute, which is finding 1's point about wear.

Why it does not bite today. Malleability is 0 on every non-metal, so for those the sum is toughness. Obsidian is hard 5.5 and tough 1.5, so it fails the sum and it would also fail a toughness gate of 3. Copper, bronze, iron and steel are tough 9, so each passes on toughness alone. Tin is hard 1.5 and gold is hard 2.5, so both fail the hardness gate before malleability matters. No current edge is admitted by malleability that toughness would have refused.

Impact. None, until a material is added that is hard enough to be an edge, tougher than nothing, and malleable enough to cross 3 without being tough. That is why the rank is low, and why it is written down: the next soft malleable metal should not inherit the gate.

Proposed fix, not applied. Require `tough >= 3.0` beside `hard >= 3.0`. Do not add malleability into the sum. `durability-v2` does not change this line.

## Wrongly sized material differences

Sub-type H. Nothing here is retuned. Michael asked for the size of the step, not only the order, and plans a separate deep dive. This section flags candidates.

A ratio is first ÷ second, from `audit_material_ratios`. Three layers. The property is the 0..10 number in `PROPS`. The quality is `cut_q`, `durability`, `boat_q` and the rest. The applied figure is what a band actually gets: a yield multiplier from `kit_yields`, fighting strength `1 + 1.5 × weapon_q`, the casualty factor `0.6 + weapon_q`, cold-season need, sea survival, or upkeep. User skill is 1 and recipe quality is 0, so the gap is the material alone.

The applied layer is the one that matters. A 1.6× cutting score that becomes a 1% plant yield is compressed even if the score looked reasonable. An order that is already wrong (steel wearing out before copper, wool tied with hide) is still listed where the size is also wrong.

Real-world numbers below are ranges, with a confidence, not a cell copied out of one table. Where a study was not re-read for this pass, the text says so. The deep dive should check the pages. Do not paste a hardness ratio onto calories: wood is soft, and once an edge stops rolling, more hardness does not cut the first stroke many times faster. The large real gap on an axe is sustained work, including the time lost to a crumpled edge. That sustained gap is still several-fold for a good hard metal against stone or soft copper. It is a small step from bronze to steel, and it is not 10%.

### Sources

Hardness. The comment on `Props` says the 0..10 figures are loosely Mohs-like. Copper at 3 and flint at 7 match Mohs. Vickers hardness is the measured scale for metals, and it is not a calorie ratio. The tool-relevant pair is a work-hardened copper edge, roughly 100–130 HV and higher if the copper is arsenical, against a historical steel edge, which was often a few hundred HV. That is about 3–6×. About 15× is only an outer bound: soft annealed copper near 40 HV against a modern quenched edge near 600 HV and up. An earlier draft of this audit stated 40–100 HV and 600+ as if they were certain, and attributed them to Michael. They are not in what he has written, and they compare the wrong pair of edges. Cold-worked tin bronze is often around 150–250 HV and bloomery iron around 100–180 HV, so bronze against copper is about 2–3× and unsteeled iron against a good bronze is about 1, give or take (Scott, *Metallography and Microstructure of Ancient and Historic Metals*, Getty, 1991). Confidence medium on the 3–6× tool pair, low on any single HV number, medium on bronze and iron, because the alloy and the hammering move them.

Felling. Saraydar and Shimada, *American Antiquity* 36 (1971) 216–217, measured one steel axe against one ground-stone axe: about 6.4 times faster and about 5.1 times less energy per inch of wood. The same pair, clearing plots, was about 3.6 times faster and about 3.3 times less energy (*American Antiquity* 38 (1973) 344–350). Mathieu and Meyer, *Journal of Field Archaeology* 24 (1997) 333–351, put the chopping studies they reviewed at roughly two to four and a half times, and they say species, diameter and the axe move the result as much as the material. Their own trial did not keep the order stone, then bronze, then steel, with a gap at each step. Their bronze axes felled about as fast as steel at every trunk diameter, and only stone fell behind as the trees got thicker. Coles, *Experimental Archaeology* (1979), covers the 1950s Draved felling (Iversen; Steensberg): flint against a modern steel axe, a much longer job than a few minutes of steel. Jørgensen's 1985 Draved trials are a later flint-axe series. Coles 1979 does not survey them. Cite the two separately. No single factor is copied out of one table. Confidence medium on the factor for steel against stone, high that that gap is not 10–20%, and high that bronze against steel is a small step rather than another several-fold one. The bronze-against-steel result is the one in the durability proposal (its section "Against the real numbers"), which read the same paper.

Boats. McGrail, *Boats of the World* (2001), and the Ferriby and Dover plank boats: cargo of a sewn-plank hull is discussed in tonnes, a one-log dugout often in a few hundred kilograms. Confidence medium that load and freeboard differ several-fold, low on a survival probability. A skin boat and a small dugout can be peers. The tie is not, by itself, a size error. The plank step is.

Nets and cloth. European nets were hemp, flax or nettle, not wool. Wool stretches and weakens when wet; bast holds. No clean ratio was found. Confidence medium that wool should lose to bast by more than a tie, low on a number. Linen against bast at a small step is plausible. A linen garment is often around half as warm as a wool one of similar cut. Confidence medium. Fur against wool depends on whether `Hide` still has the hair. Confidence low until that is answered.

Fire. A wood fire's hot zone is commonly 600–800°C; 650 is fair. Charcoal runs hotter by a few hundred degrees; +300 is fair. Bloomery iron is widely put around 1100–1300°C rather than a hard gate at 1400 (Tylecote, *A History of Metallurgy*; Pleiner, *Iron in Archaeology*). Confidence medium. A miss of 40°C is sharper than a real furnace, where the draught matters more than a few tens of degrees. Confidence medium.

Pots. Boiling starch raises what the body gets out of it. The extra over dry cooking is real and not an order of magnitude. Confidence low on the percent. Meat that is already roasted does not need the pot. The sim agrees, which may be right.

### Sizing table

Promoted rows are also in the main ranked table (ranks 2, 4 and 20). The others stay here so the deep dive can see them without a second finding.

| S | Where | Sim ratio | Real-world ratio | Way | Impact | Later |
|---|---|---|---|---|---|---|
| 1 | `effects.rs:554` `0.1 × axe`, `effects.rs:564` `0.2 × axe`, `materials.rs:532` `cut_q` | Steel/copper plants 1.009 (dig, hammer and axe) and 1.013 (axe alone). Farm, with a digging stick and a knife of the same metal, 1.119. Steel/stone farm 1.192, plants 1.025. Bronze/copper farm 1.069, plants 1.006. | Sustained felling, steel or bronze against stone: several-fold (Saraydar and Shimada 1971; Mathieu and Meyer 1997 on the stone gap; Draved). Bronze against steel: about the same felling speed at every diameter (Mathieu and Meyer 1997). Not ~1.1–1.2 for the stone step, and not a several-fold steel-over-bronze gap. | Compressed | The metal inside an axe barely changes food. Having the axe at all is only +6% plants (1.45 with dig and hammer, 1.538 with a steel axe). A farmer feels the knife and the axe more (farm 1.249 steel against 1.116 copper) and it is still about a fifth, not a different tool. | Widen the axe's share of clearing, or widen the quality itself, until a steel axe and a copper axe differ by a sustained-work factor. Do not copy Vickers onto the calorie multiplier. |
| 2 | `materials.rs:101`, copper `materials.rs:191`, steel `materials.rs:204` | Hardness steel/copper 2.00, sharp 1.50, `cut_q` 1.625. Steel/iron hard 1.09, `cut_q` 1.219. Bronze/copper hard 1.67, `cut_q` 1.354. | Vickers, a work-hardened copper edge (about 100–130 HV, more if arsenical) against a historical steel edge of a few hundred HV: about 3–6×. About 15× only as an outer bound (annealed copper against a modern quench). Worked bronze against copper about 2–3×. | Compressed | This is the scale under row 1. A Mohs-like 0..10 number cannot hold a three-to-sixfold hardness gap, and `cut_q` then averages it with sharpness and divides by 10. Steel's whole cutting advantage over copper is 63% before any food coefficient. | If the deep dive keeps a property table, store a per-job performance (felling, butchery, edge life), not a linear Mohs number asked to do every job. |
| 3 | `band.rs:701`, `band.rs:740`, `world/conflict.rs:49`, `world/conflict.rs:138` | Steel/copper spear `weapon_q` 1.124, strength `1 + 1.5 × weapon_q` 1.068, deaths `0.6 + weapon_q` 1.072. Spear and axe together: strength 1.084. Steel/iron strength 1.029. Steel/stone strength 1.183 (spear) and 1.241 (spear and axe). | A steel spear against a copper one is a different weapon in use. Edge life differs by the hardness gap above, not by 7%. No casualty table is cited; the claim is the direction. Confidence medium. | Compressed | Offsets do it. A spear is `0.35 + 0.65 × edge_q`, then strength adds 1 and casualties add 0.6. The metal moves a number that is mostly a constant. Flint against obsidian goes the other way by 2% of strength (1.020), which is durability, and is small enough to leave. | Shrink the additive floors, or let edge life scale the fight, so the metal is visible. The casualty cap of 0.45 is not what flattens this: the high end of a loss on the save was about 0.28. |
| 4 | `effects.rs:366`, `band.rs:734` `KIT_UPKEEP = 0.06` | Depth 8 multiplies every upkeep by 3.40, steel and copper alike. Nominal axe score, empty kit: steel 0.559 at depth 0, 0.318 at depth 8. Copper 0.467 and 0.255. The depth swing on steel (0.241) is 2.6× the steel-copper gap (0.092). Crop view (harvest all crops, then the 0.5 prior): steel 4.627 against copper 3.889 at depth 0, and steel still leads at depth 8 (4.386 against 3.678). `KIT_UPKEEP × wear` at depth 0 is 0.010 for steel and 0.009 for copper. | A long chain should cost labour. The question is the cost beside the food the better metal creates. | Out of scale | On the nominal mix, which has no crops, chain depth prices more food than the best metal upgrade earns. On a farm the metal still wins the score, and the yield itself is only the 1.12 of row 1. `KIT_UPKEEP` was set as upkeep 0.02 over the 0.35 a unit of game quality adds. That prices "a tool" against "no tool". It does not price steel against copper. | When the food coefficients move, re-price depth against the new gap. Do not flatten depth. The save already separates the metals by depth (copper axes near 3.2, steel near 7.3). |
| 5 | `materials.rs:537` wear, copper against stone | Upkeep copper/stone 0.265. A stone axe costs 3.8× the labour of a copper axe. Axe quality copper/stone is 1.48, and plants only 1.016. | A ground stone edge holds against wood. Worked copper mushrooms and has to be hammered back. Stone is not four times as hungry for labour as copper. Confidence medium. | Exaggerated | Finding 1's malleability term, seen from the stone side. Copper is capped at durability 1 because it is easy to reshape. Stone (tough 7, malleable 0) sits at 0.58. The food side of the same pair is row 1, compressed. | Edge life from hardness and toughness, as finding 1 already asks, shrinks this exaggeration and should be checked against stone as well as against steel. |
| 6 | `effects.rs:872` plank `+0.25`, `world.rs:1804` radius, `world.rs:1824` fish cap, `world.rs:2760` survival | Plank/dugout quality 1.077, `sea_range` at no experience 1.063, survival on a 10-cell hop 1.013 (0.969 against 0.957), fish-weight cap 1.050, fish radius 1.000 (both 3). Hide/dugout quality 1.000. An outrigger adds 0.15 of safety: dugout 0.929 becomes 1.079, plank becomes 1.150 and sits on the hazard floor 0.3. Ship quality equals the plank. Sail ×1.6 and ship ×1.25 are separate and do move range. | Planked cargo against a one-log dugout: several-fold (McGrail 2001). A skin boat can match a dugout. | Compressed | The hull upgrade is smaller than the outrigger bonus, and smaller than one fish-radius bin (radius 4 would need quality 1.25, past the cap of 1). A 10-cell crossing is about 1% safer in a plank boat. With an outrigger both hulls are on the floor, which is finding 4; the `+0.25` was already too small before that floor. | Let hull class move load and hazard by more than the outrigger does. Leave the sail's 1.6 alone unless a voyage trial says otherwise. |
| 7 | `materials.rs:563` `garment_q` | Wool/hide warmth 1.000. Cold need at −6°C, with a fire, 1.150 for both. Flax/wool warmth 0.550, and need 1.375 against 1.150 (1.196). Full kit (fire, kiln, axe) puts both wool and hide on the 0.92 cap, need 1.080. | Linen around half of wool: the 0.55 is plausible. Wool against leather, if hide has no hair, perhaps 1.5–2× loft. Wool against fur can be about 1. | Erased, if hide is leather | The ceiling is why the size cannot appear. Section 2 already keeps the hide. A loft table that stays near 1 is then eaten by the protection cap in section 11. | Same direction as the warmth finding. Decide what hide is before picking a ratio. |
| 8 | `materials.rs:558` `mesh_q`, `effects.rs:556` `0.9 × net` | Flax/fibre mesh 1.167, fish from a net alone 1.070, trap 1.131. Wool/hide mesh 1.000, fish 1.000. | Linen against bast: a small step is fair. Wool against bast: wool is the wrong wet fibre, by more than a tie. | Fair for flax/fibre. Compressed for wool | The 0.9 coefficient keeps more of a mesh gap than the axe's 0.1 does. The tie is the problem, and it is already a finding. A mesh of 2× would still be about 1.5× fish, `(1 + 1.8) / (1 + 0.9)`. | Product of fibre and strength, as that finding says. Do not also inflate the 0.9 unless nets should dominate the catch. |
| 9 | `materials.rs:542` `vessel_q`, `effects.rs:554` `0.3 × vessel`, `effects.rs:887` digest | Ceramic/hide quality 1.667. Plants from the pot alone 1.102. Cooked grain 1.450 against 1.300 (1.115). Cooked meat 1.200 for both: the pot does not touch it. | Boiling against dry cooking of grain: a real step, low tens of percent, not twofold. Confidence low. Meat unchanged is plausible. | Mild, if anything compressed for grain | Not promoted. The binary ceramic flag is the cooking effect. The 0.3 storage term is a tenth more plants. | Leave it unless the deep dive wants porridge to matter more than about a tenth. |
| 10 | `materials.rs:571` `fire_temp`, `physics.rs:476` bellows +250, iron ore `materials.rs:197` at 1400, copper ore `materials.rs:185` at 1150 | Charcoal/wood in the open 1.462 (+300°C). Ceramic/stone refractory 2.25, but charcoal temperature only 1.180 (1310 against 1110), because the fuel is most of the number. Ceramic against clay is +40°C. Charcoal and stone miss copper by 40°C (1110 against 1150). Charcoal and ceramic miss iron by 90°C (1310 against 1400). Bellows then overshoot iron by 160°C (1560). | Open wood and the charcoal step: fair. Bloomery practice is often 1100–1300°C, so 1400 is a bit high. A 40°C cliff is sharper than a draught-driven furnace. | Temperatures mostly fair. The cliff is exaggerated. The ceramic-over-clay bonus is tiny. | Order of the ladder stays right: stone does not smelt copper, clay does, iron wants bellows. The gate is a razor. | If iron should sometimes reduce in a good furnace without bellows, lower 1400 toward the bloomery range, or stop treating refractory as +40°C per point of "more ceramic than clay". Do not reorder the fuels. |

### Not a sizing error

Iron against bronze is about 1 on purpose, almost. `cut_q` 0.985, axe term 0.994, farm 0.996, spear strength 0.998. Unsteeled bloomery iron and a good tin bronze really are that close. The deep dive should not pull them apart to make the tech tree feel like progress. Bronze against steel is a small step in the felling trials as well, so steel is not a missing several-fold rung above bronze. The large step is stone, or soft copper, up to a good hard metal, and row 1 is why that step does not arrive in the food.

Flint against obsidian: `cut_q` 0.927 (obsidian sharper by 8%), game from a spear 0.976, strength 1.020 the other way. Obsidian can be many times keener in edge radius. Both butcher meat. An 8% food gap is a fair functional step and a compressed acuity. Not promoted. The fight's 2% is durability, which is the right property and a modest size.

Linen's 0.55 against a wool cloak at 1 is a plausible warmth ratio. The bug is the shared ceiling with hide, not the flax special case.

### 18. The metal in the tool barely changes food or a fight

High. Rank 2. Sub-type H, with E (the offsets) and F (the 0..10 divisor). Rows 1–4 of the sizing table, and row 5 for the exaggeration on the stone side. Not applied.

What it does. `cut_q` is `(0.6 × sharp + 0.4 × hard) / 10`. A spear's hunt term is `0.35 + 0.65 × cut_q`, and game is `1 + 1.4 ×` that, times a knife term. Plants add `0.1 × axe`. Farm adds `0.2 × axe` and `0.35 × cut`. Fight uses `edge_q`, then adds 0.35 inside the spear, 1 inside strength and 0.6 inside the death rate. Upkeep multiplies by `1 + 0.3 × depth` and does not know which metal was worth the chain.

The order it should respect. A good hard metal beats a ground stone or a soft copper edge over a long job, by a large step. Bronze against steel is a small step: the felling trial had them about equal. Iron roughly matches a good bronze. Bronze beats soft copper. The order in `cut_q` is already steel, obsidian, flint, bronze, iron, copper, stone, except iron a hair under bronze. The size is the failure, and it is the stone-to-metal size, not a missing steel-over-bronze rung.

Why it is wrong. Steel against copper is 2× on the hardness property, 1.63× on `cut_q`, 1.19× on the axe term, 1.009× on plants once a digging stick and a hammer are in the kit, 1.12× on farm yield, 1.14× on game from a spear, and 1.07× on fighting strength. Steel against stone, the pair the felling trials measured as several-fold, is 1.19× farm yield and 1.18× spear strength. That sustained-work gap is several-fold. The tool-relevant Vickers gap is about 3–6×, with about 15× only as an outer bound, and it should not be the calorie target. The sim is also far below the sustained-work target for stone against a hard metal. Bronze against steel should stay small. The same formulas exaggerate copper against stone on labour: upkeep 0.265×, because copper's malleability is scored as durability and then clamped.

Depth makes the small food gap the one that loses. On a nominal band the steel axe is worth 0.559 at depth 0 and 0.318 at depth 8. Copper is 0.467 and 0.255. Moving a steel axe from a new chain to depth 8 costs 0.241 of that score. Switching copper for steel, at the same depth, gains 0.092. The chain is the larger term. At depth 8, bronze's score is 0.319 and steel's is 0.318, which is finding 1's wear reversal sitting on top of a food gap too small to cover it. A crop-heavy band still prefers steel (4.386 against bronze's 4.134 at depth 8). The yield ratios do not change with that view. Only the weight of farm against wild food does.

Impact. The tech ladder in the story is stone, copper, bronze, iron, steel. In the food and in a fight, stone to steel is a fifth or less, and copper to steel is a few percent of plants and about 7% of strength. Bands keep the tool they can maintain. They do not live differently because the edge is steel. `1 + 0.3 × depth` remains a real spread. It is simply priced against a material gain that cannot pay it.

Proposed direction, not applied. Calibrate the axe and the blade to sustained work, including resharpening, not to Vickers and not to a 10% farm bump. The 0.1 and 0.2 coefficients cannot express a several-fold gap no matter how `cut_q` is rewritten: the whole axe term is at most 0.1 on plants (1.45 against 1.55 if the axe ran from 0 to 1). Either the coefficient grows, or clearing is its own term that can move by more than a tenth. Fight offsets should shrink so a copper spear and a steel spear are not within 7%. Re-price depth after that, so a long chain is expensive and a better edge is still worth making. Iron and bronze should stay within a few percent of each other.

### 19. A plank boat is eight percent more boat

Medium. Rank 4. Sub-type H, on top of the tie and the floor in the boat finding (rank 3). Not applied.

What it does. A plank or a ship takes `boat_q + 0.25`, then stops at 1. Wood's `boat_q` is 0.929, so the plank is 1.000. Hide is the same 0.929 as wood. Survival, range and the fish square all read that one number. An outrigger adds 0.15 of safety on top, and the hazard then floors at 0.3.

The order it should respect. A skin boat and a dugout are both small boats. A sewn or pegged plank hull carries more and stands higher in a sea than a one-log dugout. A ship is that hull, larger, which the sim already says ("no sounder, only bigger") and then implements as the same quality plus a 1.25× range when a sail is set.

Why it is wrong. Plank against dugout is 1.077 in quality, 1.063 in range with no experience, 1.013 in survival on a 10-cell crossing, 1.050 in the fish-weight cap, and 1.000 in fish radius. The outrigger's +0.15 is twice the hull's +0.071, and with it the plank's hazard is the floor (0.300) while the dugout's is 0.321. The radius cannot move: radius 4 needs quality 1.25, and nothing exceeds 1. The sail's 1.6× and the ship's 1.25× do create a range gap. The timber of the hull does not.

Impact. Once a band has any sound boat and an outrigger, a plank does not keep the crew safer and does not fish a wider square. Voyage distance still moves, mostly from the sail and from skill, not from the hull class. That matches the year-350 save, where almost every boat was already on the floor. The size error is why a fix that only unties wood and hide, and leaves `+0.25`, still will not make a planked hull a different machine.

Proposed direction, not applied. Give the plank and the ship a seaworthiness and a capacity that can exceed the outrigger's 0.15, and that can change the fish square without relying on `round`. Leave hide against wood close if hide means a real skin boat. Do not start from a tenfold hardness gap: boats are not edges, and 15× was only ever an outer bound on annealed copper against a modern quench. Several-fold cargo, and a clearly safer hull in open water, is the scale to check against McGrail and the reconstructions.

### 20. The furnace gate is a few tens of degrees

Low. Rank 20. Sub-type H. The order of the ladder was already judged intact. This is the size of the steps. Not applied.

What it does. `fire_temp` is the fuel's `fuel_t` plus `40 × refractory` of the lining. Bellows add 250°C, and only in a closed charcoal fire. Copper ore reduces at 1150, iron ore at 1400.

The order it should respect. Wood fire, then charcoal, then a clay or ceramic enclosure, then bellows. Copper smelts before iron. A stone hearth is a poor furnace. That order is what the sim does.

Why the size is off, narrowly. Charcoal against wood is +300°C, which is fair. Clay against stone on charcoal is +160°C (1270 against 1110), which is a real enclosure. Ceramic against clay is +40°C (9 refractory against 8). The copper gate sits 40°C above charcoal-and-stone (1110 against 1150), so a stone lining fails and a clay one (1270) succeeds by a wide margin the other way. Iron sits 90°C above charcoal-and-ceramic (1310), so bellows are mandatory, and they then land 160°C over the gate. Bloomery work is often described a hundred degrees cooler than 1400, which would let a good furnace touch iron without the blast, or would make the blast a help rather than a switch. A real furnace is not a 40°C razor. Draught, charge and time all move the temperature by more than the gap that decides copper here.

Impact. The story "clay smelts copper, stone does not, iron needs bellows" is intact and is probably the intended gate. What is exaggerated is how little a lining error, or a slightly cooler fire, has to be before the ore does nothing. What is tiny is ceramic over clay: one refractory point, 40°C, in a formula that has already added 320°C for clay's 8. If ceramic is meant to be a better furnace rather than a lining that merely does not melt, this does not say so.

Proposed direction, not applied. Keep the order. If the deep dive wants bellows to be important but not the only way to iron, move 1400 down into the usual bloomery range, and do not let one refractory point be the whole ceramic improvement. Leave the open-fire temperatures.

## Formulas that would arrive with skills step 4

PR #12, branch `origin/skills-step4`, does not edit `materials.rs`. The quality functions above are unchanged. What it adds is `civ-core/src/beliefs.rs` (new) and a belief-shaped kit choice in `effects.rs` and `world.rs`.

`material_term` is the prior's picture of a tool. It copies today's formulas:

- a spear, point or edge in the hunt slot uses `cut_q`, not `edge_q` (finding 6 travels into the prior)
- an axe uses `0.5 * cut_q + 0.5 * durability` (finding 1 travels into the prior)
- a net or trap uses `mesh_q` (finding 3)
- a boat, outrigger, plank or ship uses `boat_q` (finding 4)
- a bow or shield uses `tough + 1` (finding 7)
- the point slot is the constant `1.0` for every point

`food_prior` stores a ratio `mean / material_term` and applies that ratio to a new tool. Two tools with the same term get the same prior mean. Copper, bronze and iron axes already sit within 0.005 of each other on the axe term once durability is capped (0.740, 0.825, 0.820), so a belief learned on bronze transfers to iron almost unchanged. Every point transfers exactly, because every point's term is 1. That is finding 7 written into the prior: the band cannot come to believe that a steel arrow feeds anyone better than a bone arrow, because the table has no place to put the difference.

`fail.clamp(0.0, 1.0)` in the new belief update is a probability bound. It is not this class. `PRIOR_N0` of 1.0 shrinks a thin prior toward the band's average ratio. That is a shrinkage weight, not a material ordering. No new cap on a material quality was found in the step 4 diff.

## Checked, and fine

Most of the 96 clamps are limits that should bind, or that nothing reaches. Grouped by job. Line numbers are the `civ-core` ones unless noted.

**Display and input, not the sim.** `native/src/ui.rs:566`, `ui.rs:592`, `render.rs` colour and zoom clamps, `cli.rs:137` and `main.rs` map-size clamps. They do not rank materials.

**Map generation.** `map.rs` shelf, rain, latitude and coast clamps (`map.rs:450`, `:607`, `:689`, `:919`, `:940`, `:949`). They shape climate. They do not compare technologies.

**Trait bounds that are supposed to bind.** Curiosity, ambition and craft are clamped on birth and on inheritance (`world.rs:1009`, `:1014`, `:4044`, `:4045`, `:4052`, `:4117`). On this save the clamps barely bind. Curiosity at most 0.30: 0.12%. Curiosity at least 2.50: 0.27%. Ambition at most −1: 0.15%. Ambition at least 1: 0.09%. Curiosity itself: minimum 0.30, median 0.955, maximum 2.50, mean 1.027. Prestige learning's ceiling of 2 binds on 0.67% of partner edges. That ceiling stays fine. The floor of 0.5 binds on 10% and is written into finding 13, not left as reasoning.

**Demography.** The birth clamp is finding 9, not fine: the food term is a step, hit by 98.78% of bands. What stays fine is everything around it. Stress (`world.rs:4862`), `((1 − food_ratio) × 2 + 0.15 × cold).clamp(0, 1)`, is 0 for 88.46% and 1 for 0.03% (one band, food ratio 0.506). Raw stress above 1 is that same one band. The cap would bind at food ratio 0.5 with no cold. It is a safety bound this world does not reach. Deaths still use raw `(1 − food_ratio)^1.5` with no clamp. Spare is 0 for 0.03% and 1 for 0.00% (median 0.278, maximum 0.600). The experiment cap `poisson(lam).min(6)` (`world.rs:2238`) never binds: experiments at least 6 is 0%, and `lam` at least 6 is 0% (median 0.266, maximum 1.318).

**Conflict.** `world/conflict.rs:49` uses `food_ratio.min(1)`, which is idle because `Econ::finish` already caps food ratio at 1. Weapon quality: minimum 0.148, median 1.179, maximum 1.179, mean 1.150. The pile at 1.179 is steel in the kit (3,255 of 3,354 hunts), not a clamp. Loser loss before `.min(0.45)` (`world/conflict.rs:138`), at the high draw 0.16 and lethality 1: maximum 0.285. Bands whose high-end loss exceeds 0.45: 0. The winner cap of 0.2 is above the winner's draw (at most `0.05 × (0.6 + weapon_q)`, about 0.09). Tension on partner edges within 2 cells: 30,049 edges, none at 10 or above (median 0.198, maximum 1.280), so the confront cap `(0.05 × aggression × tension).min(0.5)` (`world/conflict.rs:61`) does not bind at aggression 1. Fight rankings are not crushed by these caps.

**Farming.** Warmth and the wet half of the water niche are finding 15, because 99.8% of farmers are on warmth 1. What stays a real biological limit: crop over field capacity, median 0.74, at capacity 10.7%. Wild plants over their K, on occupied cells as saved (after harvest, before regrow): 1,676 cells, at least 0.95 is 1.31%, at most 0.05 is 9.37%, median 0.37, maximum 1.26. Not piled on K. Harvest saturation after one copied wild regrow, `phi = h / ys` from the public `economy` (crops not regrown): plants mean 0.088, share at least 0.80 is 0%; game mean 0.179, 0.7% at least 0.80; fish mean 0.145 (2,027 bands), 0.4%; small game mean 0.108, 0.2%; farm mean 0.089 (2,306 bands), 0.1%. The derivative `1 − phi` is about 0.85 to 0.91, so a tool multiplier still changes the take nearly linearly. Store cap binds for 0% of bands. Food technology is not flattened.

**Herd and spoil.** `herd.min(1)` does not bind (0 of 50 herders). Spoil's temperature term `(t / 25).clamp(0, 1)` (`world.rs:2025`) is pinned for 23.55% of bands in the season just run. Temperature of that season: minimum −22.1, median 19.2, 90th percentile 26.5, maximum 28.3, mean 16.0. The range above 25°C is a few degrees. Left as fine: the cap is near the actual maximum, and little is erased.

**Skill stored in 0..1.** `effects.rs:330`, `knowledge.rs:596` and the following lines. On this save, 257,703 recipes, none outside 0..1, and 29.63% have skill at least 0.99. That is the practice asymptote `skill × (1 − skill)`, a real "fully practised" limit, not a clamp that moves a stored skill onto the bound. Quality within 0.01 of skill is only 5.35% of 236,375 recipes, so quality is not stuck on skill. `use_skill`: 107,328, none outside 0..1, 44.68% at least 0.99. `chain_m` on payoff tools: 31,774, the floor 0.05 binds for 0.00%, and 26.96% are at least 0.999. Maker waste and failure ride that chain. The floor never binds. The top is practised chains, the same shape as skill.

**Qualities whose clamp never moves a real input.** `cut_q` max is 0.820. `boat_q` max is 0.929. `mesh_q` max is 0.933. `fire_temp` is not clamped, and charcoal stays 300 degrees above wood under every lining (open 950 against 650, ceramic kiln 1310 against 1010). Kiln rank by refractory is ceramic 9, clay 8, stone 4. Charcoal plus stone is 1110, under copper ore's 1150. Charcoal plus clay is 1270 and plus ceramic is 1310, both over it. Iron ore at 1400 still needs bellows. That ladder is right. The size of the steps is not "fine": section 20. Charcoal and stone miss the copper gate by 40°C, and ceramic over clay is +40°C.

**Sails, among sails that exist.** The score is toughness, plus 10 if the cloth is not plant-fibre matting (`effects.rs:1255`). Makeable sails are linen 14.5, wool 14, mat 3. Linen over wool over matting is the right order. The +10 is large and would also prefer a hide sail, but hide is not a sail recipe.

**Hammers.** The kit ranks a hammer by hardness alone (`effects.rs:1253`), which would let a brittle hard stone beat a tough one. The only hammer recipe (`physics.rs:153`) requires `hard >= 5.0 && tough >= 5.0`, so flint is excluded and the ranking cannot bite today. That cobble is stone. Stone and steel both have hardness 6, and there is no steel hammer recipe. Food treats "has a hammer" as 1, so which stone it is does not change a yield.

**A learner's starting quality.** `world.rs:3937` stores `(QUALITY_TEACH * tq).min(0.3)` on a new recipe whose skill is 0.3. `QUALITY_TEACH` is 0.5 (`world.rs:140`), so every teacher at quality 0.6 or above hands over the same 0.3. That is the rule that quality does not exceed skill (`world.rs:132`: a recipe is never done better than it is known), applied at the learner's starting skill. The same rule is what fades and showing-off already clamp to (`world.rs:2287`, `world.rs:3625`). Probably intended. Left as fine.

**The five formula clamps.** `cut_q`, `durability`, `boat_q`, `mesh_q` and `garment_q` (`materials.rs:534` through `materials.rs:568`) are the clamps this audit already covers. `cut_q` never reaches 1 (obsidian 0.820). `boat_q` tops out at 0.929 and `mesh_q` at 0.933, so those two clamps are idle; the mesh tie is the formula, not the clamp. `durability`'s ceiling is rank 1. `garment_q`'s ceiling is rank 8. Nothing else of that shape was found.

**Shafts and digging sticks.** Every shaft scores 1.0 (`effects.rs:1247`). The only shaft physics allows is wood. Nothing is flattened that can exist.

**Cord as a food tool.** The cord slot is ranked by `tough + 1` (`effects.rs:1252`). Makeable cord is hide 7, flax 5.5, wool 5, fibre 4. Yields use the cord as present or absent, not as a quality. The ranking changes which material is demanded (rawhide thongs), not how much food the cord multiplies. If hide is sinew, preferring it is reasonable. Not raised as a finding. The bow string uses the same number; see finding 7.

**Marks.** `durability + 1` for ceramic (`effects.rs:1263`) makes a fired token beat wood and bone, which is the point of firing it. Wood's tally scores 0.500 and bone's 0.417, so a stick beats a bone tally because wood is tougher in the table. Wood also rots. The margin is small and the fired token wins whenever it exists. Left as fine.

**Cooking.** Digest pays a ceramic pot and not a hide bag. You cannot boil in a bag. The binary check is the right shape.

**Metal scrap in `marginal_mats`.** Every metal is reported at 0.02 and every other material at 0.06 (`effects.rs:812`). That would flatten copper and steel into one scarcity inside that vector. The kit choice prices labour through `tool_upkeep`, and the depth factor is not flat (finding 17). Stocks are not tied either: gold median 0.30, bronze 0.39, copper 0.68, steel 0.79. Equal draw rates are not producing equal piles. Left as fine on those numbers, not on the reasoning alone.

**Brine's durability floor.** Raw durability 0 is raised to 0.05. Brine is not a tool.

**Labour price, apart from the 3.6% floor.** `1 + 0.3 × depth` separates the metals (finding 17). `KIT_UPKEEP` is a subtracted scale in the kit score, not a cap. No second flat rate was found that piles copper and steel onto one labour cost. The wear residual in finding 1 is the term that does that, and it is already ranked.

**Sea range.** `sea_range` (`world.rs:2765`) is not floored. On this save it runs from about 10 to 20 (finding 4). Distance still orders voyages. Survival and the fish square do not.

## Questions for Michael

1. Durability is listed as high and left alone, on the assumption the other stream owns the formula. Should findings 3 and 6 wait for that change, since both are partly "wear is doing the work the quality term got wrong"?
2. Is `Hide` sinew, leather, or both? Bowstrings and snares look right if it is sinew, and wrong if it is rawhide. Nets look wrong either way.
3. Wool and hide having the same warmth makes every cold band wear hide. Is the intended clothing story "hide is as warm as wool, and wool is for sails and cloth", or should wool replace hide once sheep are sheared? If wool is made warmer, may a fully equipped band go past protection 0.92? Half the bands on this save are already on that cap, so a fix that stays near 1 will not show up in the cold.
4. Obsidian out-cutting steel is defensible for a fresh flake. Should `cut_q` stay a fresh-edge number, with wear as the only penalty, or should the food term use the working edge the way combat already does?
5. A plank hull does not fish further than a dugout, because of `round`, and on this save 99% of boats, including novices, sit on the survival floor. The outrigger's +0.15 plus hull quality at 1 is what puts them there. Was the outrigger meant to be this decisive, and should a ship reach more water than a dugout?
6. Step 4's prior copies `material_term` from these formulas, including a point slot of exactly 1. If the durability fix lands first, the prior should be re-read so it does not freeze the capped axe scores into beliefs.
7. Should surplus in the store raise births, or is "fed is fed" intended? On this save the 0.75 cliff hits 14 bands. Everyone else is on the top step, and the store is not full.
8. The meeting cap is 4 and the median band has 26 neighbours within two cells. Partner visits are already uncapped. Is 4 a short season on purpose, or should denser country teach more? The wider draw was not resimulated; 16% are over the cap from their own cell alone.
9. `push_out` is 1 for 46% of bands, from crowding, with raw values up to 22. Should a more crowded camp be more eager to leave than one that is merely crowded?
10. Farm warmth is full at 12°C, and 99.8% of farmers are there. Should hotter farmland yield more, or do biome and rain carry that?
11. A steel axe against a copper one is 1.009× plants and 1.119× farm yield, and a steel spear is 1.068× the fighting strength. Felling trials are several-fold for a hard metal against stone, not a tenth, and only a small step from bronze to steel. Should the deep dive aim at that sustained-work gap, or is a small calorie step the intended abstraction? The 0.1 and 0.2 coefficients cannot express a several-fold gap even if `cut_q` is rewritten. Vickers (about 3–6× for a work-hardened copper edge against a historical steel edge, and about 15× only as an outer bound) should not be pasted onto food.
12. Iron against bronze is inside 1% on the axe, the farm and the fight. Leave them that close? Unsteeled iron and a good bronze really are close, and bronze against steel should stay a small step too: Mathieu and Meyer's bronze axes felled about as fast as steel. The step the trials support, and the sim misses, is the large one from stone or soft copper up to a good hard metal, not steel over bronze.
13. Depth 8 multiplies upkeep by 3.4. On the nominal mix that costs more than the whole steel-for-copper upgrade. Should a long chain stay that expensive once the food gap is widened, or was the depth price set against "a tool" rather than against "a better metal"?
14. A plank is 1.077× a dugout, and about 1% safer on a 10-cell hop, with the same fish radius. The outrigger's +0.15 is larger than that hull gap. Should a planked hull be a different class of boat in load and safety, or is 8% the intended step?
15. Wool and hide are the same warmth, so cold need does not move. If hide is a fur, is 1 the right size? If hide is leather, wool should be warmer, and by how much? Linen at 0.55 of the capped wool score looks plausible either way.
16. Iron reduces at 1400°C, 90°C above charcoal in a ceramic kiln, so bellows are required, and a stone lining misses copper by 40°C. Is that razor intended, or should a good furnace be able to touch iron without the blast?
