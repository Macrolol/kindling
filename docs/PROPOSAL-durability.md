# Proposal: material-driven tool durability and wear

This is a proposal. The default build does not use it. Cargo feature `durability-v2` is off unless you ask for it, and with it off the simulation is unchanged (the golden hashes stay `0x1845e2690330d59` and `0x5cef0617c34457c6`). Nothing here is a retune of the live game. Michael decides whether the feature is ever turned on.

## The bug, in plain words

A steel axe wears out faster than a copper axe.

The sim scores how long a material lasts with one number, in `civ-core/src/materials.rs`:

```text
durability(m) = ((tough + 0.5 * malleable) / 12).clamp(0.05, 1.0)
```

That is `durability` at `materials.rs:543`. Toughness and malleability are added together, divided by 12, and then crushed into the range 0.05 to 1. Copper (tough 9, malleable 9) comes out at 1.125 and is clamped to 1. Bronze and iron land on exactly 1. Steel (tough 9, malleable 5.5) comes out at 0.979, just under the cap. Wear in the upkeep formula is `1.15 - durability`, so copper, bronze and iron all wear at 0.150 and steel wears at 0.171. The metal that should hold an edge is the one the sim throws away first.

Malleability is the wrong thing to reward for a working edge. A soft metal bends, rolls and goes dull. What keeps an edge working is hardness and how keen an edge it can take. What stops it chipping or snapping is toughness. Malleability matters afterwards: a soft edge is easier to hammer back into shape, and a malleable piece is less likely to shatter. It should make repair cheaper, not make the edge last longer.

The cap is what hides the difference. Copper, bronze and iron are not equally durable. The formula cannot say so, because all three sit on 1.0. Only brine sits on the 0.05 floor (every property it has is zero). Gold (0.833) and tin (0.750) sit under the cap and look "durable" because they are soft.

Draft PR #11 (`docs/ANALYSIS-steel-axe.md`) asked why bands that can make steel still carry copper axes. It answered that question with this formula. The answer cannot be trusted until durability can tell the metals apart. The measurement below says the cap was not hiding a 2% gap. It was hiding a large copper penalty. Turning this proposal on does not nudge a few farmers onto steel. On the year-350 seed-6 save, every band that can make a steel axe holds one.

## What the words mean

These are the ordinary meanings, and where the sim's numbers are a judgment rather than a measurement. The property table (`PROPS` in `materials.rs`, from line 175) is not changed. Values are the existing 0 to 10 scales. The comment on `Props` says they are loosely based on real materials.

**Hardness** is resistance to being dented, scratched or rubbed away. A hard edge stays keen because the metal at the edge does not flow aside when it hits something. A hard hammer face does not dish. In the table, stone is 6, flint 7, copper 3, bronze 5, wrought iron 5.5, steel 6, ceramic 5.5, obsidian 5.5, gold 2.5, tin 1.5.

**Toughness** is resistance to cracking, chipping and breaking in half. A tough axe survives a knot. A brittle one loses a flake and then the edge is gone. Obsidian is 1.5, flint 3.5, ceramic 2, clay 1, bone 5, copper, bronze, iron and steel are all 9, wood and hide are 6.

**Edge acuity** (`sharp` in the table) is how keen an edge of that material can be made, not how long it stays that way. Obsidian is 10, steel 9, flint 8, bronze 7.5, iron 7, copper 6, stone 3. The comment on steel (line 202) already says the keenness lives here: quench-hardened steel "takes and keeps a keener edge than iron", and the number they stored for that is `sharp` 9 against iron's 7, not a big hardness gap. Steel's hardness is only 6 against iron's 5.5. That understates how much better a hardened steel edge resists abrasion. This proposal does not edit `PROPS`. It does use `sharp` in the edge formula, so some of that keenness counts. Whether hardness itself should be raised is a question for Michael, not a silent change.

**Brittleness** is the habit of failing by cracking. The sim does not have a brittleness column. `fracture` (line 110) means something else: how well the material flakes into a sharp edge when you knap it. Flint is 9 and obsidian is 10 because they knap. Ceramic is 0 because it does not. Using `fracture` as "this pot is brittle" would be wrong; the pot's brittleness is its low toughness. This proposal uses `fracture` only as a chip discount on edges, striking faces and shafts, which matches knapping, and uses low toughness everywhere else a thing simply breaks.

**Malleability** is how willingly the material bends or hammers without cracking. Its real jobs are: you can re-forge it, you can straighten a rolled edge, and it does not shatter the way flint does. It is not what holds an edge. Copper is 9, tin 8, bronze and iron 6, steel 5.5, gold 10. Those numbers are used, below, only on the repair side (and as a penalty when a soft face mushrooms or a soft edge rolls).

Tools do not wear the same way.

- **Edge tools** (knife, blade, axe, adze, sickle, spear, point) wear by going dull and by chipping. Hardness and acuity keep the edge. Toughness stops the chip. Too much malleability rolls the edge over. A very sharp brittle edge (obsidian, flint) starts keen and loses the edge in chips.
- **Blunt and impact tools** (hammer, maul, and a shield face that is struck) care about toughness first. Hardness keeps the face from dishing. Brittleness is fatal: a flint hammer face shatters. Malleability mushrooms a soft face.
- **Containers** (pot, basket, bag, jar, frame, bellows) either crack, if they are a pot, or fray and rot, if they are fibre. There is no rot column. Fibre life is toughness and fibrousness only. That is a gap, called out below.
- **Cordage, nets, garments, a bowstring, a sail, cloth** are fibre. They fray. They also rot, and the model cannot see rot.
- **Boats** (dugout, skin boat, outrigger, plank, ship) break up. Float and being watertight are already `boat_q`. They are not life.
- **Moulds, kilns, crucibles** fail from heat. A kiln lining uses its own refractory rating. A crucible in this sim is often stored as the metal charge, whose refractory is 0; the body that cracks is fired clay. The model treats it that way.

Rough order for how long a working edge should last, as a target, not as a fitted curve. Judgment, from the usual workshop picture rather than from a measured test in this repo: obsidian and flint are short, because they chip. Copper is short-ish: it dulls, and it is easy to hammer sharp again. Bronze is better, especially once work-hardened. Wrought iron is about the same as bronze or a bit better. Steel is clearly the best. The numbers this proposal produces, for edge life, are obsidian 0.140, flint 0.342, copper 0.460, stone 0.659, bronze 0.783, iron 0.804, steel 0.967. Stone sitting above copper is the awkward one: a ground stone axe does outlast soft copper, and a casual flake does not. The sim has one edge class for both. That is discussed with the risks.

## The model

Feature `durability-v2` on `civ-core` (default off; `native/Cargo.toml` forwards it onto the `kindling` binary). With the feature off, `score_durability`, `record_durability` and `upkeep_wear` are bit-identical to the old formula. A unit test checks that for every material and every form (`durability_v2_orders_edges_and_v1_formula_stays`).

With the feature on, life depends on what the object is. `wear_class` (`materials.rs:574`) maps all 32 forms:

| Class | Forms |
| --- | --- |
| Edge | Edge, Point, Spear, Axe |
| Impact | Hammer, Shield |
| Vessel | Vessel, Jar, Frame, Bellows |
| Cordage | Cord, Net, Trap, Garment, Bow, Sail, Cloth |
| Hull | Boat, Outrigger, Plank, Ship |
| Thermal | Kiln, Mould, Crucible |
| Record | Mark |
| Structure | Shaft, Loom |
| Stock | Raw, Bloom, Cooked, Fire, Ornament |

`class_durability` (`materials.rs:591`) is the life. Higher lasts longer. There is no clamp. A unit test checks that two materials share a life only when the properties that class reads are equal. The ties printed by the diagnostic (meat with fish, the four metals on cordage, brine at zero) are that kind of tie. They are not a cap.

**Edge** (`edge_life`, `materials.rs:610`):

```text
keep = 0.65 * hard + 0.35 * sharp
roll = 1 + 0.09 * malleable
chip = tough * (1 - 0.50 * fracture / 10)
life = (keep / roll) * chip / 45 + 0.004 * hard
```

Hardness does most of the work of keeping the edge, acuity the rest. Malleability divides, so a soft edge rolls. Toughness resists the chip, and a high `fracture` (a knapped edge) spends that toughness faster. The `/ 45` puts steel just under 1 and obsidian near 0.14. The `+ 0.004 * hard` exists only because the product alone ties pitch (hard 0.5, tough 2) with clay (hard 1, tough 1). It does not reorder the working edges. That term is a judgment, and it is the one place a hardness nudge was added for a reason other than physics.

**Impact** (`impact_life`, line 621): toughness, cut down hard by fracture, times a face that stays proud when it is hard and dishes when it is malleable.

**Vessel** (line 629): a fibre blend if `fibrous` is high, otherwise a pot that cracks (low toughness, flaking) and abrades.

**Cordage** (line 640): `(0.55 * tough + 0.45 * fibrous) / 12`. No rot. Nothing is immortal, because neither input is 10 on these props.

**Hull** (line 646): toughness, helped by hardness, so a dugout and a skin boat of the same toughness do not tie the way they did. Float and seal stay in `boat_q`.

**Thermal** (line 653): refractory and toughness, if the material has a refractory rating. If it does not, but it melts, the body is treated as fired clay (refractory 9, tough 2) stressed by how hot the charge has to be (`melt / 2000`). That is a judgment: the crucible item's material is the charge, and the clay is what cracks.

**Record** (line 665): `(0.55 * hard + 0.45 * tough) / 10`. A tally weathers. It is not an edge. The extra +1 for a ceramic token stays in the kit choice, not in this number.

**Structure** (line 670): a shaft or a loom breaks. Toughness, a little hardness, hurt by flaking.

**Stock** (line 602): `tough / 12`. Raw stock, a bloom, and the forms whose upkeep never reaches this model (fire, cooked, ornament return before the wear term).

**Repair** (`repair_factor`, line 679): `1 - 0.045 * malleable`. Copper (9) is 0.595, bronze and iron (6) are 0.730, steel (5.5) is 0.752, gold (10) is 0.550. A material with malleability 0 stays at 1: you knap another edge, or you throw it out. Malleability on these props is at most 10, so the factor stays above 0.5 with no clamp. A crucible whose material is the charge (`refractory` 0, class Thermal) does not get the metal's discount: you are not re-forging the clay body. That exception is a judgment.

**Wear charged as labour** (`upkeep_wear`, line 690): feature off, `1.15 - durability`. Feature on, `(1.15 - class life) * repair_factor` for the item's form. The 1.15 is the old upkeep's zero point, kept so a life near 1 still costs a little and the feature-off path does not move. No current material reaches 1.15, so wear stays positive. The unit test asserts life in `[0, 1.15)` and repair in `(0.5, 1]`. A future material could cross 1.15 and then upkeep would go negative. There is no floor. That is a question, not a silent clamp.

Working-edge wear after repair, the number that actually hits the labour pool: steel 0.138, iron 0.252, bronze 0.268, copper 0.411, stone 0.491, flint 0.808, obsidian 1.010. Steel wears less than iron, iron a bit less than bronze, bronze less than copper, copper less than flint, flint less than obsidian. Repair did not put copper back in front. Easy to re-sharpen is not the same as cheap to keep using, at these constants: copper's repair factor is the best of the tool metals (0.595 against steel's 0.752) and the wear is still about three times steel's.

The old function `durability` is left in place. The sim does not call it when the feature is on. `tests.rs:1799` still asserts `durability(steel) >= 0.95 * durability(iron)` on that old function. That assertion is the bug-shaped comparison (0.979 against 1.0) and it still passes. It does not see this model. The new test checks the edge order.

### Where durability is used, and what the feature changes

`grep` for `durability(` on this branch: the old function remains the feature-off body. The sim's sites go through the three helpers. `save.rs` has no durability field.

| Site | What it is | Class the feature uses | What changes |
| --- | --- | --- | --- |
| `materials.rs:543` `durability` | The old scalar. Feature-off body of the helpers. | None | Unchanged. Still capped. |
| `effects.rs:374` `tool_upkeep` | Economy labour for a worn tool: `sealed * rate * upkeep_wear * (1 + 0.3 * depth)`. Ornaments return 0. Fire and cooked return 0.01. | `wear_class` of that item | Feature on, wear is class life times repair. `world.rs` sums `tool_upkeep` and does not call `durability` itself. |
| `effects.rs:517` `axe_term` | Food ranking of an axe: half `cut_q`, half durability. | Edge | Copper axe score 0.740 → 0.470. Bronze 0.825 → 0.716. Iron 0.820 → 0.722. Steel 0.880 → 0.874. |
| `effects.rs:665` `cand_new` | One candidate in the kit choice. `dur` is the axe arm only. `dur_term` is `upkeep_wear` for every candidate. | Edge for the axe score; the form's class for labour | Same scores as the row above. Labour follows the class. |
| `effects.rs:837` `fight_of` slot 4 | Fight term of the axe, `0.3 *` the axe score. Not food. | Edge | The fight number moves with the axe score. |
| `effects.rs:865` `apply_base` | The axe multiplier on plants (`+0.1 * axe`) and the farm (`+0.2 * axe`), and the axe inside protection. | Edge | This is the food the band actually gets, not only the rank. |
| `effects.rs:1051` `nonfood_scores` | Boat, outrigger, plank, ship, bow, shield. Ranked by material term minus `KIT_UPKEEP * upkeep_wear * depth`. Not the economy's upkeep. | Hull for the boats (slots 11, 16, 18, 22). Cordage for the bow (14). Impact for the shield (15). | `KIT_UPKEEP` stays 0.06 (`band.rs:734`). Only the wear factor changes. |
| `effects.rs:1264` mark | "The mark that lasts": `record_durability + 1` if ceramic. | Record | Wood 0.500 → 0.380. Bone 0.417 → 0.445. Ceramic 0.167 → 0.393, and the +1 still puts a fired token first. Bone now beats a wooden tally. |
| `band.rs:715` `weapon_q` | Axe term inside how dangerous the band's weapons are. Fight, not food. | Edge | Same axe-score change. |
| `band.rs:741` `edge_q` | `0.55 * cut_q + 0.45 * score_durability`. Only `weapon_q` and `fight_of` read it. Food does not (skills step 1). | Edge | Copper 0.714 → 0.471. Steel 0.870 → 0.864. |
| `probe_diffusion.rs:159` | The probe's stand-in upkeep: `KIT_UPKEEP * upkeep_wear * depth`. Not the economy. | The payoff item's class | The probe's "true value" moves. The kit choice for food slots does not use this line. |

`cut_q` is unchanged: `(0.6 * sharp + 0.4 * hard) / 10`, clamped to 0..1. Copper 0.480, bronze 0.650, iron 0.640, steel 0.780.

### Property values that look wrong for this job

Not changed. Listed so a later edit is a decision.

- Steel `hard` 6 against iron 5.5 is too close for edge retention. The comment on steel already put the real difference in `sharp` (9 against 7). This formula uses both, and steel still wins clearly (life 0.967 against 0.804, wear 0.138 against 0.252). A reader who wants steel to last several times longer, which is the workshop story, will not get it from these props. Raising steel's hardness would do that, and would also change `cut_q` in the default build. Out of scope here.
- `fracture` is knapping, not brittleness. Ceramic at 0 is right. Do not "fix" it.
- Copper `malleable` 9 is right for "easy to re-forge", and it now does that job (repair 0.595, the cheapest tool metal). It also rolls the edge, which is why copper's life falls to 0.460. Both effects are intended. The balance question is the size, not the sign.
- Bronze and iron share `malleable` 6 and `tough` 9. Bronze is a little keener (`sharp` 7.5 against 7), iron a little harder (5.5 against 5). Edge life comes out bronze 0.783, iron 0.804. That matches "wrought iron about bronze or a bit better". The gap is small on purpose of these props, not because of a cap.
- Gold `hard` 2.5, `malleable` 10. Old durability 0.833 made gold look like a lasting tool metal. Edge life is now 0.125. That is the point of the change. Gold remains the ornament (`prized`).
- There is no rot, no work-hardening state, and no separate "ground stone" versus "flake". Stone's edge life (0.659) beats copper (0.460) because stone is harder and less malleable and not very prone to knapping (`fracture` 3.5). A ground celt against a soft copper axe is fair. The same number on a stone knife is generous.

## Every material, before and after

Printed by `diag_durability_table` (ignored, diagnostic). The same table with the feature off and on: the columns are pure functions of the props. `old` is today's `durability`. The nine life columns are the proposal. `repair` is the edge repair factor (thermal crucibles differ: see above). `oldWear` is `1.15 - old`. `edgeWear` is `(1.15 - edge life) * repair`. `axeOld` / `axeNew` are the axe score (half cut, half durability or edge life). `eqOld` / `eqNew` are `edge_q`. `markNew` is the mark rank (record life, plus 1 for ceramic).

| Material | old | edge | impact | vessel | cord | hull | thermal | record | struct | stock | repair | oldWear | edgeWear | axeOld | axeNew | eqOld | eqNew | markNew |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| wood | 0.500 | 0.275 | 0.534 | 0.365 | 0.425 | 0.438 | 0.175 | 0.380 | 0.480 | 0.500 | 1.000 | 0.650 | 0.875 | 0.350 | 0.237 | 0.335 | 0.234 | 0.380 |
| stone | 0.583 | 0.659 | 0.518 | 0.473 | 0.321 | 0.595 | 0.421 | 0.645 | 0.542 | 0.583 | 1.000 | 0.567 | 0.491 | 0.502 | 0.540 | 0.493 | 0.528 | 0.645 |
| flint | 0.292 | 0.342 | 0.140 | 0.151 | 0.160 | 0.303 | 0.265 | 0.543 | 0.204 | 0.292 | 1.000 | 0.858 | 0.808 | 0.526 | 0.551 | 0.549 | 0.572 | 0.543 |
| obsidian | 0.125 | 0.140 | 0.050 | 0.056 | 0.069 | 0.126 | 0.044 | 0.370 | 0.080 | 0.125 | 1.000 | 1.025 | 1.010 | 0.472 | 0.480 | 0.507 | 0.514 | 0.370 |
| clay | 0.083 | 0.018 | 0.082 | 0.057 | 0.046 | 0.066 | 0.462 | 0.100 | 0.073 | 0.083 | 1.000 | 1.067 | 1.132 | 0.062 | 0.029 | 0.060 | 0.030 | 0.100 |
| copper ore | 0.250 | 0.172 | 0.263 | 0.220 | 0.138 | 0.240 | 0.196 | 0.344 | 0.248 | 0.250 | 1.000 | 0.900 | 0.978 | 0.201 | 0.162 | 0.196 | 0.161 | 0.344 |
| tin ore | 0.333 | 0.383 | 0.359 | 0.320 | 0.183 | 0.343 | 0.225 | 0.538 | 0.348 | 0.333 | 1.000 | 0.817 | 0.767 | 0.297 | 0.321 | 0.293 | 0.315 | 0.538 |
| plant fibre | 0.250 | 0.024 | 0.218 | 0.225 | 0.475 | 0.184 | 0.087 | 0.162 | 0.204 | 0.250 | 1.000 | 0.900 | 1.126 | 0.135 | 0.022 | 0.123 | 0.022 | 0.162 |
| hide | 0.500 | 0.091 | 0.490 | 0.358 | 0.500 | 0.398 | 0.175 | 0.325 | 0.440 | 0.500 | 1.000 | 0.650 | 1.059 | 0.270 | 0.065 | 0.247 | 0.063 | 0.325 |
| bone | 0.417 | 0.422 | 0.393 | 0.338 | 0.229 | 0.404 | 0.146 | 0.445 | 0.390 | 0.417 | 1.000 | 0.733 | 0.728 | 0.423 | 0.426 | 0.424 | 0.426 | 0.445 |
| meat | 0.083 | 0.004 | 0.061 | 0.041 | 0.046 | 0.058 | 0.029 | 0.056 | 0.064 | 0.083 | 1.000 | 1.067 | 1.146 | 0.046 | 0.006 | 0.042 | 0.006 | 0.056 |
| wild plants | 0.083 | 0.009 | 0.073 | 0.046 | 0.121 | 0.061 | 0.029 | 0.073 | 0.068 | 0.083 | 1.000 | 1.067 | 1.141 | 0.052 | 0.015 | 0.049 | 0.015 | 0.073 |
| fish | 0.083 | 0.004 | 0.061 | 0.041 | 0.046 | 0.058 | 0.029 | 0.056 | 0.064 | 0.083 | 1.000 | 1.067 | 1.146 | 0.046 | 0.006 | 0.042 | 0.006 | 0.056 |
| charcoal | 0.083 | 0.018 | 0.082 | 0.057 | 0.046 | 0.066 | 0.029 | 0.100 | 0.073 | 0.083 | 1.000 | 1.067 | 1.132 | 0.062 | 0.029 | 0.060 | 0.030 | 0.100 |
| ceramic | 0.167 | 0.196 | 0.191 | 0.165 | 0.092 | 0.168 | 0.546 | 0.393 | 0.179 | 0.167 | 1.000 | 0.983 | 0.954 | 0.223 | 0.238 | 0.229 | 0.242 | 1.393 |
| copper | 1.000 | 0.460 | 0.645 | 0.666 | 0.413 | 0.697 | 0.442 | 0.570 | 0.756 | 0.750 | 0.595 | 0.150 | 0.411 | 0.740 | 0.470 | 0.714 | 0.471 | 0.570 |
| tin | 0.750 | 0.103 | 0.319 | 0.314 | 0.229 | 0.350 | 0.524 | 0.308 | 0.386 | 0.417 | 0.640 | 0.400 | 0.670 | 0.450 | 0.126 | 0.420 | 0.129 | 0.308 |
| bronze | 1.000 | 0.783 | 0.744 | 0.733 | 0.413 | 0.748 | 0.455 | 0.680 | 0.797 | 0.750 | 0.730 | 0.150 | 0.268 | 0.825 | 0.716 | 0.808 | 0.710 | 0.680 |
| grain | 0.083 | 0.018 | 0.082 | 0.057 | 0.046 | 0.066 | 0.029 | 0.100 | 0.073 | 0.083 | 1.000 | 1.067 | 1.132 | 0.062 | 0.029 | 0.060 | 0.030 | 0.100 |
| flax | 0.375 | 0.034 | 0.326 | 0.355 | 0.562 | 0.276 | 0.131 | 0.230 | 0.306 | 0.375 | 1.000 | 0.775 | 1.115 | 0.198 | 0.027 | 0.180 | 0.027 | 0.230 |
| wool | 0.333 | 0.019 | 0.263 | 0.267 | 0.483 | 0.236 | 0.117 | 0.196 | 0.261 | 0.333 | 1.000 | 0.817 | 1.131 | 0.173 | 0.015 | 0.157 | 0.015 | 0.196 |
| iron ore | 0.333 | 0.294 | 0.355 | 0.308 | 0.183 | 0.333 | 0.225 | 0.455 | 0.340 | 0.333 | 1.000 | 0.817 | 0.856 | 0.267 | 0.247 | 0.260 | 0.243 | 0.455 |
| iron | 1.000 | 0.804 | 0.754 | 0.744 | 0.413 | 0.757 | 0.399 | 0.707 | 0.804 | 0.750 | 0.730 | 0.150 | 0.252 | 0.820 | 0.722 | 0.802 | 0.714 | 0.707 |
| brine | 0.050 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.100 | 1.150 | 0.025 | 0.000 | 0.022 | 0.000 | 0.000 |
| salt | 0.083 | 0.046 | 0.091 | 0.071 | 0.046 | 0.075 | 0.029 | 0.183 | 0.082 | 0.083 | 1.000 | 1.067 | 1.104 | 0.092 | 0.073 | 0.093 | 0.076 | 0.183 |
| pitch | 0.167 | 0.016 | 0.145 | 0.096 | 0.092 | 0.123 | 0.058 | 0.117 | 0.136 | 0.167 | 1.000 | 0.983 | 1.134 | 0.093 | 0.018 | 0.086 | 0.018 | 0.117 |
| steel | 0.979 | 0.967 | 0.769 | 0.754 | 0.413 | 0.765 | 0.407 | 0.735 | 0.810 | 0.750 | 0.752 | 0.171 | 0.138 | 0.880 | 0.874 | 0.870 | 0.864 | 0.735 |
| gold | 0.833 | 0.125 | 0.340 | 0.356 | 0.229 | 0.377 | 0.444 | 0.363 | 0.411 | 0.417 | 0.550 | 0.317 | 0.563 | 0.497 | 0.143 | 0.463 | 0.144 | 0.363 |
| resinous wood | 0.333 | 0.124 | 0.356 | 0.270 | 0.183 | 0.292 | 0.117 | 0.290 | 0.320 | 0.333 | 1.000 | 0.817 | 1.026 | 0.207 | 0.102 | 0.194 | 0.100 | 0.290 |

Brine's old 0.050 is the floor. Its new life is 0 because every input is 0, not because of a floor. Copper, bronze and iron sharing 1.000 is the cap. They do not share an edge life. They do share cordage life (0.413) and stock life (0.750), because all four have toughness 9 and no fibrousness. That is the same inputs, and a metal cord is a poor cord next to hide (0.500) or flax (0.562).

Wear on the other shapes, for the materials that actually get made into them. Old wear is one number. New wear depends on the form. Shield and hammer are impact. Bow, net and garment are cordage. Boat is hull. Vessel is vessel. Kiln, mould and crucible are thermal.

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

A ceramic crucible or kiln wears less than it does today (0.604 against 0.983), which is the thermal formula doing its job. A copper tool that is not an edge wears more than today's 0.150, because today's 0.150 was the cap, not a measurement of a copper hull or a copper cord.

## What bands do with it

Save: `DIAG_SAVE`, which in this run was `/workspace/bots/build-bot/step3-saves/main_seed6_y350_256x160.civsave`. Format v16, year 350, 3354 bands, population 67836. Not a user save. 2308 bands count as farming and 1046 as foraging. A band is a farm when the grain share of its harvest is at least 0.10, or `sow >= 0.2`. That is the same split the axe diagnostic uses.

`diag_durability_flips` scores every payoff slot both ways. It does not trust the feature flag for the comparison: both prices are computed, and the live model is only checked against `slot_score_m`. That check was 0 mismatches of 103022 either way. The kit after load matched the live pick on 0 mismatches of 31774 either way.

Axe argmax, all bands (51 bands have no axe they can make; 3234 can make steel):

| | copper | iron | steel | stone |
| --- | ---: | ---: | ---: | ---: |
| v1, today's formula | 806 | 264 | 2209 | 24 |
| v2, this proposal | 16 | 27 | 3234 | 26 |

The 16 copper, 27 iron and 26 stone left under v2 are bands that cannot make steel. 3234 is every steel-capable band. Nobody who can make steel keeps another axe.

Flips, counted in the kit as loaded with the feature off (the save's own rules). Farm, then forage.

| Slot | From | To | Farm | Forage |
| --- | --- | ---: | ---: | ---: |
| axe | copper axe | steel axe | 90 | 682 |
| axe | copper axe | iron axe | 0 | 16 |
| axe | copper axe | stone axe | 1 | 1 |
| axe | iron axe | steel axe | 81 | 172 |
| cut | copper knife | flint blade | 0 | 2 |
| cut | copper knife | obsidian blade | 0 | 1 |
| hunt | copper point | flint point | 0 | 2 |
| mark | tally stick | notched bone tally | 35 | 73 |
| net | hide net | flax net | 163 | 55 |
| net | hide net | plant fibre net | 68 | 349 |
| net | plant fibre net | flax net | 37 | 7 |
| point | copper point | steel point | 1113 | 715 |
| point | iron point | steel point | 819 | 194 |
| trap | plant fibre snare | flax snare | 4 | 0 |
| trap | rawhide snare | flax snare | 0 | 1 |
| trap | rawhide snare | plant fibre snare | 0 | 59 |

Loaded again with the feature on, every axe count above is the same. Three cordage counts move by a handful, because load refits the rest of the kit and the one-slot price sees that kit: hide net to plant fibre, forage 349 → 347; fibre snare to flax, farm 4 → 5; rawhide snare to plant fibre, forage 59 → 55. Nothing else moved.

772 copper axes go to steel (90 of them farms, 682 foragers). That 772 is the same count #11 reported as copper holders among steel-capable bands. All 253 iron holders go to steel. Two copper axes, on bands that are not choosing steel, go to stone. Knives barely move on this save (3 bands). Hunt weapons barely move (2). The axe is the food slot that mixes durability into the yield. A knife's food is `cut_q`. Flint is keener than copper, and flint's wear (0.808 against copper's 0.411) still keeps flint from taking the copper knives on this save, except 2 foragers. One forager's copper knife goes to obsidian.

There is a sharper case the save does not show. `d3_metal_blades_outrank_stone_when_practised` builds a nominal kit at chain depth 0. On the default build an iron knife beats an obsidian knife. With the feature on, the obsidian knife wins (item 67 against iron's 86). Obsidian's `cut_q` is about 0.82 and iron's is 0.64, and the upkeep rate is 0.02, so the chip wear is not charged hard enough to reject the glass edge when nothing else differs. The assertion is split by the feature flag. Feature off, it still expects iron. Feature on, it expects obsidian. Copper still loses the knife to obsidian, and the copper point still beats the obsidian point, under both. On the year-350 save no iron knife flipped: those knives are not the depth-free pair.

Points are the warning. Point food does not depend on the material (`cand_new` gives a point a hunt quality and a zero food term). The slot then ranks by wear. Steel's wear wins wherever steel can be made: 1828 copper points and 1013 iron points flip, farms and foragers together. That is not a finding about arrows. It is the slot ignoring the edge. See the questions.

Nets. Flax cord wear is 0.587, hide 0.650, plant fibre 0.675. Flax also has the better mesh (`(tough + fibrous) / 15` is 0.933 for flax, 0.800 for hide and for fibre). Hide to flax is a real cordage win, and it is mostly farms (163 against 55). Hide and fibre have the same mesh term, and fibre wears slightly faster, so a fibre net beats a hide net only through a shallower chain or a higher recipe quality. This run did not print those depths. The count is large for foragers (349). Do not read it as "fibre lasts longer than hide".

Marks. Record life weights hardness, so bone (0.445) beats wood (0.380). The old number was mostly toughness, so the wooden tally won. 108 bands switch. A fired clay token still wins when it can be made, because of the +1.

## What this does to the steel-axe question in #11

#11, re-run here as `diag_axe_breakeven` with the feature off (model v1), on the same save. Attribution of that diagnostic is in `civ-core/src/tests_diag_axe.rs`: it was copied from draft PR #11. Among 3234 steel-capable bands (population 65734):

- Held axes: copper 772, iron 253, steel 2209. Bronze 0.
- Axe terms: copper 0.740, bronze 0.825, iron 0.820, steel 0.880. Durability 1.000 / 1.000 / 1.000 / 0.979.
- Every copper holder keeps copper. All 772, rank 772 / 0 / 0, margin `cur = +0.0453` (positive means the other metal beats steel). Farms, n=90, `cur = +0.0220`. Foragers, n=682, `cur = +0.0484`.
- Every iron holder keeps iron. n=253, `cur = +0.0175`. Farms n=81, `cur = +0.0147`. Foragers n=172, `cur = +0.0188`.
- Bands already on steel prefer steel against the best alternative. n=2208, `cur = -0.1337`.
- `mis%` is 0 everywhere: the printed margin matches food minus priced labour.

That is #11's picture. A durability tweak that only made steel last a little longer than the cap would move the farmers (their margin is +0.022, close to a tie) and would not move the foragers (margin +0.048, and their food gain from a better axe is small). #11 said so, and it was right about this formula.

It is the wrong forecast for a formula that can see the metals. With the feature on, the same diagnostic (model v2):

- Axe terms: copper 0.470, bronze 0.716, iron 0.722, steel 0.874. Edge life 0.460 / 0.783 / 0.804 / 0.967.
- Held axes, after the save is loaded and the kit is re-chosen: steel 3234, copper 0, iron 0. Every steel-capable band holds steel.
- Against the best other axe, n=3233 (one band has no alternative): `cur = -0.3239`, rank 0 / 3233 / 0. Food gain of steel over the other is about +0.173. Steel's labour is about half the other's (0.0090 against 0.0171) even though steel's chain is deeper (depth 7.57 against 6.26).
- Farms, n=2288: `cur = -0.3910`, all steel.
- Foragers, n=945: `cur = -0.1615`, all steel. The forager margin is smaller than the farm margin and it is not close to a tie.
- Probe preference for steel: 100% of these bands by the probe's own threshold. `mis%` is 0.

The copper-versus-steel investigation was undermined, and the direction of the damage is now measurable. Under the capped formula, foragers who can make steel rationally keep copper, by about 0.045 of the food price. Under this proposal they rationally take steel, by about 0.16, and farmers by about 0.39. The interesting remaining question is not "why do they refuse steel". It is "is a copper axe really that much worse, and do we want the sim to abandon copper the moment steel exists".

Bronze does not appear in the held census. Under both formulas its axe score sits next to iron (v1: bronze 0.825, iron 0.820; v2: 0.716 and 0.722). Availability and chain depth, not the cap, are why this save has no bronze axes.

## Risks, and what to re-check

This is a large balance change, not a 0.979-versus-1.0 rounding fix. Copper's axe score falls from 0.740 to 0.470. Its edge wear rises from 0.150 to 0.411.

- **Population.** Bands that stay on copper or iron pay much more labour for edges. Bands that move to steel pay less wear than today (0.138 against 0.171) but sit on a deeper chain, and the food gain is large. The break-even says the food gain wins the price. It does not say what population does over 300 years. Not run locally. See below.
- **Chain depth.** Steel's depth penalty matters less once copper is this expensive. Bands have a stronger reason to climb to steel, and to climb sooner. Guard rails that watch the year steel appears, and the share of bands on it, will move. That is the point of the change, and it is also how a bad constant would show up.
- **Points.** Steel points win on wear alone, because the point slot's food ignores the material. 2841 point slots flip. If that is not wanted, the point's food term has to learn about the edge, or the wear weight on that slot has to shrink. Do not "fix" it by putting malleability back into edge life.
- **Knives at depth 0.** The same gap, the other way. A nominal iron-versus-obsidian kit keeps the obsidian knife, because `cut_q` pays for keenness and `Params::upkeep` (0.02) does not charge much for chipping. The save's iron knives did not flip. If glass knives should lose to iron, the cut slot has to feel wear more than it does, or the edge score has to enter the knife's food the way it enters the axe.
- **Cordage.** Flax replaces a lot of hide, mostly on farms. Fibre replaces hide on foragers even though fibre wears slightly faster. There is no rot, so plant cordage never gets worse in the wet. A rot term would push back toward hide and would be a new property. Not in this proposal.
- **Stone above copper.** Two axes flip copper to stone. Stone's edge life (0.659) beats copper (0.460). Fine for a ground celt, generous for a flake. One edge class cannot say both.
- **Negative wear.** No clamp. Current props stay under 1.15. A richer material later could make upkeep negative.
- **Fight.** `edge_q` and the axe fight term move with edge life. Copper weapons look much worse in a fight. Steel barely changes (axe score 0.880 to 0.874). `tests.rs` still asserts that steel weapons raise fight strength by less than 1.1 times against an iron-and-bronze kit. That assertion is about iron and bronze, not copper, and those scores stay close. Re-check it with the feature on; do not assume a copper band's fight number is stable.
- **Golden hashes.** Feature off, they must stay `0x1845e2690330d59` (the long run and the first of the v10 pair) and `0x5cef0617c34457c6` (the run-on). The asserts are `#[cfg(not(feature = "durability-v2"))]`, so a feature-on run does not fail them and also does not record the new hash. The feature-on `civ-core` suite was run for this draft and is green (112 passed, 10 ignored). One assertion is split by the flag: at chain depth 0 an obsidian knife beats an iron knife, and the default build still expects the opposite (`tests_diffusion.rs`, `d3_metal_blades_outrank_stone_when_practised`). Record the feature-on hashes when the feature is actually tried. Do not treat a mismatch with the old constants as a failure of this proposal, and do not update the constants in the default build.

No local simulation was run (seed 5, 300 years, one thread). Batch runs are Build Bot's job. The useful pair is the default map, seed 5, 300 years, `--threads 1`, feature off and feature on: population, band count, and axe material counts at the end, plus the usual guard rails (population floor, year of steel, share of bands on steel, chain depth). The ignored diagnostics to re-run on `DIAG_SAVE`, both ways, are `diag_durability_table`, `diag_durability_flips` and `diag_axe_breakeven`.

## Saves

Durability is not saved. `save.rs` does not mention it. It is derived from the material.

The kit is saved. `after_load` (`world.rs:1112`) rebuilds toolkits and calls `refresh_toolkit` (`world.rs:1693`), which calls `fit_kit` (`world.rs:1686`) when the toolkit cache misses or the kit is marked dirty. `fit_kit` calls `make_kit_for_view`. Measured on this save: with the feature off, the loaded kit matches the default scorer (0 mismatches) and the held axes are the file's own (copper 772, iron 253, steel 2209). With the feature on, the same bytes load, the kit is re-chosen, and all 3234 steel-capable bands hold a steel axe. No format version change. A save written under one model loads under the other. The kit may change on load when the feature differs from the build that wrote the file. The bytes do not.

## How this lands relative to skills step 4

PR #12, branch `origin/skills-step4`, head `3521366`, is open and not merged. `git diff origin/main origin/skills-step4` on `effects.rs`, `band.rs` and `probe_diffusion.rs`:

Step 4 does not add a new call of the `durability` function. The old sites are still there, still written as `durability(...)` or `1.15 - durability(...)`:

- `band.rs:716` and `band.rs:742`, the axe fight term and `edge_q`. The diff does not touch these lines. It only changes the comment on the beliefs field.
- `effects.rs` on that branch: `tool_upkeep` (around line 449), `axe_term` (592), `cand_new` (740), the fight axe term (912), `apply_base` (940), `nonfood_scores` (1126), the mark (1557). Same expressions as main.
- `probe_diffusion.rs:161`, `KIT_UPKEEP * (1.15 - durability) * depth`, unchanged as an expression.

What step 4 does add is a new way of *reaching* that wear. Food slots, once the belief table is non-empty, are priced by `choose_food_belief` and `belief_slot_price`. Labour is the stored belief when the band has one. When it does not, labour is `tool_upkeep`, which is the durability wear. So a belief that has already learned a labour cost stops reading durability for that slot. A prior, and every band with an empty table, still reads it. Non-food slots stay on `slot_score_m` and `KIT_UPKEEP`. The probe's own-valuation (`slot_own`) now calls `belief_slot_price` instead of `slot_score_m`. Empty table, and every non-food slot, still `KIT_UPKEEP`.

That is the interaction. This proposal changes the number inside `tool_upkeep` and inside the `KIT_UPKEEP` wear factor. Step 4 decides *whether the food rank looks at that number or at a remembered labour*.

Expected conflicts if the two branches meet:

- `effects.rs` will conflict. Both edits sit in the kit choice. This branch replaces `durability(...)` with `score_durability`, `record_durability` and `upkeep_wear` at the sites listed above. Step 4 inserts `choose_food_belief`, `belief_slot_price` and `make_kit_beliefs` around `choose_food`. The new belief path already calls `tool_upkeep`, so once `tool_upkeep` uses `upkeep_wear` the belief fallback inherits the feature with no extra edit. The conflict is the surrounding functions, not a missed formula.
- `probe_diffusion.rs` will conflict. Step 4 rewrites `slot_own`, the imports, and the adoption counters. This branch changes line 159 from `1.15 - durability(mat(it))` to `upkeep_wear(it)`.
- `band.rs` should merge. Step 4's hunk is the beliefs comment. This branch's hunks are `weapon_q` and `edge_q`.

Recommendation. Land this proposal with the feature **off** either before or after step 4. The default build does not move, so it does not block step 4's hashes. Do not turn the feature on, and do not trust the flip table in this document as the step-4 answer, until both are measured together. A band with a stored labour belief will not re-rank a food slot when wear changes; the belief has replaced `tool_upkeep`. The flips here are the empty-table price (step 3). They are an upper bound on how fast axes move once beliefs exist, not the steady state.

If this merges first, step 4's rebase must keep the helpers. Pasting `1.15 - durability(mat)` back into `nonfood_scores`, the probe, `cand_new` or `edge_q` would restore the cap on those paths while `tool_upkeep` stayed on the new model.

If step 4 merges first, re-apply the helpers at the sites above (the belief path needs nothing beyond `tool_upkeep`), then re-run `diag_durability_flips`, `diag_axe_breakeven`, and the step-4 "believed wrong" share. The food-slot price will not be the one in this document.

## Questions for Michael

1. Are the constants acceptable? Edge keep is `0.65 * hard + 0.35 * sharp`, roll is `0.09 * malleable`, chip discount is `0.50 * fracture / 10`, the scale is `/ 45`, the pitch-versus-clay split is `0.004 * hard`, and repair is `0.045 * malleable`. They were chosen to hit the order above and to avoid a shared cap. They are not fitted to a dataset.
2. Copper's axe score falls from 0.740 to 0.470, and every steel-capable band drops copper and iron. Is that the size you wanted, or should edge life be compressed so steel is clearly best without deleting copper as a working axe?
3. The point slot's food ignores the material, so steel points win on wear alone (2841 flips). Should that slot learn about the edge, or is wear-only ranking what you want for arrows?
4. Stone's edge life beats copper. One class covers a ground axe and a flake knife. Split them, or leave it?
5. There is no rot. Flax nets beat hide. Add a property later, or accept it?
6. Steel's hardness (6 against iron's 5.5) still understates edge retention. Leave `PROPS` alone, as this proposal does?
7. Ship the feature-off code before step 4, or rebase onto step 4 first? Either way the feature should stay off until the flip table is re-run with beliefs. My recommendation is: merge the off code whenever it is convenient, and do not enable it until that re-measure.
8. Wear has no floor. If a later material's life exceeds 1.15, upkeep goes negative. Floor it at 0?
9. The `kindling` crate forwards the feature (`durability-v2 = ["civ-core/durability-v2"]`) even though `civ-core` is a target-specific dependency. The native build accepts it. Say if you would rather the flag live only on `civ-core`.

## How to run the diagnostics

Feature off (today's sim; the table still prints both models):

```text
cargo test --release -p civ-core diag_durability -- --ignored --nocapture
cargo test --release -p civ-core diag_axe_breakeven -- --ignored --nocapture
```

Feature on (the sim uses the new model):

```text
cargo test --release -p civ-core --features durability-v2 diag_durability -- --ignored --nocapture
cargo test --release -p civ-core --features durability-v2 diag_axe_breakeven -- --ignored --nocapture
```

`DIAG_SAVE` selects the snapshot. The ordering test `durability_v2_orders_edges_and_v1_formula_stays` is not ignored and runs in the normal suite.
