# Kindling v7: iron through real prerequisites, kinship from descent, need-driven tinkering. Progress log

25 Oct 2026, about 10:00 to 13:30 EDT. Rules: one local sim at a time, RAYON_NUM_THREADS=4,
cargo -j4, every run under `timeout`. All multi-seed checks are 10 seeds x 500 years on GitHub
Actions (`headless-sims.yml`, 256x160 map), compared against `docs/results-v6.1-seeds1-10-500y.txt`.
Final numbers: `docs/results-v7-seeds1-10-500y.txt` (run 40207341019, commit 6719bdf).
Per-seed iron/bronze/lineage tables come from `tools/v7_metrics.py`.

## Commits (main)
- 768b7df: lineage diagnostics example (`civ-core/examples`) and `tools/v7_metrics.py`
- 2245cb9: B, kinship from real descent instead of founder lineage
- ea9b24e: A, iron needs bellows, a hot charcoal furnace and forging; save format 7, `v6_to_v7`
  migration, v6.1 fixture (`civ-core/tests/fixtures/v6_1_seed4_y125.civsave`)
- 612d805, 5e42058: A tuning (bloom yield 0.05, forging yield 0.10, a hot bloom invites the hammer)
- b7905e6, 6719bdf: C, attention from place and need; need-driven tinkering; a bellows furnace
  gets fed ores
- version 0.7.0

## A. Iron later, through real prerequisites (done)
- Before (v6.1): a stone kiln with charcoal could smelt iron ore directly to usable iron. Iron in
  8/10 seeds, median Y218, earliest Y118 (seed 4).
- Now the chain is physical, nothing is dated:
  - fire temperature = fuel temperature + 40 x refractory. Charcoal (950) in a clay furnace (+320)
    gives 1270; adding **bellows** (a new made item: hide + cord + wood) gives 1520, past the
    1400 needed to reduce iron ore. A stone kiln with bellows reaches only 1360, not enough.
  - smelting gives an **iron bloom** (spongy, slag-filled), not usable iron;
  - the bloom must be **forged**: reheated in a kiln with charcoal (>= 1100) and worked with a
    hard hammer stone (hardness >= 5) to give **wrought iron**, the input for iron tools.
  - Bands only notice an ore where it crops out (a deposit in their cell), and a bloom in hand
    draws the smith's attention to the hammer and the hottest fire.
- After (v7): usable iron in 10/10 seeds, median Y253 (range Y182 to Y461). Bellows Y110 to
  Y295, bloom Y176 to Y457. Every seed now goes bellows -> bloom -> wrought iron, in that order.
  Bronze is unchanged at 4 seeds (median Y339 vs Y356).

| seed | v6.1 iron | v7 bellows | v7 bloom | v7 wrought iron | v6.1 bronze | v7 bronze |
|---|---|---|---|---|---|---|
| 1 | Y218 | Y170 | Y252 | Y327 | never | never |
| 2 | never | Y152 | Y176 | Y182 | Y441 | never |
| 3 | never | Y110 | Y218 | Y226 | Y204 | Y225 |
| 4 | Y118 | Y131 | Y224 | Y253 | never | never |
| 5 | Y227 | Y180 | Y202 | Y214 | never | Y342 |
| 6 | Y215 | Y295 | Y457 | Y461 | never | Y339 |
| 7 | Y409 | Y209 | Y223 | Y235 | Y356 | never |
| 8 | Y182 | Y212 | Y246 | Y271 | Y267 | Y259 |
| 9 | Y216 | Y132 | Y233 | Y270 | never | never |
| 10 | Y328 | Y172 | Y208 | Y234 | never | never |

Note: the first A version (ea9b24e) made iron too rare (1 seed). The fix was not a date. It
was making the smith's work plausible: a bloom gets reworked in the hottest fire to hand, near
a hammer stone, and a forced furnace is used to cook stones (ores), not food.

## B. Lineage dominance (done; cause fixed, no cap)
- Diagnosis (per-seed lineage diagnostics, seed 7, the worst case): in v6.1 every band of the
  same *founder lineage* counted as kin, however long ago it split off. Kin got a 1.3x teaching
  rate and "close" trade terms. So the largest lineage taught and traded better within itself,
  grew faster, and pulled further ahead. With the kin bonus switched off in a diagnostic
  build (branch `diag/nokin`), seed 7's top lineage at Y500 fell from 39% to 25%.
- Fix: kinship now comes from real descent. Each band remembers its parent and three more
  ancestors. Kinship is high for recent splits and fades with time since the split. The
  teaching bonus scales with kinship, and "close" trade needs kinship >= 0.2. There is no cap
  on shares.
- Lineage max share (the highest share in any century, per seed):

| seed | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | mean | worst |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| v6.1 | 26 | 31 | 24 | 27 | 32 | 23 | 42 | 25 | 34 | 27 | 29.1% | 42% |
| v7 | 24 | 42 | 26 | 25 | 21 | 25 | 40 | 23 | 23 | 28 | 27.7% | 42% |

  Seeds over 30% went from 4 to 2. Seed 7 is still high (40%): it never farms and its
  geography is small, so one lineage still holds the best ground. Seed 2's 42% is a late
  surge (Y500) on the second continent, not a compounding kin effect.

## C. Divergence of discovery order between seeds (tau) (attempted; within noise)
- Metric: mean pairwise Kendall distance of discovery order across the 10 seeds (higher means
  more divergent). v6.1: 0.144.
- Changes kept (6719bdf), all local conditions, no random jitter:
  - ores draw attention only where they crop out;
  - carried goods draw less attention than what lies around;
  - **need-driven tinkering**: which parts get combined depends on way of life. Hunters weigh
    points, fishers cords and nets, cold places cords and points (clothing), and storers vessels;
  - a bellows furnace gets fed ores.
- Trials (each 10 seeds x 500y on Actions):

| variant | tau | pop |
|---|---|---|
| v6.1 | 0.144 | 928522 |
| B only | 0.149 | 843042 |
| A + B | 0.140 | 876876 |
| lower attention floor 0.5 (v1, v2, v3, C1, A3) | 0.120 to 0.132 | 892253 to 975491 |
| floor 1.0, no need weighting (f0) | 0.134 | 870363 |
| need weight 3 (f3) | 0.141 | 886826 |
| need weight 6 (f6) | 0.138 | 956897 |
| need weight 3 + furnace fed ores (final, 6719bdf) | 0.137 | 918614 |
| + need-aware adoption (c-try-adopt) | 0.115 | 923892 |

- Honest result: no variant raised tau beyond run-to-run noise (about +-0.01 with 10 seeds).
  Narrowing attention (a lower floor, stronger focus) or making adoption need-aware *lowered*
  tau, because broad, unfocused tinkering is itself a source of divergence. The order is
  dominated by prerequisite structure, and inventions diffuse very fast, taught across
  thousands of bands within decades. The structural lever is slower diffusion across barriers,
  which the D proposal's terrain cost and river travel (D2) provides. Re-measure tau there.

## Balance vs v6.1 (10 seeds x 500 years)

| metric | v6.1 | v7 | target |
|---|---|---|---|
| total pop | 928522 | 918614 (-1.1%) | within 15% |
| war deaths %/yr | 0.347 | 0.363 | 0.25 to 0.45 |
| disease %/yr | 0.122 | 0.116 | |
| farming seeds | 9 | 9 | about 9 |
| first domestic grain | Y252-Y476 | Y271-Y422 | |
| second continent | 4/10 (Y156, Y208, Y258, Y176) | 4/10 (Y131, Y240, Y270, Y169) | 4-6/10 |
| crossings before Y100 | 0 | 0 | 0 |
| iron seeds | 8 (median Y218) | 10 (median Y253) | later |
| bronze seeds | 4 (median Y356) | 4 (median Y339) | |
| tau | 0.144 | 0.137 | higher (not met) |

## Saves
- Format 7. The `v6_to_v7` migration re-points every learned "heat iron ore -> iron" recipe at
  the iron bloom (yield 0.05), so the band has to forge it now. Iron already made stays as it is. v1 to v6.1 saves load (fixtures for every
  format, including the new `v6_1_seed4_y125.civsave`). `cargo test` passes, with and without
  `--features parallel`.

## Scratch branches (left in place)
`c-try-v1`, `c-try-v2`, `c-try-v3`, `c-try-f0`, `c-try-f3`, `c-try-f6`, `c-try-adopt`,
`diag/nokin`. These are tuning trials, not merged.
