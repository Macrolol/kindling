# Audit: formulas that flatten or reverse a real-world ordering

No simulation behaviour was changed. The only code added is an ignored diagnostic test (`civ-core/src/tests_audit.rs`). Golden hashes are untouched. A separate stream is fixing durability; that bug is listed here as the exemplar, not re-fixed.

## Summary

Eight findings. One is high, five are medium, two are low.

The high one is the bug Michael already found. `durability` puts copper, bronze and iron on the same ceiling of 1, and steel just under it (0.979), because the formula pays for malleability and then divides by 12. Wear is `1.15 - durability`, so those three metals all wear at 0.150 and steel wears faster, at 0.171. Fight order is not reversed: a steel spear is still the deadliest weapon in the table (weapon quality 0.915, then bronze 0.875, iron 0.871, copper 0.814, flint 0.707). What the cap does change is upkeep, and therefore which tool a band keeps when the food gain is small. On a nominal band, forcing every axe to chain depth 8 makes bronze's score 0.319 and steel's 0.318. The axe study already measured the larger effect, which is chain depth, not this gap.

The other findings are the same kind of mistake elsewhere.

- Woollen clothing and hide clothing both score 1.0 for warmth, so a band that can sew both always wears hide. Linen is correctly cooler (0.55). Cold need does not change, because the two winners were tied. What changes is that wool is never the garment.
- A rawhide net outscores a bast net (7.55 against 7.40 on the nominal score) and a wool net sits between them. All three have the same mesh quality, 0.800, and hide then wins on wear. Linen, once it exists, beats all three (8.79).
- A dugout and a skin boat have the same seaworthiness, 0.929, and the same nominal score, 0.890. The tie goes to wood. A plank hull is sounder on paper but the fishing radius rounds all three to the same 3 cells, and an experienced crew hits the same survival floor in all three.
- Ornaments are ranked by hardness. An obsidian pendant (6.5) beats a bronze ring (6), and bone beads (5) beat a copper ring (4). Gold is exempt, because prized goods get a flat +10.
- A hunting spear is scored on cutting quality alone. Obsidian (cut 0.820) therefore beats steel (0.780) when both chains are depth 0 (slot score 16.868 against 16.856). At depth 8 steel is ahead (16.615 against 15.421). A realistic obsidian chain is shorter than a steel chain; scaled to depth 4 the obsidian score is about 16.14, still under steel at depth 8. The ordering of the edge is wrong. The usual winner, once steel exists, is probably still steel.

What this means for results already in hand: population, farming and the spread of steel weapons are not standing on a reversed fight ranking. Knife choice on the nominal mix already prefers steel, then bronze, then iron, then obsidian. The results are weaker wherever the question is which of two similar materials is worn, knotted, paddled or displayed. Those choices are flattened, and a few of them are backwards (steel wears out before copper, rawhide nets beat bast, bone beads beat copper rings, wool never replaces hide). The axe decision in `docs/ANALYSIS-steel-axe.md` (in the axe worktree) is the one place this class of bug has already been shown to change what bands keep.

## The class

A formula whose structure flattens or reverses a real-world ordering, or erases a difference that should change a choice. A clamp that no input reaches, or a bound that is meant to be a biological or physical limit, is not in the class.

Sub-types:

- **A.** A clamp, cap, floor or saturation where several materials, techs or bands land on the same bound, so a real difference vanishes, or the bound is reached in ordinary play.
- **B.** A property used with the wrong sign, or for the wrong job. Malleability standing in for how long an edge keeps working is the example.
- **C.** An additive mix where one property can stand in for a missing essential one (a product, a minimum or a threshold would not let it), or a weight that lets an irrelevant property dominate.
- **D.** `min`, `max`, `round` or an integer conversion that hides a difference the inputs actually had.
- **E.** A constant that dominates the term, so the property barely moves the result. `1.15 - x` with `x` near 1 is the example. A large fixed offset can do the same.
- **F.** A divisor chosen so that real inputs already sit on top of each other, or past 1, before the clamp.
- **G.** A match with a default (`_ => 0.3`) that gives unrelated materials the same value, or ranks a real article wrongly.

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
cargo test --release -p civ-core audit_ -- --ignored --nocapture
```

The run passed. The tables below are that output, cut down to the rows that matter. `CAP` means the clamp changed the value. `AT1` means the raw value was already 1.

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

Fire, open / stone lining / clay / ceramic: wood 650 / 810 / 970 / 1010; charcoal 950 / 1110 / 1270 / 1310. The charcoal advantage stays 300 degrees. Copper ore reduces at 1150, so a stone lining is not enough and a clay or ceramic one is. Iron ore reduces at 1400, which still needs the bellows (+250). That ordering is intact.

No world was re-run. Where a bound is not a material formula, the note says whether the code reaches it in ordinary play. The axe numbers already measured in the other worktree were not repeated.

## Ranked findings

| Rank | Severity | Where | Issue | Impact | Proposed fix |
|---|---|---|---|---|---|
| 1 | High | `materials.rs:537`, `effects.rs:373` | Durability caps copper, bronze and iron at 1 and ranks steel lower, because it pays for malleability. Wear is `1.15 − durability`, so steel wears out first. | Every upkeep comparison among metals. Fight order stays steel first. Axe keep-or-switch moves, as the axe study measured. | Score edge life from toughness and hardness, not malleability, and do not let three metals share the ceiling. |
| 2 | Medium | `materials.rs:563`, `effects.rs:1249` | Hide, wool and plant fibre all have warmth 1.0. Flax is special-cased to 0.55. The tie keeps hide. | Wool clothes are never worn once hide clothes exist. Cold need is unchanged, because the scores were equal. | Rank warmth by loft: wool, then hide, then linen and bast, with no shared ceiling. |
| 3 | Medium | `materials.rs:558`, `effects.rs:671` | Mesh quality ties fibre, hide and wool at 0.800. Wear then picks hide, then wool, then bast. | Until linen exists, the net and the snare are rawhide, not bast cord. | Make mesh a product of fibre and strength, so bast beats wool and rawhide. |
| 4 | Medium | `materials.rs:553`, `effects.rs:873`, `world.rs:1804`, `world.rs:2761` | Dugout and skin boat both score 0.929. Fish radius rounds plank, dugout and skin to 3. Veteran survival floors at 0.3 for all three. | Skin boats are never chosen when a dugout is known. A plank hull does not fish any further, and does not keep an experienced crew safer. | Separate wood and hide, and stop rounding the fish radius. Do not floor the survival term so early. |
| 5 | Medium | `effects.rs:1258` | Ornaments are ranked by hardness, plus 10 if the material is prized. | Obsidian beats bronze. Bone beats copper. Gold is fine. Prestige *amount* ignores hardness and only adds `prized`, so the rate of prestige barely moves; the thing people wear, and the material they then want, does. | Rank show by prized and by being metal, not by scratch hardness. |
| 6 | Medium | `effects.rs:500`, `materials.rs:532` | Hunt food uses `cut_q` and not toughness. Obsidian's edge outscores steel's. | At depth 0 a nominal band prefers the obsidian spear by 0.012. At realistic depths steel is ahead again. The edge ordering is still wrong, and the margin is borrowed from wear. | Put durability into the hunt food term (as `edge_q` already does for a fight), or stop letting sharpness alone outrank a metal. |
| 7 | Low | `effects.rs:838`, `effects.rs:693` | A bow's fight bonus is a flat 0.3 whenever any point is carried. The point's food term is presence, not material. | Bone arrows and steel arrows fish and fight the same once the bow exists. The spear in the hunt slot still cares about material. | Scale the bow's fight term by the point's `edge_q`. |
| 8 | Low | `materials.rs:549`, `effects.rs:892` | `vessel_q` defaults to 0.3, so wood, stone, every metal, bone, pitch and meat are one vessel. The protection cap 0.92 binds when fire, a warm garment, a kiln and an axe are all present (0.95 becomes 0.92). | No recipe makes those default vessels, so the 0.3 does not change play. The protection cap shaves a little warmth off a fully equipped band and cannot tell wool from hide, which were already tied. | Default unknown vessels to 0. Raise or drop the protection cap only after garment warmth actually differs. |

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

Medium. Sub-types A, B and F.

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

Medium. Sub-types C, D and E.

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

Impact. Where there are trees and an axe (cut quality above 0.4, which a flint axe has), the skin boat is dominated and never selected. In a treeless country the dugout recipe, if it has spread, is still the one in the kit, and the skin boat the band can actually cover with hide is not. Fishing reach does not reward a plank hull. Experienced sailors do not survive better in one. Reed boats remain worse, which is right.

Proposed fix, not applied. Give the dugout a small edge that the props can support (wood keeps its float in the water; hide's seal is a skin over a frame), and use the unrounded quality for the radius:

```rust
pub fn boat_q(m: Mat) -> f32 {
    let p = m.props();
    let hull = if p.float >= p.seal { p.float } else { 0.85 * p.seal };
    (p.tough + hull) / 14.0
}
```

Wood stays 0.929. Hide becomes (6 + 0.85×7) / 14 = 0.854. Fibre stays 0.643. The fish radius should be the continuous `sea_range`, or at least `1 + (2.0 * q)` without `round`, so a plank at 1.0 reaches one step further than a dugout at 0.93. The survival floor of 0.3 should sit lower than a perfect hull with a veteran crew, or veterans in a reed boat and veterans in a ship will keep meeting it. For example `max(0.15)` leaves room between q = 0.93 and q = 1 at experience 1 (hazards 0.22 and 0.15).

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

Medium. Sub-types B and C.

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

### 8. The vessel default, and the protection cap

Low. Sub-types G and A.

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

Makeable vessels are clay (then fired to ceramic), and cord baskets or bags of fibre, flax, wool and hide. Their order, ceramic 1, hide 0.6, bast and linen 0.5, wool 0.4, raw clay 0.25, is a reasonable storage order. The default 0.3 ties wood, stone, every metal, bone, pitch, meat and the ores. Physics has no recipe that makes those into a vessel, so the tie is latent. It is still the pattern the class names: a default that gives unrelated materials one rank. A metal cauldron, if a later step added one, would store worse than a basket.

Protection (`effects.rs:892`) is capped at 0.92. With a fire and a warmth-1 garment the value is 0.85. With a kiln as well, 0.90. With an axe as well, 0.95, which becomes 0.92. So the cap binds only for a band that already has fire, warm clothes, a kiln and an axe. It shaves 0.03 off protection. It cannot separate wool from hide. That is finding 2. The cap is a small extra flatten on top.

Proposed fix, not applied. `_ => 0.0` in `vessel_q`, so an unexpected material is not a mediocre pot. Leave the 0.92 cap until garment scores actually differ; then check whether a wool cloak plus a fire should be allowed to pass it.

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

**Trait bounds that are supposed to bind.** Curiosity, ambition and craft are clamped on birth and on inheritance (`world.rs:1009`, `:1014`, `:4044`, `:4045`, `:4052`, `:4117`). The ranges are the trait ranges. Prestige learning, `world.rs:163`, is clamped to 0.5..2. A large ornament-making band can hit 2 (equilibrium prestige is on the order of `sqrt(pop / 25)`, and the factor is `1 + 0.5 * gap`). Further prestige then does not teach any faster. That is a saturation of a social rate, not a reversed technology order. Left as fine.

**Demography, meant to saturate.** `world.rs:2207` sets births from `((food_ratio - 0.75) / 0.25).clamp(0.0, 1.0)`, and `food_ratio` is already capped at 1. Well-fed bands all have the same fertility from this term. Bands under 75% of need have none. Deaths still use the raw `(1 - food_ratio)`. `world.rs:4862` caps stress at 1, which binds at food_ratio 0.5, so a worse famine does not raise stress further. Migration and tinkering see the cap. Mortality does not. Both are intentional 0..1 states. `conflict.rs:49` uses `food_ratio.min(1.0)` so a well-fed fighter is not stronger for being overfed.

**Farming suitability.** `farming.rs:59` and `:60` clamp water and warmth to 0..1. A wetter field past the top of the range does not yield more. That is a niche width, not a material ranking.

**Skill stored in 0..1.** `effects.rs:330`, `knowledge.rs:596` and the following lines. Skill and quality are defined on that range.

**Qualities whose clamp never moves a real input.** `cut_q` max is 0.820. `boat_q` max is 0.929. `mesh_q` max is 0.933. `fire_temp` is not clamped, and charcoal stays 300 degrees above wood under every lining (open 950 against 650, ceramic kiln 1310 against 1010). Kiln rank by refractory is ceramic 9, clay 8, stone 4. Charcoal plus stone is 1110, under copper ore's 1150. Charcoal plus clay is 1270 and plus ceramic is 1310, both over it. Iron ore at 1400 still needs bellows. That ladder is right.

**Sails, among sails that exist.** The score is toughness, plus 10 if the cloth is not plant-fibre matting (`effects.rs:1255`). Makeable sails are linen 14.5, wool 14, mat 3. Linen over wool over matting is the right order. The +10 is large and would also prefer a hide sail, but hide is not a sail recipe.

**Hammers.** Only a raw cobble with hardness and toughness both at least 5 becomes a hammer. That is stone. Flint is hard and brittle and is excluded. The kit ranks hammers by hardness, and stone and steel both have hardness 6, but there is no steel hammer recipe. The tie cannot happen. Food treats "has a hammer" as 1, so which stone it is does not change a yield.

**Shafts and digging sticks.** Every shaft scores 1.0 (`effects.rs:1247`). The only shaft physics allows is wood. Nothing is flattened that can exist.

**Cord as a food tool.** The cord slot is ranked by `tough + 1` (`effects.rs:1252`). Makeable cord is hide 7, flax 5.5, wool 5, fibre 4. Yields use the cord as present or absent, not as a quality. The ranking changes which material is demanded (rawhide thongs), not how much food the cord multiplies. If hide is sinew, preferring it is reasonable. Not raised as a finding. The bow string uses the same number; see finding 7.

**Marks.** `durability + 1` for ceramic (`effects.rs:1263`) makes a fired token beat wood and bone, which is the point of firing it. Wood's tally scores 0.500 and bone's 0.417, so a stick beats a bone tally because wood is tougher in the table. Wood also rots. The margin is small and the fired token wins whenever it exists. Left as fine.

**Cooking.** Digest pays a ceramic pot and not a hide bag. You cannot boil in a bag. The binary check is the right shape.

**Metal scrap in `marginal_mats`.** Every metal is reported at 0.02 and every other material at 0.06 (`effects.rs:812`). That flattens copper and steel into one scarcity inside that vector. The kit choice itself prices labour through `tool_upkeep`, not through this vector. Noted so it is not mistaken for the axe decision.

**Brine's durability floor.** Raw durability 0 is raised to 0.05. Brine is not a tool.

## Questions for Michael

1. Durability is listed as high and left alone, on the assumption the other stream owns the formula. Should findings 3 and 6 wait for that change, since both are partly "wear is doing the work the quality term got wrong"?
2. Is `Hide` sinew, leather, or both? Bowstrings and snares look right if it is sinew, and wrong if it is rawhide. Nets look wrong either way.
3. Wool and hide having the same warmth makes every cold band wear hide. Is the intended clothing story "hide is as warm as wool, and wool is for sails and cloth", or should wool replace hide once sheep are sheared?
4. Obsidian out-cutting steel is defensible for a fresh flake. Should `cut_q` stay a fresh-edge number, with wear as the only penalty, or should the food term use the working edge the way combat already does?
5. A plank hull does not fish further than a dugout, because of `round`. Is the radius step a stand-in for "boats, not rafts", or should a ship actually reach more water?
6. Step 4's prior copies `material_term` from these formulas, including a point slot of exactly 1. If the durability fix lands first, the prior should be re-read so it does not freeze the capped axe scores into beliefs.
