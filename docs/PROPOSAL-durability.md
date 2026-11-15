# Proposal: material-driven tool durability and wear

This is a proposal. The default build does not use it. Cargo feature `durability-v2` is off unless you ask for it, and with it off the simulation is unchanged (the golden hashes stay `0x1845e2690330d59` and `0x5cef0617c34457c6`). Nothing here is a retune of the live game. Michael decides whether the feature is ever turned on.

**Where this draft stands.** Two corrections after the first pass, both inside the feature. The default build is untouched.

Food for a knife, a hunt weapon and a point is a working edge, not the fresh keenness. Obsidian and flint start sharper than iron and then chip. The first pass let an obsidian knife beat an iron knife at chain depth 0, which is the same class of reversal this proposal is trying to remove. That is gone. A flint knife still beats a copper knife, on purpose: flint's working edge is 0.460 and soft copper's is 0.324, while bronze, iron and steel (0.565, 0.565 and 0.765 at three decimals) all beat flint. A priced copper knife beats obsidian, and a copper point still beats an obsidian point.

Edge life is compressed toward wrought iron. The raw formula is still computed (spread 1.0). The sim uses spread 0.50, which I recommend. Working-edge life, longest first: steel 0.884, iron 0.802, bronze 0.791, stone 0.730, copper 0.630, flint 0.571, obsidian 0.470. Stone sits above copper because a ground stone edge does not roll and does not knap itself apart. The same class also covers a flake, which that number flatters. Copper's edge wear is about 1.55 times steel's (0.310 against 0.200), not the threefold gap of the raw formula. On the year-350 seed-6 save, 3150 of 3234 steel-capable bands hold a steel axe, 81 still hold copper and 3 hold iron. Spread 1.0 puts steel in every one of those 3234 bands (4766 band-slots flip). Spread 0.50 flips 4550. Spread 0.35 flips 3862 and keeps 331 copper axes, but the bronze axe term then sits just above iron, so it is not the default.

## The bug, in plain words

A steel axe wears out faster than a copper axe.

The sim scores how long a material lasts with one number, in `civ-core/src/materials.rs`:

```text
durability(m) = ((tough + 0.5 * malleable) / 12).clamp(0.05, 1.0)
```

That is `durability` at `materials.rs:543`. Toughness and malleability are added together, divided by 12, and then crushed into the range 0.05 to 1. Copper (tough 9, malleable 9) comes out at 1.125 and is clamped to 1. Bronze and iron land on exactly 1. Steel (tough 9, malleable 5.5) comes out at 0.979, just under the cap. Wear in the upkeep formula is `1.15 - durability`, so copper, bronze and iron all wear at 0.150 and steel wears at 0.171. The metal that should hold an edge is the one the sim throws away first.

Malleability is the wrong thing to reward for a working edge. A soft metal bends, rolls and goes dull. What keeps an edge working is hardness and how keen an edge it can take. What stops it chipping or snapping is toughness. Malleability matters afterwards: a soft edge is easier to hammer back into shape, and a malleable piece is less likely to shatter. It should make repair cheaper, not make the edge last longer.

The cap is what hides the difference. Copper, bronze and iron are not equally durable. The formula cannot say so, because all three sit on 1.0. Only brine sits on the 0.05 floor (every property it has is zero). Gold (0.833) and tin (0.750) sit under the cap and look "durable" because they are soft.

Draft PR #11 (`docs/ANALYSIS-steel-axe.md`) asked why bands that can make steel still carry copper axes. It answered that question with this formula. The answer cannot be trusted until durability can tell the metals apart. The measurement below says the cap was not hiding a 2% gap. The raw edge formula makes copper much worse than steel, and every steel-capable band then drops copper and iron. That is the over-correction the spread is there to shrink. It does not shrink it away.

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

Rough order for how long a working edge should last, as a target, not as a fitted curve. Judgment, from the usual workshop picture rather than from a measured test in this repo: obsidian and flint are short, because they chip. Copper is short-ish: it dulls, and it is easy to hammer sharp again. Bronze is better, especially once work-hardened. Wrought iron is about the same as bronze or a bit better. Steel is clearly the best. Stone is the awkward one. A ground stone edge outlasts soft copper: it is harder, it does not roll, and it is not very prone to knapping (`fracture` 3.5). A casual flake does not, and the sim has one edge class for both.

The raw formula (spread 1) produces obsidian 0.140, flint 0.342, copper 0.460, stone 0.659, bronze 0.783, iron 0.804, steel 0.967. The sim does not use those as the life it charges. It pulls them toward 0.80 by half (spread 0.50): obsidian 0.470, flint 0.571, copper 0.630, stone 0.730, bronze 0.791, iron 0.802, steel 0.884. Same order. Smaller gaps. Stone stays above copper on life, below bronze.

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

Hardness does most of the work of keeping the edge, acuity the rest. Malleability divides, so a soft edge rolls. Toughness resists the chip, and a high `fracture` (a knapped edge) spends that toughness faster. The `/ 45` puts raw steel just under 1 and raw obsidian near 0.14. The `+ 0.004 * hard` exists only because the product alone ties pitch (hard 0.5, tough 2) with clay (hard 1, tough 1). It does not reorder the working edges. That term is a judgment, and it is the one place a hardness nudge was added for a reason other than physics.

**Compression** (`scale_edge_life`, `EDGE_LIFE_ANCHOR` 0.80, `EDGE_LIFE_SPREAD` 0.50). The anchor is raw wrought iron, so iron and bronze barely move. Spread 1 returns the raw life bit for bit. Any other spread is `anchor + spread * (raw - anchor)`. Order is unchanged for a positive spread. Only the edge class is compressed. A cord, a hull, a pot, a mark and a shield use the formulas below with no spread. `class_durability` of an edge, and therefore `score_durability`, axe food, `edge_q` and edge upkeep, all read the compressed life. Diagnostics can score spread 1.0 and 0.35 without editing the constant. 0.35 is printed so it can be chosen; it is not what the sim uses, because at 0.35 the bronze axe term (0.722) sits just above iron (0.721).

**Working-edge food** (`working_edge_q`). Fresh keenness is `cut_q`. A season of use is not a fresh edge:

```text
working_edge_q = cut_q * (0.40 + 0.60 * raw_edge_life)
```

The 0.40 is `EDGE_KEEP_FLOOR`: a hopelessly short edge still keeps 40% of its keenness, and a life of 1 keeps all of it. The life in this product is the raw life, not the compressed one. Compressing it as well would let obsidian outrank iron again, which is the reversal this term exists to stop. Feature off, `food_edge_q` is `cut_q`, the same bits as before. Feature on, it is `working_edge_q` unless a diagnostic forces the old model.

Measured working-edge quality, which does not move when the spread changes: steel 0.765, bronze 0.565, iron 0.565, flint 0.460, obsidian 0.397, stone 0.334, copper 0.324, bone 0.281. Bronze is a hair above iron before rounding (0.5654 against 0.5647) because bronze is keener and the life gap is small. The priced knife at depth 0 still ranks iron just ahead of bronze, because iron wears less.

Where it is paid. The knife (`cut_term`) and the hunt weapon (`hunt_q_of`: spear, point, or a held edge) use `food_edge_q` instead of `cut_q`. The point slot's fish term used to be 1 whenever any point existed. Feature on, `point_food_scale` is `working_edge_q`, so a glass point is not a full point. It is not renormalized: a steel point feeds at 0.765, not at today's flat 1.0, and a copper point at 0.324. That shrinks fish for every band. Whether to divide by steel's quality so the best point matches today's 1.0 is a question, not a silent second constant. The axe stays `0.5 * cut_q + 0.5 * compressed life`. Switching the axe onto the product form would widen the copper penalty, which is the opposite of the spread. Fight stays on `edge_q` and on the axe blend. Food does not call `edge_q`.

Choice, stated. A flint knife beats a copper knife. Flint is much keener (cut 0.76 against 0.48), and a raw life of 0.342 is short but not obsidian-short, so the discount does not hand the knife to soft copper. Bronze, iron and steel clear flint on the same number. A copper knife beats an obsidian knife once the chip is in the food and in the wear: obsidian's working edge (0.397) is still above copper's (0.324), and obsidian's wear (0.680 against 0.310) is what makes the priced copper knife win, including at depth 0. The copper point beats the obsidian point on both builds. The flint point beats the copper point only with the feature on, for the same reason the flint knife does. The default build still keeps the copper point, because every point used to fish the same and copper wears less.

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

Working-edge wear after repair, at the recommended spread 0.50, the number that actually hits the labour pool: steel 0.200, iron 0.254, bronze 0.262, copper 0.310, stone 0.420, flint 0.579, obsidian 0.680. Steel wears less than iron, iron a bit less than bronze, bronze less than copper, copper less than stone, stone less than flint, flint less than obsidian. Repair did not put copper back in front of steel. Easy to re-sharpen is not the same as cheap to keep using: copper's repair factor is the best of the tool metals (0.595 against steel's 0.752) and the wear is still about 1.55 times steel's. At spread 1.0 the same wear is steel 0.138, iron 0.252, bronze 0.268, copper 0.411, and copper is about three times steel. Compression is what cuts that from three times to one and a half. It also moves steel's own wear from today's 0.171 up to 0.200. Steel is not cheaper to keep than it is today. Copper is more expensive (0.150 today, 0.310 here), and the food gap is what moves the axe.

The old function `durability` is left in place. The sim does not call it when the feature is on. `tests.rs:1799` still asserts `durability(steel) >= 0.95 * durability(iron)` on that old function. That assertion is the bug-shaped comparison (0.979 against 1.0) and it still passes. It does not see this model. The new test checks the edge order.

### Where durability is used, and what the feature changes

`grep` for `durability(` on this branch: the old function remains the feature-off body. The sim's sites go through the three helpers. `save.rs` has no durability field.

| Site | What it is | Class the feature uses | What changes |
| --- | --- | --- | --- |
| `materials.rs:545` `durability` | The old scalar. Feature-off body of the helpers. | None | Unchanged. Still capped. |
| `materials.rs:701` `working_edge_q` | Keenness discounted by raw edge life. | Edge, raw life | Feature-on food only. Not wear, not fight. |
| `effects.rs:370` `tool_upkeep` | Economy labour for a worn tool: `sealed * rate * upkeep_wear * (1 + 0.3 * depth)`. Ornaments return 0. Fire and cooked return 0.01. | `wear_class` of that item | Feature on, wear is class life times repair. Edge life is the compressed one. `world.rs` sums `tool_upkeep` and does not call `durability` itself. |
| `effects.rs:498` `hunt_q_of` | Food of the hunt slot. | Working edge | Feature on, `food_edge_q` instead of `cut_q`. Feature off, still `cut_q`. |
| `effects.rs:511` `cut_term` | Food of the knife. | Working edge | Same switch as the hunt. |
| `effects.rs:518` `point_food_scale` | Fish factor of the point. | Working edge | Feature off, 1. Feature on, `working_edge_q` (steel 0.765, not 1). |
| `effects.rs:530` `axe_term` | Food ranking of an axe: half `cut_q`, half durability. | Edge, compressed | Copper axe score 0.740 → 0.555. Bronze 0.825 → 0.721. Iron 0.820 → 0.721. Steel 0.880 → 0.832. At three decimals bronze and iron tie; iron is higher before rounding. |
| `effects.rs:692` `cand_new` | One candidate in the kit choice. `dur` is the axe arm only. `dur_term` is `upkeep_wear` for every candidate. The point's food term is `point_food_scale`. | Edge for the axe score; the form's class for labour | Same axe scores as the row above. Labour follows the class. |
| `effects.rs:875` `fight_of` slot 4 | Fight term of the axe, `0.3 *` the axe score. Not food. | Edge | The fight number moves with the axe score. |
| `effects.rs:877` `fight_of` slot 14 | The bow. Flat 0.3 if any point exists, else 0. | None | Not changed. The audit's finding that the bow ignores the point is noted below and left alone. |
| `effects.rs:901` `apply_base` | The axe multiplier on plants (`+0.1 * axe`) and the farm (`+0.2 * axe`), and the axe inside protection. Knife, hunt and point yields use the working edge. | Edge | This is the food the band actually gets, not only the rank. |
| `effects.rs:1087` `nonfood_scores` | Boat, outrigger, plank, ship, bow, shield. Ranked by material term minus `KIT_UPKEEP * upkeep_wear * depth`. Not the economy's upkeep. | Hull for the boats (slots 11, 16, 18, 22). Cordage for the bow (14). Impact for the shield (15). | `KIT_UPKEEP` stays 0.06 (`band.rs:735`). Only the wear factor changes. The bow's string is still cordage. The bow's fight term is still flat. |
| `effects.rs:1308` mark | "The mark that lasts": `record_durability + 1` if ceramic. | Record | Wood 0.500 → 0.380. Bone 0.417 → 0.445. Ceramic 0.167 → 0.393, and the +1 still puts a fired token first. Bone now beats a wooden tally. |
| `band.rs:701` `weapon_q` | Axe term inside how dangerous the band's weapons are. Fight, not food. | Edge | Same axe-score change. |
| `band.rs:741` `edge_q` | `0.55 * cut_q + 0.45 * score_durability`. Only `weapon_q` and `fight_of` read it. Food does not call it. | Edge, compressed | Copper 0.714 → 0.547. Steel 0.870 → 0.827. Bronze 0.808 → 0.714. Iron 0.802 → 0.713. Bronze stays a hair ahead of iron on this blend, as it already was under the cap, because the blend is mostly keenness. |
| `probe_diffusion.rs:159` | The probe's stand-in upkeep: `KIT_UPKEEP * upkeep_wear * depth`. Not the economy. | The payoff item's class | The probe's "true value" moves. The kit choice for food slots does not use this line. |

`cut_q` is unchanged: `(0.6 * sharp + 0.4 * hard) / 10`, clamped to 0..1. Copper 0.480, bronze 0.650, iron 0.640, steel 0.780, flint 0.760, obsidian 0.820, stone 0.420.

### Property values that look wrong for this job

Not changed. Listed so a later edit is a decision.

- Steel `hard` 6 against iron 5.5 is too close for edge retention. The comment on steel already put the real difference in `sharp` (9 against 7). This formula uses both. Raw life is steel 0.967 against iron 0.804, raw wear 0.138 against 0.252. Compressed, the life is 0.884 against 0.802 and the wear is 0.200 against 0.254. A reader who wants steel to last several times longer, which is the workshop story, will not get it from these props, and the spread makes that gap smaller on purpose. Raising steel's hardness would widen it, and would also change `cut_q` in the default build. Out of scope here.
- `fracture` is knapping, not brittleness. Ceramic at 0 is right. Do not "fix" it.
- Copper `malleable` 9 is right for "easy to re-forge", and it now does that job (repair 0.595, the cheapest tool metal). It also rolls the edge, which is why raw copper life is 0.460 and compressed life is 0.630. Both effects are intended. The balance question is the size, and the spread is the knob for that size.
- Bronze and iron share `malleable` 6 and `tough` 9. Bronze is a little keener (`sharp` 7.5 against 7), iron a little harder (5.5 against 5). Raw edge life is bronze 0.783, iron 0.804. Compressed, 0.791 and 0.802. That matches "wrought iron about bronze or a bit better". The gap is small because of these props. Spread 0.35 closes it far enough that the axe term flips the other way. Spread 0.50 does not.
- Gold `hard` 2.5, `malleable` 10. Old durability 0.833 made gold look like a lasting tool metal. Raw edge life is 0.125, compressed 0.463. Gold remains the ornament (`prized`). It is still a poor edge (axe score 0.311, was 0.497).
- There is no rot, no work-hardening state, and no separate "ground stone" versus "flake". Compressed stone life (0.730) beats copper (0.630) for the reason above. A ground celt against a soft copper axe is fair. The same number on a stone knife is generous. The priced axe still prefers copper to stone, because copper repairs and stone's axe term (0.575 against 0.555) does not pay for stone's wear (0.420 against 0.310).

## Every material, before and after

Printed by `diag_durability_table` with the feature on, spread 0.50 (ignored, diagnostic). The columns are pure functions of the props, so the feature-off binary prints the same proposal columns; only the sim's own scores change with the flag. `old` is today's `durability`. `rawEdge` is spread 1. `edge` is the compressed life the sim charges. The other life columns are not edges and are not compressed. `repair` is the edge repair factor (thermal crucibles differ: see above). `oldWear` is `1.15 - old`. `edgeWear` is `(1.15 - compressed edge) * repair`. `axeOld` / `axeNew` are the axe score (half cut, half old durability or compressed edge life). `eqOld` / `eqNew` are `edge_q`. `markNew` is the mark rank (record life, plus 1 for ceramic). `workQ` is `working_edge_q`, from the raw life, so it does not move with the spread.

| Material | old | rawEdge | edge | impact | vessel | cord | hull | thermal | record | struct | stock | repair | oldWear | edgeWear | axeOld | axeNew | eqOld | eqNew | markNew | workQ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| wood | 0.500 | 0.275 | 0.537 | 0.534 | 0.365 | 0.425 | 0.438 | 0.175 | 0.380 | 0.480 | 0.500 | 1.000 | 0.650 | 0.613 | 0.350 | 0.369 | 0.335 | 0.352 | 0.380 | 0.113 |
| stone | 0.583 | 0.659 | 0.730 | 0.518 | 0.473 | 0.321 | 0.595 | 0.421 | 0.645 | 0.542 | 0.583 | 1.000 | 0.567 | 0.420 | 0.502 | 0.575 | 0.493 | 0.559 | 0.645 | 0.334 |
| flint | 0.292 | 0.342 | 0.571 | 0.140 | 0.151 | 0.160 | 0.303 | 0.265 | 0.543 | 0.204 | 0.292 | 1.000 | 0.858 | 0.579 | 0.526 | 0.666 | 0.549 | 0.675 | 0.543 | 0.460 |
| obsidian | 0.125 | 0.140 | 0.470 | 0.050 | 0.056 | 0.069 | 0.126 | 0.044 | 0.370 | 0.080 | 0.125 | 1.000 | 1.025 | 0.680 | 0.472 | 0.645 | 0.507 | 0.662 | 0.370 | 0.397 |
| clay | 0.083 | 0.018 | 0.409 | 0.082 | 0.057 | 0.046 | 0.066 | 0.462 | 0.100 | 0.073 | 0.083 | 1.000 | 1.067 | 0.741 | 0.062 | 0.225 | 0.060 | 0.206 | 0.100 | 0.016 |
| copper ore | 0.250 | 0.172 | 0.486 | 0.263 | 0.220 | 0.138 | 0.240 | 0.196 | 0.344 | 0.248 | 0.250 | 1.000 | 0.900 | 0.664 | 0.201 | 0.319 | 0.196 | 0.302 | 0.344 | 0.076 |
| tin ore | 0.333 | 0.383 | 0.591 | 0.359 | 0.320 | 0.183 | 0.343 | 0.225 | 0.538 | 0.348 | 0.333 | 1.000 | 0.817 | 0.559 | 0.297 | 0.426 | 0.293 | 0.409 | 0.538 | 0.164 |
| plant fibre | 0.250 | 0.024 | 0.412 | 0.218 | 0.225 | 0.475 | 0.184 | 0.087 | 0.162 | 0.204 | 0.250 | 1.000 | 0.900 | 0.738 | 0.135 | 0.216 | 0.123 | 0.196 | 0.162 | 0.008 |
| hide | 0.500 | 0.091 | 0.445 | 0.490 | 0.358 | 0.500 | 0.398 | 0.175 | 0.325 | 0.440 | 0.500 | 1.000 | 0.650 | 0.705 | 0.270 | 0.243 | 0.247 | 0.222 | 0.325 | 0.018 |
| bone | 0.417 | 0.422 | 0.611 | 0.393 | 0.338 | 0.229 | 0.404 | 0.146 | 0.445 | 0.390 | 0.417 | 1.000 | 0.733 | 0.539 | 0.423 | 0.520 | 0.424 | 0.511 | 0.445 | 0.281 |
| meat | 0.083 | 0.004 | 0.402 | 0.061 | 0.041 | 0.046 | 0.058 | 0.029 | 0.056 | 0.064 | 0.083 | 1.000 | 1.067 | 0.748 | 0.046 | 0.205 | 0.042 | 0.185 | 0.056 | 0.003 |
| wild plants | 0.083 | 0.009 | 0.405 | 0.073 | 0.046 | 0.121 | 0.061 | 0.029 | 0.073 | 0.068 | 0.083 | 1.000 | 1.067 | 0.745 | 0.052 | 0.212 | 0.049 | 0.193 | 0.073 | 0.008 |
| fish | 0.083 | 0.004 | 0.402 | 0.061 | 0.041 | 0.046 | 0.058 | 0.029 | 0.056 | 0.064 | 0.083 | 1.000 | 1.067 | 0.748 | 0.046 | 0.205 | 0.042 | 0.185 | 0.056 | 0.003 |
| charcoal | 0.083 | 0.018 | 0.409 | 0.082 | 0.057 | 0.046 | 0.066 | 0.029 | 0.100 | 0.073 | 0.083 | 1.000 | 1.067 | 0.741 | 0.062 | 0.225 | 0.060 | 0.206 | 0.100 | 0.016 |
| ceramic | 0.167 | 0.196 | 0.498 | 0.191 | 0.165 | 0.092 | 0.168 | 0.546 | 0.393 | 0.179 | 0.167 | 1.000 | 0.983 | 0.652 | 0.223 | 0.389 | 0.229 | 0.378 | 1.393 | 0.145 |
| copper | 1.000 | 0.460 | 0.630 | 0.645 | 0.666 | 0.413 | 0.697 | 0.442 | 0.570 | 0.756 | 0.750 | 0.595 | 0.150 | 0.310 | 0.740 | 0.555 | 0.714 | 0.547 | 0.570 | 0.324 |
| tin | 0.750 | 0.103 | 0.451 | 0.319 | 0.314 | 0.229 | 0.350 | 0.524 | 0.308 | 0.386 | 0.417 | 0.640 | 0.400 | 0.447 | 0.450 | 0.301 | 0.420 | 0.286 | 0.308 | 0.069 |
| bronze | 1.000 | 0.783 | 0.791 | 0.744 | 0.733 | 0.413 | 0.748 | 0.455 | 0.680 | 0.797 | 0.750 | 0.730 | 0.150 | 0.262 | 0.825 | 0.721 | 0.808 | 0.714 | 0.680 | 0.565 |
| grain | 0.083 | 0.018 | 0.409 | 0.082 | 0.057 | 0.046 | 0.066 | 0.029 | 0.100 | 0.073 | 0.083 | 1.000 | 1.067 | 0.741 | 0.062 | 0.225 | 0.060 | 0.206 | 0.100 | 0.016 |
| flax | 0.375 | 0.034 | 0.417 | 0.326 | 0.355 | 0.562 | 0.276 | 0.131 | 0.230 | 0.306 | 0.375 | 1.000 | 0.775 | 0.733 | 0.198 | 0.219 | 0.180 | 0.199 | 0.230 | 0.008 |
| wool | 0.333 | 0.019 | 0.409 | 0.263 | 0.267 | 0.483 | 0.236 | 0.117 | 0.196 | 0.261 | 0.333 | 1.000 | 0.817 | 0.741 | 0.173 | 0.211 | 0.157 | 0.191 | 0.196 | 0.005 |
| iron ore | 0.333 | 0.294 | 0.547 | 0.355 | 0.308 | 0.183 | 0.333 | 0.225 | 0.455 | 0.340 | 0.333 | 1.000 | 0.817 | 0.603 | 0.267 | 0.374 | 0.260 | 0.356 | 0.455 | 0.115 |
| iron | 1.000 | 0.804 | 0.802 | 0.754 | 0.744 | 0.413 | 0.757 | 0.399 | 0.707 | 0.804 | 0.750 | 0.730 | 0.150 | 0.254 | 0.820 | 0.721 | 0.802 | 0.713 | 0.707 | 0.565 |
| brine | 0.050 | 0.000 | 0.400 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 1.100 | 0.750 | 0.025 | 0.200 | 0.022 | 0.180 | 0.000 | 0.000 |
| salt | 0.083 | 0.046 | 0.423 | 0.091 | 0.071 | 0.046 | 0.075 | 0.029 | 0.183 | 0.082 | 0.083 | 1.000 | 1.067 | 0.727 | 0.092 | 0.262 | 0.093 | 0.245 | 0.183 | 0.043 |
| pitch | 0.167 | 0.016 | 0.408 | 0.145 | 0.096 | 0.092 | 0.123 | 0.058 | 0.117 | 0.136 | 0.167 | 1.000 | 0.983 | 0.742 | 0.093 | 0.214 | 0.086 | 0.195 | 0.117 | 0.008 |
| steel | 0.979 | 0.967 | 0.884 | 0.769 | 0.754 | 0.413 | 0.765 | 0.407 | 0.735 | 0.810 | 0.750 | 0.752 | 0.171 | 0.200 | 0.880 | 0.832 | 0.870 | 0.827 | 0.735 | 0.765 |
| gold | 0.833 | 0.125 | 0.463 | 0.340 | 0.356 | 0.229 | 0.377 | 0.444 | 0.363 | 0.411 | 0.417 | 0.550 | 0.317 | 0.378 | 0.497 | 0.311 | 0.463 | 0.296 | 0.363 | 0.076 |
| resinous wood | 0.333 | 0.124 | 0.462 | 0.356 | 0.270 | 0.183 | 0.292 | 0.117 | 0.290 | 0.320 | 0.333 | 1.000 | 0.817 | 0.688 | 0.207 | 0.271 | 0.194 | 0.252 | 0.290 | 0.038 |

Brine's old 0.050 is the floor. Its raw edge life is 0 because every input is 0, not because of a floor. The compressed edge column pulls that 0 toward the anchor, so the edge column reads 0.400. Nothing in the sim makes a brine knife; the column is the formula. Copper, bronze and iron sharing 1.000 is the cap. They do not share an edge life. They do share cordage life (0.413) and stock life (0.750), because all four tool metals have toughness 9 and no fibrousness. That is the same inputs, and a metal cord is a poor cord next to hide (0.500) or flax (0.562).

The same three spreads, life / wear / axe term / working-edge quality, for the tool edges. `workQ` is identical down the three blocks.

| Spread | Material | life | wear | axe | workQ |
| --- | --- | ---: | ---: | ---: | ---: |
| 1.00 | obsidian | 0.140 | 1.010 | 0.480 | 0.397 |
| 1.00 | flint | 0.342 | 0.808 | 0.551 | 0.460 |
| 1.00 | stone | 0.659 | 0.491 | 0.540 | 0.334 |
| 1.00 | copper | 0.460 | 0.411 | 0.470 | 0.324 |
| 1.00 | bronze | 0.783 | 0.268 | 0.716 | 0.565 |
| 1.00 | iron | 0.804 | 0.252 | 0.722 | 0.565 |
| 1.00 | steel | 0.967 | 0.138 | 0.874 | 0.765 |
| 0.50 | obsidian | 0.470 | 0.680 | 0.645 | 0.397 |
| 0.50 | flint | 0.571 | 0.579 | 0.666 | 0.460 |
| 0.50 | stone | 0.730 | 0.420 | 0.575 | 0.334 |
| 0.50 | copper | 0.630 | 0.310 | 0.555 | 0.324 |
| 0.50 | bronze | 0.791 | 0.262 | 0.721 | 0.565 |
| 0.50 | iron | 0.802 | 0.254 | 0.721 | 0.565 |
| 0.50 | steel | 0.884 | 0.200 | 0.832 | 0.765 |
| 0.35 | obsidian | 0.569 | 0.581 | 0.694 | 0.397 |
| 0.35 | flint | 0.640 | 0.510 | 0.700 | 0.460 |
| 0.35 | stone | 0.751 | 0.399 | 0.585 | 0.334 |
| 0.35 | copper | 0.681 | 0.279 | 0.580 | 0.324 |
| 0.35 | bronze | 0.794 | 0.260 | 0.722 | 0.565 |
| 0.35 | iron | 0.802 | 0.254 | 0.721 | 0.565 |
| 0.35 | steel | 0.859 | 0.219 | 0.819 | 0.765 |

At spread 0.35 the bronze axe term (0.722) is above iron (0.721). That is why 0.35 is not the constant in the source, even though it keeps more copper axes on the save.

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

`diag_durability_flips`, feature on, scores every payoff slot at today's formula and at spreads 1.00, 0.50 and 0.35. The live model (spread 0.50) matched `slot_score_m` on 0 mismatches of 103022. The kit after load matched the live pick on 0 mismatches of 31774: loading the save with the feature on already refits the kit. The v1 column is that rescore, not a second load. It reproduces the axe census the feature-off build holds. Nets, marks and traps do not read edge life, so their flips are the same at every spread. The feature-off binary prints the v1 census only and says the v2 tables are not scored there.

Axe argmax, all bands. 51 bands have no axe they can make. 3234 can make steel. 3303 have some axe.

| | copper | iron | steel | stone | band-slots that flip |
| --- | ---: | ---: | ---: | ---: | ---: |
| v1, today's formula | 806 | 264 | 2209 | 24 | |
| spread 1.00, raw life | 16 | 27 | 3234 | 26 | 4766 |
| spread 0.50, this proposal | 113 | 15 | 3150 | 25 | 4550 |
| spread 0.35, comparison only | 331 | 16 | 2926 | 30 | 3862 |

At spread 1.00 the 16 copper, 27 iron and 26 stone are the 69 bands that cannot make steel. Every steel-capable band holds steel. At spread 0.50, 81 of the copper axes and 3 of the iron axes are held by bands that can make steel (81 + 3 + 3150 = 3234). The other 32 copper, 12 iron and 25 stone are the same 69. At spread 0.35, 308 steel-capable bands hold something else (3234 − 2926). Compression does not stop the swing. It restores a minority.

**Spread 0.50, the proposal.** Farm, then forage. This is the table to cite.

| Slot | From | To | Farm | Forage |
| --- | --- | ---: | ---: | ---: |
| axe | copper axe | iron axe | 0 | 3 |
| axe | copper axe | steel axe | 90 | 599 |
| axe | copper axe | stone axe | 0 | 1 |
| axe | iron axe | steel axe | 81 | 171 |
| cut | copper knife | stone blade | 1 | 0 |
| cut | flint blade | stone blade | 2 | 1 |
| cut | obsidian blade | copper knife | 0 | 1 |
| cut | obsidian blade | stone blade | 0 | 1 |
| hunt | bone point | stone blade | 0 | 1 |
| hunt | flint point | copper point | 0 | 5 |
| mark | tally stick | notched bone tally | 35 | 73 |
| net | hide net | flax net | 164 | 57 |
| net | hide net | plant fibre net | 67 | 342 |
| net | plant fibre net | flax net | 39 | 7 |
| point | bone point | flint point | 0 | 1 |
| point | bone point | steel point | 2 | 4 |
| point | copper point | iron point | 0 | 3 |
| point | copper point | steel point | 1065 | 658 |
| point | iron point | steel point | 819 | 196 |
| trap | plant fibre snare | flax snare | 5 | 0 |
| trap | rawhide snare | flax snare | 0 | 1 |
| trap | rawhide snare | plant fibre snare | 0 | 55 |

689 copper axes go to steel (90 farms, 599 foragers). 252 iron axes go to steel. One forager's copper axe goes to stone. Three foragers' copper axes go to iron. No iron knife flips, and no obsidian knife takes an iron knife. The depth-0 case that did that in the first pass is covered below.

**Spread 1.00**, the same pairs, for the choice. Rows that match 0.50 are not repeated: marks, nets and traps are identical. What changes is the edge slots.

| Slot | From | To | Farm | Forage |
| --- | --- | ---: | ---: | ---: |
| axe | copper axe | iron axe | 0 | 16 |
| axe | copper axe | steel axe | 90 | 682 |
| axe | copper axe | stone axe | 1 | 1 |
| axe | iron axe | steel axe | 81 | 172 |
| cut | copper knife | stone blade | 1 | 0 |
| cut | flint blade | copper knife | 0 | 9 |
| cut | flint blade | stone blade | 2 | 1 |
| cut | obsidian blade | copper knife | 0 | 1 |
| cut | obsidian blade | stone blade | 0 | 1 |
| hunt | bone point | stone blade | 0 | 4 |
| hunt | flint point | copper point | 0 | 6 |
| hunt | flint point | stone blade | 0 | 1 |
| point | bone point | flint point | 0 | 1 |
| point | bone point | steel point | 2 | 4 |
| point | copper point | steel point | 1115 | 715 |
| point | iron point | steel point | 819 | 196 |

772 copper axes go to steel, which is the count #11 reported as copper holders among steel-capable bands, and the rest of the copper axes leave too except the 16 who cannot make steel. Nine foragers' flint knives go to copper, which does not happen at spread 0.50: the working-edge food is the same, and the raw wear is what pushes those nine. Still no obsidian-over-iron knife.

**Spread 0.35**, comparison only. Marks, nets and traps again identical. Edge rows:

| Slot | From | To | Farm | Forage |
| --- | --- | ---: | ---: | ---: |
| axe | copper axe | iron axe | 0 | 1 |
| axe | copper axe | steel axe | 88 | 382 |
| axe | copper axe | stone axe | 0 | 4 |
| axe | iron axe | steel axe | 81 | 166 |
| axe | iron axe | stone axe | 0 | 2 |
| cut | copper knife | flint blade | 0 | 2 |
| cut | copper knife | stone blade | 1 | 0 |
| cut | flint blade | stone blade | 1 | 0 |
| cut | obsidian blade | copper knife | 0 | 1 |
| cut | obsidian blade | stone blade | 0 | 1 |
| hunt | flint point | copper point | 0 | 3 |
| point | bone point | flint point | 0 | 1 |
| point | bone point | steel point | 2 | 4 |
| point | copper point | bone point | 0 | 1 |
| point | copper point | iron point | 0 | 2 |
| point | copper point | steel point | 832 | 435 |
| point | iron point | steel point | 814 | 193 |

More copper axes survive (331 held, against 113 at spread 0.50 and 16 at spread 1). Two foragers drop an iron axe to stone, and two take a flint knife over copper. Together with the bronze axe term sitting above iron, that is why 0.35 is not the default.

Knives on the save, at the recommended spread, barely move (six bands). The nominal test is the one that used to reverse. `d3_metal_blades_outrank_stone_when_practised` builds a kit at chain depth 0. An iron knife beats an obsidian knife on both builds. A copper knife beats an obsidian knife only with the feature on; the default build still keeps the obsidian knife, because fresh `cut_q` is what it pays and the upkeep rate (0.02) does not charge the chip. A copper point beats an obsidian point on both builds. A flint knife beats a copper knife on both. A flint point beats a copper point only with the feature on. `working_edge_orders_knife_axe_and_point` is a normal (not ignored) test, compiled only with the feature. At depth 0, nominal weights, it requires the priced order steel, iron, bronze, flint, copper, obsidian for the knife and the point, and steel, iron, bronze, copper, stone, flint, obsidian for the axe. It also requires an iron spear over an obsidian spear and a steel spear over an iron spear.

Points now have a material term, and steel still takes most of them. At spread 0.50, 1723 copper points and 1015 iron points go to steel, plus 6 bone points, and 3 copper points go to iron instead. The old "2841 flips on wear alone" was 1828 copper and 1013 iron at a scoring pass that ignored the edge. The new count is not wear alone: a steel point's working edge is 0.765 against copper 0.324 and iron 0.565, so the food agrees with the wear. What did change, and is easy to miss, is the scale. Today's point multiplies fish by 1. A steel point now multiplies it by 0.765, a copper point by 0.324. Every band's fish yield shrinks, including bands that were already on steel. Renormalizing by steel's quality would put steel back at 1 and copper at about 0.42. That is not what the code does. See the questions.

The bow is a separate hole, from `docs/AUDIT-same-class.md` in the audit tree (finding 7). `fight_of` slot 14 adds a flat 0.3 when any point exists and 0 when it does not. The arrow's material never enters the fight. The audit's proposed fix is `0.3 * edge_q` of the point. Not applied here. The point's food term did change. The bow's fight term did not.

Nets. Flax cord wear is 0.587, hide 0.650, plant fibre 0.675. Flax also has the better mesh (`(tough + fibrous) / 15` is 0.933 for flax, 0.800 for hide and for fibre). Hide to flax is a real cordage win (164 farms, 57 foragers). Hide and fibre have the same mesh term, and fibre wears slightly faster, so a fibre net beats a hide net only through a shallower chain or a higher recipe quality. This run did not print those depths. The count is large for foragers (342). Do not read it as "fibre lasts longer than hide".

Marks. Record life weights hardness, so bone (0.445) beats wood (0.380). The old number was mostly toughness, so the wooden tally won. 108 bands switch. A fired clay token still wins when it can be made, because of the +1.

## What this does to the steel-axe question in #11

#11, re-run here as `diag_axe_breakeven` on the same save. Attribution of that diagnostic is in `civ-core/src/tests_diag_axe.rs`: it was copied from draft PR #11. Among 3234 steel-capable bands (population 65734).

Feature off (model v1), re-run this pass. The margins match the earlier measurement. Nothing in the default build moved:

- Held axes: copper 772, iron 253, steel 2209. Bronze 0.
- Axe terms: copper 0.740, bronze 0.825, iron 0.820, steel 0.880. Durability 1.000 / 1.000 / 1.000 / 0.979.
- Every copper holder keeps copper. All 772, rank 772 / 0 / 0, margin `cur = +0.0453` (positive means the other metal beats steel). Farms, n=90, `cur = +0.0220`. Foragers, n=682, `cur = +0.0484`.
- Every iron holder keeps iron. n=253, `cur = +0.0175`. Farms n=81, `cur = +0.0147`. Foragers n=172, `cur = +0.0188`.
- Bands already on steel prefer steel against the best alternative. n=2208, `cur = -0.1337`.
- `mis%` is 0 everywhere: the printed margin matches food minus priced labour.

That is #11's picture. A durability tweak that only made steel last a little longer than the cap would move the farmers (their margin is +0.022, close to a tie) and would not move the foragers (margin +0.048, and their food gain from a better axe is small). #11 said so, and it was right about this formula.

Feature on, spread 0.50, the same diagnostic (model v2), re-run this pass:

- Axe terms: copper 0.555, bronze 0.721, iron 0.721, steel 0.832. Compressed edge life 0.630 / 0.791 / 0.802 / 0.884. Cut 0.480 / 0.650 / 0.640 / 0.780.
- Held axes among the 3234 who can make steel: copper 81, iron 3, steel 3150. Bronze 0. All 81 copper holders are foragers.
- Those 81 keep copper. Rank 81 / 0 / 0, `cur = +0.0136`. Food gain of steel over copper is +0.0295. The priced labour gap is 0.0430, so copper still wins, by a thin margin. `cur` at maker skill 1 is still +0.0101. The probe's own value prefers steel (preference 100%, true value +0.085) and the same probe at the band's plant and farm weights does not (re +0.0021, re% 0). `mis%` is 0.
- The 3 iron holders keep iron. All foragers. `cur = +0.0322`, rank 3 / 0 / 0. Probe preference 66.7%.
- Bands already on steel prefer steel against the best other axe. n=3149 (one steel band has no alternative), `cur = -0.1912`, rank 0 / 3149 / 0. Food gain about +0.142. Farms, n=2288, `cur = -0.2376`. Foragers, n=861, `cur = -0.0679`. The forager margin is the one compression actually shrank. It is not a tie, and it is no longer the uncompressed −0.16.
- `mis%` is 0 on these rows.

Read against the raw formula, which this pass did not re-break-even (the flip census is the evidence): at spread 1.00 the axe terms are copper 0.470, bronze 0.716, iron 0.722, steel 0.874, and the census puts a steel axe on all 3234 steel-capable bands. The first draft of this note reported the matching break-even, `cur` about −0.32 overall, −0.39 on farms and −0.16 on foragers. Spread 0.50 is the smaller version of that result, not a different story. 81 foragers who can make steel still price copper ahead, by 0.014. Everyone else who can make it prices steel ahead, farms by 0.24 and the foragers who already switched by 0.07.

Bronze still does not appear in the held census. Its axe term sits on iron's (0.721 and 0.721 at three decimals; iron higher before rounding, and iron wears 0.254 against bronze 0.262). Availability and chain depth, not the cap, are why this save has no bronze axes. At spread 0.35 that near-tie flips the wrong way, which is the reason not to recommend 0.35.

## Risks, and what to re-check

This is still a large balance change, not a 0.979-versus-1.0 rounding fix. At the recommended spread, copper's axe score falls from 0.740 to 0.555 (it was 0.470 before compression). Its edge wear rises from 0.150 to 0.310 (it was 0.411). Most steel-capable bands still leave copper. Not all of them.

- **Population.** Bands that stay on copper pay more labour for the edge (0.310 against today's 0.150). Bands that move to steel do not get a cheaper edge than today: steel wear goes from 0.171 to 0.200, because compression pulls steel down toward the anchor and the repair factor does not give it back. They do get a better axe score relative to copper (0.832 against 0.555, where today is 0.880 against 0.740), and the break-even says that food gain wins for 3150 of 3234 steel-capable bands. It does not say what population does over 300 years. Not run locally. See below.
- **Chain depth.** Steel's depth penalty matters less once copper is this expensive. Bands have a stronger reason to climb to steel, and to climb sooner. Guard rails that watch the year steel appears, and the share of bands on it, will move. That is the point of the change, and it is also how a bad constant would show up. Spread 0.50 leaves a thin set of foragers on copper. A guard rail that expects zero copper among steel-capable bands will fail, and that failure is the compression working.
- **Points.** The slot now prices `working_edge_q`. Steel still takes 2744 point slots at spread 0.50 (1723 from copper, 1015 from iron, 6 from bone), because the working edge and the wear agree. The new risk is the scale: fish is multiplied by 0.765 for a steel point and by 0.324 for a copper point, where today every point multiplies by 1. That is a yield change, not only a ranking change.
- **Knives.** The obsidian-over-iron reversal at depth 0 is fixed, on the food term, and the test expects the iron knife with the feature on. Flint over copper is kept, for the reason in the model section. The two comparisons that still differ by flag are the ones that should: the default build keeps an obsidian knife over copper and a copper point over flint; the feature does the opposite.
- **The bow.** Fight pays 0.3 for any point and nothing for a better point. Mentioned so it is not mistaken for a fixed hole. Not changed in this proposal. The audit's line is `0.3 * edge_q(mat(kit.point))`.
- **Cordage.** Flax replaces a lot of hide, mostly on farms (164 against 57). Fibre replaces hide on foragers (342) even though fibre wears slightly faster. There is no rot, so plant cordage never gets worse in the wet. A rot term would push back toward hide and would be a new property. Not in this proposal.
- **Stone above copper, on life.** One forager's copper axe goes to stone at spread 0.50. Stone's compressed life (0.730) beats copper (0.630). Fine for a ground celt, generous for a flake. The priced axe usually still prefers copper, which is why the flip is one band and not the life order. One edge class cannot say both.
- **Negative wear.** No clamp. Current props stay under 1.15. A richer material later could make upkeep negative. Compression makes a zero raw life into 0.400, so the edge column for a non-tool (brine) is no longer zero. Wear stays positive.
- **Fight.** `edge_q` and the axe fight term move with compressed edge life. Copper's `edge_q` falls from 0.714 to 0.547. Steel's falls from 0.870 to 0.827. Smaller than the uncompressed drop (copper was going to 0.471). `tests.rs` still asserts that steel weapons raise fight strength by less than 1.1 times against an iron-and-bronze kit. That assertion is about iron and bronze, not copper, and those scores stay close. Do not assume a copper band's fight number is stable.
- **Golden hashes.** Feature off, they must stay `0x1845e2690330d59` (the long run and the first of the v10 pair) and `0x5cef0617c34457c6` (the run-on). The asserts are `#[cfg(not(feature = "durability-v2"))]`, so a feature-on run does not fail them and also does not record the new hash. This pass: `cargo test --release --workspace` passed (civ-core 113 tests, 10 ignored). `cargo test --release -p civ-core --features durability-v2` passed (113 tests, 10 ignored), including `working_edge_orders_knife_axe_and_point`. Record the feature-on hashes when the feature is actually tried. Do not treat a mismatch with the old constants as a failure of this proposal, and do not update the constants in the default build.

No local simulation was run (seed 5, 300 years, one thread). Batch runs are Build Bot's job. The useful pair is the default map, seed 5, 300 years, `--threads 1`, feature off and feature on: population, band count, and axe material counts at the end, plus the usual guard rails (population floor, year of steel, share of bands on steel, chain depth). The ignored diagnostics to re-run on `DIAG_SAVE`, both ways, are `diag_durability_table`, `diag_durability_flips` and `diag_axe_breakeven`.

## Saves

Durability is not saved. `save.rs` does not mention it. It is derived from the material.

The kit is saved. `after_load` (`world.rs:1112`) rebuilds toolkits and calls `refresh_toolkit` (`world.rs:1693`), which calls `fit_kit` (`world.rs:1686`) when the toolkit cache misses or the kit is marked dirty. `fit_kit` calls `make_kit_for_view`. Measured on this save: with the feature off, the loaded kit matches the default scorer and the held axes are the file's own (copper 772 among steel-capable bands, and the full-census v1 argmax is copper 806, iron 264, steel 2209, stone 24). With the feature on at spread 0.50, the same bytes load, the kit is re-chosen, and the steel-capable census is copper 81, iron 3, steel 3150. No format version change. A save written under one model loads under the other. The kit may change on load when the feature differs from the build that wrote the file. The bytes do not.

## How this lands relative to skills step 4

PR #12, branch `origin/skills-step4`, head `3521366`, is open and not merged. `git diff origin/main origin/skills-step4` on `effects.rs`, `band.rs` and `probe_diffusion.rs`:

Step 4 does not add a new call of the `durability` function. The old sites are still there, still written as `durability(...)` or `1.15 - durability(...)`:

- `band.rs:716` and `band.rs:742`, the axe fight term and `edge_q`. The diff does not touch these lines. It only changes the comment on the beliefs field.
- `effects.rs` on that branch: `tool_upkeep` (around line 449), `axe_term` (592), `cand_new` (740), the fight axe term (912), `apply_base` (940), `nonfood_scores` (1126), the mark (1557). Same expressions as main.
- `probe_diffusion.rs:161`, `KIT_UPKEEP * (1.15 - durability) * depth`, unchanged as an expression.

What step 4 does add is a new way of *reaching* that wear. Food slots, once the belief table is non-empty, are priced by `choose_food_belief` and `belief_slot_price`. Labour is the stored belief when the band has one. When it does not, labour is `tool_upkeep`, which is the durability wear. So a belief that has already learned a labour cost stops reading durability for that slot. A prior, and every band with an empty table, still reads it. Non-food slots stay on `slot_score_m` and `KIT_UPKEEP`. The probe's own-valuation (`slot_own`) now calls `belief_slot_price` instead of `slot_score_m`. Empty table, and every non-food slot, still `KIT_UPKEEP`.

That is the interaction. This proposal changes the number inside `tool_upkeep` and inside the `KIT_UPKEEP` wear factor. Step 4 decides *whether the food rank looks at that number or at a remembered labour*. It does not freeze the yield. Knife, hunt and point food change inside the yield (`food_edge_q`, `point_food_scale`), so a band with a stored labour belief still sees the new working edge. The flips in this note are the empty-table price. They are an upper bound on how fast the *labour* term moves axes once beliefs exist. They are not an upper bound on the yield change.

Expected conflicts if the two branches meet:

- `effects.rs` will conflict. Both edits sit in the kit choice. This branch replaces `durability(...)` with `score_durability`, `record_durability` and `upkeep_wear` at the sites listed above. Step 4 inserts `choose_food_belief`, `belief_slot_price` and `make_kit_beliefs` around `choose_food`. The new belief path already calls `tool_upkeep`, so once `tool_upkeep` uses `upkeep_wear` the belief fallback inherits the feature with no extra edit. The conflict is the surrounding functions, not a missed formula.
- `probe_diffusion.rs` will conflict. Step 4 rewrites `slot_own`, the imports, and the adoption counters. This branch changes line 159 from `1.15 - durability(mat(it))` to `upkeep_wear(it)`.
- `band.rs` should merge. Step 4's hunk is the beliefs comment. This branch's hunks are `weapon_q` and `edge_q`.

Recommendation. Land this proposal with the feature **off** either before or after step 4. The default build does not move, so it does not block step 4's hashes. Do not turn the feature on, and do not trust the flip table in this document as the step-4 answer, until both are measured together. A band with a stored labour belief will not re-rank a food slot when only the wear changes; the belief has replaced `tool_upkeep`. The yield change (working edge on the knife, the hunt and the point) still re-ranks those slots. The axe's food is half cut and half life, so its move is mostly the wear and the life, which a belief can freeze.

If this merges first, step 4's rebase must keep the helpers. Pasting `1.15 - durability(mat)` back into `nonfood_scores`, the probe, `cand_new` or `edge_q` would restore the cap on those paths while `tool_upkeep` stayed on the new model.

If step 4 merges first, re-apply the helpers at the sites above (the belief path needs nothing beyond `tool_upkeep`), then re-run `diag_durability_flips`, `diag_axe_breakeven`, and the step-4 "believed wrong" share. The food-slot price will not be the one in this document.

## Questions for Michael

1. Are the constants acceptable? Edge keep is `0.65 * hard + 0.35 * sharp`, roll is `0.09 * malleable`, chip discount is `0.50 * fracture / 10`, the scale is `/ 45`, the pitch-versus-clay split is `0.004 * hard`, repair is `0.045 * malleable`, and the working-edge floor is `0.40`. They were chosen to hit the order above and to avoid a shared cap. They are not fitted to a dataset.
2. Which edge-life spread? The sim currently uses **0.50**, and that is my recommendation. Spread 1.00 is the raw formula: copper wear about 3 times steel, and all 3234 steel-capable bands take a steel axe (4766 band-slots flip). Spread 0.50: copper wear about 1.55 times steel, 3150 of those bands take steel, 81 keep copper, 3 keep iron (4550 band-slots flip). Spread 0.35 keeps more copper (331 axes, 3862 band-slots flip) but puts the bronze axe term just above iron, and two foragers drop an iron axe to stone. I would not go to 0.35 to save copper if it costs the iron-over-bronze order. 0.50 keeps the order and still abandons most copper. If that remaining swing is too big, the next knob is a higher anchor or a still gentler spread that is checked against the bronze/iron axe term, not 0.35 as printed.
3. Should a steel point's fish factor stay at `working_edge_q` (0.765), or be renormalized so a steel point matches today's flat 1 and poorer points are a fraction of that? The code does the first. Ranking is the part that was wrong. Shrinking everyone's fish is an extra swing. I would renormalize if the fish drop is unwanted, and I have not done it, so the flip table above is the un-normalized one.
4. Stone's edge life beats copper (0.730 against 0.630). One class covers a ground axe and a flake knife. Split them, or leave it? The priced axe still prefers copper in almost every band.
5. There is no rot. Flax nets beat hide. Add a property later, or accept it?
6. Steel's hardness (6 against iron's 5.5) still understates edge retention. Leave `PROPS` alone, as this proposal does?
7. Ship the feature-off code before step 4, or rebase onto step 4 first? Either way the feature should stay off until the flip table is re-run with beliefs. My recommendation is: merge the off code whenever it is convenient, and do not enable it until that re-measure. Beliefs freeze labour, not the new knife and point yields.
8. Wear has no floor. If a later material's life exceeds 1.15, upkeep goes negative. Floor it at 0?
9. The `kindling` crate forwards the feature (`durability-v2 = ["civ-core/durability-v2"]`) even though `civ-core` is a target-specific dependency. The native build accepts it. Say if you would rather the flag live only on `civ-core`.
10. The bow's fight term is still a flat 0.3 whenever any point exists (`fight_of` slot 14). The audit suggests `0.3 * edge_q` of the point. Left unchanged. Say if that should be in this feature or left for the audit.

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

`DIAG_SAVE` selects the snapshot. One feature-on `diag_durability_flips` prints all three spreads. The feature-off flips run prints the v1 axe census and does not pretend to score v2. The ordering tests `durability_v2_orders_edges_and_v1_formula_stays` and, with the feature on, `working_edge_orders_knife_axe_and_point` are not ignored and run in the normal suite.
