# Kindling v8: technologies for activities (the approved D proposal). Progress log

Started 25 Oct 2026, 11:50 EDT. Same rules as v7: one local sim at a time, RAYON_NUM_THREADS=4,
cargo -j4, every run under `timeout`. Each step is checked with 10 seeds x 500 years on GitHub
Actions (`headless-sims.yml`, 256x160 map) against `docs/results-v7-seeds1-10-500y.txt`.
Balance targets: pop within about 15% of 928,522; war 0.25-0.45%/yr; about 9 farming seeds;
crossings in 4-6 of 10 seeds and none before Y100.

Build order (from the design Page): 1 groundwork, 2 terrain and rivers, 3 fighting,
4 storage/salt/dairying/manuring, 5 seafaring, 6 trade carriers/ornaments/medicine, 7 riding.

## D1. Groundwork (done, commit 935aa5d, run 40208663447)
- Item set widened: 25 materials (new: brine, salt) and 25 forms (new: bow, shield, outrigger,
  sail, plank boat, carrying frame, ornament). `ItemSet` is now 16 words (up to 1024 items).
- Toolkit (`Kit`) gains slots for bow, shield, outrigger, sail, plank, frame and ornament. They
  are filled by `make_kit` but have no effects yet; later steps give them effects.
- Craft vectors: new materials get a neutral leaning of 1.0 (`band::fit_craft`), inserted before
  the three action leanings. World creation keeps the founders' random draws unchanged.
- Save format 8 with a frozen `v7_to_v8` migration (pads stock and origin to 25 materials, inserts
  neutral leanings). Format 5, 6 and 7 binaries still decode. New fixture
  `civ-core/tests/fixtures/v7_0_seed4_y150_96x60.civsave` (written by the v0.7.0 binary) and test
  `v7_0_fixture_gains_new_materials_in_place`.
- Results: `docs/results-v8-D1-seeds1-10-500y.txt`.

| metric | v7 | D1 |
|---|---|---|
| pop (10 seeds, Y500) | 918,614 | 876,123 |
| war %/yr | 0.363 | 0.363 |
| disease %/yr | 0.116 | 0.116 |
| farming seeds | 9 | 7 |
| crossings | 4/10 | 4/10 (Y125, Y133, Y226, Y261) |
| iron seeds (median) | 10 (Y253) | 6 (Y299) |
| bronze seeds (median) | 4 (Y339) | 6 (Y307) |
| tau | 0.137 | 0.139 |
| lineage max share, mean / worst | 27.7% / 42% | 29.1% / 45% |

The step adds no behaviour, but the extra neutral entries shift the craft-leaning normalisation
slightly, and the sim is chaotic, so individual seeds diverge (iron and farming are the seed-level
outcomes most sensitive to this). The war and disease rates are unchanged and all targets but
farming seeds are met. I'm tracking farming seeds over the next steps rather than tuning for it.

## D2. Terrain cost and river travel (done, commit 635f603, run 40209240376)
- `civ-core/src/world/terrain.rs`: the cost of a step between neighbouring cells is
  `1 + max(|climb| - 150 m, 0) / 400 m + max(height - 1500 m, 0) / 1000 m`, summed along the line
  between two cells. Ranges and high plateaus become barriers and passes matter.
- River systems (8-connected river cells) are labelled on load. A band with a boat on a river
  travels along the same river for a third of the land cost (at least 1), and can scout 2-6 cells
  along it.
- The cost shows up in moves (a mild effort penalty on candidate cells), meetings and teaching
  (`teach_rate x kin / (1 + cost)`), and partner visits (effective distance compared to reach;
  rivers and sea routes shorten it for bands with boats).
- Not built from the river section: rafts for heavy loads, fish weirs.
- Test `terrain_cost_and_rivers`.

| metric | v7 | D1 | D2 |
|---|---|---|---|
| pop | 918,614 | 876,123 | 840,996 (-9.4% vs target) |
| war %/yr | 0.363 | 0.363 | 0.353 |
| disease %/yr | 0.116 | 0.116 | 0.109 |
| farming seeds | 9 | 7 | 9 |
| crossings | 4/10 | 4/10 | 4/10 (Y132, Y138, Y205, Y371) |
| iron seeds (median) | 10 (Y253) | 6 (Y299) | 8 (Y230) |
| bronze seeds (median) | 4 (Y339) | 6 (Y307) | 6 (Y360) |
| tau | 0.137 | 0.139 | 0.138 |
| lineage max share, mean / worst | 27.7% / 42% | 29.1% / 45% | 28.5% / 42% |

Farming seeds are back to 9, which supports reading D1's dip as seed-level noise. The tau of
discovery order is unchanged: terrain slows contact but every world still finds things in a
similar order.

## D3. Fighting (done, commits 0f9510a and 1e51930, run 40210394552)
- Bows: a strong wooden shaft shaped with a cord (sinew, hide thong, flax or wool string). A bow
  counts only with points to shoot. It helps hunting (game x1.2, small game x1.15), cuts a band's
  losses in a fight (x0.7) and lowers the chance its people are taken captive (0.35 to 0.15).
- Shields: hide lashed to a wooden frame with a cord, made away from water (by the water the same
  parts make a skin boat). They cut losses x0.75.
- Palisades: a settled band with an axe and a digging tool and grudges against its neighbours
  builds a wall where it lives (+0.15 of the gap a year, decaying when there's no threat, with
  upkeep from stores). The wall adds to the defender's strength (up to +28%) and cuts its losses.
  It is lost if the band moves.
- Weapon quality now counts how long a head lasts, not just how sharp it is
  (`0.55 cut + 0.45 durability`), so metal beats flint in a fight even though flint cuts better.
  Forged iron and cast bronze come out about equal on this measure.
- Not built: palisades using up local wood, and a walls count in the summary.
- Test `bows_shields_and_metal_weapons`.

| metric | D2 | D3 |
|---|---|---|
| pop | 840,996 | 848,732 (-8.6% vs target) |
| war %/yr | 0.353 | 0.328 |
| disease %/yr | 0.109 | 0.108 |
| farming seeds | 9 | 8 |
| crossings | 4/10 | 4/10 (Y124, Y171, Y243, Y347) |
| iron seeds (median) | 8 (Y230) | 5 (Y247) |
| bronze seeds (median) | 6 (Y360) | 6 (Y328) |
| tau | 0.138 | 0.133 |
| lineage max share, mean / worst | 28.5% / 42% | 30.0% / 43% |

War deaths fell slightly (0.353 to 0.328%/yr) and stay inside the 0.25-0.45 target. Bows and
shields make fights less deadly per fight, while the number of fights barely moved (4596 to
4525 a century). All 10 seeds invent a bow (the first between Y84 and Y250) and a shield.
Iron is now usable in 5 seeds instead of 8, and 4 seeds reach bronze with no usable iron. That
moves the balance toward bronze, but nothing in D3 targets it directly: tin and bloom dates are
much the same, so this looks like seed-level variation in who gets to bloomery iron first rather
than a real effect.

## D4. Storage, salt, dairying and manuring (done, commits 917d79a and a031258, run 40210898547)
- Salt: brine is at hand on sea shores and in hot, dry lowland salt pans. Heating brine in a fired
  pot over a fire leaves salt (the recipe is found, not given). Salt is kept, traded (it is now
  one of the trade goods), and wanted by bands that store meat and fish in pots or over smoke.
  Salted meat and fish spoil 30% slower.
- Storage pits and granaries: a settled band with a digging tool on dry enough ground (rain
  < 0.6) loses grain 40% slower and can store 30% more of its crop.
- Dairying: a herd tame enough to milk (tame > 0.8), with pots to hold the milk, gives 1.4 times
  the food. Cheese (the same herd and fired pots) adds up to 0.3 x pop to what a band can put
  by for winter.
- Manuring: a settled band that keeps its herd on its own fields spreads dung on purpose, and the
  soil recovers twice as fast.
- Test `salt_from_brine`.

| metric | D3 | D4 |
|---|---|---|
| pop | 848,732 | 995,992 (+7.3% vs target) |
| war %/yr | 0.328 | 0.323 |
| disease %/yr | 0.108 | 0.131 |
| farming seeds | 8 | 8 |
| crossings | 4/10 | 4/10 (Y134, Y190, Y257, Y280) |
| iron seeds (median) | 5 (Y247) | 7 (Y251) |
| bronze seeds (median) | 6 (Y328) | 5 (Y255) |
| tau | 0.133 | 0.146 |
| lineage max share, mean / worst | 30.0% / 43% | 26.8% / 32% |

Salt is invented in 9 of 10 seeds (first between Y113 and Y314). Population rose 17% as
expected, and with it disease (0.108 to 0.131%/yr). It stays inside the target band. Farming holds
more people: in seeds 1 and 3 foragers fell from 95% and 82% of people to 21% and 27%.

## D5. Seafaring (done, commit 6afafca, run 40212234218)
- Outrigger: a boat, a wooden shaft and a cord put together at the water. A float on outriggers
  is safer at sea (+0.15 to boat safety).
- Sail: a boat, a mast (shaft) and woven cloth (flax or wool garment) at the water. With a sail,
  people who already know the sea (seacraft above 0.4) can go 1.6 times as far.
- Plank boat: wood shaped with a metal axe at the water. A bigger, safer boat (+0.25 quality)
  that carries a larger daughter party (45-60% of the band instead of 35-50%) and counts as
  being prepared for a crossing.
- Wayfinding: people with seacraft above 0.6 steer half as far off their aim.
- Boats, outriggers, sails and planks need no upkeep away from the water.
- Test `outriggers_sails_and_plank_boats`.

The run used the D3 baseline; the table compares it with D4.

| metric | D4 | D5 |
|---|---|---|
| pop | 995,992 | 919,336 (-1.0% vs target) |
| war %/yr | 0.323 | 0.331 |
| disease %/yr | 0.131 | 0.108 |
| farming seeds | 8 | 9 |
| crossings | 4/10 (Y134, Y190, Y257, Y280) | 4/10 (Y134, Y186, Y195, Y276), none before Y100 |
| lost at sea | - | 34,105 (D3 38,309) |
| iron seeds (median) | 7 (Y251) | 6 (Y264) |
| bronze seeds (median) | 5 (Y255) | 4 (Y313) |
| tau | 0.146 | 0.144 |
| lineage max share, mean / worst | 26.8% / 32% | 28.2% / 34% |

Outriggers are invented in every seed (the first between Y132 and Y361), and plank boats in most
(between Y197 and Y398, with copper, bronze or iron axes). No seed invented a sail in 500 years.
Woven flax or wool cloth is rare on the coast this early, and nothing was added to make sails
appear. Crossings stay at 4 of 10 seeds and come slightly earlier, and fewer people are lost at
sea. The safer boats mainly cut drownings rather than open new crossings.

## D6. Trade carriers, ornaments and medicine (done, commit 4a01c50, run 40212585875)
- Carrying frame: a shaft, a cord and a soft basket or bag (not a sealed pot). Traders with a
  frame carry bigger lots (up to 0.3 x (1 + the bag's quality)), and a band with one reaches one
  more partner.
- Ornaments: a bone, copper, tin, bronze or obsidian point worked with a hammer gives beads, a ring
  or a pendant. Bands want ornaments (more strongly when they are made of something rare) and
  trade for them. Nothing else is tied to them yet: they bring no prestige and no tribute.
- Herbal lore: people under sickness in plant-rich land, who have fire, sometimes find what
  helps. Once known, the lore grows with use, is taught on meeting, passes to daughters, and
  fades when it is not needed. It cuts endemic sickness by up to 15% and ordinary deaths by up
  to 5%.
- Boiled water: settled bands with fire and fired pots have 15% less endemic sickness and 15%
  fewer outbreak deaths.
- Wariness: a band that has just had an outbreak keeps away from the sick for a few seasons
  (catching it from neighbours is up to 40% less likely), and the effect fades.
- Test `frames_ornaments_and_medicine`.

| metric | D5 | D6 |
|---|---|---|
| pop | 919,336 | 931,174 (+0.3% vs target) |
| war %/yr | 0.331 | 0.348 |
| disease %/yr | 0.108 | 0.113 |
| farming seeds | 9 | 8 |
| crossings | 4/10 | 4/10 (Y143, Y188, Y195, Y302), none before Y100 |
| iron seeds (median) | 6 (Y264) | 5 (Y290) |
| bronze seeds (median) | 4 (Y313) | 3 (Y369) |
| tau | 0.144 | 0.134 |
| lineage max share, mean / worst | 28.2% / 34% | 27.0% / 35% |

Carrying frames are invented early and widely (the first between Y74 and Y153), and bone beads
appear in most seeds (between Y56 and Y125). Copper, tin and bronze rings follow metal (between
Y221 and Y382). Disease stays near the D5 level: medicine roughly offsets the extra people.
Trading tin did not shift the metals toward bronze; if anything bronze came later. That is left
as it is.

## D7. Riding (done, commit 323f194, run 40212892437; saves from run 40213696876)
- A band that has tamed a wild herd (taming above 0.8, herd above 0.2), has a cord, and lives on
  open steppe sometimes starts riding (chance 0.002 x curiosity per year). Riding starts at 0.1
  and grows while the band keeps a herd on open grazing land (grazing potential 0.8 or more);
  elsewhere it fades, and it is lost with the herd. It is taught on meeting, passes to daughters
  and travels with herd animals.
- Riding counts only with a herd on open land. A mounted band scouts twice as far (6 cells
  instead of 3), can meet bands twice as far away across open land, and attacks with up to 1.3x
  strength. Defence is unchanged.
- New major event: the first band in a world to ride (`FirstRiding`).
- Test `riding_counts_only_with_a_herd_on_open_land`.
- Results: `docs/results-v8-D7-seeds1-10-500y.txt`.

| metric | D6 | D7 |
|---|---|---|
| pop | 931,174 | 891,377 (-4.0% vs target) |
| war %/yr | 0.348 | 0.356 |
| disease %/yr | 0.113 | 0.111 |
| farming seeds | 8 | 9 |
| crossings | 4/10 | 4/10 (Y143, Y188, Y195, Y302), none before Y100 |
| iron seeds (median) | 5 (Y290) | 4 (Y290) |
| bronze seeds (median) | 3 (Y369) | 3 (Y334) |
| tau | 0.134 | 0.134 |
| lineage max share, mean / worst | 27.0% / 35% | 27.4% / 34% |

Riding appeared in 8 of 10 seeds, first between Y232 and Y499 (seeds 1-10: Y335, Y325, Y292,
none, Y232, Y444, Y365, none, Y248, Y499). Where it starts early it spreads widely: in seed 5
(first rider Y232) 95% of people belong to riding bands by Y500, in seeds 2 and 3 about half do.
Over all ten worlds 49.5% of people ride at Y500. Seeds 4 and 8 have no wild herds on steppe
near herders, so nobody rides. War rises only slightly (0.348 to 0.356%/yr) because riding
strengthens attacks but does not make bands more willing to fight.

## Final balance (v0.8.0, sim code 323f194, run 40212892437)

| metric | v6.1 | v7 (v0.7.0) | v8 (v0.8.0) | target |
|---|---|---|---|---|
| pop (10 seeds, Y500) | 928,522 | 918,614 | 891,377 (-4.0%) | within ~15% of 928,522 |
| war %/yr | 0.347 | 0.363 | 0.356 | 0.25-0.45 |
| disease %/yr | - | 0.116 | 0.111 | - |
| farming seeds | 9 | 9 | 9 | ~9 |
| crossings | 4/10 | 4/10 | 4/10, none before Y100 | 4-6, none before Y100 |
| iron seeds (median) | 8 (Y218) | 10 (Y253) | 4 (Y290) | not targeted |
| bronze seeds (median) | - | 4 (Y339) | 3 (Y334) | not targeted |
| tau | - | 0.137 | 0.134 | - |
| lineage max share, mean / worst | - | 27.7% / 42% | 27.4% / 34% | - |

Per step: war after D3 (bows, shields) fell to 0.328%/yr and came back to 0.356 by D7. Crossings
stayed at 4 of 10 after D5 (seafaring); boats came a little earlier and drowned fewer people,
but they did not open new crossings. Tau was 0.138 after D2 and is 0.134 at the end, so the
order of discovery is as consistent as in v7. Iron vs bronze: bronze came closest to iron at D4
(median Y255 vs Y251), then drifted back. At the end iron is still first, in fewer seeds than in
v7 (4 vs 10), and the gap to bronze is smaller (Y290 vs Y334 against v7's Y253 vs Y339). The
metal seed counts move a lot between steps, which looks like seed-level noise. Nothing was
tuned to push either metal.

Not built: palisade wood depletion, a walls statistic, rafts and fish weirs, ornament prestige
or tribute, wary bands meeting less often. Sails exist as a form but none emerged in 500 years.
