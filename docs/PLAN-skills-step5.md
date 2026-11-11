# Plan: skills step 5 (Layer 2: risk and trials)

Implementation plan for row 5 of `docs/PROPOSAL-skills.md`. No code in this change. Line numbers are main `3b2a515` (this branch). Step 4 will move some of them; re-find the named functions after rebasing onto `skills-step4`.

This step does **not** touch material shadow prices, `fight_need`, fight beliefs, arms races, or raiding (steps 6 and 8). It does **not** change `axe_term`, `KIT_UPKEEP`, the depth term inside `tool_upkeep`, or any `MAKER_*` constant. See the question for Michael at the end.

---

## 1. Goal and pass check

A band stops flipping a coin at 20% (`KIT_TRIAL_P`) to try a tool that cannot hurt its food. It weighs what it **believes** the tool will do against the chance of going short, and it tries the tool only when the expected gain, including what it would learn, is worth that risk.

In plain words:

- Labour is cheap when the band had spare time last season, and dear when it was busy. The price is last season's food-per-labour (the number the kit choice already uses) times a spare factor. At no spare time the factor is 1, so a busy band prices labour exactly as today.
- The band's picture of the season is a mean food `mu` and a spread `sigma`. `P_short` is the chance that food plus usable stores falls below `SHORT_LINE` of need. Thin stores and food near need make a little extra spread costly. Full stores make `P_short` about 0.
- Ambition shrinks that penalty. It never removes it.
- Curiosity adds the value of a season of practice and the value of finding out (doubt about the challenger's mean). A faded, written-off tool becomes worth a re-check because its doubt grew back. That re-check still pays the risk penalty.
- The chance of actually trying is a logistic of (utility of trying minus utility of not trying, plus the value of learning), with a floor so a desperate band still tries sometimes, and a ceiling so it does not try every season.
- A trial really mixes food: `TRIAL_SHARE` of that slot's work uses the challenger, at true skill. A bad trial costs food. The incumbent's upkeep stays, and the challenger still wears, as it does today.
- The kit is chosen in one pass. A new tool replaces the old one only when its utility beats the old one by the one-off labour of making it, and only after the band has used it itself for `BELIEF_ADOPT_N` seasons. Until then the choice can only start a trial.
- Hunger still raises invention. The experiment line is not edited.

**Pass check** (proposal section 5, row 5):

- Trials fall as `P_short` rises. The counters show it (`trials_declined_risk` is the roll that would have tried without the risk penalty and did not try with it). On the diffusion probe, trial rate in the high `P_short` band is below the low band. If a full run goes the other way, tune `RISK_*` and `TRIAL_*` only.
- A bold band, or a band with full stores, takes a risky trial that an otherwise identical hungry band with thin stores declines. Locked by a unit test, not by a world seed.
- Bench at most +5% versus the step-4 binary (not versus main; step 3's chain fill is not this step's to claw back).
- Guard rails, unchanged: no crossing before Y100; (a) at most 6/10; (b) under 7/10; war rate not doubled versus D1–D6 ((a) 0.320, (b) 0.271 %/y); no population collapse (no seed down more than about 20% without a clear cause). (b) seed 499246298 may die early and counts in the x/9 columns.

---

## 2. What this plan assumes from step 4

Read on 2026-11-11 from branch `skills-step4` (worktree `/workspace/bots/build-bot/kindling-step4`). Not edited. Commits on top of main:

- `5db6530` belief table (priors, update, fade, eviction)
- `2b2edbe` founder and old-save belief fill

`PLAN-step4.md` boxes C–G were still open: payout updates, inheritance, testimony, and "layer 2 reads beliefs" are **not in the tree yet**. `civ-core/src/lib.rs` on main has no `beliefs` module. Step 5 starts only after step 4 has merged, or rebases onto it.

### Interface this plan treats as fixed

From `civ-core/src/beliefs.rs` on that branch:

- One `Belief` per `(item, slot)`: `n`, `mean`, `var`, `fail`, `labour`, plus `flags` (`FLAG_OWN` = 1). Fight fields stay 0 until step 6. Step 5 does not write them.
- `BELIEF_ADOPT_N = 1`, `BELIEF_NMAX = 8`, `BELIEF_CAP = 24`, `PRIOR_N = 0.5`, `INFO_W = 0.3`, `LC_RATE = 0.2`.
- `food_prior`, `belief_from_prior`, `expected_multiplier`, `material_term`, `update_belief`, `observe`, `blend_learn_curve`.
- `LearnState { curve, first_mean }`. `first_mean` is step 4's problem (not a saved field). Step 5 reads the band's saved `learn_curve` only.
- `Band::learn_curve` starts at `LC_PRIOR` (1.3, `effects.rs`).

From step 4's own plan for the box that is not written yet (E), which section 3.6 says step 5 builds on:

- Food-slot choice uses the belief mean turned into food the way layer 1 turns a material term into food, minus believed labour. No belief: use `food_prior`, do not store it.
- A full switch only if own use has reached `BELIEF_ADOPT_N`. Until then the slot can only trial. Step 4 still rolls `KIT_TRIAL_P` for that trial. **Step 5 deletes that coin flip.**
- The decision path does not call `tool_effects`. The economy still pays from layer 1 (invariant 1). A trial's food mix is economy code, so it may call `food_mults` / `yields`. The chooser may not.
- A trial observation's weight is `TRIAL_SHARE` (proposal 3.5). Step 4 may have hardcoded `0.25` before the named constant exists. Step 5 defines `TRIAL_SHARE` and points that weight at it if step 4 left a literal. Do not change the update formula otherwise.
- Own-use seasons. Prefer step 4's predicate if it has one. If it does not, use `(n - PRIOR_N) >= BELIEF_ADOPT_N` and `flags & FLAG_OWN != 0`. A prior alone (`n = PRIOR_N`, no `FLAG_OWN`) is not enough to adopt.

`learn_curve` in the value of learning (proposal 3.6): while `n < BELIEF_NMAX`, the challenger's believed potential mean is `mean * learn_curve`. At the cap, potential is `mean`.

### If step 4 lands differently

- If the decision still ranks by `slot_score` / true effects, stop. Do not build a second valuation beside it. Wait for step 4's belief chooser, then add risk on top of that function.
- If the belief-to-food conversion is inlined, extract it once as `belief_slot_food` without changing its arithmetic, and call it from both sites.
- If step 4's adopt test uses raw `n >= BELIEF_ADOPT_N` rather than `n - PRIOR_N`, keep step 4's test. Do not "correct" it here.
- If `learn_curve` is not per band, use the constant `LC_PRIOR` and say so in the progress note. The explore term still runs.
- If `INFO_W` moved module, use it where it lives. Do not copy the constant.
- If step 4 moved `FIXTURE_V10_HASH.0`, stop. That hash is not step 5's to update.
- If step 4 put the chooser in a new module, put the step-5 pass in that module. Do not leave a second chooser in `effects.rs`.

---

## 3. Files and functions

| Where | What changes |
|---|---|
| `civ-core/src/beliefs.rs` (step 4's module) | Pure functions: `phi`, `p_short`, `risk_penalty`, `explore_value`, `trial_chance`, `labour_price`. Constants from section 6 that are not already there. This module already must not call `tool_effects`. |
| `civ-core/src/effects.rs` | The band's kit pass (`choose_food` ~1132, `make_kit_scored` ~1221, `write_choice` ~1104, `price_of` ~789). Nominal `make_kit` / `make_kit_with` stay on true food-minus-labour with no risk and no trial. |
| `civ-core/src/world.rs` | `fit_kit` ~1686, `refresh_toolkit` ~1693, `decision_view` ~1661, `economy` trial roll ~1903–1919 and `food_mults` ~1970, `band_season` ~2077 (counters, same place as `make_failures` at 2205), `year_stats` ~4481. Invention line ~2237 and `attention` hunger ~2338 are **not edited**. |
| `civ-core/src/band.rs` | Delete `KIT_TRIAL_P` (~736). Optional `trial_p` lives on `Derived`, not on `Kit`, so `Kit` literals do not break. `KIT_UPKEEP` (~734) stays. |
| `civ-core/src/world/fx.rs` | One `f32` and two counters on `Fx`, applied in band order. Not a new event op. |
| `civ-core/src/probe_diffusion.rs` | Fill the three `trial_rate_pshort_*` columns that are hardcoded `NA` (~698). `slot_own` (~226) must score the marginal the new pass ranks by, or "worse by own valuation" stops being ~0. Probe only. |
| `civ-core/src/tests_skills.rs` | New decision tests. `payout_invariant` (~127) reconstructs trial wear with `KIT_TRIAL_P` (~191); point it at `trial_p`. |
| `civ-core/src/tests_diffusion.rs` | Rewrite `d3_trials_about_one_season_in_five` (~130). |
| `civ-core/src/tests.rs` | `skills_unfilled_fields_stay_zero` (~3001): the step-5 counters and the three `YearStats` fields leave the "must be 0" list. `switches_conserve` and `switches_fight` stay 0. |
| `docs/DATA_SCHEMA.md` | Say what the step-5 counters and `YearStats` fields mean now that they are filled. Schema version stays 1. |
| `docs/SAVE_FORMAT.md` | One sentence: step 5 adds no field and no migration. |
| `docs/PROGRESS-skills.md` | The implementer fills this when the code lands. Not in the plan commit. |

`want` (`world.rs:892`) is step 6's shadow price. Do not call it from the step-5 chooser.

### 3.1 Labour price

Today (`effects.rs:789`):

```rust
fn price_of(food: f32, labour: f32, view: &BandView) -> f32 {
    let pop = view.pop.max(1.0);
    let units = labour * LABOUR_SHARE * pop;
    let price = view.food.max(FOOD_EPS) / (WORK_SHARE * LABOUR_SHARE * pop);
    food - price * units
}
```

`price` is food per unit of working labour, and `units` is the tool's upkeep in those units. That `price` is the **full** price. Step 5 multiplies it by the spare factor and does not change the divisor. Replacing the divisor with `LABOUR_SHARE * pop` (dropping `WORK_SHARE`) would shrink every labour gap, including copper axe versus steel axe, by about 15% even when the band is fully busy. That is Michael's open decision. Do not do it.

```rust
/// `full` is today's `view.food / (WORK_SHARE * LABOUR_SHARE * pop)`.
/// `spare` is last season's `b.d.spare` (saved on `Carry`).
pub fn labour_price(full: f32, spare: f32) -> f32 {
    let s = spare.clamp(0.0, 1.0);
    let factor = LAB_FLOOR + (1.0 - LAB_FLOOR) * (1.0 - s);
    full * factor
}
```

At `spare == 0` the factor is 1 and `price_of` is bit-identical to today. At `spare == 1` labour costs `LAB_FLOOR` (0.2) of the full price. The same factor applies to every slot, including the axe. Believed labour (the belief's `labour`, or step 4's believed upkeep) is what the decision multiplies. The economy still charges true `tool_upkeep`, including maker waste. Those are different numbers on purpose: the band decides on what it believes the tool costs; the season charges what the tool costs.

Non-food slots keep `KIT_UPKEEP` in their ranking (boats, bows, shields). Step 5 does not retune that constant. Their trial decision still sees the food-slot labour price, because a trial's risk is about food. Their adoption score does not grow a fight term (step 6).

### 3.2 `P_short` and the risk penalty

```rust
/// Standard normal CDF. Abramowitz and Stegun 7.1.26 on `erf(z / sqrt(2))`.
/// Pure f32. |error| under 1.5e-7. Non-finite z → 0 or 1.
pub fn phi(z: f32) -> f32 { /* rational; no new crate */ }

pub fn p_short(mu: f32, sigma: f32, need: f32, store: f32) -> f32 {
    let gap = SHORT_LINE * need - mu - STORE_USE * store;
    if !(sigma > 1.0e-4) {
        return if gap > 0.0 { 1.0 } else { 0.0 };
    }
    phi(gap / sigma).clamp(0.0, 1.0)
}

pub fn risk_penalty(p_short: f32, need: f32, ambition: f32) -> f32 {
    RISK_STARVE * need * (-RISK_BOLD * ambition).exp() * p_short.clamp(0.0, 1.0)
}
```

**What `mu` is.** Person-seasons the band expects to eat, not the marginal `food_acc` alone. `food_acc` (`effects.rs:482`) is the extra food of one slot against an empty slot, linear in the yield ratio. The economy's harvest is concave, so a sum of marginals overstates the season. Calibrate off the band's own last season, which is saved:

```rust
// mu_raw(kit) = base_food(empty payoff slots) + sum over slots of belief_slot_food
// For the kit the band is holding, force the scale so mu equals last season's eaten food.
let scale = view.food.max(FOOD_EPS) / mu_raw(current_kit).max(FOOD_EPS);
let mu = scale * mu_raw(option);
```

`view.food` is `b.d.food` (`Carry`, `band.rs:410`). `need` is `b.d.need`. `store` is `b.store`. An unused tool never enters this: `belief_slot_food` reads the belief or the prior, and the scale reads last season. If step 4's conversion is already in eaten-food units and `mu_raw(current)` matches `view.food` within a few percent, set `scale = 1` and leave a comment saying why. The unit test below is the check.

**What `sigma` is.** In food units:

```text
sigma^2 = (BASE_SD * mu)^2
        + sum over slots of food_var(slot)
food_var = (df/dmean)^2 * (var + var/n) + fail*(1-fail)*(food - fallback_food)^2
```

`var` and `n` and `fail` are the belief's. `n` is at least `PRIOR_N` for a prior, so `var/n` is finite. Floor `var` at 0. If `belief_slot_food` is linear in `mean`, `df/dmean = food/mean` when `mean` is not ~0; otherwise one finite difference, cached on the candidate. A trial slot uses the mix in section 3.5, with variances combined by the **share**, not the share squared (a wide prior must still be able to scare a hungry band; `TRIAL_SHARE^2 = 0.0625` would hide it). Same-season independent noise would use the square. That alternative is an open question, not the default.

`fallback_food` is the runner-up already stored on `Kit::fallback` (step 3), valued with its belief. No runner-up: the empty-slot food, so a failure drops the slot to nothing.

The incumbent kit's `P_short` (no trial mixed in) is the number that goes into `YearStats`. The trial decision uses `P_short` of the mixed kit. Compute the incumbent once per band per choice, then adjust the one slot under consideration. Do not resimulate `economy`.

### 3.3 Explore value

```rust
/// `gain` = (U of the challenger at its potential mean) - (U of the incumbent now),
/// before explore is added, floored at 0. `food_slot` is the challenger's believed
/// food. `var_over_n` is `var/n` of the challenger (doubt about the mean).
pub fn explore_value(curiosity: f32, gain: f32, var_over_n: f32, food_slot: f32) -> f32 {
    let info = var_over_n.max(0.0).sqrt() * food_slot.abs();
    curiosity.max(0.0)
        * EXPLORE_HORIZON
        * (EXPLORE_W * gain.max(0.0) * USE_PRACTICE + INFO_W * info)
}
```

`USE_PRACTICE` (0.08) and `INFO_W` (0.3) already exist. Potential mean, only inside this gain and only for choosing the challenger: `mean * learn_curve` while `n < BELIEF_NMAX`, else `mean`. The utility of **not** trying uses the plain mean. Explore is added to the trial, never to a settled adoption.

### 3.4 Trial chance

```rust
pub fn trial_chance(d: f32, need: f32) -> f32 {
    let width = (TRIAL_TEMP * need.max(0.0)).max(1.0e-3);
    let x = (d / width).clamp(-40.0, 40.0); // logistic, no overflow
    let s = 1.0 / (1.0 + (-x).exp());
    TRIAL_MIN + (TRIAL_MAX - TRIAL_MIN) * s
}
```

`d = U(trial mix) - U(no trial) + explore_value(...)`. Both utilities include `E[food] - labour * labour_price - risk_penalty`. They do not include material shadow prices or fight (step 6). `U(trial mix)` uses `mu` and `sigma` with `TRIAL_SHARE` of this slot on the challenger. One `phi` per utility, so two per candidate, and the no-trial utility is shared by every candidate in the slot.

Who may be the challenger (proposal 3.6): among the candidates that are not the incumbent, the one with the highest potential utility plus the uncertainty term, but only if that beats the incumbent's utility **before** the risk of mixing. At most one. `KIT_CANDIDATES` (3) limits who is scored: the 3 best **makeable** items by believed food (`belief_slot_food` at the plain mean), plus the current item if it is not already in those three. Ties keep the earlier item id, as `consider` does today (`effects.rs:1084`).

The roll stays the existing deterministic `hash3`. Same salt, same arguments (`world.rs:1910`):

```rust
let roll = hash3(seed ^ 0x7121_A15E_D3D3_0001, b.id as i64, (tick as i64) * 16 + j as i64);
let u = (roll >> 40) as f32 / (1u64 << 24) as f32; // in [0, 1)
```

Try if `u < trial_p` (today: try if `u < KIT_TRIAL_P`, written as `u >= KIT_TRIAL_P` then `continue`). No new `Rng` draw.

`trials_declined_risk`: also compute `p_safe` with the risk penalty removed from `d` (same explore, same food). If `u < p_safe` and `u >= trial_p`, the band would have tried and the risk penalty talked it out. One roll, two thresholds.

### 3.5 Trial food risk (the economy, true effects)

Today a fired trial adds the challenger's upkeep and marks it used, and the food multipliers stay the incumbent's (`world.rs:1903–1919`, then `food_mults` at ~1970). Proposal section 2: the trial cannot go wrong. Step 5 makes it able to.

Keep the extra upkeep (the tool is made and worn; `payout_invariant` depends on this). Do **not** scale that upkeep by `TRIAL_SHARE`.

For food, mix the **slot term**, not the finished multipliers. Interactions (cut × hunt, bow × game) then apply once:

```rust
fn mix_term(inc: f32, ch: f32) -> f32 {
    (1.0 - TRIAL_SHARE) * inc + TRIAL_SHARE * ch
}
```

Only on slots whose roll fired, and only for this season's `food_mults` inside `economy`. Do not write the mix into `Kit`'s stored `plants_m` / `game_m` / … (`apply_base` stays the adopted kit). Novice noise stays per slot (`novice_factor`); two materials of one form share a user skill and so share the draw.

`trial_food_lost`: after `e_food` is chosen on the **mixed** multipliers, evaluate `food_of` once more at that same effort with the unmixed multipliers. Add `max(0, unmixed - mixed)` to the counter, times 1000, rounded like `make_scrap` (`world.rs:2181`). Not a second effort search. A trial that pays more adds 0.

The belief update (step 4) sees the true payout and should weight it by `TRIAL_SHARE`. Step 5 does not reimplement the update. If step 4 reads "was this a trial?" off `Kit::trial` plus the roll, keep that. The stash in section 3.6 makes the fired set available without a second roll.

### 3.6 Kit choice pass, switch cost, where the notes sit

`make_kit` / `make_kit_with` / `make_kit_with_view` stay the nominal ranking (tests and `d3_metal_blades_outrank_stone_when_practised`). The band path is `fit_kit` → `make_kit_for_view` (`world.rs:1686`).

If step 4 already threads `&[Belief]` into `make_kit_for_view`, extend that. If it does not, add one entry and leave the old one for nominal callers:

```rust
struct DecisionIn<'a> {
    beliefs: &'a [Belief],
    learn_curve: f32,
    spare: f32,    // b.d.spare
    need: f32,     // b.d.need
    store: f32,    // b.store
    ambition: f32,
    curiosity: f32,
}
```

`decision_view` (`world.rs:1661`) stays what it is (harvest, food, pop, upkeep rate, wet, user). Do not stuff beliefs into `BandView`: nominal callers construct `BandView` by hand.

One pass, the pass that already exists (`CHOOSE_ORDER` in `make_kit_scored`, `effects.rs:1299`). Do not add a third pass. For each food slot (`is_food_slot`, `effects.rs:376`):

1. Score at most `KIT_CANDIDATES` plus the current item, by `belief_slot_food`.
2. `U` as in section 3.2–3.3, other slots fixed at the items chosen so far (same order as today, so shared food interactions are the ones `food_deps_changed` already re-scores).
3. Adopt `it` only if own-use `>= BELIEF_ADOPT_N` **and** `U(it) > U(current) + switch_cost(it)`.
4. Otherwise keep the current item. If `it` is the best challenger, store it on `k.trial[trial_j(slot)]` and store `trial_chance` for the economy. If nothing beats the incumbent even as a trial, `trial` is `NONE` and `trial_p` is 0.

`switch_cost` for step 5 is one making's believed labour times `labour_price`. The material half of "believed labour and materials" waits for step 6's shadow price. A tool whose belief has `labour == 0` (a prior that has never been updated; `belief_from_prior` sets labour 0) pays no labour switch cost. Founders are filled with real upkeep (`starter_belief`), so they do pay it. That is enough to stop two founded tools flip-flopping. Do not invent a material price to paper over priors.

`switches_risk`: the adopted item's believed food is **lower** than the incumbent's, and `U` without the risk term would not have switched. A switch that also gains food is `kit_switches` only. `switches_conserve` and `switches_fight` stay 0.

Non-food payoff slots (boat, bow, shield, outrigger, plank, ship): keep step 4's ranking (or today's `nonfood_scores` if step 4 left them). Apply the same adopt gate and the same switch cost on believed labour. Their trial `d` is the labour-price difference plus any food the slot really moves (a bow moves game; value that through `belief_slot_food` if step 4 has a bow belief, else through the food delta the current yields give — that delta is a visible property of "there is a bow", which the band can see, not `tool_effects` of an unused material) plus explore, minus the change in `risk_penalty`. No fight term.

**Notes, not saved.** `refresh_toolkit` is `&self` (`world.rs:1693`) and phase 1 runs it in parallel (`band_prep`, ~1505). It must not touch `World::counters`.

```rust
// On Derived, not in Carry (band.rs:403). Default 0. Dropped by serde.
struct ChoiceNote {
    kit_switches: u32,
    switches_risk: u32,
    trial_p_sum: f32,   // sum of p over slots that have a challenger
    trial_n: u32,
    p_short: f32,       // incumbent, no trial mixed in
}
```

Plus `Derived::trial_p: [f32; 12]` (0 if no challenger) and `Derived::trial_fired: u16` (set by `economy`, read by the probe).

- A switch counts when the previous slot item was not `NONE` and the new one differs. `NONE → item` is a first fill, including the rebuild in `after_load` (~1205), and is not a switch.
- `after_load` does not increment world counters. The note sits until `band_season` drains it. Phase 1 and the harvest-phase refresh (`world.rs:1468`) both run before `band_season`; they add into the note. `band_season` adds the note to `fx.counters` once and zeroes it. A later refresh in `band_season` (~2236) records the kit the **next** season eats; the next `band_season` drains it. One change, one count.
- Trial rolls and `trial_food_lost` are written onto `Econ` inside `economy` (it already returns `make_failures`). `band_season` copies them next to line 2205.

`YearStats` samples: `Fx` gains `p_short: f32`, `trial_p_sum: f32`, `trial_n: u32`. `apply_fx` (`world/fx.rs:104`) runs in band order and pushes `p_short` onto a `#[serde(skip)]` `Vec<f32>` on `World`, and adds the trial sums. `year_stats` (~4481) writes:

- `trial_p_mean` = sum of `p` / number of challenger-slots (0 if none);
- `p_short_mean` = mean of the vec;
- `p_short_p90` = the sample at index `len * 9 / 10` after an ascending sort (same index style as `seacraft_p90`, `world/voyaging.rs:415`).

Then clear the vec. Empty year → 0. Do not fold these into `add_acc` (`fx.rs:96`): that macro sums counts, and a mean must not be summed. `year_acc` is already `#[serde(skip)]` (`world.rs:849`); the sample vec is the same kind of thing.

### 3.7 Hunger and invention

Do not edit `world.rs:2237`:

```rust
let lam = p.invent_rate * b.curiosity * (0.15 + spare + p.stress_invent * stress) * (pop / 25.0).sqrt();
```

or `attention`'s `hunger = 1.0 + 1.5 * stress` (~2338). The risk penalty lowers **trials** when food is short. Stress still raises the experiment rate. The test in section 8 locks the formula by calling the same expression through a one-line function **only if** the move is bit-identical (same multiply order). If extracting it is awkward, the test can build two bands that differ only in `d.stress` and compare `d.experiments` after one season with the rng fixed — but a whole `step` also moves other state, so prefer the one-line function.

---

## 4. Constants

Starting values from proposal section 6. Tune only these if a run misses the pass check. No new caps.

| Constant | Start | Lives |
|---|---|---|
| `LAB_FLOOR` | 0.2 | `beliefs.rs` (new) |
| `BASE_SD` | 0.15 | `beliefs.rs` |
| `SHORT_LINE` | 0.9 | `beliefs.rs` |
| `STORE_USE` | 1.0 | `beliefs.rs` |
| `RISK_STARVE` | 3.0 | `beliefs.rs` |
| `RISK_BOLD` | 1.0 | `beliefs.rs` |
| `TRIAL_SHARE` | 0.25 | `beliefs.rs`. Step 4's trial update weight points here. |
| `EXPLORE_W` | 0.5 | `beliefs.rs` |
| `EXPLORE_HORIZON` | 8 | `beliefs.rs` |
| `TRIAL_TEMP` | 0.02 | `beliefs.rs` |
| `TRIAL_MIN` | 0.02 | `beliefs.rs` |
| `TRIAL_MAX` | 0.5 | `beliefs.rs` |
| `KIT_CANDIDATES` | 3 | `beliefs.rs` |
| `INFO_W` | 0.3 | already in `beliefs.rs` on step 4. Do not duplicate. |
| `USE_PRACTICE` | 0.08 | already in `effects.rs`. |
| `BELIEF_ADOPT_N`, `BELIEF_NMAX`, `PRIOR_N`, `LC_PRIOR` | as step 4 | do not retune |

`OPP_W`, `OPP_USES`, every `FIGHT_*` constant: not introduced in this step. `KIT_TRIAL_P` is deleted. `KIT_UPKEEP` stays.

---

## 5. Counters, YearStats, events

All five counters already exist in the `counters!` registry (`world.rs:647–655`) and are 0. Filling them does not add a column. `metrics.csv`, `summary.json` `counters`, and frame `counters` pick them up because they walk `Counters::NAMES`.

| Name | Kind | Meaning once step 5 fills it |
|---|---|---|
| `kit_switches` | sum | Payoff slot whose adopted item changed from a real item to a different item. Not `NONE → item`. Not counted on `after_load`. |
| `switches_risk` | sum | Subset of `kit_switches` where believed food fell and the risk term is what made `U` switch. |
| `switches_conserve`, `switches_fight` | sum | Stay 0. Step 6. |
| `kit_trials` | sum | A challenger roll fired (`u < trial_p`) on a slot the band used. |
| `trials_declined_risk` | sum | Same roll would have fired with the risk penalty removed (`u < p_safe`) and did not fire with it. |
| `trial_food_lost` | sum | Thousandths of person-seasons. `max(0, food at the spent effort without the mix, minus food with the mix)`, summed. A gain adds 0. |

`YearStats` (`world.rs:480–483`), already declared, written once a year:

| Field | Meaning |
|---|---|
| `trial_p_mean` | Mean of `trial_chance` over slots that had a challenger that year. A band with nothing to try does not enter. |
| `p_short_mean` | Mean of each band-season's incumbent `P_short` (no trial mixed in). |
| `p_short_p90` | 90th percentile of those same samples. |

These three are gauges for that year, not running sums. The counters are sums. Say both in `DATA_SCHEMA.md` under "Skills step 0", replacing "step 5 reads 0". `DATA_SCHEMA` stays schema v1 (added meaning, not a new column). The test `skills_unfilled_fields_stay_zero` lists them as live, next to the step-2 and step-3 fields.

**Events.** None. Proposal section 4's optional kinds (`ToolWrittenOff`, `ArmedLikeVictors`, `ArmsRace`, `RaiderBand`) are other steps. `EventKind::COUNT` stays 43. `archive.rs` class lists stay. No per-band trial event: it would flood the log. The counters are the "why".

**Probe** (`probe_diffusion.rs:698`, three `NA`s today). Fill, probe only, so headless hashes do not move:

- `trial_rate_pshort_lo`: fired / challenger-slots with incumbent `P_short < 0.10`
- `trial_rate_pshort_mid`: `0.10 <= P_short < 0.30`
- `trial_rate_pshort_hi`: `P_short >= 0.30`

Read `Derived::p_short` and `trial_fired` after the step. Do not re-choose the kit inside the probe (it already rebuilds a toolkit for adoption scores; the trial rate must be the season that actually ran). Cuts are probe-only. `belief_err_n*` and `fight_belief_err_*` stay `NA` (steps 4 and 6) unless step 4 has already filled the belief columns; do not blank a column step 4 filled.

`slot_own` (`probe_diffusion.rs:226`) must use the marginal `U` the pass compares (food, labour price, risk; no explore, explore is trials only). Otherwise `adopt_worse_own` is no longer ~0 by construction. Step 4 may already have pointed this at belief food. Add the risk and labour-price terms to that same function. Do not score fight or shadow prices.

---

## 6. Save format

**No bump. `FORMAT_VERSION` stays 16. No migration. No new `Carry` field. No new `Kit` field that would need serializing (`Kit` is not saved).**

| New state | Saved? | Why it can be derived |
|---|---|---|
| `trial_p`, `ChoiceNote`, `trial_fired`, the year's `P_short` sample vec | No (`Derived` outside `Carry`, `World` field `serde(skip)`) | Recomputed from beliefs, `learn_curve`, `store`, `d.food`, `d.need`, `d.spare`, `d.harvest`, ambition, curiosity, and `hash3(seed, id, tick, slot)`. All of those are already saved. `after_load` already calls `refresh_toolkit` then `economy` (~1205). |
| `Kit::trial` | No, as today | The chooser rewrites it. |
| Counters and closed `YearStats` | Already saved (`World::counters`, `World::history`) | Filling existing fields. Old saves load 0 via `serde(default)`. |
| Beliefs, `learn_curve` | Already saved in v16 | Step 4's block. Step 5 does not extend it. |

A save at a year boundary has already pushed `YearStats`. A mid-year save drops the partial year's sample vec, the same way it already drops `year_acc`. The next seasons do not need those samples to continue; only that year's metrics row is short. Headless runs save at the end.

`after_load` must not change `tick`, `rng_state`, `pop`, `x`, `y`, `known.len`, or `reg.recipes.len`. Those are what `state_hash` mixes (`world.rs:4579`). Rebuilding a kit does not.

Saves from v1 through v16 keep loading. The v16 round trip (`v16_fields_round_trip_continues_identically` in `tests_skills.rs`) must stay green: a world saved and loaded continues to the same `state_hash`, because nothing new is in the blob and the derived notes are rebuilt.

---

## 7. Determinism and RNG

Step 5 adds **no** `Rng` call and **no** new `hash3` salt. The trial roll keeps `0x7121_A15E_D3D3_0001`, band id, tick, and slot index (`world.rs:1910`). Two thresholds are read off that one draw.

Runs still repeat: same seed, same hash, because beliefs (step 4) update in band order and this step only changes the threshold and the food the threshold causes. `deterministic` and `save_round_trip_continues_identically` (`tests.rs:10`, `:21`) stay as they are.

`state_hash` does **not** mix counters, `YearStats`, or kits. It moves only when pop, place, known count, recipe count, tick, or `rng_state` move. The food mix changes harvest, which changes births and deaths, which changes later `rng` consumption. So the **run-on** hash moves. The **load** hash does not.

| Hash | Where | Step 5 |
|---|---|---|
| `FIXTURE_V10_HASH.0` = `0x1845e2690330d59` | `tests.rs:1116` and the v9 load at `:944` | **Must not move.** Load, no `step`. |
| `FIXTURE_V10_HASH.1` = `0x5cef0617c34457c6` | `tests.rs:1122`, after 40 steps | Moves in the commit that wires the chooser or the food mix. Update the constant and extend the comment on `:1124` by one clause. One line in the commit message. |
| Craft checksum `0x4a026ad8e2339cbe` | `tests.rs:964` | Must not move (no craft or known edits on load). |
| Map hashes | `tests.rs:2324` | Must not move (no map edits). |

Any other test that asserts a specific pop or a specific kit on a **stepped** world gets a one-line update in that same commit, with the new value. Do not weaken the assertion. Do not update `.0`.

---

## 8. Tests

Unit tests, pure, no world, in `tests_skills.rs` unless the piece lives next to an existing test:

- `phi(0) == 0.5`; `phi` at the z of the 90th percentile is about 0.9 (tolerance 1e-5); a large negative z is 0.
- Thin stores, `mu` just under need, small `sigma`: `P_short` high. Same `mu` with `store` several times `need`: `P_short` near 0. Widening `sigma` near the line raises `P_short`. Far above the line it lowers it.
- `risk_penalty` falls as ambition rises and is still positive at `ambition = AMBITION_MAX`.
- `labour_price` at `spare = 0` equals the full price bit for bit. At `spare = 1` it equals `LAB_FLOOR * full`.
- `trial_chance(0, need)` is the midpoint of min and max. A large negative `d` is about `TRIAL_MIN`. A large positive `d` is about `TRIAL_MAX`. Never outside `[TRIAL_MIN, TRIAL_MAX]`.
- `explore_value` is 0 at `curiosity = 0`, rises with `var/n`, and rises with `gain`.
- Switch cost: two tools, `U` differing by less than one making's labour price, incumbent stays across a second call. A tool that beats the cost switches once and then sits (the second call is not a flip back).
- Own-use gate: `n` below the adopt line sets `trial` and does not change the slot item. At the line, it can adopt.

**Section 5 decision tests** (synthetic bands, not a 500-year world):

1. **Hungry twin declines, bold or stocked twin takes it.** Two `DecisionIn` values, same beliefs: a wide prior on a challenger (high `var`, low `n`) whose believed food is a bit above the incumbent. Hungry: `store` near 0, `view.food` near `need`, `ambition = 0`. The roll threshold `trial_p` is near `TRIAL_MIN`, and `p_safe` is high enough that a fixed `u` in between counts as `trials_declined_risk`. Bold: same stores, `ambition` high (inside `AMBITION_MAX`), `trial_p` clearly higher. Stocked: `ambition = 0`, `store` several seasons of `need`, `trial_p` clearly higher. Same beliefs, so this does not depend on step 4's updater.
2. **Switch cost stops flip-flopping.** Covered above. Near-equal food, labour not zero.
3. **Hunger still raises invention.** The lambda at `stress = 0.8` is greater than at `stress = 0`, same curiosity, spare, and pop. And a comment on `world.rs:2237` is not a substitute: the test calls the expression the season uses.

**Rewrites**

- `d3_trials_about_one_season_in_five` (`tests_diffusion.rs:130`): a hand-set challenger with a hand-set `trial_p` (stash it on `Derived`) fires about `trial_p` of 400 seasons, the same roll both times, and upkeep rises when it fires. Also: with `trial_p = 1` the season's food multipliers differ from the unmixed kit when the challenger is a worse tool. The old "50..110 of 400" window was `KIT_TRIAL_P = 0.2` and goes away.
- `payout_invariant`: reconstruct the roll with `trial_p`, not `KIT_TRIAL_P`. Stored kit multipliers stay the adopted kit, so the `kit_yields` half does not grow a trial mix.
- `d3_newcomer_is_tried_then_takes_over` and `d3_metal_blades_outrank_stone_when_practised` use `make_kit` / `make_kit_with` (nominal, no beliefs). They should stay. If the nominal path was accidentally wired to risk, they fail, which is the signal to unwire it. Do not retune them to beliefs.
- `skills_unfilled_fields_stay_zero`: move `kit_switches`, `switches_risk`, `kit_trials`, `trials_declined_risk`, `trial_food_lost`, `trial_p_mean`, `p_short_mean`, `p_short_p90` into the live set. Leave `switches_conserve` and `switches_fight` required at 0.

**Stay green**

- Step 4's no-peeking perturbation: changing the true effects of a tool the band has never used does not change `make_kit_for_view`'s result. The new pass reads beliefs, the prior, and band state only. The economy's food mix is allowed to change the **payout**, and step 4's updater is allowed to move the belief from that payout. The decision function itself still must not call `tool_effects`. If step 4's structural test is a grep or a module boundary, new helpers go on the belief side of that boundary.
- `deterministic`, save round trips, v1–v16 fixture loads, `FIXTURE_V10_HASH.0`, the v9 craft checksum, map hashes.
- `counter_registry_covers_every_field` (no new counter field).

---

## 9. Performance

Budget: **at most +5%** sim time on `bench.yml` against the step-4 binary, same seed and thread counts (seed 5, 300 years, default map, threads 1/2/4). Proposal 3.6's own budget is a few microseconds a band a season: about 12 slots × 4 candidates × a handful of multiply-adds, one `exp`, one `phi`.

How:

- Score `KIT_CANDIDATES + current`, not every material. Today `choose_food` walks every candidate in the bucket (`effects.rs:1162`). Three plus current is less work, not more, before `phi`.
- Incumbent `mu`, `sigma`, `P_short`, and `labour_price` once per band per choice. Each candidate adjusts one slot.
- `phi` is a short polynomial, not a crate and not a loop.
- Trial food: one extra `yields` only for a slot that **fired**. `trial_p` ≤ 0.5 and usually near the floor, six food slots, so well under one extra yields per band per season. The counterfactual `food_of` is one more call per band that fired at least one trial, at the effort already chosen.
- No new scan of the map, no new chain fill, no call into `tool_effects` from the chooser, no per-band allocation in the steady state (the year's `P_short` vec is one `f32` per band-season, cleared at year end: ~800 KB on the 800×800 run, not per choice).
- `refresh_toolkit`'s cache keys stay. The choice still runs when `kit_dirty` is set. Do not mark the kit dirty for risk; risk is an input to the choice that already runs.

If the bench is over +5%, look at `phi` being called twice per material instead of per candidate, or at the food mix rebuilding the whole `YArgs` from scratch per slot. Do not cache by skipping the adopt gate or the switch cost.

Local check the implementer runs, through the sim lock, one thread, not a guard-rail map: seed 5, 300 years, default map, against the step-4 `kindling` binary. Record seconds and pop in `PROGRESS-skills.md`. Do not run seeds 1..10 or the 800×800 set locally.

---

## 10. CI

Build Bot runs these. The implementer does not.

**(a)** `headless-sims.yml`: seeds `1..10`, 500 years, map `256x160`.

**(b)** `headless-sims.yml`: seeds `499246292..499246301`, 750 years, map `800x800`.

**Bench** `bench.yml`: seed 5, 300 years, threads 1 2 4, baseline = the step-4 ref. At most +5% on the single-thread number.

**Probes** (`probe: true` on the diffusion workflow, or the ignored `probe_diffusion` test): 128×80, 192×120, 256×160, compared with `docs/REPORT-diffusion.md` section 9 as previous steps were. New columns to read: `trial_rate_pshort_lo/mid/hi` (high below low), and `c_kit_trials`, `c_trials_declined_risk`, `c_trial_food_lost`, `c_kit_switches`, `c_switches_risk`. `c_switches_conserve` and `c_switches_fight` still 0.

**Guard rails**, every seed, both (a) and (b):

- No continent crossing before year 100.
- (a) at most 6/10 seeds cross; (b) under 7/10.
- War rate not doubled versus D1–D6: (a) 0.320 %/y, (b) 0.271 %/y, over the whole map.
- No population collapse: no seed down more than about 20% against step 4 without a cause written in the progress note.
- (b) seed 499246298 may die early; it counts in the x/9 columns.
- Population within about ±10% is the food-cost watch from proposal section 7 (novice factor already landed; trial food is the new cost). A miss here is a reason to lower `TRIAL_SHARE` or `TRIAL_MAX`, not to add a cap.

`cargo test --release --workspace` and `./build-web.sh` as on previous steps. `cargo` warnings on the kindling build: 0.

Results go in `docs/PROGRESS-skills.md` with a placeholder line for the CI run ids, the way step 3's entry is written. Build Bot fills the ids.

---

## 11. Commit order

Each commit: `cargo test --release --workspace` green, kindling build warnings 0, then push. Hashes: only the wiring commit may change `FIXTURE_V10_HASH.1`. None may change `.0`.

| # | Commit | Effort | Behaviour |
|---|---|---|---|
| A | Constants and pure functions (`phi`, `p_short`, `risk_penalty`, `explore_value`, `trial_chance`, `labour_price`) plus their unit tests. Nothing in the season calls them. | S, half a day | None. Every golden hash unchanged. |
| B | `belief_slot_food` extracted from step 4's chooser with the same arithmetic, plus `mu` / `sigma` for one slot. Unit tests for thin stores, full stores, ambition, wide prior. Still not called from the season. | S, half a day | None, if the extract is bit-identical. If it is not, stop and fix the extract before going on. |
| C | The kit pass (section 3.6) and the three decision tests, behind `fit_kit` only. Nominal `make_kit` unchanged. `KIT_TRIAL_P` still in `economy` until D, but the pass already stores `trial_p`. **Do not ship C without D in the same PR.** C alone would pick challengers the old 20% coin then tries. Land C and D as two commits only if C is not yet reached from `refresh_toolkit`. Safer: C is the pass as a function with tests, D is the one call site. | M, one day | None until D. |
| D | Wire `fit_kit`. `economy` rolls `u < trial_p`, mixes food, counts trials, declines, and `trial_food_lost`. `band_season` drains switch notes. Delete `KIT_TRIAL_P`. Update `payout_invariant` and `d3_trials_about_one_season_in_five`. Update `FIXTURE_V10_HASH.1`. | M, one to one and a half days | **This is the behaviour change.** `.1` moves. `.0` must not. |
| E | `YearStats` (`trial_p_mean`, `p_short_mean`, `p_short_p90`), probe columns, `slot_own` marginal `U`. `DATA_SCHEMA.md`. `SAVE_FORMAT.md` one sentence. `skills_unfilled_fields_stay_zero` live-list. | S–M, half a day | Probe and metrics only. `state_hash` unchanged if D already landed. |
| F | `PROGRESS-skills.md` with the local seed-5 300y note (pop, the five counters, bench seconds versus step 4) and a placeholder for CI. Open the PR. | S, half a day | Docs. |

Total about four to five days, which is the proposal's M–L for row 5. Do not start F's CI claims before A–E are in. Do not retune constants inside A–E; a miss on the local seed is a note, and the tune is a follow-up commit that names the constant and the before/after counter.

---

## 12. Risks and questions for Michael

**Steel axe, copper axe, iron axe (open decision — this step does not decide it).**

Step 3 left this open on purpose. On the seed-6 year-350 save, bands hold a copper axe while a steel axe is makeable because the steel chain is about twice the labour (depth ~8 versus ~3) and the extra food does not pay that labour at today's full price. `axe_term` (`effects.rs:511`, `0.5 * cut_q + 0.5 * durability`) is the same shape for both. `KIT_UPKEEP` is not what the food-slot decision uses; `tool_upkeep` is, and it already charges depth. The probe's `KIT_UPKEEP` ranking still prefers steel. Nobody has changed the formula.

Step 5's labour price **multiplies that existing full price by the spare factor**. It does not change `axe_term`, `KIT_UPKEEP`, the `(1 + 0.3 * depth)` term, or `MAKER_WASTE`.

- Busy band (`spare = 0`): the copper-versus-steel gap in food units is **bit-identical** to today. The open decision is untouched.
- Band with spare time: both axes' labour is scaled by the same factor below 1, so the **absolute** gap shrinks and the steel axe's extra food can win. That is the spare discount doing what section 3.2 describes ("a band with spare time barely notices a high-upkeep tool"), not a retune of the axe.

Please say if you would rather the axe slot stay on today's full price until you decide the chain question. The plan does **not** special-case the axe. It also does **not** drop `WORK_SHARE` out of the divisor, which would move the gap even for a busy band.

**Other questions, if you want a different reading. Defaults are what the code will do if you do not answer.**

1. Trial-slot variance is mixed by `TRIAL_SHARE`, not by its square (section 3.2). The square is the independent-noise reading and makes a wide prior much less frightening.
2. Switch cost this step is believed labour only. The material half arrives with shadow prices in step 6. A never-updated prior has labour 0, so it does not pay a switch cost; founders do, because their fill includes upkeep.
3. `NONE → item` is not a `kit_switch`. Say if a first fill should count.
4. Probe bands are `< 0.10`, `0.10–0.30`, `>= 0.30`. They do not affect the sim.
5. Non-food slots use the same trial logistic and the same adopt gate, with no fight term and no shadow price. A boat still gets tried because of explore and `TRIAL_MIN`.

**Risks that are not questions.**

- Trial food is a real loss on top of the novice factor. Watch population ±10% on (a). If it drops, lower `TRIAL_SHARE` or `TRIAL_MAX` before touching anything else.
- A very cautious band can sit on stone. `TRIAL_MIN`, explore, and (from step 4) fade plus testimony are what keep the door open. `tools_retried` is step 4's counter; this step should not drive it to 0. If write-offs stick after this step, raise `INFO_W` or the fade before raising `TRIAL_MIN`.
- `mu` calibration (section 3.2) is the easy way to make `P_short` sit in a range where the penalty actually bites. If the first local run shows `p_short_mean` stuck near 0 or near 1, the scale is wrong or `BASE_SD` is wrong. Fix the scale before adding a cap.
- Lock-in and herding are already discussed in proposal section 7. This step does not add a new limiter.

**Out of scope, so nobody "finishes" them while wiring trials:** shadow prices and `other_uses`, `fight_need`, fight beliefs, `switches_conserve`, `switches_fight`, arms-race events, raiding, any new `EventKind`, any save-format bump, any edit to the invention lambda.
