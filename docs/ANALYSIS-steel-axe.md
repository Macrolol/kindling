# Why bands keep a copper or iron axe when they can make a steel one

This is a decision note, not a change. Nothing in the simulation was retuned. The numbers below are from the current step-3 code (maker skill included).

## The short answer

A band keeps the axe that gives it more food than the labour costs, in its own harvest. A steel axe does cut more. It does not cut enough, for most of these bands, to pay for how deep the steel chain is.

The food term on an axe is small. Plants gain `0.1` times the axe's material score, fields gain `0.2` times it. Steel's score is 0.880 against copper's 0.740 and iron's 0.820, so the raw edge over copper is modest and the edge over iron is small. Upkeep labour is `wear × (1 + 0.3 × chain depth)`. Copper sits about 3 steps deep. Iron and steel sit about 8 or 9. That depth factor, not a lack of skill and not a missing user bonus, is what the band is paying. Steel is also a little less durable than copper in the current formula (0.979 against 1.000), because copper is more malleable and durability is already capped at 1. A steel axe does not last longer today. It wears slightly faster, and it is blamed for a much longer chain.

On a developed world (the step-2 seed-6 year-350 save, kits refit by the step-3 choice) this splits cleanly by how the band eats. Of 3,234 bands that can make a steel axe, 2,209 already hold one, and they are the farming bands. 772 hold copper and 253 hold iron. Every one of those 1,025 ranks the axe it holds above steel. The copper bands barely farm (farm weight 0.02). Steel would add about 0.049 of food and is priced at about 0.094 of labour, so copper leads by 0.045. The 90 copper bands that do farm are close: steel adds 0.124 of food against 0.146 of labour, a gap of 0.022. They are the only copper bands anywhere near a flip.

On a younger world that actually grew under step 3 (seed 5, default map, 300 years: 6,050 bands, population 104,860, 14% of food from farming) nobody who can make steel still holds copper. Copper arrives late there. 4,399 bands hold an iron axe and 619 hold steel. All 4,399 rank iron higher. Steel adds only 0.019 of food against 0.042 of priced labour, because an iron axe is already good and only about one step shallower. The probe's "worse held" yardstick mostly does not even call this pair worse: the value gap is about 0.013, inside the 0.02 tie. The 23% copper-over-steel and 8% iron-over-steel figures in the step-3 probe are the same pattern, counted across seeds 6–10 and 500 years, and only where the probe's unweighted score clears that tie.

Maker skill is a small extra shove toward the axe they already prefer. Steel's chain skill is high (about 0.96) even when the steel recipe itself is not (about 0.48), because the chain is a weighted mean of every step and the earlier steps are practised. Waste is about 3% on steel against 1–2% on copper. Failure is under half a percent. On the seed-6 save, 29 farming bands would take steel if every maker were perfect. Chain skill puts all 29 back on copper. It does not create the gap. Depth does.

So a steel axe that loses is not a bug in the kit choice. The choice matches food-minus-labour in every measured band: the two accounts never differ by more than 0.001, and the mismatch rate is 0. It can still be the wrong *design* if chain depth should not be a cost paid every season, or if the probe should not treat an unweighted farm line as the truth. Those are the options below. None of them is implemented.

## What was measured

Two snapshots, one local simulation at a time. No batch run.

1. **Seed 6, 256×160, year 350**, save `main_seed6_y350_256x160.civsave` (format 16, the step-2 world). Loaded by current step 3, which refits the kit. 3,354 bands, population 67,836. Same file as the step-2 diagnostic, which then found 747 copper-over-steel kits. After the refit there are 772. The step-2 write-up's food gain of +0.046 and margin of +0.043 are the same accounting with maker skill forced to 1; this run's maker-skill-1 margin is +0.041.
2. **Seed 5, default map (256×160), 300 years, one thread**, run from this tree and saved at year 300. Population 104,860 and 14% food from farming, matching the step-3 seed-5 check. Steel axe is invented at year 230; iron axe at 233; copper metal is widespread only late, and no steel-capable band holds a copper axe.

The ignored test `diag_axe_breakeven` (`civ-core/src/tests_diag_axe.rs`) scores each makeable axe the way `choose_food` does: `slot_score_m` at that axe's chain skill. `diag_copper_over_steel_axe` is the older test, still scoring at maker skill 1. Where the two disagree, the chain-skill score is the one the band uses. Neither test asserts a ranking or touches a golden hash.

Break-even, in the band's own food units (person-seasons):

- **Food gain** is steel's food minus the other axe's, after maker failure. The raw gain, before failure, is almost the same, because failure is tiny.
- **Priced extra labour** is `(steel labour − other labour) × food / 0.85`. That is what `price_of` subtracts. The margin is priced labour minus food gain. Positive means the band keeps the other axe.
- **k** is food gain divided by that extra labour. Steel ties if the extra labour is scaled by k. The median is the useful figure. (The mean is not, once some bands have a negative gap: steel is then already cheaper, and one near-zero gap blows the average up.)
- **wmax** is how far steel's *own* labour must fall, as a fraction of itself, before steel ties.
- **Slack %** is the share who would take steel if labour were priced with step 5's spare-labour factor, using the spare this band already has.
- **No-depth %** drops the `(1 + 0.3 × depth)` factor on both axes. **Match-depth %** charges the steel axe the other axe's depth and leaves wear and waste alone.

## The numbers

Material scores, from the properties in the code. Durability is capped at 1, and copper, bronze and iron all sit on that cap. Steel does not.

| | cut | durability | axe score (half cut, half durability) | wear `1.15 − durability` |
|---|---:|---:|---:|---:|
| copper | 0.480 | 1.000 | 0.740 | 0.150 |
| bronze | 0.650 | 1.000 | 0.825 | 0.150 |
| iron | 0.640 | 1.000 | 0.820 | 0.150 |
| steel | 0.780 | 0.979 | 0.880 | 0.171 |

User skill at the axe is 0.996 for these bands, so the user factor is 0.998 on every metal. It cannot prefer one axe over another.

### Who holds what, among bands that can already make a steel axe

| | bands that can make steel | hold steel | hold copper | hold iron |
|---|---:|---:|---:|---:|
| Seed 6, year 350 (step-2 world, step-3 kit) | 3,234 of 3,354 | 2,209 (68%) | 772 (24%) | 253 (8%) |
| Seed 5, year 300 (grown under step 3) | 5,018 of 6,050 | 619 (12%) | 0 | 4,399 (88%) |

No bronze, flint, stone, bone or empty axe slot in either set. Carried charcoal is 0.00 on both. The metal is not the shortage: seed-6 copper bands are carrying about 0.67 copper, 0.71 iron and 0.96 steel. Seed-5 iron bands are carrying about 0.99 iron and 1.07 steel.

The seed-6 steel holders are farmers (sow 0.86, raw farm share about half). The seed-6 copper holders are not (sow 0.10, 682 of 772 have no fields). Seed 5 has few farmers at all (437 farming bands in the year-300 report), which is why iron, not steel, is the usual axe.

### Seed 6, year 350: copper held over steel (772 bands)

Every band keeps copper at chain skill. Margin is copper's score minus steel's.

| | n | food gain | labour (copper / steel) | depth | margin | extra labour priced | k median (p10 / p90) | steel labour must fall to | slack would flip | depth removed | steel at copper's depth |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| all copper holders | 772 | +0.049 | 0.0056 / 0.0113 | 3.2 / 8.1 | +0.045 | 0.094 | 0.50 (0.17 / 0.79) | 74% | 14% | 100% | 97% |
| farms | 90 | +0.124 | 0.0060 / 0.0124 | 3.1 / 8.3 | +0.022 | 0.146 | 0.89 (0.63 / 0.99) | 92% | 57% | 100% | 100% |
| not farms | 682 | +0.039 | 0.0055 / 0.0111 | 3.2 / 8.0 | +0.048 | 0.087 | 0.47 (0.17 / 0.70) | 72% | 8% | 100% | 96% |

Chain skill is 0.980 on the copper axe and 0.963 on the steel axe. Recipe skill on the steel axe itself is 0.48, and it does not enter the food term. Waste is 1.016 against 1.030. Failure is about 0.001. Spare labour is 0.31 overall and 0.16 on the farms (those bands are busy, not idle). Food per person is about 0.71.

At maker skill 1 the margin shrinks to +0.041, and 29 of the 90 farms (32%) would take steel. Those 29 are why the old diagnostic, which still scores at maker skill 1, reports 29 steel wins and a 3.9% "stale axe". The live choice uses chain skill, and under that score the stale rate is zero. The kit is not a leftover from an earlier pass.

The probe still prefers steel for 94% of these 772, by +0.030. Of that, the unweighted farm multiplier is +0.029 and plants are +0.015. Upkeep in the probe is only +0.018, fight is +0.004, protection is +0.001. The same comparison with plants and farm taken at the band's own weights is −0.007, and it prefers steel for nobody.

### Seed 6, year 350: iron held over steel (253 bands)

| | n | food gain | labour (iron / steel) | depth | margin | extra labour priced | k median (p10 / p90) | steel labour must fall to | slack would flip | depth removed | steel at iron's depth |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| all iron holders | 253 | +0.030 | 0.0100 / 0.0126 | 7.7 / 8.8 | +0.018 | 0.048 | 0.61 (0.32 / 0.90) | 92% | 23% | 96% | 44% |
| farms | 81 | +0.056 | 0.0102 / 0.0133 | 7.6 / 9.0 | +0.015 | 0.070 | 0.83 (0.65 / 0.97) | 95% | 42% | 100% | 83% |
| not farms | 172 | +0.018 | 0.0099 / 0.0122 | 7.7 / 8.6 | +0.019 | 0.037 | 0.50 (0.26 / 0.75) | 90% | 14% | 95% | 26% |

Iron is already close to steel as a tool, and the chains are almost the same length. Matching the depth only flips 44%, because what is left is one extra step, steel's slightly worse wear, and a little waste. The probe's value gap is +0.013, past the 0.02 tie for only 4% of these bands. This pair is a real preference for iron. It is barely a "worse held" pair.

### Seed 5, year 300: iron held over steel (4,399 bands)

| | n | food gain | labour (iron / steel) | depth | margin | extra labour priced | k median (p10 / p90) | steel labour must fall to | slack would flip | depth removed | steel at iron's depth |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| all iron holders | 4,399 | +0.019 | 0.0099 / 0.0124 | 7.6 / 8.6 | +0.024 | 0.042 | 0.43 (0.26 / 0.81) | 89% | 12% | 99% | 11% |
| farms | 163 | +0.035 | 0.0105 / 0.0134 | 8.2 / 9.6 | +0.012 | 0.047 | 0.85 (0.52 / 0.97) | 95% | 53% | 100% | 94% |
| not farms | 4,236 | +0.018 | 0.0098 / 0.0124 | 7.5 / 8.6 | +0.024 | 0.042 | 0.43 (0.26 / 0.76) | 89% | 10% | 99% | 8% |

Same mechanism, a younger and less farmed world, so the food gain is smaller and matching depth does almost nothing for foragers. The 163 farmers are again the close call: slack pricing or equal depth would flip about half to all of them. The probe prefers steel for 1% of the 4,399. Chain skill is 0.99 on iron and 0.98 on steel. This is not bands that have not learned to smith.

The 619 bands that do hold steel rank it first against the next axe they can make (iron for 532, stone for 87), by a margin of about 0.07. Stone wears fast, so for some foragers steel is cheaper labour as well as more food. They are not refusing steel in general. They refuse it when iron is the alternative.

## How the choice is made

An axe's material score is half cutting quality and half durability. Steel wins on the edge and loses a little on durability, because durability rewards toughness and malleability and then caps at 1. Copper's malleability puts it on the cap.

```511:517:civ-core/src/effects.rs
fn axe_term(it: Item) -> f32 {
    if it == NONE {
        0.0
    } else {
        let m = mat(it);
        0.5 * cut_q(m) + 0.5 * durability(m)
    }
}
```

That score, times the user factor, is added at 0.1 on plants and 0.2 on fields. It does not touch game, fish or small game. Recipe skill is not in these lines. A knife is a different story: the cut term is 0.35 on game and on fields, and a hunting blade is larger still. That is why iron knives gave way to steel in step 2, while axes did not. The axe simply does not move enough food.

```553:565:civ-core/src/effects.rs
fn yields_base(a: &YArgs) -> [f32; 5] {
    let plants = 1.0 + 0.35 * a.dig + 0.3 * a.vq + 0.1 * a.hammer + 0.1 * a.axe;
    // ... game, fish, small game: no axe ...
    let farm = 0.5 + 0.3 * a.dig + 0.35 * a.cut + 0.2 * a.axe;
    [plants, game, fish, small, farm]
}
```

The band turns the multiplier change into food with its own harvest weights, then subtracts the food that the upkeep labour would have grown. `WORK_SHARE` is 0.85. Labour in this formula is a fraction of the work pool, and the price is last season's food per unit of working labour. Spare hands are not in it. A busy band and an idle one pay the same price per unit.

```787:794:civ-core/src/effects.rs
fn price_of(food: f32, labour: f32, view: &BandView) -> f32 {
    let pop = view.pop.max(1.0);
    let units = labour * LABOUR_SHARE * pop;
    let price = view.food.max(FOOD_EPS) / (WORK_SHARE * LABOUR_SHARE * pop);
    food - price * units
}
```

Upkeep itself is wear times a depth factor. The economy deducts this same number. Depth is one plus the longest chain under the recipe (`knowledge.rs` around the `depth + 1` stored on the toolkit), not a count of seasons the tool has been used.

```360:373:civ-core/src/effects.rs
/// One item's upkeep in the economy's units (a fraction of the labour pool, before
/// `* LABOUR_SHARE * pop`). ...
/// Anything else is `sealed * rate * (1.15 - durability) * (1 + 0.3 * depth)`.
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

The rate is `Params::upkeep`, 0.02. Pitch glue, when it is in use, multiplies both axes by 0.9, so it does not change which one wins. For a typical seed-6 copper band the depth factors are about `1 + 0.3×3.2 = 1.95` and `1 + 0.3×8.1 = 3.42`, a ratio of 1.75. Wear adds another 0.171/0.150 = 1.14. Together they double the labour (0.0056 against 0.0113), which matches the table.

Maker skill scales that labour by waste, and scales the food the band is credited with by the chance the tool fails. At skill 1 both factors are exactly 1.

```83:96:civ-core/src/effects.rs
pub fn maker_waste(m: f32) -> f32 {
    1.0 + MAKER_WASTE * (1.0 - m)
}
pub fn maker_p_fail(m: f32) -> f32 {
    let g = 1.0 - m;
    MAKER_FAIL * g * g
}
```

`MAKER_WASTE` is 0.8 and `MAKER_FAIL` is 0.25. The kit choice applies both on the current side only. The "full skill" side, used for trials, keeps the undiscounted food and the unwasted labour.

```1027:1031:civ-core/src/effects.rs
        if m_item == 1.0 {
            return (price_of(cur_food, labour, view), price_of(full_food, labour, view));
        }
        let pm = maker_p_fail(m_item);
        (price_of(cur_food * (1.0 - pm), labour * maker_waste(m_item), view), price_of(full_food, labour, view))
```

`KIT_UPKEEP` (0.06, `band.rs:734`) is not this price. The comment there says food slots rank by `tool_upkeep`, and `KIT_UPKEEP` is only the stand-in for boats, bows, shields and the probe. Changing `KIT_UPKEEP` would not move an axe. The fight term on an axe (`0.3 × axe score`, `effects.rs` around line 836) is computed and then ignored. `Effects.fight` is documented as informational.

## Why the probe says steel anyway

`true_value` (`probe_diffusion.rs`, the ignored probe) adds the five food multipliers and digest with no harvest weights, subtracts `KIT_UPKEEP` times wear times depth times maker waste, and adds `0.1` times fight. Food is also multiplied by the worst food-slot keep, which here is about 0.997 and changes nothing.

On the seed-6 copper bands the unweighted farm line alone is +0.029 of the +0.030 value gap, while those bands' farm weight in the decision is 0.018 (0.157 on the 90 farms). The probe charges about 0.018 for a chain the decision prices at 0.094, because 0.06 of multiplier is not the same unit as food per working person. Fight adds +0.004, which the band does not read. So the probe overstates the food of a band that does not farm, and understates the labour of a deep chain, and then reports the held copper axe as worse. That is the 23% pair in the step-3 probe. The iron pair is mostly inside the probe's own tie (gap about 0.013 against a tie line of 0.02) even though the band's margin is a real +0.018 to +0.024.

The probe's "worse by own valuation" column is a different number. Since step 3 it scores the changed slot with `slot_score_m` at chain skill, which is the choice itself, so that column stays near zero by construction. "Worse held" is the yardstick that disagrees.

## Options

No option is implemented. Effects on population and chain depth are reasoned from the mechanism, not from a new run.

### 1. Leave it as it is

No code change.

What stays: foragers with a copper or iron axe keep it while steel is makeable. Farming bands on a developed world already take steel (2,209 of them on the seed-6 save). The close calls stay close: 90 farming copper bands on that save, and 163 farming iron bands on seed 5, each within roughly a tenth of flipping.

Why that can be the right outcome: the steel chain is eight or nine recipes deep, and the axe's food coefficients are 0.1 and 0.2. A band that mostly gathers plants is being asked to maintain a smith's chain for a few percent more plant food. Refusing is what the food-minus-labour rule was built to do. Steel not lasting longer than copper is also already in the materials: malleable copper wears less than hard steel. Leaving it does not hide a stale kit or a missing user factor.

Effects: none, on axes, other tools, food, population or the incentive to shorten a chain. The probe will keep listing copper-over-steel as the top "worse held" pair until the yardstick changes (option 4) or the worlds become farmed enough that those bands take steel on their own.

Risk: if the depth charge was meant as a rough stand-in and not as a real seasonal cost, the sim will keep under-using steel tools whose food term is small (axes more than knives). Later steps can still move the close cases. They will not, on these numbers, move the foragers. See option 2.

### 2. Let steps 4, 5 and 6 move it, and change nothing now

No code this step. This is the path the proposal already describes.

**Beliefs (step 4, proposal 3.5).** The decision stops reading the true food and reads a belief built from priors and from what the tool paid out. The steady state does not flip if the belief converges to the true food gain and the true labour: that is the ranking above, and it already favours copper and iron for these bands. What changes is the path. A prior from the visible edge ("steel looks sharper") can start a trial; a few bad or expensive seasons can write the axe off; testimony and inheritance can bring it back. Two bands with the same chain can then hold different axes for a while. That is the point of beliefs. It does not make a forager's steel axe pay for itself. It does depend on one design choice that is not settled by these runs: whether the labour in the belief includes chain depth, or only wear the band can see. If depth never enters the belief, bands will keep a steel axe the economy is quietly charging them for.

**Labour price and risk (step 5, proposal 3.2 and 3.6).** The price of labour becomes last season's food per unit of labour, times `LAB_FLOOR + (1 − LAB_FLOOR) × (1 − spare)`, with `LAB_FLOOR` at 0.2. Spare is the share of the work pool not already used to feed people or maintain tools (`world.rs`, the `spare` line in `economy`). Doing only that, on these bands' actual spare, flips:

- 14% of seed-6 copper holders, and 57% of the 90 farms (spare 0.16, so the discount is small, but they were already close);
- 8% of the seed-6 foragers (spare 0.33, a bigger discount, but the food gain is only 0.039 and the median k is 0.47);
- 12% of seed-5 iron holders, and 53% of the 163 farms.

Fully idle bands (spare 1) would pay 0.2 of today's price and almost all of them would take steel. These bands are not idle. The risk term cuts the other way: a tool that sometimes fails is worth less when stores are thin. Practised steel fails less than 1% of the time, so the penalty is small next to the labour gap. It matters for a first trial of an unpractised chain, not for the 0.96 chain skill in these tables.

The snippet, if this part of step 5 were turned on by itself and nothing else:

```rust
// Not applied. BandView does not carry spare today; Derived::spare is the
// saved fraction of the work pool left after feeding and upkeep.
// Scaling the labour argument scales only the subtraction inside price_of.
let slack = 0.2 + 0.8 * (1.0 - spare);
let priced = price_of(food, labour * slack, view);
```

Effects of doing it now, rather than at step 5: the farming holdouts split, most foragers still refuse, and every other food tool gets the same discount. Deep tools (steel knives, which already win, and anything else with a long chain) become a bit more attractive, more so for busy farmers than for idle foragers, which is the opposite of "idle hands make light of upkeep" only in degree: farmers are busier, so they discount less, but their farm weight is what makes the axe worth it. Population would move only as far as those extra tools move the harvest. Not measured here.

Risk of doing it now: step 5's price is supposed to sit beside the risk penalty and the trial rule. Alone, it makes high-upkeep tools cheaper for everyone with any spare (the floor is 0.2 only at full spare; at spare 0.3 the factor is about 0.75) without the counterweight of going short. Pulling it forward also makes step 5's own before-and-after harder to read.

**Materials and fight (step 6, proposal 3.2, 3.5, 3.9).** A shadow price charges the materials a tool draws, scaled by how badly the band wants them elsewhere and by how many other recipes use them. These holdouts are not short of iron or steel (stocks around 0.7 to 1.1). Charcoal, which the steel chain burns, is carried at 0.00, so a shadow price on charcoal could make steel *more* expensive, not less. That direction is not measured: the diagnostic did not sum marginal draws. Do not assume step 6 rescues the steel axe.

Fight is the part that can. The axe's fight term differs by `0.3 × (0.880 − 0.740) = 0.042` against copper, and by 0.018 against iron. The proposal adds `fight × fight_need × FIGHT_W × need`, with `FIGHT_W` 0.3 and `need` about one person's food times the band's population (`world.rs`, `need = pop * (1 + cold)`). At the base fight need of 0.1 and a band of about 20, the copper gap of 0.045 gets a push of roughly 0.025. That is half the margin, not all of it. A band that is actually pressed (`fight_need` near 1) would take the steel axe for the fight alone, and the food loss would be the price of arming. Peaceful foragers would not. This is the proposal's formula applied to these margins, not a simulated fight.

Effects if step 6 lands as written: some threatened bands switch to steel axes (and to steel spears, which the fight term hits harder). Peaceful foragers keep copper or iron. Chains get a new reason to be deep when the band expects a war, and a new reason to stay short when charcoal or iron is spoken for. Population moves with who is fighting, not with a global axe upgrade.

Risk: `FIGHT_W × need` is large if `need` stays at the whole band's food. A modest fight need could then swamp a food gap of 0.05 and turn every axe choice into a weapon choice. That constant is marked for tuning. Fight beliefs also start as a guess, so early switches can be wrong and then stick until a clash teaches otherwise.

### 3. Stop charging the whole chain as seasonal upkeep

The lever that actually flips the measured bands. Today both the decision and the economy use:

```rust
// effects.rs, tool_upkeep. The 0.3 is the whole copper-over-steel result.
sealed * rate * (1.15 - durability(mat(it))) * (1.0 + 0.3 * depth as f32)
```

Two variants, neither applied:

```rust
// A. Wear only. Depth is no longer a seasonal cost.
sealed * rate * (1.15 - durability(mat(it)))

// B. Keep depth, but give steel the incumbent's depth inside the axe choice.
//    The economy would still have to charge the same number, or the band
//    adopts an axe and then pays a bill the choice hid.
let depth = if slot_is_axe { incumbent_depth.min(steel_depth) } else { steel_depth };
```

On these runs, A (dropping the depth factor on both axes) flips 100% of the copper holders and 96–99% of the iron holders, farms and foragers alike. B (steel charged at the other axe's depth) flips 97% of copper holders, because 3 steps against 8 is the whole gap, but only 44% of seed-6 iron holders and 11% of seed-5 iron holders. Their chains are already the same length. What remains is steel's worse wear.

A durability-only change does not do A's job. Steel's wear is 0.171 against copper's and iron's 0.150, so it is 14% higher. Putting steel on the same cap lowers its labour to about 88% of today's (1/1.14). Copper foragers need it down to about 72%, so they stay on copper. The farming copper bands need about 92%, so 88% is enough to flip the average one. Iron holders need about 89–95%: seed 6's average (92%) would flip, seed 5's average (89%) sits on the cut. That is the average break-even, not a re-score of each band. Making steel last *longer* than copper means a new rule, not the current `durability` function: copper is already at the cap, and the cap is what malleability buys. The properties comment says steel keeps a keener edge. The wear formula does not read the edge at all.

```537:539:civ-core/src/materials.rs
pub fn durability(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + 0.5 * p.malleable) / 12.0).clamp(0.05, 1.0)
}
```

Effects of A, because `tool_upkeep` is the labour the economy deducts as well as the price: every deep food tool gets cheaper to own, not just axes. Steel knives, which already win, win by more. Bands take the best material even when the chain is long, so the incentive flips from "keep the chain short" to "length does not matter, only the edge and the wear". Food and population should rise where those tools were the binding constraint, and chain depth stops being a reason to specialise in a simpler craft. Not measured. Boats, bows and shields would not move unless `KIT_UPKEEP`'s copy of the same depth factor is changed too (`nonfood_scores` in `effects.rs`). The probe would also need that copy changed, or it will start calling the new choice worse.

Effects of a "steel lasts longer" rule aimed only at axes: a smaller switch, concentrated on the farming holdouts, and a new special case that other hard-but-not-malleable tools will want next. If the keen edge is the thing that should last, the wear term has to read sharpness or a kept-edge property for every tool, not an axe exception.

Risk: A will also cheapen deep tools that are genuinely costly to keep (a chain that really does have to be re-smelted when the head wears out). The seasonal bill and the one-time cost of learning the chain get blurred further. B, if applied only in the scorer, desynchronises the choice from the economy. Either variant changes adoption of every metal tool, so the step-3 population and farm-share guard rails are no longer a baseline. That is a behaviour change and wants its own step, not a quiet edit.

### 4. Fix the probe's yardstick only

No simulation change. `true_value` in `probe_diffusion.rs` would weight the plants and farm multipliers by the band's harvest shares instead of adding the farm line in full. Upkeep, fight and protection stay as they are. The diagnostic already sketches this: for every copper and iron holdout above, that reweighted gap is negative, and the share the probe would call "steel is better by more than 0.02" is 0. The unweighted probe says 94% on seed-6 copper and about 1–4% on iron.

A sketch, not the probe code:

```rust
// Replace the unweighted (plants_m + farm_m) delta inside true_value's food.
// w_plant and w_farm are the band's harvest weights, the same ones price_of uses.
// They sum with the other three resources to 1, so a farm line no longer counts
// as a full multiplier for a band that does not farm.
let food_delta = (w_plant * d_plants + w_farm * d_farm) * keep;
```

Effects: copper-over-steel should drop out of "worse held", or shrink to the rare band whose weighted food really does clear the upkeep and the tie line. Iron-over-steel is already mostly a tie and would stay one. Axes, food, population and chain depth do not move. Other pairs that look bad only because of an unweighted line (the fibre net is the obvious one to re-check) will move too. The 0.02 tie threshold stops meaning the same thing if the food term changes scale, so the threshold has to be looked at in the same change.

Risk: low for the simulation, real for the history of the probe. Step-2 and step-3 "worse held" percentages would not be comparable after this, which is already true once between step 2 and step 3 because waste and failure entered `true_value`. The sketch mixes units on purpose (weighted multiplier deltas against an unweighted `KIT_UPKEEP`). A cleaner yardstick is the band's own person-seasons, but then "worse held" collapses to zero whenever the choice is working, and the probe loses the ability to say the choice is locally right and globally odd. The useful fix is the middle one: weight the food, keep a labour term, and accept a new baseline.

## Recommendation

Leave the simulation as it is. The held axe is the one the band's own score selects, in all 772 + 253 + 4,399 cases, and the reason is a real one: the steel chain costs more food than the axe returns, except for farmers on a world where steel is already in their hands. Cheapening every deep chain to correct a probe label would throw out that reason for knives, axes and every other food tool together.

What is worth doing, when the probe is next touched, is option 4: stop counting a full farm multiplier for bands that do not farm. Until then, read "copper axe over steel axe, 23%" as "the yardstick disagrees with the band", not as "the band is wrong".

Let steps 4–6 land on their own. From these numbers they should move the close farming cases (step 5's spare price flips about half of them; step 6's fight term flips bands that are actually under pressure) and should not move foragers, unless the labour belief is blind to chain depth or fight need is large enough to swamp a gap of 0.05. Both of those are tuning choices, not bugs to fix in advance.

Do not change `KIT_UPKEEP` for this. The axe decision does not read it.

## Questions

1. Is a forager keeping copper or iron, while a steel axe is makeable, the outcome you want? On these runs that is most of the holdouts, and the food gain really does not cover the chain.
2. The farming holdouts are within about 8–15% on steel's labour (90 copper bands on seed 6, 163 iron bands on seed 5). Should "not quite" stand, or should those bands take steel?
3. Should chain depth be a cost paid every season the tool is owned, or only a cost of learning it and a risk of failing to make it? Dropping the seasonal depth factor flips nearly every measured holdout, and it cheapens every other deep food tool too.
4. Steel is slightly less durable than copper, because malleability caps copper at 1 and the wear formula ignores the edge. Should a kept edge last longer? Matching copper's wear is probably enough for the farmers and not for the foragers.
5. Should the probe weight the farm line by the band's harvest before step 4, so this pair stops heading "worse held"? That changes no band's axe.
6. When step 4 prices labour from beliefs: should that belief include the chain's depth, or only wear? If depth is invisible, bands will keep steel axes the economy is still charging them for.
