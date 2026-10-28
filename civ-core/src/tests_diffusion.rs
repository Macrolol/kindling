//! Unit tests for the diffusion step (D1-D6, docs/REPORT-diffusion.md section 8).
use crate::materials::*;
use crate::{Config, Params, World};

fn world() -> World {
    World::new(Config { seed: 7, map_w: 80, map_h: 50, ..Default::default() }, Params::default())
}

/// D1: hide and bone are at hand in a season the band hunted big game, wherever it is; not
/// otherwise (they have no deposit). Other materials are unchanged.
#[test]
fn d1_hide_and_bone_at_hand_for_hunters() {
    let w = world();
    let b = &w.bands[0];
    let ci = b.cell(w.map.w);
    for mm in [Mat::Hide, Mat::Bone] {
        assert!(w.mat_at_hand(b, ci, mm, 3.0), "{mm:?} after a hunting season");
        assert!(!w.mat_at_hand(b, ci, mm, 0.5), "{mm:?} without big game");
    }
    assert_eq!(w.mat_at_hand(b, ci, Mat::Wood, 3.0), w.map.k_wood[ci] > 2.0);
    assert!(!w.mat_at_hand(b, ci, Mat::Brine, 3.0));
}

/// D1: a hunting band that needs hide keeps a stock of it: the season's gain (0.01 per unit
/// hunted) covers the plain wear (0.02) once the band hunts 2+, with no extra wear.
#[test]
fn d1_hide_stock_holds_for_hunters() {
    let mut w = world();
    let mut kept = 0;
    let mut hunters = 0;
    for _ in 0..4 * 40 {
        w.step();
    }
    for b in &w.bands {
        let h = b.d.harvest;
        if h[1] + 0.3 * h[3] >= 2.5 {
            hunters += 1;
            if b.stock[Mat::Hide as usize] > 0.05 {
                kept += 1;
            }
        }
    }
    assert!(hunters > 0, "some bands hunt");
    assert_eq!(kept, hunters, "every band hunting 2.5+ holds hide");
}

/// D2: a worn ornament counts as practised (it is in the season's `used`, so its making stays
/// in mind) but takes no upkeep and draws on no stock.
#[test]
fn d2_ornament_practised_without_wear() {
    let mut w = world();
    for _ in 0..8 {
        w.step();
    }
    let mut b = w.bands[0].clone();
    b.d.kit.ornament = NONE;
    let bare = w.economy(&b, 1);
    let beads = item(Form::Ornament, Mat::Bone);
    b.d.kit.ornament = beads;
    let worn = w.economy(&b, 1);
    assert!(worn.used.contains(&beads), "worn ornament is practised");
    assert!(!bare.used.contains(&beads));
    assert_eq!(worn.upkeep.to_bits(), bare.upkeep.to_bits(), "no wear");
    assert_eq!(worn.needed, bare.needed, "no stock drawn");
}

/// D3: fully practised, a copper knife outranks flint (it keeps its edge and needs little
/// mending), and so do bronze, iron and steel blades over flint and obsidian.
#[test]
fn d3_metal_blades_outrank_stone_when_practised() {
    use crate::band::make_kit;
    use crate::materials::ItemSet;
    for metal in [Mat::Copper, Mat::Bronze, Mat::Iron, Mat::Steel] {
        for stone in [Mat::Flint, Mat::Obsidian] {
            let mut a = ItemSet::default();
            for f in [Form::Edge, Form::Spear, Form::Point] {
                a.add(item(f, metal));
                a.add(item(f, stone));
            }
            let k = make_kit(&a);
            assert_eq!(k.cut, item(Form::Edge, metal), "{metal:?} knife over {stone:?}");
            assert_eq!(k.hunt, item(Form::Spear, metal), "{metal:?} spear over {stone:?}");
            assert_eq!(k.point, item(Form::Point, metal));
            assert_eq!(k.trial, [NONE; 12], "nothing better in prospect");
        }
    }
}

/// D3: a newly learnt copper knife is not yet relied on while the flint one is honed, but it
/// is tried out; once practised it takes over and flint is no longer in prospect.
#[test]
fn d3_newcomer_is_tried_then_takes_over() {
    use crate::band::make_kit_with;
    use crate::materials::ItemSet;
    let (flint, copper) = (item(Form::Edge, Mat::Flint), item(Form::Edge, Mat::Copper));
    let mut a = ItemSet::default();
    a.add(flint);
    a.add(copper);
    let kit = |sk: f32| make_kit_with(&a, &|it| if it == copper { (sk, 0.0, 3) } else { (1.0, 0.0, 1) });
    let k = kit(0.3);
    assert_eq!((k.cut, k.trial[1]), (flint, copper), "newcomer tried beside the honed tool");
    let k = kit(0.9);
    assert_eq!((k.cut, k.trial[1]), (copper, NONE), "practised, it takes over");
}

/// D3: a band using a slot with a challenger tries it about one season in five
/// (`KIT_TRIAL_P`), deterministically; the trial adds wear and marks it used.
#[test]
fn d3_trials_about_one_season_in_five() {
    let mut w = world();
    for _ in 0..8 {
        w.step();
    }
    let mut b = w.bands[0].clone();
    b.d.kit.cut = item(Form::Edge, Mat::Flint);
    let ch = item(Form::Edge, Mat::Copper);
    b.d.kit.trial = [NONE; 12];
    let plain = w.economy(&b, 1);
    assert!(plain.used.contains(&b.d.kit.cut));
    b.d.kit.trial[1] = ch;
    let mut tried = 0;
    for t in 0..400 {
        w.tick = 1000 + t;
        let e = w.economy(&b, 1);
        if e.used.contains(&ch) {
            tried += 1;
            assert!(e.upkeep > plain.upkeep, "a trial wears the tool");
        }
        let e2 = w.economy(&b, 1);
        assert_eq!(e.used, e2.used, "deterministic");
    }
    assert!((50..110).contains(&tried), "tried {tried} of 400 seasons");
}

/// D4: a recipe whose inputs the learner has is shown four times as often as one it has
/// nothing for; partly usable ones lie between.
#[test]
fn d4_teaching_weighted_to_usable_recipes() {
    use crate::world::teach_weight;
    let (a, b) = (item(Form::Raw, Mat::Flint), item(Form::Raw, Mat::Wood));
    let mut avail = ItemSet::default();
    avail.add(a);
    assert_eq!(teach_weight(&[a, NONE, NONE, NONE], &avail), 4.0);
    assert_eq!(teach_weight(&[b, NONE, NONE, NONE], &avail), 1.0);
    assert_eq!(teach_weight(&[a, b, NONE, NONE], &avail), 2.5);
    assert_eq!(teach_weight(&[NONE; 4], &avail), 4.0);
}

/// D5: an unused recipe fades at half the rate for 40 seasons after it was last practised or
/// makeable, then at the usual rate; practice and inputs at hand renew the date.
#[test]
fn d5_recent_recipes_fade_slower() {
    use crate::knowledge::Known;
    use crate::world::{practise, RECENT_FADE, RECENT_SEASONS};
    let fresh = Known { skill: 0.5, last_ok: 100, ..Default::default() };
    let (mut a, mut b) = (fresh, Known { last_ok: 0, ..fresh });
    practise(&mut a, 0.1, 0.01, 100 + RECENT_SEASONS, false);
    practise(&mut b, 0.1, 0.01, 100 + RECENT_SEASONS, false);
    assert!((0.5 - a.skill - RECENT_FADE * 0.01).abs() < 1e-6, "recent: half the loss");
    assert!((0.5 - b.skill - 0.01).abs() < 1e-6, "long ago: the usual loss");
    let mut c = fresh;
    practise(&mut c, 0.1, 0.01, 100 + RECENT_SEASONS + 1, false);
    assert!((0.5 - c.skill - 0.01).abs() < 1e-6, "past the window: the usual loss");
    // inputs at hand: still unpractised (it fades), but the date renews
    let mut d = b;
    practise(&mut d, 0.1, 0.01, 500, true);
    assert_eq!(d.last_ok, 500);
    assert!(d.skill < b.skill);
    let mut e = Known { used: true, ..b };
    practise(&mut e, 0.1, 0.01, 600, false);
    assert_eq!(e.last_ok, 600);
    assert!(e.skill > b.skill);
}

/// D5: bands' recipes carry their dates in play and through a save (bands blob layout 2).
#[test]
fn d5_last_ok_kept_and_saved() {
    let mut w = world();
    for _ in 0..4 * 30 {
        w.step();
    }
    let dated = w.bands.iter().flat_map(|b| &b.known).filter(|k| k.last_ok > 0).count();
    assert!(dated > 0, "recipes in use are dated");
    let bytes = w.save_bytes(None, "test", "now").unwrap();
    let (w2, _, info) = World::load_bytes(&bytes).unwrap();
    assert_eq!(info.format_version, crate::save::FORMAT_VERSION);
    let dates = |w: &World| w.bands.iter().flat_map(|b| b.known.iter().map(|k| (k.recipe, k.last_ok))).collect::<Vec<_>>();
    assert_eq!(dates(&w), dates(&w2));
}

/// D5: a save from before the diffusion step (bands blob layout 1) loads with every date at 0
/// ("long ago"), so its recipes fade as they did.
#[test]
fn d5_old_saves_load_with_no_dates() {
    let bytes = include_bytes!("../tests/fixtures/v13_seed3_y350_96x60.civsave");
    let (w, _, _) = World::load_bytes(bytes).expect("v13 save loads");
    assert!(w.bands.iter().any(|b| !b.known.is_empty()));
    assert!(w.bands.iter().flat_map(|b| &b.known).all(|k| k.last_ok == 0));
}
