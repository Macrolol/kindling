//! (skills step 1) The economy pays what `tool_effects` says, and a food slot is the item
//! that maximises food minus the food its upkeep labour would have grown.
use crate::band::{edge_q, Band, Kit, Partner};
use crate::band::{KIT_UPKEEP, QUALITY_YIELD};
use crate::effects::{
    food_mults, harvest_for, kit_yields, make_kit_with_view, novice_factor, pitch_in_use, recipe_info, sealed_factor, slot_score, tool_effects,
    tool_upkeep, transfer_table, u_eff, user_factor, BandView, HARVEST_EPS, HARVEST_PRIOR, LABOUR_SHARE, NOMINAL_MIX, NOVICE_SD, USE_FADE_X,
    USE_FOUNDER, USE_INHERIT, USE_NOVICE, USE_PRACTICE, USE_TEACH_SHARE, USE_TRANSFER, WORK_SHARE,
};
use crate::materials::*;
use crate::rng::Rng;
use crate::world::fx::{Fx, Pair};
use crate::world::{inherit_use, practise_use, CLOSE_TIE, PITCH_GLUE};
use crate::{Config, Params, World};

/// Last season's harvest shrunk halfway toward `NOMINAL_MIX` at the same total.
/// Written out here, not via `harvest_for`, so the slow score stays an independent check.
fn smoothed(view: &BandView) -> [f32; 5] {
    let t: f32 = (0..5).map(|r| view.harvest[r]).sum();
    let mut h = [0.0; 5];
    for r in 0..5 {
        h[r] = (1.0 - HARVEST_PRIOR) * view.harvest[r] + HARVEST_PRIOR * NOMINAL_MIX[r] * t;
    }
    h
}

fn world() -> World {
    World::new(Config { seed: 7, map_w: 80, map_h: 50, ..Default::default() }, Params::default())
}

fn mult(k: &Kit, r: usize) -> f32 {
    match r {
        0 => k.plants_m,
        1 => k.game_m,
        2 => k.fish_m,
        3 => k.small_m,
        _ => k.farm_m,
    }
}

fn clear_slot(k: &mut Kit, slot: usize) {
    match slot {
        0 => k.hunt = NONE,
        1 => k.cut = NONE,
        10 => k.point = NONE,
        4 => k.axe = NONE,
        12 => k.net = NONE,
        13 => k.trap = NONE,
        _ => {}
    }
}

fn item_in(k: &Kit, slot: usize) -> Item {
    match slot {
        0 => k.hunt,
        1 => k.cut,
        10 => k.point,
        4 => k.axe,
        12 => k.net,
        13 => k.trap,
        _ => NONE,
    }
}

/// The wear the season charges, written out here so a shared wrong formula in `tool_upkeep`
/// and `World::economy` still fails. Ornaments are show. A fire or a cooked meal is 0.01.
/// Anything else is `sealed * rate * (1.15 - durability) * (1 + 0.3 * depth)`.
fn wear(rate: f32, it: Item, depth: u8, sealed: f32) -> f32 {
    if form(it) == Form::Ornament {
        return 0.0;
    }
    if matches!(form(it), Form::Fire | Form::Cooked) {
        return 0.01;
    }
    sealed * rate * (1.15 - durability(mat(it))) * (1.0 + 0.3 * depth as f32)
}

/// Which payoff slot already holds `it`, so `tool_effects` does not move it.
fn slot_holding(k: &Kit, it: Item) -> usize {
    let slots = [
        (0, k.hunt),
        (1, k.cut),
        (10, k.point),
        (4, k.axe),
        (11, k.boat),
        (12, k.net),
        (13, k.trap),
        (14, k.bow),
        (15, k.shield),
        (16, k.outrigger),
        (18, k.plank),
        (22, k.ship),
    ];
    for (slot, cur) in slots {
        if cur == it {
            return slot;
        }
    }
    // not a payoff slot (a pot, a fire): leave the kit where it is
    99
}

fn charged_kit_items(b: &Band, ys_fish: f32, wet: bool, pitch: bool) -> Vec<Item> {
    let kit = &b.d.kit;
    let mut v = Vec::new();
    for &it in kit.used_items().iter() {
        if it == NONE {
            continue;
        }
        let craft = it == kit.boat || it == kit.outrigger || it == kit.sail || it == kit.plank || it == kit.ship;
        if it == kit.ornament {
            v.push(it);
            continue;
        }
        let idle_mark = it == kit.mark && !World::keeps_count(b);
        if (craft && !wet) || (it == kit.net && ys_fish < 1.0) || (it == kit.pitch && !pitch) || idle_mark {
            continue;
        }
        v.push(it);
    }
    v
}

#[test]
fn payout_invariant() {
    let mut w = world();
    // edges are invented, not given: run long enough that some band is hunting with a tool
    for _ in 0..4 * 80 {
        w.step();
    }
    let food_tool = |b: &Band| {
        let kit = [
            b.d.kit.hunt,
            b.d.kit.cut,
            b.d.kit.point,
            b.d.kit.axe,
            b.d.kit.net,
            b.d.kit.trap,
        ];
        b.d.harvest.iter().sum::<f32>() > 1.0 && kit.iter().any(|&it| it != NONE)
    };
    let b = w.bands.iter().find(|b| food_tool(b)).expect("a band harvesting with a food tool");
    let s = w.season();
    let ec = w.economy(b, s);
    let view = w.band_view(b);
    let kit = &b.d.kit;
    assert!(view.harvest.iter().sum::<f32>() > 1.0, "the real harvest path, not the nominal fallback");
    let info = |it: Item| recipe_info(&b.d.tk, &b.known, it);
    let y = kit_yields(kit, &info, &view.user);
    for (got, want, name) in [
        (y.plants_m, kit.plants_m, "plants"),
        (y.game_m, kit.game_m, "game"),
        (y.fish_m, kit.fish_m, "fish"),
        (y.small_m, kit.small_m, "small"),
        (y.farm_m, kit.farm_m, "farm"),
        (y.boat_q, kit.boat_q, "boat"),
        (y.protect, kit.protect, "protect"),
        (y.fine, kit.fine, "fine"),
        (y.ship_q, kit.ship_q, "ship_q"),
    ] {
        assert_eq!(got.to_bits(), want.to_bits(), "{name}");
    }
    assert_eq!(y.digest.map(f32::to_bits), kit.digest.map(f32::to_bits));
    assert_eq!(y.smoke, kit.smoke);

    let pitch = pitch_in_use(kit, view.wet);
    let charged = charged_kit_items(b, ec.ys[2], view.wet, pitch);
    let mut sum_fx = 0.0;
    let mut sum_wear = 0.0;
    let mut rest = ec.used.clone();
    for &it in &charged {
        let depth = b.d.tk.via(it).map(|v| v.2).unwrap_or(0);
        let mask = b.d.tk.via(it).map(|v| v.1).unwrap_or(0);
        let sealed = sealed_factor(it, kit, pitch);
        sum_fx += tool_effects(&view, kit, slot_holding(kit, it), it, &info, mask).labour;
        sum_wear += wear(w.params.upkeep, it, depth, sealed);
        if let Some(i) = rest.iter().position(|&x| x == it) {
            rest.remove(i);
        }
    }
    // what is left was tried out beside the kit. Trials are not glued or caulked.
    for &ch in &rest {
        let depth = b.d.tk.via(ch).map(|v| v.2).unwrap_or(0);
        sum_fx += tool_upkeep(w.params.upkeep, ch, depth, 1.0);
        sum_wear += wear(w.params.upkeep, ch, depth, 1.0);
    }
    assert!((sum_fx - ec.upkeep).abs() < 1e-6, "effects {sum_fx} economy {}", ec.upkeep);
    assert!((sum_wear - ec.upkeep).abs() < 1e-6, "wear {sum_wear} economy {}", ec.upkeep);

    let mut checked = 0;
    for &slot in &[0usize, 1, 10, 4, 12, 13] {
        let it = item_in(kit, slot);
        if it == NONE {
            continue;
        }
        let mut bare = *kit;
        clear_slot(&mut bare, slot);
        let yw = kit_yields(kit, &info, &view.user);
        let ye = kit_yields(&bare, &info, &view.user);
        let dg = yw.digest;
        let hd: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
        assert!(hd > HARVEST_EPS);
        let h = smoothed(&view);
        let mut food = 0.0;
        for r in 0..5 {
            let (m1, m0) = (mult(&yw, r), mult(&ye, r));
            food += h[r] * dg[r] * (m1 - m0) / m0;
        }
        let mask = b.d.tk.via(it).map(|v| v.1).unwrap_or(0);
        let fx = tool_effects(&view, kit, slot, it, &info, mask);
        assert!((fx.food - food).abs() < 1e-5, "slot {slot} effects {} formula {food}", fx.food);
        let u = view.user[form(it) as usize];
        let want_sd = NOVICE_SD * (1.0 - u) / 3.0f32.sqrt() * fx.food.abs();
        assert_eq!(fx.food_sd.to_bits(), want_sd.to_bits(), "slot {slot} novice spread");
        assert_eq!(fx.p_fail, 0.0);
        checked += 1;
    }
    assert!(
        checked > 0,
        "some food slot is filled; hunt {:?} cut {:?} point {:?} axe {:?} net {:?} trap {:?} avail {}",
        kit.hunt,
        kit.cut,
        kit.point,
        kit.axe,
        kit.net,
        kit.trap,
        b.d.tk.avail.count()
    );
}

#[test]
fn nominal_harvest_fallback_matches_the_mix() {
    let mut view = BandView::nominal();
    view.harvest = [0.0; 5];
    view.food = 0.0;
    let it = item(Form::Edge, Mat::Flint);
    let mut k = Kit::default();
    k.cut = it;
    let info = |_: Item| (1.0, 0.0, 0);
    let fx = tool_effects(&view, &k, 1, it, &info, 0);
    // no digested harvest: nominal mix times this band's pop, digest 1 on a bare kit
    let pop = view.pop;
    let h = NOMINAL_MIX.map(|x| x * pop);
    let mut bare = k;
    bare.cut = NONE;
    let yw = kit_yields(&k, &info, &view.user);
    let ye = kit_yields(&bare, &info, &view.user);
    let mut food = 0.0;
    for r in 0..5 {
        food += h[r] * yw.digest[r] * (mult(&yw, r) - mult(&ye, r)) / mult(&ye, r);
    }
    assert!(food > 0.0);
    assert!((fx.food - food).abs() < 1e-5, "{} vs {food}", fx.food);
}

/// One season of pure game is not the weight the kit sees: half of that total is spread
/// in the nominal mix. No digested harvest still falls back to `NOMINAL_MIX * pop`.
#[test]
fn harvest_prior_smooths_mix() {
    let mut view = BandView::nominal();
    view.harvest = [0.0, 20.0, 0.0, 0.0, 0.0];
    let dg = [1.0; 5];
    let h = harvest_for(&view, &dg);
    let want = smoothed(&view);
    assert_eq!(h.map(f32::to_bits), want.map(f32::to_bits));
    assert_ne!(h[1].to_bits(), view.harvest[1].to_bits(), "game is shrunk toward the mix");
    assert!(h[0] > 0.0, "plants take a share of an all-game season");

    // the nominal mix still matters: a flint blade beats a stone blade on this view
    let flint = item(Form::Edge, Mat::Flint);
    let stone = item(Form::Edge, Mat::Stone);
    let mut avail = ItemSet::default();
    avail.add(flint);
    avail.add(stone);
    let info = |_: Item| (1.0, 0.0, 0);
    let k = make_kit_with_view(&avail, &info, &view);
    assert_eq!(k.cut, flint, "all-game, smoothed toward the nominal mix, still takes the flint blade");

    view.harvest = [0.0; 5];
    view.pop = 17.0;
    let h0 = harvest_for(&view, &dg);
    let pop = view.pop.max(1.0);
    assert_eq!(h0.map(f32::to_bits), NOMINAL_MIX.map(|x| x * pop).map(f32::to_bits));
}

/// No two of these share a `cut_q` (stone 0.42, bone 0.43, copper 0.48, iron 0.64, flint 0.76,
/// steel 0.78, obsidian 0.82), so equal food cannot be asserted by pairing materials. Food per
/// unit of `cut_q` is the same for all of them; labour follows durability and is not.
#[test]
fn knife_food_is_cut_q_and_has_no_fight() {
    let view = BandView::nominal();
    let info = |_: Item| (1.0, 0.0, 0);
    let mats = [Mat::Flint, Mat::Stone, Mat::Copper, Mat::Iron, Mat::Obsidian, Mat::Steel, Mat::Bone];
    let mut qs: Vec<f32> = Vec::new();
    for &m in &mats {
        let c = cut_q(m);
        assert!(qs.iter().all(|&q| (q - c).abs() > 1e-6), "{m:?} shares cut_q {c}");
        qs.push(c);
    }
    let mut ratio = None;
    let mut labours = Vec::new();
    for &m in &mats {
        let it = item(Form::Edge, m);
        let mut k = Kit::default();
        k.cut = it;
        let fx = tool_effects(&view, &k, 1, it, &info, 0);
        assert_eq!(fx.fight, 0.0, "{m:?} in the cut slot is not a weapon");
        let r = fx.food / cut_q(m);
        match ratio {
            None => ratio = Some(r),
            Some(r0) => assert!((r - r0).abs() < 1e-4, "{m:?} food/cut_q {r} vs {r0}"),
        }
        labours.push(fx.labour);
    }
    assert!(labours.iter().any(|l| (l - labours[0]).abs() > 1e-6), "labour follows toughness: {labours:?}");
    // the same edge, asked as a hunt weapon, does fight; asked as a knife, it does not
    let flint = item(Form::Edge, Mat::Flint);
    let mut both = Kit::default();
    both.hunt = flint;
    both.cut = flint;
    let as_knife = tool_effects(&view, &both, 1, flint, &info, 0);
    let as_hunt = tool_effects(&view, &both, 0, flint, &info, 0);
    assert_eq!(as_knife.fight, 0.0);
    assert!((as_hunt.fight - 0.2 * edge_q(Mat::Flint)).abs() < 1e-6);
}

#[test]
fn flint_knife_beats_stone_and_is_optimal() {
    use crate::band::make_kit;
    let flint = item(Form::Edge, Mat::Flint);
    let stone = item(Form::Edge, Mat::Stone);
    let mut a = ItemSet::default();
    a.add(flint);
    a.add(stone);
    let k = make_kit(&a);
    assert_eq!(k.cut, flint, "flint feeds a band better than a stone blade");
    assert_eq!(k.hunt, flint, "a lone pair of edges: the same blade is the hunt weapon");
    let view = BandView::nominal();
    let info = |_: Item| (1.0f32, 0.0, 0u8);
    let mut best = f32::NEG_INFINITY;
    let mut best_it = NONE;
    for it in [flint, stone] {
        let cur = slot_score(&view, &k, 1, it, &info).0;
        if cur > best {
            best = cur;
            best_it = it;
        }
    }
    assert_eq!(best_it, k.cut, "the cut slot maximises food - price * labour on the final kit");
}

#[test]
fn lone_flint_edge_multipliers_and_mats() {
    use crate::band::make_kit;
    let flint = item(Form::Edge, Mat::Flint);
    let mut s = ItemSet::default();
    s.add(flint);
    let k = make_kit(&s);
    assert_eq!((k.hunt, k.cut), (flint, flint));
    let c = cut_q(Mat::Flint);
    let game = (1.0 + 1.4 * 0.2 * c) * (1.0 + 0.35 * c);
    let farm = 0.5 + 0.35 * c;
    assert_eq!(k.game_m.to_bits(), game.to_bits());
    assert_eq!(k.farm_m.to_bits(), farm.to_bits());

    let view = BandView::nominal();
    let info = |_: Item| (1.0, 0.0, 0);
    let flint_i = Mat::Flint as u8;
    let copper_i = Mat::Copper as u8;
    let mask = (1u32 << flint_i) | (1u32 << copper_i);
    let mut held = Kit::default();
    held.cut = flint;
    let fx = tool_effects(&view, &held, 1, flint, &info, mask);
    assert_eq!(fx.mats[0], (flint_i, 0.06));
    assert_eq!(fx.mats[1], (copper_i, 0.02));
    assert_eq!(fx.mats[2], (0, 0.0));
    let mut at = view;
    at.at_hand = 1u32 << flint_i;
    let fx = tool_effects(&at, &held, 1, flint, &info, mask);
    assert_eq!(fx.mats[0], (copper_i, 0.02));
    assert_eq!(fx.mats[1], (0, 0.0));
}

fn set_pay(k: &mut Kit, slot: usize, it: Item) {
    match slot {
        0 => k.hunt = it,
        1 => k.cut = it,
        4 => k.axe = it,
        10 => k.point = it,
        11 => k.boat = it,
        12 => k.net = it,
        13 => k.trap = it,
        14 => k.bow = it,
        15 => k.shield = it,
        16 => k.outrigger = it,
        18 => k.plank = it,
        22 => k.ship = it,
        _ => {}
    }
}

fn accepts(slot: usize, it: Item) -> bool {
    let f = form(it);
    matches!(
        (slot, f),
        (0, Form::Spear) | (0, Form::Point) | (0, Form::Edge)
            | (1, Form::Edge)
            | (10, Form::Point)
            | (4, Form::Axe)
            | (12, Form::Net)
            | (13, Form::Trap)
            | (11, Form::Boat)
            | (14, Form::Bow)
            | (15, Form::Shield)
            | (16, Form::Outrigger)
            | (18, Form::Plank)
            | (22, Form::Ship)
    )
}

/// The score `slot_score` used to compute by rebuilding the kit. Kept here so a cheaper
/// evaluation has to match it bit for bit, including the upkeep association.
fn slow_score<F: Fn(Item) -> (f32, f32, u8)>(view: &BandView, kit: &Kit, slot: usize, it: Item, info: &F) -> (f32, f32) {
    let (_sk, ql, d) = info(it);
    if matches!(slot, 0 | 1 | 10 | 4 | 12 | 13) {
        let food_of = |user: &[f32; NFORM]| {
            let mut empty = *kit;
            set_pay(&mut empty, slot, NONE);
            let ye = kit_yields(&empty, info, user);
            let mut with = empty;
            set_pay(&mut with, slot, it);
            let yw = kit_yields(&with, info, user);
            let dg = yw.digest;
            let sum: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
            let h = if sum > HARVEST_EPS { smoothed(view) } else { NOMINAL_MIX.map(|x| x * view.pop.max(1.0)) };
            let mut food = 0.0;
            for r in 0..5 {
                let (m1, m0) = (mult(&yw, r), mult(&ye, r));
                food += h[r] * dg[r] * (m1 - m0) / m0;
            }
            food
        };
        let food = food_of(&view.user);
        let food_full = food_of(&[1.0; NFORM]);
        let labour = tool_upkeep(view.upkeep_rate, it, d, sealed_factor(it, &with_slot(kit, slot, it), pitch_in_use(&with_slot(kit, slot, it), view.wet)));
        let pop = view.pop.max(1.0);
        let units = labour * LABOUR_SHARE * pop;
        let price = view.food.max(1e-6) / (WORK_SHARE * LABOUR_SHARE * pop);
        let cost = price * units;
        (food - cost, food_full - cost)
    } else {
        let m = mat(it);
        let up = KIT_UPKEEP * (1.15 - durability(m)) * (1.0 + 0.3 * d as f32);
        let qv = match slot {
            11 | 16 | 18 | 22 => boat_q(m),
            14 | 15 => m.props().tough + 1.0,
            _ => 0.0,
        };
        let gain = qv * (1.0 + QUALITY_YIELD * ql);
        let uf = user_factor(view.user[form(it) as usize]);
        (gain * uf - up, gain - up)
    }
}

fn with_slot(kit: &Kit, slot: usize, it: Item) -> Kit {
    let mut with = *kit;
    set_pay(&mut with, slot, it);
    with
}

const CHOOSE: [usize; 12] = [0, 1, 10, 4, 12, 13, 11, 14, 15, 16, 18, 22];

fn trial_j(slot: usize) -> usize {
    match slot {
        0 => 0,
        1 => 1,
        10 => 2,
        4 => 3,
        11 => 4,
        12 => 5,
        13 => 6,
        14 => 7,
        15 => 8,
        16 => 9,
        18 => 10,
        22 => 11,
        _ => 0,
    }
}

/// The two-pass choice as it was when every candidate rebuilt a kit, trials on pass 2 only.
fn slow_make<F: Fn(Item) -> (f32, f32, u8)>(avail: &ItemSet, info: &F, view: &BandView) -> Kit {
    let mut k = Kit::default();
    let mut best = [0f32; 24];
    for it in avail.iter() {
        let (f, m) = (form(it), mat(it));
        let mut upd = |slot: usize, s: f32, tgt: &mut Item| {
            if s > best[slot] {
                best[slot] = s;
                *tgt = it;
            }
        };
        match f {
            Form::Shaft => upd(2, 1.0, &mut k.dig),
            Form::Vessel => upd(3, vessel_q(m), &mut k.vessel),
            Form::Garment => upd(5, garment_q(m), &mut k.garment),
            Form::Fire => upd(6, m.props().fuel_t, &mut k.fire),
            Form::Kiln => upd(7, m.props().refractory, &mut k.kiln),
            Form::Cord => upd(8, m.props().tough + 1.0, &mut k.cord),
            Form::Hammer => upd(9, m.props().hard, &mut k.hammer),
            Form::Sail => upd(17, m.props().tough + if m == Mat::Fibre { 0.0 } else { 10.0 }, &mut k.sail),
            Form::Frame => upd(19, vessel_q(m) + 0.1, &mut k.frame),
            Form::Ornament => upd(20, m.props().hard + 1.0 + 10.0 * m.props().prized, &mut k.ornament),
            Form::Jar if m == Mat::Grain => k.ale = it,
            Form::Jar => upd(21, vessel_q(m), &mut k.jar),
            Form::Mark => upd(23, durability(m) + if m == Mat::Ceramic { 1.0 } else { 0.0 }, &mut k.mark),
            Form::Raw if m == Mat::Pitch => k.pitch = it,
            Form::Cooked => match m {
                Mat::Plants => k.cooked[0] = it,
                Mat::Meat => k.cooked[1] = it,
                Mat::Fish => k.cooked[2] = it,
                Mat::Grain => k.cooked_grain = it,
                _ => {}
            },
            _ => {}
        }
    }
    for pass in 0..2 {
        for &slot in &CHOOSE {
            let mut best_cur = f32::NEG_INFINITY;
            let mut best_it = NONE;
            let mut pot = [(f32::NEG_INFINITY, NONE); 2];
            for it in avail.iter() {
                if !accepts(slot, it) {
                    continue;
                }
                let (cur, full) = slow_score(view, &k, slot, it, info);
                if cur > best_cur {
                    best_cur = cur;
                    best_it = it;
                }
                if full > pot[0].0 {
                    pot[1] = pot[0];
                    pot[0] = (full, it);
                } else if full > pot[1].0 {
                    pot[1] = (full, it);
                }
            }
            set_pay(&mut k, slot, best_it);
            if pass == 1 {
                let ch = if pot[0].1 != best_it { pot[0] } else { pot[1] };
                k.trial[trial_j(slot)] = if best_it != NONE && ch.1 != NONE && ch.0 > best_cur { ch.1 } else { NONE };
            }
        }
    }
    // multipliers are applied the same way afterwards; the choice is the items and the trials
    k
}

fn kit_picks(k: &Kit) -> [Item; 24] {
    let mut v = [NONE; 24];
    v[0] = k.hunt;
    v[1] = k.cut;
    v[2] = k.point;
    v[3] = k.axe;
    v[4] = k.net;
    v[5] = k.trap;
    v[6] = k.boat;
    v[7] = k.bow;
    v[8] = k.shield;
    v[9] = k.outrigger;
    v[10] = k.plank;
    v[11] = k.ship;
    v[12..24].copy_from_slice(&k.trial);
    v
}

fn kit_rest(k: &Kit) -> [Item; 18] {
    [
        k.dig, k.vessel, k.garment, k.fire, k.kiln, k.cord, k.hammer, k.sail, k.frame, k.ornament, k.jar, k.mark, k.pitch, k.ale, k.cooked[0], k.cooked[1],
        k.cooked[2], k.cooked_grain,
    ]
}

/// A cheap score has to pick the same kit, and the same bits, as rebuilding yields per candidate.
#[test]
fn fast_choice_matches_full_yields_bits() {
    let mats = [Mat::Flint, Mat::Stone, Mat::Obsidian, Mat::Copper, Mat::Iron, Mat::Bone, Mat::Wood, Mat::Fibre, Mat::Hide];
    let forms = [
        Form::Edge, Form::Point, Form::Spear, Form::Axe, Form::Net, Form::Trap, Form::Bow, Form::Shield, Form::Boat, Form::Shaft, Form::Vessel,
        Form::Cord, Form::Hammer, Form::Cooked, Form::Jar, Form::Raw,
    ];
    let mut avail = ItemSet::default();
    let mut rows: Vec<(Item, f32, f32, u8)> = Vec::new();
    let mut n = 0u32;
    for f in forms {
        for &m in &mats {
            if f == Form::Cooked && !matches!(m, Mat::Plants | Mat::Meat | Mat::Fish | Mat::Grain) {
                continue;
            }
            if f == Form::Raw && m != Mat::Pitch {
                continue;
            }
            if f == Form::Jar && m == Mat::Grain {
                continue;
            }
            let it = item(f, m);
            avail.add(it);
            // distinct skill, quality and depth so a reassociated score cannot tie by accident
            let sk = ((n * 17) % 11) as f32 / 10.0;
            let ql = ((n * 3) % 7) as f32 / 10.0;
            let d = (n % 4) as u8;
            rows.push((it, sk, ql, d));
            n += 1;
        }
    }
    avail.add(item(Form::Jar, Mat::Ceramic));
    rows.push((item(Form::Jar, Mat::Ceramic), 0.4, 0.2, 1));
    avail.add(item(Form::Sail, Mat::Fibre));
    rows.push((item(Form::Sail, Mat::Fibre), 1.0, 0.0, 0));
    let info = |it: Item| rows.iter().find(|r| r.0 == it).map(|r| (r.1, r.2, r.3)).unwrap_or((1.0, 0.0, 0));
    let views = [
        BandView::nominal(),
        BandView { harvest: [0.0; 5], food: 0.0, pop: 8.0, upkeep_rate: 0.02, wet: true, at_hand: 0, user: [1.0; NFORM] },
        BandView { harvest: [2.0, 40.0, 1.0, 5.0, 0.0], food: 30.0, pop: 40.0, upkeep_rate: 0.02, wet: true, at_hand: 0, user: [1.0; NFORM] },
        BandView { harvest: [10.0, 0.0, 25.0, 0.0, 8.0], food: 12.0, pop: 1.0, upkeep_rate: 0.05, wet: false, at_hand: 0, user: [1.0; NFORM] },
        BandView { harvest: [0.0, 0.0, 0.0, 0.0, 0.0], food: 3.0, pop: 200.0, upkeep_rate: 0.02, wet: true, at_hand: 0, user: [1.0; NFORM] },
        BandView {
            harvest: [4.0, 4.0, 4.0, 4.0, 1.0],
            food: 20.0,
            pop: 25.0,
            upkeep_rate: 0.02,
            wet: true,
            at_hand: 0,
            user: {
                let mut u = [0.2f32; NFORM];
                u[Form::Edge as usize] = 0.9;
                u[Form::Bow as usize] = 0.0;
                u[Form::Boat as usize] = 1.0;
                u
            },
        },
    ];
    for view in &views {
        let fast = make_kit_with_view(&avail, &info, view);
        let slow = slow_make(&avail, &info, view);
        assert_eq!(kit_picks(&fast), kit_picks(&slow), "payoff choice diverged");
        assert_eq!(kit_rest(&fast), kit_rest(&slow), "non-payoff choice diverged");
        assert_eq!((fast.pitch, fast.ale), (slow.pitch, slow.ale));
        for &slot in &CHOOSE {
            for it in avail.iter() {
                if !accepts(slot, it) {
                    continue;
                }
                let (a, b) = slot_score(view, &slow, slot, it, &info);
                let (c, d) = slow_score(view, &slow, slot, it, &info);
                assert_eq!(a.to_bits(), c.to_bits(), "slot {slot} discounted");
                assert_eq!(b.to_bits(), d.to_bits(), "slot {slot} full");
            }
        }
    }
}

/// Wet, with a hull and pitch, so pitch is in use. The only hunting tools are the stone and
/// flint blades, so a cut candidate can be the same item as `k.hunt`. That blade is glued.
#[test]
fn cut_slot_glues_a_blade_shared_with_hunt() {
    let flint = item(Form::Edge, Mat::Flint);
    let stone = item(Form::Edge, Mat::Stone);
    let hull = item(Form::Boat, Mat::Wood);
    let tar = item(Form::Raw, Mat::Pitch);
    let mut avail = ItemSet::default();
    for it in [flint, stone, hull, tar] {
        avail.add(it);
    }
    let info = |_: Item| (1.0f32, 0.0, 0u8);
    let view = BandView { harvest: [2.0, 40.0, 1.0, 5.0, 0.0], food: 30.0, pop: 40.0, upkeep_rate: 0.02, wet: true, at_hand: 0, user: [1.0; NFORM] };
    let fast = make_kit_with_view(&avail, &info, &view);
    let slow = slow_make(&avail, &info, &view);
    assert_eq!(kit_picks(&fast), kit_picks(&slow), "payoff choice diverged");
    assert_eq!(kit_rest(&fast), kit_rest(&slow), "non-payoff choice diverged");
    assert_eq!((fast.pitch, fast.ale), (slow.pitch, slow.ale));
    assert!(pitch_in_use(&fast, view.wet), "a hull and pitch on a wet view");
    assert_eq!((fast.boat, fast.pitch), (hull, tar));
    assert!(fast.hunt == flint || fast.hunt == stone, "the hunting tool is one of the blades");
    for &slot in &CHOOSE {
        for it in [flint, stone] {
            if !accepts(slot, it) {
                continue;
            }
            let (a, b) = slot_score(&view, &slow, slot, it, &info);
            let (c, d) = slow_score(&view, &slow, slot, it, &info);
            assert_eq!(a.to_bits(), c.to_bits(), "slot {slot}");
            assert_eq!(b.to_bits(), d.to_bits(), "slot {slot} full");
        }
    }
    // The cut-slot score of the hunting blade is food minus `tool_upkeep(..., PITCH_GLUE)`,
    // not the bare upkeep. `food_sealed(slot, pitch)` charged the bare rate here.
    let it = fast.hunt;
    let (_, _, d) = info(it);
    let mut empty = fast;
    set_pay(&mut empty, 1, NONE);
    let ye = kit_yields(&empty, &info, &view.user);
    let mut with = empty;
    set_pay(&mut with, 1, it);
    let yw = kit_yields(&with, &info, &view.user);
    let dg = yw.digest;
    let sum: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
    assert!(sum > HARVEST_EPS);
    let h = smoothed(&view);
    let mut food = 0.0;
    for r in 0..5 {
        let (m1, m0) = (mult(&yw, r), mult(&ye, r));
        food += h[r] * dg[r] * (m1 - m0) / m0;
    }
    let cost = |labour: f32| {
        let pop = view.pop.max(1.0);
        let units = labour * LABOUR_SHARE * pop;
        let price = view.food.max(1e-6) / (WORK_SHARE * LABOUR_SHARE * pop);
        price * units
    };
    let glue = tool_upkeep(view.upkeep_rate, it, d, PITCH_GLUE);
    let bare = tool_upkeep(view.upkeep_rate, it, d, 1.0);
    let (cur, full) = slot_score(&view, &fast, 1, it, &info);
    // user is 1 and recipe skill no longer discounts, so cur and full are the same food
    assert_eq!(cur.to_bits(), (food - cost(glue)).to_bits(), "cut slot charges PITCH_GLUE");
    assert_eq!(full.to_bits(), (food - cost(glue)).to_bits());
    assert_ne!(full.to_bits(), (food - cost(bare)).to_bits(), "bare upkeep is a different charge");
}

/// (skills step 2) A v15 save has no user-skill column. Load fills each form from the best
/// known recipe of that output, then the world runs on for 20 years.
#[test]
fn v15_fixture_loads_fills_use_skill_and_runs() {
    use crate::effects::LC_PRIOR;
    let bytes = include_bytes!("../tests/fixtures/v15_seed9_y150_64x40.civsave");
    let (mut w, _, info) = World::load_bytes(bytes).expect("v15 save loads");
    assert_eq!(info.format_version, 15);
    assert_eq!(info.migrated_from, Some(15));
    assert!(w.bands.iter().all(|b| b.use_skill.len() == NFORM), "every band has one skill per form");
    assert!(w.bands.iter().all(|b| b.learn_curve == LC_PRIOR && b.last_fight_switch == u32::MAX && b.beliefs.is_empty()));
    assert!(w.bands.iter().all(|b| b.fight_calib == [0.0; 2] && b.raid == [0.0; 4] && b.raid_share == 0.0));
    let b = w
        .bands
        .iter()
        .find(|b| {
            b.known.iter().any(|kn| {
                w.reg.recipes.get(kn.recipe as usize).is_some_and(|r| valid_item(r.output) && form(r.output) == Form::Edge)
            })
        })
        .expect("a band knows an Edge recipe");
    let best = b
        .known
        .iter()
        .filter_map(|kn| {
            let r = w.reg.recipes.get(kn.recipe as usize)?;
            (valid_item(r.output) && form(r.output) == Form::Edge).then_some(kn.skill)
        })
        .fold(0.0f32, f32::max);
    assert!(best > 0.0, "the Edge recipe has some skill");
    assert_eq!(b.use_skill[Form::Edge as usize], best);
    let t0 = w.tick;
    for _ in 0..80 {
        w.step();
    }
    assert_eq!(w.tick, t0 + 80);
    assert!(w.bands.iter().all(|b| b.use_skill.len() == NFORM));
}

fn same_skill_fields(a: &World, b: &World) {
    use crate::band::Belief;
    assert_eq!(a.bands.len(), b.bands.len());
    let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    let belief = |xs: &[Belief]| {
        xs.iter()
            .map(|e| {
                (
                    e.item,
                    e.slot,
                    e.flags,
                    e.n.to_bits(),
                    e.mean.to_bits(),
                    e.var.to_bits(),
                    e.fail.to_bits(),
                    e.labour.to_bits(),
                    e.fight_n.to_bits(),
                    e.fight_mean.to_bits(),
                    e.fight_var.to_bits(),
                )
            })
            .collect::<Vec<_>>()
    };
    for (x, y) in a.bands.iter().zip(&b.bands) {
        assert_eq!(bits(&x.use_skill), bits(&y.use_skill));
        assert_eq!(x.learn_curve.to_bits(), y.learn_curve.to_bits());
        assert_eq!(x.fight_calib.map(f32::to_bits), y.fight_calib.map(f32::to_bits));
        assert_eq!(x.last_fight_switch, y.last_fight_switch);
        assert_eq!(x.raid.map(f32::to_bits), y.raid.map(f32::to_bits));
        assert_eq!(x.raid_share.to_bits(), y.raid_share.to_bits());
        assert_eq!(x.year_food.to_bits(), y.year_food.to_bits());
        assert_eq!(x.year_raid_food.to_bits(), y.year_raid_food.to_bits());
        assert_eq!(belief(&x.beliefs), belief(&y.beliefs));
    }
}

/// (skills step 2) Layout 3 round-trips the new columns in binary and JSON, and the loaded
/// worlds keep stepping in lockstep with the one that was saved.
#[test]
fn v16_fields_round_trip_continues_identically() {
    use crate::band::Belief;
    assert_eq!(crate::save::FORMAT_VERSION, 16);
    let mut a = world();
    for _ in 0..8 {
        a.step();
    }
    a.bands[0].use_skill[Form::Edge as usize] = 0.42;
    a.bands[0].use_skill[Form::Bow as usize] = 0.2;
    a.bands[0].learn_curve = 1.7;
    a.bands[0].fight_calib = [0.3, 0.4];
    a.bands[0].last_fight_switch = 17;
    a.bands[0].raid = [2.0, 0.5, 0.25, 1.0];
    a.bands[0].raid_share = 0.15;
    a.bands[0].year_food = 3.5;
    a.bands[0].year_raid_food = 0.75;
    a.bands[0].beliefs = vec![
        Belief {
            item: item(Form::Edge, Mat::Flint),
            slot: 1,
            flags: 2,
            n: 4.0,
            mean: 0.5,
            var: 0.1,
            fail: 0.2,
            labour: 0.3,
            fight_n: 1.0,
            fight_mean: 0.4,
            fight_var: 0.05,
        },
        Belief {
            item: item(Form::Spear, Mat::Wood),
            slot: 3,
            flags: 0,
            n: 2.0,
            mean: 0.8,
            var: 0.2,
            fail: 0.0,
            labour: 0.1,
            fight_n: 3.0,
            fight_mean: 0.9,
            fight_var: 0.07,
        },
    ];
    assert_eq!(a.bands[0].beliefs.len(), 2);
    assert!(a.bands[1].beliefs.is_empty(), "a second band keeps zero beliefs");
    assert_eq!(a.bands[1].use_skill.len(), NFORM);
    let bytes = a.save_bytes(None, "test", "now").unwrap();
    let js = a.save_json(None, "test", "now").unwrap();
    let (mut bin, _, info) = World::load_bytes(&bytes).unwrap();
    assert_eq!(info.format_version, 16);
    let (mut json, _, info_j) = World::load_bytes(js.as_bytes()).unwrap();
    assert_eq!(info_j.format_version, 16);
    same_skill_fields(&a, &bin);
    same_skill_fields(&a, &json);
    assert_eq!(bin.bands[0].beliefs.len(), 2);
    assert!(bin.bands[1].beliefs.is_empty());
    assert!(json.bands[1].beliefs.is_empty());
    for _ in 0..8 {
        a.step();
        bin.step();
        json.step();
    }
    assert_eq!(a.state_hash(), bin.state_hash());
    assert_eq!(a.state_hash(), json.state_hash());
    same_skill_fields(&a, &bin);
    same_skill_fields(&a, &json);
}

fn no_profile(f: Form) -> bool {
    matches!(
        f,
        Form::Raw
            | Form::Vessel
            | Form::Garment
            | Form::Cooked
            | Form::Fire
            | Form::Kiln
            | Form::Bellows
            | Form::Bloom
            | Form::Frame
            | Form::Ornament
            | Form::Jar
            | Form::Mark
            | Form::Mould
            | Form::Crucible
            | Form::Cloth
            | Form::Loom
    )
}

/// Transfer is symmetric, 1 on the diagonal, and 0 wherever either form has no motion profile.
#[test]
fn transfer_is_symmetric_and_profiles_only() {
    let t = transfer_table();
    for f in 0..NFORM {
        for g in 0..NFORM {
            assert_eq!(t[f][g].to_bits(), t[g][f].to_bits(), "T({f},{g})");
            if f == g {
                assert_eq!(t[f][g], 1.0);
            } else {
                assert!(t[f][g] >= 0.0 && t[f][g] <= USE_TRANSFER, "T({f},{g}) = {}", t[f][g]);
            }
        }
    }
    for f in ALL_FORMS {
        if no_profile(f) {
            for g in 0..NFORM {
                if g != f as usize {
                    assert_eq!(t[f as usize][g], 0.0, "{f:?} has no profile");
                }
            }
        }
    }
    // Net and Trap train the same motion, so the cosine is 1 and T is USE_TRANSFER.
    assert_eq!(t[Form::Net as usize][Form::Trap as usize], USE_TRANSFER);
    let mut sk = vec![0.0; NFORM];
    sk[Form::Axe as usize] = 1.0;
    let u = u_eff(&sk);
    assert_eq!(u[Form::Axe as usize], 1.0);
    assert!(u[Form::Edge as usize] > 0.0 && u[Form::Edge as usize] < 1.0, "axe transfers to edge");
    assert_eq!(u[Form::Vessel as usize], 0.0);
    assert_eq!(u_eff(&[]), [1.0; NFORM]);
}

/// A band that barely handles an edge gets less from it than the same kit at full skill.
#[test]
fn low_edge_skill_lowers_cut_yield() {
    let flint = item(Form::Edge, Mat::Flint);
    let mut avail = ItemSet::default();
    avail.add(flint);
    let info = |_: Item| (1.0f32, 0.0, 0u8);
    let full = make_kit_with_view(&avail, &info, &BandView::nominal());
    let mut user = [1.0f32; NFORM];
    user[Form::Edge as usize] = 0.0;
    let view = BandView { user, ..BandView::nominal() };
    let low = make_kit_with_view(&avail, &info, &view);
    assert_eq!((low.hunt, low.cut), (flint, flint));
    assert!(low.game_m < full.game_m, "game {} vs {}", low.game_m, full.game_m);
    assert!(low.farm_m < full.farm_m, "farm {} vs {}", low.farm_m, full.farm_m);
    assert_eq!(full.game_m.to_bits(), {
        let c = cut_q(Mat::Flint);
        ((1.0 + 1.4 * 0.2 * c) * (1.0 + 0.35 * c)).to_bits()
    });
}

/// The novice factor averages out, shrinks as skill rises, and is exactly 1 at full skill.
#[test]
fn novice_spread_is_mean_preserving_and_shrinks() {
    let flint = item(Form::Edge, Mat::Flint);
    let mut k = Kit::default();
    k.hunt = flint;
    k.cut = flint;
    let qual = |_: Item| 0.0;
    let quiet = food_mults(&k, &[1.0; NFORM], qual, |_| 1.0);
    // the kit's own multipliers, fitted at u = 1, are that same number
    let mut avail = ItemSet::default();
    avail.add(flint);
    let fitted = make_kit_with_view(&avail, &|_| (1.0, 0.0, 0), &BandView::nominal());
    assert_eq!(quiet[1].to_bits(), fitted.game_m.to_bits());
    assert_eq!(quiet[4].to_bits(), fitted.farm_m.to_bits());

    let spread = |u_edge: f32| {
        let mut user = [1.0f32; NFORM];
        user[Form::Edge as usize] = u_edge;
        let mean = food_mults(&k, &user, qual, |_| 1.0);
        let n = 4000u32;
        let mut acc = 0.0f32;
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for tick in 0..n {
            let m = food_mults(&k, &user, qual, |slot| novice_factor(&k, &user, 9, 3, tick, slot));
            acc += m[1];
            lo = lo.min(m[1]);
            hi = hi.max(m[1]);
            if u_edge == 1.0 {
                assert_eq!(m[1].to_bits(), mean[1].to_bits(), "tick {tick}");
                assert_eq!(m[4].to_bits(), mean[4].to_bits());
            }
        }
        (acc / n as f32, hi - lo, mean[1])
    };
    let (avg0, width0, mean0) = spread(0.0);
    let (avg9, width9, _) = spread(0.9);
    let (_, width1, _) = spread(1.0);
    assert!((avg0 - mean0).abs() < 0.01, "mean {avg0} vs quiet {mean0}");
    assert!((avg9 - spread(0.9).2).abs() < 0.01);
    assert!(width0 > width9, "spread shrinks with skill: {width0} vs {width9}");
    assert_eq!(width1, 0.0);
}

/// At full skill the economy's noisy multipliers are the kit's, so the harvest does not
/// depend on the tick. A novice's does.
#[test]
fn economy_novice_spread_tracks_skill() {
    let mut w = world();
    let flint = item(Form::Edge, Mat::Flint);
    let mut avail = ItemSet::default();
    avail.add(flint);
    let kit = crate::band::make_kit(&avail);
    assert!(kit.game_m > 1.0);
    {
        let b = &mut w.bands[0];
        b.use_skill = vec![1.0; NFORM];
        b.d.kit = kit;
    }
    let mw = w.map.w;
    let ci = w.bands[0].cell(mw);
    w.game[ci] = 20.0;
    w.plants[ci] = 20.0;
    w.tick = 0;
    let h0 = w.economy(&w.bands[0], 0).h;
    w.tick = 100;
    let h1 = w.economy(&w.bands[0], 0).h;
    assert_eq!(h0.map(f32::to_bits), h1.map(f32::to_bits), "u = 1 is not noisy");
    w.bands[0].use_skill = vec![0.0; NFORM];
    w.tick = 0;
    let n0 = w.economy(&w.bands[0], 0).h;
    w.tick = 1;
    let n1 = w.economy(&w.bands[0], 0).h;
    assert_ne!(n0.map(f32::to_bits), n1.map(f32::to_bits), "a novice's harvest moves with the tick");
}

fn knife_kit(it: Item, edge_u: f32) -> Kit {
    let mut avail = ItemSet::default();
    avail.add(it);
    let mut user = [1.0f32; NFORM];
    user[Form::Edge as usize] = edge_u;
    let view = BandView { user, ..BandView::nominal() };
    make_kit_with_view(&avail, &|_| (1.0, 0.0, 0), &view)
}

/// An iron knife and a steel knife are the same form, so the band keeps its edge skill.
/// The cut yield does not fall from the user factor; a reset skill would cut it.
#[test]
fn iron_to_steel_keeps_knife_skill() {
    let mut sk = vec![0.0; NFORM];
    sk[Form::Edge as usize] = 0.8;
    let before = u_eff(&sk);
    let after = u_eff(&sk);
    assert_eq!(before[Form::Edge as usize].to_bits(), after[Form::Edge as usize].to_bits());
    assert_eq!(before[Form::Edge as usize], 0.8);
    let u = before[Form::Edge as usize];
    let iron = knife_kit(item(Form::Edge, Mat::Iron), u);
    let steel = knife_kit(item(Form::Edge, Mat::Steel), u);
    let dropped = knife_kit(item(Form::Edge, Mat::Steel), 0.0);
    assert_eq!((iron.cut, steel.cut), (item(Form::Edge, Mat::Iron), item(Form::Edge, Mat::Steel)));
    assert!(steel.game_m >= iron.game_m, "game {} vs {}", steel.game_m, iron.game_m);
    assert!(steel.farm_m >= iron.farm_m, "farm {} vs {}", steel.farm_m, iron.farm_m);
    assert!(steel.game_m > dropped.game_m, "reset skill would cut game: {} vs {}", steel.game_m, dropped.game_m);
    assert!(steel.farm_m > dropped.farm_m, "reset skill would cut farm: {} vs {}", steel.farm_m, dropped.farm_m);
    assert_eq!(user_factor(u).to_bits(), user_factor(u_eff(&sk)[Form::Edge as usize]).to_bits());
}

/// A first bow is not stored skill. It starts at the transfer from forms the band
/// already has (point shares aim; a spear alone does not, its profile misses the bow).
#[test]
fn first_bow_starts_at_its_transfer() {
    let mut sk = vec![0.0; NFORM];
    sk[Form::Spear as usize] = 0.9;
    sk[Form::Point as usize] = 0.8;
    assert_eq!(sk[Form::Bow as usize], 0.0);
    let t = transfer_table();
    let mut m = 0.0f32;
    for f in 0..NFORM {
        m = m.max(t[f][Form::Bow as usize] * sk[f]);
    }
    assert!(m > 0.0, "throw/aim overlap with the point reaches the bow");
    assert_eq!(t[Form::Spear as usize][Form::Bow as usize], 0.0);
    let u = u_eff(&sk);
    assert_eq!(u[Form::Bow as usize].to_bits(), m.to_bits());
    let mut practised = sk.clone();
    practise_use(&mut practised, 1u32 << Form::Bow as u32, 0.0, 3);
    let expect = m + USE_PRACTICE * (1.0 - m);
    assert_eq!(practised[Form::Bow as usize].to_bits(), expect.to_bits());
    assert!(practised[Form::Bow as usize] > USE_PRACTICE, "not a start from zero");
}

/// Founders are at USE_FOUNDER on whatever forms their starting kit holds, and a
/// founding band's first season of a new form is floored there before practice.
#[test]
fn founders_start_at_use_founder() {
    let w = world();
    assert!(!w.bands.is_empty());
    for b in &w.bands {
        assert_eq!(b.gen, 0);
        assert_eq!(b.use_skill.len(), NFORM);
        for &it in b.d.kit.used_items().iter() {
            if valid_item(it) {
                assert_eq!(b.use_skill[form(it) as usize], USE_FOUNDER, "{:?}", form(it));
            }
        }
        for f in 0..NFORM {
            let in_kit = b.d.kit.used_items().iter().any(|&it| valid_item(it) && form(it) as usize == f);
            if !in_kit {
                assert_eq!(b.use_skill[f], 0.0, "form {f} is not in the starting kit");
            }
        }
    }
    let mut sk = vec![0.0; NFORM];
    practise_use(&mut sk, 1u32 << Form::Edge as u32, 0.02, 0);
    let expect = USE_FOUNDER + USE_PRACTICE * (1.0 - USE_FOUNDER);
    assert_eq!(sk[Form::Edge as usize].to_bits(), expect.to_bits());
    assert_eq!(sk[Form::Bow as usize], 0.0);
}

/// A used form climbs toward 1. An unused one fades by USE_FADE_X * floss and stops at 0.
#[test]
fn practice_raises_and_fade_lowers() {
    let mut sk = vec![0.0; NFORM];
    sk[Form::Edge as usize] = 0.5;
    sk[Form::Axe as usize] = 0.4;
    sk[Form::Net as usize] = 0.001;
    let floss = 0.02;
    let before = sk.clone();
    practise_use(&mut sk, 1u32 << Form::Edge as u32, floss, 4);
    let ue = u_eff(&before);
    let mut edge = if before[Form::Edge as usize] > ue[Form::Edge as usize] { before[Form::Edge as usize] } else { ue[Form::Edge as usize] };
    edge += USE_PRACTICE * (1.0 - edge);
    assert_eq!(sk[Form::Edge as usize].to_bits(), edge.to_bits());
    assert!(sk[Form::Edge as usize] > 0.5 && sk[Form::Edge as usize] < 1.0);
    let axe = 0.4 - USE_FADE_X * floss;
    assert_eq!(sk[Form::Axe as usize].to_bits(), axe.to_bits());
    assert!(sk[Form::Axe as usize] < 0.4);
    assert_eq!(sk[Form::Net as usize], 0.0, "fade clamps at 0");
    // another season of the edge, still climbing, never past 1 from below
    let mut high = vec![0.0; NFORM];
    high[Form::Edge as usize] = 1.0;
    practise_use(&mut high, 1u32 << Form::Edge as u32, floss, 4);
    assert_eq!(high[Form::Edge as usize], 1.0);
}

/// The economy's form bitmask is exactly the forms of `used`, trial included.
#[test]
fn economy_used_forms_match_used_items() {
    let w = world();
    let ec = w.economy(&w.bands[0], 0);
    let mut bits = 0u32;
    for &it in &ec.used {
        assert!(valid_item(it));
        bits |= 1u32 << (form(it) as u32);
    }
    assert_eq!(ec.used_forms, bits);
    let mut seen = [0u32; NFORM];
    for &it in &ec.used {
        seen[form(it) as usize] += 1;
    }
    for (f, &n) in seen.iter().enumerate() {
        let on = ec.used_forms & (1u32 << f) != 0;
        assert_eq!(on, n > 0, "form {f} counted {n}");
    }
}

/// A daughter keeps 0.9 of the parent. That is above a novice, and above the one
/// lesson a tied partner can pick up from the same teacher.
#[test]
fn daughters_inherit_above_a_novice_partner() {
    let mut parent = vec![0.0; NFORM];
    parent[Form::Edge as usize] = 0.8;
    parent[Form::Spear as usize] = 0.5;
    let mut daughter = vec![0.0; NFORM];
    assert!(inherit_use(&mut daughter, &parent));
    assert_eq!(daughter[Form::Edge as usize].to_bits(), (0.8 * USE_INHERIT).to_bits());
    assert_eq!(daughter[Form::Spear as usize].to_bits(), (0.5 * USE_INHERIT).to_bits());
    assert_eq!(daughter[Form::Bow as usize], 0.0);
    assert!(daughter[Form::Edge as usize] > USE_NOVICE);
    let one_lesson = USE_TEACH_SHARE * 0.8;
    assert!(daughter[Form::Edge as usize] > one_lesson);
    let mut blank = vec![0.0; NFORM];
    assert!(!inherit_use(&mut blank, &[0.0; NFORM]));

    let mut w = world();
    let parent_id = w.bands[0].id;
    w.bands.truncate(1);
    w.bands[0].pop = 90;
    w.bands[0].use_skill = vec![0.0; NFORM];
    w.bands[0].use_skill[Form::Edge as usize] = 0.85;
    let before = w.counters.use_inherited_bands;
    let mut found = false;
    for _ in 0..12 {
        w.step();
        if let Some(d) = w.bands.iter().find(|b| b.parent == parent_id) {
            assert!(d.use_skill[Form::Edge as usize] > USE_NOVICE, "daughter edge {}", d.use_skill[Form::Edge as usize]);
            assert!(d.use_skill[Form::Edge as usize] < 1.0);
            found = true;
            break;
        }
    }
    assert!(found, "a crowded band should split");
    assert!(w.counters.use_inherited_bands > before);
}

/// Teaching moves the listener toward the teacher and never past them. A stranger
/// with no close tie learns nothing. A form the listener does not carry is skipped.
#[test]
fn teaching_stays_with_kin_and_close_ties() {
    let w = world();
    let mut src = w.bands[0].clone();
    let mut dst = w.bands[1].clone();
    src.start_skills();
    dst.start_skills();
    src.use_skill[Form::Edge as usize] = 0.9;
    src.use_skill[Form::Spear as usize] = 0.8;
    src.use_skill[Form::Bow as usize] = 0.7;
    dst.use_skill[Form::Edge as usize] = 0.2;
    dst.use_skill[Form::Spear as usize] = 0.2;
    dst.d.kit.cut = item(Form::Edge, Mat::Flint);
    dst.d.kit.hunt = item(Form::Spear, Mat::Wood);
    // bow is not in the kit, so it is not taught
    dst.lineage = src.lineage;
    dst.parent = src.id;
    dst.anc = [0, 0, 0];
    dst.born = w.tick;
    src.born = w.tick;
    assert!(src.kinship(&dst, w.tick) >= 0.2);

    let mut rng = Rng::new(11);
    let mut fx = Fx::default();
    for _ in 0..40 {
        let mut pair = Pair(&mut src, &mut dst);
        w.exchange(&mut pair, 0, 1, &mut rng, &mut fx);
    }
    assert!(dst.use_skill[Form::Edge as usize] > 0.2);
    assert!(dst.use_skill[Form::Spear as usize] > 0.2);
    assert!(dst.use_skill[Form::Edge as usize] <= src.use_skill[Form::Edge as usize]);
    assert!(dst.use_skill[Form::Spear as usize] <= src.use_skill[Form::Spear as usize]);
    assert_eq!(dst.use_skill[Form::Bow as usize], 0.0, "a form not in the kit is not taught");
    assert!(fx.counters.use_taught > 0);
    assert_eq!(src.use_skill[Form::Edge as usize], 0.9, "the teacher is unchanged");

    // already at the teacher's level: a lesson cannot go past
    dst.use_skill[Form::Edge as usize] = src.use_skill[Form::Edge as usize];
    let held = dst.use_skill[Form::Edge as usize];
    for _ in 0..15 {
        let mut pair = Pair(&mut src, &mut dst);
        w.exchange(&mut pair, 0, 1, &mut rng, &mut fx);
    }
    assert!(dst.use_skill[Form::Edge as usize] <= held);

    // stranger: different lineage, partner strength under CLOSE_TIE
    let mut stranger = dst.clone();
    stranger.lineage = src.lineage.wrapping_add(9);
    stranger.parent = 0;
    stranger.partners.clear();
    stranger.partners.push(Partner { id: src.id, strength: CLOSE_TIE * 0.5, ..Default::default() });
    stranger.use_skill[Form::Edge as usize] = 0.2;
    stranger.use_skill[Form::Spear as usize] = 0.2;
    let mut fx2 = Fx::default();
    for _ in 0..30 {
        let mut pair = Pair(&mut src, &mut stranger);
        w.exchange(&mut pair, 0, 1, &mut rng, &mut fx2);
    }
    assert_eq!(stranger.use_skill[Form::Edge as usize], 0.2);
    assert_eq!(stranger.use_skill[Form::Spear as usize], 0.2);
    assert_eq!(fx2.counters.use_taught, 0);

    // the same stranger, once the tie is close, can be taught
    stranger.partners[0].strength = CLOSE_TIE;
    let mut fx3 = Fx::default();
    let mut rose = false;
    for _ in 0..50 {
        let mut pair = Pair(&mut src, &mut stranger);
        w.exchange(&mut pair, 0, 1, &mut rng, &mut fx3);
        if stranger.use_skill[Form::Edge as usize] > 0.2 || stranger.use_skill[Form::Spear as usize] > 0.2 {
            rose = true;
            break;
        }
    }
    assert!(rose, "a close tie is taught");
    assert!(stranger.use_skill[Form::Edge as usize] <= src.use_skill[Form::Edge as usize]);
    assert!(stranger.use_skill[Form::Spear as usize] <= src.use_skill[Form::Spear as usize]);
    assert!(fx3.counters.use_taught > 0);
}

/// Year-end means are the mean use_skill of bands whose kit carries that form.
#[test]
fn yearstats_use_skill_matches_kit_means() {
    let mut w = world();
    for _ in 0..16 {
        w.step();
    }
    let h = w.history.last().unwrap();
    let mean = |f: Form| {
        let mut s = 0.0f64;
        let mut n = 0u32;
        for b in &w.bands {
            let has = b.d.kit.used_items().iter().any(|&it| valid_item(it) && form(it) == f);
            if has {
                s += b.use_skill.get(f as usize).copied().unwrap_or(0.0) as f64;
                n += 1;
            }
        }
        if n == 0 { 0.0 } else { (s / n as f64) as f32 }
    };
    assert_eq!(h.use_skill_edge.to_bits(), mean(Form::Edge).to_bits());
    assert_eq!(h.use_skill_spear.to_bits(), mean(Form::Spear).to_bits());
    assert_eq!(h.use_skill_bow.to_bits(), mean(Form::Bow).to_bits());
    assert_eq!(h.use_skill_net.to_bits(), mean(Form::Net).to_bits());
    assert_eq!(h.use_skill_boat.to_bits(), mean(Form::Boat).to_bits());
}
