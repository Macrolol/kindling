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
    assert_eq!(w.map.gen_version, 2, "legacy map upgraded");
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
