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
    let ctx = Ctx { dryness: 0.5 };
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
