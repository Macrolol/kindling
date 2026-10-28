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

## 7. 256x160 run

See below.
