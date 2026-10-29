# Proposal: users and makers (skill, trials and tool choice)

Status: **approved (Michael, Oct 28 2026), revision 4 with decisions 17-18; being built step by step,
progress in `docs/PROGRESS-skills.md`.** Written against
`main` 7231103 (the D1-D6 diffusion build reported in `docs/REPORT-diffusion.md` section 9).
Folded in (section 7, "Decisions"):
- Michael's answers of Oct 28 to the six design questions;
- his comments on revision 1: two-layer choice, risk against reward, material opportunity cost,
  chain skill, fight need, founders' skill;
- his answer on revision 2's open question: bands choose by **beliefs** formed from priors and
  their own experience;
- his comments on revision 3:
  - fight value is a belief like everything else, learned from clashes, hunting and what others
    saw (3.5);
  - arms races may happen and should be visible, bounded by their costs, not by caps (3.9);
  - a band that has forgotten how to feed itself may live by raiding (3.10).

Why now: the D1-D6 probe found iron knives and spears "adopted while worse" in 58-76% of cases
(`REPORT-diffusion.md` 9.5). The investigation found two causes in the sim and some in the probe:

1. **One skill number does two jobs.** A band's skill is kept per *recipe* (a maker's know-how),
   but the kit chooser also uses it as if it were the skill of the *user*, and food yields never
   use it at all. So a band holds a practised iron knife for years after it can make steel, while
   steel would feed it better from the first season. About three-quarters of the "worse" cases.
2. **Blades are ranked by toughness but feed by sharpness.** The chooser ranks knives by
   `edge_q = 0.55*cut_q + 0.45*durability` (and charges durability again through upkeep); yields
   use `cut_q` only, and the knife never enters fighting. So iron beats sharper flint in the
   choice and loses in the food. About a quarter.
3. **The probe** leaves out upkeep and fighting, counts tiny gaps as "worse", and samples every
   band-year rather than adoptions.

Michael's model fixes the first cause at its root: to a *user* a knife is a knife whatever it is
made of, while to a *maker* copper-smithing and steel-forging are different crafts. Both causes
share a deeper root: the chooser *misjudged what a tool does*. So this proposal splits tool choice
into two layers. One layer computes what a tool really does, and the economy pays out from the
same layer. The other layer is the band's own judgement, given its situation and temperament.
That judgement is made from what the band **believes** a tool does: before first use, an estimate
from the material and from similar tools it knows; after that, its own experience, luck included.
On top of that the proposal builds user and maker skill, risk-weighed trials, material opportunity
costs, fight readiness judged from experience, local arms races, and (as the last step, 8)
raiding as a way of life.

Effort scale (same as earlier proposals): **S** = under a day, **M** = 1 to 3 days,
**L** = a week or more. Each estimate covers code, tests, the save migration and the CI check.

---

## 1. Goals (in Michael's terms)

- **Realism.** People get better at what they do, and users and makers learn different things at
  different speeds. Bands choose tools the way people would: for what the tools do, weighed against
  their own situation.
- **Emergence, not scripting.** No "metal in year N", no caps, no hand-picked winners, no "hungry
  bands never switch" rule. Outcomes come from food, risk, materials, skill, ties and temperament.
- **The payout never lies, and bands judge from what they have seen.** One shared function
  computes a tool's true effects (layer 1), and the economy pays out from exactly those.
  - A band does not see those numbers. It holds a **belief** about each tool: an educated guess
    before first use, then its own track record, luck included.
  - How much it *values* what it believes depends on its stores, hunger, labour, materials,
    neighbours and temperament (layer 2).
  - Two bands can rightly choose differently with the same tools, and one unlucky season can put
    a band off a good tool for a while.
- **Risk against reward.** A new tool is a gamble: more food on average, but a chance of a bad
  season. A band that could starve if the gamble fails is much less likely to take it, and a bold
  or well-stocked band is more likely to. It is a weighing, not a rule.
- **Interesting stories.** A band short of copper but rich in flint keeps its copper for the
  things only copper does. A band with enemies nearby carries a spear that holds up in a fight.
  Well-fed regions switch to steel first while frontier bands wait for a safe year.
- **Diverse worlds.** Seeds and regions should differ more than now in what they use and when.
- **Local wars make stories, not map-wide rules.** Two rivals can drive each other into better
  weapons, and a band can end up living off its neighbours. What stops these is what they cost
  (food, labour, materials, lives, dissent, and the threat fading when the fighting ends), not a
  cap.

## 2. How it works today (plain version, with code)


**Skill is per recipe** (`civ-core/src/knowledge.rs:181`):
```rust
pub struct Known {
    pub recipe: u32,   // one recipe = (action, inputs) -> output item
    pub skill: f32,    // 0..1: +8% of the gap per season used, fades unused, forgotten below 0.04
    pub quality: f32,  // refinement of the method, capped at skill (v9)
    pub last_ok: u32,  // last practised or makeable (D5, v15)
    ...
}
```
The iron knife, the steel knife, the steel itself, the bloom and the bar are all separate recipes
with separate skills. Using a tool marks its whole recipe chain as practised (`mark_used`,
`knowledge.rs:291`); practice and fading are in `practise` (`world.rs:4248`).

**The chooser** (`civ-core/src/band.rs`):
```rust
640  pub fn edge_q(m: Mat) -> f32 { 0.55*cut_q(m) + 0.45*durability(m) }
680  let up   = KIT_UPKEEP * (1.15 - durability(m)) * (1.0 + 0.3*d as f32);
683  let cur  = gain * (0.5 + 0.5*sk) - up;   // recipe skill used as if it were user skill
684  let full = gain - up;                    // value if fully practised
698  Form::Spear => pupd(0, 0.35 + 0.65*edge_q(m)),
703  Form::Edge  => { pupd(0, 0.2*edge_q); pupd(1, edge_q) }
759  if inc != NONE && ch.1 != NONE && ch.0 > pay[s] { k.trial[j] = ch.1; }   // challenger
```
**What the economy pays** (same file, `make_kit_with`):
```rust
769  Form::Spear => 0.35 + 0.65*cut_q(m),               // cut_q, not edge_q; no skill
797  k.game_m = (1.0 + 1.4*hunt_q + ..) * (1.0 + 0.35*cut + ..);
812  k.farm_m = 0.5 + 0.3*dig + 0.35*cut + 0.2*axe;
602  pub fn weapon_q(..)   // fighting: spear and axe edges; the knife is never counted
```
**Trials** (`civ-core/src/world.rs:1687-1698`): every season a slot is used, a flat
`KIT_TRIAL_P = 0.2` (`band.rs:635`) coin flip (a deterministic `hash3`) puts the challenger to
work beside the old tool. That costs its upkeep and practises its recipe, but has **no effect on
food**: the trial can never go wrong, and hunger, stores or materials play no part.

**Where skill matters today:** the choice (683), the quality cap (`world.rs:1962`, 3254),
passing on at splits (2808, 3638), teaching (3785), forgetting (1964). **Where it does not:**
food, fighting, boats, and whether a tool gets made (a known recipe with its inputs at hand gives
the item every season; the recipe's `p` is only used once, at invention, `world.rs:2340`).


**Today vs Michael's model**

| | today | proposed |
|---|---|---|
| user skill | none; maker skill stands in for it when choosing, never in yields | per tool form; shared across materials; partial carry between similar forms |
| maker skill | per recipe; affects only quality cap, teaching, forgetting | per recipe plus carry-over within a technique; every chain step counts; affects labour, materials, quality and failure |
| iron knife to steel knife | the band starts from zero as user *and* maker | the user keeps full knife skill; the smith has to learn steel-making |
| what a tool does (yields, costs) | material numbers only; the chooser uses different numbers | one function (layer 1): material x user skill x maker quality, labour, material use, failure, spread, fight value; payout and decision both use it |
| how a band chooses | best skill-discounted score, same for every band | its own valuation (layer 2) of what it *believes* the tool does: food against need and starvation risk, labour price, material shadow prices, fight need, curiosity, ambition |
| what a band knows about a tool | everything, instantly (the chooser reads the true numbers) | a belief per item: before first use, a prior from the material's properties and similar tools; then a running mean and variance of what it actually got; fades when unused; partly inherited and partly heard from ties |
| trying a new tool | flat 20% a season, risk-free | risk against reward plus the value of learning; costs real food if it goes badly |
| how good a weapon is in a fight | computed from the material, and the same for everyone | a belief: a guess from the material at first, then learned from the band's own clashes, more weakly from hunting, and from what ties and enemies showed it |
| raiding | only a by-product of chance meetings; loot is a windfall nobody counts | (step 8) a way of earning food, weighed against hunting and farming by what past raids brought in and cost |


## 3. The design

### 3.1 Layer 1, consequences: what a tool really does

One function computes a tool's **true expected effects** for this band now. The economy pays out
from it, real season-to-season spread included. The decision layer never reads it directly; it
reads the band's beliefs (3.5), which are formed from what the economy paid out:

```rust
/// What `it` would do in `slot` for this band this season (true effects; no preferences).
struct Effects {
    food: f32,             // expected extra food (person-seasons) vs. the slot empty
    food_sd: f32,          // spread of that food from novice erraticness (3.3) and failure (3.4)
    labour: f32,           // upkeep labour (person-seasons), maker waste included (3.4)
    mats: [(u8, f32); 4],  // marginal material drawn per season (beyond what other kit items draw)
    p_fail: f32,           // chance the tool is not renewed this season (3.4)
    fight: f32,            // its term in fighting strength (weapon_q or loss_guard), 0 if none
}
fn tool_effects(b: &BandView, slot: usize, it: Item) -> Effects;
```

- **Yield** = material term x user factor x maker quality:
  `material_term(slot, it) * (USE_NOVICE + (1 - USE_NOVICE)*u_eff) * (1 + QUALITY_YIELD*q)`.
  - The material term is what the economy uses today: `cut_q` for knives (fixing cause 2), the
    `hunt_q` formula for spears, and so on.
  - It is turned into food by linearising around last season's harvest. The slot's multiplier
    change times the food that resource gave (`Derived::harvest`) is cheap and tracks what the
    economy will pay; the food term uses last season's harvest shrunk halfway toward `NOMINAL_MIX`
    at the same total (`HARVEST_PRIOR`): one season swings with the season, and there is no new
    saved state.
- **Spread** = novice erraticness plus failure (both real; see 3.3 and 3.4).
- **Labour** = today's upkeep formula times maker waste. **Materials** = the marginal stock drawn
  (`world.rs:~1874`) times maker scrap.
- **Fight** = the tool's term in the fight model, where toughness (`edge_q`) really belongs:
  - for weapons, its term in `weapon_q` (`band.rs:602`): a spear `0.35 + 0.65*edge`, a point
    `0.35*edge`, an axe `0.3*(0.5*cut_q + 0.5*durability)`;
  - for guards, its cut in losses in `loss_guard` (`band.rs:626`): a bow with points 0.3, a
    shield 0.25.
  - **New in revision 4:** this term is multiplied by the same user factor and maker quality as
    the yield. A novice with a bronze spear fights worse than a practised one. So the true fight
    value is not just the visible material, and there is something to learn. `power`
    (`conflict.rs:48`) and the losses in `fight` (`conflict.rs:135-146`) read the sum of these
    terms instead of `weapon_q(&kit)`.

The kit multipliers (`game_m`, `farm_m`, `hunt_q`, ...), the upkeep, the stock draw and the failure
rolls in the economy are all built from these same `Effects`.

**The invariants (revision 3).**
1. **Payout = layer 1.** The economy's multipliers, upkeep, stock draw and failure rolls come from
   `tool_effects`. Over many seasons, realised mean yield, failure frequency and material draw
   match `food`, `p_fail` and `mats`.
2. **Decision inputs = beliefs.** Layer 2 receives only `&Beliefs` plus band state; it has no
   access to `tool_effects`. This is enforced by the module boundary (the decision module does not
   import it) and by a perturbation test (5).
3. **Beliefs come only from priors and observed outcomes.** This holds for fight value as for
   food; there is no fight exception.
   - A prior uses visible material properties and the band's own beliefs about other tools.
   - An update uses an outcome the band saw: what the economy just paid, or what happened in a
     clash (who won, how many died on each side, and the arms the enemy carried).
   - Inheritance and testimony pass on only other bands' beliefs.
   - No peeking at true values, at `lethality`, or at the random draws of a fight.
   - Founders and bands loaded from old saves get a **starting state** for the tools they have
     used for generations (3.5, 4). That is where a world begins; from then on the same rules
     apply to them.

### 3.2 Layer 2, decision: the band's own valuation

The band scores each kit option by a utility in **food units (person-seasons)**, built from its
**beliefs** about each tool (3.5: believed food, spread, labour, materials, failure and fight value)
and its own state:

```
U(option) =  E_belief[food]                                (sum of the slots' believed food)
           - labour      * labour_price                    (cheap when there is spare labour)
           - sum(mats)   * shadow_price(material)          (opportunity cost of supplies)
           + F_believed  * fight_need * FIGHT_W * need     (readiness for expected fighting)
           - risk_penalty(P_short)                          (risk against reward, 3.6)
           + explore     (trials only, 3.6)
```

- **Labour price** = food per unit of labour last season x `(LAB_FLOOR + (1 - LAB_FLOOR)*(1 - spare))`.
  A band with spare time barely notices a high-upkeep tool; an overworked one does.
- **Material shadow price.** This is Michael's opportunity cost: what the supplies are worth for
  everything else the band does with them. It uses the band's own existing valuation `want(b, mi)`
  (`world.rs:804`):

  ```
  shadow_price(mi) = OPP_W * worth(b, mi) * (1 + OPP_USES * other_uses(mi))
  ```

  - `worth = want(b, mi).1`. It is already `reason x (1.3 - stock)`, where the reason is 1.0 if a
    tool in use needs the material (`d.needed`), 0.5 if a known recipe uses it (`d.uses`), or
    partner demand. So it rises as stock runs low.
  - `other_uses(mi)` counts the *other* kit items and makeable recipes needing it, worked out once
    when the toolkit is rebuilt.
  - Story: a band with little copper, which also needs copper for ornaments and bronze, finds its
    copper knife's shadow cost high. It switches to an iron or flint knife to keep the copper for
    what only copper does. When copper is plentiful again, the knife comes back.
- **Believed fight strength of an option**, built like the fight model (3.1) but from beliefs:
  `F_believed = sum of believed weapon terms + FIGHT_GUARD_W * (1 - product of believed guard
  factors)`. A band under threat compares a bronze and a flint spear by what it *believes* each
  does in a fight, after its own clashes and what it has seen.
- **Fight need** (Michael's answer 2: carry fight-worthy weapons when fighting is expected soon):

  ```
  fight_need = max( FIGHT_BASE + FIGHT_PRESS*pressed + FIGHT_GRUDGE*sum(grudge w)
                    + FIGHT_DISSENT*dissent + FIGHT_BOLD*max(ambition, 0)
                    + FIGHT_RAID*raid_share, 0 )                        // raid_share: 3.10
  ```

  - **No ceiling (revision 4).** Revision 3 had `FIGHT_MAX`; it is dropped, as Michael asked for
    no artificial limits on arms races. Each input already has its own natural bound: `pressed`
    is at most 1 and fades 0.8 a year; grudges fade (`grudge_memory` 0.92 a year,
    `conflict.rs:371`) and a band holds at most 6 (`MAX_GRUDGES`, `band.rs:176`). A feuding band
    still does not ignore food, because going short is priced steeply by the risk penalty (3.6).

  - `pressed` is recent pressure from stronger neighbours (`band.rs`, fades 0.8 a year,
    `world.rs:2852`).
  - Grudges are remembered raids and killings (fading). Dissent (friction that spills into
    quarrels) and ambition (bold bands press quarrels harder) count less.
  - All of these are already saved, so no new state. A band under threat values a spear's
    believed fight term; a peaceful one chooses by the hunt.
- **Temperament.** Ambition lowers risk aversion (3.6); curiosity raises the value of learning a
  new tool and of re-checking an uncertain one (3.6).

**Choosing the kit** (kept tractable):
- For each payoff slot, take the `KIT_CANDIDATES` (3) best makeable items by *believed* food,
  plus the current item.
- Starting from the current kit, do one pass over the slots. In each slot, pick the candidate with
  the highest `U`, with the other slots fixed (so shared materials and the whole band's
  starvation risk are counted).
- A new pick must beat the current one by its one-off making cost (believed labour and materials of
  making it). This is a real cost of switching, not a fudge, and it stops flip-flopping.
- A fully adopted switch happens only to a tool with at least `BELIEF_ADOPT_N` (1) season of the
  band's own use (a trial). Until then, the kit choice can only start trials; that is how first
  uses happen.

### 3.3 User skill by form, with a general transfer model

**State.** Each band keeps `use_skill: [f32; NFORM]` (32 forms, 128 bytes a band; only tool
forms ever move off 0).

**Transfer by shared motions, not hand-picked pairs.** Each tool form gets a small, fixed profile
of the motions and uses it trains (weights 0..1), in a new table next to the forms:

| motion | examples of forms that train it |
|---|---|
| cut (draw an edge) | Edge 1.0, Axe 0.3, Spear 0.2 |
| chop (heavy swing) | Axe 1.0, Edge 0.2 |
| thrust (stab with a shaft) | Spear 1.0, Point 0.3 |
| throw (hurl by arm) | Spear 0.5, Point 0.4 |
| aim and release (projectile) | Bow 1.0, Point 0.5 |
| fend (block, parry) | Shield 1.0, Spear 0.2 |
| knot and set (cord, mesh, snare) | Net 1.0, Trap 1.0, Cord 0.6 |
| dig and pry | Shaft (the digging stick in the dig slot) 1.0, Axe 0.2 |
| paddle and handle a hull | Boat 1.0, Outrigger 0.8, Plank 0.7, Ship 0.6 |
| sail and steer | Sail 1.0, Ship 0.6, Outrigger 0.3 |
| pound | Hammer 1.0, Axe 0.3 |

Transfer between two forms is the overlap of their profiles (cosine similarity), scaled:
`T(f, g) = USE_TRANSFER * cos(profile_f, profile_g)`, and `T(f, f) = 1`. A band's *effective*
skill with form g is

```
u_eff(g) = max( use_skill[g],  max over f of T(f, g) * use_skill[f] )
```

This is computed on the fly, so transfers need no saved state. Examples: Boat → Outrigger
transfers a lot; Spear → Bow only through "throw/aim" overlap, so little; Net ↔ Trap a lot.
Adding a form later only means writing its motion profile.

**Material does not matter to the user.** An iron and a steel knife are both form Edge, so
switching between them keeps full user skill.

**Practice and fade.**
- Practice: the form of each slot item in use gains `USE_PRACTICE * (1 - u)` a season, and a
  trial counts as practice for the challenger's form. With raiding (step 8, 3.10) food-slot
  practice is scaled by the share of effort still spent on food, `1 - raid_share`.
- Fade: an unused form loses `USE_FADE_X * floss` a season, where `floss` is the band's existing
  forgetting rate (so small bands lose it faster and records slow it, as for recipes).
  User skill is not "forgotten"; it just drifts toward 0.

**Daughters inherit it.** At a split, the daughter gets `USE_INHERIT * parent` for every form.
That is the same place Known entries are copied (`world.rs:2808`, 3638).

**Close ties teach it, more weakly.** In the yearly exchange (the seacraft and herbal-lore
pattern, `world.rs:3450-3466`), a band with a close tie (trade strength at least `CLOSE_TIE`, or
kin) can pick up skill with forms it *has in its kit* from the stronger partner:
`u += USE_TEACH_SHARE * (u_src - u)`, with chance `USE_TEACH_P`. Nobody learns past their
teacher, and this is well below what daughters get (Michael's answer 5).

**Starting values (Michael's decision).** Founding bands start at **0.7** (`USE_FOUNDER`) for the
forms of their starting kit, with room to improve, "and see how that goes". A band's first-ever
tool of a new form starts at its transfer value `u_eff(g)`, which may be 0.

**Novices are erratic, for real.** An unpractised user's yield from a tool varies from season to
season around its mean: a mean-preserving factor `1 + NOVICE_SD * (1 - u_eff) * z`, where `z` is
a deterministic `hash3` draw in -1..1 per band, slot and tick. This spread is a true effect of the
tool (layer 1). Bands see it only through what they get, so a novice's first season can be lucky
or unlucky, and its belief (3.5) records that.

### 3.4 Maker skill: technique labels, carry-over, and every chain step counting

**Keep per-recipe skill as the maker's skill**, so teaching, inheritance, forgetting and D5 work
unchanged. Add a **technique label** to each recipe, worked out from its action, inputs and output
when the recipe is registered (stored in the registry; never in the save):

knapping, grinding and carving, cordage and netting, hafting, sewing, spinning and weaving,
potting and firing, fire-making, cooking, cold-hammering, smelting, casting (crucible and mould),
hot-forging (bar and hammer at the forge), steel-making (the smith's hearth, `physics.rs:450`),
boat-building.

**Carry-over within a technique.** The effective maker skill of a recipe is

```
m(recipe) = max( skill,  TECH_CARRY * best skill of the band's other recipes of the same technique )
```

A smith who forges iron knives forges steel knives almost as well, but steel-making is its own
technique, learned from scratch. That is Michael's "years to move from copper to steel", and it
comes from practice speed rather than a timer.

**Every chain step contributes (Michael's answer 1).** A tool's maker skill is a **weighted
geometric mean** of its chain steps:

```
m_item = exp( sum_i w_i * ln(max(m_i, CHAIN_EPS)) / sum_i w_i ),   w_i = max(-ln(p_i), CHAIN_W_MIN)
```

`p_i` is the step's physics success chance (`Recipe::p`), so hard steps weigh more: steel-making
(p 0.03, w 3.5) and smelting the bloom (0.05, w 3.0) count more than forging the knife (0.10,
w 2.3).

Why this combination:
- **Not the minimum:** that would ignore every step but one, so getting better at the rest would
  count for nothing.
- **Not the plain product:** a product punishes long chains twice. Chain length already costs
  through upkeep depth (`1 + 0.3*depth`), materials and a failure chance at each roll, and a
  product would also make a ten-step tool look bad even with every step mastered.
- **The weighted geometric mean** lets every step count, lets one weak step pull hard (through the
  log), and does not fall just because a chain is long.

It is cached per item when the toolkit is rebuilt (the chain walk already exists in `mark_used`).

**What maker skill does** (Michael's answer 4: all of these). All of these are layer-1 effects:

| effect | formula (sketch) | where |
|---|---|---|
| labour (upkeep) | `upkeep * (1 + MAKER_WASTE * (1 - m_item))` | economy upkeep, `world.rs:~1679` |
| materials used up | stock draw `* (1 + MAKER_SCRAP * (1 - m_item))` | `world.rs:~1874` |
| quality | already capped at skill; refinement rate `* m_item` | `refine`, `world.rs:3243` |
| failure | each season, chance `p_fail = MAKER_FAIL * (1 - m_item)^2` that the tool is not renewed: the slot falls back to the next-best item that season and the extra material is lost | deterministic `hash3` roll per band, slot and tick |

### 3.5 Beliefs: what a band thinks a tool does (Michael's answer of revision 2)

This sits between the two layers. The payout is true (layer 1). The decision (layer 2) uses what
the band **believes**: before first use an estimate from what it knows, after that its own track
record, even when that record is luck.

**Key: one belief per item (form + material).** An `Item` is already `(form, material)`
(`materials.rs:346`). Why this key:
- **Per form alone is too coarse.** A flint and a steel knife are exactly what a band must tell
  apart.
- **Per item and quality is too fine.** Quality changes slowly, and the running mean tracks it.
- **Per item matches how people talk:** "copper knives don't hold up", "our iron spear is good".

**Storage: bounded.** Each band keeps at most `BELIEF_CAP` (24) entries in a small vec:

```rust
struct Belief {          // 32 bytes
    item: u16,
    slot: u8,            // slot it was used in (spears fill two slots; one entry each)
    flags: u8,           // own use / inherited / told / seen on a winner
    n: f32,              // effective number of observations, capped at BELIEF_NMAX
    mean: f32,           // believed realised slot multiplier (dimensionless; see below)
    var: f32,            // believed season-to-season variance of it
    fail: f32,           // believed failure chance (Beta mean, prior strength BELIEF_FAIL_N0)
    labour: f32,         // believed upkeep labour per season (running mean)
    fight_n: f32,        // fight evidence weight, capped at FIGHT_NMAX (0 for non-fighting items)
    fight_mean: f32,     // believed fight term (3.1)
    fight_var: f32,      // doubt about it
}
learn_curve: f32         // one per band: how much tools improve with practice, learned (see below)
fight_calib: [f32; 2]    // one per band: learned baseline loss ratio when winning / losing (below)
```

- **Size:** at most 24 x 32 = 768 bytes a band, and typically about 14 entries (one per payoff
  slot in use plus a few tried, heard of or seen on enemies), so about 450 bytes.
- An item can have a fight belief without a food belief: a band that was beaten by bronze spears
  it has never held knows something about them in a fight, and nothing yet about hunting with
  them.
- **Eviction:** when full, drop the entry with the smallest `n` that is not in the current kit.
  Forgetting a belief returns that tool to its prior.
- **Materials believed:** the drawn amounts are known from the recipe (the band can count what it
  uses), scaled by the believed scrap factor of that technique, which is derived from `labour`. So
  no extra field is needed.
- **What a belief measures:** the realised slot multiplier (what the tool did to that resource's
  take that season). It does not depend on band size or season, so it transfers across years and
  between bands. Layer 2 turns it into food with last season's harvest, as layer 1 does.

**Prior (before first use).** It is built only from things the band can see or has experienced:

```
P_vis   = material_term(slot, M)            // visible properties: sharpness, toughness, weight
rho_j   = mean_j / P_vis(j)                 // how the band's own tools turned properties into yield
s_j     = cos(profile(form_j), profile(form))     // motion similarity from user skill (3.3)
rho     = ( sum_j s_j*n_j*rho_j + PRIOR_N0*rho_all ) / ( sum_j s_j*n_j + PRIOR_N0 )
mean_0  = P_vis * rho
var_0   = (PRIOR_CV*mean_0)^2 * (1 + PRIOR_NOVEL*(1 - max_j s_j))    // more doubt for new kinds
fail_0  = the band's believed failure rate for items of the same technique, else PRIOR_FAIL
n_0     = PRIOR_N (0.5)                     // the first real season outweighs the guess
```

`rho_all` is the band's mean ratio over all its beliefs (founders start with beliefs for their
starting kit).

Why this is sensible and not peeking:
- A band that has used iron knives judges a steel knife by its edge and by how iron knives worked
  for it.
- A band that has never used anything like a bow gets a wide prior based on the band's average.
- The prior cannot know the true user factor for a new form, the maker quality it will reach, or
  the novice spread. So a first bow often disappoints, and that is realistic.

**Update (after each season of use).** `x` is the realised multiplier the economy just paid and
`w` is the share of the slot's work done with the tool (1, or `TRIAL_SHARE` in a trial):

```
n'    = min(n + w, BELIEF_NMAX)
k     = w / (n + w)
mean' = mean + k*(x - mean)
var'  = max( (1 - k)*(var + k*(x - mean)^2), BELIEF_VAR_MIN*mean'^2 )
fail' : Beta count update with this season's make roll (failed or not), same weight w
labour' = labour + k*(observed labour - labour)
```

- Capping `n` at `BELIEF_NMAX` (8 seasons) makes old seasons fade, so the belief follows a tool
  that gets better as the band practises it, with a lag.
- Uncertainty of the mean is `var/n`, which shrinks with use. Layer 2's spread for a tool is
  `var + var/n`: the season-to-season spread plus doubt about the mean.
- Luck matters: with `n_0 = 0.5`, a first full season has weight 2/3. A bad first draw (novice
  spread, a failure) can sink the belief below the incumbent, and the band drops the tool.

**Learning curve, learned.** Each band also keeps `learn_curve`: its own experience of how much a
tool's yield grows from its first seasons to later ones.
- It is updated when a belief passes `n = BELIEF_NMAX`: the ratio of the mean then to the mean
  after the first season, blended at rate `LC_RATE`.
- It starts at `LC_PRIOR` (1.3).
- Layer 2 uses it for the value of learning (3.6): a band that has seen tools improve expects a
  disappointing new tool to get better. That counters the "every first season looks bad" bias
  without telling the band the true novice factor.

**Fade when unused.** Once a year, every entry not used that year has `n *= BELIEF_FADE` (0.8).
Its mean and variance are pulled toward a fresh prior by the share of `n` lost.
- Old bad (or good) experiences lose their grip after some years, so a written-off tool becomes
  uncertain again. Curiosity then makes it worth a re-trial (3.6).
- Entries with `n < 0.1` are dropped.

**Daughters inherit beliefs.** At a split (`world.rs:2808`, 3638), the daughter copies the
parent's entries with `n *= BELIEF_INHERIT_N` (0.6) and `var *= 1 + BELIEF_WIDEN` (1.5x), and
copies `learn_curve`. The daughter trusts the parent's experience, but less than its own, and it
may live in different country.

**Close ties tell (testimony).** In the yearly close-tie exchange (`CLOSE_TIE` or kin; the seacraft
pattern, `world.rs:3450-3466`), with chance `BELIEF_TELL_P`, a band hears about one item the
partner has used more (`n` higher) and that it can make or nearly make.
- **How it counts:** it is folded in as a pseudo-observation, `x = partner.mean`, with weight
  `w = TELL_W * partner.n / BELIEF_NMAX`, and the variance gets the partner's variance widened by
  `1 + BELIEF_WIDEN`. Told beliefs never take a band's `n` past `TELL_NMAX` (1.0): hearsay is never
  as good as one's own season.
- **Bias comes for free:** the partner's numbers come from its own land, skill and luck, not from
  the listener's. No artificial bias is added.
- **It spreads news:** "steel knives are good" travels along ties ahead of the knives, and so does
  a bad reputation.

**Founders and old saves.**
- **Founders** start with beliefs for their starting kit. They have used these tools for
  generations, so their starting state comes from the true effects (the starting state of the
  world, not an exception to how beliefs then change):
  `n = BELIEF_NMAX`, `mean` = the true expected multiplier at user skill 0.7, `var` from the
  novice spread.
- **Old saves** are handled the same way for the kit items in use (4).
- After that, every belief is formed by priors and observation only.

**Fight value is a belief too (revision 4, Michael's call).** The visible-property estimate is
only the prior. After that, a band's idea of what a weapon or shield does in a fight comes from
three kinds of evidence, strongest first: its own clashes, what it saw others do (ties' news and
the enemies who beat it), and, weakly, hunting.

*What counts as a clash.* There is no separate "war" or "battle" in the code. Every violent
meeting, raid or feud alike, is one call of `fight` (`conflict.rs:126`), reached through a
chance meeting of neighbours (`contacts`, `world.rs:3011`; `meeting`, `world.rs:3207`) and
`confront` (`conflict.rs:53`). A war is a run of such fights between bands holding grudges. The
evidence comes from:

| outcome | code | what each side sees | evidence |
|---|---|---|---|
| fight | `fight`, `conflict.rs:126-226`; `Op::Fight` (`conflict.rs:202`) becomes `EventKind::Fight`, `FirstFight`, `BloodiestFight` (`world.rs:197-206`) | who won; dead on each side (`dl`, `dw`, `conflict.rs:147-148`) against each side's size; the other side's kit (hunt weapon, axe, bow and points, shield) | full: `W_CLASH` |
| broken and absorbed | `EventKind::Absorbed` (`conflict.rs:210`) | as a fight; the absorbed people bring their beliefs (below) | full, plus testimony |
| fled | `confront` → `displace` (`conflict.rs:85-94`, 256; `EventKind::Fled`, 336) | the pressing side's arms and that we ran | win/loss only: `W_YIELD` |
| tribute | `conflict.rs:97-120` (`EventKind::Tribute`) | the same, for paying | win/loss only: `W_YIELD` |
| backed off | `conflict.rs:73-79` | nothing new (the band judged by its beliefs and left) | none |

*What the band expects, and the surprise.* Before a clash the band has its own believed fight
strength `B_own` (3.2) and a belief about the enemy's visible arms, `B_enemy`, from its own
entries for those items (a prior if it has never met them). It also knows in rough terms how a
fight works: better arms kill more, shields and bows save lives, the loser loses more. These are
the forms at `conflict.rs:129` and 138-146, without their numbers. From these it predicts the
ratio of enemy losses to its own:

```
R_pred = fight_calib[won] * (0.6 + B_own) / (0.6 + B_enemy) * G_enemy / G_own
R_obs  = (enemy dead + 0.5) / enemy size  /  ((own dead + 0.5) / own size)
z      = ln(R_obs / R_pred) + WIN_Z * (won - p_pred)     // p_pred from the believed power ratio
```

`fight_calib` is the band's learned baseline: a running mean (rate `FIGHT_CALIB_RATE`) of the
unexplained part of `R_obs / R_pred`, one value for won fights and one for lost. It soaks up
everything the band cannot attribute to arms (lethality, luck, numbers), so the band never reads
`lethality` or the random ranges.

*Credit split (attribution).* Half of `z` is evidence about the band's own side and half about
the enemy's, with the opposite sign. Within a side, each item gets the share it had in that
side's believed strength:
- a weapon `i`: `share_i = b_i / (0.6 + B)`, its term in `0.6 + weapon_q`;
- a guard `j` (shield, or bow with points): `share_j = ln(1/g_j) / ln(1/G)` of the guard part;
- the rest (the `0.6` baseline) goes to `fight_calib`.

Shares on a side add to 1, so the credit given out equals the surprise. A spear that did nearly
all the work gets nearly all the credit, and an axe carried alongside a little. Each item then
takes a pseudo-observation `x_i = fight_mean_i * exp(z_side)` with weight `w_i = W_CLASH *
share_i * cas`. Here `cas = min(1, (dl + dw) / CAS_REF)` scales a skirmish with one death below a
bloody fight. The update is the same running mean and variance as for food, in `fight_n`,
`fight_mean` and `fight_var`, with `fight_n` capped at `FIGHT_NMAX`. That cap is the same
"old evidence fades" device as `BELIEF_NMAX`, not a limit on how good a weapon can seem.

*Weights.* One clash is worth `W_CLASH = 2` seasons of evidence, against at most `0.1` for a
season of hunting (below). People remember a fight. With `FIGHT_PRIOR_N = 0.5`, one bloody clash
moves a belief about 80% of the way to what it showed.

*The losers learn most from what beat them.* The enemy-side half of `z` is weighted by
`W_SEEN_LOSER = 1.5` for the side that lost and `W_SEEN_WINNER = 0.5` for the side that won.
Defeat is memorable, and the winners' spears were the thing that killed. So a band beaten by
bronze spears comes away believing in bronze spears, even if it has never held one. The entry is
flagged "seen on a winner", which drives `weapons_adopted_after_defeat` (4). Captives and
absorbed remnants (`absorb`, `conflict.rs:229`) bring their old band's fight beliefs into the
winner as testimony, weight `TELL_W`. That is how reputations spread after wars, with no
peeking.

*Hunting is weaker evidence.* A season of hunting with a spear tells the band something about the
spear in a fight, because the same user factor and maker quality are at work (3.1). It also
tells it less, because game does not fight back.
- From the hunting update the band gets an implied user-and-maker factor
  `phi = x_hunt / P_vis_hunt`. The fight pseudo-observation is `x = P_vis_fight * phi`.
- Its weight is `HUNT_FIGHT_W * s_f`, where `s_f` is the cosine between the form's motion profile
  restricted to hunting motions (thrust, throw, aim) and to fighting motions (thrust, chop, fend,
  aim), as in 3.3. A spear transfers much, a bow fairly much, and an axe only through chop. A
  shield transfers nothing.
- Hunting alone never takes `fight_n` past `HUNT_FIGHT_NMAX` (1.5): a band that has only hunted
  is never sure of its spear in a fight.

*Ties tell of their fights.* In the yearly close-tie exchange, testimony (above) may also carry a
fight belief the partner updated from a clash in the last `FIGHT_NEWS_YEARS` (3), with the same
weight and `TELL_NMAX` cap rules. "The river people's bronze spears beat the hill band" travels
along ties.

*Prior, fade, inheritance.*
- **Prior:** `fight_mean_0 = P_vis_fight * rho_fight`. `P_vis_fight` is the 3.1 term from visible
  toughness and edge, and `rho_fight` is the band's own observed-to-visible ratio over its other
  fight beliefs (1 if none). The spread is `FIGHT_PRIOR_CV` widened by novelty as for food, and
  `fight_n_0 = FIGHT_PRIOR_N`.
- **Fade:** an entry with no clash evidence in a year has `fight_n *= BELIEF_FADE` and is pulled
  toward its prior, which keeps the band's hunting experience. Long years of peace return
  weapons to their property-and-hunting estimate.
- **Inheritance:** daughters copy fight beliefs and `fight_calib` with the same `n` and widening
  rules as food beliefs.

*Founders and old saves* get fight beliefs for their kit's weapons and guards as a starting
state: the visible-property term at user skill 0.7, `fight_n = FIGHT_N_START` (1.0), and
`fight_calib` at the model's baseline. That is a starting point, weaker than one clash, and
it is learned over like any other belief.

**Performance.**
- **Priors** are computed only for candidates without an entry: at most about 36 a season, each
  looping over at most 24 entries with a precomputed 32x32 similarity table. That is under 1k
  multiply-adds.
- **Updates:** about 12 a season, a few flops each. **Fade:** once a year.
- **Fight beliefs:** a clash updates at most about 8 entries on each side (4 own, 4 enemy) plus
  one `exp` and one `ln`. Clashes are rare (well under one a band a year), so this is noise.
- **Total:** with 3.6, a few µs a band a season. The bounds are `BELIEF_CAP`, `KIT_CANDIDATES` and
  one challenger per slot. It is checked on `bench.yml` (at most +5% in total for the proposal).

### 3.6 Trials and risk: reward weighed against the chance of going short

Everything in this section uses beliefs (3.5), never layer 1.

**Who is a challenger.** The candidate with the highest *believed potential*,
`mean * learn_curve` for a tool still being learned (`n < BELIEF_NMAX`), plus an uncertainty bonus
(below), when that beats the incumbent's believed utility.

**Spread of outcomes, as the band sees it.** In a trial season, `TRIAL_SHARE` of the slot's work is
done with the challenger. The payout uses true effects at current skill (Michael's answer 1, so a
trial costs real food). The band's expectation of the season's food has:
- mean `mu` = believed food with the trial mixed in;
- variance `sigma^2` = the band's base harvest spread `(BASE_SD * food)^2`, plus each slot's
  believed `var + var/n` (an unfamiliar tool is doubtful as well as erratic), plus the failure
  terms `fail*(1 - fail)*(term - fallback)^2`.

**Chance of going short.** Food available = this season's food + usable stores:

```
P_short = Phi( (SHORT_LINE * need - mu - STORE_USE * store) / sigma )      // normal approximation
risk_penalty = RISK_STARVE * need * exp(-RISK_BOLD * ambition) * P_short
```

- The penalty grows **sharply** as the band nears starvation. With thin stores and food near need,
  a small extra spread lifts `P_short` steeply. With full stores it is close to 0 whatever the
  spread.
- In effect this is a utility of food that bends sharply below need.
- Ambition lowers the penalty (bold bands gamble); it never removes it.
- Hunger lowers trials through this risk, while it **keeps raising invention** (Michael's answer 6;
  `world.rs:1915` unchanged).

**The value of learning and of finding out** (why anyone tries anything):

```
gain    = (U_believed_potential(ch) - U_now(inc)).max(0)          // learn_curve inside
explore = curiosity * EXPLORE_HORIZON * ( EXPLORE_W * gain * USE_PRACTICE
                                          + INFO_W * sqrt(var_ch/n_ch) * food_slot )
```

- The first term is a season of practice: the believed gap, times how much one season closes it,
  times a horizon.
- The second term is **what the band would learn by finding out**: it grows with the doubt about
  the challenger's mean. A tool written off long ago, whose belief has faded, regains doubt and so
  becomes worth a re-check.
- Curious bands value both more.

**Decision.** `d = U(trial) - U(no trial) + explore`, in food units. The chance of trying is

```
p = TRIAL_MIN + (TRIAL_MAX - TRIAL_MIN) * logistic(d / (TRIAL_TEMP * need))
```

rolled with the existing deterministic `hash3` (`world.rs:1692`), so runs repeat.
- `TRIAL_MIN` keeps even desperate bands trying something now and then.
- The logistic keeps choices graded rather than all-or-nothing: people are not perfect optimisers.

Stories this gives:
- A hungry band with thin stores mostly declines a risky first bow (wide prior), and takes the
  safe iron → steel knife switch (similar tool, narrow prior).
- A band whose first bow season went badly drops the bow. Its neighbour, which was lucky, keeps
  it. Years later the first band hears good things from that neighbour, or its own bad memory
  fades, and it tries again.
- A bold band, or a well-stocked one, takes the bow.
- A band rich in flint but short of copper uses flint knives and keeps the copper for bronze.

**Material opportunity cost** enters the trial through `U`, exactly as for the kit choice. A
challenger made of plentiful materials, replacing an incumbent made of scarce, much-used ones, is
favoured even at equal food.

**Performance and how it is bounded** (with 3.5's belief work). For each band, each season:
- about 12 payoff slots x at most 4 candidates, so about 50 utility evaluations;
- each evaluation is a handful of multiply-adds, at most 4 materials, one `exp` and one normal-CDF
  approximation (`erf`, rational form);
- the static parts (visible material terms, chain maker skill, material masks, `other_uses`) are
  cached when the toolkit is rebuilt;
- `u_eff` for the about 12 tool forms is recomputed only when `use_skill` changes (a 32x12 max);
- trials consider at most one challenger per slot (as now).

Together with the belief priors and updates, that is under 2k flops a band a season, a few µs.
Even 50k bands add tens of ms per simulated
year across 4 threads. The bounds are `KIT_CANDIDATES`, the single pass, and the caching. The
target is at most +5% sim time on `bench.yml`, checked at steps 1, 4 and 5.

### 3.7 Blade scoring fix (cause 2)

This falls out of layer 1:
- A knife's food effect is `cut_q`-based, as the economy pays. Its prior uses visible sharpness,
  so bands expect that too.
- Toughness counts only through labour (upkeep) and material use.
- Weapons get their fight value in layer 2 only as believed fight strength times `fight_need`
  (3.2), and that belief is learned (3.5). A knife has no fight term, because it is not used in
  fighting. `edge_q` survives only inside the weapon terms of the fight model.

### 3.8 Probe fixes (`civ-core/src/probe_diffusion.rs`)

- The probe's payoff uses layer 1 (3.1): food, labour and material costs, and the fight value.
  It reports three things separately:
  - **"worse in food"** (true food). This can now be legitimately non-zero: a band can trade food
    for safety, labour, saved materials or fight readiness, or it can simply be mistaken.
  - **"worse in the band's own valuation"**. This should be about 0.
  - **"believed wrong"**: the gap between the band's belief and the true value of the items in its
    kit and of the best alternative. This splits "chose worse" into a judgement part and an
    information part, and the report shows which applied.
- Gaps under `PROBE_TIE = 0.02` count as **ties**, not "worse".
- **Adoption** is measured once, when an item enters the kit, against the item it replaced (the
  previous kit's slot item). The yearly figure is kept under a new name, `worse_held_share`
  (band-years held while a better makeable item existed), so the report says what it means.
- New probe columns: trial rate by `P_short` band, user skill by form, maker skill by technique,
  make failures, switches made to conserve a material, belief error by `n` (food and fight),
  write-offs and re-trials, and the invariant checks (3.1).

### 3.9 Arms races: allowed, visible, bounded by their costs (Michael, revision 4)

**How one starts, with nothing scripted.**
1. Band A beats band B with bronze spears. B's belief in bronze spears jumps (the loser learns
   from what beat it, 3.5). B's `pressed` rises by 0.4 (`conflict.rs:224`), and its grudge by
   `0.4 + 3*fl` (195). So B's `fight_need` rises.
2. B now values a bronze spear's believed fight term highly. If it can make bronze, or trade for
   it, it switches, even at some cost in food.
3. At the next clash A does worse than it expected against the newly armed B. Its winner's
   grudge (`0.1 + 3*fw`, 196) and its own surprise raise its `fight_need` and its interest in
   something better again: a shield, or iron.
4. Each step is the same decision rule as any tool choice.

**No caps.** Following Michael, there is no ceiling on `fight_need` (`FIGHT_MAX` is dropped,
3.2), on fight beliefs, or on arms spending. The race is bounded only by what it costs and by the
threat fading:
- **Labour:** metal weapons and shields have upkeep, and unskilled makers waste labour (3.4).
- **Materials:** copper, tin and iron used for weapons carry the shadow price of everything else
  they are for (3.2). A race draws down stock that tools and ornaments need.
- **Food and hunting effort:** a spear chosen for fighting over the best hunter costs game every
  season. The risk penalty makes that cost steep for a band near going short (3.6).
- **Lives:** fights kill both sides (`conflict.rs:138-148`) and thin skills (149-157). A band that
  keeps fighting loses people and know-how.
- **Dissent (new, small):** a band's own recent fight deaths add `DISSENT_GRIEF * deaths/pop` to
  its friction (`world.rs:3575`). Grief and war-weariness make quarrels and splits more likely,
  and a split daughter starts with weaker grudges (`world.rs:3668` keeps 0.6 of them).
- **Fading threat:** when the fighting ends (one side flees, is absorbed, or the pair simply stops
  meeting), grudges fade at 0.92 a year, `pressed` at 0.8 a year, and fight beliefs fade back
  toward their priors (3.5). So `fight_need` and the believed worth of heavy arms fall, and the
  food terms win again.
- **Locality:** clashes need meetings, and meetings are local (`teach_radius`, twice for riders,
  `world.rs:3046-3049`). Distant bands hear of a race only through testimony, which raises
  beliefs but not their `fight_need`.

**Spotting arms races in the data.**
- A **rivalry** is a pair of bands with mutual grudges of at least `ARMS_GRUDGE` (0.3), where
  both made a fight-decided switch (`switches_fight`) within `ARMS_WINDOW` (10 years).
- An **arms race** is a connected group of rivalries, worked out once a year with union-find over
  the grudge lists (at most 6 per band), so it is cheap.
- New per-band state: `last_fight_switch: u32` (tick of its last fight-decided switch), saved.
- Counters and YearStats in 4: `arms_race_switches`, `arms_races`, `arms_race_max_bands`,
  `arms_race_max_share`, `arms_race_max_span`.

**Guard rail (new):** no single arms race spreads across the map. In every year of (a) and (b),
the largest arms race holds under `ARMS_MAX_SHARE` (25%) of living bands and spans under half the
map's width. The other side is checked too: arms races should happen, so at least one with 3 or
more bands appears in at least half the (b) seeds. Otherwise the costs are too steep, and the
story is missing.

### 3.10 Raiding as a way of life (step 8)

Michael's scenario: a band in an arms race finally beats its rival. But while it focused on
weapons it has forgotten how to hunt and farm well: its user skill, maker skill and food beliefs
for food tools faded. Now it lives mainly by raiding weaker neighbours. This should be able to
emerge, not be scripted.

**How raids work today** (checked in the code):
- **No one decides to raid.** Conflict happens only in *chance* meetings of neighbours. Each band
  draws a few nearby bands to meet each season, with a chance set by `teach_rate`, kinship and
  travel cost, at most `MAX_MEETINGS` (`contacts`, `world.rs:3011-3104`). A full meeting calls
  `confront` first (`world.rs:3207-3216`).
- **Whether it turns hostile:** with chance `0.05 * aggression * tension`, at most 0.5
  (`conflict.rs:61`).
  - `tension` (`conflict.rs:14-45`) is hunger, crowding, the lure of the other's stores (times
    hunger), goods the band lacks, and grudges.
  - It is reduced by trade ties, kinship and marriage.
  - Weapons do not enter it. They enter only the odds: `power` = people times `(1 + 1.5 *
    weapon_q)` times feeding (48-50); the pressing side's resolve (72) uses that power ratio.
- **What is taken:**
  - **victim flees:** 30% of its store (`conflict.rs:85-94`);
  - **tribute:** 25% of the store, and one good the raider lacks (97-120);
  - **fight:** 30-60% of the loser's store, half of every trade good the winner uses (or grain)
    (165-180), 30% of herds (182-189), and sometimes captives (214-220).
- **It is a windfall, not a livelihood.** Loot goes straight into `store`. Nothing records it as
  income; no band learns what raiding brings in; and nothing weighs raiding against hunting or
  farming. Raiding costs no work time, and movement (`cell_score`, `world.rs:2357`) looks only at
  foraging and farming ground, never at neighbours to raid.
- **Victims' responses today:** flee (`displace`, `conflict.rs:256`; `pressed` then makes the band
  readier to move, `world.rs:2849`), pay tribute (with a remembered debt, `owed`, 115-116), build
  palisades (`build_walls`, `conflict.rs:347`), hold grudges that make retaliation likely (0.6 x
  grudge in `tension`, 41), and with this proposal, arm up (3.5, 3.9). There are **no coalitions**
  and no standing protection payments.

**What is needed for raiding to be a real choice.**

*Layer 1 (true effects).* A band chooses a **raid share** `raid_share` of its effort each season,
from the grid {0, 0.1, 0.25, 0.5, 0.75}.
- **Lost food effort:** the harvest of every food slot is scaled by `1 - raid_share`, and so is
  practice of food tools' user and maker skill. A band that raids a lot loses its hunting and
  farming skill through the fade that already exists (3.3, 3.4). That is how "forgot how to hunt"
  comes about; no rule says so.
- **Seeking:** each season the band gets, with chance `RAID_SEEK * raid_share`, one extra
  *sought* meeting with a band within reach. It picks the target with the best believed odds (its
  fight beliefs about the target's visible arms and size) and the largest remembered take.
- **Intent:** in a sought meeting, `tension` gains `RAID_INTENT * raid_share`. Everything after
  that is today's code: back off, flee, tribute or fight, with today's loot, deaths, grudges and
  captives. So the true return of raiding is exactly what the existing conflict code pays.

*Belief about raiding.* One band-level belief, `raid: {n, mean, var, dead}` (16 bytes):
- `mean` and `var`: food taken per season of full raid effort (store taken, plus goods at their
  shadow price);
- `dead`: own deaths per raid, as a share of the band.
- It is updated after every sought raid, at weight 1, and after every chance clash the band won or
  lost, at weight 0.5. So a band that keeps winning fights learns that fighting pays, and one that
  keeps losing learns the opposite.
- It fades, is inherited and is told like other beliefs. Its prior is `RAID_PRIOR` times the
  band's believed power ratio against its neighbours, with wide doubt.

*Layer 2 (choice).* The raid share is picked by the same utility as everything else:

```
U(raid_share) = (1 - raid_share) * believed food from the kit
              + raid_share * raid.mean * targets          (targets: reachable bands it believes weaker)
              - raid_share * raid.dead * pop * PERSON_W * need/pop   (expected own losses)
              - labour, material and fight terms as usual
              - risk_penalty(P_short)                     (raids are lumpy: raid.var enters sigma)
```

- **Retaliation** enters without peeking. Being raided back shows up in the band's own experience
  (`raid.dead`, losses in clashes it did not seek) and in its `fight_need` through its own grudges
  and `pressed`.
- A band with faded food skills and low food beliefs, strong believed arms, and weak neighbours
  with stores finds raiding scores highest. That is Michael's scenario.
- A twin with good hunters raids less, because it has more to lose by not hunting.

*Victims.* They respond with what exists: fleeing, tribute, walls, grudges and retaliation, and,
new here, arming up after defeat (3.5) and raising their `fight_need`. Coalitions (neighbours
joining against a raider), standing protection or tribute deals, and raiders turning into rulers
are **politics, left for a later proposal**. These hooks make them possible later: `owed`,
grudges, partners and the raid belief.

*Natural limits (why it stays local).*
- **Depletion:** loot is a share of the victim's store, so raiding the same neighbours empties
  them. The returns fall, the belief follows, and raiding fades.
- **Flight and absorption:** victims flee out of reach or are absorbed. Absorbed people make the
  raider bigger and hungrier, and big bands split.
- **Victims arm and strike back:** the losers learn from what beat them, so raids get bloodier.
- **Raiders' own deaths** and grief-driven dissent (3.9) lead to splits.
- **Forgotten food skills make raiders fragile.** When the targets are gone, a raider band starves,
  splits, or slowly relearns to hunt. That is a story too.
- **Reach** is local, as for every meeting.

**Scope, honestly.** Fight beliefs and arms races (3.5, 3.9) belong in this proposal: they are
beliefs about tools and the same decision rule. Raiding as a livelihood is a new economic choice
(effort allocation, sought meetings, a new belief) and sits halfway into culture and politics. It
is **step 8 of this proposal** (decision 17): nothing in steps 0-7 depends on it, and its save
fields go into the one format bump (16) at step 2. Coalitions and lasting tribute stay later
politics work.

## 4. Save format, data and events

**Save format 16** (`save.rs:18`, now 15):
- **New band fields.** `Band` gains `use_skill`, `beliefs` (at most `BELIEF_CAP` entries, now
  with fight fields), `learn_curve`, `fight_calib` and `last_fight_switch`, appended at the end of
  the struct as the rules require. With step 8 in the same bump, it also gains `raid` (4 f32),
  `raid_share`, and the year's food and raid-food accumulators (2 f32, saved so mid-year saves
  continue identically).
- **Binary layout.** The band layout goes from `BANDS_V = 2` to **3** (`codec.rs:221`), keeping the
  v2 reader.
  - `use_skill`, `learn_curve`, `fight_calib` and the raid fields are byte-shuffled f32 columns,
    like `skill` and `quality` (`codec.rs:240-260`). `last_fight_switch` is a u32 column.
  - `beliefs` is a ragged block like the Known lists: a u8 count per band, then the entries as
    columns (item u16, slot and flags u8, then eight shuffled f32). Non-weapon entries have zero
    fight fields, which shuffle and compress to almost nothing.
  - The cap keeps it bounded: at most 768 bytes a band, typically about 450, before compression,
    plus about 40 bytes of band-level fields.
- Migration `v15_to_v16` is the identity in JSON (like `v14_to_v15`, `save.rs:342`). On load, an
  empty `use_skill` is **filled from the old recipe skills**: for each form, the best `Known.skill`
  among recipes whose output has that form and is makeable. So an old world continues with
  sensible user skills.
- On load, empty `beliefs` are **filled for the items in the current kit** as a starting state,
  the same way as founders (3.5): `n = BELIEF_NMAX / 2`, mean and variance from the true effects
  at the loaded state. These bands have used the tools for years.
  - Fight fields of kit weapons and guards get the visible-property estimate with
    `fight_n = FIGHT_N_START`. Old saves hold no record of past fights to learn from.
  - `fight_calib` is set to the model's baseline, `last_fight_switch` to "never", and `raid` to its
    prior with `raid_share = 0`.
  - Everything else starts from priors, and `learn_curve = LC_PRIOR`.
- `Known` is unchanged. Technique labels are derived when recipes are registered (never saved), so
  maker carry-over needs no format change.
- Old saves (v1-v15) keep loading through the existing chain; a v15 fixture save is added to the
  tests.
- `docs/SAVE_FORMAT.md` gets the v16 entry.


**Counters** (`counters!` registry, `world.rs:465`). These appear in `metrics.csv` and frames
automatically, and added columns are not a schema change (`DATA_SCHEMA.md` line 55). Each new one
is also listed with its meaning in `DATA_SCHEMA.md`, so streaming clients get them from the
start.
- **Kit:**
  - `kit_switches`;
  - `switches_conserve` (switch where the material shadow cost decided it, i.e. the new pick has
    less food but higher `U`);
  - `switches_fight` (the fight term decided it);
  - `switches_risk` (the risk penalty decided it).
- **Trials:**
  - `kit_trials`;
  - `trials_declined_risk` (would be taken without the risk penalty, rolled out with it);
  - `trial_food_lost` (x1000, food lost in trial seasons).
- **Makers:** `make_failures`, `make_scrap` (x1000, extra material used).
- **Users:** `use_taught` (user skill gained through close ties), `use_inherited_bands`.
- **Beliefs:**
  - `beliefs_formed` (priors made for a first use);
  - `tools_written_off`: a tool dropped within `WRITE_OFF_SEASONS` (4) of first use while its
    belief is below its prior. This is "an unlucky start put them off".
  - `tools_retried` (a trial of a previously written-off item);
  - `beliefs_told` (testimony received), `beliefs_inherited` (entries copied at splits);
  - `belief_err` (x1000, mean absolute relative error of kit beliefs, a gauge);
  - `beliefs_evicted`.
- **Fight beliefs:**
  - `fight_beliefs_updated` (item updates from clashes, own side and enemy side, not clashes);
  - `fight_beliefs_hunt` (fight updates from hunting seasons);
  - `fight_beliefs_told` (fight testimony from ties, captives and absorbed people);
  - `weapons_adopted_after_defeat`: a trial or switch to an item flagged "seen on a winner" within
    `ADOPT_AFTER_DEFEAT_YEARS` (5) of the defeat;
  - `fight_belief_err` (x1000, a gauge: mean absolute relative error of fight beliefs for kit
    weapons and guards).
- **Arms races:** `arms_race_switches`, the fight-decided switches made by bands in a rivalry
  (3.9).
- **Raiding (step 8):** `raids` (sought confrontations), `raid_food` (x1000, food taken in sought
  raids), `raid_deaths` (own deaths in sought raids).

**YearStats** (new fields, documented in `DATA_SCHEMA.md`):
- mean user skill of the forms in kits (Edge, Spear, Bow, Net, Boat);
- mean maker skill of hot-forging, steel-making, casting and smelting among bands that know them;
- the mean trial chance, and the mean and 90th-percentile `P_short`;
- the mean `fight_need`, and the share of weapons chosen above the best-for-food item;
- the mean belief entries per band, the mean `n` of kit beliefs, and the mean `learn_curve`;
- the mean believed fight strength `F_believed` and the mean true one, so their gap shows;
- arms races (3.9): `arms_races` (groups with 2 or more bands), `arms_race_max_bands`,
  `arms_race_max_share` (largest group's share of living bands) and `arms_race_max_span` (its
  width as a share of the map width). The guard rail reads the last two.
- raiding (step 8): `raider_bands` (bands that took at least `RAIDER_FOOD_SHARE`, 30%, of the
  year's food from raiding), `raid_food_share` (the world's share of food from raiding), and the
  mean `raid_share`.

**Events.** No per-band events: write-offs and belief updates would flood the log, so they are
counters. Optional minor events are appended after the last kind, so older ids do not move, and
all are listed in `DATA_SCHEMA.md`:
- `FirstSteelSmith`: the first band with steel-making maker skill of at least 0.6;
- `ToolWrittenOff`: the first write-off of a metal tool in a region (once per region and item
  kind), which gives stories like "the copper knife that failed at the river camp";
- `ArmedLikeVictors` (`band`, `other` = the band that beat it, `item`): the first adoption in a
  region of a weapon seen on a winner;
- `ArmsRace` (`band`, `other`, `recipe` = number of bands): when a group first reaches 3 bands;
- `RaiderBand` (step 8): when a band first takes over half a year's food by raiding.

`archive.rs:36` classes the last three as `CLASS_CONFLICT`, next to `Fight`, `Fled`, `Tribute`
and `Absorbed`.

**Streaming (coming):** frames reuse the codec band layout, so band layout 3 and its `use_skill`
column and the belief block should be in the stream frame layout from the start.
`DATA_SCHEMA` stays 1 (columns and event kinds are only added). The switch and trial counters
give a stream the "why" of each change.

**`DATA_SCHEMA.md` notes** to add with the columns:
- the fight-belief counters count *item updates*, not clashes (one clash usually updates 4-8);
- `fight_belief_err` and `belief_err` are gauges (the last season's value), not sums;
- `arms_race_*` fields are recomputed yearly from grudges and `last_fight_switch`; a group's
  bands can change from year to year;
- `raid_food_share` is food taken in *sought* raids; loot from chance clashes is still in
  `conflict_*`, as today.

## 5. Build order (small steps, each checked before the next)

**Every step's check, on GitHub Actions** (`headless-sims.yml`):
- **(a) early game:** 10 seeds x 500 years on 256x160, seeds 1..10;
- **(b) late game:** 10 seeds x 750 years on 800x800, seeds 499246292..499246301;
- **diffusion probes** (`probe: true`) on 128x80, 192x120 and 256x160, compared with
  `REPORT-diffusion.md` section 9.

Each step also gets `cargo test` (fixtures included), a load test of a v15 save, `./build-web.sh`
and, from step 1, `bench.yml`. Results go to a new `docs/PROGRESS-skills.md`.

**Guard rails, every step:**
- no continent crossing before Y100; (b) under 7/10; (a) at most 6/10;
- the war rate not doubled versus D1-D6 ((a) 0.320, (b) 0.271 %/y), over the whole map (local
  arms races can be fierce);
- from step 6: no single arms race spreads across the map (3.9: largest under 25% of living bands
  and half the map width, every year), and arms races do happen (at least one of 3 or more bands
  in half the (b) seeds);
- from step 8: raider bands under 10% of living bands in every year, and no raider group spans
  more than one region;
- no population collapse (no seed down more than about 20% without a clear cause);
- (b) seed 499246298 dies early as expected and counts in the "x/9" columns.

If a step misses, tune that step's own constants; no caps.

| # | step | contents | pass check | effort |
|---|---|---|---|---|
| 0 | **Measure first** | probe fixes (3.8), new counters and YearStats fields (zero yet) | sim results byte-identical to 7231103 (same `state_hash`); probes report the new split; baselines recorded | S-M |
| 1 | **Layer 1 and blade scoring** | `Effects` and `tool_effects`; the economy builds multipliers, upkeep and stock draw from it; the decision is temporarily plain "food minus labour" read from true effects (an interim; beliefs replace it at step 4), which fixes cause 2 | payout invariant (1); metal knives no longer "worse" against flint; flint and obsidian keep knives where local; bench at most +5%; guard rails | M |
| 2 | **User skill by form** (3.3) | `use_skill`, the motion table and transfer, yield factor, novice spread, practice and fade, founders at 0.7, inheritance, weaker teaching via close ties; save 16 and band layout 3 with all new fields (the belief block stays empty until step 4, so there is one format bump); fill-in on load | iron → steel "worse_held" mostly gone; population within ±10% (the novice factor costs food); v15 saves load; guard rails | M-L |
| 3 | **Maker skill** (3.4) | technique labels, carry-over, weighted geometric chain skill, effects on labour, scrap, quality and failure | steel-making lags hot-forging by years and varies by band; failures fall with practice; guard rails | M |
| 4 | **Beliefs** (3.5) | belief table and cap, priors from visible properties and similar tools, updates from payouts, `learn_curve`, fade, inheritance, testimony, founder and old-save fill; layer 2 switched to beliefs and cut off from `tool_effects`; belief counters | invariants 2 and 3 (perturbation test); belief tests; `belief_err` falls with `n`; write-offs happen but `tools_retried` is non-zero and good tools still spread; bench; guard rails | M-L |
| 5 | **Layer 2: risk and trials** (3.2, 3.6) | labour price, `P_short` risk penalty with ambition, explore value with curiosity, logistic trial chance with floor, trial food risk, kit choice pass with switch cost; `KIT_TRIAL_P` removed | trials fall as `P_short` rises (counters); bold or well-stocked bands try more; bench at most +5%; guard rails | M-L |
| 6 | **Layer 2: materials, fight need, fight beliefs and arms races** (3.2, 3.5, 3.9) | shadow prices from `want`, stock, `d.needed`, `d.uses`, `other_uses`; `fight_need` from pressure, grudges, dissent and ambition, no ceiling; fight terms in layer 1 with user and maker factors; fight beliefs from clashes (credit split, `fight_calib`), the losers' view of the winners, hunting, ties and captives; fade; grief dissent; rivalry and arms-race detection; their counters, YearStats and events | `switches_conserve` and `switches_fight` non-zero and local; `weapons_adopted_after_defeat` non-zero; `fight_belief_err` falls with clashes; arms races seen and local (guard rail); war rate not doubled; guard rails | L |
| 7 | **Calibration** | tune `USE_NOVICE`, `USE_TRANSFER`, `MAKER_*`, `BELIEF_*`, `PRIOR_*`, `RISK_*`, `OPP_*`, `FIGHT_*`, `W_*`, `DISSENT_GRIEF` on 3-4 seeds, then one full (a)+(b)+probe check | guard rails; more spread between seeds and regions in metal timing than D1-D6 | M |
| 8 | **Raiding as a way of life** (3.10; decision 17) | `raid_share` and lost food effort; practice scaled by effort; sought meetings and intent; the raid belief; the raid term in `U`; raid counters, YearStats and `RaiderBand`; then a short re-calibration | raider bands appear in some seeds and stay local (guard rail); the scenario test passes; population and war rate guard rails | L |

**Total:** about 4.5-5 weeks for steps 0-7, and about 6-6.5 weeks with step 8.
- Revision 1 was 1.5-2 weeks, revision 2 was 2.5-3, and revision 3 was 3.5-4.
- Beliefs (revision 3) added about a week: the table, priors and updates, the save block,
  inheritance and testimony, and their tests.
- Fight beliefs and arms races (revision 4) add about a week, mostly to step 6 (M → L): the
  clash evidence and credit split, the hunting transfer, testimony from ties and captives, arms
  race detection, and their tests. Calibration also has more to tune.
- Raiding (step 8) is about 1.5 weeks more, including its own re-calibration.

**Tests** (`civ-core/src/tests_diffusion.rs` and new `tests_skills.rs`):
- rewrite: `d3_trials_about_one_season_in_five` (109, becomes "trials follow risk and reward");
- retune: `d3_newcomer_is_tried_then_takes_over` (92) and
  `d3_metal_blades_outrank_stone_when_practised` (70, now "by cut, labour and materials");
- update for v16: `d5_last_ok_kept_and_saved` (177), `d5_old_saves_load_with_no_dates` (194);
- new, layer 1:
  - **payout invariant**: the economy's multipliers, upkeep, draw and failure come from
    `tool_effects`, and realised mean yield, failure rate and material draw match over many
    seasons;
  - novice spread is mean-preserving and shrinks with skill;
  - make failure is deterministic and falls with skill.
- new, skills:
  - a steel knife keeps the band's knife skill;
  - a first bow starts at its transfer value;
  - transfer is symmetric and 1 on the diagonal;
  - founders start at 0.7;
  - daughters get more user skill than tied partners;
  - chain skill: raising any step raises `m_item`, a longer chain of equally skilled steps does
    not lower it, and one weak step pulls it down;
  - steel-making does not inherit hot-forging skill, while a steel knife inherits iron-knife
    forging skill.
- new, decisions:
  - **a hungry band with thin stores declines a risky trial, while a bold band (high ambition)
    or a well-stocked one, otherwise identical, takes it**;
  - **a band switches its knife from copper to flint (or iron) when copper is scarce and needed
    elsewhere, and switches back when copper is plentiful**;
  - a band with high `pressed` or grudges picks the spear it *believes* tougher, and a peaceful
    twin picks the best hunter;
  - the switch cost prevents flip-flopping between near-equal tools;
  - hunger still raises invention.
- new, beliefs:
  - **convergence:** with steady use and fixed skill, a belief's mean converges to the true mean
    multiplier (within a few % after `BELIEF_NMAX` seasons), and `var/n` shrinks;
  - **luck shifts the choice:** two identical bands, differing only in the noise draw of the first
    trial season, end up with different kit picks (the unlucky one drops the tool, the lucky one
    keeps it);
  - **sensible priors:**
    - a band that has used iron knives rates a steel knife above iron, with a narrow prior;
    - a band with nothing like a bow gets a wide prior near its average ratio;
    - visible properties order priors the same way as true material terms;
  - **no peeking (perturbation test):**
    - changing the true effects of a tool the band has never used does not change the band's
      decisions, until it uses it;
    - after first use, the decision changes only through the observed payout;
    - a compile-time check that the decision module does not import `tool_effects`;
  - **fade and re-trial:** a written-off tool's `n` fades, and a curious band re-tries it within a
    few years, adopting it if it really is better;
  - **inheritance:** daughters get beliefs with lower `n` and wider variance;
  - **testimony:** a told belief moves the listener's mean toward the teller's, never past
    `TELL_NMAX`, and less than one season of own use;
  - **bounded:** a band never holds more than `BELIEF_CAP` entries, and eviction keeps kit items.
- new, fight beliefs:
  - **defeat teaches:** a band beaten by bronze spears, never having held one, ends with a higher
    belief in bronze spears than before, flagged "seen on a winner". If it can make them and has
    reason to fight, it trials or adopts them, and `weapons_adopted_after_defeat` counts it.
  - **a clash outweighs a hunt:** the same surprise moves the fight belief much more from one
    clash than from one hunting season. Hunting alone never takes `fight_n` past
    `HUNT_FIGHT_NMAX`. A shield gets nothing from hunting.
  - **no peeking for fight value:** changing the true fight term of a weapon (its user or maker
    factor) or `lethality` does not change any band's fight beliefs or choices until a clash (or
    a hunting season, for the user factor) shows it. Changing an *enemy's* true weapons changes
    only what is seen in clashes with it.
  - **credit split:** in a clash where a band carries a spear, an axe and a shield, the weights
    given out on each side add up to `W_CLASH * cas`; the spear gets the larger weapon share;
    the shield is credited only through the guard part.
  - **winner and loser differ:** the loser's belief about the winner's weapons moves more than the
    winner's belief about the loser's (`W_SEEN_LOSER` against `W_SEEN_WINNER`).
  - **fade:** with no clashes, fight beliefs return toward their prior within a few years.
  - **testimony:** a tie's fight news and captives' beliefs move the listener less than its own
    clash.
- new, arms races (3.9):
  - **a race can start and is local:** two rival bands with metal and repeated clashes both make
    fight-decided switches and show up as an arms race. Their `fight_need` and believed
    strength rise with no ceiling.
  - **costs bound it:** with copper scarce and needed elsewhere, or food near need, the race slows
    (fewer fight-decided switches) compared with a rich twin pair.
  - **it ends:** once one side flees or is absorbed, both sides' `fight_need` and believed weapon
    worth fall within some years, and food picks return.
  - detection: union-find on a hand-built set of rivalries gives the right groups and spans.
- new, raiding (step 8):
  - **Michael's scenario:** a band with strong believed arms (bronze spears and shield), faded
    hunting and farming skill and food beliefs, and three weaker, stocked neighbours in reach
    raises its `raid_share` and takes at least `RAIDER_FOOD_SHARE` of its food by raiding within
    5 years. An otherwise identical twin with good hunting skill raids clearly less.
  - **lost effort is real:** raiding lowers food-slot practice, and over years the raider's food
    skills fade further.
  - **limits:** when the neighbours are emptied, flee or arm up, the raider's raid belief falls and
    its `raid_share` drops. With no targets left, it goes back to food work, at its faded skill.
  - **no peeking:** the raid choice changes only through observed raids and clashes, and the
    targets' visible arms and size.
- save tests: a v15 save loads and fills `use_skill` and the kit beliefs (fight fields included);
  save, load, continue gives the same `state_hash` (beliefs, `fight_calib`, `last_fight_switch`
  and the raid fields included).
- Determinism tests (`tests.rs:17, 36, 94, 127`) stay; every world's `state_hash` changes from
  step 1.

## 6. Constants (starting values, to tune)

| constant | start | meaning |
|---|---|---|
| **user skill** | | |
| `USE_NOVICE` | 0.5 | yield factor of a complete novice (expert = 1.0) |
| `USE_FOUNDER` | 0.7 | founders' skill with their starting tools (Michael) |
| `USE_PRACTICE` | 0.08 | gain per season used (`* (1 - u)`) |
| `USE_FADE_X` | 0.5 | unused-form fade, times the band's `floss` |
| `USE_TRANSFER` | 0.6 | scale on motion-profile similarity between different forms |
| `USE_INHERIT` | 0.9 | share of a parent's user skill a daughter keeps |
| `USE_TEACH_P` / `USE_TEACH_SHARE` | 0.2 / 0.15 | close-tie teaching chance a year and gap closed (seacraft uses 0.25) |
| `NOVICE_SD` | 0.3 | season-to-season yield spread of a complete novice |
| **maker skill** | | |
| `TECH_CARRY` | 0.5 | maker skill carried to another recipe of the same technique |
| `CHAIN_EPS` / `CHAIN_W_MIN` | 0.05 / 0.1 | floor on a step's skill and on its weight in the geometric mean |
| `MAKER_WASTE` | 0.8 | extra labour of an unskilled maker |
| `MAKER_SCRAP` | 1.0 | extra material used up by an unskilled maker |
| `MAKER_FAIL` | 0.25 | season failure chance at maker skill 0 (`* (1-m)^2`) |
| **decision: labour and materials** | | |
| `LAB_FLOOR` | 0.2 | labour price with all labour spare, as a share of its full price |
| `OPP_W` | 0.5 | food value of one unit of material at worth 1 |
| `OPP_USES` | 0.25 | extra shadow price per other use of the material |
| `KIT_CANDIDATES` | 3 | candidates per slot (plus the current item) |
| **decision: risk and trials** | | |
| `BASE_SD` | 0.15 | base harvest spread, as a share of food |
| `SHORT_LINE` | 0.9 | food (with stores) below this share of need counts as going short |
| `STORE_USE` | 1.0 | share of stores counted as a buffer |
| `RISK_STARVE` | 3.0 | penalty per unit `P_short`, in seasons of need |
| `RISK_BOLD` | 1.0 | how much ambition lowers the penalty (`exp(-RISK_BOLD*ambition)`) |
| `TRIAL_SHARE` | 0.25 | share of the slot's work done with the challenger in a trial |
| `EXPLORE_W` / `EXPLORE_HORIZON` | 0.5 / 8 | weight and horizon (seasons) of the value of learning |
| `TRIAL_TEMP` | 0.02 | graded-choice width, as a share of need |
| `TRIAL_MIN` / `TRIAL_MAX` | 0.02 / 0.5 | floor and ceiling of the trial chance |
| **decision: fight need** | | |
| `FIGHT_W` | 0.3 | food value of fight strength at full fight need, as a share of need |
| `FIGHT_BASE` / `FIGHT_PRESS` / `FIGHT_GRUDGE` / `FIGHT_DISSENT` / `FIGHT_BOLD` | 0.1 / 1.0 / 0.5 / 0.2 / 0.2 | terms of `fight_need` (no ceiling; `FIGHT_MAX` dropped in revision 4) |
| `FIGHT_RAID` | 0.5 | extra fight need per unit of `raid_share` (step 8) |
| `FIGHT_GUARD_W` | 1.0 | weight of believed loss reduction against believed weapon terms in `F_believed` |
| **fight beliefs** | | |
| `W_CLASH` | 2.0 | evidence weight of one clash (in seasons), before the casualty scale |
| `CAS_REF` | 6 | deaths at which a clash counts in full |
| `WIN_Z` | 0.5 | weight of the win/loss surprise in `z` |
| `W_SEEN_LOSER` / `W_SEEN_WINNER` | 1.5 / 0.5 | weight of the enemy-side evidence for the loser and the winner |
| `W_YIELD` | 0.5 | evidence weight of a flight or tribute (win/loss only) |
| `FIGHT_NMAX` | 6 | cap on `fight_n` (old clashes fade beyond it) |
| `FIGHT_PRIOR_N` / `FIGHT_PRIOR_CV` | 0.5 / 0.4 | weight and spread of the visible-property prior |
| `FIGHT_N_START` | 1.0 | `fight_n` of founders' and old saves' starting fight beliefs |
| `FIGHT_CALIB_RATE` | 0.2 | learning rate of `fight_calib` |
| `HUNT_FIGHT_W` / `HUNT_FIGHT_NMAX` | 0.1 / 1.5 | weight of a hunting season as fight evidence (times motion similarity), and the most `fight_n` hunting alone can give |
| `FIGHT_NEWS_YEARS` | 3 | how recent a tie's clash must be to be told |
| `ADOPT_AFTER_DEFEAT_YEARS` | 5 | window for `weapons_adopted_after_defeat` |
| **arms races** | | |
| `DISSENT_GRIEF` | 0.5 | friction added per unit of own fight deaths over population |
| `ARMS_GRUDGE` / `ARMS_WINDOW` | 0.3 / 10 y | mutual grudge and switch window that make a rivalry |
| `ARMS_MAX_SHARE` | 0.25 | guard rail: largest arms race's share of living bands (a check, not a limit in the sim) |
| **raiding (step 8)** | | |
| `RAID_SEEK` | 0.5 | chance a season of a sought meeting at full raid share |
| `RAID_INTENT` | 1.0 | tension added in a sought meeting at full raid share |
| `RAID_PRIOR` | 0.3 | prior raid take per season, as a share of need, at even believed odds |
| `PERSON_W` | 8 | value of a band member lost, in seasons of their need |
| `RAIDER_FOOD_SHARE` | 0.3 | share of a year's food from raiding that makes a "raider band" |
| **beliefs** | | |
| `BELIEF_CAP` | 24 | belief entries a band can hold |
| `BELIEF_NMAX` | 8 | cap on effective observations (old seasons fade beyond it) |
| `BELIEF_ADOPT_N` | 1 | own seasons of use needed before a full switch (else trial only) |
| `BELIEF_VAR_MIN` | 0.0025 | floor on believed variance, as a share of mean squared (5% sd) |
| `BELIEF_FADE` | 0.8 | yearly factor on `n` of unused beliefs (pulled back toward the prior) |
| `BELIEF_INHERIT_N` / `BELIEF_WIDEN` | 0.6 / 0.5 | daughters' share of `n`, and the variance widening for inherited or told beliefs |
| `BELIEF_TELL_P` / `TELL_W` / `TELL_NMAX` | 0.2 / 0.5 / 1.0 | testimony chance a year, its weight, and the cap on `n` from hearsay |
| `BELIEF_FAIL_N0` | 2 | prior strength of the believed failure chance |
| `PRIOR_N` | 0.5 | weight of a prior against one season of own use |
| `PRIOR_N0` | 1.0 | pull of the band's average ratio when few similar tools are known |
| `PRIOR_CV` / `PRIOR_NOVEL` | 0.25 / 2.0 | prior spread, and its extra for tools unlike anything used |
| `PRIOR_FAIL` | 0.15 | believed failure chance with no same-technique experience |
| `LC_PRIOR` / `LC_RATE` | 1.3 / 0.2 | starting learned learning curve, and its update rate |
| `INFO_W` | 0.3 | weight of finding out (uncertainty) in the value of a trial |
| `WRITE_OFF_SEASONS` | 4 | window for counting a drop as a write-off |
| **probe** | | |
| `PROBE_TIE` | 0.02 | probe gap counted as a tie |

`KIT_TRIAL_P` (0.2) and the edge-based knife ranking go away. `KIT_UPKEEP` and the economy's upkeep
become the single layer-1 labour formula.

## 7. Expected effect, risks and decisions

**Expected effect on the early, uniform metal age** (`REPORT-diffusion.md` 9.6.1: steel in every
(a) world by Y188-306, steel knives at half the bands 20-40 years after the first):
- **Step 1:** copper and iron knives lose to local flint and obsidian more often, so the knife slot
  turns to metal later and more patchily. Spears change less (their fight value comes back in
  step 6).
- **Step 2:** once a band uses metal, iron → steel is quick for users. The "iron held while steel
  waits" years mostly vanish.
- **Step 3:** the real delay moves to makers. Steel-making is learned from scratch, fails often at
  first, costs labour and iron, and needs charcoal, water and a hearth.
- **Step 4:** adoption becomes bumpier and more local. Some bands are put off by bad first seasons,
  and good reports travel along ties ahead of the tools.
- **Step 5:** comfortable, bold, curious regions adopt first, and hungry frontiers wait for a safe
  year. The spread comes from situation and temperament, not from a timer.
- **Step 6:** where metal is scarce and wanted for other things, bands save it. Under threat they
  carry the weapons they believe in. A defeat by better arms spreads those arms to the losers and
  their ties. Some rival pairs and small groups race into metal weapons ahead of their
  neighbours, and the race ends when one side is gone. Weapon change becomes more local and more
  tied to war history.
- **Step 8:** in some seeds a few bands near weaker, stocked neighbours live largely by raiding
  for a while. They fall back to hunting or farming, badly, when the targets are gone.
- Expect steel later, in fewer (a) worlds, and more uneven between regions. No target is set; steel
  timing is watched as a diversity measure (first year and share of bands by region, per seed).

**Risks**
- **Food and population.** The novice factor, novice spread and trial food loss cost food. Watched
  every step (±10% in (a)).
- **Lock-in.** Very cautious bands could stay with stone forever. The `TRIAL_MIN` floor, the value
  of learning and the carry-over keep doors open, and the probes check "worse_held" streak lengths.
- **Writing off good tools for good.** An unlucky first season can sink a belief, and then the tool
  is never used again to correct it. Four things keep the door open:
  - the `TRIAL_MIN` floor;
  - fade, which brings doubt back;
  - the curiosity-driven value of finding out, which grows with doubt;
  - testimony from luckier neighbours.

  `tools_written_off` against `tools_retried` and the re-trial test watch it. If write-offs still
  stick, raise `INFO_W` or `BELIEF_FADE`'s pull before anything else.
- **Systematic pessimism about new kinds of tool.** First seasons are novice seasons, so new forms
  look worse than they will be. The learned `learn_curve` corrects part of this, and the
  probe's belief error by `n` shows how much is left.
- **Herding and rumour.** Testimony could spread a bad (or good) reputation faster than the truth.
  `TELL_NMAX` keeps hearsay below one season of own use.
- **Probe interpretation.** "Worse in food" no longer means a bug. The three-way probe split (3.8)
  tells judgement, information and true trade-offs apart.
- **Determinism.** Beliefs update in band order with `hash3` noise only, so runs repeat. Save, load,
  continue is covered by the `state_hash` test.
- **Over-tuned decision layer.** It has many constants. Each comes in at its own step with its own
  check (risk at 5, materials and fight at 6, raiding at 8), and the switch counters say which
  term decided each switch, so a misbehaving term is visible.
- **Runaway arms races.** There is a feedback: defeat raises belief and fight need, so the band
  rearms, which raises the rival's fight need. With no caps, only the costs bound it (3.9).
  - Why it should not run away: each step costs food, labour and materials; deaths and grief
    thin the band; threats fade when fighting stops; and clashes are local.
  - Beliefs also follow the evidence. Fight value is relative to the enemy (`z` compares
    against what was expected), so when both sides rearm, neither keeps getting good surprises.
  - Watched by the arms-race guard rail (no group over 25% of bands or half the map), the war
    rate guard rail, `arms_race_*` and the grief counters.
  - If races do spread too far, raise the costs (`DISSENT_GRIEF`, upkeep of metal weapons) or
    speed the fading before anything else, not a cap.
- **Too few arms races.** The opposite failure: costs so steep that nothing ever races. The guard
  rail checks that races appear in half the (b) seeds.
- **Fight beliefs learned from little.** Clashes are rare, so many bands keep near-prior fight
  beliefs for long. That is realistic. The losers' view of winners, testimony and hunting add
  evidence, and `fight_belief_err` shows how much is left.
- **Novice fighters.** The user factor in the fight term (3.1) makes newly armed bands weaker in
  their first seasons, which can shift war outcomes. The war-rate guard rail and the probes watch
  this.
- **Raiders taking over (step 8).** A raider band that absorbs victims grows. The limits are
  depletion, flight, victims arming, losses and splits (3.10), checked by the raider guard rail
  (under 10% of bands, one region at most). Coalitions against raiders would be the natural
  counter, and they are later politics work. If raiders spread, that is the first sign the
  follow-on is needed.
- **Too slow metal.** Maker failure, waste and material conservation together could stall metal
  where it was fine. Step 6 tunes them on a few seeds first.
- **Performance and memory.** Bounded as in 3.5 and 3.6. `bench.yml` runs at steps 1, 4, 5, 6
  and 8, with at most +5% in total. Beliefs add at most 768 bytes a band, typically about 450;
  the yearly arms-race pass is a union-find over at most 6 grudges a band.
- **Save format.** One bump (16) at step 2 with all new fields, following the v14 → v15 pattern;
  fixture test added.
- **Web build:** `civ-core` only, no new dependencies.

**Decisions (Michael, Oct 28 2026)**
1. Trials can cost food, based on the challenger's calculated yield at current skill.
2. User skill affects yields, novice 0.5x to expert 1.0x (to tune).
3. Transfer between tool kinds is a general model (shared motions and uses), not hand-picked
   pairs; no sling is added.
4. Maker skill affects labour, materials used up, quality and failure chance.
5. Close ties can teach user skill, less effectively than parents pass it to daughters.
6. Hunger keeps raising invention while lowering trials (through risk, not a rule).
7. **Two layers (revision 2):** one shared function gives a tool's true effects, and the payout
   uses them. The choice is the band's own valuation, given its economy (stores, food, stress,
   spare labour, materials, neighbours) and temperament (curiosity, ambition).
8. **Risk against reward:** bands weigh expected gain against the chance of going short. Failure
   that could mean starvation makes the risky play much less likely; boldness and stores make it
   more likely.
9. **Material opportunity cost:** bands weigh the supplies each tool uses against what else those
   supplies are for, and may switch materials to conserve one.
10. **Maker skill of a chain:** all steps contribute (weighted geometric mean, 3.4).
11. **Fight need:** bands value fight-worthy weapons more when they expect fighting soon.
12. **Founders** start at user skill 0.7 for their starting tools.
13. **Beliefs (revision 3):** the payout keeps true effects with real season-to-season spread.
    - **Before first use:** a band estimates a tool from what it knows (the material's properties
      and similar tools it has used), with extra doubt.
    - **After first use:** it goes by its own past results, luck included. Doubt shrinks with use.
    - The decision uses beliefs, not true effects. The invariant is "decision inputs = beliefs,
      and beliefs come only from priors and observed payouts".
    - Proposed with this, for Michael to confirm or change: one belief per item (form +
      material), at most 24 a band; daughters inherit beliefs with less weight and more doubt;
      close ties pass weaker testimony; unused beliefs fade.
14. **Fight value is a belief, not an exception (revision 4).** The visible-property estimate is
    only the prior.
    - A band learns a weapon's worth in a fight from its own clashes (strongest), from what beat it
      (the loser learns most from the winner's arms), from ties' and captives' accounts, and
      weakly from hunting, scaled by motion similarity.
    - Credit is split by each item's share of the fight model. Fight beliefs fade like others.
    - Founders and old saves get only a starting state.
    - Proposed with this: `W_CLASH` 2 seasons a clash, hunting at most 0.1 a season, the loser
      weighting 1.5 against 0.5.
15. **Arms races are allowed and visible (revision 4).** No caps or artificial limits: `FIGHT_MAX`
    is dropped. Races are bounded by labour, materials, food and hunting effort, deaths, grief
    dissent, and threat fading as conflicts end. Counters and yearly fields show them, and a guard
    rail checks that no single race spreads across the map.
16. **Raiding as a livelihood (revision 4, scenario).** A band that has lost its food skills and
    holds strong arms near weaker neighbours may live by raiding. It is weighed in the same
    decision layer by beliefs learned from past raids, with risk from casualties and retaliation,
    and bounded by depletion, flight, victims arming and the raiders' own losses. Built as
    step 8 (decision 17); coalitions and lasting tribute are later politics work.
17. **Raiding ships with this proposal (Oct 28 2026, answer to revision 4's open question 1).**
    It is step 8, the last step, about 1.5 weeks with its own re-calibration; its save fields go
    into format 16 with the rest.
18. **Grief dissent is accepted (Oct 28 2026, answer to open question 2).** A band's own recent
    fight deaths add `DISSENT_GRIEF * deaths/pop` to its friction (3.9). It is the one new cost
    mechanism added to bound arms races and raiding; all other bounds are existing costs.

**Open questions:** none (both of revision 4's were answered: decisions 17 and 18).

The other choices beyond Michael's answers (the belief key, inheritance, testimony, fade, and the
fight-evidence weights in decision 14) are proposals, and he can change them in review.
