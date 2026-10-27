# Ideas for later updates

Status: notes only, not scheduled. No sim code is changed by this document. Each entry records
what the sim does now, the options considered, and rough effect, risk and effort, so it can be
picked up later without redoing the research.

---

## Fishing as a reason to go to sea (looked at Oct 2026, base 70260e9)

Michael's question: do fish ever run short from overfishing a specific area, and could fishing
motivate sea travel?

### How fishing works now

- **Fish are a per-cell stock** (`World::fish`), with a natural level `k` from the map:
  - **Land coastal cells** hold about 25. **River cells** hold 15 + 10·ln(flow/threshold), up to
    60. Both are halved below −4 °C (`map.rs` ~816–826). These are fished from the bank, with no
    boat.
  - **Sea shelf cells** (`add_shelf_fish`, `map.rs` ~405–440) hold k = 40 · shelf · cold · ice.
    `shelf` falls from 1 at one cell offshore to 0 at six cells. `cold` is 1.25 below 12 °C, and
    `ice` is 0.3 below −4 °C. **Beyond six cells the sea is empty.**
- There are no seasons and no species. Every fish stock is the same generic fish all year round.
- **Overfishing does happen, locally.** A harvest takes from the real cells
  (`take_harvest`: v −= φ·weight·v). Regrowth (`regrow_cell`, `world.rs` ~3459) is logistic plus
  a floor: f += 0.3 f(1 − f/k) + 0.02 k per season (game uses 0.12 and 0.01).
  - An emptied cell is back to 90 % in about 15 seasons (about 4 years), from 10 % in 11
    seasons, and from 30 % in 8.
  - The most it can yield indefinitely is about 9.5 % of k per season.
  - Because of the floor, **a stock never goes extinct**.
- **Reach.** A band with a boat adds the sea cells within r = 1 + round(2·boat_q) to its home
  range (`home_range`). A cell's weight is 0.12 at distance ≤ 1 and 0.25/d beyond, with the total
  capped at 0.6 + 1.2·boat_q. Effort goes to fish by weight 2.5·fish_m, where
  fish_m = 1 + 0.8 hunt + 0.4 vq + 0.3 point + 0.2 cord + 0.9 net.
- **Fishing and the sea.**
  - In `sea_season`, a fish share above 10 % builds seacraft, widens how far a band can see, and
    lets fishing parties drift.
  - But **sea trips themselves are driven only by push** (`push_out`: stress, crowding, pressed)
    and by the lure of a known far shore. They are never driven by fish.
  - `sea_survival_x` already has a "prepared" factor (0.7). Salt makes fish spoil 30 % slower.

### Measured (fish stock as % of its natural level, by distance from camps)

| Save | Sea, 1 cell | Sea, 2–3 cells | Sea, 4–6 cells | Bank fish at camp cells |
|---|---|---|---|---|
| Seed 3, Y300 | 65 % | 74 % | 88 % | 42 % |
| s865, Y750 | 28 % | 54 % | 98 % | 9 % |
| s870, Y750 | 41 % | 59 % | 94 % | 28 % |

- Game near camps sits at 5–25 %.
- Fish are 5–8 % of all food eaten, and 18–25 % for coastal bands with boats. About one band in
  five gets more than 10 % of its food from fish.
- Total fish capacity is only about 10 % of plants plus game.

So depletion is real and local: inshore and bank fish are worn down near camps. But fish are a
small part of the diet, and **nothing turns that depletion into a reason to go farther out**.

### Options

| # | Option | Effect | Risk | Effort |
|---|---|---|---|---|
| 1 | **Depletion pushes boats out.** Fishing range grows with local depletion, boat range and seacraft; parties can land on coasts they find. Also slow fish regrowth or lower the floor so the push actually bites. | More seacraft and sightings, and short hops of 3–8 cells. | Low. | Low–medium, 1–2 days. |
| 2 | **Offshore banks, upwellings and reefs** out to 10–15 cells. | A real reason to go out, and sightings from the fishing grounds. | Near other landmasses, crossings could become almost automatic. | Medium: map generation changes, and old saves need the new layer filled on load (`Map::upgrade_legacy` pattern). |
| 3 | **Seasonal runs** (salmon-like river runs, spawning shoals). | Mostly about settling and storage; little direct effect on crossings. | Low. | Low–medium. |
| 4 | **Preserved fish as voyage provisions.** Use the existing "prepared" flag and stored food: dried or salted fish lengthen how long a crossing can be survived. | Longer crossings survived, mainly late in the game. | Too easy if it stacks with sails; tie it to the amount actually stored. | Low. |
| 5 | **Whales and seals** (cold coasts; a new stock layer, items and recipes). | Strong reason for long cold-water trips. | Needs slow regrowth, or it is just more fish. With slow regrowth, real collapses become possible. | Medium–high. |

**Suggested order:** 1 together with a light version of 2 first, then 4. Keep 3 and 5 for a later
content pass. Option 5 overlaps with the "hunting sea mammals" item in `REFACTOR-NOTES.md`.

## Wells and drains (decided Oct 26 2026)

Disease stays as it is for now: it rising with crowding and farming is realistic. Michael's
decision on the inventions proposal (question 6): wells, drains and other public-health works
come later, together with towns and politics (they need lime or brick, and someone to organise
the digging). Not part of the inventions plan.


## Steel armour (noted Oct 27 2026)

Step 7 makes fights deadlier only through the edge (Michael's decision). Steel (or iron) mail,
helmets and plate would cut losses (`loss_guard`) and so offset steel weapons; left for a later
pass, with a war-rate check like step 7's.
