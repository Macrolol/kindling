use crate::materials::*;
use crate::physics::{self, Action, Ctx};
use crate::{Config, Params, World};

fn small() -> World {
    World::new(
        Config {
            seed: 7,
            map_w: 80,
            map_h: 50,
            ..Default::default()
        },
        Params::default(),
    )
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
    let ctx = Ctx {
        dryness: 0.5,
        water: true,
    };
    let raw = |m| item(Form::Raw, m);
    // no copper from an open wood fire
    let inp = physics::canonical(
        Action::Heat,
        [raw(Mat::CopperOre), NONE, item(Form::Fire, Mat::Wood), NONE],
    );
    assert!(physics::apply(Action::Heat, &inp, &ctx)
        .map(|o| mat(o.out) != Mat::Copper)
        .unwrap_or(true));
    // knapping flint with a stone hammer gives an edge
    let inp = physics::canonical(
        Action::Shape,
        [raw(Mat::Flint), item(Form::Hammer, Mat::Stone), NONE, NONE],
    );
    assert_eq!(
        physics::apply(Action::Shape, &inp, &ctx).map(|o| o.out),
        Some(item(Form::Edge, Mat::Flint))
    );
    // charcoal in a clay kiln smelts copper
    let inp = physics::canonical(
        Action::Heat,
        [
            raw(Mat::CopperOre),
            NONE,
            item(Form::Fire, Mat::Charcoal),
            item(Form::Kiln, Mat::Clay),
        ],
    );
    assert_eq!(
        physics::apply(Action::Heat, &inp, &ctx).map(|o| mat(o.out)),
        Some(Mat::Copper)
    );
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
    assert!(
        w.tend.iter().any(|&t| t > 0.0),
        "camps start to tend wild stands"
    );
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
    assert_eq!(info.migrated_from, None);
    assert!(w.total_pop() > 0);
    let n = w.map.w * w.map.h;
    assert_eq!(w.tend.len(), n);
    assert!(
        w.tend.iter().any(|&t| t > 0.0),
        "beta tending survives the load"
    );
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
    assert_eq!(
        w.params.aggression,
        crate::params::Params::default().aggression
    );
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
    assert_eq!(
        w.map.gen_version,
        crate::map::GEN_VERSION,
        "legacy map upgraded (iron ore added too)"
    );
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
    let mut w = World::new(
        Config {
            seed: 2,
            map_w: 160,
            map_h: 100,
            ..Default::default()
        },
        Params::default(),
    );
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
            if n > 0 {
                println!("--");
            }
        }
    }
}

/// Basic sanity and conservation over a few hundred seasons: everyone is on habitable land, no
/// NaNs or negative stocks creep in, wild food stays within the land's capacity, population
/// totals agree with the bands, and band ids stay unique.
#[test]
fn sanity_and_conservation() {
    let mut w = World::new(
        Config {
            seed: 3,
            map_w: 120,
            map_h: 80,
            ..Default::default()
        },
        Params::default(),
    );
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
            assert!(
                b.store.is_finite() && b.store >= 0.0,
                "band {} store {}",
                b.id,
                b.store
            );
            assert!(
                b.stock.iter().all(|v| v.is_finite() && *v >= -1e-3),
                "band {} stock",
                b.id
            );
        }
        assert_eq!(pop, w.total_pop());
        for i in 0..n {
            // tended land carries more wild plants than its natural capacity
            let kp = match w.tend.get(i) {
                Some(&td) if td > 0.0 => {
                    crate::world::farming::wild_plant_cap(w.map.k_plants[i], td)
                }
                _ => w.map.k_plants[i],
            };
            for (v, k) in [
                (w.plants[i], kp),
                (w.game[i], w.map.k_game[i]),
                (w.fish[i], w.map.k_fish[i]),
            ] {
                assert!(v.is_finite() && v >= -1e-3, "stock {v} at {i}");
                // (tending decays after the regrowth step, so the tended cap can sit slightly below)
                assert!(
                    v <= k.max(0.0) * 1.02 + 1e-3,
                    "stock {v} over capacity {k} at {i}"
                );
            }
        }
        assert!(w
            .trade_flow
            .iter()
            .chain(&w.conflict_flow)
            .all(|v| v.is_finite() && *v >= 0.0));
        assert!(w
            .tend
            .iter()
            .chain(&w.dom)
            .chain(&w.fert)
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
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
    let base = Config {
        seed: 7,
        map_w: 80,
        map_h: 50,
        ..Default::default()
    };
    let m = crate::map::Map::generate(base.map_w, base.map_h, base.seed);
    let good: Vec<(u16, u16)> = (0..m.h)
        .flat_map(|y| (0..m.w).map(move |x| (x, y)))
        .filter(|&(x, y)| m.start_check(x, y) == StartCheck::Ok)
        .map(|(x, y)| (x as u16, y as u16))
        .collect();
    assert!(good.len() > 10);
    let sea = (0..m.w * m.h)
        .find(|&i| !m.is_land(i))
        .map(|i| ((i % m.w) as u16, (i / m.w) as u16))
        .unwrap();
    assert_eq!(
        m.start_check(sea.0 as usize, sea.1 as usize),
        StartCheck::Uninhabitable
    );
    // shared area
    let p = good[good.len() / 2];
    let w = World::new(
        Config {
            starts: vec![p],
            ..base.clone()
        },
        Params::default(),
    );
    assert_eq!(w.bands.len(), base.founders);
    for b in &w.bands {
        assert!((b.x as i32 - p.0 as i32).abs() <= 3 && (b.y as i32 - p.1 as i32).abs() <= 3);
    }
    assert!(w
        .events
        .iter()
        .chain(&w.chronicle)
        .any(|e| w.event_text(e).contains("(chosen start)")));
    // one per lineage (more than the default number of founders)
    let picks: Vec<(u16, u16)> = good
        .iter()
        .step_by(good.len() / 8)
        .take(8)
        .copied()
        .collect();
    let mut w = World::new(
        Config {
            starts: picks.clone(),
            ..base.clone()
        },
        Params::default(),
    );
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
    let a = World::new(
        Config {
            starts: vec![sea],
            ..base.clone()
        },
        Params::default(),
    );
    let b = World::new(base.clone(), Params::default());
    let pos = |w: &World| {
        w.bands
            .iter()
            .map(|b| (b.x, b.y, b.pop))
            .collect::<Vec<_>>()
    };
    assert_eq!(pos(&a), pos(&b));
    // parsing
    assert_eq!(crate::parse_starts("3,4").unwrap(), vec![(3, 4)]);
    assert_eq!(
        crate::parse_starts(" 3,4 ; 10, 2;").unwrap(),
        vec![(3, 4), (10, 2)]
    );
    assert!(crate::parse_starts("3").is_err());
    assert!(crate::parse_starts("a,b").is_err());
}

/// With the parallel feature, the result does not depend on the number of threads (each band
/// and each meeting draws from its own random stream and effects apply in a fixed order).
#[cfg(feature = "parallel")]
#[test]
fn same_result_on_any_thread_count() {
    let run = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| {
            let mut w = World::new(
                Config {
                    seed: 5,
                    map_w: 120,
                    map_h: 80,
                    ..Default::default()
                },
                Params::default(),
            );
            for _ in 0..320 {
                w.step();
            }
            (w.state_hash(), w.total_pop(), w.bands.len())
        })
    };
    let one = run(1);
    assert!(
        one.2 >= 20,
        "too few bands ({}) to exercise the parallel paths",
        one.2
    );
    assert_eq!(one, run(4));
}
