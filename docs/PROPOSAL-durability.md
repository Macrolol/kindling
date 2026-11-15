# Proposal: material-driven tool durability and wear

This is a proposal. Cargo feature `durability-v2` is off unless you ask for it. With it off, the simulation is unchanged: the golden hashes stay `0x1845e2690330d59` and `0x5cef0617c34457c6`. Nothing here retunes the live game. Michael decides whether the feature is ever turned on.

**What the feature does.** An edge is no longer one durability number. Keeping it sharp is resharpening labour. Snapping it is a separate breakage bill. The food of a knife or a hunt weapon is the average cut between hones, not the fresh cut. Other forms (impact, vessel, cordage, hull, thermal, record, structure, stock) each have their own life. Those lives also change behaviour under the feature. This is not an axe-only switch.

**What it does not do.** There is no second wear account. The combined account, the edge-life spread, and the overrides that existed to flip between them are gone. The sim reads the split constants. A test can still force today's formula (`with_v1_model`); that override is `#[cfg(test)]` only and is not in the sim binary. A point still fishes on presence, on both builds. A lesson still waits until the next refit to show up in the food multipliers, on both builds.

**Do not switch it on yet.** Two bugs are still in the split formulas. Dull time is counted twice (in the working cut and again in the hone labour). The axe score lets toughness stand in for cutting. Both are bugs, not knobs. The fixes and the two candidate repairs for the double count are under Known bugs. The census will move after those fixes. Land the code with the feature off. Fix the bugs. Re-measure. Only then decide to turn it on.

Malleability does not buy edge life. `edge_retention`, `split_working_cut` and `split_axe_score` never read it. Retention is hardness and acuity, squared, so a soft rank dulls sooner than a straight reading of it. Toughness, after the knap discount, is breakage. `resharpen_cost` and `repair_factor` (`1 - 0.045 * malleable`) are the only places malleability helps an edge, and they make the hone cheaper. They do not make the edge last longer. Gold is more malleable than copper and still retains less. The old `durability` clamp at 1.0 remains only on the feature-off path, and inside a test that forces the old formula. Class life has no cap.

## Separate proposals (not in this feature)

Two behaviour changes were drafted on this branch and then taken out. They are not wear.

**Point fish term.** Today a point multiplies fish by 1 whenever any point is in the kit. The material does not matter. The draft multiplied that term by a working-edge factor (steel about 0.765, copper about 0.324), so every band's fish fell, including bands already on steel. That is a global fish cut, not a wear fix, and the factor was picked so a flint point would beat copper. It belongs in the sizing deep dive, next to the audit's bow/arrow finding (the bow's fight term ignores which point is nocked). With the feature on, a point still fishes on presence, exactly as in the default build.

**Same-season refit after a lesson.** The kit is fitted before practice and before teaching. A lesson then moves `use_skill`, and the food multipliers stay at the kit-time user factor until the next refit. That one-season lag exists on both builds. The feature used to call `apply_base` and `refine_kit` again in the season of the lesson, only so `payout_invariant` would see the new user factor. That mixed a second effect into every feature-on comparison. The test now skips a band taught this season, and says why: the stored multipliers are the kit-time user, not `band_user` at the end of the step. Changing the lag on both builds would be its own change, and it would need its own golden-hash update.

## The bug, in plain words

A steel axe wears out faster than a copper axe.

```text
durability(m) = ((tough + 0.5 * malleable) / 12).clamp(0.05, 1.0)
```

That is `durability` in `materials.rs`. Copper, bronze and iron all clamp to 1, so they all wear at `1.15 - 1 = 0.150`. Steel (tough 9, malleable 5.5) comes out at 0.979 and wears at 0.171. The metal that should hold an edge is the one the sim throws away first. The cap is what hides the difference. Malleability is the wrong thing to reward for a working edge: a soft edge rolls, and it is easier to hammer sharp again. Hardness and acuity keep the edge. Toughness stops the chip.

Draft PR #11 asked why bands that can make steel still carry copper axes, and it answered with this formula. The answer cannot be trusted until the metals come apart. The old function stays. `tests.rs` still asserts `durability(steel) >= 0.95 * durability(iron)` on it. That assertion is the bug-shaped comparison and it still passes. It does not see this model.

The property table is not changed. Hardness, toughness, acuity (`sharp`), knapping (`fracture`, not brittleness) and malleability keep their existing 0..10 values. Steel's hardness is 6 against iron's 5.5. The comment on steel already put the real difference in `sharp` (9 against 7). Whether to raise hardness is a question, because `cut_q` is in the default build.

Rough order for a working edge, as a target rather than a fitted curve: obsidian and flint are short because they chip; copper is short because it dulls and is easy to hone; bronze is better; wrought iron is about bronze or a bit better; steel is clearly the best. A ground stone edge outlasts soft copper. A flake does not. The sim has one edge class for both.

## The model

Feature `durability-v2` on `civ-core`, default off. `native/Cargo.toml` forwards it (`durability-v2 = ["civ-core/durability-v2"]`). With the feature off, `score_durability`, `record_durability`, `food_edge_q`, `axe_food_score` and `upkeep_wear` are the old expressions. `durability_v2_orders_edges_and_v1_formula_stays` checks that for every material and every form.

`wear_class` maps the 32 forms:

| Class | Forms | What changes under the feature |
| --- | --- | --- |
| Edge | Edge, Point, Spear, Axe | Not a class life. Resharpen labour plus breakage. |
| Impact | Hammer, Shield | Own life. A flint face shatters; a soft face dishes. |
| Vessel | Vessel, Jar, Frame, Bellows | Pot cracks, or fibre frays. No rot column. |
| Cordage | Cord, Net, Trap, Garment, Bow, Sail, Cloth | Fibre toughness. No rot. |
| Hull | Boat, Outrigger, Plank, Ship | Breaks up. Float and seal stay in `boat_q`. |
| Thermal | Kiln, Mould, Crucible | Heat. A metal crucible is scored as a clay body. |
| Record | Mark | Weathers. Not an edge. Ceramic still gets +1 in the kit choice. |
| Structure | Shaft, Loom | Breaks. Does not dull. |
| Stock | Raw, Bloom, Cooked, Fire, Ornament | `tough / 12`. Fire, cooked and ornament return before the wear term. |

The non-edge classes are part of the feature. A cord, a hull, a pot, a mark and a shield do not keep today's `durability` when the feature is on. A feature-on comparison is not "the axe, and nothing else."

**Edge, the split.** `class_durability` of an edge is 0 and the sim does not read it.

```text
keep        = (0.65 * hard + 0.35 * sharp) / 10
retention   = keep * keep
interval    = max(0.02, retention)
hone cost   = (1 - fracture/10) * repair + (fracture/10) * 1
resharpen   = 0.07 / interval * hone cost
toughness   = chip_tough / 10          chip_tough = tough * (1 - 0.50 * fracture / 10)
break       = (1 - toughness) * 0.60
wear        = resharpen + break
working cut = cut_q * retention * (0.25 + 0.75 * toughness)
axe score   = 0.5 * working cut + 0.5 * toughness     known bug, see below
```

`RESHARPEN_USE` is 0.07 and `BREAK_SCALE` is 0.60, so the labour sits near today's wear (about 0.15 to 1), not on a scale of hours. The 0.25 chip floor keeps a knapped edge's food off zero; without it flint loses the knife to soft copper. A fracture of 10 makes the hone cost 1: a new edge, not a hammered one. `repair` is `1 - 0.045 * malleable`, except a thermal item whose material has no refractory (the crucible's charge). You are not re-forging the clay. Copper's repair is 0.595, bronze and iron 0.730, steel 0.752, gold 0.550. Malleability on these props is at most 10, so the factor stays above 0.5 with no clamp.

`cut_q` is unchanged: `(0.6 * sharp + 0.4 * hard) / 10`. Copper 0.480, bronze 0.650, iron 0.640, steel 0.780, flint 0.760, obsidian 0.820, stone 0.420.

**Non-edge lives.** No shared cap. Two materials share a life only when the properties that class reads are equal (the unit test checks this). The ties the diagnostic prints are that kind of tie.

- Impact: toughness, cut down by fracture, times a face that stays proud when it is hard and dishes when it is malleable.
- Vessel: a fibre blend when `fibrous` is high, otherwise a pot that cracks and abrades.
- Cordage: `(0.55 * tough + 0.45 * fibrous) / 12`. Nothing is immortal. The four tool metals share this life (toughness 9, no fibre) and lose to hide and flax.
- Hull: toughness, helped by hardness, so a dugout and a skin boat of the same toughness do not tie.
- Thermal: refractory and toughness when the material has a refractory rating. If it melts and refractory is 0, the body is fired clay (refractory 9, tough 2) stressed by `melt / 2000`.
- Record: `(0.55 * hard + 0.45 * tough) / 10`.
- Structure: toughness, a little hardness, hurt by flaking.
- Stock: `tough / 12`.

**Wear charged as labour.** Feature off, `1.15 - durability` for every form. Feature on, an edge pays `split_edge_wear`. Anything else pays `(1.15 - class life) * repair`. The 1.15 is the old zero point, so a life near 1 still costs a little. No current material reaches 1.15. There is no floor. A later material could make non-edge upkeep negative. That is a question.

**Food.** Feature off, `food_edge_q` is `cut_q` and `axe_food_score` is `0.5 * cut_q + 0.5 * durability`, the same operations as before. Feature on, `food_edge_q` is `split_working_cut` and `axe_food_score` is `split_axe_score`. The knife (`cut_term`) and the hunt weapon (`hunt_q_of`) use `food_edge_q`. Plants add `0.1 * axe` and the farm adds `0.2 * axe`, through `axe_food_score`. The point's fish term is presence: 0 if the slot is empty, otherwise the user factor. The material does not enter it.

**Fight.** `edge_q` is `0.55 * cut_q + 0.45 * score_durability`. Feature off, that durability is the old function. Feature on, it is `edge_toughness`. Food does not call `edge_q`. The axe's fight term is `0.3 * axe_food_score`. The bow is still a flat 0.3 whenever any point exists. That is the audit's finding, and it is not changed here.

**Priced order at chain depth 0**, nominal band, feature on. Locked by `working_edge_orders_knife_axe_and_point` and `split_model_orders_retention_break_and_price`.

- Knife: steel, iron, bronze, flint, copper, obsidian. Flint beats stone. Copper beats obsidian. Food alone still has obsidian above copper (0.126 against 0.073); wear is what hands the priced knife to copper.
- Point: steel, iron, bronze, copper, flint, obsidian. Fish is the same for every point, so this is wear. Copper beats flint. That is the opposite of the removed fish term, which had been tuned so flint would win.
- Axe: steel, iron, bronze, copper, stone, flint, obsidian.
- Iron spear over obsidian. Steel spear over iron.

`split_model_orders_retention_break_and_price` also locks the mechanism: copper is honed clearly more often than iron, copper's working cut is well below its fresh cut and below iron's, a knapped edge's cost is mostly replacement, copper's cost is mostly the hone, and malleability is absent from the interval.

## Where the sim reads it

`save.rs` has no durability field. The sim goes through the helpers.

| Site | Feature off | Feature on |
| --- | --- | --- |
| `durability` | The capped scalar. Still there. | Not called by the sim. |
| `upkeep_wear` / `tool_upkeep` | `1.15 - durability` | Edge: resharpen + break. Else: `(1.15 - class life) * repair`. |
| `food_edge_q`, `cut_term`, `hunt_q_of` | `cut_q` | `split_working_cut` |
| Point fish (`yields_base`) | Presence | Presence. Same bits. |
| `axe_food_score`, plants `0.1 *`, farm `0.2 *`, axe fight | `0.5 * cut_q + 0.5 * durability` | `split_axe_score` (the known bug) |
| `edge_q`, `weapon_q`, hunt fight | Old durability in the blend | `edge_toughness` in the blend |
| `nonfood_scores` | Old wear | Hull, cordage or impact life, times repair |
| Mark | Old durability, +1 if ceramic | Record life, +1 if ceramic |
| Probe upkeep | `KIT_UPKEEP * upkeep_wear * depth` | Same door. `KIT_UPKEEP` stays 0.06. |

## Every material, before and after

Printed by `diag_durability_table`. The columns are pure functions of the props, so both binaries print them. `old` is today's `durability`. `oldWear` is `1.15 - old`. `work` is `split_working_cut`. `wear` is `split_edge_wear`. `axeOld` is half fresh cut, half old durability. `axeSplit` is `split_axe_score`. The class columns are lives, not wear. An edge of a non-tool material is the formula, not a thing the sim makes. Non-edge forms do not pay the `wear` column. They pay `(1.15 - class life) * repair`.

| Material | old | oldWear | work | interval | resh | brk | wear | axeOld | axeSplit | impact | vessel | cord | hull | thermal | record | struct | stock |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| wood | 0.500 | 0.650 | 0.006 | 0.040 | 1.750 | 0.240 | 1.990 | 0.350 | 0.303 | 0.534 | 0.365 | 0.425 | 0.438 | 0.175 | 0.380 | 0.480 | 0.500 |
| stone | 0.583 | 0.567 | 0.070 | 0.245 | 0.286 | 0.254 | 0.539 | 0.502 | 0.324 | 0.518 | 0.473 | 0.321 | 0.595 | 0.421 | 0.645 | 0.542 | 0.583 |
| flint | 0.292 | 0.858 | 0.162 | 0.540 | 0.130 | 0.485 | 0.614 | 0.526 | 0.177 | 0.140 | 0.151 | 0.160 | 0.303 | 0.265 | 0.543 | 0.204 | 0.292 |
| obsidian | 0.125 | 1.025 | 0.126 | 0.501 | 0.140 | 0.555 | 0.695 | 0.472 | 0.100 | 0.050 | 0.056 | 0.069 | 0.126 | 0.044 | 0.370 | 0.080 | 0.125 |
| clay | 0.083 | 1.067 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.062 | 0.050 | 0.082 | 0.057 | 0.046 | 0.066 | 0.462 | 0.100 | 0.073 | 0.083 |
| copper ore | 0.250 | 0.900 | 0.004 | 0.061 | 1.147 | 0.429 | 1.576 | 0.201 | 0.145 | 0.263 | 0.220 | 0.138 | 0.240 | 0.196 | 0.344 | 0.248 | 0.250 |
| tin ore | 0.333 | 0.817 | 0.025 | 0.179 | 0.392 | 0.372 | 0.764 | 0.297 | 0.202 | 0.359 | 0.320 | 0.183 | 0.343 | 0.225 | 0.538 | 0.348 | 0.333 |
| plant fibre | 0.250 | 0.900 | 0.000 | 0.020 | 3.500 | 0.420 | 3.920 | 0.135 | 0.150 | 0.218 | 0.225 | 0.475 | 0.184 | 0.087 | 0.162 | 0.204 | 0.250 |
| hide | 0.500 | 0.650 | 0.000 | 0.020 | 3.500 | 0.240 | 3.740 | 0.270 | 0.300 | 0.490 | 0.358 | 0.500 | 0.398 | 0.175 | 0.325 | 0.440 | 0.500 |
| bone | 0.417 | 0.733 | 0.043 | 0.174 | 0.402 | 0.338 | 0.739 | 0.423 | 0.240 | 0.393 | 0.338 | 0.229 | 0.404 | 0.146 | 0.445 | 0.390 | 0.417 |
| meat | 0.083 | 1.067 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.046 | 0.050 | 0.061 | 0.041 | 0.046 | 0.058 | 0.029 | 0.056 | 0.064 | 0.083 |
| wild plants | 0.083 | 1.067 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.052 | 0.050 | 0.073 | 0.046 | 0.121 | 0.061 | 0.029 | 0.073 | 0.068 | 0.083 |
| fish | 0.083 | 1.067 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.046 | 0.050 | 0.061 | 0.041 | 0.046 | 0.058 | 0.029 | 0.056 | 0.064 | 0.083 |
| charcoal | 0.083 | 1.067 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.062 | 0.050 | 0.082 | 0.057 | 0.046 | 0.066 | 0.029 | 0.100 | 0.073 | 0.083 |
| ceramic | 0.167 | 0.983 | 0.017 | 0.154 | 0.454 | 0.480 | 0.934 | 0.223 | 0.109 | 0.191 | 0.165 | 0.092 | 0.168 | 0.546 | 0.393 | 0.179 | 0.167 |
| copper | 1.000 | 0.150 | 0.073 | 0.164 | 0.254 | 0.060 | 0.314 | 0.740 | 0.486 | 0.645 | 0.666 | 0.413 | 0.697 | 0.442 | 0.570 | 0.756 | 0.750 |
| tin | 0.750 | 0.400 | 0.002 | 0.023 | 1.991 | 0.300 | 2.291 | 0.450 | 0.251 | 0.319 | 0.314 | 0.229 | 0.350 | 0.524 | 0.308 | 0.386 | 0.417 |
| bronze | 1.000 | 0.150 | 0.208 | 0.345 | 0.148 | 0.060 | 0.208 | 0.825 | 0.554 | 0.744 | 0.733 | 0.413 | 0.748 | 0.455 | 0.680 | 0.797 | 0.750 |
| grain | 0.083 | 1.067 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.062 | 0.050 | 0.082 | 0.057 | 0.046 | 0.066 | 0.029 | 0.100 | 0.073 | 0.083 |
| flax | 0.375 | 0.775 | 0.000 | 0.020 | 3.500 | 0.330 | 3.830 | 0.198 | 0.225 | 0.326 | 0.355 | 0.562 | 0.276 | 0.131 | 0.230 | 0.306 | 0.375 |
| wool | 0.333 | 0.817 | 0.000 | 0.020 | 3.500 | 0.360 | 3.860 | 0.173 | 0.200 | 0.263 | 0.267 | 0.483 | 0.236 | 0.117 | 0.196 | 0.261 | 0.333 |
| iron ore | 0.333 | 0.817 | 0.011 | 0.106 | 0.663 | 0.372 | 1.035 | 0.267 | 0.196 | 0.355 | 0.308 | 0.183 | 0.333 | 0.225 | 0.455 | 0.340 | 0.333 |
| iron | 1.000 | 0.150 | 0.215 | 0.363 | 0.141 | 0.060 | 0.201 | 0.820 | 0.557 | 0.754 | 0.744 | 0.413 | 0.757 | 0.399 | 0.707 | 0.804 | 0.750 |
| brine | 0.050 | 1.100 | 0.000 | 0.020 | 3.500 | 0.600 | 4.100 | 0.025 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| salt | 0.083 | 1.067 | 0.001 | 0.026 | 2.651 | 0.540 | 3.191 | 0.092 | 0.050 | 0.091 | 0.071 | 0.046 | 0.075 | 0.029 | 0.183 | 0.082 | 0.083 |
| pitch | 0.167 | 0.983 | 0.000 | 0.020 | 3.500 | 0.480 | 3.980 | 0.093 | 0.100 | 0.145 | 0.096 | 0.092 | 0.123 | 0.058 | 0.117 | 0.136 | 0.167 |
| steel | 0.979 | 0.171 | 0.359 | 0.497 | 0.106 | 0.060 | 0.166 | 0.880 | 0.629 | 0.769 | 0.754 | 0.413 | 0.765 | 0.407 | 0.735 | 0.810 | 0.750 |
| gold | 0.833 | 0.317 | 0.004 | 0.039 | 0.987 | 0.300 | 1.287 | 0.497 | 0.252 | 0.340 | 0.356 | 0.229 | 0.377 | 0.444 | 0.363 | 0.411 | 0.417 |
| resinous wood | 0.333 | 0.817 | 0.001 | 0.020 | 3.500 | 0.360 | 3.860 | 0.207 | 0.200 | 0.356 | 0.270 | 0.183 | 0.292 | 0.117 | 0.290 | 0.320 | 0.333 |

Brine's old 0.050 is the floor. Its working cut is 0 because every input is 0. The interval floor (0.02) only keeps the hone division finite. Copper, bronze and iron sharing `old` 1.000 is the cap. They do not share a working cut. They do share cordage life (0.413) and stock life (0.750), and they share break labour 0.060, because toughness is 9 and fracture is 0. A metal cord is a poor cord next to hide (0.500) or flax (0.562).

Tool edges, from that table. Working cut, highest first: steel 0.359, iron 0.215, bronze 0.208, flint 0.162, obsidian 0.126, copper 0.073, stone 0.070, bone 0.043. Axe score, highest first: steel 0.629, iron 0.557, bronze 0.554, copper 0.486, stone 0.324, bone 0.240, flint 0.177, obsidian 0.100. Iron is just above bronze on both. Stone outlasts copper between hones (interval 0.245 against 0.164) and then loses the axe, because it breaks (0.254 against 0.060) and its fresh cut is poor. Flint and obsidian stay keen on paper and lose the axe because the edge is gone: flint's wear is 0.485 break of 0.614 total, obsidian's 0.555 of 0.695. Copper's wear is mostly the hone (0.254 of 0.314). A hide axe scores 0.300 and a wooden one 0.303, both above flint. That is the axe-score bug, not a result to keep.

Copper against iron. Fresh cut 0.480 against 0.640, so a newly sharp copper axe is still about three quarters of an iron one. Average working cut 0.073 against 0.215: about a third of iron's cut, and 15% of its own fresh cut. Resharpen labour 0.254 against 0.141 (1.8 times iron, 2.4 times steel's 0.106). Break labour 0.060 for both. Copper's bill is the hone. Obsidian's is the opposite.

Wear on the shapes that are not edges, for materials that are actually made into them. Old wear is one number. New wear is `(1.15 - class life) * repair`.

| Material | oldWear | impact | cord | hull | vessel | thermal |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| wood | 0.650 | 0.616 | 0.725 | 0.712 | 0.785 | 0.975 |
| hide | 0.650 | 0.660 | 0.650 | 0.752 | 0.792 | 0.975 |
| plant fibre | 0.900 | 0.932 | 0.675 | 0.966 | 0.925 | 1.062 |
| flax | 0.775 | 0.824 | 0.587 | 0.874 | 0.795 | 1.019 |
| wool | 0.817 | 0.887 | 0.667 | 0.914 | 0.883 | 1.033 |
| ceramic | 0.983 | 0.959 | 1.058 | 0.982 | 0.985 | 0.604 |
| clay | 1.067 | 1.068 | 1.104 | 1.084 | 1.093 | 0.688 |
| stone | 0.567 | 0.632 | 0.829 | 0.555 | 0.677 | 0.729 |
| copper | 0.150 | 0.301 | 0.439 | 0.269 | 0.288 | 0.708 |

A ceramic kiln wears less than it does today (0.604 against 0.983). A copper hull or a copper cord wears more than today's 0.150, because today's 0.150 was the cap. The mark: wood's record life is 0.380 against today's 0.500, bone 0.445 against 0.417, ceramic 0.393 against 0.167, and the ceramic +1 still puts a fired token first. Bone now beats a wooden tally.

## Ratios, and the real numbers

Before is today's formula. After is the split. A ratio is the first material divided by the second. For wear, hone and break, below 1 means the first costs less to keep. For life, interval, cut and the axe, above 1 means the first is better. Plants and farm ratios equal the axe ratio. The absolute gap, which is what has to beat chain labour, is under the ratio tables.

Fresh cut (before = after):

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| fresh | 1.625 | 1.219 | 0.985 | 1.354 | 1.143 | 1.333 | 0.927 | 1.857 | 1.548 |

Edge life before, resharpen interval after. Before, steel is worse than copper (0.979), because of the cap.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before, life | 0.979 | 0.979 | 1.000 | 1.000 | 1.714 | 1.000 | 2.333 | 1.679 | 1.714 |
| split, interval | 3.030 | 1.369 | 1.052 | 2.104 | 0.669 | 2.213 | 1.079 | 2.028 | 1.409 |

Hone and break. Before has only total wear. Split break is 1.000 across the four tool metals: toughness is 9 for all of them.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before, wear | 1.139 | 1.139 | 1.000 | 1.000 | 0.265 | 1.000 | 0.837 | 0.301 | 0.265 |
| split, break | 1.000 | 1.000 | 1.000 | 1.000 | 0.237 | 1.000 | 0.873 | 0.237 | 0.237 |
| split, hone | 0.417 | 0.753 | 0.951 | 0.583 | 0.889 | 0.554 | 0.927 | 0.371 | 0.518 |
| split, upkeep | 0.529 | 0.827 | 0.965 | 0.663 | 0.582 | 0.640 | 0.884 | 0.308 | 0.386 |

Working cut. The knife's food term is this row. Before, the knife is fresh cut.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 1.625 | 1.219 | 0.985 | 1.354 | 1.143 | 1.333 | 0.927 | 1.857 | 1.548 |
| split | 4.924 | 1.669 | 1.036 | 2.850 | 1.036 | 2.951 | 1.288 | 5.101 | 2.952 |

Axe score. Split steel:copper is 1.29. The working-cut ratio above is 4.92. The blend throws most of that away. That is the second known bug.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 1.189 | 1.073 | 0.994 | 1.115 | 1.475 | 1.108 | 1.113 | 1.753 | 1.645 |
| split | 1.294 | 1.129 | 1.007 | 1.138 | 1.502 | 1.146 | 1.766 | 1.943 | 1.710 |

Absolute food. Delta is steel minus copper.

| | steel | iron | bronze | copper | stone | flint | obsidian | steel−copper |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| split axe | 0.629 | 0.557 | 0.554 | 0.486 | 0.324 | 0.177 | 0.100 | 0.143 |
| split plants, 0.1× | 0.063 | 0.056 | 0.055 | 0.049 | 0.032 | 0.018 | 0.010 | 0.014 |
| split farm, 0.2× | 0.126 | 0.111 | 0.111 | 0.097 | 0.065 | 0.035 | 0.020 | 0.029 |
| before, axe | 0.880 | 0.820 | 0.825 | 0.740 | 0.502 | 0.526 | 0.472 | 0.140 |
| before, plants | 0.088 | 0.082 | 0.083 | 0.074 | 0.050 | 0.053 | 0.047 | 0.014 |
| before, farm | 0.176 | 0.164 | 0.165 | 0.148 | 0.100 | 0.105 | 0.094 | 0.028 |

Today's plants gap between steel and copper is already 0.014. Split does not widen it. Both axe scores fell, the ratio went from 1.19 to 1.29, and the absolute gap stayed 0.014. A working-cut gap of 0.286 arrives at the farm line as 0.029. The 0.1 and 0.2 do that shrinking, and they are outside this feature. The 0.5/0.5 blend does the rest, and it is the bug.

Felling time, steel against stone. Saraydar and Shimada, American Antiquity 36 (1971), pp. 216–217: one steel axe against one ground stone axe, same chopper, about 6.4 times faster and about 5.1 times less energy per inch of wood. The same pair, clearing plots, came out about 3.6 times faster and 3.3 times less energy (American Antiquity 38 (1973), pp. 344–350). Mathieu and Meyer, Journal of Field Archaeology 24 (1997), pp. 333–351, put the chopping studies they reviewed at roughly two to four and a half times. Carneiro's plot-clearing figures run much higher and mix axes with adzes. I use 3 to 6 times, and I do not use the clearing extremes. Confidence: medium. Small trials, modern steel, ground stone rather than a flake.

The sim, steel against stone. Fresh cut 1.86. Split working cut 5.10, inside that band. Split axe score 1.94, the bottom of the band. The trials were maintained edges, closer to fresh cut than to a season of not honing. Matching them with the dull-time term is partly luck, and that term is one of the two bugs. I am not retuning to chase 6.4.

Bronze against steel. Mathieu and Meyer's own trial found bronze not significantly slower than steel. Confidence: medium. Split working cut steel:bronze is 1.73 (0.359 against 0.208). Split axe score is 1.14. The working cut exaggerates steel against bronze relative to that trial. These props cannot say a quench-hardened edge against work-hardened bronze, because steel's hardness is 6 and iron's is 5.5.

Copper against bronze, edge holding. Kienlin and Ottaway, in *L'Atelier du bronzier* vol. 2 (1998), pp. 271–286, found tin bronze better at keeping an edge than copper. Direction only; I do not have their wear rates. Confidence: medium on the direction, none on a number. Soriano and Gutiérrez Sáez, 2009, table 2: pure copper about 79–104 HV, about 5% tin about 97–134 HV, about 15% tin about 125–189 HV. One set of replicas. Confidence: medium for those replicas. The 15% tin bronze is roughly one and a half to two times the copper. The split interval bronze:copper is 2.10, and copper takes about 1.7 times the hone. Same neighbourhood, for bronze against copper only. Iron:bronze interval is 1.05. Unhardened iron against work-hardened bronze can sit that close. Steeled iron would not.

Copper and steel hardness, the gap the props cannot say. Modern C11000 copper is often quoted around 50 HV annealed and around 100 HV hard. I am treating that as a commercial range, not an archaeological measurement. Confidence: low to medium. A quench-hardened carbon-steel edge is several hundred HV. Confidence that the gap is several times: medium. Confidence in any single ratio: low. The sim's ranks are copper 3, bronze 5, iron 5.5, steel 6. The split interval is then steel:copper 3.03 and steel:iron only 1.37. The 1.37 is the props.

## Known bugs

These two are bugs in the split formulas. They are not tuning knobs and they are not questions. Both must be fixed before `durability-v2` is switched on. The census below was measured with the bugs still in the code. It will move after the fixes. Do not turn the feature on and then treat that census as the result.

**Dull time is counted twice.** `split_working_cut` multiplies the fresh cut by `edge_retention`, and retention is already the keep index squared. `resharpen_labour` divides by the same retention. A band that pays to hone more often should spend less of the season dull. Charging both at full strength does the opposite: copper pays about 6.1 times the honing of a perfect edge and keeps about 15% of its fresh cut (0.073 against 0.480). Steel against copper then compounds to about 3 squared, roughly nine times, across labour and food together. Hardness is also already inside `cut_q` (`0.4 * hard`), so it is in the product a third time.

Two candidate fixes. Pick one. Then re-run the census. Copper should be able to survive in more places than it does in the census below. That is acceptable if it is what the corrected physics says.

- Hone at a fixed loss of keenness. Average cut is the same fraction of fresh for every material. Only the labour scales with `1 / retention`.
- Charge the optimal interval. Honing and dullness each scale with the square root of `1 / retention`, not with the whole factor.

**The axe score lets toughness stand in for cutting.** `split_axe_score` is half the working cut and half toughness. On that blend a hide axe scores 0.300 and a wooden one 0.303, against stone 0.324 and copper 0.486, and a flint axe (0.177) scores below the wooden one. Toughness is already inside the working cut (the body term) and inside `break_labour`, so it is counted three times. The four tool metals are all toughness 9, so that half of the score is a flat 0.45 for each of them. Steel against copper on the axe score shrinks to 1.29 even though the working cut differs by about 4.9 times.

The fix: the axe's food is the working cut alone, and breakage stays on the labour side. The plants coefficient (0.1) and the farm coefficient (0.2) are not this bug. They stay until a later sizing pass.

After both fixes, re-measure `diag_durability_flips` and `diag_axe_breakeven` on `DIAG_SAVE` and replace the census.

## What the save does

`diag_durability_flips` and `diag_axe_breakeven`, feature on, after the point fish term was removed. The snapshot is the one under Saves: format 16, year 350, seed 6, map 256x160, made from main `f8d80dd`. The numbers in this section are filled from that re-run. They are the broken formulas. They are not a target for the bug fixes.

A band is a farm when the grain share of its harvest is at least 0.10, or `sow >= 0.2`. The same split as the axe diagnostic.

The feature-on scorer is checked against `slot_score_m`, and the kit after load is checked against the live split pick. Load refits. The v1 column is a rescore with `with_v1_model`, not a second load. The feature-off binary prints the v1 census only.

Point rows are not the earlier pass. That pass multiplied fish by a working-edge factor. This one does not. Axe rows do not read the fish term; they are re-measured anyway.

<!-- CENSUS-START -->
Census not yet replaced by this pass's re-run.
<!-- CENSUS-END -->

Step 4, once main is merged into it, prices a food slot from a belief when the table is non-empty. The prior is `tool_upkeep`, which is `upkeep_wear` times the seal and the chain depth. The yield the belief learns is `split_working_cut` for a knife and a hunt weapon, and the plants and farm lines on `split_axe_score` for an axe. The point's fish factor is presence, so a point belief does not learn a material fish factor. A stored labour belief freezes the upkeep and does not freeze the yield. The flips in this note are the empty-table price. Re-measure them on the merged tree. They will not match.

## Saves

Durability is not saved. It is derived from the material. No format version change. A save written under one build loads under the other. `after_load` rebuilds toolkits and `fit_kit` runs when the cache misses or the kit is dirty, so the kit may change on load when the feature differs from the build that wrote the file. The bytes do not.

The diagnostics read `DIAG_SAVE` only. If it is unset or empty they print a skip line and return. There is no default path. The snapshot measured here was made from main `f8d80dd`, seed 6, 350 years, map 256x160. Regenerate it from that commit with those settings and point `DIAG_SAVE` at the new file.

## How this lands

Decided. This branch lands on main before PR #12 (`skills-step4`). #12 then merges main into `skills-step4`. A merge, not a rebase, and no force-push. The feature stays off in that merge. The default build does not move the pre-step-4 hashes, so it does not block #12's own hash update. Do not turn the feature on until the two known bugs are fixed and the flip table has been re-run, including on a build that has both this and step 4.

`materials.rs` and the diagnostics are only on this branch. `beliefs.rs` is only on step 4. `effects.rs`, `tests.rs`, `tests_skills.rs`, `world.rs`, `band.rs`, `lib.rs` and `probe_diffusion.rs` are touched by both. Resolve them so the helpers survive:

- `effects.rs`. Keep step 4's beliefs paragraph and `use crate::beliefs`, and keep this branch's `durability-v2` sentences. `tool_upkeep`, `cut_term`, `hunt_q_of`, `axe_term` and the mark stay on `upkeep_wear`, `food_edge_q`, `axe_food_score` and `record_durability`. Do not paste `1.15 - durability` back.
- `tests.rs`. Keep step 4's new hashes (the run-on becomes `0x4c41b83c2c81e1f5` because beliefs change the run). Keep the `#[cfg(not(feature = "durability-v2"))]` gate and the `allow(dead_code)` around the pre-step-4 pair. A feature-on mismatch with either pair is expected. Do not edit the constants.
- `tests_skills.rs`. The feature-on side of `wear()` is `upkeep_wear`. Do not restore `1.15 - durability` on that side.
- `probe_diffusion.rs`. Keep `upkeep_wear`. Step 4's belief price calls `tool_upkeep`. Both doors stay on the helpers.
- `band.rs`. Keep `axe_food_score` and `score_durability` in `weapon_q` and `edge_q`.
- `lib.rs`. Keep step 4's `beliefs` module and this branch's diag modules.
- `world.rs`. Step 4 owns the teaching and belief calls. This branch does not refit in the season of a lesson. Do not put that refit back while resolving.

After the merge, search `civ-core/src` for `1.15 - durability`. The only hits should be the feature-off body of `upkeep_wear`, the diagnostic that compares against the old expression, the feature-off branch of the skills-test `wear()`, and comments.

**CI.** `workflow_dispatch` on `.github/workflows/headless-sims.yml` and `.github/workflows/bench.yml` takes an input `features` (string, default empty, described as cargo features, for example `civ-core/durability-v2`). Empty produces today's cargo command. Non-empty is passed as `--features` to the sim build, the bench build of this branch (not the baseline ref: an older tag does not define the feature), and the probe test. The input is an environment variable, not interpolated into the script. The headless test job also runs `cargo test --release --locked --workspace --features civ-core/durability-v2` after the plain workspace test. Do not start a workflow from this branch to try it. Build Bot dispatches batch runs.

What to re-measure on the merged tree, feature off and feature on, after the two bug fixes: `diag_durability_flips` and `diag_axe_breakeven` on `DIAG_SAVE`; the step-4 "believed wrong" share; the two ordering tests; the default-build hashes, which should be step 4's pair, not the pair this proposal froze.

## Questions

The landing order is decided. The two formula bugs are not questions. These are still open.

1. Turn the feature on at all, once the two bugs are fixed and the census has been re-run? My recommendation is to land the code with the feature off, and to use the split model when it is turned on. The default build stays byte-identical until that decision.
2. Are the constants acceptable? Retention is `(0.65 * hard + 0.35 * sharp) / 10`, squared. Hone scale `RESHARPEN_USE` 0.07. Break scale 0.60. Chip floor on the working cut 0.25. Interval floor 0.02. Repair `0.045 * malleable`. They were chosen so the orders hold and the wear sits near today's scale. They are not fitted to the felling trials. The double count and the axe blend are not in this list.
3. Steel's hardness is 6 against iron's 5.5. Leave `PROPS` alone, as this proposal does? Raising it would also change `cut_q` in the default build.
4. One edge class covers a ground axe and a flake. Under split, stone's interval beats copper, which is right for a ground celt and generous for a flake. Split them later, or leave it?
5. There is no rot. Flax nets beat hide. Add a property later, or accept it?
6. Wear has no floor. If a later material's class life exceeds 1.15, non-edge upkeep goes negative. Edge wear is a sum of two positive terms, so this only bites the other classes. Floor it at 0?
7. `kindling` forwards the feature (`durability-v2 = ["civ-core/durability-v2"]`). The native build accepts it. The sim workflows now take a `features` input so CI can build and test it without editing the YAML. Say if you would rather the flag live only on `civ-core`.
8. The bow's fight term is still a flat 0.3 whenever any point exists. The audit suggests `0.3 * edge_q` of the point. Left for the audit, not this feature. The point's fish term is the same kind of question and is listed under Separate proposals, not here.

## How to run the diagnostics

`DIAG_SAVE` must be set to a `.civsave`. Unset, the test prints a skip line and returns. The measured snapshot was main `f8d80dd`, seed 6, 350 years, 256x160.

Feature off (today's sim; the table still prints the split functions):

```text
cargo test --release -p civ-core diag_durability -- --ignored --nocapture
cargo test --release -p civ-core diag_axe_breakeven -- --ignored --nocapture
```

Feature on (the sim uses the split model):

```text
cargo test --release -p civ-core --features durability-v2 diag_durability -- --ignored --nocapture
cargo test --release -p civ-core --features durability-v2 diag_axe_breakeven -- --ignored --nocapture
```

`durability_v2_orders_edges_and_v1_formula_stays` runs in the normal suite on both builds. `working_edge_orders_knife_axe_and_point` and `split_model_orders_retention_break_and_price` run only with the feature. To build the same way in CI, dispatch `headless-sims` or `bench` with input `features` set to `civ-core/durability-v2`. An empty input is today's command. This note does not start that dispatch.
