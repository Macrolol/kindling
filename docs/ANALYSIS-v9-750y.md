# v9 at 750 years on an 800×800 map: analysis

Data: `docs/results-v9-750y-800x800.txt`. GitHub Actions run 40302724447 was pinned to main `4d28945` (v9, after the motivation work). It ran 12 random seeds, 295068863–295068874, for 750 years each on an 800×800 map, one seed per job. All jobs were green. The run was dispatched at 05:50 EDT on 2026-10-26 and took about 26 minutes of wall time.

I compare it against two earlier runs:
- **step 8**: `results-v9-step8-seeds1-10-500y.txt`, 10 seeds on the default map, 500 years, same code;
- **v8 long run**: `results-v8-longrun-1200-1500y.txt`, 8 seeds, 1200×1200, 1500 years, pre-v9 code.

No sim code was changed for this analysis.

## Headline

| | 750y, 800×800, 12 seeds | step 8, 500y | v8 long run at Y701–800 |
|---|---|---|---|
| Pop per seed (mean, range) | 887k (292k–1.25M) | 119k | 1.0M–2.7M (bigger map) |
| War %/yr | 0.34 (0.21–0.40) | 0.36 | 0.36–0.51 |
| Disease %/yr (whole run; last 50 yrs) | 0.21; **0.32** | 0.12 | 0.13–0.32 |
| Foragers, share of people | 36% (17–62) | 51% | — |
| Items | 83 (67–87); all-seed union 87 | 69 | 82–89 at Y1500 |
| Tau (order similarity) | 0.139 | 0.15 | — |
| Wrought iron / bronze / sail of any kind | 11 / 10 / 10 of 12 | 4 / 4 / few | 8 / 8 / 8 of 8 |
| Median year: wrought iron / bronze / sail | Y267 / Y441 / ~Y630 | — | Y274 / Y470 / Y568–898 |
| Effective lineages, top lineage | 4.84, 30% | 5.15, 29% | top 46.6% at Y1500 |
| Prestige mean / Gini / top-10% share | 0.47 / 0.39 / 27% | 0.48 / 0.34 / 23% | — |
| Taught by more prestigious | 59% (every seed) | 59% | — |
| Recipe quality / boat seaworthiness | 0.33 / 0.98 | 0.35 / 0.99 | — |
| Seeds that settled a second continent by boat | 3 of 12 | 4 of 10 | 1 of 8 (long crossing) |

## Findings, ranked by how solid they are

### Solid: mechanism plus consistent data

**1. What blocks crossings is sight, not boat range.**
- I checked all ~380 sea landfalls (islands and continents). Every one was a single hop of 18 sea cells or less. The counts drop off roughly geometrically: 143 one-cell hops, 58 two-cell, 41 three-cell, and so on down to 2 at 17 cells and 1 at 18.
- Every second-continent settlement came at Y238–523, before anyone had a sail:
  - s872, gaps of 5 and 3;
  - s870, gap 12;
  - s866, which island-hopped to a continent 57 cells from home with an 11-cell last hop.
- After sails arrived (Y578–740), there were only 23 more landfalls. The longest of those was 10 cells.
- s871 is the clearest example. It had a linen sail at Y578, the 90th percentile of its boat range was 29.6 cells, and a continent sat 29 cells away. It never reached it.
- The code explains why:
  - A deliberate voyage needs a `far_shore` target, which comes from a sighting or a rumour.
  - The sighting horizon is 3 + up to 3 for height, plus at most about 7 when fishing, so at most about 13 sea cells.
  - Scouting voyages (random heading) are the only way to find land farther out: 10,278 finds in 202k scouting voyages.
  - Boat range (`2 + 10q + 12·xp·q`, ×1.6 with a sail) is already 20–38 cells, which is past the horizon. So more range adds nothing.
- `boat_q` is saturated: 55–92% of boat-owning bands sit at the 1.0 cap. Better boats can't help either.
- This supports Michael's view that the fix is new technology, not a bigger range number. The useful levers are what a band *knows* is out there and how it navigates. See "Implications".

**2. The invention stall comes from the tech tree running out. It starts around Y550–650, well before Y1000.**
- First-ever inventions per century, summed over all seeds: 260, 211, 174, 136, 99, 86, 34, then 10 in the last 50 years.
- Rich worlds top out at 85–87 items, and the union across all 12 seeds is 87 names, so they have found almost the whole tree. s864's last new item came at Y533; s867's at Y570.
- The late finds belong to worlds that split across the sea (s866, s872). Their colonies re-run parts of the tree.
- The mean number of items per band levels off at 33–37 from about Y500. Learning and forgetting balance out; for example, s863 at Y750 learned 28,788 items and forgot 28,889.
- The v8 long run hit the same ceiling (82–89) later, around Y1000, because a bigger map develops more slowly.
- The ceiling is the content limit, not a dynamic one. Adding inventions is the right next step.

**3. Disease grows with farming and does not level off.**
- Disease deaths by century: 0.01 → 0.03 → 0.06 → 0.08 → 0.13 → 0.21 → 0.28 → 0.32 %/yr.
- Farmer share by century: 0 → 64%.
- Across seeds, the correlation between farmer share and disease is +0.82.
- Over the whole run disease is double step 8's 500-year figure. That is because step 8 stopped just as disease was starting to climb.
- In the last 50 years, disease plus war is about 0.7%/yr. In the cramped farming worlds it is about 0.8%/yr.
- This matches the v8 long run at the same ages (0.13–0.32). Nothing new is broken, but disease is now the main late killer in small farming worlds.

**4. Geography explains most of the differences between seeds.** With n = 12, read these correlations as moderate evidence.
- Population vs reachable land (home plus any continent reached): r = +0.77. Against home landmass alone: +0.58.
- The small, cramped worlds (s863, s865, s868) end at 290–580k people. They have 80%+ farmers, the highest disease, and stop growing by about Y600–650 (s863: +0.7% over its last 50 years).
- Larger worlds were still growing 11–23% per 50 years at Y750. They are not saturated.
- Reachable land vs forager share: +0.81. More room means people can keep foraging.
- Bigger worlds also have:
  - more lineage dominance: top lineage +0.65, effective lineages −0.74;
  - more prestige inequality: Gini +0.70.
- Ore deposits only weakly predict when metals appear (iron r = −0.38, tin −0.27).

**5. Lineage drift happens mostly through founder effects at crossings.**
- The home continent stays mixed, with about 4–6 effective lineages.
- Continents settled by a few early crossings are each dominated by one lineage:
  - s872: 82%, 65% and 59%;
  - s866: 70%.
- s870 is the exception, and it makes sense. Its 12-cell channel was crossed over and over (60k landings), so its second continent stayed mixed (top lineage 29%).
- At the world level drift is slow and partly reverses. s872 peaked at 53% around Y400–500 and is down to 39%; s873 went 22 → 46 → 40%. The mean is 30%, against 46.6% for v8 at Y1500.
- This is the most useful structural fact for the culture and language phases. Isolated colonies already form genetically distinct populations, so culture could plausibly ride on lineage and isolation.

### Fairly solid: consistent, but not deeply probed

**6. Prestige** works as designed: 59% of teaching runs from higher to lower prestige, in every seed.
- Prestige barely tracks band size (correlation 0.08–0.28 per seed).
- Inequality grows with world size and run length: Gini 0.39 vs 0.34 at 500 years.
- Prestige has no hard cap (maximum 5.4–10.1). That is fine for now. A politics phase will probably want prestige to build up into something lasting, such as leadership or chiefdoms, instead of floating.

**7. Marriage partner lists are saturated.**
- Mean partners per band is 7.64–7.76 in every seed, against `MAX_PARTNERS = 8` (band.rs:368).
- The married share is 0.29–0.34.
- So partnerships no longer separate well-connected bands from others. They are at the cap everywhere. Any diplomacy or alliance mechanic built on partners will need a higher cap or a strength-weighted measure. (This is not a bug.)

**8. Dissent and wanderlust**
- Splits caused by dissent are 9–45% of all splits. The share is highest in the crossing worlds (s866 45%, s872 43%) and lowest in the stagnant s865 (9%).
- Mean dissent is 0.14–0.27.
- Bands overseas are more ambitious where crossings happened: s866 +0.39 vs +0.21 overall, s868 +0.18 vs −0.03. This is the selection effect we intended.
- Curiosity averages 0.80–1.19. In every seed at least one band reaches the curiosity cap (2.5) and the ambition cap (1.0), but the 90th percentiles sit far below both, so the caps aren't distorting anything.

**9. Refinement is saturated.** Each seed made 10.7M–70.8M refinements, yet recipe quality stays at about 0.33 and boat_q is pinned at 1.0. More refinement has no visible effect late in the game.

### Noise or one-offs (don't build on these)

- **s865 is the "stagnant" world.** It has the fewest items (67), the lowest war (0.21) and the lowest prestige (0.30). It got sails (woollen at Y600, linen at Y629) without ever having a plank boat. It found copper at Y445 and iron bloom at Y441 and lost both. Its headline reads "stage now: pottery, ever: copper". s863 is the same size but made 78 items with iron at Y210, so this is chance, not geography.
- **s867** had bronze (Y221) about 180 years before wrought iron (Y403). The other seeds mostly had iron first, as in v8.
- **Metal dates** vary by 300+ years between seeds. Tin (metal) was lost worldwide and found again several times (s870 lost it three times).
- **Wool items** show low or negative spread correlation (`r`) in the firsts lists, e.g. "bow (wool string)" −0.11 and "tin (metal)" in s865 −0.20. I suspect wool items spread along herding regions instead of by distance. Not investigated.

## Bugs and oddities (reported, not fixed)

1. **Losses missing from the "lost worldwide" line.** Items that never reached 5 bands are logged as `LostLocal`, so they don't appear in that line. s865 lost copper (2 bands ever), which shows only as "stage now: pottery, ever: copper" with no matching entry. This is by design (world.rs ~3116), but it is confusing when reading results. A "lost (minor)" count would help.
2. **Repeated names in firsts lists.** "outrigger canoe" can appear twice because two recipes produce items with the same name. Tools that dedupe by name, including my tau calculation, merge them. Harmless, but worth knowing.
3. **s870's headline** labels the 34k-cell continent as home. The bigger 192k continent it settled at Y356 holds 68% of its people. The label is correct, but readers expect "home" to be the main landmass.
4. **Misleading `gap_home` for islands.** Values like 739 are measured from the home landmass, not from where the voyage started. Real hops are always ≤ 18 cells (see the per-hop analysis). This should be renamed or supplemented with a gap measured from the nearest settled land, which continents already have ("N from settled").
5. **Tau is "-" in aggregate.md** whenever each job runs one seed, because it is computed per process. The workflow's aggregate step could compute it across jobs.
6. **Caps that bind:**
   - `MAX_PARTNERS` 8: 97% full.
   - `boat_q` 1.0: 55–92% of boat bands.
   - curiosity 2.5 and ambition 1.0: reached by the tail only.
7. **No stuck counters.** Every counter is non-zero in every seed, and all large counters are u64. `pairs_seen` reaches 2.8e9 per seed, which would have overflowed a u32 if it were one.

## Performance

- Sim time per job ranged from 278 to 1489 s (sum 9,193 s) and scales with final population (r = +0.71).
- The heaviest seed (s869, 1.17M people on a 174k-cell home) took 25 minutes on a CI runner. All 12 seeds in parallel took 26 minutes of wall time.
- The 1200×1200, 1500-year size is therefore about 4–6× this cost per seed. 800×800 at 750 years is a good routine test size.
- Saves are 43–147 MB.

## Comparison summary

- **Against step 8 (500 years):** the extra 250 years and 15× the area add:
  - near-universal metallurgy (iron 11/12 vs 4/10) and sails (10/12);
  - double the disease;
  - a 20–30% forager drop;
  - slightly more inequality.

  Prestige-weighted teaching (59%), tau (0.14–0.15) and the boat-crossing rate (about 1 in 3–4 seeds) are unchanged.
- **Against the v8 long run:**
  - The same item ceiling, reached sooner.
  - Similar war and disease at the same ages.
  - Iron and bronze dates within noise.
  - Lineage dominance lower so far (30% vs 47% at Y1500). That may just be the shorter run.
  - The v8 finding that sails never carried anyone across gaps of 27–48 cells reappears here, now with the explanation from finding 1.

## Implications for the next phases

- **Later technology for long crossings (instead of more boat range):**
  - Navigation lore (stars, birds, swells, currents): extends the effective sighting horizon or lets a band aim at land it can't see.
  - Charts or carried knowledge: shores found by scouts become rumours that last longer and travel farther than RUMOUR_YEARS = 50.
  - Provisioning (salted or dried food, water jars): lets voyages survive longer at sea.

  The cheapest lever in the current code is the sighting horizon and the rumour reach. Range and seaworthiness are already past what bands can use.
- **New inventions:** the tree is used up by about Y600 on 800×800. New content should sit mostly after iron, sail and plank boat, so worlds still have things to find in Y600–1000. Wool items might also need a look at their spread.
- **Culture and language:** founder-effect continents (one lineage at 60–80%) and isolated home regions are natural seeds for distinct cultures and languages. Tracking cultural or linguistic distance alongside lineage would show it right away.
- **Politics:** the ingredients are there (prestige inequality growing with scale, dissent splits up to 45%, in-law fights 20k), but they have nothing to build up into. Partner lists are full and prestige floats. A politics layer needs a lasting structure, such as alliances or leadership, with its own capacity rather than the partner cap of 8.
- **Disease** will dominate late farming worlds. Expect politics and settlement density to interact with it.
