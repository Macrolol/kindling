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
    assert_eq!(w.map.gen_version, crate::map::GEN_VERSION, "legacy map upgraded (iron ore added too)");
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
        assert_eq!(&b.craft[23..25], &[1.0, 1.0]);
        assert_eq!(&b.craft[25..], &oc[23..]);
        assert_eq!(&b.stock[23..], &[0.0, 0.0]);
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
    assert!(info.migrated_from.is_none() || info.migrated_from == Some(8));
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
        assert_eq!(a.known.len(), b.known.len());
    }
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
    assert_eq!(comb([boat, shaft, item(Form::Garment, Mat::Flax), NONE], &wet), Some(item(Form::Sail, Mat::Flax)));
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
    let k1 = crate::band::make_kit(&s);
    assert!(World::boat_safety(&k1) > World::boat_safety(&k0));
    assert!(World::boat_range(&k1, 0.5) > 1.5 * World::boat_range(&k0, 0.5));
    assert_eq!(World::boat_range(&k1, 0.2), World::boat_range(&k0, 0.2));
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
    assert_eq!(shape(item(Form::Point, Mat::Tin)), Some(item(Form::Ornament, Mat::Tin)));
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
