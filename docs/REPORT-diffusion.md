# Why inventions get dropped or fail to spread: diagnosis report

Michael's question (2026-10-27): useful things seem to get used and passed on less than
expected, maybe because new tech struggles against the prevalence of old tech even when
it is much better. This report is diagnosis only. Nothing in the sim was changed. The
instrumentation is a read-only `cfg(test)` probe, checked byte-identical (same civsave
sha256 with and without it). Method and raw notes are in `docs/PROGRESS-diffusion.md`.

## Data

| source | maps | what |
|---|---|---|
| CI run [40606688049](https://github.com/Macrolol/kindling/actions/runs/40606688049) | 128x80, 8 seeds, 1500 y | probe + sims |
| CI run [40606696457](https://github.com/Macrolol/kindling/actions/runs/40606696457) | 192x120, 8 seeds, 1200 y | probe + sims |
| CI run [40606704688](https://github.com/Macrolol/kindling/actions/runs/40606704688) | 256x160, 5 seeds, 1000 y | probe + sims (see section 7) |
| historical CI pre-releases, `docs/results-inv-step*.txt` | default, 800x800 | frames lore, steps 0-13 |
| local probe, seed 1 | 128x80, 600 y | `probe_diffusion`, `probe_hide` |

Only CI-generated saves and outputs were used; no user saves.

Probe terms:
- **known**: the band holds the recipe.
- **make**: its inputs are available, so it is producible.
- **kit**: chosen for a job.
- **forget reasons**:
  - no_input: unused because the inputs are missing
  - displaced: another item holds the slot
  - unneeded: an intermediate whose end product isn't chosen
  - idle_slot: a context gate, e.g. ornaments, idle tallies
  - redundant: another recipe makes the same item

## 1. Headline: knowledge spreads fast, making and keeping it does not

| metric | 128x80 | 192x120 |
|---|---|---|
| forgets: no_input / displaced / unneeded / redundant / idle_slot % | 58.7 / 22.9 / 11.2 / 4.6 / 2.7 | 58.6 / 24.6 / 9.3 / 4.3 / 3.1 |
| taught recipes arriving producible (learner has inputs) % | 44.5 | 44.0 |
| taught / inherited / invented gains later forgotten % | 73.6 / 28.9 / 78.4 | 70.8 / 28.5 / 75.8 |
| forgotten without any practice after the learning season % | 75.5 | 77.9 |

- Most recipes reach 50% of bands within 10-40 years of first appearing (t50).
  Wool cord is the exception (t50 115-215 y).
- The gap is between knowing and making:

| item | known | make | kit |
|---|---|---|---|
| hide thong | 0.89-0.91 | 0.08-0.12 | 0.08-0.12 |
| bone point | 0.91 | 0.21 | 0.06 |
| cooked meat | 0.98 | 0.28 | 0.28 |

  Values are second-half means across runs.
- Historical CI frames show the same since step 0. forgot/learned is 0.86-0.92 on every
  step. Animal-product items show the same peak → final collapse at every step, e.g.:
  - hide thong 0.63 → 0.16
  - cooked meat 0.98 → 0.34
  - bone beads 0.39 → 0.10

  So it is structural, not a recent regression. Caveat: frames `lore` counts bands *able
  to make* an item, not bands that know it (see B4).
- The results are the same across both map sizes, all seeds, and the local run, to
  within 1-3 points.

**On the incumbency hypothesis.** Mostly the opposite happens. `make_kit` switches to a
new item the moment it is producible and has a higher material score, whatever the
band's skill at the old one. Then the old recipe decays as "displaced":
- reed bundle boat 99%
- sharpened-stick spear 98%
- stone blade 100%
- plant-fibre snare and net 95-99%

There is one real case of "better tech loses to old tech": metal edges vs flint and
obsidian (section 3). It comes from the scoring rule, not from prevalence.

## 2. Most dropped or under-spread techs, and why

| tech | evidence (128x80 / 192x120) | main cause |
|---|---|---|
| hide thong, hide clothing, hide bag, bone point, rawhide snare, hide net | known ~0.9, make 0.04-0.21; forgets 93-98% no_input | **B1** hide/bone stock budget unreachable |
| cooked meat / fish | known 0.96-0.98, make 0.21-0.28; 94-95% no_input | game/fish scarcity + B1 for hunting kit; partly realistic |
| bone beads, copper/bronze/gold rings, obsidian pendant | bone beads peak 0.64-0.66 → final 0.08-0.09, 28-47 extinctions; idle_slot 38% (beads), 63-68% (copper ring), 85-88% (bronze ring), 55% (pendant) | **B2** ornaments never practised |
| copper knife, copper-tipped spear | make 0.33-0.42, kit 0.11-0.17 | **B3** edge scoring underrates metal |
| iron knife | make 0.50-0.53, kit 0.05-0.09 | displaced by steel in the same seed (fine); vs obsidian, B3 |
| warp-weighted loom, linen cloth, clay/fired moulds, crucibles | loom 99% unneeded (make 0.79, kit 0); moulds 96/79%; crucible of copper 72% | **I3** chain intermediates forgotten between uses |
| copper, tin, salt, obsidian, metals generally | extinctions in 128x80: tin 40, copper 27, salt 15; woollen bag 28, hide bellows 16 | geology + trade reach: mostly realistic (I1) |
| sealed jar | known 0.81-0.88, make = kit 0.21-0.27; 100% of forgets no_input | pitch/resinous wood (step-12 design); realistic, input-bound |
| plank boat, woven mat sail, linen sail, fired clay tokens | make = kit wherever makeable (plank boat 0.39-0.55, sail 0.38-0.48, tokens 0.59-0.72) | input-bound only; **not** losing to incumbents |
| nailed plank ship | 1 seed of 16, peak 0.30; 95% no_input | needs nails + pitch: depends on the whole metal chain |
| charts | first chart in 10/16 seeds, peak 0-17 bands, 0 at end in 14/16 | needs records >= 0.4 and pilot >= 0.4 and a boat; not a recipe, so the probe doesn't track it (see I4) |
| wool cord / wool line | t50 115-215 y; make 0.44-0.47 but kit 0.02-0.05 | loses the cord slot to fibre/hide thong by material score; wool is herd-bound |

There are no "drying rack" items. Smoking is a kit flag (`kit.smoke`), so this probe
could not assess it.

## 3. Is newer, better tech losing to incumbents?

The edge, point and spear slots use `score(q, m) = q*(0.75+0.25*durability)`:

| material | cut slot (q = cut_q) | spear slot | axe slot | weapon_q edge (0.55 cut + 0.45 dur) |
|---|---|---|---|---|
| stone | 0.376 | 0.558 | 0.449 | 0.49 |
| flint | 0.625 | 0.695 | 0.433 | 0.55 |
| obsidian | 0.641 | 0.690 | 0.369 | 0.50 |
| copper | 0.480 | 0.662 | **0.740** | 0.71 |
| bronze | 0.650 | 0.772 | 0.825 | 0.81 |
| iron | 0.640 | 0.766 | 0.820 | 0.80 |
| steel | 0.776 | 0.853 | 0.875 | 0.88 |

- Copper loses the knife and spear slots to flint and obsidian. Iron ties or loses
  against obsidian and only just beats flint; bronze barely beats flint.
- The data matches. Bands that can make a copper knife carry it only about one time in
  three (make 0.38-0.42, kit 0.13-0.14). The same holds for copper-tipped spears (make
  0.33-0.36, kit 0.11-0.17).
- Copper axes win their slot easily and are carried whenever makeable (kit 0.24-0.36 of
  make 0.38-0.40; the gap is mainly bronze or iron axes replacing them).
- Elsewhere the sim values durability much more:
  - `weapon_q` gives 45% weight to durability.
  - Upkeep labour scales with `1.15 - durability` (flint 0.86 vs copper 0.15, about 6x).
- So the chooser undervalues the main economic advantage of metal that the sim itself
  models.
- Realism: early copper knives really were no sharper than flint, and flint stayed in
  use well into the Bronze Age, so "copper loses the knife slot" is defensible. Iron
  losing to obsidian is not: iron replaced stone edges almost everywhere because it
  holds up and can be re-sharpened.
- The reverse failure is more common: **worse-in-practice newcomers win**. The
  refined-payoff comparison at the moment of adoption shows the newcomer worse in:
  - hide net 47-58% of adoptions (mean gain -0.03 to -0.06)
  - rawhide snare 53-57%
  - iron-tipped spear 18-52%
  - iron knife 21-45%
  - bronze knife 5-31%
  - steel knife 0-22%

  These items win on material score while the band is unskilled with them. That is
  quality-blind choice (B3), not incumbency.
- Boats, sails, sealed jars, tokens and plank boats never lose to an incumbent. Make
  equals kit wherever they are makeable; their spread is capped by inputs.

## 4. Bugs (or unintended interactions)

**B1. Hide and bone are never `present`, so their stock budget can't be met.**
- In `world.rs` (stock upkeep, ~1807), Hide and Bone fall through to the `DEPOSITS` arm,
  which never matches them, so `present` is always false.
- Bands that use hide items therefore pay 0.02 + 0.06 per season but gain only
  `0.01*hunted`. Break-even is a big-game harvest of 8 per season (2 without the
  penalty).
- `probe_hide`: needers hunt 3.1-4.5 on average and only 3-14% reach 8. That holds even
  at year 100, when 96% of game is left. Game depletion comes later (0.61 at y200, 0.13
  at y600).
- This drives the largest single block of no_input forgets and the hide/bone family's
  collapse.

**B2. Worn ornaments are never practised.**
- `economy` skips `kit.ornament` so ornaments don't wear out ("kept for show, not worn
  out at work", commit 4a01c50). The same `continue` also stops their via-chain being
  marked used.
- The only other `mark_used` caller is experiment inputs, so every ornament recipe
  always decays, even while worn every season.
- Result: bone beads go 0.64 → 0.09 with 28-47 extinctions per batch; rings and pendants
  idle_slot 55-88%.

**B3. Kit choice ignores both skill and the durability the economy rewards.**
- `make_kit` ranks by material score only. `refine_kit` scales effects afterwards by
  small amounts.
- Two effects:
  - (a) Unskilled newcomers displace honed incumbents (section 3, worse_refined 0.2-0.58).
  - (b) Edge slots weigh durability at 25%, while `weapon_q` uses 45% and upkeep about 6x
    differences.
- (a) is a design gap, not a coding bug. (b) is an internal inconsistency.

**B4. Reporting: "lost worldwide" means "nobody can make it now".**
- frames `lore` (`World::item_bands`) and `LostWorldwide` count bands able to make the
  item with the materials at hand.
- So the "collapse" in historical results mixes input loss (hide) with real knowledge
  loss. Past reports overstated knowledge loss for input-bound items.

## 5. Realistic inertia (keep, maybe tune)

- **I1. Geology and trade reach.**
  - Copper, tin, salt and obsidian are local and get lost when the bands near them die
    or move.
  - Trade gives raw inputs of known recipes only weight 0.5 (`want`).
  - That is realistic. A slightly higher pull for scarce inputs of recipes the band
    actually *used recently* could be tuned emergently.
- **I2. Teaching without inputs.**
  - 56% of taught recipes reach bands that cannot make them, and those fade within about
    13 years.
  - Real "heard of it" knowledge also fades, but this is the main reason 71-74% of taught
    gains are lost. Teaching picks one random recipe per meeting, whatever its relevance.
- **I3. Chain intermediates.**
  - Looms, moulds and crucibles are only practised while their end product is in the kit.
  - Real crafts kept tools between uses. The current rule makes deep chains fragile:
    loom 99% unneeded, moulds 79-96%.
- **I4. Charts and sea lore.**
  - Rare and short-lived (max 17 bands). They need two separately fading skills (records
    and pilot, `PILOT_FADE` 0.998 per season) at >= 0.4 plus a boat.
  - Plausible, but a dedicated probe would be needed to tell which threshold binds.

## 6. Proposed adjustments (ranked; not implemented)

1. **Fix hide/bone `present` (B1).**
   - Change: treat Hide and Bone as present when the band hunted big game this season
     (e.g. `hunted > 1`), or drop the `needed && !present` penalty for animal products and
     let the per-season use match a sustainable harvest.
   - Realism: hunters process hides and bones from their own kills; they don't need a
     "deposit".
   - Expected effect: biggest single change. Hide and bone make-share should move from
     about 0.1 toward the share of bands that hunt. No_input forgets should drop sharply,
     and the animal-product family's peak → final collapse should shrink. Game depletion
     will still bite later, so diversity stays.
   - Risk: hide clothing and bags are cheap and broad, so cold-climate and carrying
     advantages may grow. Possibly more hunting pressure and faster game depletion.
     Re-check cold-region survival and game_left curves.
2. **Practise worn ornaments without wearing them (B2).**
   - Change: mark the ornament's via-chain used, but keep it out of upkeep and the
     `needed` stock penalty.
   - Realism: things worn and given every day are remembered.
   - Expected effect: ornaments persist where their materials do. Gift and prestige
     dynamics (gold, feasts) get stable carriers.
   - Risk: low. More ornament-driven trade demand; check gold/prestige summaries.
3. **Skill-aware, cost-aware kit choice (B3), emergent S-curves.**
   - Change: rank candidates by expected payoff = material score × refined quality, minus
     an upkeep term using the same `1.15 - durability` the economy charges. Give
     challengers occasional trial use (curiosity / `refine` time) so they gain skill and
     win once they pay.
   - Realism: newcomers take over gradually as skill builds. Old tech hangs on where it
     is honed. Durable metal wins when upkeep matters, flint stays where it is local and
     skill is high.
   - Expected effect: fewer worse-in-practice adoptions (hide net, iron spear), slower
     but more stable replacement, and less churn ("displaced" 23-25% of forgets should
     fall). Copper knives would be adopted where upkeep dominates, iron would clearly beat
     obsidian, and outcomes would vary more by region.
   - Risk: medium. It touches every slot. It could slow early metal adoption overall, so
     tune with CI on all three sizes.
4. **Relevance-weighted teaching and slower "heard-of" fade (I2).**
   - Change: weight the candidate choice in `exchange` by the learner's available
     materials and wants (it is uniform now), and/or halve decay for recipes that have
     never been producible since learning.
   - Realism: people pass on what is useful to the listener.
   - Expected effect: taught-and-producible should rise above 44%, and taught-forget
     should fall below 70%. Knowledge reservoirs would survive input droughts.
   - Risk: low-medium. Faster spread of everything; watch for homogenised outcomes.
5. **Keep tools of a chain while its product is recent (I3), and soften input-scarcity
   forgetting.**
   - Change: count intermediates as practised while the end product was used within N
     years. Decay a no_input recipe more slowly while its inputs were seen or traded
     recently.
   - Realism: looms and moulds sit idle between seasons without being forgotten.
   - Expected effect: textile and casting chains survive. Fewer rediscoveries, and the
     metal age is more robust.
   - Risk: low-medium. Some knowledge persists longer, so slightly more homogeneity.
6. **Report known alongside able (B4).**
   - Change: add a `known` count beside the `lore` "able" count in frames, and a
     separate "forgotten everywhere" event next to `LostWorldwide`. Update `DATA_SCHEMA`.
   - Expected effect: diagnostics only. Risk: schema change.
7. **Scarce-metal trade pull and geology (I1).**
   - Change: optional tuning only. Raise `want()` for inputs of recipes used in the last
     few years, so tin, copper and salt are carried further along trade routes.
   - Risk: low. Can blur regional contrasts, so keep it small.

In line with the brief, none of these add caps or scripted interventions.
Items 1-2 fix bugs; items 3-5 change the mechanics so S-curve diffusion and real
inertia can emerge.

## 7. 256x160 run (40606704688, 5 seeds, 1000 y)

The probe artifacts (`results-probe-1-1` … `5-5`) were aggregated directly. The seed-5
probe ran about twice as long as the others.

| metric | 256x160 |
|---|---|
| forgets no_input / displaced / unneeded / redundant / idle_slot % | 60.8 / 23.5 / 7.4 / 5.0 / 3.3 |
| taught arriving producible % | 42.1 |
| taught / inherited / invented forgotten % | 71.8 / 29.6 / 75.8 |
| forgotten unpractised % | 77.6 |

Item checks (known / make / kit):

| item | known | make | kit | other |
|---|---|---|---|---|
| hide thong | 0.90 | 0.09 | 0.09 | |
| bone point | 0.88 | 0.22 | 0.04 | |
| cooked meat | 0.94 | 0.14 | 0.14 | |
| copper knife | 0.81 | 0.46 | 0.12 | |
| copper-tipped spear | 0.62 | 0.38 | 0.12 | |
| plank boat | 0.81 | 0.51 | 0.51 | |
| sealed jar | 0.72 | 0.17 | 0.17 | |
| fired clay token | 0.99 | 0.77 | 0.77 | |
| bone beads | | | | peak 0.62 → 0.12, idle_slot 41% |
| copper ring | | | | idle_slot 50% |
| hide net | | | | worse_refined 35% |
| rawhide snare | | | | worse_refined 38% |
| warp-weighted loom | | | | 97% unneeded |

No iron or plank ships appeared in these seeds. Every finding above holds on all three
map sizes.

## 8. Diffusion step D1-D6: decisions (PROPOSAL, approved by Michael 2026-10-27)

Michael approved adjustments 1-6 of section 6 as the "diffusion" step, D1-D6. Item 7
(scarce-metal trade pull) is not part of it. There are no caps and no scripted outcomes.
Each sub-step gets unit tests, a short local smoke and its own commit. Progress is in
docs/PROGRESS-diffusion.md, section D.

- **D1 (B1): hide and bone are at hand for hunters.**
  - In stock upkeep, `Mat::Hide | Mat::Bone` are "present" when the band hunted big game
    this season (`hunted > 1`, the same test that already sets their origin).
  - Then the extra `needed && !present` wear (0.06 a season) no longer applies to them.
  - Budget: a season's gain is `0.01 x hunted` and the plain use is 0.02, so the stock holds
    at `hunted >= 2`. Bands that need hide hunt 3.1-4.5 (`probe_hide`).
  - Hunting effort does not depend on `needed`, so D1 adds no hunting pressure by itself.
    `game_left` is checked again with `probe_hide`.
- **D2 (B2): worn ornaments are practised.**
  - The ornament's chain is marked used (and so practised and refined).
  - It still adds no upkeep and no `needed` stock wear.
- **D3 (B3): kit choice by expected payoff.**
  - For each tool slot (hunt, cut, point, axe, boat, net, trap, bow, shield, outrigger,
    plank, ship), `make_kit` ranks candidates by
    `payoff = q x (0.5 + 0.5 x skill) x (1 + QUALITY_YIELD x quality) - KIT_UPKEEP x (1.15 - durability) x (1 + 0.3 x depth)`.
    - `skill`, `quality` and `depth` come from the recipe the toolkit makes the item by;
      raw items count as skill 1.
    - `KIT_UPKEEP = 0.06` is the economy's `upkeep` (0.02) per unit of the 0.35 yield a
      unit of tool quality is worth (the cut term in `game_m`).
  - For blades, `q` is the edge measure `weapon_q` already uses (0.55 cut + 0.45
    durability). Spear = 0.35 + 0.65 edge, point = edge, edge = edge; in the hunt slot,
    point = 0.35 edge and edge = 0.2 edge.
    - So knife and spear choice now agrees with weapon quality and upkeep. Fully practised,
      copper, bronze, iron and steel blades outrank flint and obsidian.
    - A newly learnt one does not, while the old one is honed.
    - Yields still use `cut_q`, so a copper knife cuts meat a little worse than flint. It
      lasts, and it fights better.
  - Other slots keep their quality measure (boat_q, mesh_q, tough + 1, the axe blend).
    Durability enters only through the upkeep term.
  - Trials: a slot's challenger is the candidate whose fully practised payoff beats the
    incumbent's current payoff.
    - Each season the slot is used, the band also uses the challenger with chance
      `KIT_TRIAL_P = 0.2` (a deterministic per-band, per-season, per-slot roll).
    - A trial adds the challenger's upkeep and marks it used, so it is practised and refined.
    - Once its payoff passes the incumbent's, it takes over. Better tools spread in
      S-curves, and old tools hang on where they are honed.
  - The kit is now rebuilt from the toolkit every time it is refreshed, because skill moves
    every season. The toolkit cache is unchanged.
- **D4 (I2): teaching what the learner can use.**
  - `exchange` picks the candidate recipe weighted by `1 + 3 x` the share of its inputs
    the learner already has, instead of uniformly. The listening odds are unchanged.
  - Decision on the "and/or": there is no separate slower fade for never-producible
    recipes. It would inflate "known" without use; D5 covers recently-seen inputs instead.
- **D5 (I3): idle chain tools and seen inputs fade slowly.**
  - `Known.last_ok` (new, tick) records the last season the recipe was practised or its
    inputs were all at hand (including traded stock).
  - While that is within `RECENT_SEASONS = 40` (10 years), an unused recipe loses skill at
    `RECENT_FADE = 0.5` of the usual rate.
  - Chain tools are covered: `mark_used` marks the whole chain whenever the end product is
    used, so a loom or mould keeps `last_ok` fresh while cloth or castings are made.
  - The save gains the `last_ok` column (bands blob layout 2), so the save format becomes
    15 at D5. Old saves load with `last_ok = 0` (long ago), which is neutral.
- **D6 (B4): reporting known vs able, "forgotten everywhere".**
  - Frames `lore` rows gain a fifth value: bands that know a recipe for the item, beside
    bands able to make it.
  - A new event kind `ForgottenEverywhere` fires when no living band knows any recipe for an
    item that had been known. It is distinct from `LostWorldwide` (nobody able to make it).
  - A new counter `forgotten_everywhere` is in the counter registry.
  - Still save format 15 (from D5). Old saves fill the new values with 0.
  - DATA_SCHEMA.md and SAVE_FORMAT.md are updated, and the tools are checked.

## 9. Before/after D1-D6 (CI checks on build 79d5fd4; report only, nothing tuned)

Runs (all headless-sims, build 79d5fd4):

- (a) seeds 1-10, 500 y, 256x160: [40620593419](https://github.com/Macrolol/kindling/actions/runs/40620593419), docs/results-diff-seeds1-10-500y.txt (vs step 13, 40588153931)
- (b) seeds 499246292-301, 750 y, 800x800: [40620602590](https://github.com/Macrolol/kindling/actions/runs/40620602590), docs/results-diff-750y-800x800.txt (vs step 13, 40588163981)
- Probes, compared with the pre-fix runs in docs/results-diff-probes.txt:
  - 128x80, 8 seeds, 1500 y: [40620611442](https://github.com/Macrolol/kindling/actions/runs/40620611442) (pre-fix 40606688049)
  - 192x120, 8 seeds, 1200 y: [40620619645](https://github.com/Macrolol/kindling/actions/runs/40620619645) (pre-fix 40606696457)
  - 256x160, 5 seeds, 1000 y: [40620627632](https://github.com/Macrolol/kindling/actions/runs/40620627632) (pre-fix 40606704688)

### 9.1 World checks, step 13 -> D1-D6

| metric | (a) step 13 | (a) D1-D6 | (b) step 13 | (b) D1-D6 |
|---|---|---|---|---|
| total pop | 1,096,051 | 1,250,829 | 7,736,599 | 8,097,746 |
| war %/y (mean) | 0.392 | 0.320 | 0.294 | 0.271 |
| disease %/y (mean) | 0.117 | 0.124 | 0.177 | 0.194 |
| continent crossings | 4/10, first Y114 | 5/10 (Y115, Y180, Y196, Y248, Y484) | 1/10, Y632 | 1/10, Y634 |
| crossings before Y100 | 0 | 0 | 0 | 0 |
| items now (mean) | 74.1 | 89.7 | 84.3 | 90.4 |
| steel | 2/10, med Y382 | **10/10**, med Y258 | 8/9, med Y373 | **9/9**, med Y240 |
| forged iron tools | 2/10, med Y339 | 10/10, med Y252 | 8/9, med Y353 | 9/9, med Y224 |
| bronze metal | 6/10, med Y283 | 10/10, med Y314 | 6/9, med Y345 | 8/9, med Y356 |
| pitch | 10/10, med Y196 | 10/10, med Y176 | 9/9, med Y265 | 9/9, med Y250 |
| sealed jar | 10/10, med Y214 | 10/10, med Y195 | 9/9, med Y341 | 9/9, med Y394 |
| ships | 0/10 | 0/10 | 1/9, Y665 | 2/9, med Y640 |
| ale | 1/10 | 2/10 | 2/9 | 5/9 |
| any sail / cloth sail | 5/10 / 0/10 | 8/10 / 3/10 | 7/9 / 7/9 | 9/9 / 9/9 |
| gold ring | 1/10 | 4/10 | 7/9, med Y436 | 6/9, med Y437 |
| steel_made (sum) | 0.88M | 10.7M | 21.1M | 85.4M |
| forgotten_everywhere (sum) | (no counter) | 18 | (no counter) | 45 |

(b) seed 499246298 dies by about Y10 in both builds, as expected; it counts as a world in the 9-seed "x/9"
milestone columns (the first ten seeds minus that one).

Guard rails: no crossing before Y100 in either; (a) 5/10 (limit 6/10), (b) 1/10 (limit under 7/10);
war is lower in both ((a) 0.392 -> 0.320, (b) 0.294 -> 0.271); no population collapse (only (a)
seed 10 is down, 55,294 -> 48,969, -11%). All pass.

### 9.2 Probe totals, pre-fix -> D1-D6

| metric | 128x80 | 192x120 | 256x160 |
|---|---|---|---|
| forget reasons no_input / displaced / unneeded / redundant / idle_slot % | 58.7/22.9/11.2/4.6/2.7 -> 67.6/20.4/5.7/5.7/0.6 | 58.6/24.6/9.3/4.3/3.1 -> 68.5/20.6/4.6/5.7/0.6 | 60.8/23.5/7.4/5.0/3.3 -> 70.3/19.5/3.5/6.1/0.6 |
| taught recipes producible on arrival % | 44.5 -> 37.9 | 44.0 -> 36.3 | 42.1 -> 34.2 |
| taught then forgotten % (of taught gains) | 73.6 -> 60.0 | 70.8 -> 60.2 | 71.8 -> 60.6 |
| forgotten never practised after learning % | 75.5 -> 81.5 | 77.9 -> 83.1 | 77.6 -> 83.5 |
| forgets within 20 y of being taught | 17.4M -> 6.8M | 30.2M -> 10.9M | 30.3M -> 10.9M |
| item extinction events (probe) | 277 -> 16 | 149 -> 10 | 57 -> 18 |
| ForgottenEverywhere (counter) | - -> 16 | - -> 10 | - -> 18 |

### 9.3 Known / makeable / in kit (share of bands, 2nd-half means), pre -> post

known/make/kit; "(n seeds)" = item appeared in only n seeds (8, 8, 5 probe seeds).

| item | 128x80 | 192x120 | 256x160 |
|---|---|---|---|
| hide thong | 0.91/0.12/0.12 -> 0.98/0.36/0.36 | 0.89/0.08/0.08 -> 0.98/0.36/0.36 | 0.90/0.09/0.09 -> 0.98/0.33/0.33 |
| cooked meat | 0.98/0.28/0.28 -> 0.98/0.15/0.15 | 0.94/0.14/0.14 -> 0.94/0.11/0.11 | 0.94/0.14/0.14 -> 0.95/0.10/0.10 |
| bone beads | 0.10/0.04/0.04 -> 0.97/0.45/0.35 | 0.10/0.03/0.03 -> 0.93/0.43/0.28 | 0.17/0.07/0.07 -> 0.92/0.40/0.27 |
| bone point | 0.91/0.21/0.06 -> 0.99/0.45/0.03 | 0.88/0.17/0.04 -> 0.98/0.44/0.03 | 0.88/0.22/0.04 -> 0.98/0.41/0.02 |
| flint blade | 0.85/0.44/0.38 -> 0.76/0.40/0.04 | 0.78/0.41/0.32 -> 0.67/0.36/0.02 | 0.85/0.44/0.27 -> 0.66/0.33/0.01 |
| stone blade | 1.00/1.00/0.39 -> 1.00/1.00/0.09 | 1.00/1.00/0.26 -> 1.00/1.00/0.06 | 1.00/1.00/0.20 -> 1.00/1.00/0.06 |
| copper knife | 0.76/0.38/0.14 (6 seeds) -> 0.79/0.40/0.06 | 0.76/0.42/0.13 -> 0.83/0.48/0.05 | 0.81/0.46/0.12 -> 0.87/0.51/0.04 |
| bronze knife | 0.63/0.29/0.29 (1 seeds) -> 0.76/0.37/0.07 (4 seeds) | 0.76/0.52/0.52 (3 seeds) -> 0.71/0.42/0.08 | 0.49/0.34/0.32 (4 seeds) -> 0.79/0.51/0.05 (4 seeds) |
| iron knife | 0.94/0.50/0.05 (1 seeds) -> 0.96/0.64/0.04 | 0.95/0.53/0.09 (1 seeds) -> 0.95/0.67/0.03 | 0.94/0.63/0.05 (1 seeds) -> 0.97/0.68/0.03 |
| steel knife | 0.96/0.60/0.60 (1 seeds) -> 0.99/0.79/0.73 | 0.97/0.54/0.54 (1 seeds) -> 0.99/0.83/0.77 | 0.97/0.66/0.66 (1 seeds) -> 1.00/0.87/0.82 |
| steel-tipped spear | 0.95/0.60/0.60 (1 seeds) -> 0.98/0.78/0.69 | 0.95/0.54/0.54 (1 seeds) -> 0.98/0.83/0.74 | 0.96/0.66/0.66 (1 seeds) -> 1.00/0.87/0.70 |

### 9.4 Knife slot: metal vs stone share of bands' cut-slot kit (tools/diffusion_knives.py)

| year | 128x80 metal/stone pre -> post | 192x120 pre -> post | 256x160 pre -> post |
|---|---|---|---|
| Y200 | 0.00/1.00 -> 0.07/0.93 | 0.09/0.91 -> 0.41/0.59 | 0.07/0.92 -> 0.24/0.76 |
| Y300 | 0.05/0.95 -> 0.62/0.38 | 0.32/0.68 -> 0.88/0.12 | 0.48/0.52 -> 0.97/0.03 |
| Y400 | 0.10/0.90 -> 0.79/0.21 | 0.33/0.67 -> 0.97/0.03 | 0.54/0.46 -> 0.97/0.03 |
| Y500 | 0.08/0.92 -> 0.95/0.05 | 0.39/0.61 -> 0.93/0.07 | 0.51/0.49 -> 0.95/0.05 |
| Y600 | 0.16/0.84 -> 0.89/0.11 | 0.43/0.57 -> 0.92/0.08 | 0.52/0.48 -> 0.92/0.08 |
| Y800 | 0.19/0.81 -> 0.85/0.15 | 0.39/0.61 -> 0.92/0.08 | 0.50/0.50 -> 0.92/0.08 |
| Y1000 | 0.22/0.78 -> 0.86/0.14 | 0.40/0.60 -> 0.93/0.07 | 0.52/0.48 -> 0.92/0.08 |
| Y1200 | 0.23/0.77 -> 0.86/0.14 | 0.42/0.58 -> 0.94/0.06 | - |
| Y1500 | 0.23/0.77 -> 0.85/0.15 | - | - |

Steel knife spread after D1-D6 (years from the first band knowing it): t50 20-40 y, t75 30-90 y
on 128x80; t50 30-40 y, t75 30-50 y on 192x120; t50 20-30 y, t75 40-50 y on 256x160. Before the fix steel
appeared in only one probe seed per map: 128x80 seed 5 at Y680 (t50 80 y, t75 200 y), 192x120 seed 5
at Y230 (t50 20 y, t75 30 y), 256x160 seed 5 at Y280 (t50 20 y, t75 30 y). So within one world the
speed is not new; what changed is that steel now appears in every world, about Y200-460.

### 9.5 Displacement by worse newcomers (worse_ref: share of a newcomer's kit entries that lowered the refined kit payoff)

| newcomer | 128x80 pre -> post | 192x120 pre -> post | 256x160 pre -> post |
|---|---|---|---|
| hide net | 0.58 -> 0.27 | 0.47 -> 0.30 | 0.35 -> 0.30 |
| rawhide snare | 0.57 -> 0.19 | 0.53 -> 0.25 | 0.38 -> 0.24 |
| plant fibre net | nan -> 0.55 | nan -> 0.59 | nan -> 0.59 |
| bone-tipped spear | 0.00 -> 0.17 | 0.00 -> 0.16 | 0.00 -> 0.14 |
| iron-tipped spear | 0.52 -> 0.62 | 0.18 -> 0.76 | 0.54 -> 0.72 |
| copper knife | 0.00 -> 0.37 | 0.00 -> 0.36 | 0.00 -> 0.29 |
| bronze knife | 0.05 -> 0.26 | 0.31 -> 0.28 | 0.33 -> 0.24 |
| iron knife | 0.45 -> 0.58 | 0.21 -> 0.61 | 0.54 -> 0.67 |
| steel knife | 0.22 -> 0.00 | 0.00 -> 0.00 | 0.00 -> 0.00 |
| stone blade | nan -> 1.00 | nan -> 1.00 | nan -> 1.00 |
| sharpened-stick spear | nan -> 1.00 | nan -> 1.00 | nan -> 1.00 |
| stone axe | 1.00 -> 1.00 | 1.00 -> 1.00 | 1.00 -> 1.00 |

Rows are the newcomer, as in section 3: the share of its adoptions where the refined kit payoff
went down. Hide net and rawhide snare adoptions are worse less often (0.47-0.58 -> 0.19-0.30 on
128x80/192x120, smaller drop on 256x160). Iron knives and iron-tipped spears are adopted while worse
**more** often (iron knife 0.21-0.54 -> 0.58-0.67, iron-tipped spear 0.18-0.54 -> 0.62-0.76), and
copper knives now 0.29-0.37 (was 0). Steel knives are never worse. The stone rows at 1.00 (stone
blade, sharpened-stick spear; stone axe already 1.00 before) are probably bands falling back to a
stone tool after losing the metal one. So D3 did not remove quality-blind adoption for the middle
metals. Its trial rule and the probe's refined-payoff measure may disagree, and that is worth
checking before tuning.

### 9.6 Findings and concerns (not tuned)

1. **Metal arrives too early and everywhere.** Steel is in every (a) world by Y188-306 (step 13:
   2/10) and every living (b) world; steel_made is 12x (a) and 4x (b). In the probes the cut slot at
   Y300 goes from metal 0.05/0.32/0.48 of bands (128/192/256) to 0.62/0.88/0.97, and steel knives
   reach half the bands 20-40 years after the first one. That per-world speed matches the single
   pre-fix steel seeds on 192x120 and 256x160, so the change is mostly that every world now gets
   there. Likely cause: the metal chain stops being lost. Pre-fix 128x80 probe extinctions included
   tin 40, copper metal 27, hide bellows 16 and iron bloom 6 (277 in all); post-fix there are 16 in
   all. D3 then puts practised metal blades above honed stone, trials make every band practise the
   challenger, and D5 keeps moulds, bloomery and crucibles from fading. Iron and copper knives are
   mostly passed through on the way to steel (kit 0.03-0.06, known 0.79-0.97).
2. **Hunting/cooking.** Cooked meat makeable fell (128x80 0.28 -> 0.15, 192x120 0.14 -> 0.11, 256x160 0.14 ->
   0.10), always for no_input (98% of its forgets): the bands know it but lack meat that season. Hunting
   kit is not missing (bow 0.70, flax snare 0.92 and steel-tipped spear 0.69-0.74 of bands in kit on 128x80/192x120), and
   population is up, so this looks like more people on the same game (seen in the D3 smoke) rather than D3
   starving hunting outright. Forager density in (a) fell 36.6 -> 31.7 and farm food share rose
   0.31 -> 0.43. Worth a targeted look at game_left before any tuning.
3. D1/D2 worked as meant: hide thong makeable 0.08-0.12 -> 0.33-0.36, bone beads known
   0.10-0.17 -> 0.92-0.97 and makeable 0.03-0.07 -> 0.40-0.45.
4. D4/D5 halve fast forgetting (within-20-y forgets -61 to -64%; probe extinctions -68 to -94%), but
   taught recipes are less often producible on arrival (-7 to -8 points) and more forgets are of
   never-practised recipes (+5 to +6 points). no_input is now two thirds of forgets.
5. Iron knives and spears are adopted while worse more often than before (9.5).
