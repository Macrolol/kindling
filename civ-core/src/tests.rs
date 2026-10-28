use crate::materials::*;
use crate::physics::{self, Action, Ctx};
use crate::{Config, Params, World};

fn small() -> World {
    World::new(Config { seed: 7, map_w: 80, map_h: 50, ..Default::default() }, Params::default())
}

#[test]
fn deterministic() {
    let mut a = small();
    let mut b = small();
    for _ in 0..120 {
        a.step();
        b.step();
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn save_round_trip_continues_identically() {
    let mut a = small();
    for _ in 0..80 {
        a.step();
    }
    let bytes = a.save_bytes(None, "test", "now").unwrap();
    let (mut b, _, info) = World::load_bytes(&bytes).unwrap();
    assert_eq!(info.format_version, crate::save::FORMAT_VERSION);
    let js = a.save_json(None, "test", "now").unwrap();
    let (mut c, _, _) = World::load_bytes(js.as_bytes()).unwrap();
    for _ in 0..40 {
        a.step();
        b.step();
        c.step();
    }
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.state_hash(), c.state_hash());
}

#[test]
fn physics_sanity() {
    let ctx = Ctx { dryness: 0.5, water: true };
    let raw = |m| item(Form::Raw, m);
    // no copper from an open wood fire
    let inp = physics::canonical(Action::Heat, [raw(Mat::CopperOre), NONE, item(Form::Fire, Mat::Wood), NONE]);
    assert!(physics::apply(Action::Heat, &inp, &ctx).map(|o| mat(o.out) != Mat::Copper).unwrap_or(true));
    // knapping flint with a stone hammer gives an edge
    let inp = physics::canonical(Action::Shape, [raw(Mat::Flint), item(Form::Hammer, Mat::Stone), NONE, NONE]);
    assert_eq!(physics::apply(Action::Shape, &inp, &ctx).map(|o| o.out), Some(item(Form::Edge, Mat::Flint)));
    // charcoal in a clay kiln smelts copper
    let inp = physics::canonical(
        Action::Heat,
        [raw(Mat::CopperOre), NONE, item(Form::Fire, Mat::Charcoal), item(Form::Kiln, Mat::Clay)],
    );
    assert_eq!(physics::apply(Action::Heat, &inp, &ctx).map(|o| mat(o.out)), Some(Mat::Copper));
}

/// A save written by the v4 build (before farming) must load through v4->v5: stocks grow to the
/// new materials, the new craft leanings go in before the action leanings, farming layers start
/// fresh (untended, fertile), nobody farms yet; it keeps running and round-trips.
#[test]
fn v4_fixture_loads_and_runs() {
    let bytes = include_bytes!("../tests/fixtures/v4_seed7_y60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v4 save loads");
    assert_eq!(info.format_version, 4);
    assert_eq!(info.migrated_from, Some(4));
    assert!(w.total_pop() > 0);
    let n = w.map.w * w.map.h;
    assert_eq!(w.tend.len(), n);
    assert!(w.fert.iter().all(|&f| f == 1.0));
    assert!(w.tend.iter().all(|&t| t == 0.0));
    assert!(w.map.gen_version >= 4);
    for b in &w.bands {
        assert_eq!(b.stock.len(), NMAT);
        assert_eq!(b.origin.len(), NMAT);
        assert_eq!(b.craft.len(), crate::band::CRAFT_LEN);
        assert!(b.craft[18..23].iter().all(|&c| c == 1.0));
        assert_eq!(b.sow, 0.0);
        assert_eq!(b.herd, 0.0);
        assert_eq!(b.stock[Mat::Iron as usize], 0.0);
    }
    let t0 = w.tick;
    for _ in 0..80 {
        w.step();
    }
    assert_eq!(w.tick, t0 + 80);
    assert!(w.tend.iter().any(|&t| t > 0.0), "camps start to tend wild stands");
    let again = w.save_bytes(None, "test", "test").expect("saves");
    let (mut w2, _, info2) = World::load_bytes(&again).expect("reloads");
    assert_eq!(info2.format_version, crate::save::FORMAT_VERSION);
    assert_eq!(w2.tend, w.tend);
    w.step();
    w2.step();
    assert_eq!(w.state_hash(), w2.state_hash());
}

/// A save written by the farming beta (`dist/kindling-farming-beta-linux-x86_64`, the q7 build
/// Michael ran long sims with) is already format 5: it must load as-is (no migration), rebuild
/// the derived wild-progenitor layer, keep its farming state, run on and round-trip.
#[test]
fn v5beta_fixture_loads_and_runs() {
    let bytes = include_bytes!("../tests/fixtures/v5beta_seed4_y180_64x40.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("beta save loads");
    assert_eq!(info.format_version, 5);
    // format 5 -> 6 only made the first-reach table sparse
    assert_eq!(info.migrated_from, Some(5));
    assert!(w.total_pop() > 0);
    let n = w.map.w * w.map.h;
    assert_eq!(w.tend.len(), n);
    assert!(w.tend.iter().any(|&t| t > 0.0), "beta tending survives the load");
    for b in &w.bands {
        assert_eq!(b.stock.len(), NMAT);
        assert_eq!(b.craft.len(), crate::band::CRAFT_LEN);
    }
    let pop0 = w.total_pop();
    let t0 = w.tick;
    for _ in 0..80 {
        w.step();
    }
    assert_eq!(w.tick, t0 + 80);
    assert_eq!(w.wild.len(), n, "wild progenitor layer rebuilt");
    assert!(w.total_pop() > pop0 / 2);
    let again = w.save_bytes(None, "test", "test").expect("saves");
    let (mut w2, _, _) = World::load_bytes(&again).expect("reloads");
    w.step();
    w2.step();
    assert_eq!(w.state_hash(), w2.state_hash());
}

/// A save written by the v3 build (before conflict) must load through v3->v4, start with no
/// grudges and an empty conflict overlay, and keep running (fights may now happen).
#[test]
fn v3_fixture_loads_and_runs() {
    let bytes = include_bytes!("../tests/fixtures/v3_seed7_y60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v3 save loads");
    assert_eq!(info.format_version, 3);
    assert_eq!(info.migrated_from, Some(3));
    assert!(w.total_pop() > 0);
    assert_eq!(w.tick, 240);
    for b in &w.bands {
        assert!(b.grudges.is_empty());
        assert_eq!(b.pressed, 0.0);
    }
    assert_eq!(w.counters.fights, 0);
    assert_eq!(w.params.aggression, crate::params::Params::default().aggression);
    let t0 = w.tick;
    for _ in 0..40 {
        w.step();
    }
    assert_eq!(w.tick, t0 + 40);
    assert_eq!(w.conflict_flow.len(), w.map.w * w.map.h);
    let again = w.save_bytes(None, "test", "test").expect("saves");
    let (w2, _, info2) = World::load_bytes(&again).expect("reloads");
    assert_eq!(info2.format_version, crate::save::FORMAT_VERSION);
    assert_eq!(w2.counters.fights, w.counters.fights);
}

/// A save written by the v2 build (before lineage knowledge / craft habits) must load through
/// the v2->v3 migration and keep running.
#[test]
fn v2_fixture_loads_and_runs() {
    let bytes = include_bytes!("../tests/fixtures/v2_seed7_y60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v2 save loads");
    assert_eq!(info.format_version, 2);
    assert_eq!(info.migrated_from, Some(2));
    assert!(w.total_pop() > 0);
    let mut own = 0;
    for b in &w.bands {
        assert_eq!(b.stock.len(), NMAT);
        assert_eq!(b.craft.len(), crate::band::CRAFT_LEN);
        assert!(b.far_shore.is_none());
        for k in &b.known {
            assert_eq!(k.kin, k.from == crate::knowledge::FROM_SELF);
            own += k.kin as usize;
        }
    }
    assert!(own > 0, "own finds kept as lineage knowledge");
    let t0 = w.tick;
    for _ in 0..40 {
        w.step();
    }
    assert_eq!(w.tick, t0 + 40);
    // and it round-trips in the current format
    let again = w.save_bytes(None, "test", "test").expect("saves");
    let (_, _, info2) = World::load_bytes(&again).expect("reloads");
    assert_eq!(info2.format_version, crate::save::FORMAT_VERSION);
}

/// A save written by the v1 (pre-food-tech, pre-trade) build must still load, be migrated,
/// have its per-season figures recomputed, and keep running.
#[test]
fn v1_fixture_loads_and_runs() {
    let bytes = include_bytes!("../tests/fixtures/v1_seed7_y60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v1 save loads");
    assert_eq!(info.format_version, 1);
    assert_eq!(info.migrated_from, Some(1));
    assert!(w.total_pop() > 0);
    assert!(!w.bands.is_empty());
    for b in &w.bands {
        assert_eq!(b.stock.len(), NMAT);
        assert_eq!(b.origin.len(), NMAT);
        // figures are recomputed on load, not left at zero
        assert!(b.d.need > 0.0, "need recomputed");
        assert!(b.d.food_ratio > 0.0, "food ratio recomputed");
    }
    assert_eq!(w.map.gen_version, 4, "legacy map upgraded (iron ore added too), but gains no gold");
    let t0 = w.tick;
    for _ in 0..40 {
        w.step();
    }
    assert_eq!(w.tick, t0 + 40);
    assert!(w.total_pop() > 0);
    // and it saves again at the current version
    let again = w.save_bytes(None, "test", "now").unwrap();
    let (_, _, info2) = World::load_bytes(&again).unwrap();
    assert_eq!(info2.format_version, crate::save::FORMAT_VERSION);
}

/// Diagnostic (run by hand): what farming bands harvest.
#[test]
#[ignore]
fn farm_diag() {
    let mut w = World::new(Config { seed: 2, map_w: 160, map_h: 100, ..Default::default() }, Params::default());
    for y in 0..200 {
        for _ in 0..4 {
            w.step();
        }
        if y % 10 == 9 {
            let s = w.season();
            let mut n = 0;
            for b in w.bands.iter().filter(|b| b.sow > 0.0).take(4) {
                let e = w.economy(b, s);
                let ci = b.cell(w.map.w);
                println!(
                    "Y{} band {} pop {} sow {:.2} strain {:.2} settled {:.2} | ys {:?} h {:?} need {:.0} food {:.0} ratio {:.2} | cell tend {:.2} crop {:.1} pot {:.2} dom {:.2} plants {:.1} | farm_m {:.2} dg4 {:.2}",
                    y + 1, b.id, b.pop, b.sow, b.strain, b.settled, e.ys.map(|v| v as i32), e.h.map(|v| v as i32), e.need, e.food, e.food_ratio,
                    w.tend[ci], w.crop[ci], w.crop_pot[ci], w.dom[ci], w.plants[ci], b.d.kit.farm_m, b.d.kit.digest[4]
                );
                n += 1;
            }
            if n > 0 { println!("--"); }
        }
    }
}

/// Basic sanity and conservation over a few hundred seasons: everyone is on habitable land, no
/// NaNs or negative stocks creep in, wild food stays within the land's capacity, population
/// totals agree with the bands, and band ids stay unique.
#[test]
fn sanity_and_conservation() {
    let mut w = World::new(Config { seed: 3, map_w: 120, map_h: 80, ..Default::default() }, Params::default());
    for t in 0..240 {
        w.step();
        if t % 20 != 19 {
            continue;
        }
        let n = w.map.w * w.map.h;
        let mut ids = std::collections::HashSet::new();
        let mut pop = 0u32;
        for b in &w.bands {
            assert!(ids.insert(b.id), "duplicate band id {}", b.id);
            assert!(b.pop > 0, "empty band {} left in the list", b.id);
            pop += b.pop;
            let c = b.cell(w.map.w);
            assert!(c < n && w.map.is_land(c), "band {} off land", b.id);
            assert!(b.store.is_finite() && b.store >= 0.0, "band {} store {}", b.id, b.store);
            assert!(b.stock.iter().all(|v| v.is_finite() && *v >= -1e-3), "band {} stock", b.id);
        }
        assert_eq!(pop, w.total_pop());
        for i in 0..n {
            // tended land carries more wild plants than its natural capacity
            let kp = match w.tend.get(i) {
                Some(&td) if td > 0.0 => crate::world::farming::wild_plant_cap(w.map.k_plants[i], td),
                _ => w.map.k_plants[i],
            };
            for (v, k) in [(w.plants[i], kp), (w.game[i], w.map.k_game[i]), (w.fish[i], w.map.k_fish[i])] {
                assert!(v.is_finite() && v >= -1e-3, "stock {v} at {i}");
                // (tending decays after the regrowth step, so the tended cap can sit slightly below)
                assert!(v <= k.max(0.0) * 1.02 + 1e-3, "stock {v} over capacity {k} at {i}");
            }
        }
        assert!(w.trade_flow.iter().chain(&w.conflict_flow).all(|v| v.is_finite() && *v >= 0.0));
        assert!(w.tend.iter().chain(&w.dom).chain(&w.fert).all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
    }
    assert!(w.total_pop() > 0);
    let last = w.history.last().unwrap();
    assert_eq!(last.bands as usize, w.bands.len());
}

/// Chosen starts: a shared start area puts every founder within 3 cells of it; one cell per
/// lineage puts each exactly there; sea or ice is refused (falls back to a random homeland);
/// the starts are kept in the save and noted in the chronicle.
#[test]
fn chosen_starts() {
    use crate::StartCheck;
    let base = Config { seed: 7, map_w: 80, map_h: 50, ..Default::default() };
    let m = crate::map::Map::generate(base.map_w, base.map_h, base.seed);
    let good: Vec<(u16, u16)> = (0..m.h)
        .flat_map(|y| (0..m.w).map(move |x| (x, y)))
        .filter(|&(x, y)| m.start_check(x, y) == StartCheck::Ok)
        .map(|(x, y)| (x as u16, y as u16))
        .collect();
    assert!(good.len() > 10);
    let sea = (0..m.w * m.h).find(|&i| !m.is_land(i)).map(|i| ((i % m.w) as u16, (i / m.w) as u16)).unwrap();
    assert_eq!(m.start_check(sea.0 as usize, sea.1 as usize), StartCheck::Uninhabitable);
    // shared area
    let p = good[good.len() / 2];
    let w = World::new(Config { starts: vec![p], ..base.clone() }, Params::default());
    assert_eq!(w.bands.len(), base.founders);
    for b in &w.bands {
        assert!((b.x as i32 - p.0 as i32).abs() <= 3 && (b.y as i32 - p.1 as i32).abs() <= 3);
    }
    assert!(w.events.iter().chain(&w.chronicle).any(|e| w.event_text(e).contains("(chosen start)")));
    // one per lineage (more than the default number of founders)
    let picks: Vec<(u16, u16)> = good.iter().step_by(good.len() / 8).take(8).copied().collect();
    let mut w = World::new(Config { starts: picks.clone(), ..base.clone() }, Params::default());
    assert_eq!(w.bands.len(), 8);
    for (b, &(x, y)) in w.bands.iter().zip(&picks) {
        assert_eq!((b.x, b.y), (x, y));
    }
    // kept in the save
    for _ in 0..8 {
        w.step();
    }
    let bytes = w.save_bytes(None, "test", "now").unwrap();
    let (l, _, _) = World::load_bytes(&bytes).unwrap();
    assert_eq!(l.cfg.starts, picks);
    // sea refused: same as no choice at all
    let a = World::new(Config { starts: vec![sea], ..base.clone() }, Params::default());
    let b = World::new(base.clone(), Params::default());
    let pos = |w: &World| w.bands.iter().map(|b| (b.x, b.y, b.pop)).collect::<Vec<_>>();
    assert_eq!(pos(&a), pos(&b));
    // parsing
    assert_eq!(crate::parse_starts("3,4").unwrap(), vec![(3, 4)]);
    assert_eq!(crate::parse_starts(" 3,4 ; 10, 2;").unwrap(), vec![(3, 4), (10, 2)]);
    assert!(crate::parse_starts("3").is_err());
    assert!(crate::parse_starts("a,b").is_err());
}

/// With the parallel feature, the result does not depend on the number of threads (each band
/// and each meeting draws from its own random stream and effects apply in a fixed order).
#[cfg(feature = "parallel")]
#[test]
fn same_result_on_any_thread_count() {
    let run = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| {
            let mut w = World::new(Config { seed: 5, map_w: 120, map_h: 80, ..Default::default() }, Params::default());
            for _ in 0..320 {
                w.step();
            }
            (w.state_hash(), w.total_pop(), w.bands.len())
        })
    };
    let one = run(1);
    assert!(one.2 >= 20, "too few bands ({}) to exercise the parallel paths", one.2);
    assert_eq!(one, run(4));
}



/// A save written by the v0.6.0 release (format 5, dense first-reach table) must load in v6.1
/// (format 6, sparse table) by both routes: the binary save decoded directly (the dense field is
/// read and converted on load) and the same payload as JSON through the frozen `v5_to_v6`
/// migration. Both must give the same table, which must match the dense one cell by cell, and
/// the world must run on and round-trip in the new format.
#[test]
fn v6_0_fixture_loads_sparse_cell_first() {
    use std::io::Read;
    let bytes = include_bytes!("../tests/fixtures/v6_0_seed4_y150_64x40.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v6.0 save loads");
    assert_eq!(info.format_version, 5);
    assert_eq!(info.migrated_from, Some(5));
    assert!(w.total_pop() > 0);
    assert!(!w.cell_first.is_empty(), "first-reach table survives");
    // the dense original, straight from the file
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(&bytes[8..]).read_to_end(&mut raw).unwrap();
    let env: serde_json::Value = rmp_serde::from_slice(&raw).unwrap();
    let n = w.map.w * w.map.h;
    let dense = env["payload"]["world"]["cell_first"].as_array().expect("dense table in a v6.0 save");
    assert_eq!(dense.len(), w.cell_first.len());
    let mut reached = 0;
    for p in dense {
        let it = p[0].as_u64().unwrap() as Item;
        let cells = p[1].as_array().unwrap();
        assert_eq!(cells.len(), n);
        let cf = &w.cell_first[&it];
        for (c, t) in cells.iter().enumerate() {
            let t = t.as_u64().unwrap() as u32;
            assert_eq!(cf.get(c), t);
            reached += (t != u32::MAX) as usize;
        }
        assert_eq!(cf.iter().count(), cells.iter().filter(|t| t.as_u64() != Some(u32::MAX as u64)).count());
    }
    assert!(reached > 0);
    // the JSON route runs the frozen migration
    let json = serde_json::to_string(&env).unwrap();
    let (wj, _, ij) = World::load_bytes(json.as_bytes()).expect("v6.0 save as JSON loads");
    assert_eq!(ij.migrated_from, Some(5));
    assert_eq!(wj.cell_first, w.cell_first);
    assert_eq!(wj.state_hash(), w.state_hash());
    // runs on, then round-trips in format 6 with the sparse table
    for _ in 0..40 {
        w.step();
    }
    let again = w.save_bytes(None, "test", "test").expect("saves");
    let (mut w2, _, info2) = World::load_bytes(&again).expect("reloads");
    assert_eq!(info2.format_version, crate::save::FORMAT_VERSION);
    assert_eq!(info2.migrated_from, None);
    assert_eq!(w2.cell_first, w.cell_first);
    let js = w.save_json(None, "test", "test").expect("json");
    assert!(!js.contains("\"cell_first\":"), "the dense field is no longer written");
    w.step();
    w2.step();
    assert_eq!(w.state_hash(), w2.state_hash());
}

/// The sparse first-reach table: unset cells read as never, the first tick sticks, iteration is
/// in ascending cell order, and only touched tiles take memory.
#[test]
fn cell_first_sparse_basics() {
    use crate::world::CellFirst;
    let (w, h) = (150, 70);
    let mut cf = CellFirst::new(w, h);
    assert_eq!(cf.get(5), u32::MAX);
    let empty = cf.bytes();
    cf.set_first(149 * 1 + 69 * 150, 9);
    cf.set_first(3, 7);
    cf.set_first(3, 11);
    cf.set_first(64 + 150, 5);
    assert_eq!(cf.get(3), 7);
    let v: Vec<(usize, u32)> = cf.iter().collect();
    assert_eq!(v, vec![(3, 7), (214, 5), (69 * 150 + 149, 9)]);
    assert!(cf.bytes() < empty + 3 * 64 * 64 * 4 + 1);
    let mut dense = vec![u32::MAX; w * h];
    for &(c, t) in &v {
        dense[c] = t;
    }
    assert_eq!(CellFirst::from_dense(w, h, &dense), cf);
}

/// v7 iron: ore only reduces under a bellows blast into a closed charcoal furnace, gives a bloom,
/// and the bloom must be forged hot on a hammerstone into wrought iron.
#[test]
fn iron_needs_bellows_and_forging() {
    let ctx = Ctx { dryness: 0.5, water: false };
    let raw = |m| item(Form::Raw, m);
    let ch = item(Form::Fire, Mat::Charcoal);
    let bel = item(Form::Bellows, Mat::Hide);
    let heat = |inp| physics::apply(Action::Heat, &physics::canonical(Action::Heat, inp), &ctx).map(|o| o.out);
    // the best kiln without a blast no longer reduces iron
    assert_eq!(heat([raw(Mat::IronOre), NONE, ch, item(Form::Kiln, Mat::Ceramic)]), None);
    // with bellows into a clay furnace: a bloom, not usable iron
    assert_eq!(heat([raw(Mat::IronOre), bel, ch, item(Form::Kiln, Mat::Clay)]), Some(item(Form::Bloom, Mat::Iron)));
    // a stone-lined hearth is still too cool, and bellows do nothing for an open fire
    assert_eq!(heat([raw(Mat::IronOre), bel, ch, item(Form::Kiln, Mat::Stone)]), None);
    assert_eq!(heat([raw(Mat::IronOre), bel, ch, NONE]), None);
    // forging: bloom + hammerstone in a hot hearth gives wrought iron, either input order
    let bloom = item(Form::Bloom, Mat::Iron);
    let ham = item(Form::Hammer, Mat::Stone);
    assert_eq!(heat([bloom, ham, ch, item(Form::Kiln, Mat::Stone)]), Some(raw(Mat::Iron)));
    assert_eq!(heat([ham, bloom, ch, item(Form::Kiln, Mat::Clay)]), Some(raw(Mat::Iron)));
    // not in a plain wood fire
    assert_eq!(heat([bloom, ham, item(Form::Fire, Mat::Wood), NONE]), None);
    // a bloom cannot be cold-hammered into a knife
    assert_eq!(physics::apply(Action::Shape, &[bloom, ham, NONE, NONE], &ctx), None);
    // bellows: a hide bag on a stick with a cord
    let inp = physics::canonical(Action::Combine, [item(Form::Vessel, Mat::Hide), item(Form::Shaft, Mat::Wood), item(Form::Cord, Mat::Fibre), NONE]);
    assert_eq!(physics::apply(Action::Combine, &inp, &ctx).map(|o| o.out), Some(bel));
    assert_eq!(item_name(bloom), "iron bloom");
    assert_eq!(item_name(raw(Mat::Iron)), "wrought iron");
}

/// A v6.1 save (format 6) whose bands already smelt iron the old way: the v6->v7 migration
/// re-points those recipes at the bloom, by both the binary and the JSON route; it keeps running.
#[test]
fn v6_1_fixture_iron_recipes_become_bloom() {
    use std::io::Read;
    let bytes = include_bytes!("../tests/fixtures/v6_1_seed4_y125.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v6.1 save loads");
    assert_eq!(info.format_version, 6);
    assert_eq!(info.migrated_from, Some(6));
    let ore = item(Form::Raw, Mat::IronOre);
    let smelt = |w: &World| w.reg.recipes.iter().filter(|r| r.action == 2 && r.inputs.contains(&ore)).map(|r| r.output).collect::<Vec<_>>();
    let s = smelt(&w);
    assert!(!s.is_empty(), "fixture has an iron smelting recipe");
    assert!(s.iter().all(|&o| o != item(Form::Raw, Mat::Iron)));
    assert!(s.contains(&item(Form::Bloom, Mat::Iron)));
    let pop = w.total_pop();
    for _ in 0..20 {
        w.step();
    }
    assert!(w.total_pop() > pop / 2);
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(&bytes[8..]).read_to_end(&mut raw).unwrap();
    let env: serde_json::Value = rmp_serde::from_slice(&raw).unwrap();
    let json = serde_json::to_string(&env).unwrap();
    let (wj, _, ij) = World::load_bytes(json.as_bytes()).expect("v6.1 save as JSON loads");
    assert_eq!(ij.migrated_from, Some(6));
    let (wb, _, _) = World::load_bytes(bytes).unwrap();
    assert_eq!(smelt(&wj), smelt(&wb));
}

/// A v0.7.0 save (format 7): brine and salt join the material list, so every band's stock,
/// origin and craft vectors grow; the craft leanings must keep their meaning (the new neutral
/// material leanings go in before the action leanings), by both the binary and the JSON route.
#[test]
fn v7_0_fixture_gains_new_materials_in_place() {
    use crate::band::CRAFT_LEN;
    use crate::materials::NMAT;
    use std::io::Read;
    let bytes = include_bytes!("../tests/fixtures/v7_0_seed4_y150_96x60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v7.0 save loads");
    assert_eq!(info.format_version, 7);
    assert_eq!(info.migrated_from, Some(7));
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(&bytes[8..]).read_to_end(&mut raw).unwrap();
    let env: serde_json::Value = rmp_serde::from_slice(&raw).unwrap();
    let old = env["payload"]["world"]["bands"].as_array().unwrap();
    assert_eq!(old.len(), w.bands.len());
    for (b, o) in w.bands.iter().zip(old) {
        assert_eq!(b.stock.len(), NMAT);
        assert_eq!(b.origin.len(), NMAT);
        assert_eq!(b.craft.len(), CRAFT_LEN);
        let oc: Vec<f32> = o["craft"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect();
        assert_eq!(oc.len(), 26);
        assert_eq!(&b.craft[..23], &oc[..23]);
        // every material added since v7 (brine and salt in v8, pitch in v11) leans neutral
        assert!(b.craft[23..NMAT].iter().all(|&c| c == 1.0));
        assert_eq!(&b.craft[NMAT..], &oc[23..]);
        assert!(b.stock[23..].iter().all(|&x| x == 0.0));
    }
    let json = serde_json::to_string(&env).unwrap();
    let (wj, _, ij) = World::load_bytes(json.as_bytes()).expect("v7.0 save as JSON loads");
    assert_eq!(ij.migrated_from, Some(7));
    for (a, b) in wj.bands.iter().zip(&w.bands) {
        assert_eq!(a.craft, b.craft);
        assert_eq!(a.stock, b.stock);
        assert_eq!(a.origin, b.origin);
    }
    let pop = w.total_pop();
    for _ in 0..20 {
        w.step();
    }
    assert!(w.total_pop() > pop / 2);
}

/// A v0.8.0 save (format 8, written by the release code at 4d301dc: `--seed 4 --years 150
/// --map 96x60`). It must keep loading, by the binary and the JSON route alike, with the same
/// bands, and keep running. Its landfalls predate the v9 boat/foot record, so the analysis
/// reports how they were reached as unknown.
#[test]
fn v8_0_fixture_loads_and_runs() {
    use std::io::Read;
    let bytes = include_bytes!("../tests/fixtures/v8_0_seed4_y150_96x60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v8.0 save loads");
    assert_eq!(info.format_version, 8);
    assert_eq!(info.migrated_from, Some(8));
    assert_eq!(w.year(), 150.0);
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(&bytes[8..]).read_to_end(&mut raw).unwrap();
    let env: serde_json::Value = rmp_serde::from_slice(&raw).unwrap();
    let old = env["payload"]["world"]["bands"].as_array().unwrap();
    assert_eq!(old.len(), w.bands.len());
    assert!(w.landfalls.is_empty());
    let json = serde_json::to_string(&env).unwrap();
    let (wj, _, _) = World::load_bytes(json.as_bytes()).expect("v8.0 save as JSON loads");
    assert_eq!(wj.bands.len(), w.bands.len());
    for (a, b) in wj.bands.iter().zip(&w.bands) {
        assert_eq!((a.id, a.pop, a.x, a.y, a.lineage), (b.id, b.pop, b.x, b.y, b.lineage));
        assert_eq!(a.far_shore, b.far_shore);
        assert_eq!(a.far_shore_year, b.far_shore_year);
        assert_eq!(a.known.len(), b.known.len());
        // (v9) rumours already held are old news: heard 40 years before the save
        if b.far_shore.is_some() {
            assert_eq!(b.far_shore_year, 110);
        }
    }
    // the fixture predates boats, so give a few of its bands a rumour and run the migration
    let mut env2 = env.clone();
    for b in env2["payload"]["world"]["bands"].as_array_mut().unwrap().iter_mut().take(5) {
        b["far_shore"] = serde_json::json!(17);
    }
    let (wm, _, im) = World::load_bytes(serde_json::to_string(&env2).unwrap().as_bytes()).expect("loads");
    assert_eq!(im.migrated_from, Some(8));
    assert!(wm.bands.iter().take(5).all(|b| b.far_shore == Some(17) && b.far_shore_year == 110));
    assert!(wm.bands.iter().skip(5).all(|b| b.far_shore.is_some() || b.far_shore_year == 0));
    // (v9) no camp layer in the save: settled land counts as camped in the load year
    assert_eq!(w.camped.len(), w.map.w * w.map.h);
    assert!(w.bands.iter().all(|b| w.camped[b.cell(w.map.w)] == 151));
    assert!((0..w.camped.len()).all(|i| w.map.is_land(i) || w.camped[i] == 0));
    let s = crate::analysis::summarize(&w);
    assert!(s.continents.iter().all(|c| c.by_sea.is_none()));
    let pop = w.total_pop();
    for _ in 0..20 {
        w.step();
    }
    assert!(w.total_pop() > pop / 2);
}

/// Terrain effort (v8 step 2): never below the cell distance, equal to it on flat ground, more
/// over mountains; a boat makes a shared river cheaper but never cheaper than one step.
#[test]
fn terrain_cost_and_rivers() {
    let w = World::new(Config { seed: 4, map_w: 160, map_h: 100, ..Default::default() }, Params::default());
    let m = &w.map;
    let n = m.w * m.h;
    let mut steep = 0;
    for a in (0..n).step_by(37) {
        let b = (a + 3 * m.w + 2) % n;
        let d = crate::world::terrain::cheb(a, b, m.w) as f32;
        let c = w.land_cost(a, b);
        assert!(c >= d - 1e-4, "cost {c} below distance {d}");
        steep += (c > 1.5 * d) as u32;
        assert!(w.travel_cost(a, b, true) >= 1.0f32.min(d));
    }
    assert!(steep > 0, "some lines cross rough country");
    // flat: a sea-level line costs exactly its length
    let sea: Vec<usize> = (0..m.w).map(|x| 5 * m.w + x).collect();
    if sea.iter().all(|&c| !m.is_land(c)) {
        assert!((w.land_cost(sea[0], sea[10]) - 10.0).abs() < 1e-4);
    }
    let rc: Vec<usize> = (0..n).filter(|&c| m.river[c]).collect();
    assert!(!rc.is_empty());
    assert!(rc.iter().all(|&c| w.river_sys[c] != u32::MAX));
    let far = rc.iter().copied().find(|&c| rc.iter().any(|&o| w.same_river(c, o) && crate::world::terrain::cheb(c, o, m.w) >= 5));
    let a = far.expect("a river at least 5 cells long");
    let b = rc.iter().copied().find(|&o| w.same_river(a, o) && crate::world::terrain::cheb(a, o, m.w) >= 5).unwrap();
    assert!(w.travel_cost(a, b, true) < w.travel_cost(a, b, false));
}

#[test]
fn bows_shields_and_metal_weapons() {
    let dry = Ctx { dryness: 0.5, water: false };
    let wet = Ctx { dryness: 0.5, water: true };
    let shaft = item(Form::Shaft, Mat::Wood);
    let thong = item(Form::Cord, Mat::Hide);
    let bow = physics::apply(Action::Shape, &physics::canonical(Action::Shape, [shaft, thong, NONE, NONE]), &dry).unwrap();
    assert_eq!(bow.out, item(Form::Bow, Mat::Hide));
    let inp = physics::canonical(Action::Combine, [item(Form::Raw, Mat::Hide), shaft, thong, NONE]);
    assert_eq!(physics::apply(Action::Combine, &inp, &dry).unwrap().out, item(Form::Shield, Mat::Hide));
    assert_eq!(physics::apply(Action::Combine, &inp, &wet).unwrap().out, item(Form::Boat, Mat::Hide));
    // a spear's worth in a fight counts how long its head lasts: metal beats flint
    let spear = |m: Mat| {
        let mut s = ItemSet::default();
        s.add(item(Form::Spear, m));
        crate::band::weapon_q(&crate::band::make_kit(&s))
    };
    assert!(spear(Mat::Bronze) > spear(Mat::Flint) && spear(Mat::Iron) > spear(Mat::Flint));
    // a bow without arrows is no use
    let mut s = ItemSet::default();
    s.add(item(Form::Bow, Mat::Hide));
    assert!(!crate::band::has_bow(&crate::band::make_kit(&s)));
    s.add(item(Form::Point, Mat::Bone));
    let k = crate::band::make_kit(&s);
    assert!(crate::band::has_bow(&k) && crate::band::loss_guard(&k) < 1.0);
}

#[test]
fn salt_from_brine() {
    let ctx = Ctx { dryness: 0.5, water: true };
    let brine = item(Form::Raw, Mat::Brine);
    let fire = item(Form::Fire, Mat::Wood);
    let heat = |inp| physics::apply(Action::Heat, &physics::canonical(Action::Heat, inp), &ctx).map(|o| o.out);
    // boiled down in a fired pot it leaves salt; in an unfired or wooden one it does not
    assert_eq!(heat([brine, item(Form::Vessel, Mat::Ceramic), fire, NONE]), Some(item(Form::Raw, Mat::Salt)));
    assert_ne!(heat([brine, item(Form::Vessel, Mat::Wood), fire, NONE]), Some(item(Form::Raw, Mat::Salt)));
    assert_ne!(heat([brine, NONE, fire, NONE]), Some(item(Form::Raw, Mat::Salt)));
}

#[test]
fn outriggers_sails_and_plank_boats() {
    let wet = Ctx { dryness: 0.5, water: true };
    let dry = Ctx { dryness: 0.5, water: false };
    let boat = item(Form::Boat, Mat::Wood);
    let shaft = item(Form::Shaft, Mat::Wood);
    let cord = item(Form::Cord, Mat::Fibre);
    let comb = |inp, c: &Ctx| physics::apply(Action::Combine, &physics::canonical(Action::Combine, inp), c).map(|o| o.out);
    assert_eq!(comb([boat, shaft, cord, NONE], &wet), Some(item(Form::Outrigger, Mat::Wood)));
    assert_eq!(comb([boat, shaft, cord, NONE], &dry), None);
    // (inventions step 12) a sail is cloth on a mast stepped in a plank hull: not clothing, and
    // not on a dugout
    let plank = item(Form::Plank, Mat::Wood);
    let linen = item(Form::Cloth, Mat::Flax);
    assert_eq!(comb([boat, shaft, item(Form::Garment, Mat::Flax), NONE], &wet), None);
    assert_eq!(comb([boat, shaft, linen, NONE], &wet), None);
    assert_eq!(comb([plank, shaft, item(Form::Garment, Mat::Flax), NONE], &wet), None);
    assert_eq!(comb([plank, shaft, linen, NONE], &wet), Some(item(Form::Sail, Mat::Flax)));
    assert_eq!(comb([plank, shaft, linen, NONE], &dry), None);
    // planks need a metal axe; a stone axe only hollows a dugout
    let shape = |t, tool| physics::apply(Action::Shape, &physics::canonical(Action::Shape, [t, tool, NONE, NONE]), &wet).map(|o| o.out);
    assert_eq!(shape(item(Form::Raw, Mat::Wood), item(Form::Axe, Mat::Bronze)), Some(item(Form::Plank, Mat::Wood)));
    assert_ne!(shape(item(Form::Raw, Mat::Wood), item(Form::Axe, Mat::Flint)), Some(item(Form::Plank, Mat::Wood)));
    // an outrigger is safer, a sail goes further (for people who know the sea)
    let mut s = ItemSet::default();
    s.add(boat);
    let k0 = crate::band::make_kit(&s);
    s.add(item(Form::Outrigger, Mat::Wood));
    s.add(item(Form::Sail, Mat::Flax));
    s.add(plank);
    let k1 = crate::band::make_kit(&s);
    assert!(World::boat_safety(&k1) > World::boat_safety(&k0));
    assert!(World::boat_range(&k1, 0.5) > 1.5 * World::boat_range(&k0, 0.5));
    let mut k1b = k1.clone();
    k1b.sail = NONE;
    assert_eq!(World::boat_range(&k1, 0.2), World::boat_range(&k1b, 0.2));
    // (inventions step 12) no sail on a dugout, whatever it is woven or sewn of
    let mut k2 = k1.clone();
    k2.plank = NONE;
    k2.ship = NONE;
    assert!(!World::sail_usable(&k2, 0.9) && World::sail_usable(&k1, 0.9));
}

#[test]
fn frames_ornaments_and_medicine() {
    let dry = Ctx { dryness: 0.5, water: false };
    let shaft = item(Form::Shaft, Mat::Wood);
    let cord = item(Form::Cord, Mat::Fibre);
    let comb = |inp| physics::apply(Action::Combine, &physics::canonical(Action::Combine, inp), &dry).map(|o| o.out);
    // a basket on a frame carries goods; a hide bag on a stick is still a bellows
    assert_eq!(comb([item(Form::Vessel, Mat::Fibre), shaft, cord, NONE]), Some(item(Form::Frame, Mat::Fibre)));
    assert_eq!(comb([item(Form::Vessel, Mat::Hide), shaft, cord, NONE]), Some(item(Form::Bellows, Mat::Hide)));
    let shape = |t| physics::apply(Action::Shape, &physics::canonical(Action::Shape, [t, item(Form::Hammer, Mat::Stone), NONE, NONE]), &dry).map(|o| o.out);
    // (inventions step 12) soft tin makes no point to beat into a pendant; copper and bone do
    assert_eq!(shape(item(Form::Point, Mat::Tin)), None);
    assert_eq!(shape(item(Form::Point, Mat::Copper)), Some(item(Form::Ornament, Mat::Copper)));
    assert_eq!(shape(item(Form::Point, Mat::Flint)), None);
    // herbal lore and wariness load as zero from older saves
    let b = crate::band::Band::default();
    assert_eq!((b.herbal, b.wary), (0.0, 0.0));
}

#[test]
fn riding_counts_only_with_a_herd_on_open_land() {
    let w = small();
    let open = (0..w.map.w * w.map.h).find(|&c| w.open_land(c)).expect("some open land");
    let wood = (0..w.map.w * w.map.h).find(|&c| w.map.is_land(c) && !w.open_land(c)).expect("some closed land");
    let at = |c: usize, ride: f32, herd: f32| {
        let mut b = crate::band::Band::default();
        b.x = (c % w.map.w) as u16;
        b.y = (c / w.map.w) as u16;
        b.ride = ride;
        b.herd = herd;
        w.mounted(&b)
    };
    assert_eq!(at(open, 0.8, 0.5), 0.8);
    assert_eq!(at(wood, 0.8, 0.5), 0.0);
    assert_eq!(at(open, 0.8, 0.0), 0.0);
    assert_eq!(at(open, 0.0, 0.5), 0.0);
}


/// (v9) Measurement helpers: the effective number of lineages, and open-water gaps between
/// landmasses (0 for the source itself, more than 0 for any other landmass it can reach).
#[test]
fn effective_number_and_sea_gaps() {
    use crate::analysis::effective_number;
    assert_eq!(effective_number([10u64, 10, 10].into_iter()), 3.0);
    assert_eq!(effective_number([7u64].into_iter()), 1.0);
    assert_eq!(effective_number(std::iter::empty()), 0.0);
    let e = effective_number([90u64, 10].into_iter());
    assert!((e - 1.0 / (0.81 + 0.01)).abs() < 1e-5);
    let w = World::new(Config { seed: 4, map_w: 160, map_h: 100, ..Default::default() }, Params::default());
    let m = &w.map;
    let g = m.sea_gaps(|l| l == 1);
    assert_eq!(g.len(), m.landmass_size.len());
    assert_eq!(g[1], Some(0));
    assert!(g.iter().skip(2).all(|x| x.map_or(true, |d| d >= 1)));
    assert!(m.continent_cells() >= crate::map::Map::CONTINENT_CELLS);
}

/// (v9 step 1b) Nobody walks across water: a straight walk is allowed only over land, and the
/// random homeland lies on a continent away from any strait.
#[test]
fn no_walking_across_water_and_clear_starts() {
    for seed in [1u64, 4, 7] {
        let w = World::new(Config { seed, ..Default::default() }, Params::default());
        let m = &w.map;
        let n = m.w * m.h;
        let mut checked = 0;
        for a in (0..n).step_by(97).filter(|&i| m.is_land(i)) {
            for b in [a + 1, a + 4 * m.w + 3, a + 6].into_iter().filter(|&b| b < n) {
                if m.walkable(a, b) {
                    assert_eq!(m.landmass_of(a), m.landmass_of(b));
                    checked += 1;
                }
            }
            assert!(m.walkable(a, a));
        }
        assert!(checked > 0);
        let home = w.bands[0].cell(m.w);
        let l = m.landmass_of(home);
        assert!(w.bands.iter().all(|b| m.landmass_of(b.cell(m.w)) == l), "founders on one landmass");
        assert!(m.is_continent(l as usize));
    }
}

#[test]
fn ancestry_mixes_and_stays_normalised() {
    let mut b = crate::band::Band { lineage: 1, ..Default::default() };
    assert_eq!(b.ancestry(), vec![(1, 1.0)]);
    b.mix_ancestry(&[(2, 1.0)], 0.25);
    let a = b.ancestry();
    assert_eq!(a[0].0, 1);
    assert!((a[0].1 - 0.75).abs() < 1e-5 && (a[1].1 - 0.25).abs() < 1e-5);
    for l in 3..20 {
        b.mix_ancestry(&[(l, 1.0)], 0.05);
    }
    let a = b.ancestry();
    assert!(a.len() <= crate::band::ANCESTRY_SLOTS);
    assert!((a.iter().map(|e| e.1).sum::<f32>() - 1.0).abs() < 1e-4);
    // a band whose ancestry is wholly its own lineage stores nothing (older saves look the same)
    let mut c = crate::band::Band { lineage: 5, ..Default::default() };
    c.mix_ancestry(&[(5, 1.0)], 0.5);
    assert!(c.ancestry.is_empty());
}

#[test]
fn refined_recipes_make_better_kits() {
    let mut w = small();
    for _ in 0..200 {
        w.step();
    }
    w.refresh_all_toolkits();
    // (diffusion D3) a finer recipe can also change which tool is relied on (payoff, not raw
    // cutting yield): compare bands whose chosen kit stays the same
    let before: Vec<(f32, [Item; 30])> = w.bands.iter().map(|b| (b.d.kit.game_m * b.d.kit.plants_m, b.d.kit.used_items())).collect();
    for b in &mut w.bands {
        for k in &mut b.known {
            k.quality = k.skill;
        }
        b.d.fin_dirty = true;
    }
    w.refresh_all_toolkits();
    let after: Vec<(f32, [Item; 30])> = w.bands.iter().map(|b| (b.d.kit.game_m * b.d.kit.plants_m, b.d.kit.used_items())).collect();
    let same: Vec<(f32, f32)> = before.iter().zip(&after).filter(|(a, b)| a.1 == b.1).map(|(a, b)| (a.0, b.0)).collect();
    assert!(same.len() * 2 > before.len());
    assert!(same.iter().all(|(a, b)| b >= a));
    assert!(same.iter().any(|(a, b)| b > a));
}

#[test]
fn prestige_biases_learning_and_gini_measures_spread() {
    use crate::analysis::gini;
    use crate::world::prestige_bias;
    let mut w = small();
    // (inventions step 10) standing now comes from giving (feasts, food to the hungry), fine
    // work and voyages, not from spare labour, so it takes the first stores of food to show
    for _ in 0..600 {
        w.step();
    }
    // bands earn standing over time, and it never goes negative
    assert!(w.bands.iter().all(|b| b.prestige >= 0.0));
    assert!(w.bands.iter().any(|b| b.prestige > 0.0));
    let (mut hi, mut lo) = (w.bands[0].clone(), w.bands[0].clone());
    hi.prestige = 2.0;
    lo.prestige = 0.0;
    assert!(prestige_bias(&hi, &lo) > 1.0 && prestige_bias(&lo, &hi) < 1.0);
    assert!(prestige_bias(&hi, &lo) <= 2.0 && prestige_bias(&lo, &hi) >= 0.5);
    assert_eq!(gini(&[1.0, 1.0, 1.0]), 0.0);
    assert!(gini(&[0.0, 0.0, 0.0, 9.0]) > 0.7);
}

#[test]
fn item_names_are_unique() {
    // reports and summaries key items by name; two items sharing one would be merged
    let mut seen = std::collections::HashMap::new();
    for (fi, _) in ALL_FORMS.iter().enumerate() {
        for (mi, _) in ALL_MATS.iter().enumerate() {
            let i = (fi * 32 + mi) as Item;
            if !valid_item(i) {
                continue;
            }
            if let Some(prev) = seen.insert(item_name(i), i) {
                panic!("items {prev} and {i} are both called {:?}", item_name(i));
            }
        }
    }
}

#[test]
fn summary_accounts_for_lost_items_and_landmasses() {
    let mut w = small();
    for _ in 0..600 {
        w.step();
    }
    let s = crate::analysis::summarize(&w);
    // everything once made but made by nobody now is listed, quiet local losses included
    let gone: Vec<Item> = s.gone_now.iter().map(|g| g.item).collect();
    for f in &s.firsts {
        assert_eq!(f.bands_now == 0, gone.contains(&f.item), "{}", f.name);
    }
    if s.stage_now != s.stage_ever {
        assert!(!s.gone_now.is_empty());
    }
    // the homeland is flagged once; reached land is 0 sea cells from settled land
    assert!(s.continents.iter().filter(|c| c.home).count() <= 1);
    assert!(s.continents.iter().all(|c| c.home == (c.id == s.home_landmass)));
    for c in &s.continents {
        if c.landfall_year.is_some() {
            assert_eq!(c.gap_now, Some(0));
        }
    }
    assert!(s.islands_by_sea <= s.islands_reached);
    assert_eq!(s.island_hop_max.is_some(), s.islands_by_sea > 0 && w.landfalls.iter().any(|r| r.by_sea && r.gap != u32::MAX && (r.landmass as usize) < w.map.landmass_size.len() && w.map.landmass_size[r.landmass as usize] < w.map.continent_cells()));
}

/// Format 10 binary saves (compact blobs) hold exactly the same world as JSON, and the blobs
/// survive a world with partners, teachers and inherited recipes.
#[test]
fn v10_binary_is_exact() {
    let mut a = small();
    for _ in 0..160 {
        a.step();
    }
    assert!(a.bands.iter().any(|b| !b.partners.is_empty()), "test world should have partners");
    let bytes = a.save_bytes(None, "t", "t").unwrap();
    let (b, _, info) = World::load_bytes(&bytes).unwrap();
    assert_eq!(info.format_version, crate::save::FORMAT_VERSION);
    // (`after_load` recomputes the season's figures, so compare loaded worlds with each other)
    let (c, _, _) = World::load_bytes(a.save_json(None, "t", "t").unwrap().as_bytes()).unwrap();
    assert_eq!(c.save_json(None, "t", "t").unwrap(), b.save_json(None, "t", "t").unwrap());
    assert_eq!(c.save_bytes(None, "t", "t").unwrap(), b.save_bytes(None, "t", "t").unwrap());
    let (b2, _, _) = World::load_bytes(&b.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(b.save_json(None, "t", "t").unwrap(), b2.save_json(None, "t", "t").unwrap());
    // a map that differs from the generated one is still stored exactly
    let mut d = b.clone();
    d.map.k_plants[5] += 0.25;
    d.map.biome[7] ^= 3;
    let (e, _, _) = World::load_bytes(&d.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(e.map.k_plants[5], d.map.k_plants[5]);
    assert_eq!(e.map.biome[7], d.map.biome[7]);
    assert_eq!(serde_json::to_string(&d.map).unwrap(), serde_json::to_string(&e.map).unwrap());
}

/// A v9 save (made by the v9 build) loads unchanged; checksums were computed from that build's
/// JSON export of the fixture.
#[test]
fn v9_fixture_loads_exactly() {
    let bytes = include_bytes!("../tests/fixtures/v9_seed9_y150_64x40.civsave");
    let (w, _, info) = World::load_bytes(bytes).expect("v9 save loads");
    assert_eq!(info.format_version, 9);
    assert_eq!(w.state_hash(), 0x1845e2690330d59);
    assert_eq!((w.tick, w.bands.len()), (600, 606));
    assert_eq!(w.bands.iter().map(|b| b.known.len()).sum::<usize>(), 17622);
    let mut s: u64 = 0;
    for b in &w.bands {
        for k in &b.known {
            s = s.wrapping_mul(31).wrapping_add(k.skill.to_bits() as u64 + k.quality.to_bits() as u64 + k.from as u64 + k.since as u64);
        }
        for p in &b.partners {
            s = s.wrapping_mul(31).wrapping_add(p.strength.to_bits() as u64 + p.married.to_bits() as u64 + p.id as u64);
        }
        // (v11 inserts the neutral pitch leaning, v12 the steel one, v13 the gold one, v14 the
        // resinous-wood one; the v9
        // leanings must be unchanged around them)
        for (i, c) in b.craft.iter().enumerate() {
            if i != Mat::Pitch as usize && i != Mat::Steel as usize && i != Mat::Gold as usize && i != Mat::ResinWood as usize {
                s = s.wrapping_mul(31).wrapping_add(c.to_bits() as u64);
            }
        }
    }
    assert_eq!(s, 0x4a026ad8e2339cbe);
    assert_eq!(w.cell_first.values().map(|c| c.iter().count()).sum::<usize>(), 24715);
    // re-saved as v10 it is the same world
    let (w2, _, _) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(w.save_json(None, "t", "t").unwrap(), w2.save_json(None, "t", "t").unwrap());
}

/// The `counters!` registry: every serialized field is in `NAMES` (same order), `values` /
/// `from_values` round-trip, and `add` sums every `sum` counter (so none silently stays 0).
#[test]
fn counter_registry_covers_every_field() {
    use crate::world::Counters;
    let names: Vec<String> = match serde_json::to_value(Counters::default()).unwrap() {
        serde_json::Value::Object(m) => m.keys().cloned().collect(),
        _ => unreachable!(),
    };
    let mut sorted: Vec<String> = Counters::NAMES.iter().map(|s| s.to_string()).collect();
    sorted.sort();
    let mut n2 = names.clone();
    n2.sort();
    assert_eq!(sorted, n2, "Counters::NAMES must list every field");
    let v: Vec<u64> = (1..=Counters::NAMES.len() as u64).collect();
    let c = Counters::from_values(&v);
    assert_eq!(c.values(), v);
    let mut a = Counters::default();
    a.add(&c);
    a.add(&c);
    for (i, (n, x)) in Counters::NAMES.iter().zip(a.values()).enumerate() {
        let want = if matches!(*n, "max_fight_deaths" | "longest_leg") { v[i] } else { 2 * v[i] };
        assert_eq!(x, want, "counter {n} not summed by Counters::add");
    }
    assert_eq!(a.since(&c).values().iter().zip(Counters::NAMES).filter(|(_, n)| !matches!(**n, "max_fight_deaths" | "longest_leg")).map(|(x, _)| *x).collect::<Vec<_>>(),
        v.iter().zip(Counters::NAMES).filter(|(_, n)| !matches!(**n, "max_fight_deaths" | "longest_leg")).map(|(x, _)| *x).collect::<Vec<_>>());
}

/// The event log keeps each class in its own ring, and round-trips through both encodings.
#[test]
fn event_log_retention_and_round_trip() {
    use crate::world::{Event, EventKind};
    use crate::materials::NONE;
    use crate::archive::{EventLog, CLASS_CONFLICT, CLASS_LIFE, CLASS_LORE, RING_CAPS};
    let mut l = EventLog::default();
    let ev = |tick: u32, kind: EventKind, band: u32| Event { tick, kind: kind as u8, band, other: band + 3, cell: 7, item: NONE, recipe: u32::MAX };
    l.push(ev(0, EventKind::Split, 1));
    l.push(ev(0, EventKind::Fight, 2));
    for t in 0..(RING_CAPS[CLASS_LORE] as u32 + 500) {
        l.push(ev(t, EventKind::Learned, t));
    }
    // a flood of lore never pushes out older fights or splits
    assert_eq!(l.ring(CLASS_LORE).len(), RING_CAPS[CLASS_LORE]);
    assert_eq!(l.ring(CLASS_LIFE).len(), 1);
    assert_eq!(l.ring(CLASS_CONFLICT).len(), 1);
    assert_eq!(l.reach()[CLASS_LORE], Some(500));
    let r = l.recent(3, |_| true);
    assert_eq!(r.iter().map(|e| e.tick).collect::<Vec<_>>(), vec![RING_CAPS[CLASS_LORE] as u32 + 497, RING_CAPS[CLASS_LORE] as u32 + 498, RING_CAPS[CLASS_LORE] as u32 + 499]);
    assert_eq!(l.recent(5, |e| e.kind == EventKind::Fight as u8).len(), 1);
    let all = l.all();
    let blob = crate::codec::encode_events(&all);
    assert_eq!(crate::codec::decode_events(&blob).unwrap(), all);
    let j: EventLog = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
    assert_eq!(j.all(), all);
}

/// The history store: a frame a year, folded into decades once older than `RECENT_YEARS`;
/// event counts in the frames add up to what was logged; the genealogy holds every founding.
#[test]
fn archive_frames_and_folding() {
    use crate::world::{Counters, EventKind};
    use crate::archive::{Archive, Frame, DECADE, RECENT_YEARS};
    let mut a = Archive::default();
    let mut c = Counters::default();
    for y in 1..=257u32 {
        c.splits += 2;
        a.close_year(y, y * 4, &c, Frame::default());
    }
    // years 1-150 in 15 decades (whole decades older than 257-100 = 157), 151-257 yearly
    let dec = a.frames.iter().filter(|f| f.span == DECADE).count();
    let yearly = a.frames.iter().filter(|f| f.span == 1).count();
    assert_eq!((dec, yearly), (15, 107), "{:?}", a.frames.iter().map(|f| (f.year, f.span)).collect::<Vec<_>>());
    assert!(yearly as u32 >= RECENT_YEARS);
    assert_eq!(a.frames.iter().map(|f| f.span).sum::<u32>(), 257);
    assert_eq!(a.frames.iter().map(|f| f.counters.splits).sum::<u64>(), 2 * 257);
    assert!(a.frames.windows(2).all(|p| p[0].year < p[1].year));

    let mut w = small();
    let founders = w.bands.len();
    for _ in 0..160 {
        w.step();
    }
    let fr = &w.archive.frames;
    assert_eq!(fr.len() as usize, w.history.len());
    let logged: u64 = fr.iter().map(|f| f.kinds[EventKind::Split as usize] as u64).sum::<u64>() + w.archive.tally.kinds.get(EventKind::Split as usize).copied().unwrap_or(0) as u64;
    assert_eq!(logged, w.counters.splits as u64);
    let founded = w.archive.genealogy.iter().filter(|e| e.kind == EventKind::Founded as u8).count();
    assert!(founded >= founders, "{founded} < {founders}");
    assert_eq!(fr.last().unwrap().lineages.iter().map(|r| r.2).sum::<u32>(), w.history.last().unwrap().pop);
}

/// The structured outputs (data schema v1): one metrics.csv row per closed year with as many
/// fields as the header, deltas matching the archive frames, and parseable event records.
#[test]
fn data_outputs_shape() {
    let mut w = World::new(Config { seed: 5, map_w: 80, map_h: 50, ..Default::default() }, Params::default());
    let mut deltas = Vec::new();
    for _ in 0..40 {
        w.step();
        if w.history.len() > deltas.len() {
            deltas.push(w.archive.frames.last().unwrap().counters.clone());
        }
    }
    assert_eq!(deltas.len(), 10);
    let csv = crate::data::metrics_csv(&w.history, 0, &deltas);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 11);
    let n = lines[0].split(',').count();
    assert!(lines.iter().all(|l| l.split(',').count() == n), "ragged csv");
    assert!(lines[0].contains(",c_experiments,") && lines[0].ends_with(",c_max_fight_deaths,c_longest_leg"));
    let mut total = crate::world::Counters::default();
    for d in &deltas {
        total.add(d);
    }
    assert_eq!(total, w.counters, "the yearly deltas add up to the run totals");
    for e in w.events.all().iter().take(50) {
        let r = crate::data::event_record(&w, e);
        assert!(r["kind"].is_string() && r["text"].is_string());
        serde_json::from_str::<serde_json::Value>(&r.to_string()).unwrap();
    }
}

/// (inventions step 0) A v10 save written by the v10 build before the inventions work loads,
/// gains the crossing-ledger fields at their defaults, and runs on. Step 0 ran it on exactly as
/// the v10 build did; since step 1 (pitch, v11) the extra material shifts the craft-leaning
/// normalisation, since step 2 provisions and endurance change the sea toll and spoilage, and
/// since step 3 pilotage learned on long legs turns crews toward signs of land, and since step 7
/// (steel, v12) one more material again, so the checksum after 10 more years is this build's own.
#[test]
fn v10_fixture_loads_and_runs() {
    let bytes = include_bytes!("../tests/fixtures/v10_seed9_y150_64x40.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v10 save loads");
    assert_eq!(info.format_version, 10);
    assert_eq!(info.migrated_from, Some(10));
    // (inventions step 1, v11) pitch joins the materials: every per-material vector grows, the
    // new craft leaning going in neutral before the action leanings
    for b in &w.bands {
        assert_eq!((b.stock.len(), b.origin.len(), b.craft.len()), (NMAT, NMAT, crate::band::CRAFT_LEN));
        assert_eq!(b.craft[Mat::Pitch as usize], 1.0);
        assert_eq!(b.stock[Mat::Pitch as usize], 0.0);
        assert_eq!(b.pilot, 0.0); // (inventions step 3) an additive field: older rows load at 0
        assert_eq!(b.sky, 0.0); // (inventions step 4) likewise
        assert_eq!((b.records, b.owed), (0.0, 0.0)); // (inventions step 8) likewise
    }
    assert_eq!((w.tick, w.bands.len()), (600, 606));
    assert_eq!(w.state_hash(), FIXTURE_V10_HASH.0);
    assert_eq!(w.leg_record, 0.0);
    assert!(w.landfalls.iter().all(|r| r.leg == 0.0 && r.tech == 0));
    for _ in 0..40 {
        w.step();
    }
    assert_eq!(w.state_hash(), FIXTURE_V10_HASH.1);
}
const FIXTURE_V10_HASH: (u64, u64) = (0x1845e2690330d59, 0x56415fec651c4be8); // run-on changed by step 8 (records, no partner cap), step 8b (close ties), step 10 (feasts, prestige from giving), step 10b (feast scale, ornaments seen), step 11 (gold: one more material), step 11b (moulds and crucibles; gold is cast), step 12 (recipe realism) and step 12b (experiments reach for tar pot, forge hammer, mast and sail) step 13 (resinous wood drawn like wood, pitch daubed on a pot) and the diffusion step (D1 hide and bone at hand for hunters, D2 worn ornaments practised, D3 kit by payoff with trials, D4 teaching what can be used)

/// (inventions step 0) The crossing ledger: a band that came ashore on another landmass over
/// more open water than anyone before is a major event and sets the record; a shorter one does
/// not; and the tech bits read back as names.
#[test]
fn longest_crossing_sets_the_record() {
    use crate::world::EventKind;
    let mut w = World::new(Config { seed: 3, map_w: 120, map_h: 80, ..Default::default() }, Params::default());
    let m = &w.map;
    let home = w.bands[0].cell(m.w);
    let lh = m.landmass_of(home);
    let other = (0..m.w * m.h).find(|&c| m.is_land(c) && m.landmass_of(c) != lh && m.landmass_of(c) != 0).expect("a second landmass");
    let n0 = w.chronicle.iter().filter(|e| e.kind == EventKind::LongestCrossing as u8).count();
    let arrive = |w: &mut World, leg: f32| {
        let mw = w.map.w;
        let b = &mut w.bands[0];
        b.x = (other % mw) as u16;
        b.y = (other / mw) as u16;
        b.d.sailed_from = Some(home as u32);
        b.d.sailed_sea = leg;
        b.d.sailed_tech = crate::world::TECH_OUTRIGGER | crate::world::TECH_PROVISIONED;
        w.track_leg_record();
    };
    arrive(&mut w, 7.0);
    assert_eq!(w.leg_record, 7.0);
    arrive(&mut w, 5.0);
    assert_eq!(w.leg_record, 7.0);
    let n1 = w.chronicle.iter().filter(|e| e.kind == EventKind::LongestCrossing as u8).count();
    assert_eq!(n1, n0 + 1);
    assert_eq!(crate::world::tech_names(crate::world::TECH_OUTRIGGER | crate::world::TECH_ADRIFT), vec!["outrigger", "adrift"]);
    // the new event kind survives the u8 round trip used by the event columns
    assert_eq!(EventKind::from_u8(EventKind::LongestCrossing as u8) as u8, EventKind::LongestCrossing as u8);
}

/// (inventions step 1) Pitch: wood heated in a closed fired pot over any fire gives pitch; a
/// fired pot sealed with it is a jar. Neither works with an unfired or woven pot, and a kit with
/// them caulks, glues and keeps food better.
#[test]
fn pitch_and_sealed_jars() {
    let ctx = Ctx { dryness: 0.5, water: false };
    // (inventions step 12) pitch is sweated only from resinous wood (pine, birch)
    let wood = item(Form::Raw, Mat::ResinWood);
    let pitch = item(Form::Raw, Mat::Pitch);
    let pot = item(Form::Vessel, Mat::Ceramic);
    let fire = item(Form::Fire, Mat::Wood);
    assert_eq!(physics::apply(Action::Heat, &physics::canonical(Action::Heat, [item(Form::Raw, Mat::Wood), pot, fire, NONE]), &ctx).map(|o| o.out), None);
    let heat = |inp| physics::apply(Action::Heat, &physics::canonical(Action::Heat, inp), &ctx).map(|o| o.out);
    let comb = |inp| physics::apply(Action::Combine, &physics::canonical(Action::Combine, inp), &ctx).map(|o| o.out);
    assert_eq!(heat([wood, pot, fire, NONE]), Some(pitch));
    assert_eq!(heat([pot, wood, fire, NONE]), Some(pitch));
    assert_ne!(heat([wood, item(Form::Vessel, Mat::Clay), fire, NONE]), Some(pitch));
    assert_ne!(heat([wood, item(Form::Vessel, Mat::Fibre), fire, NONE]), Some(pitch));
    assert_eq!(comb([pot, pitch, NONE, NONE]), Some(item(Form::Jar, Mat::Ceramic)));
    assert_eq!(comb([item(Form::Vessel, Mat::Clay), pitch, NONE, NONE]), None);
    assert_eq!(item_name(item(Form::Jar, Mat::Ceramic)), "sealed jar");
    assert_eq!(item_name(pitch), "pitch");
    // pitch is never a fuel: feeding it to a fire does nothing
    assert_ne!(comb([fire, pitch, NONE, NONE]).map(form), Some(Form::Fire));
    let mut s = ItemSet::default();
    s.add(pot);
    s.add(pitch);
    s.add(item(Form::Jar, Mat::Ceramic));
    let k = crate::band::make_kit(&s);
    assert_eq!((k.pitch, k.jar), (pitch, item(Form::Jar, Mat::Ceramic)));
    assert!(k.used_items().contains(&k.jar) && k.used_items().contains(&k.pitch));
}

/// (inventions step 1) Jars come only after pottery: in a short run no band has a jar without
/// also having fired pots and pitch.
#[test]
fn jars_only_after_pottery() {
    let mut w = small();
    for _ in 0..4 * 120 {
        w.step();
    }
    for b in &w.bands {
        if b.d.kit.jar != NONE {
            assert!(b.d.tk.avail.has(item(Form::Vessel, Mat::Ceramic)) && b.d.tk.avail.has(item(Form::Raw, Mat::Pitch)));
        }
    }
}

/// (inventions step 2) Provisions and endurance: continuous provisioning matches the old
/// all-or-nothing rule at its ends, legs within the base endurance are untouched, food that
/// keeps lengthens endurance only as far as the water lasts, and jars carry water.
#[test]
fn provisions_and_endurance() {
    use crate::world::*;
    // the old rule's two ends
    let old = |prepared: bool, q: f32, xp: f32, sea: f32| (-0.02 * sea * if prepared { 0.7 } else { 1.0 } * (1.4f32 - q - 0.25 * xp).max(0.3).powi(2)).exp();
    for sea in [2.0, 9.0, 18.0, 30.0] {
        assert!((World::sea_survival_x(0.6, 0.3, 1.0, sea) - old(true, 0.6, 0.3, sea)).abs() < 1e-6);
        assert!((World::sea_survival_x(0.6, 0.3, 0.0, sea) - old(false, 0.6, 0.3, sea)).abs() < 1e-6);
    }
    // no extra toll within the base endurance; steep past it
    for sea in [1.0, 10.0, ENDURANCE_BASE] {
        assert_eq!(World::endurance_factor(sea, ENDURANCE_BASE), 1.0);
    }
    assert!(World::endurance_factor(ENDURANCE_BASE + 8.0, ENDURANCE_BASE) < 0.2);

    let mut w = small();
    let mut b = w.bands[0].clone();
    let c = b.cell(w.map.w);
    // a mild, wet place: no drying racks, no cellar
    w.map.rain[c] = 0.8;
    w.map.temp[c] = 18.0;
    w.map.tamp[c] = 4.0;
    b.pop = 40;
    b.store = 0.0;
    b.d.kit = crate::band::Kit::default();
    b.d.farm_share = 0.0;
    b.tame = 0.0;
    for s in b.stock.iter_mut() {
        *s = 0.0;
    }
    assert!(!w.dries_food(&b, c) && !w.cold_cellar(&b, c));
    assert_eq!(w.provisions(&b, c), (0.0, ENDURANCE_BASE));
    // food put by but none of it keeps: no longer
    b.store = 40.0;
    assert_eq!(w.provisions(&b, c), (1.0, ENDURANCE_BASE));
    // half provisioned is half fed
    b.store = 10.0;
    assert!((w.provisions(&b, c).0 - 0.5).abs() < 1e-6);
    // salted food: longer, but only as far as skins of water last
    b.store = 40.0;
    b.stock[Mat::Salt as usize] = 1.0;
    assert_eq!(w.provisions(&b, c).1, ENDURANCE_BASE + WATER_SKINS);
    // sealed jars carry water: the salted food now counts in full
    b.d.kit.jar = item(Form::Jar, Mat::Ceramic);
    assert_eq!(w.provisions(&b, c).1, ENDURANCE_BASE + ENDURANCE_FOOD.min(WATER_JARS));
    // a plank boat has room for it all, store or not
    b.store = 0.0;
    b.d.kit.plank = item(Form::Plank, Mat::Wood);
    let (fed, e) = w.provisions(&b, c);
    assert_eq!(fed, 1.0);
    assert_eq!(e, ENDURANCE_BASE + ENDURANCE_FOOD.min(WATER_JARS) + ENDURANCE_PLANK);

    // drying racks need an edge and cord, and dry or freezing air
    b.d.kit.cut = item(Form::Edge, Mat::Flint);
    b.d.kit.cord = item(Form::Cord, Mat::Fibre);
    assert!(!w.dries_food(&b, c));
    w.map.rain[c] = 0.2;
    assert!(w.dries_food(&b, c));
    w.map.rain[c] = 0.8;
    w.map.temp[c] = -2.0;
    w.map.tamp[c] = 10.0;
    assert!(w.dries_food(&b, c));
    // cold cellars: settled, a digging stick, hard winters
    b.settled = 0.8;
    assert!(!w.cold_cellar(&b, c));
    b.d.kit.dig = item(Form::Shaft, Mat::Wood);
    assert!(w.cold_cellar(&b, c));
    w.map.temp[c] = 10.0;
    assert!(!w.cold_cellar(&b, c));
}

/// (inventions step 3) Pilotage: signs of land turn a crew toward an island it would have
/// sailed past, without adding range; it is learned, fades, and keeps word of a shore longer.
#[test]
fn pilotage_signs_and_routes() {
    use crate::world::*;
    let mut w = small();
    let (mw, mh) = (w.map.w, w.map.h);
    // a long coast on the west, open sea, and a small island 15 cells out
    for y in 0..mh {
        for x in 0..mw {
            let i = y * mw + x;
            let island = (24..27).contains(&x) && (24..27).contains(&y);
            w.map.height[i] = if x < 10 || island { 0.2 } else { -0.5 };
        }
    }
    w.map.label_landmasses();
    let signs = w.sea_signs();
    assert_eq!(signs.dist[25 * mw + 25], 0);
    assert_eq!(signs.dist[25 * mw + 30], 4);
    assert_eq!(w.map.landmass_of(signs.near[25 * mw + 30] as usize), w.map.landmass_of(25 * mw + 25));
    assert_eq!(signs.dist[2 * mw + 60], 255); // far from any land

    let mut b = w.bands[0].clone();
    (b.x, b.y) = (9, 25);
    b.ambition = 0.0;
    b.seacraft = 1.0;
    b.d.kit = crate::band::Kit::default();
    b.d.kit.boat = item(Form::Boat, Mat::Wood);
    b.d.kit.boat_q = 1.0;
    let range = World::boat_range(&b.d.kit, b.seacraft);
    let island = w.map.landmass_of(25 * mw + 25);
    let trial = |pilot: f32| {
        let mut bb = b.clone();
        bb.pilot = pilot;
        let mut rng = crate::rng::Rng::new(11);
        let (mut found, mut steered) = (0, 0);
        for _ in 0..4000 {
            if let Some((c, sea, s)) = w.voyage_x(&bb, &mut rng, None) {
                assert!(sea <= range + 1e-3, "signs add no range");
                if w.map.landmass_of(c) == island {
                    found += 1;
                }
                steered += s as u32;
            }
        }
        (found, steered)
    };
    let (f0, s0) = trial(0.0);
    let (f1, s1) = trial(1.0);
    assert_eq!(s0, 0, "no pilotage, no signs");
    assert!(s1 > 0 && f1 > 2 * f0.max(1), "pilots find the island more often: {f0} vs {f1}");

    // learned toward 1, never past it; fades unused; keeps a far shore in mind longer
    let mut p = b.clone();
    p.pilot = 0.0;
    World::learn_pilot(&mut p, PILOT_SCOUT);
    assert!((p.pilot - PILOT_SCOUT).abs() < 1e-6);
    for _ in 0..1000 {
        World::learn_pilot(&mut p, 0.5);
    }
    assert!(p.pilot <= 1.0);
    assert!((World::rumour_keep(&p) - RUMOUR_YEARS as f32 * (1.0 + ROUTE_KEEP)).abs() < 1e-3);
    p.pilot = 0.0;
    assert!((World::rumour_keep(&p) - RUMOUR_YEARS as f32).abs() < 1e-6);

    // pilotage is saved with the band (binary and JSON)
    let mut w = small();
    w.bands[0].pilot = 0.37;
    let bytes = w.save_bytes(None, "test", "now").unwrap();
    let (w2, _, _) = World::load_bytes(&bytes).unwrap();
    assert_eq!(w2.bands[0].pilot, 0.37);
    let json = w.save_json(None, "test", "now").unwrap();
    let (w3, _, _) = World::load_bytes(json.as_bytes()).unwrap();
    assert_eq!(w3.bands[0].pilot, 0.37);
}

/// (inventions step 4) Sky lore: it steadies an aimed heading only as far as the crew has
/// pilotage (sky lore alone steers nobody), finds a far island more often with no extra range,
/// sets the ledger's `sky` bit, is learned toward 1, and is saved with the band.
#[test]
fn sky_lore_heading_and_save() {
    use crate::world::*;
    let mut w = small();
    let (mw, mh) = (w.map.w, w.map.h);
    // a long coast on the west, open sea, and a small island 16 cells out
    for y in 0..mh {
        for x in 0..mw {
            let i = y * mw + x;
            let island = (25..27).contains(&x) && (24..26).contains(&y);
            w.map.height[i] = if x < 10 || island { 0.2 } else { -0.5 };
        }
    }
    w.map.label_landmasses();
    let mut b = w.bands[0].clone();
    (b.x, b.y) = (9, 10);
    b.ambition = 0.0;
    b.seacraft = 1.0;
    b.d.kit = crate::band::Kit::default();
    b.d.kit.boat = item(Form::Boat, Mat::Wood);
    b.d.kit.boat_q = 1.0;
    // heading error: sky lore without pilotage changes nothing; with it, 0.15 -> 0.06
    b.pilot = 0.0;
    b.sky = 1.0;
    assert!((World::heading_error(&b) - 0.15).abs() < 1e-6);
    assert_eq!(World::sky_at_sea(&b), 0.0);
    assert_eq!(World::voyage_tech(&b, false) & TECH_SKY, 0);
    b.pilot = SKY_PILOT;
    assert!((World::heading_error(&b) - 0.06).abs() < 1e-6);
    assert_ne!(World::voyage_tech(&b, false) & TECH_SKY, 0);
    b.seacraft = 0.5;
    assert!((World::heading_error(&b) - 0.12).abs() < 1e-6);
    b.seacraft = 1.0;

    let range = World::boat_range(&b.d.kit, b.seacraft);
    let target = 25 * mw + 25;
    let island = w.map.landmass_of(target);
    let trial = |sky: f32| {
        let mut bb = b.clone();
        bb.pilot = 0.05; // too little to read signs (half a cell), enough to show it is gated
        bb.sky = sky;
        let mut rng = crate::rng::Rng::new(5);
        let mut found = 0;
        for _ in 0..4000 {
            if let Some((c, sea, _)) = w.voyage_x(&bb, &mut rng, Some(target as u32)) {
                assert!(sea <= range + 1e-3, "sky lore adds no range");
                found += (w.map.landmass_of(c) == island) as u32;
            }
        }
        found
    };
    let (f0, f1) = (trial(0.0), trial(1.0));
    assert!(f1 > f0, "a little pilotage lets the stars help a little: {f0} vs {f1}");
    let trial_p = |sky: f32| {
        let mut bb = b.clone();
        bb.pilot = SKY_PILOT;
        bb.sky = sky;
        bb.d.kit.boat_q = 1.0;
        let mut rng = crate::rng::Rng::new(5);
        let mut found = 0;
        for _ in 0..4000 {
            if let Some((c, _, _)) = w.voyage_x(&bb, &mut rng, Some(target as u32)) {
                found += (w.map.landmass_of(c) == island) as u32;
            }
        }
        found
    };
    let (g0, g1) = (trial_p(0.0), trial_p(1.0));
    assert!(g1 as f32 > 1.3 * g0.max(1) as f32, "star paths: the island is hit far more often: {g0} vs {g1}");

    // learned toward 1, never past it
    let mut p = b.clone();
    p.sky = 0.0;
    World::learn_sky(&mut p, SKY_FARM);
    assert!((p.sky - SKY_FARM).abs() < 1e-6);
    for _ in 0..1000 {
        World::learn_sky(&mut p, 0.5);
    }
    assert!(p.sky <= 1.0);

    // saved with the band (binary and JSON)
    let mut w = small();
    w.bands[0].sky = 0.41;
    let bytes = w.save_bytes(None, "test", "now").unwrap();
    let (w2, _, _) = World::load_bytes(&bytes).unwrap();
    assert_eq!(w2.bands[0].sky, 0.41);
    let json = w.save_json(None, "test", "now").unwrap();
    let (w3, _, _) = World::load_bytes(json.as_bytes()).unwrap();
    assert_eq!(w3.bands[0].sky, 0.41);
}

/// (inventions step 4) Settled farmers build up sky lore season by season; nobody else does.
#[test]
fn farmers_learn_sky_lore() {
    let mut w = small();
    for (k, b) in w.bands.iter_mut().enumerate() {
        b.sky = 0.0;
        b.pilot = 0.0;
        (b.sow, b.settled) = if k % 2 == 0 { (1.0, 1.0) } else { (0.0, 0.0) };
    }
    for _ in 0..40 {
        w.step();
    }
    let top = w.bands.iter().map(|b| b.sky).fold(0.0f32, f32::max);
    assert!(top > 0.005 && top < 0.2, "settled farmers learn the sky, slowly: {top}");
    assert!(w.bands.iter().all(|b| (0.0..=1.0).contains(&b.sky)));
}

/// (inventions step 5) Ships: a plank hull, metal nails and pitch, put together at the water.
/// A ship is no more seaworthy than a plank boat, but it lasts longer at sea (more so when well
/// built) and, under a usable sail, goes further.
#[test]
fn ships_recipe_and_reach() {
    use crate::world::*;
    let plank = item(Form::Plank, Mat::Wood);
    let pitch = item(Form::Raw, Mat::Pitch);
    let nail = item(Form::Point, Mat::Copper);
    let ship = item(Form::Ship, Mat::Wood);
    let comb = |inp, water| {
        let ctx = Ctx { dryness: 0.5, water };
        physics::apply(Action::Combine, &physics::canonical(Action::Combine, inp), &ctx).map(|o| o.out)
    };
    assert_eq!(comb([plank, pitch, nail, NONE], true), Some(ship));
    assert_eq!(comb([nail, plank, pitch, NONE], true), Some(ship));
    assert_eq!(comb([plank, pitch, item(Form::Point, Mat::Iron), NONE], true), Some(ship));
    // not away from the water, not pegged with stone points, not without pitch
    assert_ne!(comb([plank, pitch, nail, NONE], false), Some(ship));
    assert_ne!(comb([plank, pitch, item(Form::Point, Mat::Flint), NONE], true), Some(ship));
    assert_ne!(comb([plank, item(Form::Raw, Mat::Wood), nail, NONE], true), Some(ship));
    assert_eq!(item_name(ship), "nailed plank ship");
    assert_eq!(ALL_FORMS[NFORM - 6], Form::Ship); // (step 11b: mould and crucible after it; step 12: cloth and loom)
    assert!(valid_item(ship));

    // the kit: a ship is a plank hull, no sounder
    let mut s = ItemSet::default();
    for it in [plank, pitch, nail] {
        s.add(it);
    }
    let k0 = crate::band::make_kit(&s);
    s.add(ship);
    let k1 = crate::band::make_kit(&s);
    assert_eq!(k1.ship, ship);
    assert!(k1.used_items().contains(&ship));
    assert_eq!(k0.boat_q, k1.boat_q);
    assert_eq!(World::ship_endurance(&k0), 1.0);
    let mut k = k1;
    k.ship_q = 0.0;
    assert!((World::ship_endurance(&k) - SHIP_ENDURANCE).abs() < 1e-6);
    k.ship_q = 1.0;
    assert!((World::ship_endurance(&k) - SHIP_ENDURANCE * (1.0 + SHIP_QUALITY)).abs() < 1e-6);
    // range: the same as the plank boat without a sail (or seamen to use it); x1.25 with
    assert_eq!(World::boat_range(&k0, 0.8), World::boat_range(&k1, 0.8));
    let (mut s0, mut s1) = (k0, k1);
    s0.sail = item(Form::Sail, Mat::Flax);
    s1.sail = item(Form::Sail, Mat::Flax);
    assert_eq!(World::boat_range(&s0, 0.3), World::boat_range(&s1, 0.3));
    assert!((World::boat_range(&s1, 0.8) - SHIP_RANGE * World::boat_range(&s0, 0.8)).abs() < 1e-4);

    // endurance in a band's provisions, and the ledger bit
    let mut w = small();
    let mut b = w.bands[0].clone();
    let c = b.cell(w.map.w);
    b.d.kit = k0;
    let e0 = w.provisions(&b, c).1;
    b.d.kit = k1;
    let e1 = w.provisions(&b, c).1;
    assert!((e1 - e0 * World::ship_endurance(&k1)).abs() < 1e-4);
    assert!(World::voyage_tech(&b, true) & TECH_SHIP != 0);
    assert_eq!(tech_names(TECH_SHIP), vec!["ship"]);

    // the first crossing by ship is a major event, once
    use crate::world::EventKind;
    let m = &w.map;
    let home = w.bands[0].cell(m.w);
    let lh = m.landmass_of(home);
    let other = (0..m.w * m.h).find(|&c| m.is_land(c) && m.landmass_of(c) != lh && m.landmass_of(c) != 0);
    if let Some(other) = other {
        let mw = w.map.w;
        let count = |w: &World| w.chronicle.iter().filter(|e| e.kind == EventKind::FirstShip as u8).count();
        for tech in [TECH_PLANK, TECH_SHIP, TECH_SHIP] {
            let bb = &mut w.bands[0];
            bb.x = (other % mw) as u16;
            bb.y = (other / mw) as u16;
            bb.d.sailed_from = Some(home as u32);
            bb.d.sailed_sea = 30.0;
            bb.d.sailed_tech = tech;
            bb.d.kit.ship = ship;
            w.track_leg_record();
            assert_eq!(count(&w), if tech == TECH_PLANK { 0 } else { 1 });
        }
        assert!(EventKind::FirstShip.major());
        assert_eq!(EventKind::from_u8(EventKind::FirstShip as u8) as u8, EventKind::FirstShip as u8);
    }
}

#[test]
fn expeditions_mat_sails_and_deep_water() {
    use crate::world::*;
    let plank = item(Form::Plank, Mat::Wood);
    let ship = item(Form::Ship, Mat::Wood);
    let shaft = item(Form::Shaft, Mat::Wood);
    let basket = item(Form::Vessel, Mat::Fibre);
    let mat_sail = item(Form::Sail, Mat::Fibre);
    let linen = item(Form::Sail, Mat::Flax);
    let comb = |inp, water| {
        let ctx = Ctx { dryness: 0.5, water };
        physics::apply(Action::Combine, &physics::canonical(Action::Combine, inp), &ctx).map(|o| o.out)
    };
    // (2) mat sails: plaited matting on a mast in a plank hull or a ship, by the water only
    assert_eq!(comb([plank, shaft, basket, NONE], true), Some(mat_sail));
    assert_eq!(comb([basket, ship, shaft, NONE], true), Some(mat_sail));
    assert_ne!(comb([plank, shaft, basket, NONE], false), Some(mat_sail));
    assert_ne!(comb([item(Form::Boat, Mat::Wood), shaft, basket, NONE], true), Some(mat_sail));
    assert_eq!(item_name(mat_sail), "woven mat sail");
    // a cloth sail is kept before a mat one
    let mut s = ItemSet::default();
    for it in [item(Form::Boat, Mat::Wood), mat_sail, linen] {
        s.add(it);
    }
    assert_eq!(crate::band::make_kit(&s).sail, linen);
    // a mat sail does nothing for a dugout, but drives a plank hull like cloth
    let mut s = ItemSet::default();
    for it in [item(Form::Boat, Mat::Wood), mat_sail] {
        s.add(it);
    }
    let dug = crate::band::make_kit(&s);
    assert_eq!(dug.sail, mat_sail);
    assert!(!World::sail_usable(&dug, 0.8));
    let mut bare = dug.clone();
    bare.sail = NONE;
    assert_eq!(World::boat_range(&dug, 0.8), World::boat_range(&bare, 0.8));
    s.add(plank);
    let pk = crate::band::make_kit(&s);
    assert!(World::sail_usable(&pk, 0.8));
    assert!(!World::sail_usable(&pk, 0.3));
    let mut pk_linen = pk.clone();
    pk_linen.sail = linen;
    assert_eq!(World::boat_range(&pk, 0.8), World::boat_range(&pk_linen, 0.8));
    assert!(World::voyage_tech(&{
        let mut b = small().bands[0].clone();
        b.d.kit = dug.clone();
        b.seacraft = 0.8;
        b
    }, false) & TECH_SAIL == 0);

    // (1) planned expeditions: only a ship crew with stores for all and food that keeps
    let w = small();
    let mut b = w.bands[0].clone();
    let c = b.cell(w.map.w);
    let mut k = pk.clone();
    k.ship = ship;
    k.ship_q = 0.5;
    b.d.kit = k.clone();
    b.seacraft = 0.3;
    b.ambition = 0.0;
    b.store = b.pop as f32; // a full season's food for everyone
    b.stock[Mat::Salt as usize] = 1.0; // salted, it keeps
    let e = w.provisions(&b, c).1;
    assert_eq!(w.expedition_reach(&b), Some(e));
    assert!(w.planned_reach(&b) >= w.usual_reach(&b));
    assert!((w.planned_reach(&b) - e.max(w.usual_reach(&b))).abs() < 1e-5);
    assert!(e > w.usual_reach(&b), "a stocked ship outlasts its usual reach: {e} vs {}", w.usual_reach(&b));
    let mut hungry = b.clone();
    hungry.store = 0.1 * hungry.pop as f32;
    assert_eq!(w.expedition_reach(&hungry), None);
    assert_eq!(w.planned_reach(&hungry), w.usual_reach(&hungry));
    let mut boat = b.clone();
    boat.d.kit.ship = NONE;
    assert_eq!(w.expedition_reach(&boat), None);
    assert_eq!(tech_names(TECH_EXPEDITION), vec!["expedition"]);

    // (3) deep-water seamanship: a ship leg out of sight of land teaches more than a small boat's
    let mut a = b.clone();
    a.pilot = 0.1;
    a.seacraft = 0.5;
    let mut p = a.clone();
    p.d.kit.ship = NONE;
    assert!(World::sea_lessons(&mut a, 30.0));
    assert!(!World::sea_lessons(&mut p, 30.0));
    assert!(a.seacraft > p.seacraft && a.pilot > p.pilot);
    // within sight of land, a ship is no school
    let (mut a2, mut p2) = (b.clone(), b.clone());
    p2.d.kit.ship = NONE;
    assert!(!World::sea_lessons(&mut a2, 4.0));
    World::sea_lessons(&mut p2, 4.0);
    assert_eq!((a2.seacraft, a2.pilot), (p2.seacraft, p2.pilot));
}

/// (inventions step 6) Iron in the fields: iron tools let farmers open forest and heavy wet
/// ground (raising its crop potential, slowly lost again when it is no longer farmed), and make
/// field work faster; bands without iron see and work the ground as before.
#[test]
fn iron_opens_forest_and_heavy_ground() {
    use crate::map::Biome;
    use crate::world::*;
    let mut w = small();
    let n = w.map.w * w.map.h;
    assert_eq!(w.opened.len(), n);
    assert!(w.opened.iter().all(|&o| o == 0.0));
    // forest cells gain potential once opened; open dry grassland gains nothing
    let forest = (0..n).find(|&i| w.map.biome_at(i) == Biome::TempForest && w.crop_pot[i] > 0.05).expect("a farmable forest cell");
    assert!(w.open_pot[forest] > 1.3 * w.crop_pot[forest], "{} vs {}", w.open_pot[forest], w.crop_pot[forest]);
    for i in 0..n {
        assert!(w.open_pot[i] >= w.crop_pot[i]);
        if w.map.biome_at(i) == Biome::Grassland && w.map.rain[i] < 0.6 {
            assert_eq!(w.open_pot[i], w.crop_pot[i]);
        }
    }
    // iron tools: an iron axe or an iron edge the band can make
    let mut b = w.bands[0].clone();
    assert!(!farming_iron(&b));
    for f in [Form::Axe, Form::Edge] {
        let mut raws = ItemSet::default();
        raws.add(item(f, Mat::Iron));
        b.d.tk = crate::knowledge::closure(&w.reg, &[], &raws, false);
        assert!(farming_iron(&b));
    }
    let mut bronze = ItemSet::default();
    bronze.add(item(Form::Axe, Mat::Bronze));
    let mut nb = b.clone();
    nb.d.tk = crate::knowledge::closure(&w.reg, &[], &bronze, false);
    assert!(!farming_iron(&nb));
    // potentials as seen by each band; a cell's crop cap rises as it is opened
    assert_eq!(w.band_pot(false, forest), w.crop_pot[forest]);
    assert_eq!(w.band_pot(true, forest), w.open_pot[forest]);
    w.tend[forest] = 1.0;
    let cap0 = w.crop_cap(forest);
    w.opened[forest] = 1.0;
    assert!((w.pot_at(forest) - w.open_pot[forest]).abs() < 1e-6);
    assert!(w.crop_cap(forest) > 1.3 * cap0);
    // field work goes faster with iron
    b.sow = 0.8;
    nb.sow = 0.8;
    let cells = [(b.cell(w.map.w), 1.0)];
    let fi = w.farm_terms(&b, &cells, 15.0).plough;
    let fs = w.farm_terms(&nb, &cells, 15.0).plough;
    assert!((fi / fs - farming::IRON_FIELD_WORK).abs() < 1e-5);
    // opened ground under fields stays open; left to scrub it slowly goes back
    let other = (0..n).find(|&i| i != forest && w.map.biome_at(i) == Biome::TempForest && w.crop_pot[i] > 0.05).unwrap();
    w.opened[other] = 1.0;
    w.tend[other] = 0.0;
    w.regrow_fields();
    assert_eq!(w.opened[forest], 1.0);
    assert!(w.opened[other] < 1.0 && w.opened[other] > 0.98);
    // a farming band with iron on forest ground opens it (and counts the season); without iron not
    let mut wf = small();
    let mut fb = b.clone();
    fb.x = (forest % wf.map.w) as _;
    fb.y = (forest / wf.map.w) as _;
    let ec = wf.economy(&fb, 1);
    let mut fx = crate::world::fx::Fx::default();
    wf.farm_season(&mut fb, &ec, &mut crate::rng::Rng::new(1), &mut fx);
    assert_eq!(fx.counters.iron_clearing, 1);
    assert!(fx.ops.iter().any(|o| matches!(o, crate::world::fx::Op::Toward { l: crate::world::fx::Layer::Opened, .. })));
    let mut sb = nb.clone();
    sb.x = fb.x;
    sb.y = fb.y;
    let ec = wf.economy(&sb, 1);
    let mut fx = crate::world::fx::Fx::default();
    wf.farm_season(&mut sb, &ec, &mut crate::rng::Rng::new(1), &mut fx);
    assert_eq!(fx.counters.iron_clearing, 0);
    // the layer survives a save round trip, and a save without it loads with nothing opened
    wf.opened[forest] = 0.5;
    let bytes = wf.save_bytes(None, "test", "now").unwrap();
    let back = World::load_bytes(&bytes).unwrap().0;
    assert_eq!(back.opened[forest], 0.5);
    wf.opened.clear();
    wf.ensure_farm_layers();
    assert_eq!(wf.opened.len(), n);
}

fn farming_iron(b: &crate::band::Band) -> bool {
    crate::world::farming::iron_tools(b)
}

/// (inventions step 7) Steel: wrought iron carburised and quenched in water, at a low chance per
/// try; (inventions step 13) worked under the hammer at the open charcoal hearth blown by
/// bellows, no longer soaked alone in the furnace. It is shaped like iron (a hammerstone still works it hot),
/// wins the kit's cutting and weapon slots from iron and bronze, and makes fights deadlier only
/// through the edge: weapon quality rises a little, armour (shields, bows) is unchanged.
#[test]
fn steel_recipe_kit_and_weapons() {
    let raw = |m| item(Form::Raw, m);
    let ch = item(Form::Fire, Mat::Charcoal);
    let bel = item(Form::Bellows, Mat::Hide);
    let clay = item(Form::Kiln, Mat::Clay);
    let heat = |inp, water| {
        let ctx = Ctx { dryness: 0.5, water };
        physics::apply(Action::Heat, &physics::canonical(Action::Heat, inp), &ctx)
    };
    let steel = raw(Mat::Steel);
    // (inventions step 13) steel is a smith's work: a bar of wrought iron beaten under a hard
    // hammer on the open charcoal hearth the bellows blow (the bellows in the chamber's place),
    // quenched in water
    let ham = item(Form::Hammer, Mat::Stone);
    let got = heat([raw(Mat::Iron), ham, ch, bel], true).expect("carburised under the hammer and quenched");
    assert_eq!(got.out, steel);
    assert_eq!(got.p, physics::STEEL_P);
    assert_eq!(heat([ham, raw(Mat::Iron), ch, bel], true).map(|o| o.out), Some(steel));
    // no quench away from water; no carbon (or heat) in a wood fire; no soft hammer; the bar on
    // its own in a blown furnace (the step 7 recipe) is just hot iron now
    assert_eq!(heat([raw(Mat::Iron), ham, ch, bel], false).map(|o| o.out), None);
    assert_eq!(heat([raw(Mat::Iron), ham, item(Form::Fire, Mat::Wood), bel], true).map(|o| o.out), None);
    assert_eq!(heat([raw(Mat::Iron), item(Form::Hammer, Mat::Wood), ch, bel], true).map(|o| o.out), None);
    assert_eq!(heat([raw(Mat::Iron), bel, ch, clay], true).map(|o| o.out), None);
    assert_eq!(heat([raw(Mat::Iron), NONE, ch, bel], true).map(|o| o.out), None);
    assert_eq!(heat([raw(Mat::Iron), NONE, ch, clay], true).map(|o| o.out), None);
    // in the closed forge the hammer still makes an iron knife, as before
    assert_eq!(heat([raw(Mat::Iron), ham, ch, clay], true).map(|o| o.out), Some(item(Form::Edge, Mat::Iron)));
    // the hearth does nothing but forging
    assert_eq!(heat([raw(Mat::IronOre), NONE, ch, bel], true).map(|o| o.out), None);
    assert_eq!(heat([item(Form::Bloom, Mat::Iron), ham, ch, bel], true).map(|o| o.out), None);
    assert_eq!(heat([raw(Mat::Copper), ham, ch, bel], true).map(|o| o.out), None);
    // and forges iron blades and steel there as in the closed forge
    assert_eq!(heat([item(Form::Edge, Mat::Iron), ham, ch, bel], false).map(|o| o.out), Some(item(Form::Point, Mat::Iron)));
    assert_eq!(heat([steel, ham, ch, bel], false).map(|o| o.out), Some(item(Form::Edge, Mat::Steel)));
    // the old iron recipes are untouched
    assert_eq!(heat([raw(Mat::IronOre), bel, ch, clay], true).map(|o| o.out), Some(item(Form::Bloom, Mat::Iron)));
    // (inventions step 12) forged hot with a hammerstone into a knife, drawn into a point,
    // hafted; never beaten cold
    let ctx = Ctx { dryness: 0.5, water: false };
    let edge = item(Form::Edge, Mat::Steel);
    assert_eq!(physics::apply(Action::Shape, &[steel, ham, NONE, NONE], &ctx).map(|o| o.out), None);
    assert_eq!(physics::apply(Action::Shape, &[edge, ham, NONE, NONE], &ctx).map(|o| o.out), None);
    assert_eq!(heat([steel, ham, ch, clay], false).map(|o| o.out), Some(edge));
    assert_eq!(heat([edge, ham, ch, clay], false).map(|o| o.out), Some(item(Form::Point, Mat::Steel)));
    assert_eq!(item_name(steel), "steel bar");
    assert_eq!(item_name(edge), "steel knife");
    assert!(Mat::Steel.is_metal() && valid_item(steel));
    assert_eq!(ALL_MATS[Mat::Steel as usize], Mat::Steel);
    assert_eq!(ALL_MATS[NMAT - 2], Mat::Gold); // (step 11) gold appended after steel; (step 12) resinous wood after it
    assert!(crate::world::TRADE_MATS.contains(&Mat::Steel));
    // keener than iron or bronze, about as tough
    assert!(cut_q(Mat::Steel) > cut_q(Mat::Bronze) + 0.1);
    assert!(durability(Mat::Steel) >= 0.95 * durability(Mat::Iron));
    // the kit takes the steel tools over iron and bronze ones
    let mut s = ItemSet::default();
    for m in [Mat::Iron, Mat::Bronze] {
        for f in [Form::Edge, Form::Spear, Form::Axe, Form::Point] {
            s.add(item(f, m));
        }
    }
    s.add(item(Form::Shield, Mat::Hide));
    let k0 = crate::band::make_kit(&s);
    for f in [Form::Edge, Form::Spear, Form::Axe, Form::Point] {
        s.add(item(f, Mat::Steel));
    }
    let k1 = crate::band::make_kit(&s);
    assert_eq!((mat(k1.hunt), mat(k1.cut), mat(k1.axe)), (Mat::Steel, Mat::Steel, Mat::Steel));
    // deadlier only through the edge: losses inflicted scale with (0.6 + weapon quality), and
    // steel raises that well under the 1.3x limit; armour is unchanged
    let (q0, q1) = (crate::band::weapon_q(&k0), crate::band::weapon_q(&k1));
    assert!(q1 > q0);
    assert!((0.6 + q1) / (0.6 + q0) < 1.1, "steel weapons {q1} vs iron/bronze {q0}");
    assert_eq!(crate::band::loss_guard(&k0), crate::band::loss_guard(&k1));
    // the technology stage, and iron in the fields counts steel
    assert_eq!(crate::analysis::STAGES[crate::analysis::stage_of(&[edge])], "steel");
    assert_eq!(crate::analysis::STAGES[crate::analysis::stage_of(&[raw(Mat::Iron)])], "iron");
}

/// (inventions step 7, v12) A v11 save written by the step 6 build loads: every per-material
/// vector grows by one, the steel craft leaning goes in neutral before the action leanings, and
/// the rest is unchanged; by the JSON route the migration does the same; it runs on.
#[test]
fn v11_fixture_gains_steel() {
    let bytes = include_bytes!("../tests/fixtures/v11_seed9_y160_64x40.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v11 save loads");
    assert_eq!(info.format_version, 11);
    assert_eq!(info.migrated_from, Some(11));
    assert_eq!((w.tick, w.bands.len()), (640, 625));
    for b in &w.bands {
        assert_eq!((b.stock.len(), b.origin.len(), b.craft.len()), (NMAT, NMAT, crate::band::CRAFT_LEN));
        assert_eq!(b.craft[Mat::Steel as usize], 1.0);
        assert_eq!((b.stock[Mat::Steel as usize], b.origin[Mat::Steel as usize]), (0.0, u32::MAX));
    }
    // the JSON route: a v11-shaped payload (vectors one short) migrates to the same shape
    let mut v: serde_json::Value = serde_json::to_value(&w.bands[0]).unwrap();
    let craft_v12 = v["craft"].clone();
    v["stock"].as_array_mut().unwrap().pop();
    v["origin"].as_array_mut().unwrap().pop();
    v["craft"].as_array_mut().unwrap().remove(Mat::Steel as usize);
    let payload = serde_json::json!({ "world": { "bands": [v] } });
    let m = crate::save::migrate(payload, 11).unwrap();
    let b = &m["world"]["bands"][0];
    assert_eq!(b["stock"].as_array().unwrap().len(), NMAT);
    assert_eq!(b["origin"].as_array().unwrap().len(), NMAT);
    assert_eq!(b["craft"], craft_v12);
    // re-saved as v12 it is the same world, and it runs on
    let (w2, _, _) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(w.save_json(None, "t", "t").unwrap(), w2.save_json(None, "t", "t").unwrap());
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.tick, 648);
}

/// (inventions step 8) Counting marks: notches cut with a good edge into a wooden stick or a
/// bone (a tally), or a lump of soft clay pressed with a point (a token), fired hard like a pot.
/// The kit keeps the best one, a fired token above all.
#[test]
fn tallies_and_tokens() {
    let ctx = Ctx { dryness: 0.5, water: false };
    let comb = |inp| physics::apply(Action::Combine, &physics::canonical(Action::Combine, inp), &ctx).map(|o| o.out);
    let shape = |t, tool| physics::apply(Action::Shape, &[t, tool, NONE, NONE], &ctx).map(|o| o.out);
    let heat = |inp| physics::apply(Action::Heat, &physics::canonical(Action::Heat, inp), &ctx).map(|o| o.out);
    let flint = item(Form::Edge, Mat::Flint);
    let stick = item(Form::Shaft, Mat::Wood);
    let bone = item(Form::Raw, Mat::Bone);
    let tally = item(Form::Mark, Mat::Wood);
    assert_eq!(comb([flint, stick, NONE, NONE]), Some(tally));
    assert_eq!(comb([stick, flint, NONE, NONE]), Some(tally));
    assert_eq!(comb([flint, bone, NONE, NONE]), Some(item(Form::Mark, Mat::Bone)));
    // a blunt edge does not notch; a third item makes something else
    assert_ne!(comb([item(Form::Edge, Mat::Wood), stick, NONE, NONE]), Some(tally));
    assert_ne!(comb([flint, stick, item(Form::Cord, Mat::Fibre), NONE]), Some(tally));
    // a clay token, pressed with a point; fired like a pot
    let token = item(Form::Mark, Mat::Clay);
    assert_eq!(shape(item(Form::Raw, Mat::Clay), item(Form::Point, Mat::Bone)), Some(token));
    assert_ne!(shape(item(Form::Raw, Mat::Stone), item(Form::Point, Mat::Bone)), Some(item(Form::Mark, Mat::Stone)));
    let fire = item(Form::Fire, Mat::Wood);
    let pot = heat([item(Form::Vessel, Mat::Clay), NONE, fire, NONE]);
    assert_eq!(pot, Some(item(Form::Vessel, Mat::Ceramic)));
    assert_eq!(heat([token, NONE, fire, NONE]), Some(item(Form::Mark, Mat::Ceramic)));
    assert_eq!(item_name(tally), "tally stick");
    assert_eq!(item_name(item(Form::Mark, Mat::Bone)), "notched bone tally");
    assert_eq!(item_name(token), "clay token");
    assert_eq!(item_name(item(Form::Mark, Mat::Ceramic)), "fired clay token");
    assert_eq!(ALL_FORMS[NFORM - 5], Form::Mark); // (step 11b: mould and crucible after it; step 12: cloth and loom)
    assert!(valid_item(tally) && valid_item(token));
    // the kit
    let mut s = ItemSet::default();
    s.add(tally);
    let k = crate::band::make_kit(&s);
    assert_eq!(k.mark, tally);
    assert!(k.used_items().contains(&tally));
    s.add(item(Form::Mark, Mat::Ceramic));
    assert_eq!(crate::band::make_kit(&s).mark, item(Form::Mark, Mat::Ceramic));
    assert_eq!(crate::band::make_kit(&ItemSet::default()).mark, NONE);
}

/// (inventions step 8) Records: a band keeps count only with something to count (a big store,
/// three or more real partners, a herd, tribute); only then is its mark used (and so kept),
/// and records cut spoilage. Trade ties have no fixed cap any more.
#[test]
fn records_need_something_to_count() {
    use crate::world::*;
    let w = small();
    let mut b = w.bands[0].clone();
    b.store = 0.0;
    b.herd = 0.0;
    b.owed = 0.0;
    b.partners.clear();
    b.d.need = 2.0;
    assert!(!World::keeps_count(&b));
    let mut t = b.clone();
    t.store = 2.0 * RECORDS_STORE;
    assert!(World::keeps_count(&t));
    let mut t = b.clone();
    t.herd = 0.5;
    assert!(World::keeps_count(&t));
    let mut t = b.clone();
    t.owed = 1.0;
    assert!(World::keeps_count(&t));
    let mut t = b.clone();
    for k in 0..RECORDS_PARTNERS as u32 {
        t.bond(1000 + k, 0.6 * RECORDS_PARTNER_TIE, 0.0, 0);
    }
    assert!(!World::keeps_count(&t), "weak ties are not worth counting");
    for k in 0..RECORDS_PARTNERS as u32 {
        t.bond(1000 + k, 0.6 * RECORDS_PARTNER_TIE, 0.0, 0);
    }
    assert!(World::keeps_count(&t));
    // no partner cap
    for k in 0..20u32 {
        t.bond(2000 + k, 0.1, 0.0, 0);
    }
    assert_eq!(t.partners.len(), RECORDS_PARTNERS + 20);

    // the mark counts as used (kept up) only with something to count
    let tally = item(Form::Mark, Mat::Wood);
    b.d.kit.mark = tally;
    let s = 1;
    assert!(!w.economy(&b, s).used.contains(&tally));
    let mut t = b.clone();
    t.owed = 1.0;
    assert!(w.economy(&t, s).used.contains(&tally));
    // records cut spoilage
    let mut r = t.clone();
    r.store = 3.0;
    let e0 = w.economy(&r, s);
    r.records = 1.0;
    let e1 = w.economy(&r, s);
    assert!(e1.spoil <= e0.spoil);
    if e0.spoil > 0.0 {
        assert!(e1.spoil < e0.spoil);
    }
}

/// (inventions step 8) Records fade without a mark, the first band to keep them is chronicled,
/// and they are saved with the band (as is tribute owed).
#[test]
fn records_fade_event_and_save() {
    use crate::world::*;
    let mut w = small();
    for b in &mut w.bands {
        b.records = 0.6;
    }
    for _ in 0..4 {
        w.step();
    }
    for b in &w.bands {
        assert!((0.0..=1.0).contains(&b.records));
        if b.d.kit.mark == NONE {
            assert!(b.records <= 0.6 * RECORDS_FADE.powi(4) + 1e-5, "no mark, records fade: {}", b.records);
        }
    }
    assert_eq!(w.chronicle.iter().filter(|e| e.kind == EventKind::FirstRecords as u8).count(), 1);
    assert!(EventKind::FirstRecords.major());
    assert_eq!(EventKind::from_u8(EventKind::FirstRecords as u8) as u8, EventKind::FirstRecords as u8);
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.chronicle.iter().filter(|e| e.kind == EventKind::FirstRecords as u8).count(), 1, "only the first");
    let mut w = small();
    w.bands[0].records = 0.37;
    w.bands[0].owed = 0.5;
    let (w2, _, _) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!((w2.bands[0].records, w2.bands[0].owed), (0.37, 0.5));
    let (w3, _, _) = World::load_bytes(w.save_json(None, "t", "t").unwrap().as_bytes()).unwrap();
    assert_eq!((w3.bands[0].records, w3.bands[0].owed), (0.37, 0.5));
    // old saves: records and owed load as 0
    let (w4, _, _) = World::load_bytes(include_bytes!("../tests/fixtures/v11_seed9_y160_64x40.civsave")).unwrap();
    assert!(w4.bands.iter().all(|b| b.records == 0.0 && b.owed == 0.0));
    let s = crate::analysis::summarize(&w2);
    assert_eq!(s.records.marks_used, w2.counters.marks_used);
}

/// (inventions step 8b) Care for ties goes mostly to the ties that already mean most: a band
/// with few ties keeps them all up; one with many keeps its strongest (trade, marriage or kin)
/// and lets loose ones fade towards `LOOSE_KEEP`; records widen the care. No cap.
#[test]
fn close_ties_get_the_care() {
    use crate::world::*;
    let w = small();
    let mut b = w.bands[0].clone();
    b.records = 0.0;
    b.partners.clear();
    // a couple of ties: both kept up in full
    b.bond(1, 0.6, 0.0, 0);
    b.bond(2, 0.2, 0.0, 0);
    let mut t = b.clone();
    t.fade_ties(&[0.0, 0.0]);
    assert!((t.partners[0].strength - 0.6 * 0.8).abs() < 1e-5);
    assert!((t.partners[1].strength - 0.2 * 0.8).abs() < 1e-5);
    // two strong ties among thirty loose ones: the strong are kept up far better
    let mut m = b.clone();
    m.partners.clear();
    m.bond(1, 0.9, 0.0, 0);
    m.bond(2, 0.8, 0.0, 0);
    for k in 0..30u32 {
        m.bond(100 + k, 0.1, 0.0, 0);
    }
    let before: Vec<f32> = m.partners.iter().map(|p| p.strength).collect();
    let kin = vec![0.0; m.partners.len()];
    let mut t = m.clone();
    t.fade_ties(&kin);
    let keep: Vec<f32> = t.partners.iter().zip(&before).map(|(p, s)| p.strength / s).collect();
    assert!(keep[0] > 0.75 && keep[1] > 0.75, "strong ties kept up: {keep:?}");
    assert!(keep[2] < 0.5 && keep[2] >= LOOSE_KEEP - 1e-5, "loose ties fade: {}", keep[2]);
    assert!(t.partners[0].strength >= CLOSE_TIE);
    assert_eq!(t.partners.len(), 32, "nothing is cut, no cap");
    // kinship and marriage earn care too
    let mut kin2 = kin.clone();
    kin2[5] = 0.5;
    let mut t2 = m.clone();
    t2.partners[6].married = 0.5;
    t2.fade_ties(&kin2);
    assert!(t2.partners[5].strength > t.partners[5].strength);
    assert!(t2.partners[6].strength > t.partners[6].strength);
    // records widen the care, so more is kept up
    let mut r = m.clone();
    r.records = 1.0;
    r.fade_ties(&kin);
    assert!(r.partners[2].strength > t.partners[2].strength);
    assert!(r.partners[0].strength >= t.partners[0].strength);
    // close ties are counted every year
    let mut w = small();
    for _ in 0..8 {
        w.step();
    }
    let h = w.history.last().unwrap();
    let n = w.bands.iter().map(|b| b.partners.iter().filter(|p| p.strength >= CLOSE_TIE).count()).collect::<Vec<_>>();
    assert_eq!(h.close_ties_max as usize, n.iter().copied().max().unwrap_or(0));
    assert!((h.close_ties_mean - n.iter().sum::<usize>() as f32 / w.bands.len().max(1) as f32).abs() < 1e-4);
    let s = crate::analysis::summarize(&w);
    assert_eq!(s.records.close_ties_max, h.close_ties_max);
}

/// (inventions step 9) Charts: only bands with records and pilotage both at `CHART_MIN` and a
/// boat keep one; a chart keeps a route past its fade (renewed) where plain word is forgotten;
/// a chart is taught further and more readily, as fresh directions; `FirstChart` fires once and
/// YearStats / summary count chart bands.
#[test]
fn charts_keep_and_teach_routes() {
    use crate::world::*;
    let mut w = small();
    w.tick = 400 * 4;
    let mw = w.map.w;
    let mut b = w.bands[0].clone();
    (b.x, b.y) = (10, 10);
    b.d.kit = crate::band::Kit::default();
    b.d.kit.boat = item(Form::Boat, Mat::Wood);
    b.d.kit.boat_q = 1.0;
    b.seacraft = 0.5;
    (b.records, b.pilot) = (CHART_MIN, CHART_MIN);
    assert!(World::charts(&b));
    let mut c = b.clone();
    c.records = CHART_MIN - 0.01;
    assert!(!World::charts(&c), "records too low");
    c = b.clone();
    c.pilot = CHART_MIN - 0.01;
    assert!(!World::charts(&c), "pilotage too low");
    c = b.clone();
    c.d.kit.boat_q = 0.0;
    assert!(!World::charts(&c), "no boat to use it in");
    (b.records, b.pilot) = (0.5, 0.5); // clear of the line (pilotage fades a little each season)

    // an old route: forgotten from plain memory, kept (and renewed) on a chart
    b.far_shore = Some((10 * mw + 30) as u32);
    b.far_shore_year = 100; // 300 years old: past even a full pilot's 150
    let mut fx = crate::world::fx::Fx::default();
    let mut rng = crate::rng::Rng::new(3);
    let mut plain = b.clone();
    plain.records = 0.0;
    w.sea_season(&mut plain, &mut rng, &mut fx);
    assert_eq!(plain.far_shore, None);
    assert_eq!((fx.counters.routes_faded, fx.counters.routes_kept), (1, 0));
    let mut charted = b.clone();
    w.sea_season(&mut charted, &mut rng, &mut fx);
    assert_eq!(charted.far_shore, b.far_shore);
    assert_eq!(charted.far_shore_year, 400);
    assert_eq!((fx.counters.routes_faded, fx.counters.routes_kept), (1, 1));

    // teaching: a hearer with a boat and no far shore; the shore is just beyond what plain
    // word reaches, but within a chart's reach
    let mut h = b.clone();
    h.far_shore = None;
    h.records = 0.0;
    h.pilot = 0.0;
    let reach = 2.0 * World::boat_range(&h.d.kit, h.seacraft) * (1.0 + PILOT_TELL * 0.5);
    let far = (reach * 1.2).ceil() as usize;
    assert!((far as f32) < reach * CHART_TELL && 10 + far < mw);
    let trial = |src: &crate::band::Band, at: usize| {
        let mut rng = crate::rng::Rng::new(9);
        let mut fx = crate::world::fx::Fx::default();
        let mut got = 0;
        for _ in 0..400 {
            let (mut s, mut d) = (src.clone(), h.clone());
            s.far_shore = Some((10 * mw + 10 + at) as u32);
            s.far_shore_year = 390;
            w.exchange(&mut crate::world::fx::Pair(&mut s, &mut d), 0, 1, &mut rng, &mut fx);
            if d.far_shore.is_some() {
                got += 1;
                assert_eq!(d.far_shore_year, if World::charts(&s) { 400 } else { 390 });
            }
        }
        (got, fx.counters.routes_taught)
    };
    let mut teller = b.clone();
    teller.records = 0.0;
    let (near_plain, t0) = trial(&teller, 3);
    let (near_chart, t1) = trial(&b, 3);
    let (far_plain, _) = trial(&teller, far);
    let (far_chart, _) = trial(&b, far);
    assert_eq!(t0, 0);
    assert_eq!(t1, near_chart as u64);
    assert!(near_chart as f32 > 1.5 * near_plain as f32, "charts are taught more readily: {near_plain} vs {near_chart}");
    assert_eq!(far_plain, 0, "plain word does not carry that far");
    assert!(far_chart > 0, "a chart does");

    // FirstChart once; YearStats and summary
    let mut w = small();
    for _ in 0..4 {
        w.step();
    }
    let k = w.bands.len().min(3);
    for b in w.bands.iter_mut().take(k) {
        b.d.kit.boat = item(Form::Boat, Mat::Wood);
        b.d.kit.boat_q = 1.0;
        (b.records, b.pilot) = (0.9, 0.9);
    }
    // (the kit is worked out afresh each season, so close the year straight away)
    w.year_stats();
    let n = |w: &World| w.chronicle.iter().filter(|e| e.kind == EventKind::FirstChart as u8).count();
    assert_eq!(n(&w), 1);
    assert!(EventKind::FirstChart.major());
    assert_eq!(EventKind::from_u8(EventKind::FirstChart as u8), EventKind::FirstChart);
    let last = w.history.last().unwrap();
    assert_eq!(last.chart_bands, w.bands.iter().filter(|b| World::charts(b)).count() as u32);
    let s = crate::analysis::summarize(&w);
    assert!(s.charts.first_chart_year.is_some());
    assert_eq!(last.chart_bands as usize, k);
    assert_eq!(s.charts.chart_bands, last.chart_bands);
    w.year_stats();
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(n(&w), 1, "fires once");
}

#[test]
fn ale_is_brewed_from_grain_in_a_jar_at_the_water() {
    let jar = item(Form::Jar, Mat::Ceramic);
    let inp = physics::canonical(Action::Combine, [item(Form::Raw, Mat::Grain), jar, NONE, NONE]);
    let wet = Ctx { dryness: 0.3, water: true };
    let dry = Ctx { dryness: 0.3, water: false };
    let ale = item(Form::Jar, Mat::Grain);
    assert_eq!(physics::apply(Action::Combine, &inp, &wet).map(|o| o.out), Some(ale));
    assert!(physics::apply(Action::Combine, &inp, &dry).map(|o| o.out != ale).unwrap_or(true), "no water to steep it in");
    // an ale jar is not another jar to brew in
    let inp = physics::canonical(Action::Combine, [item(Form::Raw, Mat::Grain), ale, NONE, NONE]);
    assert!(physics::apply(Action::Combine, &inp, &wet).map(|o| o.out != ale).unwrap_or(true));
    assert_eq!(item_name(ale), "ale");
    let mut s = ItemSet::default();
    s.add(ale);
    s.add(jar);
    let k = crate::band::make_kit(&s);
    assert_eq!(k.ale, ale);
    assert!(k.used_items().contains(&ale));
}

#[test]
fn feasts_give_food_and_earn_standing_by_the_gift() {
    use crate::world::fx::{Fx, Op, Pair};
    use crate::world::*;
    assert!((pearson(&[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 8.0]) - 1.0).abs() < 1e-5);
    assert!((pearson(&[1.0, 2.0, 3.0, 4.0], &[8.0, 6.0, 4.0, 2.0]) + 1.0).abs() < 1e-5);
    assert_eq!(pearson(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]), 0.0);
    assert!((give_prestige(16.0, 1.0) - 16.0 * PRESTIGE_GIVE).abs() < 1e-6);
    assert!((give_prestige(16.0, ALE_DRAW) - 2.0 * give_prestige(16.0, 1.0)).abs() < 1e-6);

    let mut w = small();
    for _ in 0..8 {
        w.step();
    }
    let mut h = w.bands[0].clone();
    let mut g = w.bands[1 % w.bands.len()].clone();
    g.id = h.id + 100_000;
    h.partners.clear();
    g.partners.clear();
    (h.prestige, g.prestige) = (0.0, 0.0);
    h.d.kit.ale = NONE;
    (h.d.food_ratio, h.d.need) = (1.2, 50.0);
    (g.d.food_ratio, g.d.need) = (1.0, 400.0);
    h.store = 150.0; // 150 - FEAST_KEEP x 50 to spare
    g.store = 0.0;
    let run = |h: &crate::band::Band, g: &crate::band::Band| {
        let (mut h, mut g) = (h.clone(), g.clone());
        let mut fx = Fx::default();
        w.feast(&mut Pair(&mut g, &mut h), 0, 1, &mut fx);
        (h, g, fx)
    };
    let (h1, g1, fx) = run(&h, &g);
    let give = FEAST_SHARE * (150.0 - FEAST_KEEP * 50.0);
    assert!((h1.store - (150.0 - give)).abs() < 1e-3 && (g1.store - give).abs() < 1e-3, "the host gives a share of its surplus");
    assert!((h1.prestige - give_prestige(give, 1.0)).abs() < 1e-6 && g1.prestige == 0.0, "standing goes to the giver");
    assert!(h1.partners.iter().any(|p| p.id == g.id) && g1.partners.iter().any(|p| p.id == h.id));
    assert_eq!((fx.counters.feasts, fx.counters.ale_feasts, fx.acc.feasts), (1, 0, 1));
    assert!(matches!(fx.ops.last(), Some(Op::Feast { ale: false, .. })));
    // a richer host gives more and stands higher, in proportion to what it gives
    let mut rich = h.clone();
    rich.store = 450.0;
    let (r1, rg, _) = run(&rich, &g);
    assert!((r1.prestige / h1.prestige - rg.store / give).abs() < 1e-3 && rg.store > give);
    // (step 10b) a feast is at most a season's food for a crowd the size of the bigger band:
    // here the guest band (need 400) ...
    let mut glut = h.clone();
    glut.store = 50.0 + 10_000.0;
    let (_, g2, _) = run(&glut, &g);
    assert!((g2.store - FEAST_EAT * 400.0).abs() < 1e-3);
    // ... and a big host (need 2000) feeds a crowd of its own size, more than the guest band
    let mut big = glut.clone();
    big.d.need = 2000.0;
    big.store = 2000.0 + 20_000.0;
    let (b2, g3, _) = run(&big, &g);
    assert!((g3.store - FEAST_EAT * 2000.0).abs() < 1e-2 && (b2.prestige - give_prestige(2000.0, 1.0)).abs() < 1e-4);
    assert_eq!(feast_cap(30.0, 10.0), FEAST_EAT * 30.0);
    // (step 10b) ornament standing grows with the square root of band size
    let o = |n: f32| ornament_prestige(0.5, n);
    assert!((o(ORNAMENT_SEEN) - PRESTIGE_FINE).abs() < 1e-6 && (o(4.0 * ORNAMENT_SEEN) - 2.0 * o(ORNAMENT_SEEN)).abs() < 1e-6);
    assert_eq!(o(0.0), 0.0);
    // no feast from a band short of food, or with nothing to spare
    let mut lean = h.clone();
    lean.d.food_ratio = 0.9;
    assert_eq!(run(&lean, &g).2.counters.feasts, 0);
    let mut bare = h.clone();
    bare.store = FEAST_KEEP * 50.0 + FEAST_MIN; // its share of what is spare is under FEAST_MIN
    assert_eq!(run(&bare, &g).2.counters.feasts, 0);
    // with ale (if this band's country is warm enough to brew), the feast draws twice the crowd
    let c = h.cell(w.map.w);
    if w.ale_warm(c) {
        let mut br = h.clone();
        br.d.kit.ale = item(Form::Jar, Mat::Grain);
        br.sow = 1.0;
        br.stock[Mat::Grain as usize] = 1.0;
        assert!(w.brews(&br));
        let (b1, _, fx) = run(&br, &g);
        assert!((b1.prestige - give_prestige(give, ALE_DRAW)).abs() < 1e-6);
        assert_eq!(fx.counters.ale_feasts, 1);
    }

    // FirstFeast once, through the effects; YearStats and the summary
    let mut fx = Fx::default();
    w.feast(&mut Pair(&mut h.clone(), &mut g.clone()), 0, 1, &mut fx);
    w.feast(&mut Pair(&mut h.clone(), &mut g.clone()), 0, 1, &mut fx);
    w.apply_fx(fx, None);
    let n = |w: &World| w.chronicle.iter().filter(|e| e.kind == EventKind::FirstFeast as u8).count();
    assert_eq!(n(&w), 1);
    // (step 10b) once the first feast is known, feasts send no effect of their own
    let mut fx = Fx::default();
    w.feast(&mut Pair(&mut h.clone(), &mut g.clone()), 0, 1, &mut fx);
    assert!(fx.ops.is_empty() && fx.counters.feasts == 1);
    assert!(EventKind::FirstFeast.major() && !EventKind::Feast.major());
    assert_eq!(EventKind::from_u8(EventKind::Feast as u8), EventKind::Feast);
    assert_eq!(w.counters.feasts, 2);
    w.year_stats();
    let y = w.history.last().unwrap();
    assert_eq!(y.feasts, 2);
    assert!((y.feast_food - 2.0 * give).abs() < 1e-3);
    assert!(y.prestige_size_r.abs() <= 1.0 && y.prestige_top_pct >= 0.0 && y.prestige_top_pct <= 100.0);
    // the history store tallies the year's feasts
    let kinds = &w.archive.frames.last().unwrap().kinds;
    assert_eq!(kinds[EventKind::Feast as usize], 2);
    let s = crate::analysis::summarize(&w);
    assert_eq!(s.feasts.feasts, 2);
    assert!(s.feasts.first_feast_year.is_some());
}



/// FNV-1a over a map's layers with the gold bit cleared (and the generator version left out),
/// so a later generator that only adds gold hashes the same as generator 4.
fn map_hash_sans_gold(m: &crate::map::Map) -> u64 {
    let mut m = m.clone();
    for d in &mut m.deposits {
        *d &= 0x7f;
    }
    m.gen_version = 0;
    let v = serde_json::to_vec(&m).unwrap();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in v {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// (inventions step 11) Gold comes from its own random stream: with the gold bits masked off,
/// generator-5 maps are byte for byte the generator-4 maps (hashes recorded with the step 10b
/// build), the map without gold is exactly that, and gold does turn up.
#[test]
fn gold_leaves_every_other_map_layer_identical() {
    use crate::map::{Map, D_GOLD};
    let mut with_gold = 0;
    for (w, h, s, want) in [(256usize, 160usize, 1u64, 0x7f4b796ad04695d4u64), (256, 160, 7, 0xd467f3d67d821f1e), (200, 120, 499246292, 0xe6adb9cd4629db3)] {
        let m = Map::generate(w, h, s);
        assert_eq!(map_hash_sans_gold(&m), want, "{w}x{h} seed {s}: a layer other than gold changed");
        let old = Map::generate_layers(w, h, s, false);
        assert_eq!(old.gen_version, 4);
        assert!(old.deposits.iter().all(|&d| d & D_GOLD == 0));
        assert_eq!(map_hash_sans_gold(&old), want);
        let cells = m.deposits.iter().filter(|&&d| d & D_GOLD != 0).count();
        // rare: a handful of veins and the gravels below them
        assert!(cells < w * h / 200, "{cells} gold cells");
        // placer gold lies only on rivers; vein gold only in the hills
        for c in (0..w * h).filter(|&c| m.deposits[c] & D_GOLD != 0) {
            assert!(m.is_land(c) && (m.river[c] || m.height[c] > 300.0));
        }
        with_gold += (cells > 0) as u32;
    }
    assert!(with_gold >= 2, "gold turns up on most maps");
}

/// (inventions step 11) Native gold makes an ornament, never a tool (step 11b: cast, never
/// beaten cold); the kit wears gold before anything else, and gold standing counts above the work.
#[test]
fn gold_ornaments_are_prized() {
    let ctx = Ctx { dryness: 0.5, water: false };
    let shape = |t, tool| physics::apply(Action::Shape, &[t, tool, NONE, NONE], &ctx).map(|o| o.out);
    let nugget = item(Form::Raw, Mat::Gold);
    let ring = item(Form::Ornament, Mat::Gold);
    // (step 11b) no ring from a nugget and a hammerstone any more, or from any hammer
    assert_eq!(shape(nugget, item(Form::Hammer, Mat::Stone)), None);
    assert_eq!(shape(nugget, item(Form::Hammer, Mat::Copper)), None);
    assert_eq!(shape(nugget, NONE), None);
    assert_eq!(shape(nugget, item(Form::Edge, Mat::Flint)), None);
    assert!(!Mat::Gold.is_metal(), "gold is no metal for tools or fields");
    assert_eq!(Mat::Gold.props().prized, 1.0);
    assert!(ALL_MATS.iter().filter(|&&m| m != Mat::Gold).all(|m| m.props().prized == 0.0));
    let mut s = ItemSet::default();
    s.add(item(Form::Ornament, Mat::Tin));
    s.add(item(Form::Ornament, Mat::Bone));
    s.add(ring);
    assert_eq!(crate::band::make_kit(&s).ornament, ring);
    let p = |fine: f32| crate::world::feasting::ornament_prestige(fine, 25.0);
    assert!(p(0.5 + Mat::Gold.props().prized) > 1.6 * p(0.5));
}

/// (inventions step 11b) A gold ring needs the whole of metalworking: a clay mould cut with an
/// edge and fired hard, gold charged into a fired pot (a crucible), and the closed charcoal
/// furnace that smelts copper, hot enough to melt gold. Without any one of them, no ring.
#[test]
fn gold_ring_needs_smelting_crucible_and_mould() {
    let ctx = Ctx { dryness: 0.5, water: false };
    let shape = |t, tool| physics::apply(Action::Shape, &[t, tool, NONE, NONE], &ctx).map(|o| o.out);
    let comb = |a: Item, b: Item| {
        let mut v = [a, b, NONE, NONE];
        v[..2].sort();
        physics::apply(Action::Combine, &v, &ctx).map(|o| o.out)
    };
    let heat = |a: Item, b: Item, fire: Item, kiln: Item| {
        let (a, b) = if b != NONE && b < a { (b, a) } else { (a, b) };
        physics::apply(Action::Heat, &[a, b, fire, kiln], &ctx).map(|o| o.out)
    };
    let nugget = item(Form::Raw, Mat::Gold);
    let ring = item(Form::Ornament, Mat::Gold);
    let clay = item(Form::Raw, Mat::Clay);
    let (wet_mould, mould) = (item(Form::Mould, Mat::Clay), item(Form::Mould, Mat::Ceramic));
    let crucible = item(Form::Crucible, Mat::Gold);
    let pot = item(Form::Vessel, Mat::Ceramic);
    let (charcoal, wood_fire) = (item(Form::Fire, Mat::Charcoal), item(Form::Fire, Mat::Wood));
    let (clay_kiln, stone_kiln) = (item(Form::Kiln, Mat::Clay), item(Form::Kiln, Mat::Stone));
    // the mould: a hollow cut in soft clay with a sharp edge, then fired like a pot
    assert_eq!(shape(clay, item(Form::Edge, Mat::Flint)), Some(wet_mould));
    assert_eq!(shape(clay, NONE), Some(item(Form::Vessel, Mat::Clay)), "bare hands still make a pot");
    assert_eq!(shape(clay, item(Form::Point, Mat::Bone)), Some(item(Form::Mark, Mat::Clay)), "a point still makes a token");
    assert_eq!(heat(wet_mould, NONE, wood_fire, NONE), Some(mould));
    // the crucible: gold in a fired pot (not an unfired one, not a basket)
    assert_eq!(comb(nugget, pot), Some(crucible));
    assert_eq!(comb(nugget, item(Form::Vessel, Mat::Clay)), None);
    assert_eq!(comb(nugget, item(Form::Vessel, Mat::Fibre)), None);
    // (inventions step 12) copper, tin and bronze melt in it too (bronze tools are cast), but
    // not iron, which no crucible fire here melts
    assert_eq!(comb(item(Form::Raw, Mat::Copper), pot), Some(item(Form::Crucible, Mat::Copper)));
    assert_eq!(comb(item(Form::Raw, Mat::Iron), pot), None);
    // the cast: crucible + fired mould in a closed charcoal furnace
    assert_eq!(heat(crucible, mould, charcoal, clay_kiln), Some(ring));
    assert_eq!(heat(crucible, mould, charcoal, stone_kiln), Some(ring));
    assert_eq!(heat(crucible, mould, charcoal, NONE), None, "an open fire does not melt gold");
    assert_eq!(heat(crucible, mould, wood_fire, clay_kiln), None, "a wood-fired kiln is not hot enough");
    assert_eq!(heat(crucible, wet_mould, charcoal, clay_kiln), None, "an unfired mould cracks");
    assert_eq!(heat(crucible, NONE, charcoal, clay_kiln), None, "molten gold needs a mould");
    assert_eq!(heat(crucible, pot, charcoal, clay_kiln), None);
    assert_eq!(heat(nugget, mould, charcoal, clay_kiln), None, "a bare nugget needs a crucible");
    assert_eq!(heat(nugget, NONE, charcoal, clay_kiln), None);
    // the furnace that melts gold is the one that smelts copper
    assert_eq!(heat(item(Form::Raw, Mat::CopperOre), NONE, charcoal, clay_kiln), Some(item(Form::Raw, Mat::Copper)));
    assert_eq!(heat(item(Form::Raw, Mat::CopperOre), NONE, wood_fire, clay_kiln), None);
    assert_eq!((item_name(wet_mould), item_name(mould), item_name(crucible)), ("clay mould".into(), "fired clay mould".into(), "crucible of gold".into()));
    assert_eq!((ALL_FORMS[NFORM - 4], ALL_FORMS[NFORM - 3]), (Form::Mould, Form::Crucible)); // (step 12: cloth and loom after them)
    assert!(NFORM <= 32, "ItemSet has room for 32 forms");
    assert!(valid_item(mould) && valid_item(crucible));
    // through the toolkit: a band that knows the whole chain has a ring where gold lies, and
    // none without the charcoal furnace
    use crate::knowledge::{closure, Known, Recipe, Registry};
    let mut reg = Registry::default();
    let mut known = vec![];
    let mut learn = |reg: &mut Registry, a: Action, inp: [Item; 4]| {
        let out = physics::apply(a, &inp, &ctx).expect("recipe works").out;
        let id = reg.insert(Recipe { action: a as u8, inputs: inp, output: out, ..Default::default() });
        known.push(Known { recipe: id, skill: 0.5, ..Default::default() });
    };
    learn(&mut reg, Action::Shape, [clay, item(Form::Edge, Mat::Flint), NONE, NONE]);
    learn(&mut reg, Action::Heat, [wet_mould, NONE, wood_fire, NONE]);
    let mut cv = [nugget, pot, NONE, NONE];
    cv[..2].sort();
    learn(&mut reg, Action::Combine, cv);
    let mut hv = [crucible, mould, charcoal, clay_kiln];
    hv[..2].sort();
    learn(&mut reg, Action::Heat, hv);
    let mut raws = ItemSet::default();
    for i in [nugget, clay, item(Form::Edge, Mat::Flint), pot, wood_fire, clay_kiln] {
        raws.add(i);
    }
    assert!(!closure(&reg, &known, &raws, false).producible.has(ring), "no charcoal fire, no ring");
    raws.add(charcoal);
    assert!(closure(&reg, &known, &raws, false).producible.has(ring));
    let mut raws = ItemSet::default();
    for i in [clay, item(Form::Edge, Mat::Flint), pot, wood_fire, clay_kiln, charcoal] {
        raws.add(i);
    }
    assert!(!closure(&reg, &known, &raws, false).producible.has(ring), "no gold, no ring");
}

/// (inventions step 11b) A v13 save from before step 11b, where bands learned to hammer gold
/// cold, still loads with no format change: those bands keep the recipe and their rings, but
/// it is retired, so it is never taught to anyone else.
#[test]
fn legacy_cold_gold_rings_are_kept_not_taught() {
    use crate::knowledge::{closure, Known, Recipe};
    let bytes = include_bytes!("../tests/fixtures/v12_seed9_y170_64x40.civsave");
    let (mut w, _, _) = World::load_bytes(bytes).expect("fixture loads");
    let (nugget, hammer) = (item(Form::Raw, Mat::Gold), item(Form::Hammer, Mat::Stone));
    let ring = item(Form::Ornament, Mat::Gold);
    let old = [nugget, hammer, NONE, NONE];
    assert!(physics::apply(Action::Shape, &old, &Ctx { dryness: 0.5, water: false }).is_none());
    let pre = w.reg.recipes.len() as u32;
    assert!((0..pre).all(|r| !w.reg.retired(r)), "nothing else is retired");
    let rid = w.reg.insert(Recipe { action: Action::Shape as u8, inputs: old, output: ring, p: 0.4, ..Default::default() });
    assert!(w.reg.retired(rid));
    w.bands[0].known.push(Known { recipe: rid, skill: 0.9, ..Default::default() });
    let (w2, _, info) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(info.format_version, crate::save::FORMAT_VERSION);
    assert!(w2.reg.retired(rid) && w2.bands[0].knows(rid), "kept, and still retired after a reload");
    assert!((0..pre).all(|r| !w2.reg.retired(r)));
    let mut raws = ItemSet::default();
    raws.add(nugget);
    raws.add(hammer);
    let kn = w2.bands[0].known.clone();
    assert!(closure(&w2.reg, &kn, &raws, false).producible.has(ring), "the old ring stays in its kit");
}

/// (inventions step 11, v13) A v12 save written by the step 10b build loads: every per-material
/// vector grows by one, the gold craft leaning goes in neutral before the action leanings, the
/// map stays generator 4 with no gold, and by the JSON route the migration does the same.
#[test]
fn v12_fixture_gains_gold_vectors_but_no_gold() {
    let bytes = include_bytes!("../tests/fixtures/v12_seed9_y170_64x40.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v12 save loads");
    assert_eq!(info.format_version, 12);
    assert_eq!(info.migrated_from, Some(12));
    assert_eq!((w.tick, w.bands.len()), (680, 678));
    assert_eq!(w.map.gen_version, 4);
    assert!(w.map.deposits.iter().all(|&d| d & crate::map::D_GOLD == 0), "old maps get no gold");
    for b in &w.bands {
        assert_eq!((b.stock.len(), b.origin.len(), b.craft.len()), (NMAT, NMAT, crate::band::CRAFT_LEN));
        assert_eq!(b.craft[Mat::Gold as usize], 1.0);
        assert_eq!((b.stock[Mat::Gold as usize], b.origin[Mat::Gold as usize]), (0.0, u32::MAX));
    }
    let mut v: serde_json::Value = serde_json::to_value(&w.bands[0]).unwrap();
    let craft_v13 = v["craft"].clone();
    v["stock"].as_array_mut().unwrap().pop();
    v["origin"].as_array_mut().unwrap().pop();
    v["craft"].as_array_mut().unwrap().remove(Mat::Gold as usize);
    let payload = serde_json::json!({ "world": { "bands": [v] } });
    let m = crate::save::migrate(payload, 12).unwrap();
    let b = &m["world"]["bands"][0];
    assert_eq!(b["stock"].as_array().unwrap().len(), NMAT);
    assert_eq!(b["origin"].as_array().unwrap().len(), NMAT);
    assert_eq!(b["craft"], craft_v13);
    // re-saved as v13 (the map against the generator-4 base) it is the same world; it runs on
    let (w2, _, _) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(w.save_json(None, "t", "t").unwrap(), w2.save_json(None, "t", "t").unwrap());
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.tick, 688);
    assert!(w.map.deposits.iter().all(|&d| d & crate::map::D_GOLD == 0));
}

#[test]
#[ignore]
fn print_map_hashes() {
    for (w, h, s) in [(256usize, 160usize, 1u64), (256, 160, 7), (200, 120, 499246292)] {
        let m = crate::map::Map::generate(w, h, s);
        println!("MAPHASH {w}x{h} seed {s}: {:#x}", map_hash_sans_gold(&m));
    }
}

/// (inventions step 12) Recipe realism: each impossible path gives nothing, each real one works.
#[test]
fn recipe_realism_rules() {
    let dry = Ctx { dryness: 0.5, water: false };
    let wet = Ctx { dryness: 0.5, water: true };
    let at = |a: Action, inp: [Item; 4], c: &Ctx| physics::apply(a, &physics::canonical(a, inp), c).map(|o| o.out);
    let shape = |t: Item, tool: Item| at(Action::Shape, [t, tool, NONE, NONE], &dry);
    let comb = |inp: [Item; 4]| at(Action::Combine, inp, &wet);
    let comb_dry = |inp: [Item; 4]| at(Action::Combine, inp, &dry);
    let heat = |inp: [Item; 4]| at(Action::Heat, inp, &dry);
    let (raw, edge, point) = (|m| item(Form::Raw, m), |m| item(Form::Edge, m), |m| item(Form::Point, m));
    let flint = edge(Mat::Flint);
    let ham = item(Form::Hammer, Mat::Stone);
    let stone = raw(Mat::Stone);
    let cord = item(Form::Cord, Mat::Fibre);
    let wshaft = item(Form::Shaft, Mat::Wood);
    let (charcoal, wood_fire) = (item(Form::Fire, Mat::Charcoal), item(Form::Fire, Mat::Wood));
    let (clay_kiln, stone_kiln) = (item(Form::Kiln, Mat::Clay), item(Form::Kiln, Mat::Stone));
    let pot = item(Form::Vessel, Mat::Ceramic);
    let mould = item(Form::Mould, Mat::Ceramic);

    // 1. only woody material makes a shaft; only a wooden shaft makes a spear
    assert_eq!(shape(raw(Mat::Wood), flint), Some(wshaft));
    for m in [Mat::Wool, Mat::Flax, Mat::Fibre, Mat::Hide] {
        assert_ne!(shape(raw(m), flint).map(form), Some(Form::Shaft), "{m:?} carved");
        let s = item(Form::Shaft, m);
        assert_eq!(shape(s, flint), None, "{m:?} shaft sharpened");
        assert_eq!(comb_dry([s, point(Mat::Flint), cord, NONE]), None, "{m:?} shaft hafted");
        assert_eq!(comb_dry([s, flint, cord, NONE]), None);
    }
    assert_eq!(shape(wshaft, flint), Some(item(Form::Spear, Mat::Wood)));
    assert_eq!(comb_dry([wshaft, point(Mat::Flint), cord, NONE]), Some(item(Form::Spear, Mat::Flint)));
    assert_eq!(comb_dry([wshaft, flint, cord, NONE]), Some(item(Form::Axe, Mat::Flint)));

    // 2. copper is beaten cold; bronze is cast; iron and steel are forged hot
    assert_eq!(shape(raw(Mat::Copper), ham), Some(edge(Mat::Copper)));
    assert_eq!(shape(edge(Mat::Copper), ham), Some(point(Mat::Copper)));
    for m in [Mat::Tin, Mat::Bronze, Mat::Iron, Mat::Steel, Mat::Gold] {
        assert_eq!(shape(raw(m), ham), None, "{m:?} cold-hammered");
        assert_eq!(shape(edge(m), ham), None, "{m:?} edge cold-drawn");
    }
    let bronze_cr = item(Form::Crucible, Mat::Bronze);
    assert_eq!(comb_dry([raw(Mat::Bronze), pot, NONE, NONE]), Some(bronze_cr));
    assert_eq!(heat([bronze_cr, mould, charcoal, clay_kiln]), Some(edge(Mat::Bronze)));
    assert_eq!(heat([bronze_cr, mould, wood_fire, clay_kiln]), None, "a wood fire does not melt bronze");
    assert_eq!(heat([bronze_cr, mould, charcoal, NONE]), None, "nor an open fire");
    assert_eq!(heat([bronze_cr, item(Form::Mould, Mat::Clay), charcoal, clay_kiln]), None, "an unfired mould");
    assert_eq!(shape(edge(Mat::Bronze), stone), Some(point(Mat::Bronze)), "ground on a grindstone");
    assert_eq!(shape(edge(Mat::Bronze), raw(Mat::Clay)), None);
    for m in [Mat::Iron, Mat::Steel] {
        assert_eq!(heat([raw(m), ham, charcoal, clay_kiln]), Some(edge(m)), "{m:?} forged");
        assert_eq!(heat([edge(m), ham, charcoal, clay_kiln]), Some(point(m)));
        assert_eq!(heat([raw(m), ham, charcoal, NONE]), None, "{m:?}: a bare charcoal fire is not forging heat");
        assert_eq!(heat([raw(m), ham, wood_fire, clay_kiln]), None);
        assert_eq!(heat([raw(m), item(Form::Hammer, Mat::Bone), charcoal, clay_kiln]), None);
        assert_eq!(comb_dry([raw(m), pot, NONE, NONE]), None, "{m:?} is not melted");
    }
    // 3. tin makes no edge, point, needle or nail
    let tin_cr = item(Form::Crucible, Mat::Tin);
    assert_eq!(comb_dry([raw(Mat::Tin), pot, NONE, NONE]), Some(tin_cr));
    assert_eq!(heat([tin_cr, mould, charcoal, clay_kiln]), None);
    assert_eq!(shape(edge(Mat::Tin), stone), None);
    let tin_pt = point(Mat::Tin);
    assert_eq!(comb_dry([wshaft, tin_pt, cord, NONE]), None, "no tin-tipped spear");
    assert_eq!(comb_dry([wshaft, edge(Mat::Tin), cord, NONE]), None, "no tin axe");
    assert_eq!(shape(cord, tin_pt), None, "no netting with a tin needle");
    assert_eq!(shape(raw(Mat::Clay), tin_pt), None, "no token pressed with tin");
    assert_eq!(comb_dry([raw(Mat::Hide), cord, tin_pt, NONE]), None, "no sewing with tin");
    let plank = item(Form::Plank, Mat::Wood);
    let pitch = raw(Mat::Pitch);
    assert_eq!(comb([plank, pitch, tin_pt, NONE]), None, "no tin nails");
    assert_eq!(comb([plank, pitch, point(Mat::Bone), NONE]), None, "nails are metal");
    assert_eq!(comb([plank, pitch, point(Mat::Copper), NONE]), Some(item(Form::Ship, Mat::Wood)));
    assert_eq!(comb([plank, pitch, point(Mat::Bronze), NONE]), Some(item(Form::Ship, Mat::Wood)));
    assert_eq!(shape(cord, point(Mat::Bone)), Some(item(Form::Net, Mat::Fibre)));
    assert_eq!(shape(cord, wshaft), Some(item(Form::Net, Mat::Fibre)));
    assert_eq!(comb_dry([raw(Mat::Hide), cord, point(Mat::Bone), NONE]), Some(item(Form::Garment, Mat::Hide)));

    // 4. bronze from the metals: melted together in a crucible at copper-melting heat
    let cu_cr = item(Form::Crucible, Mat::Copper);
    assert_eq!(comb_dry([raw(Mat::Copper), pot, NONE, NONE]), Some(cu_cr));
    assert_eq!(heat([cu_cr, raw(Mat::Tin), charcoal, stone_kiln]), Some(raw(Mat::Bronze)));
    assert_eq!(heat([tin_cr, raw(Mat::Copper), charcoal, clay_kiln]), Some(raw(Mat::Bronze)));
    assert_eq!(heat([cu_cr, raw(Mat::Tin), wood_fire, clay_kiln]), None, "not in a wood fire");
    assert_eq!(heat([cu_cr, raw(Mat::Tin), charcoal, NONE]), None, "not in an open fire");
    assert_eq!(heat([raw(Mat::Copper), raw(Mat::Tin), charcoal, clay_kiln]), None, "the bare metals in a kiln");
    assert_eq!(heat([raw(Mat::Copper), raw(Mat::Tin), wood_fire, clay_kiln]), None);
    assert_eq!(heat([cu_cr, mould, charcoal, stone_kiln]), Some(edge(Mat::Copper)), "copper casts too");
    // the ore route is unchanged
    assert_eq!(heat([raw(Mat::CopperOre), raw(Mat::TinOre), charcoal, clay_kiln]), Some(raw(Mat::Bronze)));
    assert_eq!(heat([raw(Mat::Copper), raw(Mat::TinOre), charcoal, clay_kiln]), Some(raw(Mat::Bronze)));

    // 5. sails come from cloth, on a plank hull or a ship
    let linen = item(Form::Cloth, Mat::Flax);
    let ship = item(Form::Ship, Mat::Wood);
    let boat = item(Form::Boat, Mat::Wood);
    assert_eq!(comb([plank, wshaft, linen, NONE]), Some(item(Form::Sail, Mat::Flax)));
    assert_eq!(comb([ship, wshaft, item(Form::Cloth, Mat::Wool), NONE]), Some(item(Form::Sail, Mat::Wool)));
    assert_eq!(comb([boat, wshaft, linen, NONE]), None, "no mast in a dugout");
    assert_eq!(comb([item(Form::Boat, Mat::Hide), wshaft, linen, NONE]), None);
    assert_eq!(comb([plank, wshaft, item(Form::Garment, Mat::Flax), NONE]), None, "clothing is no sail");
    assert_eq!(comb([plank, wshaft, item(Form::Garment, Mat::Hide), NONE]), None);
    let mut s = ItemSet::default();
    s.add(boat);
    s.add(item(Form::Sail, Mat::Wool));
    assert!(!World::sail_usable(&crate::band::make_kit(&s), 0.9), "a cloth sail on a dugout is no use");
    s.add(plank);
    assert!(World::sail_usable(&crate::band::make_kit(&s), 0.9));

    // 6. cloth: same-fibre yarn on a loom
    let loom = item(Form::Loom, Mat::Wood);
    assert_eq!(comb_dry([wshaft, cord, stone, NONE]), Some(loom));
    assert_eq!(comb_dry([item(Form::Shaft, Mat::Wool), cord, stone, NONE]), None);
    for m in [Mat::Flax, Mat::Wool] {
        let yarn = item(Form::Cord, m);
        assert_eq!(shape(raw(m), NONE), Some(yarn), "{m:?} spun");
        assert_eq!(comb_dry([loom, yarn, NONE, NONE]), Some(item(Form::Cloth, m)));
        assert_eq!(comb_dry([item(Form::Cloth, m), cord, point(Mat::Bone), NONE]), Some(item(Form::Garment, m)));
        assert_eq!(comb_dry([raw(m), cord, point(Mat::Bone), NONE]), None, "raw {m:?} is not cloth");
    }
    assert_eq!(comb_dry([loom, item(Form::Cord, Mat::Hide), NONE, NONE]), None, "thongs are not yarn");
    assert_eq!(comb_dry([item(Form::Cloth, Mat::Flax), cord, tin_pt, NONE]), None);

    // 7. obsidian pendants are ground, not hammered
    assert_eq!(shape(point(Mat::Obsidian), ham), None);
    assert_eq!(shape(point(Mat::Obsidian), stone), Some(item(Form::Ornament, Mat::Obsidian)));
    assert_eq!(shape(point(Mat::Bone), ham), Some(item(Form::Ornament, Mat::Bone)));

    // 8. names
    assert_eq!(item_name(item(Form::Bow, Mat::Hide)), "rawhide-strung bow");
    assert_eq!(item_name(item(Form::Kiln, Mat::Ceramic)), "sherd-lined kiln");
    assert_eq!(item_name(linen), "linen cloth");
    assert_eq!(item_name(item(Form::Cloth, Mat::Wool)), "woollen cloth");
    assert_eq!(item_name(loom), "warp-weighted loom");
    assert_eq!(item_name(raw(Mat::ResinWood)), "resinous wood");
    assert_eq!((ALL_FORMS[NFORM - 2], ALL_FORMS[NFORM - 1]), (Form::Cloth, Form::Loom));
    assert_eq!(ALL_MATS[NMAT - 1], Mat::ResinWood);
    assert!(valid_item(loom) && valid_item(linen) && valid_item(raw(Mat::ResinWood)));

    // 9. pitch only from resinous wood
    assert_eq!(heat([raw(Mat::ResinWood), pot, wood_fire, NONE]), Some(pitch));
    assert_eq!(heat([raw(Mat::Wood), pot, wood_fire, NONE]), None);
    assert_eq!(heat([raw(Mat::ResinWood), item(Form::Vessel, Mat::Clay), wood_fire, NONE]), None);
    // resinous wood is no shaft, hull or fuel upgrade
    assert_ne!(shape(raw(Mat::ResinWood), flint).map(form), Some(Form::Shaft));
    assert_eq!(shape(raw(Mat::ResinWood), NONE).map(form), None);
}

/// (inventions step 12) Resinous wood grows on the cool wooded lands and hills, not in tropical
/// forest, desert or the sea; every big map has some.
#[test]
fn resin_wood_on_the_map() {
    use crate::map::Biome;
    let w = World::new(Config { seed: 3, map_w: 96, map_h: 60, ..Default::default() }, Params::default());
    let m = &w.map;
    let n = m.w * m.h;
    let cells: Vec<usize> = (0..n).filter(|&i| m.resin_wood(i)).collect();
    assert!(!cells.is_empty());
    for &i in &cells {
        assert!(m.k_wood[i] > 2.0);
        assert!(!matches!(m.biome_at(i), Biome::TropForest | Biome::Desert | Biome::Ocean | Biome::Ice | Biome::Savanna | Biome::Grassland));
    }
    let wooded = (0..n).filter(|&i| m.k_wood[i] > 2.0).count();
    assert!(cells.len() < wooded, "not every wood is resinous");
}

/// (inventions step 12, v14) A v13 save written by the step 11b build: its bands know tin knives,
/// tin points and tin-tipped spears and boil pitch from plain wood. On load the forbidden recipes
/// are retired and dropped, wood pitch is re-pointed to resinous wood, kits are re-derived, the
/// vectors grow by one, and the JSON route matches.
#[test]
fn v13_fixture_cleans_forbidden_recipes() {
    let bytes = include_bytes!("../tests/fixtures/v13_seed3_y350_96x60.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v13 save loads");
    assert_eq!(info.format_version, 13);
    assert_eq!(info.migrated_from, Some(13));
    let tin_edge = item(Form::Edge, Mat::Tin);
    let tin_pt = item(Form::Point, Mat::Tin);
    let tin_spear = item(Form::Spear, Mat::Tin);
    let find = |w: &World, out: Item| (0..w.reg.recipes.len() as u32).filter(|&r| w.reg.recipes[r as usize].output == out).collect::<Vec<_>>();
    let forbidden: Vec<u32> = [tin_edge, tin_pt, tin_spear].iter().flat_map(|&o| find(&w, o)).collect();
    assert!(!forbidden.is_empty(), "the fixture has tin blades");
    for &r in &forbidden {
        assert!(w.reg.retired(r), "recipe {r} retired");
        assert!(w.bands.iter().all(|b| !b.knows(r)), "recipe {r} dropped");
    }
    // every recipe still known is allowed (bar the step 11 cold-gold rings)
    for b in &w.bands {
        for k in &b.known {
            assert!(!w.reg.retired(k.recipe) || crate::knowledge::is_legacy_gold(&w.reg.recipes[k.recipe as usize]));
        }
        for it in b.d.tk.avail.iter() {
            assert!(it != tin_edge && it != tin_pt && it != tin_spear, "kit re-derived");
        }
        assert_eq!((b.stock.len(), b.origin.len(), b.craft.len()), (NMAT, NMAT, crate::band::CRAFT_LEN));
        assert_eq!(b.craft[Mat::ResinWood as usize], 1.0);
    }
    // wood pitch is now resinous-wood pitch, the same recipe for the same bands
    let pitch = find(&w, item(Form::Raw, Mat::Pitch));
    assert!(!pitch.is_empty());
    let mut kept = 0;
    for &r in &pitch {
        let rc = &w.reg.recipes[r as usize];
        assert!(!rc.inputs.contains(&item(Form::Raw, Mat::Wood)));
        if rc.inputs.contains(&item(Form::Raw, Mat::ResinWood)) {
            assert!(!w.reg.retired(r));
            kept += w.bands.iter().filter(|b| b.knows(r)).count();
        }
    }
    assert!(kept > 0, "bands keep their pitch know-how");
    // idempotent, and stable over re-saves (the season figures recomputed on the first load
    // come from the cleaned kits, so they settle after it: compare the second and third loads)
    let known: usize = w.bands.iter().map(|b| b.known.len()).sum();
    assert_eq!(w.clean_recipes(), (0, 0));
    let (w2, _, _) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(w2.bands.iter().map(|b| b.known.len()).sum::<usize>(), known);
    let (w3, _, _) = World::load_bytes(&w2.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(w2.save_json(None, "t", "t").unwrap(), w3.save_json(None, "t", "t").unwrap());
    // the JSON route: a v13 band row goes through v13_to_v14
    let mut v: serde_json::Value = serde_json::to_value(&w.bands[0]).unwrap();
    let craft_v14 = v["craft"].clone();
    v["stock"].as_array_mut().unwrap().pop();
    v["origin"].as_array_mut().unwrap().pop();
    v["craft"].as_array_mut().unwrap().remove(Mat::ResinWood as usize);
    let m = crate::save::migrate(serde_json::json!({ "world": { "bands": [v] } }), 13).unwrap();
    let b = &m["world"]["bands"][0];
    assert_eq!(b["stock"].as_array().unwrap().len(), NMAT);
    assert_eq!(b["origin"].as_array().unwrap().len(), NMAT);
    assert_eq!(b["craft"], craft_v14);
    // and it runs on
    for _ in 0..8 {
        w.step();
    }
}

/// (inventions step 12, diagnostic) What stands between bands and pitch, forged iron and cloth
/// sails: every 25 years, how many bands have each ingredient at hand. Run with
/// `PROBE_SEEDS=5,8 PROBE_YEARS=500 cargo test --release -p civ-core probe_step12 -- --ignored --nocapture`.
#[test]
#[ignore]
fn probe_step12() {
    let seeds: Vec<u64> = std::env::var("PROBE_SEEDS").unwrap_or("5".into()).split(',').map(|s| s.trim().parse().unwrap()).collect();
    let years: u32 = std::env::var("PROBE_YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(500);
    let (mw, mh): (usize, usize) = (std::env::var("PROBE_W").ok().and_then(|s| s.parse().ok()).unwrap_or(256), std::env::var("PROBE_H").ok().and_then(|s| s.parse().ok()).unwrap_or(160));
    for seed in seeds {
        let mut w = World::new(Config { seed, map_w: mw, map_h: mh, ..Default::default() }, Params::default());
        let m = &w.map;
        let n = m.w * m.h;
        let land = (0..n).filter(|&i| m.is_land(i)).count();
        let resin = (0..n).filter(|&i| m.resin_wood(i)).count();
        let wood = (0..n).filter(|&i| m.is_land(i) && m.k_wood[i] > 2.0).count();
        println!("seed {seed}: land {land}, wooded {wood}, resinous {resin} ({:.1}% of land, {:.1}% of wood)", 100.0 * resin as f32 / land as f32, 100.0 * resin as f32 / wood.max(1) as f32);
        println!("  year bands | resin here near3 stock | pot fire pitchrdy pitch | biome(top) | iron hamr5 kiln chfire ironrdy ironedge | plank cloth shaft water sailrdy sail");
        for t in 0..years * 4 {
            w.step();
            if w.bands.is_empty() {
                break;
            }
            if (t + 1) % 100 != 0 {
                continue;
            }
            let m = &w.map;
            let near = |c: usize, r: i64| {
                let (x, y) = ((c % m.w) as i64, (c / m.w) as i64);
                (-r..=r).any(|dy| (-r..=r).any(|dx| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= 0 && ny >= 0 && (nx as usize) < m.w && (ny as usize) < m.h && m.resin_wood(ny as usize * m.w + nx as usize)
                }))
            };
            let mut c = [0u32; 20];
            let mut biomes = std::collections::BTreeMap::<String, u32>::new();
            for b in &w.bands {
                let a = &b.d.tk.avail;
                let ci = b.cell(m.w);
                let has = |i: Item| a.has(i);
                let any = |f: &dyn Fn(Item) -> bool| a.iter().any(|i| f(i));
                let rh = has(item(Form::Raw, Mat::ResinWood));
                let pot = has(item(Form::Vessel, Mat::Ceramic));
                let fire = any(&|i| form(i) == Form::Fire);
                c[0] += 1;
                c[1] += rh as u32;
                c[2] += near(ci, 3) as u32;
                c[3] += (b.stock[Mat::ResinWood as usize] > 0.05) as u32;
                c[4] += pot as u32;
                c[5] += fire as u32;
                c[6] += (rh && pot && fire) as u32;
                c[7] += has(item(Form::Raw, Mat::Pitch)) as u32;
                *biomes.entry(format!("{:?}", m.biome_at(ci))).or_default() += 1;
                let iron = has(item(Form::Raw, Mat::Iron));
                let hm = any(&|i| form(i) == Form::Hammer && mat(i).props().hard >= 5.0);
                let kiln = any(&|i| form(i) == Form::Kiln);
                let chf = any(&|i| form(i) == Form::Fire && mat(i) == Mat::Charcoal);
                c[8] += iron as u32;
                c[9] += hm as u32;
                c[10] += kiln as u32;
                c[11] += chf as u32;
                c[12] += (iron && hm && kiln && chf) as u32;
                c[13] += has(item(Form::Edge, Mat::Iron)) as u32;
                let plank = b.d.kit.plank != NONE;
                let cloth = any(&|i| form(i) == Form::Cloth);
                let shaft = any(&|i| form(i) == Form::Shaft);
                let water = w.near_water(ci);
                c[14] += plank as u32;
                c[15] += cloth as u32;
                c[16] += shaft as u32;
                c[17] += water as u32;
                c[18] += (plank && cloth && shaft && water) as u32;
                c[19] += any(&|i| form(i) == Form::Sail) as u32;
            }
            let mut bv: Vec<_> = biomes.into_iter().collect();
            bv.sort_by(|a, b| b.1.cmp(&a.1));
            let top: Vec<String> = bv.iter().take(3).map(|(k, v)| format!("{k} {}%", 100 * v / c[0].max(1))).collect();
            println!(
                "  Y{:<4} {:>5} | {:>5} {:>5} {:>5} | {:>5} {:>5} {:>5} {:>5} | {} | {:>4} {:>5} {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>5} {:>5} {:>4}",
                (t + 1) / 4, c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], top.join(", "), c[8], c[9], c[10], c[11], c[12], c[13], c[14], c[15], c[16], c[17], c[18], c[19]
            );
        }
    }
}

/// (inventions step 13) A save from before step 13, whose bands learned steel by soaking a bar
/// alone in the blown furnace, loads with no format change: that recipe is retired and dropped
/// on load (`World::clean_recipes`), the steel the band holds stays in its stores, and the
/// smith's recipe (hammer at the open charcoal hearth) is allowed.
#[test]
fn step13_old_furnace_steel_is_retired_on_load() {
    use crate::knowledge::{Known, Recipe};
    let bytes = include_bytes!("../tests/fixtures/v12_seed9_y170_64x40.civsave");
    let (mut w, _, _) = World::load_bytes(bytes).expect("fixture loads");
    let (iron, steel) = (item(Form::Raw, Mat::Iron), item(Form::Raw, Mat::Steel));
    let (bel, ch, clay) = (item(Form::Bellows, Mat::Hide), item(Form::Fire, Mat::Charcoal), item(Form::Kiln, Mat::Clay));
    let ham = item(Form::Hammer, Mat::Stone);
    let old = physics::canonical(Action::Heat, [iron, bel, ch, clay]);
    let new = physics::canonical(Action::Heat, [iron, ham, ch, bel]);
    let r_old = w.reg.insert(Recipe { action: Action::Heat as u8, inputs: old, output: steel, p: physics::STEEL_P, ..Default::default() });
    let r_new = w.reg.insert(Recipe { action: Action::Heat as u8, inputs: new, output: steel, p: physics::STEEL_P, ..Default::default() });
    assert!(w.reg.retired(r_old) && !w.reg.retired(r_new));
    w.bands[0].known.push(Known { recipe: r_old, skill: 0.9, ..Default::default() });
    w.bands[1].known.push(Known { recipe: r_new, skill: 0.9, ..Default::default() });
    w.bands[0].stock[Mat::Steel as usize] = 0.8;
    let (w2, _, info) = World::load_bytes(&w.save_bytes(None, "t", "t").unwrap()).unwrap();
    assert_eq!(info.format_version, crate::save::FORMAT_VERSION);
    assert!(!w2.bands[0].knows(r_old), "furnace steel dropped");
    assert!(w2.bands[1].knows(r_new), "smith's steel kept");
    assert_eq!(w2.bands[0].stock[Mat::Steel as usize], 0.8, "the steel itself stays");
}

/// (inventions step 13, diagnostic) Why pitch and sealed jars come late on big maps: per season,
/// how many bands could make pitch (resinous wood at hand + a fired pot + a fire) and how many
/// could seal a jar (pitch + a fired pot), summed until each is first found, against when bands
/// first reach resinous country at all and when the first potters do. Same knobs as
/// `probe_step12` (PROBE_SEEDS, PROBE_YEARS, PROBE_W, PROBE_H).
#[test]
#[ignore]
fn probe_step13_pitch() {
    let seeds: Vec<u64> = std::env::var("PROBE_SEEDS").unwrap_or("5".into()).split(',').map(|s| s.trim().parse().unwrap()).collect();
    let years: u32 = std::env::var("PROBE_YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(500);
    let (mw, mh): (usize, usize) = (std::env::var("PROBE_W").ok().and_then(|s| s.parse().ok()).unwrap_or(256), std::env::var("PROBE_H").ok().and_then(|s| s.parse().ok()).unwrap_or(160));
    let (resin, pot, pitch) = (item(Form::Raw, Mat::ResinWood), item(Form::Vessel, Mat::Ceramic), item(Form::Raw, Mat::Pitch));
    let jar = item(Form::Jar, Mat::Ceramic);
    for seed in seeds {
        let mut w = World::new(Config { seed, map_w: mw, map_h: mh, ..Default::default() }, Params::default());
        let m = &w.map;
        let n = m.w * m.h;
        let land = (0..n).filter(|&i| m.is_land(i)).count();
        let rc: Vec<usize> = (0..n).filter(|&i| m.resin_wood(i)).collect();
        let s0 = w.bands[0].cell(m.w);
        let (sx, sy) = ((s0 % m.w) as i64, (s0 / m.w) as i64);
        let d0 = rc.iter().map(|&c| ((c % m.w) as i64 - sx).abs().max(((c / m.w) as i64 - sy).abs())).min().unwrap_or(-1);
        println!("seed {seed} {mw}x{mh}: land {land}, resinous cells {} ({:.1}% of land); start {:?} lat {:.0}, nearest resinous cell {d0} cells away", rc.len(), 100.0 * rc.len() as f32 / land as f32, m.biome_at(s0), m.lat(s0 / m.w));
        let (mut f_on, mut f_pot, mut f_rdy, mut f_pitch, mut f_jrdy, mut f_jar) = (None, None, None, None, None, None);
        let (mut rdy_bs, mut jrdy_bs, mut exp_rdy) = (0u64, 0u64, 0u64);
        let yr = |t: u32| t as f32 / 4.0;
        for t in 0..years * 4 {
            w.step();
            if w.bands.is_empty() {
                break;
            }
            let (mut on, mut pots, mut rdy, mut jrdy, mut bands) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for b in &w.bands {
                let a = &b.d.tk.avail;
                bands += 1;
                let r = a.has(resin);
                let p = a.has(pot);
                let f = a.iter().any(|i| form(i) == Form::Fire);
                on += r as u32;
                pots += p as u32;
                if r && p && f {
                    rdy += 1;
                    if f_pitch.is_none() {
                        exp_rdy += b.d.experiments as u64;
                    }
                }
                jrdy += (a.has(pitch) && p) as u32;
            }
            let found = |o: Item| w.reg.recipes.iter().any(|r| r.output == o);
            if f_on.is_none() && on > 0 { f_on = Some(yr(t)); }
            if f_pot.is_none() && pots > 0 { f_pot = Some(yr(t)); }
            if f_rdy.is_none() && rdy > 0 { f_rdy = Some(yr(t)); }
            if f_pitch.is_none() {
                rdy_bs += rdy as u64;
                if found(pitch) { f_pitch = Some(yr(t)); println!("  Y{:.0} pitch found: {rdy_bs} pitch-ready band-seasons ({exp_rdy} experiments by them) since the first ready band", yr(t)); }
            }
            if f_jrdy.is_none() && jrdy > 0 { f_jrdy = Some(yr(t)); }
            if f_jar.is_none() && f_pitch.is_some() {
                jrdy_bs += jrdy as u64;
                if found(jar) { f_jar = Some(yr(t)); println!("  Y{:.0} sealed jar found: {jrdy_bs} jar-ready band-seasons", yr(t)); }
            }
            if (t + 1) % 100 == 0 {
                // the picker's odds (attention only, before need weighting), per season, summed
                // over the ready bands: expected finds of pitch (heat) and of a sealed jar (combine)
                let (mut e_pitch, mut e_jar, mut pool_n, mut nj) = (0f64, 0f64, 0f64, 0u32);
                let (mut r_pitch, mut r_pot, mut c_pitch, mut c_cer) = (0f64, 0f64, 0f64, 0f64);
                for b in &w.bands {
                    let a = &b.d.tk.avail;
                    let fire = a.iter().any(|i| form(i) == Form::Fire);
                    if !(a.has(pot) && fire && (a.has(resin) || a.has(pitch))) {
                        continue;
                    }
                    let att = w.attention(b);
                    let pool: Vec<Item> = a.iter().filter(|&i| !matches!(form(i), Form::Cooked | Form::Fire | Form::Kiln)).collect();
                    let nf = a.iter().filter(|&i| form(i) == Form::Fire).count();
                    let cr = |k: usize| b.craft.get(NMAT + k).copied().unwrap_or(1.0) as f64;
                    let (wh, ws, wc) = (0.15 * cr(Action::Heat as usize), 0.45 * cr(Action::Shape as usize), 0.40 * cr(Action::Combine as usize));
                    let tot = wh + ws + wc;
                    let ex = b.d.experiments as f64;
                    if a.has(resin) {
                        let hw = |i: Item| {
                            let m = mat(i);
                            let keeps = if matches!(m, Mat::Fibre | Mat::Hide | Mat::Bone) { 0.3 } else if m.is_food() || m.props().fuel_t > 0.0 { 1.0 } else { 1.5 };
                            let unf = if matches!(form(i), Form::Bloom | Form::Crucible) || (matches!(form(i), Form::Raw | Form::Edge) && physics::hot_worked(m)) { 8.0 } else { 1.0 };
                            (att[m as usize] as f64).sqrt() * keeps * unf
                        };
                        let hs: f64 = pool.iter().map(|&i| hw(i)).sum();
                        let (pr, pp) = (hw(resin) / hs, hw(pot) / hs);
                        let p_pair = pr * (0.5 + 0.5 * 0.25 * pp) + pp * 0.25 * pr;
                        e_pitch += ex * wh / tot * p_pair * 0.2;
                    }
                    if a.has(pitch) {
                        nj += 1;
                        pool_n += pool.len() as f64;
                        let cs: f64 = pool.iter().map(|&i| att[mat(i) as usize] as f64).sum::<f64>() + nf as f64 * 1.0;
                        let (pp, pv) = (att[Mat::Pitch as usize] as f64 / cs, att[Mat::Ceramic as usize] as f64 / cs);
                        e_jar += ex * wc / tot * 2.0 * pp * pv * 0.55 * 0.25;
                        let mean = cs / (pool.len() + nf) as f64;
                        r_pitch += att[Mat::Pitch as usize] as f64 / mean;
                        r_pot += att[Mat::Ceramic as usize] as f64 / mean;
                        c_pitch += b.craft[Mat::Pitch as usize] as f64;
                        c_cer += b.craft[Mat::Ceramic as usize] as f64;
                    }
                }
                println!(
                    "  Y{:<4} bands {bands:>6} | on resin {on:>6} | pots {pots:>6} | pitch-ready {rdy:>6} | jar-ready {jrdy:>6} | expected finds/season (attention only, before the 12b tar-pot and step 13 pitch-pot pairings for jars): pitch {e_pitch:.3}, jar {e_jar:.4} (jar-ready pool {:.0} items; attention vs pool mean: pitch {:.2} pot {:.2}; craft pitch {:.2} ceramic {:.2})",
                    (t + 1) / 4,
                    pool_n / nj.max(1) as f64,
                    r_pitch / nj.max(1) as f64,
                    r_pot / nj.max(1) as f64,
                    c_pitch / nj.max(1) as f64,
                    c_cer / nj.max(1) as f64
                );
            }
            if f_jar.is_some() { break; }
        }
        println!("  firsts: band at resin {f_on:?}, pots {f_pot:?}, pitch-ready {f_rdy:?}, pitch {f_pitch:?} | jar-ready {f_jrdy:?}, jar {f_jar:?} | ready band-seasons before pitch {rdy_bs}, before jar {jrdy_bs}");
    }
}
