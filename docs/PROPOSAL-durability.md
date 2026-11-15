# Proposal: material-driven tool durability and wear

This is a proposal. The default build does not use it. Cargo feature `durability-v2` is off unless you ask for it, and with it off the simulation is unchanged (the golden hashes stay `0x1845e2690330d59` and `0x5cef0617c34457c6`). Nothing here is a retune of the live game. Michael decides whether the feature is ever turned on.

**Answer on re-sharpening.** Model it as its own upkeep. Do not fold it into one durability number.

Soft copper is a poor axe next to iron because the edge rolls. The tool spends the season dull, and someone has to hammer it sharp again and again. Snapping is a different bill, paid by flint and obsidian. One wear number cannot say which bill it is. The split account can. It is what the feature-on sim uses (`WEAR_ACCOUNT = Split`). The combined account (one life per class, malleability only as a repair discount, edge life pulled toward wrought iron) stays in the feature so the two can be scored on the same save. It is not the recommendation.

Malleability does not add to a working edge, and the 1.0 cap is not in the new formulas. Confirmed in the code, not only in this note. `edge_retention`, `split_working_cut` and `split_axe_score` never read `malleable`. Retention is hardness and acuity, squared, so a soft rank dulls sooner than a straight reading of it. Toughness, after the knap discount, is breakage. `resharpen_cost` and `repair_factor` (`1 - 0.045 * malleable`) are the only places malleability helps, and they make the hone cheaper. They do not make the edge last longer. Gold is more malleable than copper and still retains less. The old `durability` clamp at 1.0 remains only on the feature-off path, and inside a diagnostic that forces the old formula. Class life has no cap.

Measured on the year-350 seed-6 save (3354 bands, 3234 of them able to make steel). Feature on, both accounts, and once feature off. The tables are in "Resharpening as its own cost".

Copper against iron, split. Fresh cut 0.480 against 0.640, so a newly sharp copper axe is still about three quarters of an iron one. Average working cut 0.073 against 0.215: copper keeps about a third of iron's cut, and 15% of its own fresh cut. Resharpen labour 0.254 against 0.141 (1.8 times iron, and 2.4 times steel's 0.106). Break labour 0.060 for both, because every tool metal is toughness 9. Copper's bill is the hone. Obsidian's is the opposite: break labour 0.555, resharpen labour 0.140. That is the shape you described.

It is not "nearly useless" in the food number the plants and farm lines actually multiply. The split axe score is 0.486 for copper, 0.557 for iron, 0.629 for steel. The farm term (`0.2 * axe`) moves from 0.097 to 0.111. Half of the axe score is toughness, and toughness does not differ, and 0.1 and 0.2 then shrink what is left. Of the 3234 bands that can make steel, 3197 take a steel axe, 29 foragers still price copper ahead by 0.004, and 8 foragers still price iron ahead. Those 29 are a short copper chain against a long steel chain, not a claim that copper cuts like iron.

Combined, spread 0.50, same save. There is no hone line. Copper's working cut is 0.324 against iron's 0.565 (57% of iron, not a third) and its wear is 0.310 against steel's 0.200 (1.55 times, one blended number). 81 foragers who can make steel keep copper, by 0.014. Spread 1 makes copper's wear about three times steel's and puts steel on all 3234, and it still does not say that the cost is a hone.

I recommend the split account. The reasons, the ratio check against felling trials and hardness, and why I no longer recommend compressing edge life, are below. `EDGE_LIFE_SPREAD` stays 0.50 so the combined comparison does not move. The feature-on sim does not read it.

The knife reversal stays fixed under both accounts. At chain depth 0 a priced iron, bronze or steel knife beats obsidian, a flint knife beats copper, and a copper knife beats obsidian. `working_edge_orders_knife_axe_and_point` runs that under both accounts. `split_model_orders_retention_break_and_price` locks the split account on its own: copper's hone above iron's, copper's working cut well below its fresh cut and below iron's, a knapped edge's cost mostly replacement, malleability absent from the interval. Both are ordinary tests, compiled only with the feature.

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

The raw formula (spread 1) produces obsidian 0.140, flint 0.342, copper 0.460, stone 0.659, bronze 0.783, iron 0.804, steel 0.967. The combined account does not charge those. It pulls them toward 0.80 by half (spread 0.50): obsidian 0.470, flint 0.571, copper 0.630, stone 0.730, bronze 0.791, iron 0.802, steel 0.884. Same order. Smaller gaps. Stone stays above copper on life, below bronze. The feature-on sim does not charge this life either. An edge pays the split account.

## The model

Feature `durability-v2` on `civ-core` (default off; `native/Cargo.toml` forwards it onto the `kindling` binary). With the feature off, `score_durability`, `record_durability` and `upkeep_wear` are bit-identical to the old formula. A unit test checks that for every material and every form (`durability_v2_orders_edges_and_v1_formula_stays`).

The formulas in the rest of this section are the combined account. The feature-on sim does not use them for an edge. It uses the split account in "Resharpening as its own cost". Non-edge forms (cord, hull, pot, mark, shield, and the rest) use these class formulas either way. The spread applies only to the combined edge.

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

**Compression** (`scale_edge_life`, `EDGE_LIFE_ANCHOR` 0.80, `EDGE_LIFE_SPREAD` 0.50). The anchor is raw wrought iron, so iron and bronze barely move. Spread 1 returns the raw life bit for bit. Any other spread is `anchor + spread * (raw - anchor)`. Order is unchanged for a positive spread. Only the edge class is compressed. A cord, a hull, a pot, a mark and a shield use the formulas below with no spread. On the combined account, `class_durability` of an edge, and therefore `score_durability`, axe food, `edge_q` and edge upkeep, all read the compressed life. The split account does not. Diagnostics can score spread 1.0 and 0.35 without editing the constant. 0.35 is printed so it can be chosen for the combined account; it is not a setting I recommend, because at 0.35 the bronze axe term (0.722) sits just above iron (0.721).

**Working-edge food** (`working_edge_q`). Fresh keenness is `cut_q`. A season of use is not a fresh edge:

```text
working_edge_q = cut_q * (0.40 + 0.60 * raw_edge_life)
```

The 0.40 is `EDGE_KEEP_FLOOR`: a hopelessly short edge still keeps 40% of its keenness, and a life of 1 keeps all of it. The life in this product is the raw life, not the compressed one. Compressing it as well would let obsidian outrank iron again, which is the reversal this term exists to stop. Feature off, `food_edge_q` is `cut_q`, the same bits as before. Feature on, combined, it is `working_edge_q`. Feature on, split, it is `split_working_cut`. A diagnostic can force the old model.

Measured working-edge quality, which does not move when the spread changes: steel 0.765, bronze 0.565, iron 0.565, flint 0.460, obsidian 0.397, stone 0.334, copper 0.324, bone 0.281. Bronze is a hair above iron before rounding (0.5654 against 0.5647) because bronze is keener and the life gap is small. The priced knife at depth 0 still ranks iron just ahead of bronze, because iron wears less.

Where it is paid. The knife (`cut_term`) and the hunt weapon (`hunt_q_of`: spear, point, or a held edge) use `food_edge_q` instead of `cut_q`. The point slot's fish term used to be 1 whenever any point existed. Feature on, `point_food_scale` is `working_edge_q` on both accounts, not `food_edge_q`: steel 0.765, copper 0.324. A v1 diagnostic keeps the flat 1. Putting the point slot on `split_working_cut` (steel 0.359, copper 0.073) was tried and rejected: the fish weight is only 0.3, flint's break labour then wins, and the priced flint point loses to copper (−0.058 against −0.048). The flint-over-copper point order, and `d3_metal_blades_outrank_stone_when_practised`, need the larger factor. Neither account renormalizes. Whether to divide by steel's quality so the best point matches today's 1.0 is a question, not a silent second constant. The combined axe stays `0.5 * cut_q + 0.5 * compressed life`. The split axe is `split_axe_score` (half the dull-time cut, half toughness). Fight stays on `edge_q` and on the axe blend. Food does not call `edge_q`.

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

Working-edge wear after repair, on the combined account at spread 0.50 (not what the feature-on sim charges; the split wear is in the later section): steel 0.200, iron 0.254, bronze 0.262, copper 0.310, stone 0.420, flint 0.579, obsidian 0.680. Steel wears less than iron, iron a bit less than bronze, bronze less than copper, copper less than stone, stone less than flint, flint less than obsidian. Repair did not put copper back in front of steel. Easy to re-sharpen is not the same as cheap to keep using: copper's repair factor is the best of the tool metals (0.595 against steel's 0.752) and the wear is still about 1.55 times steel's. At spread 1.0 the same wear is steel 0.138, iron 0.252, bronze 0.268, copper 0.411, and copper is about three times steel. Compression is what cuts that from three times to one and a half. It also moves steel's own wear from today's 0.171 up to 0.200. Steel is not cheaper to keep than it is today. Copper is more expensive (0.150 today, 0.310 here), and the food gap is what moves the axe.

The old function `durability` is left in place. The sim does not call it when the feature is on. `tests.rs:1799` still asserts `durability(steel) >= 0.95 * durability(iron)` on that old function. That assertion is the bug-shaped comparison (0.979 against 1.0) and it still passes. It does not see this model. The new test checks the edge order.

### Where durability is used, and what the feature changes

`grep` for `durability(` on this branch: the old function remains the feature-off body. The sim's sites go through the three helpers. `save.rs` has no durability field.

| Site | What it is | Class the feature uses | What changes |
| --- | --- | --- | --- |
| `materials.rs:545` `durability` | The old scalar. Feature-off body of the helpers. | None | Unchanged. Still capped. |
| `materials.rs:701` `working_edge_q` | Keenness discounted by raw edge life. | Edge, raw life | Feature-on food only. Not wear, not fight. |
| `effects.rs:371` `tool_upkeep` | Economy labour for a worn tool: `sealed * rate * upkeep_wear * (1 + 0.3 * depth)`. Ornaments return 0. Fire and cooked return 0.01. | `wear_class` of that item | Feature on, wear is class life times repair. Edge life is the compressed one. `world.rs` sums `tool_upkeep` and does not call `durability` itself. |
| `effects.rs:499` `hunt_q_of` | Food of the hunt slot. | Working edge | Feature on, `food_edge_q` instead of `cut_q`. Feature off, still `cut_q`. |
| `effects.rs:512` `cut_term` | Food of the knife. | Working edge | Same switch as the hunt. |
| `effects.rs:526` `point_food_scale` | Fish factor of the point slot. | Working edge | Feature off, 1. Feature on, `working_edge_q` on both accounts (steel 0.765, copper 0.324), not the split cut. |
| `effects.rs:538` `axe_term` | Food ranking of an axe: half `cut_q`, half durability. | Edge, compressed | Copper axe score 0.740 → 0.555. Bronze 0.825 → 0.721. Iron 0.820 → 0.721. Steel 0.880 → 0.832. At three decimals bronze and iron tie; iron is higher before rounding. |
| `effects.rs:695` `cand_new` | One candidate in the kit choice. `dur` is the axe arm only. `dur_term` is `upkeep_wear` for every candidate. The point's food term is `point_food_scale`. | Edge for the axe score; the form's class for labour | Same axe scores as the row above. Labour follows the class. |
| `effects.rs:861` `fight_of` slot 4 | Fight term of the axe, `0.3 *` the axe score. Not food. | Edge | The fight number moves with the axe score. |
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

Save: `DIAG_SAVE`, which in this run was `/workspace/bots/build-bot/step3-saves/main_seed6_y350_256x160.civsave`. Format v16, year 350, 3354 bands, population 67836. Not a user save. 2308 bands count as farming and 1046 as foraging. A band is a farm when the grain share of its harvest is at least 0.10, or `sow >= 0.2`. That is the same split the axe diagnostic uses. The tables below are the combined account. The split account, which is the one the feature-on sim uses, is measured in "Resharpening as its own cost". Its axe census is copper 56, iron 26, steel 3197, stone 24.

`diag_durability_flips`, feature on, scores every payoff slot at today's formula, at combined spreads 1.00, 0.50 and 0.35, and at the split account. This pass the live model is split. It matched `slot_score_m` on 0 mismatches of 103022. The kit after load matched the live pick on 0 mismatches of 31774: loading the save with the feature on already refits the kit to split. The v1 column is that rescore, not a second load. It reproduces the axe census the feature-off build holds (re-run feature off this pass: the same 806 / 264 / 2209 / 24, and the same 0 mismatches). Nets, marks and traps do not read the edge account, so their flips are the same in every column. The feature-off binary prints the v1 census only.

Axe argmax, all bands. 51 bands have no axe they can make. 3234 can make steel. 3303 have some axe.

| | copper | iron | steel | stone | band-slots that flip |
| --- | ---: | ---: | ---: | ---: | ---: |
| v1, today's formula | 806 | 264 | 2209 | 24 | |
| spread 1.00, raw life | 16 | 27 | 3234 | 26 | 4780 |
| spread 0.50, combined | 113 | 15 | 3150 | 25 | 4563 |
| spread 0.35, comparison only | 331 | 16 | 2926 | 30 | 3876 |
| split, the recommendation | 56 | 26 | 3197 | 24 | 4752 |

At spread 1.00 the 16 copper, 27 iron and 26 stone are the 69 bands that cannot make steel. Every steel-capable band holds steel. At spread 0.50, 81 of the copper axes and 3 of the iron axes are held by bands that can make steel (81 + 3 + 3150 = 3234). The other 32 copper, 12 iron and 25 stone are the same 69. At spread 0.35, 308 steel-capable bands hold something else (3234 − 2926). Compression does not stop the swing. It restores a minority.

**Combined, spread 0.50.** Farm, then forage. The split table, which is the one to cite, is in "Resharpening as its own cost". The band-slot totals above are this pass. An earlier pass printed 4766, 4550 and 3862. The axe counts did not change. The kit was refit to split before the rescore, which moves a few non-axe slots.

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

Knives on the save, on the combined account at spread 0.50, barely move (six bands). The nominal test is the one that used to reverse. `d3_metal_blades_outrank_stone_when_practised` builds a kit at chain depth 0. An iron knife beats an obsidian knife on both builds. A copper knife beats an obsidian knife only with the feature on; the default build still keeps the obsidian knife, because fresh `cut_q` is what it pays and the upkeep rate (0.02) does not charge the chip. A copper point beats an obsidian point on both builds. A flint knife beats a copper knife on both. A flint point beats a copper point only with the feature on. `working_edge_orders_knife_axe_and_point` is a normal (not ignored) test, compiled only with the feature. At depth 0, nominal weights, it requires the priced order steel, iron, bronze, flint, copper, obsidian for the knife and the point, and steel, iron, bronze, copper, stone, flint, obsidian for the axe. It also requires an iron spear over an obsidian spear and a steel spear over an iron spear.

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

Feature on, split account, the same diagnostic reloaded under that account, this pass:

- Axe score: copper 0.486, bronze 0.554, iron 0.557, steel 0.629. Working cut 0.073 / 0.208 / 0.215 / 0.359. Hone labour 0.254 / 0.148 / 0.141 / 0.106. Break labour 0.060 for all four.
- Held axes among the 3234 who can make steel: copper 29, iron 8, steel 3197. Bronze 0. All 29 copper holders and all 8 iron holders are foragers.
- Those 29 keep copper. Rank 29 / 0 / 0, `cur = +0.0042`. Food gain of steel over copper is +0.0123. The priced labour gap is 0.0166. `cur` at maker skill 1 is +0.0016. The probe prefers steel (100%, true value +0.0435) and the same probe at the band's plant and farm weights does not (re +0.0018, re% 0). `mis%` is 0.
- The 8 iron holders keep iron. `cur = +0.0176`, food gain of steel +0.0136. Probe preference 62.5%.
- Bands already on steel prefer steel. n=3196, `cur = -0.1679` (farms -0.2112, foragers -0.0587). `mis%` is 0.

The split margin is thinner than the combined one (+0.004 against +0.014) because the food line barely moves, not because the hone is small. Copper's hone is 1.8 times iron's. The plants coefficient is what keeps those 29 foragers on copper.

## Risks, and what to re-check

This is still a large balance change, not a 0.979-versus-1.0 rounding fix. The recommended account is now the split one, not spread 0.50. Under split, copper's axe score falls from 0.740 to 0.486 and its edge wear rises from 0.150 to 0.314, of which 0.254 is the hone. Steel's wear falls slightly, from 0.171 to 0.166. Of 3234 steel-capable bands, 3197 take steel, 29 foragers keep copper and 8 keep iron. The bullets that quote 0.555, 0.200 and 3150 are the combined account at spread 0.50, kept so that comparison stays in the note. They are not the recommendation.

- **Population.** Bands that stay on copper pay more labour for the edge (0.310 against today's 0.150). Bands that move to steel do not get a cheaper edge than today: steel wear goes from 0.171 to 0.200, because compression pulls steel down toward the anchor and the repair factor does not give it back. They do get a better axe score relative to copper (0.832 against 0.555, where today is 0.880 against 0.740), and the break-even says that food gain wins for 3150 of 3234 steel-capable bands. It does not say what population does over 300 years. Not run locally. See below.
- **Chain depth.** Steel's depth penalty matters less once copper is this expensive. Bands have a stronger reason to climb to steel, and to climb sooner. Guard rails that watch the year steel appears, and the share of bands on it, will move. That is the point of the change, and it is also how a bad constant would show up. Spread 0.50 leaves a thin set of foragers on copper. A guard rail that expects zero copper among steel-capable bands will fail, and that failure is the compression working.
- **Points.** The slot prices `working_edge_q` on both accounts. Steel still takes 2744 point slots at spread 0.50 (1723 from copper, 1015 from iron, 6 from bone), because that factor and the wear agree. Fish is multiplied by 0.765 for a steel point and by 0.324 for a copper point, where today every point multiplies by 1. The split working cut is not this term: using it drops the priced flint point behind copper. That is a yield change, not only a ranking change.
- **Knives.** The obsidian-over-iron reversal at depth 0 is fixed, on the food term, and the test expects the iron knife with the feature on. Flint over copper is kept, for the reason in the model section. The two comparisons that still differ by flag are the ones that should: the default build keeps an obsidian knife over copper and a copper point over flint; the feature does the opposite.
- **The bow.** Fight pays 0.3 for any point and nothing for a better point. Mentioned so it is not mistaken for a fixed hole. Not changed in this proposal. The audit's line is `0.3 * edge_q(mat(kit.point))`.
- **Cordage.** Flax replaces a lot of hide, mostly on farms (164 against 57). Fibre replaces hide on foragers (342) even though fibre wears slightly faster. There is no rot, so plant cordage never gets worse in the wet. A rot term would push back toward hide and would be a new property. Not in this proposal.
- **Stone above copper, on life.** One forager's copper axe goes to stone at spread 0.50. Stone's compressed life (0.730) beats copper (0.630). Fine for a ground celt, generous for a flake. The priced axe usually still prefers copper, which is why the flip is one band and not the life order. One edge class cannot say both.
- **Negative wear.** No clamp. Current props stay under 1.15. A richer material later could make upkeep negative. Compression makes a zero raw life into 0.400, so the edge column for a non-tool (brine) is no longer zero. Wear stays positive.
- **Fight.** `edge_q` and the axe fight term move with compressed edge life. Copper's `edge_q` falls from 0.714 to 0.547. Steel's falls from 0.870 to 0.827. Smaller than the uncompressed drop (copper was going to 0.471). `tests.rs` still asserts that steel weapons raise fight strength by less than 1.1 times against an iron-and-bronze kit. That assertion is about iron and bronze, not copper, and those scores stay close. Do not assume a copper band's fight number is stable.
- **Golden hashes.** Feature off, they must stay `0x1845e2690330d59` (the long run and the first of the v10 pair) and `0x5cef0617c34457c6` (the run-on). The asserts are `#[cfg(not(feature = "durability-v2"))]`, so a feature-on run does not fail them and also does not record the new hash. This pass: `cargo test --release -p civ-core --features durability-v2` passed (114 tests, 0 failed, 10 ignored), including `payout_invariant`, `working_edge_orders_knife_axe_and_point` and `split_model_orders_retention_break_and_price`. `cargo test --release --workspace` (feature off) passed: civ-core 113 tests, 0 failed, 10 ignored, and the kindling and doc-test targets were empty. The hash asserts in that run still expect `0x1845e2690330d59` and `0x5cef0617c34457c6`. `cargo build --release -p kindling` finished with 0 warnings. Record the feature-on hashes when the feature is actually tried. Do not treat a mismatch with the old constants as a failure of this proposal, and do not update the constants in the default build.

The kit is fitted at the start of the season. Practice, then a lesson, move `use_skill` after that. `payout_invariant` rebuilds the fish multiplier from the user factor at the end of the step and compares bits with the stored one. Under the feature that comparison was off by about 0.0009: the band the test finds had just been taught a food form, and the point, hunt and net terms now scale with the user factor instead of a flat presence, so the lesson shows up in the fish number. The feature build, after a lesson only, calls `apply_base` and `refine_kit` again on the same items. No new choice. That is a feature-on behaviour change: a taught band's food yields update in the season of the lesson, instead of lagging until the next refit. An untaught band still lags one season, on both builds. The default build does not reapply. Its golden hashes include that lag.

No local simulation was run (seed 5, 300 years, one thread). Batch runs are Build Bot's job. The useful pair is the default map, seed 5, 300 years, `--threads 1`, feature off and feature on: population, band count, and axe material counts at the end, plus the usual guard rails (population floor, year of steel, share of bands on steel, chain depth). The ignored diagnostics to re-run on `DIAG_SAVE`, both ways, are `diag_durability_table`, `diag_durability_flips` and `diag_axe_breakeven`.

## Saves

Durability is not saved. `save.rs` does not mention it. It is derived from the material.

The kit is saved. `after_load` (`world.rs:1114`) rebuilds toolkits and calls `refresh_toolkit` (`world.rs:1695`), which calls `fit_kit` (`world.rs:1688`) when the toolkit cache misses or the kit is marked dirty. `fit_kit` calls `make_kit_for_view`. Measured on this save: with the feature off, the loaded kit matches the default scorer and the held axes are the file's own (copper 772 among steel-capable bands, and the full-census v1 argmax is copper 806, iron 264, steel 2209, stone 24). With the feature on at spread 0.50, the same bytes load, the kit is re-chosen, and the steel-capable census is copper 81, iron 3, steel 3150. No format version change. A save written under one model loads under the other. The kit may change on load when the feature differs from the build that wrote the file. The bytes do not.

## Resharpening as its own cost

Two accounts, both behind the feature, selected by `WEAR_ACCOUNT`. The sim uses `Split`. A diagnostic can score `Combined` without a second build (`with_durability_model` / `with_split_model`). Neither account adds a per-band, per-tick loop. Both are pure functions of the material, called from the upkeep and food paths that already existed.

Split, for an edge only:

```text
keep        = (0.65 * hard + 0.35 * sharp) / 10
retention   = keep * keep                         malleability is not here
interval    = max(0.02, retention)                seasons of keenness, relative
hone cost   = (1 - fracture/10) * repair + (fracture/10) * 1
resharpen   = 0.07 / interval * hone cost         RESHARPEN_USE = 0.07
toughness   = chip_tough / 10                     same knap discount as the combined edge
break       = (1 - toughness) * 0.60              BREAK_SCALE = 0.60
wear        = resharpen + break
working cut = cut_q * retention * (0.25 + 0.75 * toughness)
axe score   = 0.5 * working cut + 0.5 * toughness
```

`0.07` and `0.60` put the labour on the same scale as today's wear (about 0.15 to 1.0), not on a scale of hours. The `0.25` floor is so a knapped edge's food does not go to zero; without it flint would lose the knife to soft copper. A fracture of 10 makes the hone cost 1, which is a new edge, not a hammered one. Malleability enters only through `repair` (`1 - 0.045 * malleable`).

Combined is unchanged from the previous pass. One compressed edge life. Wear is `(1.15 - life) * repair`. The axe is half fresh cut, half that life. The knife and the point are `working_edge_q = cut_q * (0.40 + 0.60 * raw life)`. Spread 0.50 is that account's knob. The split account does not compress.

### Every material, split account

Printed by `diag_durability_table` (feature on; the functions do not read the flag, so the feature-off binary prints the same columns). `fresh` is `cut_q`. `work` and `knife` are the average cut between hones. `plants` is `0.1 * axe`, `farm` is `0.2 * axe`, the coefficients the yield already uses. Brine, meat, grain and the other non-tools sit on the interval floor (0.02). They are not edges. The floor only keeps the division finite.

| material | fresh | work | interval | resh | brk | wear | axe | knife | plants | farm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| wood | 0.200 | 0.006 | 0.040 | 1.750 | 0.240 | 1.990 | 0.303 | 0.006 | 0.030 | 0.061 |
| stone | 0.420 | 0.070 | 0.245 | 0.286 | 0.254 | 0.539 | 0.324 | 0.070 | 0.032 | 0.065 |
| flint | 0.760 | 0.162 | 0.540 | 0.130 | 0.485 | 0.614 | 0.177 | 0.162 | 0.018 | 0.035 |
| obsidian | 0.820 | 0.126 | 0.501 | 0.140 | 0.555 | 0.695 | 0.100 | 0.126 | 0.010 | 0.020 |
| clay | 0.040 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.050 | 0.000 | 0.005 | 0.010 |
| copper ore | 0.152 | 0.004 | 0.061 | 1.147 | 0.429 | 1.576 | 0.145 | 0.004 | 0.014 | 0.029 |
| tin ore | 0.260 | 0.025 | 0.179 | 0.392 | 0.372 | 0.764 | 0.202 | 0.025 | 0.020 | 0.040 |
| plant fibre | 0.020 | 0.000 | 0.020 | 3.500 | 0.420 | 3.920 | 0.150 | 0.000 | 0.015 | 0.030 |
| hide | 0.040 | 0.000 | 0.020 | 3.500 | 0.240 | 3.740 | 0.300 | 0.000 | 0.030 | 0.060 |
| bone | 0.430 | 0.043 | 0.174 | 0.402 | 0.338 | 0.739 | 0.240 | 0.043 | 0.024 | 0.048 |
| meat | 0.008 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.050 | 0.000 | 0.005 | 0.010 |
| wild plants | 0.020 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.050 | 0.000 | 0.005 | 0.010 |
| fish | 0.008 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.050 | 0.000 | 0.005 | 0.010 |
| charcoal | 0.040 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.050 | 0.000 | 0.005 | 0.010 |
| ceramic | 0.280 | 0.017 | 0.154 | 0.454 | 0.480 | 0.934 | 0.109 | 0.017 | 0.011 | 0.022 |
| copper | 0.480 | 0.073 | 0.164 | 0.254 | 0.060 | 0.314 | 0.486 | 0.073 | 0.049 | 0.097 |
| tin | 0.150 | 0.002 | 0.023 | 1.991 | 0.300 | 2.291 | 0.251 | 0.002 | 0.025 | 0.050 |
| bronze | 0.650 | 0.208 | 0.345 | 0.148 | 0.060 | 0.208 | 0.554 | 0.208 | 0.055 | 0.111 |
| grain | 0.040 | 0.000 | 0.020 | 3.500 | 0.540 | 4.040 | 0.050 | 0.000 | 0.005 | 0.010 |
| flax | 0.020 | 0.000 | 0.020 | 3.500 | 0.330 | 3.830 | 0.225 | 0.000 | 0.023 | 0.045 |
| wool | 0.012 | 0.000 | 0.020 | 3.500 | 0.360 | 3.860 | 0.200 | 0.000 | 0.020 | 0.040 |
| iron ore | 0.200 | 0.011 | 0.106 | 0.663 | 0.372 | 1.035 | 0.196 | 0.011 | 0.020 | 0.039 |
| iron | 0.640 | 0.215 | 0.363 | 0.141 | 0.060 | 0.201 | 0.557 | 0.215 | 0.056 | 0.111 |
| brine | 0.000 | 0.000 | 0.020 | 3.500 | 0.600 | 4.100 | 0.000 | 0.000 | 0.000 | 0.000 |
| salt | 0.100 | 0.001 | 0.026 | 2.651 | 0.540 | 3.191 | 0.050 | 0.001 | 0.005 | 0.010 |
| pitch | 0.020 | 0.000 | 0.020 | 3.500 | 0.480 | 3.980 | 0.100 | 0.000 | 0.010 | 0.020 |
| steel | 0.780 | 0.359 | 0.497 | 0.106 | 0.060 | 0.166 | 0.629 | 0.359 | 0.063 | 0.126 |
| gold | 0.160 | 0.004 | 0.039 | 0.987 | 0.300 | 1.287 | 0.252 | 0.004 | 0.025 | 0.050 |
| resinous wood | 0.080 | 0.001 | 0.020 | 3.500 | 0.360 | 3.860 | 0.200 | 0.001 | 0.020 | 0.040 |

Tool edges, read off that table. Working cut, highest first: steel 0.359, iron 0.215, bronze 0.208, flint 0.162, obsidian 0.126, copper 0.073, stone 0.070, bone 0.043, gold 0.004. Axe score, highest first: steel 0.629, iron 0.557, bronze 0.554, copper 0.486, stone 0.324, gold 0.252, bone 0.240, flint 0.177, obsidian 0.100. Iron is just above bronze on both. Stone outlasts copper between hones (interval 0.245 against 0.164) and then loses the axe, because it breaks (0.254 against 0.060) and its fresh cut is poor. Flint and obsidian stay keen on paper and lose the axe because the edge is gone. Their wear is mostly breakage (flint 0.485 of 0.614, obsidian 0.555 of 0.695). Copper's wear is mostly the hone (0.254 of 0.314).

The knife column is the food term, not the priced slot. On food alone, obsidian (0.126) still beats copper (0.073): a glass edge, while it lasts, cuts more than dull copper. The priced slot does the opposite, because obsidian wear is 0.695 against copper 0.314. The unit test locks the priced order, not this column.

### Combined account, the tool edges

Same diagnostic. `work` here is `working_edge_q`. It does not move with the spread. `life` and `wear` and `axe` do.

Spread 1.00 (raw life):

| material | fresh | work | life | wear | axe | knife | plants | farm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| obsidian | 0.820 | 0.397 | 0.140 | 1.010 | 0.480 | 0.397 | 0.048 | 0.096 |
| flint | 0.760 | 0.460 | 0.342 | 0.808 | 0.551 | 0.460 | 0.055 | 0.110 |
| stone | 0.420 | 0.334 | 0.659 | 0.491 | 0.540 | 0.334 | 0.054 | 0.108 |
| copper | 0.480 | 0.324 | 0.460 | 0.411 | 0.470 | 0.324 | 0.047 | 0.094 |
| bronze | 0.650 | 0.565 | 0.783 | 0.268 | 0.716 | 0.565 | 0.072 | 0.143 |
| iron | 0.640 | 0.565 | 0.804 | 0.252 | 0.722 | 0.565 | 0.072 | 0.144 |
| steel | 0.780 | 0.765 | 0.967 | 0.138 | 0.874 | 0.765 | 0.087 | 0.175 |
| bone | 0.430 | 0.281 | 0.422 | 0.728 | 0.426 | 0.281 | 0.043 | 0.085 |
| gold | 0.160 | 0.076 | 0.125 | 0.563 | 0.143 | 0.076 | 0.014 | 0.029 |

Spread 0.50 (what this account would charge):

| material | fresh | work | life | wear | axe | knife | plants | farm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| obsidian | 0.820 | 0.397 | 0.470 | 0.680 | 0.645 | 0.397 | 0.064 | 0.129 |
| flint | 0.760 | 0.460 | 0.571 | 0.579 | 0.666 | 0.460 | 0.067 | 0.133 |
| stone | 0.420 | 0.334 | 0.730 | 0.420 | 0.575 | 0.334 | 0.057 | 0.115 |
| copper | 0.480 | 0.324 | 0.630 | 0.310 | 0.555 | 0.324 | 0.055 | 0.111 |
| bronze | 0.650 | 0.565 | 0.791 | 0.262 | 0.721 | 0.565 | 0.072 | 0.144 |
| iron | 0.640 | 0.565 | 0.802 | 0.254 | 0.721 | 0.565 | 0.072 | 0.144 |
| steel | 0.780 | 0.765 | 0.884 | 0.200 | 0.832 | 0.765 | 0.083 | 0.166 |
| bone | 0.430 | 0.281 | 0.611 | 0.539 | 0.520 | 0.281 | 0.052 | 0.104 |
| gold | 0.160 | 0.076 | 0.463 | 0.378 | 0.311 | 0.076 | 0.031 | 0.062 |

At spread 0.50 the axe term puts stone (0.575) above copper (0.555), and flint (0.666) above both. The priced axe still prefers copper, because stone wears at 0.420 against copper 0.310. The split axe term does not need that rescue: copper 0.486 already beats stone 0.324, flint 0.177 and obsidian 0.100.

### Orders, priced, both accounts

At chain depth 0, nominal band, `slot_score`. Required, and passing, for combined at spread 0.50 and for split:

- Knife and point: steel, iron, bronze, flint, copper, obsidian.
- Axe: steel, iron, bronze, copper, stone, flint, obsidian.
- Iron spear over obsidian. Steel spear over iron. Flint knife over stone. Copper knife over obsidian. Copper point over obsidian.

Bronze does not appear in the held axe census on this save. The score puts it just under iron (split axe 0.554 against 0.557; combined 0.721 against 0.721 before the last digit, iron higher). Availability and chain depth are why the census is empty of bronze, not the cap.

The food term and the priced slot disagree in one place under split, and the same place under combined. Obsidian's working cut is above copper's (split 0.126 against 0.073; combined 0.397 against 0.324). Wear is what hands the priced knife and the priced point to copper. I am leaving that as it is. A glass point really is sharper. It also really is the one you replace.

### What the save does

`diag_durability_flips` and `diag_axe_breakeven`, this pass. Save `/workspace/bots/build-bot/step3-saves/main_seed6_y350_256x160.civsave`, format 16, year 350, 3354 bands, population 67836, 2308 farms and 1046 foragers. Feature on, the scorer matched `slot_score_m` on 0 of 103022, and the loaded kit matched the live split pick on 0 of 31774 (load refits). Feature off, the same two checks were 0, and the loaded kit is the file's own. The feature-off binary does not score the new accounts. Its "split" census column is a copy of v1, not a measurement.

Axe argmax, all bands that take an axe (3303 of 3354):

| account | copper | iron | steel | stone |
| --- | ---: | ---: | ---: | ---: |
| v1, feature off or the v1 column | 806 | 264 | 2209 | 24 |
| combined, spread 1.00 | 16 | 27 | 3234 | 26 |
| combined, spread 0.50 | 113 | 15 | 3150 | 25 |
| combined, spread 0.35 | 331 | 16 | 2926 | 30 |
| split | 56 | 26 | 3197 | 24 |

Among the 3234 who can make steel (`diag_axe_breakeven`, which reloads under the account it is scoring):

| account | copper | iron | steel |
| --- | ---: | ---: | ---: |
| v1, feature off | 772 | 253 | 2209 |
| combined, spread 0.50 | 81 | 3 | 3150 |
| split | 29 | 8 | 3197 |

The 69 bands that cannot make steel are, under split, 27 copper, 18 iron and 24 stone. None of them moved onto a stone axe they did not already hold. Combined at spread 0.50 moves one copper axe to stone. Split does not.

`cur` is the holder's margin over steel at chain skill. Positive means they keep what they have.

- Feature off. All 772 copper holders keep copper, `cur = +0.0453` (farms +0.0220, foragers +0.0484). All 253 iron holders keep iron, `cur = +0.0175`. Bands on steel prefer steel, n=2208, `cur = -0.1337`.
- Combined, spread 0.50. The 81 copper holders are all foragers, `cur = +0.0136`, food gain of steel only +0.0295 against a labour gap of 0.0430. The 3 iron holders are foragers, `cur = +0.0322`. Steel holders, n=3149, `cur = -0.1912` (farms -0.2376, foragers -0.0679).
- Split. The 29 copper holders are all foragers, `cur = +0.0042`, food gain of steel only +0.0123 against a labour gap of 0.0166. Depth of the copper chain averages 3.2; steel's averages 10.5. The 8 iron holders are foragers, `cur = +0.0176`, food gain of steel +0.0136. Steel holders, n=3196, `cur = -0.1679` (farms -0.2112, foragers -0.0587). `mis%` is 0 on these rows. One steel band has no alternative, which is why 3196 and not 3197.

Flips from the v1 pick, farm then forage. Feature on. Nets, marks and traps do not read the edge account, so those rows are the same at every column (hide net to flax, 168 farm and 58 forage; hide net to fibre, 63 and 327; fibre net to flax, 64 and 12; tally stick to bone, 35 and 73; the snare rows). They are not repeated here.

Axes, knives, points, hunt. Split:

| slot | from | to | farm | forage |
| --- | --- | --- | --- | ---: |
| axe | copper axe | iron axe | 0 | 13 |
| axe | copper axe | steel axe | 90 | 647 |
| axe | iron axe | steel axe | 81 | 170 |
| knife | flint blade | copper knife | 0 | 10 |
| knife | flint blade | stone blade | 2 | 0 |
| knife | obsidian blade | copper knife | 0 | 1 |
| knife | obsidian blade | stone blade | 0 | 1 |
| point | bone point | flint point | 1 | 1 |
| point | bone point | steel point | 2 | 4 |
| point | copper point | iron point | 0 | 3 |
| point | copper point | steel point | 1113 | 712 |
| point | iron point | steel point | 819 | 196 |
| hunt | bone point | stone blade | 0 | 9 |
| hunt | copper point | copper knife | 0 | 6 |
| hunt | flint point | copper knife | 0 | 3 |
| hunt | flint point | copper point | 0 | 1 |
| hunt | flint point | stone blade | 0 | 3 |
| hunt | sharpened-stick spear | steel point | 1 | 0 |
| hunt | sharpened-stick spear | stone blade | 2 | 2 |

No iron, bronze or steel knife moves to obsidian. The ten foragers who drop a flint knife for copper are a chain-depth case. At depth 0 the test still has flint ahead of copper. Combined at spread 0.50 does not make those ten moves; spread 1.00 makes nine of the same kind. Split behaves like the raw combined life on that knife, not like the compressed one.

Combined, spread 0.50, the edge rows only (the census above is the summary): copper axe to steel 90 farm and 599 forage, copper axe to iron 0 and 3, copper axe to stone 0 and 1, iron axe to steel 81 and 171. Points: copper to steel 1064 and 658, iron to steel 819 and 196, copper to iron 0 and 3. Knives barely move (one farm's copper knife to stone, two flint to stone, two obsidian leaving). Hunt: one bone point to stone, five flint points to copper. Spread 1.00 moves more axes (copper to steel 90 and 682, and 16 foragers' copper axes to iron) and nine foragers' flint knives to copper. Spread 0.35 is the one that puts two foragers' iron axes onto stone and two flint knives over copper, and it puts the bronze axe term above iron. That is still why 0.35 is not a combined-account setting.

Band-slot totals, v1 to the new score: spread 1.00 is 4780, spread 0.50 is 4563, spread 0.35 is 3876, split is 4752. An earlier pass printed 4766, 4550 and 3862 for the three spreads. The axe argmax is the same numbers as that pass. The small difference is the kit context: this run loads once, under the live split account, and then rescores. The break-even reloads under the account it is reporting, and its combined census matches the earlier pass exactly (81, 3, 3150).

Points, the scale. Today's point multiplies fish by 1. Both accounts multiply a steel point by 0.765 and a copper point by 0.324 (`working_edge_q`). The split working cut (steel 0.359, copper 0.073) is the knife and the hunt weapon, not the point slot. Ranking still moves almost every copper and iron point to steel (1825 copper and 1015 iron under split), and those counts were scored with this factor. Not renormalized. Still a question. The point slot was left off `split_working_cut` on purpose: that cut lets copper's cheaper wear beat a flint point.

### How this plugs into step 4

Step 4 prices a food slot from a belief once the table is non-empty. Labour is the stored belief when the band has one. When it does not, labour is `tool_upkeep`, which is `upkeep_wear` times the seal and the chain depth. The food side of the belief is a learned yield, not a copy of the formula.

Both accounts use that one door. There is nothing extra for step 4 to call, and nothing that runs per tick.

- Split. The prior, and every band with an empty table, pays `resharpen + break`. A copper axe's prior labour is the hone (0.254) plus a small break (0.060), times rate, seal and depth. A flint axe's prior is mostly the break. After the band has learned, the belief stores that experienced labour and stops reading the formula until the belief moves. The yield the belief learns is `split_working_cut` for a knife and for a hunt weapon. The point slot's fish factor stays `working_edge_q` (copper 0.324, not 0.073), on both accounts. For the axe it is the plants and farm lines on `split_axe_score`. Maker skill and chain depth stay outside the material term, where step 4 already put them.
- Combined. The same door pays `(1.15 - compressed life) * repair`. One number. The belief cannot later tell a hone from a breakage, because the formula never did. The yield it learns is `working_edge_q` on the knife and the point (copper 0.324, not 0.073) and the combined axe score on the farm lines.

A stored labour belief freezes the upkeep. It does not freeze the yield. Turning the feature on after beliefs exist will move food slots through the yield even where the labour number is already learned. The flip tables in this note are the empty-table price. They are an upper bound on how fast the labour term alone moves an axe. They are not an upper bound on the yield.

### Recommendation, and the spread

Use the split account. Four reasons.

Realism of the mechanism. You asked whether re-sharpening should be its own cost. It should. Copper's measured hone is 1.8 times iron's and 2.4 times steel's, and its working cut is a third of iron's and 15% of its own fresh cut. Malleability cheapens that hone (copper's hone cost is 0.595, iron's 0.730, steel's 0.752) and does not buy a longer interval. A knapped edge is replaced rather than hammered. The combined account can only say "copper wears 1.55 times steel" at spread 0.50, or "three times" at spread 1, and both of those are the same kind of number as a snapped obsidian edge.

Emergence. A forager with a short copper chain and no fields can still keep the soft axe, because the food line barely moves and the steel chain is deep. Twenty-nine do, by 0.004. Everyone else who can make steel drops copper. That is "useless as an axe next to steel, unless you are paying for it and you have no fields," which is as close as this food formula can get without retuning the 0.1 and 0.2. The combined account at 0.50 keeps 81 such foragers, and their working cut is still 57% of iron's, which is not the sentence you wrote.

Cost. Both accounts are a few arithmetic operations on the material, inlined into `upkeep_wear`, `food_edge_q` and `axe_food_score`. No per-band sharpening state. No per-tick pass. The split numbers are not compressed, so there is no spread to evaluate on the hot path either.

The knife. Both accounts keep iron, bronze and steel ahead of obsidian at depth 0, and the tests cover both. Split does not bring the reversal back.

I do not recommend the combined account, and I no longer recommend spread 0.50 as the way to run the sim.

Pass 2 compressed edge life toward wrought iron because the raw combined formula made copper about three times as costly as steel and put a steel axe on every steel-capable band (the census at spread 1.00, re-measured this pass: 16 copper, all of them unable to make steel). The 0.50 spread was a patch so that swing would shrink. Your note cuts the other way. Soft copper should look nearly useless as an axe beside iron unless it is resharpened constantly. Pulling the life toward 0.80 hides that. It leaves copper's working cut at 57% of iron's and makes the hone invisible. The ratio section says the same thing: the stone-to-steel chopping gap in the trials is several times, and a quenched edge against soft copper is wider than these props can say. Compression makes those gaps smaller on purpose. That was the wrong direction.

The constant stays at 0.50. The feature-on sim does not read it. If the const is later switched to `Combined`, 0.50 is still the only measured spread that keeps iron's axe term above bronze. Spread 0.35 does not (bronze 0.722, iron 0.721) and it moves two iron axes to stone. I would not raise the spread back to 1 inside the combined account either: that census is the empty-copper result, and it still is not a hone. Changing the constant now would only change a comparison I am not recommending. The sizing pass is the place to retune, and the candidates are listed under the ratios, not applied here.

What I am not claiming. Twenty-nine foragers keeping copper is not a calibrated "right" count. The food coefficients mute the dull-time, so more copper survives than a felling-time ratio alone would suggest. Steel's split wear (0.166) is a little below today's 0.171, where the combined account made steel more expensive than today (0.200). Copper's split wear (0.314) is about today's 0.150 doubled. I have not run a 300-year sim. The population effect is unmeasured.

## Ratios between materials

Before is today's formula. Fresh cut is `cut_q` and does not change. "Life" before is `durability` (the capped sum of toughness and malleability). Wear before is `1.15 - durability`. The axe before is half fresh cut, half that durability. The knife before is fresh cut. Combined is spread 0.50. Split uses the split functions. A ratio is the first material divided by the second. For wear and for the hone and the break, a ratio below 1 means the first material costs less to keep. For life, interval, cut and the axe, above 1 means the first is better.

The plants ratio and the farm ratio are the axe ratio. `0.1 * axe` and `0.2 * axe` do not change a ratio. They change the absolute gap, which is the column that has to beat chain labour. That absolute gap is under the table, not in it.

Fresh cut (before = after; the props did not move):

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| fresh | 1.625 | 1.219 | 0.985 | 1.354 | 1.143 | 1.333 | 0.927 | 1.857 | 1.548 |

Edge life, or the split resharpen interval. Before, steel is *worse* than copper (0.979), because of the cap.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before, life | 0.979 | 0.979 | 1.000 | 1.000 | 1.714 | 1.000 | 2.333 | 1.679 | 1.714 |
| combined, life | 1.403 | 1.101 | 1.014 | 1.257 | 0.863 | 1.274 | 1.215 | 1.211 | 1.085 |
| split, interval | 3.030 | 1.369 | 1.052 | 2.104 | 0.669 | 2.213 | 1.079 | 2.028 | 1.409 |

Break and hone. The before and combined rows are total wear, because those accounts have no separate break. Split break is the same for steel, iron, bronze and copper (ratio 1), which is the props: toughness 9 for all four.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before, wear | 1.139 | 1.139 | 1.000 | 1.000 | 0.265 | 1.000 | 0.837 | 0.301 | 0.265 |
| combined, wear | 0.648 | 0.790 | 0.970 | 0.845 | 0.736 | 0.820 | 0.851 | 0.477 | 0.623 |
| split, break | 1.000 | 1.000 | 1.000 | 1.000 | 0.237 | 1.000 | 0.873 | 0.237 | 0.237 |
| split, hone | 0.417 | 0.753 | 0.951 | 0.583 | 0.889 | 0.554 | 0.927 | 0.371 | 0.518 |

Working cut. This is where the two accounts separate. Combined steel:copper is 2.36. Split is 4.92. Iron:copper is 1.74 combined and 2.95 split.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 1.625 | 1.219 | 0.985 | 1.354 | 1.143 | 1.333 | 0.927 | 1.857 | 1.548 |
| combined | 2.357 | 1.354 | 0.999 | 1.743 | 0.971 | 1.742 | 1.160 | 2.288 | 1.692 |
| split | 4.924 | 1.669 | 1.036 | 2.850 | 1.036 | 2.951 | 1.288 | 5.101 | 2.952 |

Total upkeep. Below 1 means the first material is cheaper to keep. Split steel costs about half of copper (0.529). Combined steel costs about two thirds (0.648). Today steel costs *more* than copper (1.139).

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 1.139 | 1.139 | 1.000 | 1.000 | 0.265 | 1.000 | 0.837 | 0.301 | 0.265 |
| combined | 0.648 | 0.790 | 0.970 | 0.845 | 0.736 | 0.820 | 0.851 | 0.477 | 0.623 |
| split | 0.529 | 0.827 | 0.965 | 0.663 | 0.582 | 0.640 | 0.884 | 0.308 | 0.386 |

Axe score. The plants ratio and the farm ratio are this row, before and after. Split steel:copper is 1.29. The working-cut ratio on the row above is 4.92. The axe blend throws most of that away.

| | st/cu | st/fe | fe/br | br/cu | cu/sn | fe/cu | fl/ob | st/sn | br/sn |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 1.189 | 1.073 | 0.994 | 1.115 | 1.475 | 1.108 | 1.113 | 1.753 | 1.645 |
| combined | 1.499 | 1.153 | 1.001 | 1.299 | 0.965 | 1.300 | 1.032 | 1.447 | 1.254 |
| split | 1.294 | 1.129 | 1.007 | 1.138 | 1.502 | 1.146 | 1.766 | 1.943 | 1.710 |

Knife score, the food term (fresh cut before, `working_edge_q` combined, `split_working_cut` split). Same numbers as the working-cut table. The priced knife is not this row: upkeep moves copper ahead of obsidian even where this row does not.

Absolute food, so the mute is visible. Delta is steel minus copper.

| | steel | iron | bronze | copper | stone | flint | obsidian | steel−copper |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| split axe | 0.629 | 0.557 | 0.554 | 0.486 | 0.324 | 0.177 | 0.100 | 0.143 |
| split plants, 0.1× | 0.063 | 0.056 | 0.055 | 0.049 | 0.032 | 0.018 | 0.010 | 0.014 |
| split farm, 0.2× | 0.126 | 0.111 | 0.111 | 0.097 | 0.065 | 0.035 | 0.020 | 0.029 |
| combined axe, 0.50 | 0.832 | 0.721 | 0.721 | 0.555 | 0.575 | 0.666 | 0.645 | 0.277 |
| combined plants | 0.083 | 0.072 | 0.072 | 0.055 | 0.057 | 0.067 | 0.064 | 0.028 |
| combined farm | 0.166 | 0.144 | 0.144 | 0.111 | 0.115 | 0.133 | 0.129 | 0.055 |
| before, axe | 0.880 | 0.820 | 0.825 | 0.740 | 0.502 | 0.526 | 0.472 | 0.140 |
| before, plants | 0.088 | 0.082 | 0.083 | 0.074 | 0.050 | 0.053 | 0.047 | 0.014 |
| before, farm | 0.176 | 0.164 | 0.165 | 0.148 | 0.100 | 0.105 | 0.094 | 0.028 |

Today's plants gap between steel and copper is already only 0.014. Split does not widen it. The axe scores both fell (steel 0.880 to 0.629, copper 0.740 to 0.486), so the ratio went from 1.19 to 1.29 while the absolute gap stayed 0.014. The farm gap goes from 0.028 to 0.029. A working-cut gap of 0.286 (0.359 against 0.073) arrives at the farm line as 0.029. That is the mute. It is why 29 foragers can prefer copper by 0.004: steel's food gain on their weights is +0.012, and the chain is several steps deeper.

### Against the real numbers

Felling time, steel against stone. Saraydar and Shimada, American Antiquity 36 (1971), pp. 216–217: one steel axe against one ground stone axe, same chopper, about 6.4 times faster and about 5.1 times less energy per inch of wood. The same pair of axes, clearing plots, came out about 3.6 times faster and 3.3 times less energy (Saraydar and Shimada, American Antiquity 38 (1973), pp. 344–350). Mathieu and Meyer, Journal of Field Archaeology 24 (1997), pp. 333–351, put the chopping studies they reviewed at roughly two to four and a half times, and they say tree species, diameter and the axe itself move the result as much as the material. Carneiro's plot-clearing figures, which they cite, run much higher (tens of times) and mix axes with adzes. I use 3 to 6 times as the band, and I do not use the clearing extremes. Confidence: medium. Small trials, modern steel, ground stone rather than a flake.

The sim, steel against stone. Fresh cut 1.86. Split working cut 5.10, which sits in that 3-to-6 band. Split axe score 1.94, which is the bottom of the band. Split farm coefficient, same 1.94, absolute gap 0.061. Combined at spread 0.50: working cut 2.29, axe score 1.45. So the split working cut is the one that looks like the chopping trials, and the food coefficient then compresses it to the low end. The trials were maintained edges, closer in spirit to fresh cut than to a season of not honing. Matching them with the dull-time term is partly luck. I am not retuning to chase 6.4.

Bronze against steel, felling. Mathieu and Meyer's own trial found bronze not significantly slower than steel. Confidence: medium, and they say so. The sim does not say that. Split working cut steel:bronze is 1.73 (0.359 against 0.208). Split axe score is 1.14 (0.629 against 0.554). Combined working cut is 1.35 and the axe scores are 0.832 against 0.721. The axe score is closer to "about the same" than the working cut is. The working cut exaggerates steel against bronze relative to that trial. A quench-hardened edge against work-hardened bronze would exaggerate the other way, and these props cannot say it, because steel's hardness is 6 and iron's is 5.5.

Copper against bronze, edge holding. Kienlin and Ottaway, in *L'Atelier du bronzier* vol. 2 (1998), pp. 271–286, found tin bronze better at keeping an edge than copper. I do not have their wear rates, so this is a direction, reported again by the 2023 Journal of Archaeological Science axe-efficiency paper. Confidence: medium on the direction, none on a number. Soriano and Gutiérrez Sáez, 2009, table 2 of their experimental axes (use-wear on metal; the copy I could read): pure copper about 79–104 HV, about 5% tin about 97–134 HV, about 15% tin about 125–189 HV. That is one set of replicas, not a survey of ancient edges. Confidence: medium for those replicas. The 15% tin bronze is roughly one and a half to two times the copper. The split interval bronze:copper is 2.10, and the hone labour bronze:copper is 0.58 (copper takes about 1.7 times the hone). Same neighbourhood as that hardness ratio, for bronze against copper only. Iron:bronze interval is 1.05. The sim treats wrought iron and tin bronze as nearly the same, which is defensible for unhardened iron against work-hardened bronze and too close if the iron is steeled.

Copper and steel hardness, the gap the props cannot say. Modern C11000 copper is often quoted around 50 HV annealed and around 100 HV hard. I am treating that as a commercial range, not an archaeological measurement. Confidence: low to medium. A quench-hardened carbon-steel edge is several hundred HV. I am not pinning a number I did not read off a table in this pass. Confidence that the gap is several times, and can be an order of magnitude against annealed copper: medium. Confidence in any single ratio: low. The sim's hardness ranks are copper 3, bronze 5, iron 5.5, steel 6. The split interval then comes out steel:copper 3.03, steel:iron only 1.37, iron:copper 2.21. The 3.0 is in the range of work-hardened copper against a modest steel, and it is small if the comparison is annealed copper against a file-hard edge. The 1.37 is the compressed one. It is the props. Raising steel's hardness would widen it, and would also change `cut_q` in the default build. Not in this proposal.

Where the sim is compressed. Steel against iron, on every term except the knife's working cut. Interval 1.37, axe 1.13, upkeep 0.83. A reader who wants hardened steel to hold an edge several times longer than wrought iron will not get it from these props, and the combined spread makes it worse (life 1.10). Split at least does not pull steel down toward the anchor. Steel's own wear goes from today's 0.171 to 0.166, not up to the combined account's 0.200.

Where the sim is exaggerated. The split working cut, steel against copper (4.92) and steel against bronze (1.73), relative to felling trials on maintained axes. The mechanism is the second use of retention: it is already `keep` squared, and the working cut multiplies by it again, so copper keeps 15% of its fresh cut (0.073 against 0.480) and steel keeps 46% (0.359 against 0.780). That is the "spends its time dull" term, and it is harsher than a smith who hones when the edge rolls. The hone labour is the other half of the same idea. Using both at full strength double-counts the dull season. I am not turning either down now. It is the first sizing candidate that is actually inside this feature.

Where the sim is muted, which is the one that changes the census. The axe score gives half its weight to toughness. The four tool metals share toughness 9, so they share break labour 0.060 and they share the 0.45 that half of the axe score is made of. The plants and farm coefficients then multiply by 0.1 and 0.2. A 4.9-times working cut becomes a 1.29-times axe and a plants gap of 0.014, the same plants gap the capped formula already had. The order of the axes is right. The size of the food effect is not, if the reference is felling time. The 0.1 and 0.2 are outside this feature. The 0.5/0.5 blend is inside it, and I am not changing it in this pass: it would move every axe score, and the census with it, and the instruction was to flag candidates rather than retune.

Candidates for the later sizing pass. Not changed here.

1. `PROPS` hardness, steel against iron most of all, copper against both. Biggest lever on the interval. Touches the default build's `cut_q`. Out of scope.
2. The second use of retention inside `split_working_cut`. The knob if copper's average cut (15% of fresh) is too harsh once the hone is already being charged. Inside this feature.
3. The 0.5/0.5 weights in `split_axe_score`. The knob if the food line should hear the dull-time. Inside this feature. This is the one that would change the 29 foragers.
4. The plants coefficient 0.1 and the farm coefficient 0.2. Outside this feature. They set the absolute gap (0.014 and 0.029) that a deep steel chain has to beat.
5. `RESHARPEN_USE` (0.07) and `BREAK_SCALE` (0.60). They set the labour units so the orders hold and the wear sits near today's 0.15 to 0.7. Not fitted to hours.
6. Point fish left at `working_edge_q` on both accounts (steel 0.765, copper 0.324, against today's flat 1). Not the split working cut: that one puts the priced flint point behind copper. A ranking fix that is also a yield cut. Question below, not a silent renormalization.
7. One edge class for a ground celt and a flake. Stone's interval beating copper is right for the celt and generous for the flake.

## How this lands relative to skills step 4

Decided. This wear fix lands on main **before** PR #12 (`skills-step4`, head `3521366`). #12 then merges main into `skills-step4`. A merge, not a rebase, and no force-push. The feature stays off in that merge. The default build does not move the pre-step-4 hashes, so it does not block #12's own hash update. Do not turn the feature on until the flip table has been re-run on a build that has both.

`git diff origin/main origin/skills-step4 -- civ-core/src` touches `band.rs`, `beliefs.rs` (new), `effects.rs`, `lib.rs`, `probe_diffusion.rs`, `tests.rs`, `tests_skills.rs` and `world.rs`. This branch touches `band.rs`, `effects.rs`, `lib.rs`, `materials.rs` (new code), `probe_diffusion.rs`, `tests.rs`, `tests_skills.rs`, `tests_diffusion.rs` and the diagnostics. `materials.rs` and the diagnostics are only on this branch. `beliefs.rs` and `world.rs` are only on step 4. Those four should come across whole.

Step 4 does not add a new `durability(` or `1.15 - durability` expression. Confirmed by searching that diff. The old expressions are still on main, at the sites this branch already replaced. Step 4's new food path calls `tool_upkeep` for the labour prior (`choose_food_belief`, `belief_slot_price`). It does not paste the old formula. `world.rs` calls `make_kit_beliefs` and does not mention durability.

Hunks that will conflict, and how to resolve them so the helpers survive:

- `effects.rs`, the module comment at main lines 10–16. Both branches rewrite it. Keep step 4's paragraph on beliefs, and keep this branch's sentences on `durability-v2`. Keep `use crate::beliefs`. The wear sites themselves are not in step 4's hunks (`tool_upkeep`, `hunt_q_of`, `cut_term`, `axe_term`, `cand_new`, `fight_of`, `apply_base`, `nonfood_scores`, the mark) and should merge. After the merge, `tool_upkeep` must still call `upkeep_wear`, and `cut_term` / `hunt_q_of` / `axe_term` / `cand_new` must still call `food_edge_q` and `axe_food_score`. `choose_food_belief` is inserted after `choose_food` and already calls `tool_upkeep`. Do not paste `1.15 - durability` back into any of those.
- `tests.rs`, `v10_fixture_loads_and_runs`, main around line 1122. This branch wraps the hash asserts in `#[cfg(not(feature = "durability-v2"))]` and allows the constant to be dead under the feature. Step 4 changes the run-on hash from `0x5cef0617c34457c6` to `0x4c41b83c2c81e1f5`, because beliefs change the run, and it extends the skills-counter tests. Keep step 4's new hashes. Keep the `cfg` gate and the `allow(dead_code)` around them. Do not restore the pre-step-4 run-on hash, and do not let a feature-on build be required to match either hash. The v9 hash assert is only on this branch; leave its `cfg` gate in place.
- `tests_skills.rs`. This branch's `wear()` (main line 78), `slow_score`, the mark, and the `food_edge_q` checks are not inside step 4's hunks (imports, the v16 fixture, and the new belief tests at the bottom). They should merge. If a conflict shows `sealed * rate * (1.15 - durability(mat(it)))` on the feature-on side, that line stays only under `#[cfg(not(feature = "durability-v2"))]`. The feature-on side is `upkeep_wear`. Step 4's new tests do not contain a durability expression. Do not add one while resolving them.
- `probe_diffusion.rs` line 159, `fn upkeep`, sits between step 4's hunks (the `ItemAgg` field at line 139 and `own_value` at line 210). It should merge. Keep `upkeep_wear(it)`. Step 4's `slot_own` now calls `belief_slot_price`, which calls `tool_upkeep`. Both doors stay on the helpers. Do not restore `1.15 - durability(mat(it))` in `upkeep`.

Hunks that should not conflict. `band.rs`: step 4 changes the beliefs-field comment (main line 174); this branch changes `weapon_q` and `edge_q` (main lines 704–742). Keep `axe_food_score` and `score_durability`. `lib.rs`: step 4 adds `pub mod beliefs` next to `band`; this branch adds the diag modules further down. Keep both. `tests_diffusion.rs` is only this branch. `world.rs` is only step 4.

After the merge, search `civ-core/src` for `1.15 - durability`. The only hits should be the feature-off body of `upkeep_wear`, the diagnostic that compares against the old expression, the feature-off branch of the skills-test `wear()`, and comments. A hit in `choose_food`, `cand_new`, `nonfood_scores`, `edge_q`, the probe, or `world.rs` is the cap coming back. Delete it in favour of the helper.

What to re-measure, on the merged `skills-step4` tree, feature off and feature on (`WEAR_ACCOUNT` still Split):

- `diag_durability_flips` and `diag_axe_breakeven` on `DIAG_SAVE`. The empty-table numbers in this note will not match once beliefs price food slots. The feature-off run is the control.
- The step-4 "believed wrong" share. Food beliefs will learn the split working cut on a knife and a hunt weapon, `working_edge_q` on the point slot, and labour beliefs will learn `resharpen + break` instead of `1.15 - durability`.
- The ordering tests, `working_edge_orders_knife_axe_and_point` and `split_model_orders_retention_break_and_price`.
- Default-build golden hashes. They should be step 4's pair (`0x1845e2690330d59` and `0x4c41b83c2c81e1f5`), not the pair this proposal froze. A feature-on mismatch with either pair is expected and is not a reason to edit the constants.

## Questions for Michael

The landing order is decided (this branch first, then #12 merges main in). These are still open.

1. Turn the feature on at all? My recommendation is to land the code with the feature off, and to use the split account when it is turned on. The default build stays byte-identical until that decision.
2. Are the constants acceptable? Combined edge: keep `0.65 * hard + 0.35 * sharp`, roll `0.09 * malleable`, chip `0.50 * fracture / 10`, scale `/ 45`, the pitch-versus-clay term `0.004 * hard`, repair `0.045 * malleable`, working-edge floor `0.40`, spread `0.50` (combined only; the sim does not read it while the account is Split). Split edge: retention is that keep, squared; hone scale `RESHARPEN_USE` 0.07; break scale `0.60`; chip floor on the working cut `0.25`; interval floor `0.02`. They were chosen to hit the orders and to sit on today's wear scale. They are not fitted to the felling trials. The sizing candidates are listed above and are not questions for this pass except where one of them is also a constant you might reject outright.
3. Leave `EDGE_LIFE_SPREAD` at 0.50? I recommend yes, as a dormant combined-account knob. I do not recommend using the combined account, and I do not recommend 0.35 (bronze's axe term then sits above iron) or 1.00 (every steel-capable band drops copper, and there is still no hone). Say if you want the constant deleted instead of left dormant.
4. Should a steel point's fish factor stay at `working_edge_q` (0.765, copper 0.324, on both accounts), or be renormalized so a steel point matches today's flat 1 and poorer points are a fraction of that? The code does the first. I would renormalize if the fish drop is unwanted, and I have not done it. I would not move the point slot onto `split_working_cut` (0.359 / 0.073): the priced flint point then loses to copper.
5. One edge class covers a ground axe and a flake. Split them, or leave it? Under split, stone's interval beats copper and the priced axe still prefers copper. The flake is the one that number flatters.
6. There is no rot. Flax nets beat hide. Add a property later, or accept it?
7. Steel's hardness (6 against iron's 5.5) still understates a quenched edge. Leave `PROPS` alone, as this proposal does? It is the main sizing candidate, and it would change the default build.
8. Wear has no floor. If a later material's life exceeds 1.15, combined upkeep goes negative. The split edge wear is a sum of two positive terms, so this only bites the non-edge classes and the combined account. Floor it at 0?
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

`DIAG_SAVE` selects the snapshot. One feature-on `diag_durability_flips` prints v1, the three combined spreads, and the split account. One feature-on `diag_axe_breakeven` reloads under combined (spread 0.50) and under split. The feature-off runs print the v1 census only. The ordering tests `durability_v2_orders_edges_and_v1_formula_stays` and, with the feature on, `working_edge_orders_knife_axe_and_point` and `split_model_orders_retention_break_and_price` are not ignored and run in the normal suite.
