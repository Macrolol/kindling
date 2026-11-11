//! (skills step 1) The economy pays what `tool_effects` says, and a food slot is the item
//! that maximises food minus the food its upkeep labour would have grown.
use crate::band::{edge_q, Band, Belief, Kit, Partner};
use crate::band::{KIT_TRIAL_P, KIT_UPKEEP, QUALITY_YIELD};
use crate::beliefs::{
    accept_testimony, belief_from_prior, blend_learn_curve, claim_fresh_writeoffs, evict_smallest, expected_multiplier, fade_unused, fold_learn_curve, food_prior, inherit_beliefs, mark_writeoffs, material_term, observe, pick_testimony, realised_multiplier, remember, technique_fail, update_belief, FoodPrior, LearnState, BELIEF_ADOPT_N,
    BELIEF_CAP, BELIEF_DROP_N, BELIEF_FAIL_N0, BELIEF_FADE, BELIEF_INHERIT_N, BELIEF_NMAX, BELIEF_TELL_P, BELIEF_VAR_MIN, BELIEF_WIDEN, FLAG_INHERITED, FLAG_OWN, FLAG_TOLD, FLAG_WO_FRESH, FLAG_WROTE_OFF, FLAG_YEAR, INFO_W, LC_RATE, PAYOFF_SLOT, PRIOR_CV,
    PRIOR_FAIL, PRIOR_N, PRIOR_N0, PRIOR_NOVEL, TELL_NMAX, TELL_W, TRIAL_SHARE, WRITE_OFF_SEASONS,
};
use crate::effects::{
    food_mults, harvest_for, kit_yields, make_kit_beliefs, make_kit_for, make_kit_scored, make_kit_with, make_kit_with_view, maker_fail_roll, maker_p_fail, maker_scrap_factor, maker_waste, motion_cos,
    novice_factor, novice_factor_u, pitch_in_use, recipe_info, sealed_factor, slot_score, slot_score_m, tool_effects, tool_upkeep, transfer_table, u_eff, user_factor, BandView, CHAIN_EPS,
    CHAIN_W_MIN, HARVEST_EPS, HARVEST_PRIOR, LABOUR_SHARE, LC_PRIOR, NOMINAL_MIX, NOVICE_SD, TECH_CARRY, USE_FADE_X, USE_FOUNDER, USE_INHERIT, USE_NOVICE, USE_PRACTICE,
    USE_TEACH_SHARE, USE_TRANSFER, WORK_SHARE,
};
use crate::knowledge::{closure, fill_chain, maker_skill, maker_tech_best, technique, Known, Recipe, Registry, Tech};
use crate::materials::*;
use crate::physics::{canonical, Action};
use crate::rng::{hash3, Rng};
use crate::world::fx::{Fx, Pair};
use crate::world::{band_user, belief_err_x1000, inherit_use, practise_use, Event, EventKind, CLOSE_TIE, PITCH_GLUE};
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
    let mut used = Vec::new();
    for &it in &charged {
        let depth = b.d.tk.via(it).map(|v| v.2).unwrap_or(0);
        let mask = b.d.tk.via(it).map(|v| v.1).unwrap_or(0);
        let sealed = sealed_factor(it, kit, pitch);
        let waste = maker_waste(b.d.tk.chain_m(it));
        sum_fx += tool_effects(&view, kit, slot_holding(kit, it), it, &info, mask).labour * waste;
        sum_wear += wear(w.params.upkeep, it, depth, sealed) * waste;
        used.push(it);
    }
    // D3 trials wear at sealed = 1. A failed food slot then wears its fallback, glued
    // against the original kit, and only if that item is not already in `used`.
    for j in 0..kit.trial.len() {
        let (inc, ch) = (kit.slot_item(j), kit.trial[j]);
        if ch == NONE || !used.contains(&inc) || used.contains(&ch) {
            continue;
        }
        let roll = hash3(w.cfg.seed ^ 0x7121_A15E_D3D3_0001, b.id as i64, (w.tick as i64) * 16 + j as i64);
        if (roll >> 40) as f32 / (1u64 << 24) as f32 >= KIT_TRIAL_P {
            continue;
        }
        let depth = b.d.tk.via(ch).map(|v| v.2).unwrap_or(0);
        let waste = maker_waste(b.d.tk.chain_m(ch));
        sum_fx += tool_upkeep(w.params.upkeep, ch, depth, 1.0) * waste;
        sum_wear += wear(w.params.upkeep, ch, depth, 1.0) * waste;
        used.push(ch);
    }
    for j in 0..12 {
        let it = kit.slot_item(j);
        if it == NONE || !used.contains(&it) {
            continue;
        }
        let pm = maker_p_fail(b.d.tk.chain_m(it));
        if pm <= 0.0 || maker_fail_roll(w.cfg.seed, b.id, w.tick, j) >= pm {
            continue;
        }
        if !matches!(j, 0 | 1 | 2 | 3 | 5 | 6) {
            continue;
        }
        let fb = kit.fallback[j];
        if fb == NONE || used.contains(&fb) {
            continue;
        }
        let depth = b.d.tk.via(fb).map(|v| v.2).unwrap_or(0);
        let sealed = sealed_factor(fb, kit, pitch);
        let waste = maker_waste(b.d.tk.chain_m(fb));
        sum_fx += tool_upkeep(w.params.upkeep, fb, depth, sealed) * waste;
        sum_wear += wear(w.params.upkeep, fb, depth, sealed) * waste;
        used.push(fb);
    }
    assert_eq!(used, ec.used, "practised list");
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
fn slow_score<F: Fn(Item) -> (f32, f32, u8)>(view: &BandView, kit: &Kit, slot: usize, it: Item, info: &F, m_item: f32) -> (f32, f32) {
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
        let price = view.food.max(1e-6) / (WORK_SHARE * LABOUR_SHARE * pop);
        // Same association as `price_of`: scale labour to person-seasons first, then by price.
        // Multiplying price by labour first is one ulp off on some slots.
        let cost = |lab: f32| {
            let units = lab * LABOUR_SHARE * pop;
            price * units
        };
        if m_item == 1.0 {
            let c = cost(labour);
            (food - c, food_full - c)
        } else {
            let pm = maker_p_fail(m_item);
            (food * (1.0 - pm) - cost(labour * maker_waste(m_item)), food_full - cost(labour))
        }
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
fn slow_make<F, M>(avail: &ItemSet, info: &F, view: &BandView, m_of: &M) -> Kit
where
    F: Fn(Item) -> (f32, f32, u8),
    M: Fn(Item) -> f32,
{
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
            let mut second_cur = f32::NEG_INFINITY;
            let mut second_it = NONE;
            let mut pot = [(f32::NEG_INFINITY, NONE); 2];
            for it in avail.iter() {
                if !accepts(slot, it) {
                    continue;
                }
                let (cur, full) = slow_score(view, &k, slot, it, info, m_of(it));
                if cur > best_cur {
                    second_cur = best_cur;
                    second_it = best_it;
                    best_cur = cur;
                    best_it = it;
                } else if it != best_it && cur > second_cur {
                    second_cur = cur;
                    second_it = it;
                }
                if full > pot[0].0 {
                    pot[1] = pot[0];
                    pot[0] = (full, it);
                } else if full > pot[1].0 {
                    pot[1] = (full, it);
                }
            }
            set_pay(&mut k, slot, best_it);
            // The second pass is the final context. Slots the fast path does not re-score
            // kept their first-pass runner-up, which matches this one when the inputs did not move.
            if pass == 1 {
                let ch = if pot[0].1 != best_it { pot[0] } else { pot[1] };
                k.trial[trial_j(slot)] = if best_it != NONE && ch.1 != NONE && ch.0 > best_cur { ch.1 } else { NONE };
                k.fallback[trial_j(slot)] = if best_it != NONE { second_it } else { NONE };
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
        let slow = slow_make(&avail, &info, view, &|_| 1.0);
        assert_eq!(kit_picks(&fast), kit_picks(&slow), "payoff choice diverged");
        assert_eq!(fast.fallback, slow.fallback, "fallback diverged");
        assert_eq!(kit_rest(&fast), kit_rest(&slow), "non-payoff choice diverged");
        assert_eq!((fast.pitch, fast.ale), (slow.pitch, slow.ale));
        for &slot in &CHOOSE {
            for it in avail.iter() {
                if !accepts(slot, it) {
                    continue;
                }
                let (a, b) = slot_score(view, &slow, slot, it, &info);
                let (c, d) = slow_score(view, &slow, slot, it, &info, 1.0);
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
    let slow = slow_make(&avail, &info, &view, &|_| 1.0);
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
            let (c, d) = slow_score(&view, &slow, slot, it, &info, 1.0);
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
    assert!(w.bands.iter().all(|b| b.learn_curve == LC_PRIOR && b.last_fight_switch == u32::MAX));
    assert!(w.bands.iter().all(|b| b.fight_calib == [0.0; 2] && b.raid == [0.0; 4] && b.raid_share == 0.0));
    let half = BELIEF_NMAX * 0.5;
    assert!(w.bands.iter().any(|b| !b.beliefs.is_empty()), "an old save fills beliefs for the kit");
    assert!(w.bands.iter().all(|b| b.beliefs.iter().all(|e| {
        e.n.to_bits() == half.to_bits() && e.fight_n == 0.0 && e.fight_mean == 0.0 && e.fight_var == 0.0
    })));
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
        // An empty table is filled on load. A table that was saved is kept as it was.
        if !x.beliefs.is_empty() {
            assert_eq!(belief(&x.beliefs), belief(&y.beliefs));
        }
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
    assert_eq!(json.bands[0].beliefs.len(), 2);
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

fn spread_var(mean: f32, u: f32, slot: u8) -> f32 {
    let floor = BELIEF_VAR_MIN * mean * mean;
    if !matches!(slot, 0 | 1 | 10 | 4 | 12 | 13) {
        return floor;
    }
    let s = NOVICE_SD * (1.0 - u) / 3.0f32.sqrt();
    let spread = (mean * s) * (mean * s);
    if spread > floor { spread } else { floor }
}

fn payoff_count(k: &Kit) -> usize {
    (0..PAYOFF_SLOT.len())
        .filter(|&j| valid_item(k.slot_item(j)) && material_term(PAYOFF_SLOT[j] as usize, k.slot_item(j)) > 0.0)
        .count()
}

fn assert_starter(w: &World, b: &Band, n: f32, u_of: impl Fn(Item) -> f32) {
    let pitch = pitch_in_use(&b.d.kit, w.near_water(b.cell(w.map.w)));
    assert_eq!(b.beliefs.len(), payoff_count(&b.d.kit));
    for bel in &b.beliefs {
        let it = bel.item;
        let u = u_of(it);
        let term = material_term(bel.slot as usize, it);
        let (_, q, depth) = recipe_info(&b.d.tk, &b.known, it);
        let mean = term * user_factor(u) * (1.0 + QUALITY_YIELD * q);
        let m = b.d.tk.chain_m(it);
        let labour = tool_upkeep(w.params.upkeep, it, depth, sealed_factor(it, &b.d.kit, pitch)) * maker_waste(m);
        assert!(term > 0.0);
        assert_eq!(bel.n.to_bits(), n.to_bits());
        assert_eq!(bel.mean.to_bits(), mean.to_bits(), "slot {}", bel.slot);
        assert_eq!(bel.var.to_bits(), spread_var(mean, u, bel.slot).to_bits());
        assert_eq!(bel.fail.to_bits(), maker_p_fail(m).to_bits());
        assert_eq!(bel.labour.to_bits(), labour.to_bits());
        assert_eq!(bel.flags, 0);
        assert_eq!((bel.fight_n, bel.fight_mean, bel.fight_var), (0.0, 0.0, 0.0));
    }
}

/// A founding kit of raw materials has no payoff slot, so the table stays empty.
/// Once the kit holds payoff items, the fill uses skill 0.7 even if stored skill differs.
#[test]
fn founder_beliefs_match_the_kit_at_skill_0_7() {
    let mut w = world();
    for b in &w.bands {
        assert_eq!(b.beliefs.len(), payoff_count(&b.d.kit));
        assert_eq!(b.learn_curve.to_bits(), LC_PRIOR.to_bits());
    }
    let mut b = std::mem::take(&mut w.bands[0]);
    b.beliefs.clear();
    b.learn_curve = 0.0;
    b.gen = 1;
    b.use_skill[Form::Edge as usize] = 0.2;
    b.d.kit = Kit::default();
    b.d.kit.cut = item(Form::Edge, Mat::Flint);
    b.d.kit.hunt = item(Form::Spear, Mat::Wood);
    b.d.kit.point = item(Form::Point, Mat::Flint);
    b.d.kit.boat = item(Form::Boat, Mat::Wood);
    w.seed_beliefs(&mut b, BELIEF_NMAX, true);
    assert_eq!(b.learn_curve.to_bits(), LC_PRIOR.to_bits());
    assert_starter(&w, &b, BELIEF_NMAX, |_| USE_FOUNDER);
    let edge = b.beliefs.iter().find(|e| e.slot == 1).unwrap();
    let term = material_term(1, edge.item);
    let at_stored = term * user_factor(0.2);
    assert_ne!(edge.mean.to_bits(), at_stored.to_bits(), "founder fill uses 0.7, not the stored skill");
    let boat = b.beliefs.iter().find(|e| e.slot == 11).unwrap();
    assert_eq!(boat.var.to_bits(), (BELIEF_VAR_MIN * boat.mean * boat.mean).to_bits(), "a hull is not noised");
    let n = b.beliefs.len();
    w.seed_beliefs(&mut b, BELIEF_NMAX, true);
    assert_eq!(b.beliefs.len(), n, "a second fill does not append");
}

/// An empty table is filled at half `n` from the loaded user skill. A table
/// that already has an entry is not replaced. `learn_curve` of 0 becomes `LC_PRIOR`.
#[test]
fn empty_beliefs_fill_on_load_and_a_saved_table_stays() {
    let bytes = include_bytes!("../tests/fixtures/v15_seed9_y150_64x40.civsave");
    let (mut w, _, _) = World::load_bytes(bytes).expect("v15 save loads");
    let with: Vec<usize> = w.bands.iter().enumerate().filter(|(_, b)| !b.beliefs.is_empty()).map(|(i, _)| i).take(2).collect();
    assert!(with.len() >= 2, "the fixture has two bands with payoff tools");
    let (keep_i, fill_i) = (with[0], with[1]);
    let marker = Belief {
        item: 7,
        slot: 1,
        flags: 1,
        n: 1.25,
        mean: 0.33,
        var: 0.04,
        fail: 0.1,
        labour: 0.2,
        fight_n: 0.5,
        fight_mean: 0.6,
        fight_var: 0.07,
    };
    w.bands[keep_i].beliefs = vec![marker.clone()];
    w.bands[fill_i].beliefs.clear();
    w.bands[fill_i].learn_curve = 0.0;
    let again = w.save_bytes(None, "t", "t").unwrap();
    let (loaded, _, _) = World::load_bytes(&again).unwrap();
    assert_eq!(loaded.bands[keep_i].beliefs, vec![marker]);
    let b = &loaded.bands[fill_i];
    assert_eq!(b.learn_curve.to_bits(), LC_PRIOR.to_bits());
    let user = band_user(b);
    assert_starter(&loaded, b, BELIEF_NMAX * 0.5, |it| user[form(it) as usize]);
    assert!(b.beliefs.iter().any(|e| {
        let u = user[form(e.item) as usize];
        let term = material_term(e.slot as usize, e.item);
        let q = recipe_info(&b.d.tk, &b.known, e.item).1;
        let at_founder = term * user_factor(USE_FOUNDER) * (1.0 + QUALITY_YIELD * q);
        (u - USE_FOUNDER).abs() > 1e-4 && e.mean.to_bits() != at_founder.to_bits()
    }), "at least one loaded belief uses the band's skill, not 0.7");
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

/// A founder that has just gained an edge still has stored skill 0. The decision
/// and the economy both read that form at USE_FOUNDER in the same season. A
/// gen-1 band with the same stored skill does not.
#[test]
fn founder_reads_use_founder_the_season_an_edge_is_gained() {
    let mut w = world();
    let flint = item(Form::Edge, Mat::Flint);
    let mut avail = ItemSet::default();
    avail.add(flint);
    let kit = crate::band::make_kit(&avail);
    assert!(kit.hunt == flint || kit.cut == flint, "the new edge has to fill a food slot");
    let mw = w.map.w;
    let ci = w.bands[0].cell(mw);
    w.game[ci] = 20.0;
    w.plants[ci] = 20.0;
    w.tick = 3;
    let arm = |b: &mut Band, gen: u32, skill: f32| {
        b.gen = gen;
        b.use_skill = vec![skill; NFORM];
        b.d.kit = kit;
    };
    arm(&mut w.bands[0], 0, 0.0);
    let view = w.band_view(&w.bands[0]);
    assert_eq!(view.user[Form::Edge as usize].to_bits(), USE_FOUNDER.to_bits());
    assert!(view.user.iter().all(|u| u.to_bits() == USE_FOUNDER.to_bits()));
    assert_eq!(w.bands[0].use_skill[Form::Edge as usize], 0.0, "the floor is not written back before practice");
    let h_founder = w.economy(&w.bands[0], 0).h;
    // Stored skill already at the floor, and not a founder: the same user array,
    // so the economy's harvest matches. A gen-1 band still at 0 does not.
    arm(&mut w.bands[0], 1, USE_FOUNDER);
    assert_eq!(w.band_view(&w.bands[0]).user[Form::Edge as usize].to_bits(), USE_FOUNDER.to_bits());
    let h_at_floor = w.economy(&w.bands[0], 0).h;
    assert_eq!(h_founder.map(f32::to_bits), h_at_floor.map(f32::to_bits), "economy did not read the founder floor");
    arm(&mut w.bands[0], 1, 0.0);
    let view1 = w.band_view(&w.bands[0]);
    assert_eq!(view1.user[Form::Edge as usize], 0.0);
    assert_ne!(view1.user[Form::Edge as usize].to_bits(), USE_FOUNDER.to_bits());
    let h_novice = w.economy(&w.bands[0], 0).h;
    assert_ne!(h_novice.map(f32::to_bits), h_founder.map(f32::to_bits), "a gen-1 novice must not get the founder floor");
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

fn shape_recipe(target: Item, tool: Item, output: Item) -> Recipe {
    Recipe { action: Action::Shape as u8, inputs: [target, tool, NONE, NONE], output, p: 0.5, ..Default::default() }
}

fn combine_recipe(items: [Item; 3], output: Item) -> Recipe {
    Recipe { action: Action::Combine as u8, inputs: canonical(Action::Combine, [items[0], items[1], items[2], NONE]), output, p: 0.5, ..Default::default() }
}

fn heat_recipe(a: Item, b: Item, fire: Item, kiln: Item, output: Item) -> Recipe {
    Recipe { action: Action::Heat as u8, inputs: canonical(Action::Heat, [a, b, fire, kiln]), output, p: 0.5, ..Default::default() }
}

/// Seed 1, 128×80, 300y. The only discovered recipe with no named technique
/// is picking up a stone cobble as a hammerstone (physics: "a hard, tough
/// cobble picked and used as a pounder"). `Other` carries nothing.
const OTHER_RECIPES_SEED1: &[&str] = &["shape stone with bare hands -> stone hammerstone"];

/// Every recipe a seeded world actually reaches has a named technique, except
/// the hammerstone line above. Steel bar is steel-making; both knives are
/// hot-forging. The table below locks the same labels on one recipe per branch
/// before the world is run.
#[test]
fn technique_labels_on_a_seeded_world() {
    let stone = item(Form::Raw, Mat::Stone);
    let flint = item(Form::Raw, Mat::Flint);
    let fibre = item(Form::Raw, Mat::Fibre);
    let hide = item(Form::Raw, Mat::Hide);
    let clay = item(Form::Raw, Mat::Clay);
    let wood = item(Form::Raw, Mat::Wood);
    let bone = item(Form::Raw, Mat::Bone);
    let copper = item(Form::Raw, Mat::Copper);
    let iron = item(Form::Raw, Mat::Iron);
    let steel = item(Form::Raw, Mat::Steel);
    let tin = item(Form::Raw, Mat::Tin);
    let cu_ore = item(Form::Raw, Mat::CopperOre);
    let tin_ore = item(Form::Raw, Mat::TinOre);
    let iron_ore = item(Form::Raw, Mat::IronOre);
    let brine = item(Form::Raw, Mat::Brine);
    let resin = item(Form::Raw, Mat::ResinWood);
    let meat = item(Form::Raw, Mat::Meat);
    let grain = item(Form::Raw, Mat::Grain);
    let pitch = item(Form::Raw, Mat::Pitch);
    let charcoal = item(Form::Raw, Mat::Charcoal);
    let hammer = item(Form::Hammer, Mat::Stone);
    let flint_edge = item(Form::Edge, Mat::Flint);
    let cu_edge = item(Form::Edge, Mat::Copper);
    let bronze_edge = item(Form::Edge, Mat::Bronze);
    let iron_edge = item(Form::Edge, Mat::Iron);
    let obsidian_point = item(Form::Point, Mat::Obsidian);
    let bone_point = item(Form::Point, Mat::Bone);
    let cu_point = item(Form::Point, Mat::Copper);
    let iron_point = item(Form::Point, Mat::Iron);
    let shaft = item(Form::Shaft, Mat::Wood);
    let fibre_cord = item(Form::Cord, Mat::Fibre);
    let flax_cord = item(Form::Cord, Mat::Flax);
    let hide_cord = item(Form::Cord, Mat::Hide);
    let clay_pot = item(Form::Vessel, Mat::Clay);
    let fibre_basket = item(Form::Vessel, Mat::Fibre);
    let hide_bag = item(Form::Vessel, Mat::Hide);
    let ceramic = item(Form::Vessel, Mat::Ceramic);
    let clay_mould = item(Form::Mould, Mat::Clay);
    let fired_mould = item(Form::Mould, Mat::Ceramic);
    let crucible = item(Form::Crucible, Mat::Copper);
    let gold_pot = item(Form::Crucible, Mat::Gold);
    let bloom = item(Form::Bloom, Mat::Iron);
    let axe = item(Form::Axe, Mat::Copper);
    let plank = item(Form::Plank, Mat::Wood);
    let loom = item(Form::Loom, Mat::Wood);
    let cloth = item(Form::Cloth, Mat::Flax);
    let fire = item(Form::Fire, Mat::Wood);
    let char_fire = item(Form::Fire, Mat::Charcoal);
    let kiln = item(Form::Kiln, Mat::Stone);
    let bellows = item(Form::Bellows, Mat::Hide);
    let jar = item(Form::Jar, Mat::Ceramic);

    let cases = [
        (shape_recipe(stone, NONE, hammer), Tech::Other),
        (shape_recipe(flint, hammer, flint_edge), Tech::Knapping),
        (shape_recipe(flint_edge, bone_point, item(Form::Point, Mat::Flint)), Tech::Knapping),
        (shape_recipe(fibre, NONE, fibre_cord), Tech::CordageNetting),
        (shape_recipe(hide, flint_edge, hide_cord), Tech::CordageNetting),
        (shape_recipe(fibre_cord, shaft, item(Form::Net, Mat::Fibre)), Tech::CordageNetting),
        (shape_recipe(fibre_cord, NONE, fibre_basket), Tech::CordageNetting),
        (shape_recipe(wood, NONE, shaft), Tech::GrindCarve),
        (shape_recipe(wood, flint_edge, shaft), Tech::GrindCarve),
        (shape_recipe(bone, flint_edge, bone_point), Tech::GrindCarve),
        (shape_recipe(shaft, flint_edge, item(Form::Spear, Mat::Wood)), Tech::GrindCarve),
        (shape_recipe(shaft, fibre_cord, item(Form::Bow, Mat::Fibre)), Tech::GrindCarve),
        (shape_recipe(cu_point, hammer, item(Form::Ornament, Mat::Copper)), Tech::GrindCarve),
        (shape_recipe(obsidian_point, stone, item(Form::Ornament, Mat::Obsidian)), Tech::GrindCarve),
        (shape_recipe(shaft, flint_edge, item(Form::Mark, Mat::Wood)), Tech::Other), // tally is a combine; this shape is not a recipe
        (combine_recipe([shaft, flint_edge, NONE], item(Form::Mark, Mat::Wood)), Tech::GrindCarve),
        (shape_recipe(bronze_edge, stone, item(Form::Point, Mat::Bronze)), Tech::GrindCarve),
        (shape_recipe(copper, hammer, cu_edge), Tech::ColdHammer),
        (shape_recipe(cu_edge, hammer, cu_point), Tech::ColdHammer),
        (shape_recipe(clay, NONE, clay_pot), Tech::PottingFiring),
        (shape_recipe(clay, flint_edge, clay_mould), Tech::PottingFiring),
        (shape_recipe(clay, bone_point, item(Form::Mark, Mat::Clay)), Tech::PottingFiring),
        (shape_recipe(wood, axe, plank), Tech::BoatBuilding),
        (shape_recipe(wood, axe, item(Form::Boat, Mat::Wood)), Tech::BoatBuilding),
        (combine_recipe([shaft, fibre_cord, bone_point], item(Form::Spear, Mat::Bone)), Tech::Hafting),
        (combine_recipe([shaft, fibre_cord, flint_edge], item(Form::Axe, Mat::Flint)), Tech::Hafting),
        (combine_recipe([shaft, fibre_cord, NONE], item(Form::Trap, Mat::Fibre)), Tech::Hafting),
        (combine_recipe([shaft, fibre_cord, fibre_basket], item(Form::Frame, Mat::Fibre)), Tech::Hafting),
        (combine_recipe([shaft, fibre_cord, hide_bag], bellows), Tech::Hafting),
        (combine_recipe([shaft, fibre_cord, stone], loom), Tech::Hafting),
        (combine_recipe([shaft, fibre_cord, hide], item(Form::Shield, Mat::Hide)), Tech::Hafting),
        (combine_recipe([loom, flax_cord, NONE], cloth), Tech::SpinWeave),
        (combine_recipe([hide, fibre_cord, bone_point], item(Form::Garment, Mat::Hide)), Tech::Sewing),
        (combine_recipe([cloth, flax_cord, bone_point], item(Form::Garment, Mat::Flax)), Tech::Sewing),
        (combine_recipe([shaft, fibre_cord, hide], item(Form::Boat, Mat::Hide)), Tech::BoatBuilding),
        (combine_recipe([fibre, fibre_cord, NONE], item(Form::Boat, Mat::Fibre)), Tech::BoatBuilding),
        (combine_recipe([item(Form::Boat, Mat::Wood), shaft, fibre_cord], item(Form::Outrigger, Mat::Wood)), Tech::BoatBuilding),
        (combine_recipe([plank, shaft, cloth], item(Form::Sail, Mat::Flax)), Tech::BoatBuilding),
        (combine_recipe([plank, pitch, iron_point], item(Form::Ship, Mat::Wood)), Tech::BoatBuilding),
        (combine_recipe([copper, ceramic, NONE], crucible), Tech::Casting),
        (combine_recipe([pitch, ceramic, NONE], jar), Tech::PottingFiring),
        (combine_recipe([grain, jar, NONE], item(Form::Jar, Mat::Grain)), Tech::Cooking),
        (combine_recipe([shaft, wood, NONE], fire), Tech::FireMaking),
        (combine_recipe([fire, charcoal, NONE], char_fire), Tech::FireMaking),
        (combine_recipe([fire, stone, NONE], kiln), Tech::FireMaking),
        (heat_recipe(meat, NONE, fire, NONE, item(Form::Cooked, Mat::Meat)), Tech::Cooking),
        (heat_recipe(brine, ceramic, fire, NONE, item(Form::Raw, Mat::Salt)), Tech::Cooking),
        (heat_recipe(resin, ceramic, fire, NONE, pitch), Tech::Cooking),
        (heat_recipe(wood, NONE, fire, kiln, charcoal), Tech::FireMaking),
        (heat_recipe(clay_pot, NONE, fire, kiln, ceramic), Tech::PottingFiring),
        (heat_recipe(clay_mould, NONE, fire, kiln, fired_mould), Tech::PottingFiring),
        (heat_recipe(cu_ore, NONE, char_fire, kiln, copper), Tech::Smelting),
        (heat_recipe(iron_ore, bellows, char_fire, kiln, bloom), Tech::Smelting),
        (heat_recipe(cu_ore, tin_ore, char_fire, kiln, item(Form::Raw, Mat::Bronze)), Tech::Smelting),
        (heat_recipe(crucible, tin, char_fire, kiln, item(Form::Raw, Mat::Bronze)), Tech::Casting),
        (heat_recipe(crucible, fired_mould, char_fire, kiln, bronze_edge), Tech::Casting),
        (heat_recipe(gold_pot, fired_mould, char_fire, kiln, item(Form::Ornament, Mat::Gold)), Tech::Casting),
        (heat_recipe(bloom, hammer, char_fire, kiln, iron), Tech::HotForging),
        (heat_recipe(iron, hammer, char_fire, kiln, iron_edge), Tech::HotForging),
        (heat_recipe(iron_edge, hammer, char_fire, kiln, iron_point), Tech::HotForging),
        (heat_recipe(steel, hammer, char_fire, kiln, item(Form::Edge, Mat::Steel)), Tech::HotForging),
        (heat_recipe(iron, hammer, char_fire, bellows, steel), Tech::SteelMaking),
        (heat_recipe(steel, hammer, char_fire, bellows, item(Form::Edge, Mat::Steel)), Tech::HotForging),
    ];
    // A shape that merely outputs a mark is not carving: the tally is a combine.
    for (r, want) in &cases {
        assert_eq!(technique(r), *want, "{} -> {}", r.describe(), item_name(r.output));
    }
    let mut reg = Registry::default();
    let iron_knife = heat_recipe(iron, hammer, char_fire, kiln, iron_edge);
    let id = reg.insert(iron_knife);
    assert_eq!(reg.tech(id), Tech::HotForging);
    assert_eq!(reg.tech(id), technique(&reg.recipes[id as usize]));
    reg.rebuild_index();
    assert_eq!(reg.tech(id), Tech::HotForging);
    assert_eq!(reg.tech(u32::MAX), Tech::Other);

    let mut w = World::new(Config { seed: 1, map_w: 128, map_h: 80, ..Default::default() }, Params::default());
    for _ in 0..300 * 4 {
        w.step();
    }
    let mut got: Vec<String> = Vec::new();
    for (id, r) in w.reg.recipes.iter().enumerate() {
        let tech = w.reg.tech(id as u32);
        assert_eq!(tech, technique(r), "registry label drifted: {}", r.describe());
        if tech == Tech::Other {
            got.push(format!("{} -> {}", r.describe(), item_name(r.output)));
        }
        // These three are not guaranteed by year 300 once maker waste changes the
        // invention lottery. The synthetic cases above are what locks their labels.
        match item_name(r.output).as_str() {
            "steel bar" => assert_eq!(tech, Tech::SteelMaking, "{}", r.describe()),
            "iron knife" | "steel knife" => assert_eq!(tech, Tech::HotForging, "{}", r.describe()),
            _ => {}
        }
    }
    got.sort();
    let expect: Vec<&str> = OTHER_RECIPES_SEED1.to_vec();
    let got_ref: Vec<&str> = got.iter().map(|s| s.as_str()).collect();
    assert_eq!(got_ref, expect, "Other recipes in seed 1, 128x80, 300y");
}

/// The weighted geometric mean, written out here so a shared mistake in
/// `fill_chain` still fails. `steps` is `(maker skill, success chance)`.
fn geom_mean(steps: &[(f32, f32)]) -> f32 {
    let mut sw = 0.0f32;
    let mut swl = 0.0f32;
    for &(m, p) in steps {
        let w = (-p.ln()).max(CHAIN_W_MIN);
        let mc = m.max(CHAIN_EPS);
        sw += w;
        swl += w * mc.ln();
    }
    (swl / sw).exp()
}

fn with_p(mut r: Recipe, p: f32) -> Recipe {
    r.p = p;
    r
}

/// Chain skill of `out` for a band that knows `parts` (recipe, skill) and holds `raws`.
fn chain_of(parts: Vec<(Recipe, f32)>, raws: &[Item], out: Item) -> f32 {
    let mut reg = Registry::default();
    let mut known = Vec::new();
    for (r, skill) in parts {
        let id = reg.insert(r);
        known.push(Known { recipe: id, skill, ..Default::default() });
    }
    let mut have = ItemSet::default();
    for &i in raws {
        have.add(i);
    }
    let mut tk = closure(&reg, &known, &have, false);
    fill_chain(&reg, &known, &mut tk);
    tk.chain_m(out)
}

/// Chain skill rises when any step rises, a longer chain of equal skills is not
/// lower, and one weak step pulls the mean down without becoming the minimum.
#[test]
fn chain_skill_tracks_every_step() {
    let fibre = item(Form::Raw, Mat::Fibre);
    let wood = item(Form::Raw, Mat::Wood);
    let cord = item(Form::Cord, Mat::Fibre);
    let shaft = item(Form::Shaft, Mat::Wood);
    let snare = item(Form::Trap, Mat::Fibre);
    let cord_r = |p: f32| with_p(shape_recipe(fibre, NONE, cord), p);
    let shaft_r = |p: f32| with_p(shape_recipe(wood, NONE, shaft), p);
    let snare_r = |p: f32| with_p(combine_recipe([shaft, cord, NONE], snare), p);

    // One step, from the finished cord and shaft treated as raw.
    let one = chain_of(vec![(snare_r(0.5), 0.7)], &[cord, shaft], snare);
    // Three equally skilled steps. A product would be 0.7^3.
    let three = chain_of(
        vec![(cord_r(0.5), 0.7), (shaft_r(0.5), 0.7), (snare_r(0.5), 0.7)],
        &[fibre, wood],
        snare,
    );
    assert!((one - 0.7).abs() < 1e-4, "one step is its own skill, got {one}");
    assert!((three - one).abs() < 1e-4, "equal skills: longer {three} vs one {one}");
    assert!(three + 1e-5 >= one, "a longer equal chain must not be lower");
    assert!(three > 0.7 * 0.7 * 0.7 + 0.05, "not the plain product");

    let at = |c: f32, s: f32, n: f32| {
        chain_of(vec![(cord_r(0.4), c), (shaft_r(0.3), s), (snare_r(0.2), n)], &[fibre, wood], snare)
    };
    let base = at(0.4, 0.5, 0.6);
    assert!(at(0.7, 0.5, 0.6) > base + 1e-4, "raising the cord step");
    assert!(at(0.4, 0.8, 0.6) > base + 1e-4, "raising the shaft step");
    assert!(at(0.4, 0.5, 0.9) > base + 1e-4, "raising the snare step");

    let strong = at(0.8, 0.8, 0.8);
    let weak = at(0.8, 0.8, 0.1);
    assert!(weak < strong - 0.05, "weak step {weak} did not pull down {strong}");
    assert!(weak > 0.15, "weak step collapsed to the minimum: {weak}");

    let steps = [(0.8f32, 0.5f32), (0.4, 0.05), (0.6, 0.2)];
    let got = chain_of(
        vec![(cord_r(steps[0].1), steps[0].0), (shaft_r(steps[1].1), steps[1].0), (snare_r(steps[2].1), steps[2].0)],
        &[fibre, wood],
        snare,
    );
    let want = geom_mean(&steps);
    assert!((got - want).abs() < 1e-5, "weighted geometric mean {got} vs {want}");

    // Raws and a kept flame are not steps.
    let mut raws = ItemSet::default();
    raws.add(wood);
    let reg = Registry::default();
    let mut tk = closure(&reg, &[], &raws, true);
    fill_chain(&reg, &[], &mut tk);
    assert_eq!(tk.chain_m(wood).to_bits(), 1.0f32.to_bits());
    assert_eq!(tk.chain_m(item(Form::Fire, Mat::Wood)).to_bits(), 1.0f32.to_bits());
}

/// Steel-making does not inherit hot-forging. A steel knife does, at TECH_CARRY
/// of the iron-knife skill. Other recipes carry nothing to each other.
#[test]
fn steel_making_does_not_carry_forging_skill() {
    let iron = item(Form::Raw, Mat::Iron);
    let steel = item(Form::Raw, Mat::Steel);
    let hammer = item(Form::Hammer, Mat::Stone);
    let fire = item(Form::Fire, Mat::Charcoal);
    let kiln = item(Form::Kiln, Mat::Stone);
    let bellows = item(Form::Bellows, Mat::Hide);
    let iron_knife = item(Form::Edge, Mat::Iron);
    let steel_knife = item(Form::Edge, Mat::Steel);
    let mut reg = Registry::default();
    let id_iron = reg.insert(heat_recipe(iron, hammer, fire, kiln, iron_knife));
    let id_knife = reg.insert(heat_recipe(steel, hammer, fire, kiln, steel_knife));
    let id_bar = reg.insert(heat_recipe(iron, hammer, fire, bellows, steel));
    assert_eq!((reg.tech(id_iron), reg.tech(id_knife), reg.tech(id_bar)), (Tech::HotForging, Tech::HotForging, Tech::SteelMaking));
    let known = vec![
        Known { recipe: id_iron, skill: 0.8, ..Default::default() },
        Known { recipe: id_knife, skill: 0.1, ..Default::default() },
        Known { recipe: id_bar, skill: 0.12, ..Default::default() },
    ];
    assert_eq!(maker_skill(&reg, &known, 0).to_bits(), 0.8f32.to_bits());
    let carried = TECH_CARRY * 0.8;
    assert!(carried > 0.1);
    assert_eq!(maker_skill(&reg, &known, 1).to_bits(), carried.to_bits());
    assert_eq!(maker_skill(&reg, &known, 2).to_bits(), 0.12f32.to_bits());

    let stone = reg.insert(shape_recipe(item(Form::Raw, Mat::Stone), NONE, hammer));
    let bone = reg.insert(shape_recipe(item(Form::Raw, Mat::Bone), NONE, item(Form::Hammer, Mat::Bone)));
    assert_eq!((reg.tech(stone), reg.tech(bone)), (Tech::Other, Tech::Other));
    let hammers = vec![
        Known { recipe: stone, skill: 0.9, ..Default::default() },
        Known { recipe: bone, skill: 0.2, ..Default::default() },
    ];
    assert_eq!(maker_skill(&reg, &hammers, 1).to_bits(), 0.2f32.to_bits());
    assert_eq!(maker_skill(&reg, &hammers, 0).to_bits(), 0.9f32.to_bits());

    let best = maker_tech_best(&reg, &known);
    assert_eq!(best[0].unwrap().to_bits(), 0.8f32.to_bits(), "hot-forging best is the stronger own skill");
    assert_eq!(best[1].unwrap().to_bits(), 0.12f32.to_bits(), "steel-making stands alone");
    assert!(best[2].is_none() && best[3].is_none());
}

/// The cache is filled on a live band, and a skill change with no new closure
/// (the cache-hit path of refresh_toolkit) refills it.
#[test]
fn chain_cache_follows_skill_without_a_new_closure() {
    let mut w = world();
    for _ in 0..60 {
        w.step();
    }
    let mut saw_low = false;
    for b in &w.bands {
        for it in b.d.tk.producible.iter() {
            let m = b.d.tk.chain_m(it);
            assert!(m.is_finite() && m > 0.0 && m <= 1.0, "{m}");
            if m < 0.999 {
                saw_low = true;
            }
        }
    }
    assert!(saw_low, "a produced item should still be short of maker skill 1");
    for b in &mut w.bands {
        for k in &mut b.known {
            k.skill = 1.0;
        }
        // The cache-hit path refills only when recipe skill's version moves.
        b.d.skill_ver = b.d.skill_ver.wrapping_add(1);
    }
    w.refresh_all_toolkits();
    for b in &w.bands {
        for it in b.d.tk.producible.iter() {
            assert_eq!(b.d.tk.chain_m(it).to_bits(), 1.0f32.to_bits(), "{}", item_name(it));
        }
    }
}

/// Nominal kit choice ignores a filled chain. `make_kit_for` matches `make_kit_with`
/// (maker skill forced to 1) even when every edge was made at 0.2. `slot_score` matches
/// `slot_score_m(..., 1)`; the live chain skill moves only the current side.
#[test]
fn nominal_kit_ignores_a_weak_chain() {
    let flint_raw = item(Form::Raw, Mat::Flint);
    let stone_raw = item(Form::Raw, Mat::Stone);
    let flint = item(Form::Edge, Mat::Flint);
    let stone = item(Form::Edge, Mat::Stone);
    let mut reg = Registry::default();
    let mut known = Vec::new();
    for (r, skill) in [(shape_recipe(flint_raw, NONE, flint), 0.2f32), (shape_recipe(stone_raw, NONE, stone), 0.2)] {
        let id = reg.insert(r);
        known.push(Known { recipe: id, skill, ..Default::default() });
    }
    let mut have = ItemSet::default();
    have.add(flint_raw);
    have.add(stone_raw);
    let mut tk = closure(&reg, &known, &have, false);
    fill_chain(&reg, &known, &mut tk);
    let mf = tk.chain_m(flint);
    assert!(mf < 0.5 && mf > 0.1, "flint chain skill {mf}");
    assert!(tk.chain_m(stone) < 0.5, "stone chain skill");

    let info = |it: Item| recipe_info(&tk, &known, it);
    let forced = make_kit_for(&tk, &known);
    let plain = make_kit_with(&tk.avail, &info);
    assert_eq!(kit_picks(&forced), kit_picks(&plain), "nominal choice saw the chain");
    assert_eq!(forced.fallback, plain.fallback, "nominal fallback saw the chain");
    assert_eq!(kit_rest(&forced), kit_rest(&plain), "nominal non-payoff saw the chain");

    let view = BandView::nominal();
    let (a, b) = slot_score(&view, &plain, 1, flint, &info);
    let (c, d) = slot_score_m(&view, &plain, 1, flint, &info, 1.0);
    assert_eq!(a.to_bits(), c.to_bits(), "m = 1 current side");
    assert_eq!(b.to_bits(), d.to_bits(), "m = 1 full side");
    let (e, f) = slot_score_m(&view, &plain, 1, flint, &info, mf);
    assert_ne!(e.to_bits(), a.to_bits(), "a weak maker should change the current food price");
    assert_eq!(f.to_bits(), b.to_bits(), "the full side stays the mastered price");
}

/// Fast and slow choice agree, including the runner-up, when maker skill is not all 1.
#[test]
fn fast_choice_matches_slow_when_maker_skill_varies() {
    let flint = item(Form::Edge, Mat::Flint);
    let stone = item(Form::Edge, Mat::Stone);
    let mut avail = ItemSet::default();
    avail.add(flint);
    avail.add(stone);
    let info = |it: Item| if it == flint { (0.4f32, 0.1, 1u8) } else { (0.8, 0.0, 0) };
    let m_of = |it: Item| if it == flint { 0.2 } else { 1.0 };
    // Same view as the m = 1 bit-match (user 1). A uniform user of 0.4 leaves a
    // one-ulp gap in the cut slot that the mastered path already had; this view does not.
    let view = BandView { harvest: [2.0, 40.0, 1.0, 5.0, 0.0], food: 30.0, pop: 40.0, upkeep_rate: 0.02, wet: true, at_hand: 0, user: [1.0; NFORM] };
    let fast = make_kit_scored(&avail, &info, &view, &m_of);
    let slow = slow_make(&avail, &info, &view, &m_of);
    assert_eq!(kit_picks(&fast), kit_picks(&slow), "payoff choice diverged");
    assert_eq!(fast.fallback, slow.fallback, "fallback diverged");
    assert_eq!(kit_rest(&fast), kit_rest(&slow), "non-payoff choice diverged");
    for &slot in &CHOOSE {
        for it in [flint, stone] {
            if !accepts(slot, it) {
                continue;
            }
            let m = m_of(it);
            let (a, b) = slot_score_m(&view, &slow, slot, it, &info, m);
            let (c, d) = slow_score(&view, &slow, slot, it, &info, m);
            assert_eq!(a.to_bits(), c.to_bits(), "slot {slot} m {m}");
            assert_eq!(b.to_bits(), d.to_bits(), "slot {slot} full");
        }
    }
}

/// The failure roll repeats for the same seed, band, tick and slot. Its chance is
/// 0 at maker skill 1, and a skill of 0 fails more often than 0.5. Labour and scrap
/// factors are bit-identical to 1 at maker skill 1.
#[test]
fn maker_fail_roll_is_deterministic_and_rate_falls() {
    let roll = maker_fail_roll(1, 7, 40, 3);
    assert_eq!(roll.to_bits(), maker_fail_roll(1, 7, 40, 3).to_bits());
    assert!(roll >= 0.0 && roll < 1.0);
    assert_eq!(maker_p_fail(1.0).to_bits(), 0.0f32.to_bits());
    assert!(maker_p_fail(0.0) > maker_p_fail(0.5));
    assert_eq!(maker_waste(1.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(maker_scrap_factor(1.0).to_bits(), 1.0f32.to_bits());
    let p0 = maker_p_fail(0.0);
    let p5 = maker_p_fail(0.5);
    let mut n0 = 0u32;
    let mut n5 = 0u32;
    for i in 0..4000u32 {
        let r = maker_fail_roll(9, i, 1, 0);
        if r < p0 {
            n0 += 1;
        }
        if r < p5 {
            n5 += 1;
        }
    }
    assert!(n0 > n5, "skill 0 should fail more often than 0.5 ({n0} vs {n5})");
}

/// A band that knows hot-forging pulls `maker_hot_forge` off zero. The mean is the
/// best carried skill of that technique, over bands that know at least one such recipe.
#[test]
fn maker_hot_forge_mean_follows_known_skill() {
    let iron = item(Form::Raw, Mat::Iron);
    let knife = item(Form::Edge, Mat::Iron);
    let hammer = item(Form::Hammer, Mat::Stone);
    let fire = item(Form::Fire, Mat::Charcoal);
    let kiln = item(Form::Kiln, Mat::Stone);
    let mut w = world();
    assert!(w.bands.len() >= 2, "need two bands");
    for b in &mut w.bands {
        b.known.clear();
    }
    let id = w.reg.insert(heat_recipe(iron, hammer, fire, kiln, knife));
    assert_eq!(w.reg.tech(id), Tech::HotForging);
    w.bands[0].known.push(Known { recipe: id, skill: 0.4, ..Default::default() });
    w.bands[1].known.push(Known { recipe: id, skill: 0.8, ..Default::default() });
    w.year_stats();
    let h = w.history.last().unwrap();
    assert!(h.maker_hot_forge > 0.0, "hot-forging skill should show in the mean");
    let mut sum = 0.0f64;
    let mut n = 0u32;
    for b in &w.bands {
        if let Some(m) = maker_tech_best(&w.reg, &b.known)[0] {
            sum += m as f64;
            n += 1;
        }
    }
    assert_eq!(n, 2);
    let expect = (sum / n as f64) as f32;
    assert_eq!(h.maker_hot_forge.to_bits(), expect.to_bits());
    assert_eq!(h.maker_steel.to_bits(), 0.0f32.to_bits());
    assert_eq!(h.maker_cast.to_bits(), 0.0f32.to_bits());
    assert_eq!(h.maker_smelt.to_bits(), 0.0f32.to_bits());
}

/// First steel-smith is the first band in order at maker skill 0.6, logged once.
/// A second `year_stats`, and a log that already holds the event, do not log it again.
#[test]
fn first_steel_smith_is_once_and_comes_from_the_log() {
    let iron = item(Form::Raw, Mat::Iron);
    let steel = item(Form::Raw, Mat::Steel);
    let hammer = item(Form::Hammer, Mat::Stone);
    let fire = item(Form::Fire, Mat::Charcoal);
    let bellows = item(Form::Bellows, Mat::Hide);
    let mut w = world();
    assert!(w.bands.len() >= 2, "need two bands");
    for b in &mut w.bands {
        b.known.clear();
    }
    let id = w.reg.insert(heat_recipe(iron, hammer, fire, bellows, steel));
    assert_eq!(w.reg.tech(id), Tech::SteelMaking);
    w.bands[0].known.push(Known { recipe: id, skill: 0.4, ..Default::default() });
    w.bands[1].known.push(Known { recipe: id, skill: 0.6, ..Default::default() });
    assert!(!EventKind::FirstSteelSmith.major());
    assert_eq!(EventKind::COUNT, 43);
    assert_eq!(EventKind::FirstSteelSmith as u8, 42);
    assert_eq!(EventKind::from_u8(42), EventKind::FirstSteelSmith);
    let n = |w: &World| w.events.iter().filter(|e| e.kind == EventKind::FirstSteelSmith as u8).count();
    assert_eq!(n(&w), 0);
    w.year_stats();
    assert_eq!(n(&w), 1);
    let e = w.events.iter().find(|e| e.kind == EventKind::FirstSteelSmith as u8).copied().unwrap();
    assert_eq!(e.band, w.bands[1].id);
    let mw = w.map.w.max(1);
    let (cx, cy) = (e.cell as usize % mw, e.cell as usize / mw);
    assert_eq!(w.event_text(&e), format!("Y{}: band {} at ({cx},{cy}) is the first to master steel-making", e.tick / 4, e.band));
    w.year_stats();
    assert_eq!(n(&w), 1, "a second year_stats must not log it again");

    let mut w2 = world();
    for b in &mut w2.bands {
        b.known.clear();
    }
    let id2 = w2.reg.insert(heat_recipe(iron, hammer, fire, bellows, steel));
    w2.bands[0].known.push(Known { recipe: id2, skill: 0.9, ..Default::default() });
    w2.events.push(Event { tick: 4, kind: EventKind::FirstSteelSmith as u8, band: 999_999, other: 0, cell: 0, item: NONE, recipe: u32::MAX });
    w2.year_stats();
    assert_eq!(n(&w2), 1);
    assert_eq!(w2.events.iter().find(|e| e.kind == EventKind::FirstSteelSmith as u8).unwrap().band, 999_999);
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-5
}

/// Proposal table, so a later edit cannot drift one constant.
#[test]
fn belief_constants_match_the_proposal() {
    assert_eq!(BELIEF_CAP, 24);
    assert_eq!(BELIEF_NMAX.to_bits(), 8.0f32.to_bits());
    assert_eq!(BELIEF_ADOPT_N.to_bits(), 1.0f32.to_bits());
    assert_eq!(BELIEF_VAR_MIN.to_bits(), 0.0025f32.to_bits());
    assert_eq!(BELIEF_FADE.to_bits(), 0.8f32.to_bits());
    assert_eq!(BELIEF_DROP_N.to_bits(), 0.1f32.to_bits());
    assert_eq!(BELIEF_INHERIT_N.to_bits(), 0.6f32.to_bits());
    assert_eq!(BELIEF_WIDEN.to_bits(), 0.5f32.to_bits());
    assert_eq!(BELIEF_TELL_P.to_bits(), 0.2f32.to_bits());
    assert_eq!(TELL_W.to_bits(), 0.5f32.to_bits());
    assert_eq!(TELL_NMAX.to_bits(), 1.0f32.to_bits());
    assert_eq!(BELIEF_FAIL_N0.to_bits(), 2.0f32.to_bits());
    assert_eq!(PRIOR_N.to_bits(), 0.5f32.to_bits());
    assert_eq!(PRIOR_N0.to_bits(), 1.0f32.to_bits());
    assert_eq!(PRIOR_CV.to_bits(), 0.25f32.to_bits());
    assert_eq!(PRIOR_NOVEL.to_bits(), 2.0f32.to_bits());
    assert_eq!(PRIOR_FAIL.to_bits(), 0.15f32.to_bits());
    assert_eq!(LC_PRIOR.to_bits(), 1.3f32.to_bits());
    assert_eq!(LC_RATE.to_bits(), 0.2f32.to_bits());
    assert_eq!(INFO_W.to_bits(), 0.3f32.to_bits());
    assert_eq!(WRITE_OFF_SEASONS, 4);
    assert_eq!(TRIAL_SHARE.to_bits(), 0.25f32.to_bits());
}

#[test]
fn motion_cos_is_raw_and_material_term_follows_the_slot() {
    let edge = Form::Edge as u8;
    let bow = Form::Bow as u8;
    let vessel = Form::Vessel as u8;
    assert_eq!(motion_cos(edge, edge).to_bits(), 1.0f32.to_bits());
    assert_eq!(motion_cos(edge, bow).to_bits(), 0.0f32.to_bits());
    assert_eq!(motion_cos(vessel, vessel).to_bits(), 0.0f32.to_bits());
    assert_eq!(motion_cos(vessel, edge).to_bits(), 0.0f32.to_bits());
    let sp = motion_cos(Form::Spear as u8, Form::Point as u8);
    assert!(sp > 0.5 && sp < 1.0, "spear and point share thrust and throw, cos {sp}");
    // transfer_table scales the off-diagonal; the diagonal there is 1 even with no profile
    let t = transfer_table();
    assert_eq!(t[vessel as usize][vessel as usize].to_bits(), 1.0f32.to_bits());
    assert!(t[edge as usize][Form::Axe as usize] > 0.0);

    let iron = item(Form::Edge, Mat::Iron);
    let steel = item(Form::Edge, Mat::Steel);
    assert_eq!(material_term(1, iron).to_bits(), cut_q(Mat::Iron).to_bits());
    assert!(material_term(1, steel) > material_term(1, iron));
    assert!(material_term(1, item(Form::Edge, Mat::Flint)) > material_term(1, item(Form::Edge, Mat::Stone)));
    let spear = item(Form::Spear, Mat::Flint);
    assert_eq!(material_term(0, spear).to_bits(), (0.35 + 0.65 * cut_q(Mat::Flint)).to_bits());
    assert_eq!(material_term(10, item(Form::Point, Mat::Bone)).to_bits(), 1.0f32.to_bits());
    assert_eq!(material_term(14, item(Form::Bow, Mat::Wood)).to_bits(), (Mat::Wood.props().tough + 1.0).to_bits());
    assert_eq!(material_term(1, NONE).to_bits(), 0.0f32.to_bits());
}

/// A band that has used iron knives rates a steel knife above iron, narrowly.
/// A bow, with nothing like it, sits on the average ratio with the novelty widening.
/// Visible terms order priors the same way as `cut_q`.
#[test]
fn priors_follow_similar_tools_and_visible_terms() {
    let iron = item(Form::Edge, Mat::Iron);
    let steel = item(Form::Edge, Mat::Steel);
    let p_iron = cut_q(Mat::Iron);
    let used = Belief {
        item: iron,
        slot: 1,
        n: 8.0,
        mean: p_iron * 1.1,
        var: 0.01,
        fail: 0.05,
        ..Default::default()
    };
    let steel_p = food_prior(&[used.clone()], 1, steel, None, None);
    assert!(near(steel_p.n, PRIOR_N));
    assert!(near(steel_p.fail, PRIOR_FAIL));
    assert!(near(steel_p.mean, cut_q(Mat::Steel) * 1.1), "steel mean {}", steel_p.mean);
    assert!(steel_p.mean > used.mean, "steel is rated above the iron knife");
    let narrow = (PRIOR_CV * steel_p.mean) * (PRIOR_CV * steel_p.mean);
    assert!(near(steel_p.var, narrow), "s = 1 leaves no novelty, var {}", steel_p.var);

    let bow = item(Form::Bow, Mat::Wood);
    let bow_p = food_prior(&[used.clone()], 14, bow, None, None);
    let p_bow = material_term(14, bow);
    assert!(near(bow_p.mean / p_bow, 1.1), "wide prior stays on the average ratio");
    let wide = narrow_of(bow_p.mean) * (1.0 + PRIOR_NOVEL);
    assert!(near(bow_p.var, wide), "nothing like a bow widens the prior");
    assert!(bow_p.var / (bow_p.mean * bow_p.mean) > 2.5 * (steel_p.var / (steel_p.mean * steel_p.mean)));

    let bow_b = Belief { item: bow, slot: 14, n: 4.0, mean: p_bow * 0.5, ..Default::default() };
    let mixed = food_prior(&[used.clone(), bow_b], 1, steel, None, None);
    // iron rho 1.1 weight s*n = 8; bow s = 0; rho_all = (8*1.1 + 4*0.5) / 12
    let rho_all = (8.0 * 1.1 + 4.0 * 0.5) / 12.0;
    let rho = (8.0 * 1.1 + PRIOR_N0 * rho_all) / (8.0 + PRIOR_N0);
    assert!(near(mixed.mean, cut_q(Mat::Steel) * rho), "mixed rho {}", mixed.mean / cut_q(Mat::Steel));
    assert!(rho > 1.0 && rho < 1.1, "the dissimilar bow should pull rho down a little, got {rho}");

    let mut terms = Vec::new();
    for m in [Mat::Stone, Mat::Flint, Mat::Iron, Mat::Steel, Mat::Obsidian] {
        let it = item(Form::Edge, m);
        let p = food_prior(&[used.clone()], 1, it, Some(0.4), None);
        assert!(near(p.fail, 0.4));
        terms.push((material_term(1, it), p.mean));
    }
    for w in terms.windows(2) {
        assert_eq!((w[0].0 < w[1].0), (w[0].1 < w[1].1), "prior order left the material order");
    }

    let empty = food_prior(&[], 1, steel, None, None);
    assert!(near(empty.mean, cut_q(Mat::Steel)));
    assert!(near(empty.var, narrow_of(empty.mean) * (1.0 + PRIOR_NOVEL)));
    assert!(technique_fail(&[], |_| true).is_none());
    let rated = technique_fail(&[used], |_| true).unwrap();
    assert!(near(rated, 0.05));
}

fn narrow_of(mean: f32) -> f32 {
    (PRIOR_CV * mean) * (PRIOR_CV * mean)
}

#[test]
fn belief_update_moves_mean_var_fail_and_labour() {
    let mut b = belief_from_prior(1, item(Form::Edge, Mat::Iron), FoodPrior { n: PRIOR_N, mean: 1.0, var: 0.04, fail: 0.15 });
    b.labour = 0.2;
    b.flags = FLAG_INHERITED;
    b.fight_mean = 0.25;
    assert!(!update_belief(&mut b, 1.6, 0.0, true, 0.5));
    assert_eq!(b.n.to_bits(), PRIOR_N.to_bits(), "a zero weight must not touch n");
    let crossed = update_belief(&mut b, 1.6, 1.0, true, 0.5);
    assert!(!crossed, "0.5 + 1 stays under the cap");
    assert!(near(b.n, 1.5));
    assert!(near(b.mean, 1.4), "mean {}", b.mean);
    // (1 - 2/3) * (0.04 + (2/3) * 0.36) = (1/3) * 0.28
    assert!(near(b.var, 0.28 / 3.0), "var {}", b.var);
    // strength 2.5, fail' = (0.15 * 2.5 + 1) / 3.5
    assert!(near(b.fail, 1.375 / 3.5), "fail {}", b.fail);
    assert!(near(b.labour, 0.4), "labour {}", b.labour);
    assert_eq!(b.flags, FLAG_OWN | FLAG_INHERITED);
    assert_eq!(b.fight_mean.to_bits(), 0.25f32.to_bits(), "fight fields stay put");

    b.n = 7.5;
    b.mean = 1.0;
    b.var = 0.0;
    assert!(update_belief(&mut b, 1.0, 1.0, false, 1.0));
    assert_eq!(b.n.to_bits(), BELIEF_NMAX.to_bits());
    assert!(near(b.var, BELIEF_VAR_MIN), "var floor {}", b.var);
    assert!(!update_belief(&mut b, 1.0, 1.0, false, 1.0), "already at the cap");
    assert_eq!(b.n.to_bits(), BELIEF_NMAX.to_bits());
}

/// Steady use pulls the mean to the true multiplier within a few percent by the
/// time `n` reaches the cap, and `var/n` shrinks.
#[test]
fn belief_mean_converges_and_doubt_shrinks() {
    let x = 1.2f32;
    let mut b = Belief { item: item(Form::Edge, Mat::Flint), slot: 1, n: PRIOR_N, mean: 1.5 * x, var: narrow_of(1.5 * x), fail: PRIOR_FAIL, ..Default::default() };
    let doubt0 = b.var / b.n;
    for _ in 0..8 {
        observe(&mut b, &mut LearnState::new(), x, 1.0, false, 0.1);
    }
    assert_eq!(b.n.to_bits(), BELIEF_NMAX.to_bits());
    assert!((b.mean - x).abs() / x < 0.05, "mean {} after a cap of seasons", b.mean);
    assert!(b.var / b.n < doubt0, "doubt {} vs {doubt0}", b.var / b.n);
    let mut learn = LearnState::new();
    let mut flat = Belief { n: PRIOR_N, mean: 1.0, var: 0.01, fail: PRIOR_FAIL, item: b.item, slot: 1, ..Default::default() };
    for _ in 0..8 {
        observe(&mut flat, &mut learn, 1.0, 1.0, false, 0.0);
    }
    assert_eq!(flat.n.to_bits(), BELIEF_NMAX.to_bits());
    // The proposal's one-shot blend toward ratio 1. The season rule spreads that
    // over the climb and does not store the first-season mean, so it lands close.
    let shot = blend_learn_curve(LC_PRIOR, 1.0, 1.0);
    assert!((learn.curve - shot).abs() < 0.02, "flat climb {} vs one-shot {shot}", learn.curve);
    assert_eq!(learn.first_mean.to_bits(), 0.0f32.to_bits());
    let span = BELIEF_NMAX - PRIOR_N - 1.0;
    let frac = 1.0 / span;
    let expect = (LC_PRIOR.ln() * (1.0 - LC_RATE * frac) + LC_RATE * 1.2f32.ln()).exp();
    assert!(near(fold_learn_curve(LC_PRIOR, 1.0, 1.2, 1.5, 2.5), expect), "one step of the climb");
    assert_eq!(fold_learn_curve(LC_PRIOR, 1.0, 1.5, PRIOR_N, PRIOR_N + 1.0).to_bits(), LC_PRIOR.to_bits(), "the first season is the baseline");
    assert_eq!(fold_learn_curve(LC_PRIOR, 1.0, 1.5, BELIEF_NMAX, BELIEF_NMAX).to_bits(), LC_PRIOR.to_bits(), "at the cap a founder does not teach");
    assert!(near(blend_learn_curve(LC_PRIOR, 1.5, 1.0), LC_PRIOR + LC_RATE * (1.5 - LC_PRIOR)));
    assert_eq!(blend_learn_curve(LC_PRIOR, 1.0, 0.0).to_bits(), LC_PRIOR.to_bits());
}

#[test]
fn unused_beliefs_fade_toward_a_fresh_prior_and_tiny_ones_drop() {
    let iron = item(Form::Edge, Mat::Iron);
    let kept = Belief { item: iron, slot: 1, n: 1.0, mean: 2.0, var: 0.5, fail: 0.2, labour: 0.3, ..Default::default() };
    // Alone, so the fresh prior is the face-value term (nothing else to learn from).
    let prior = food_prior(&[], kept.slot, kept.item, None, None);
    assert!(near(prior.mean, cut_q(Mat::Iron)));
    let mut table = vec![kept];
    fade_unused(&mut table, &[false]);
    assert_eq!(table.len(), 1);
    let got = &table[0];
    assert!(near(got.n, BELIEF_FADE));
    let lost = 1.0 - BELIEF_FADE;
    assert!(near(got.mean, 2.0 + lost * (prior.mean - 2.0)), "mean {}", got.mean);
    assert!(got.mean < 2.0, "the mean should move toward the prior");
    assert!(near(got.var, 0.5 + lost * (prior.var - 0.5)), "var {}", got.var);
    assert!(near(got.fail, 0.2), "fade does not touch fail");
    assert!(near(got.labour, 0.3));

    let tiny = Belief { item: item(Form::Edge, Mat::Stone), slot: 1, n: 0.12, mean: 1.0, var: 0.2, ..Default::default() };
    let used = Belief { item: item(Form::Edge, Mat::Flint), slot: 1, n: 3.0, mean: 0.7, var: 0.05, flags: FLAG_OWN, ..Default::default() };
    let used_n = used.n.to_bits();
    let used_mean = used.mean.to_bits();
    let mut pair = vec![tiny, used];
    fade_unused(&mut pair, &[false, true]);
    assert_eq!(pair.len(), 1, "n 0.12 fades under 0.1 and is dropped");
    assert_eq!(pair[0].n.to_bits(), used_n);
    assert_eq!(pair[0].mean.to_bits(), used_mean);
}

#[test]
fn eviction_keeps_kit_items_and_the_table_stays_bounded() {
    let knife = item(Form::Edge, Mat::Iron);
    let mut table = Vec::new();
    for i in 0..BELIEF_CAP {
        let stored = remember(
            &mut table,
            Belief { item: knife, slot: i as u8, n: (i as f32) + 1.0, mean: 1.0, ..Default::default() },
            |_| false,
        );
        assert!(stored.stored);
        assert!(table.len() <= BELIEF_CAP);
    }
    assert_eq!(table.len(), BELIEF_CAP);
    let again = remember(&mut table, Belief { item: knife, slot: 0, n: 9.0, ..Default::default() }, |_| false);
    assert!(!again.stored && again.evicted.is_none(), "a duplicate does not evict");
    assert_eq!(table.len(), BELIEF_CAP);

    // slot 0 has the smallest n and is in the kit, so the next-smallest goes
    let gone = remember(&mut table, Belief { item: knife, slot: 100, n: 0.4, mean: 1.0, ..Default::default() }, |b| b.slot == 0);
    assert!(gone.stored);
    let ev = gone.evicted.expect("a non-kit entry was dropped");
    assert_eq!(ev.slot, 1, "smallest n outside the kit");
    assert!(near(ev.n, 2.0));
    assert!(table.iter().any(|b| b.slot == 0 && near(b.n, 1.0)));
    assert_eq!(table.len(), BELIEF_CAP);

    let mut full = (0..BELIEF_CAP)
        .map(|i| Belief { item: knife, slot: i as u8, n: 1.0, ..Default::default() })
        .collect::<Vec<_>>();
    assert!(evict_smallest(&mut full, |_| true).is_none());
    let blocked = remember(&mut full, Belief { item: knife, slot: 200, n: 0.1, ..Default::default() }, |_| true);
    assert!(!blocked.stored && blocked.evicted.is_none());
    assert_eq!(full.len(), BELIEF_CAP);

    let mut many = Vec::new();
    for i in 0..40 {
        let r = remember(&mut many, Belief { item: item(Form::Edge, Mat::Flint), slot: i as u8, n: 0.2, ..Default::default() }, |_| false);
        assert!(many.len() <= BELIEF_CAP);
        if i >= BELIEF_CAP {
            assert!(r.evicted.is_some());
        }
    }
    assert_eq!(many.len(), BELIEF_CAP);
}

/// A first use stores a prior and then one season of the realised multiplier.
/// The novice factor is the draw the economy already takes for that slot.
/// A trial weighs `TRIAL_SHARE`. The first season does not move `learn_curve`.
/// At the year, an unpaid entry fades and a paid one only loses `FLAG_YEAR`.
#[test]
fn payout_forms_a_belief_and_the_year_fades_what_was_not_used() {
    use crate::world::Econ;
    let mut w = world();
    for _ in 0..4 {
        w.step();
    }
    let y = w.history.last().unwrap();
    assert!(y.learn_curve_mean > 1.0, "learn_curve_mean {}", y.learn_curve_mean);

    let mut b = std::mem::take(&mut w.bands[0]);
    b.beliefs.clear();
    b.learn_curve = LC_PRIOR;
    let it = item(Form::Edge, Mat::Flint);
    let trial = item(Form::Edge, Mat::Stone);
    b.d.kit.cut = it;
    b.d.kit.trial[1] = trial;
    let u = band_user(&b)[Form::Edge as usize];
    let noise = novice_factor_u(u, w.cfg.seed, b.id, w.tick, 1);
    let (_, q, _) = recipe_info(&b.d.tk, &b.known, it);
    let x = realised_multiplier(1, it, u, q, noise);
    let prior = food_prior(&[], 1, it, None, None);
    let k = 1.0 / (prior.n + 1.0);
    let mean = prior.mean + k * (x - prior.mean);
    let mut fx = Fx::default();
    let ec = Econ { used: vec![it, trial], trial_on: 1u16 << 1, ..Default::default() };
    w.note_belief_payout(&mut b, &ec, &mut fx);
    assert_eq!(fx.counters.beliefs_formed, 2);
    let got = b.beliefs.iter().find(|e| e.item == it && e.slot == 1).expect("cut belief");
    assert!(near(got.n, PRIOR_N + 1.0), "n {}", got.n);
    assert!(near(got.mean, mean), "mean {} vs {mean}", got.mean);
    assert_eq!(got.flags, FLAG_OWN | FLAG_YEAR);
    assert_eq!((got.fight_n, got.fight_mean, got.fight_var), (0.0, 0.0, 0.0));
    let tr = b.beliefs.iter().find(|e| e.item == trial && e.slot == 1).expect("trial belief");
    assert!(near(tr.n, PRIOR_N + TRIAL_SHARE), "trial n {}", tr.n);
    assert_eq!(b.learn_curve.to_bits(), LC_PRIOR.to_bits());

    let iron = item(Form::Edge, Mat::Iron);
    let copper = item(Form::Edge, Mat::Copper);
    b.beliefs.push(Belief { item: iron, slot: 1, n: 1.0, mean: 2.0, var: 0.5, flags: 0, ..Default::default() });
    b.beliefs.push(Belief { item: copper, slot: 1, n: 0.12, mean: 1.0, var: 0.2, flags: 0, ..Default::default() });
    let iron_i = b.beliefs.iter().position(|e| e.item == iron).unwrap();
    let p = food_prior(&b.beliefs, 1, iron, None, Some(iron_i));
    let iron_mean = 2.0 + (1.0 - BELIEF_FADE) * (p.mean - 2.0);
    w.bands[0] = b;
    w.fade_year_beliefs();
    let kept = w.bands[0].beliefs.iter().find(|e| e.item == it).unwrap();
    assert_eq!(kept.flags & FLAG_YEAR, 0);
    assert!(kept.flags & FLAG_OWN != 0);
    assert!(near(kept.mean, mean));
    assert!(w.bands[0].beliefs.iter().all(|e| e.item != copper), "n 0.12 fades under 0.1");
    let faded = w.bands[0].beliefs.iter().find(|e| e.item == iron).unwrap();
    assert!(near(faded.n, BELIEF_FADE), "faded n {}", faded.n);
    assert!(near(faded.mean, iron_mean), "faded mean {} vs {iron_mean}", faded.mean);

    let mut b = std::mem::take(&mut w.bands[0]);
    let curve0 = b.learn_curve;
    let mean0 = b.beliefs.iter().find(|e| e.item == it).unwrap().mean;
    let mut fx2 = Fx::default();
    let ec2 = Econ { used: vec![it], ..Default::default() };
    w.note_belief_payout(&mut b, &ec2, &mut fx2);
    assert_eq!(fx2.counters.beliefs_formed, 0, "the second season updates, it does not form");
    let got = b.beliefs.iter().find(|e| e.item == it).unwrap();
    assert!(near(got.n, PRIOR_N + 2.0), "n {}", got.n);
    let expect_c = fold_learn_curve(curve0, mean0, got.mean, PRIOR_N + 1.0, PRIOR_N + 2.0);
    assert_eq!(b.learn_curve.to_bits(), expect_c.to_bits(), "curve {} vs {expect_c}", b.learn_curve);
}

/// A daughter keeps the parent's beliefs with a smaller `n`, a wider variance,
/// the same fight fields and the parent's learning curve.
#[test]
fn daughter_inherits_weaker_wider_beliefs() {
    let parent = Belief {
        item: item(Form::Edge, Mat::Flint),
        slot: 1,
        flags: FLAG_OWN | FLAG_YEAR,
        n: BELIEF_NMAX,
        mean: 2.0,
        var: 0.1,
        fail: 0.05,
        labour: 0.4,
        fight_n: 1.5,
        fight_mean: 0.7,
        fight_var: 0.02,
    };
    let mut beliefs = Vec::new();
    let mut curve = LC_PRIOR;
    let n = inherit_beliefs(&mut beliefs, &mut curve, &[parent.clone()], 1.7);
    assert_eq!(n, 1);
    assert_eq!(curve.to_bits(), 1.7f32.to_bits());
    let d = &beliefs[0];
    assert!(near(d.n, BELIEF_NMAX * BELIEF_INHERIT_N), "n {}", d.n);
    assert!(near(d.var, 0.1 * (1.0 + BELIEF_WIDEN)), "var {}", d.var);
    assert_eq!(d.mean.to_bits(), 2.0f32.to_bits());
    assert_eq!(d.flags, FLAG_OWN | FLAG_YEAR | FLAG_INHERITED);
    assert_eq!((d.fight_n, d.fight_mean, d.fight_var), (1.5, 0.7, 0.02));
    let mut empty = vec![parent];
    let mut curve0 = 1.7;
    assert_eq!(inherit_beliefs(&mut empty, &mut curve0, &[], 0.0), 0);
    assert!(empty.is_empty());
    assert_eq!(curve0.to_bits(), LC_PRIOR.to_bits());
}

/// Testimony pulls the mean toward the teller, widens the teller's variance,
/// and never pushes `n` past `TELL_NMAX`. The weight is under one season.
#[test]
fn testimony_moves_the_mean_and_caps_n() {
    let flint = item(Form::Edge, Mat::Flint);
    let ship = item(Form::Ship, Mat::Iron);
    let told = Belief { item: flint, slot: 1, n: BELIEF_NMAX, mean: 3.0, var: 0.04, fail: 0.2, labour: 0.3, fight_n: 4.0, ..Default::default() };
    let w = TELL_W * told.n / BELIEF_NMAX;
    assert!(w < 1.0 && w > 0.0, "hearsay weighs less than a season: {w}");
    let mut beliefs = Vec::new();
    let (ok, evicted) = accept_testimony(&mut beliefs, &told, w, |_| false);
    assert!(ok && !evicted);
    let g = &beliefs[0];
    assert!(near(g.n, w), "n {}", g.n);
    assert!(near(g.mean, 3.0), "mean {}", g.mean);
    assert!(near(g.var, 0.04 * (1.0 + BELIEF_WIDEN)), "var {}", g.var);
    assert!(near(g.fail, 0.2) && near(g.labour, 0.3));
    assert_eq!(g.flags, FLAG_TOLD);
    assert_eq!((g.fight_n, g.fight_mean, g.fight_var), (0.0, 0.0, 0.0), "a new hear does not copy fight fields");
    let (ok2, _) = accept_testimony(&mut beliefs, &told, w, |_| false);
    assert!(ok2);
    assert!(near(beliefs[0].n, (w + w).min(TELL_NMAX)), "second hear {}", beliefs[0].n);
    let (ok3, _) = accept_testimony(&mut beliefs, &told, w, |_| false);
    assert!(ok3);
    assert!(near(beliefs[0].n, TELL_NMAX), "capped {}", beliefs[0].n);
    assert!(near(beliefs[0].mean, 3.0));

    let mut owned = vec![Belief { item: flint, slot: 1, flags: FLAG_OWN, n: 4.0, mean: 1.0, var: 0.2, ..Default::default() }];
    let k = w / (4.0 + w);
    let expect = 1.0 + k * (3.0 - 1.0);
    accept_testimony(&mut owned, &told, w, |_| false);
    assert!(near(owned[0].mean, expect), "mean {} vs {expect}", owned[0].mean);
    assert!(near(owned[0].n, 4.0), "own use stays above the hearsay cap");
    assert_eq!(owned[0].flags, FLAG_OWN | FLAG_TOLD);

    let louder = Belief { item: ship, slot: 22, n: 8.0, mean: 9.0, var: 0.1, ..Default::default() };
    let src = vec![louder, told.clone()];
    let picked = pick_testimony(&src, &[], |it| it == flint);
    assert_eq!(picked, Some(1), "an item the listener cannot make is not told");
    let used_more = vec![Belief { item: flint, slot: 1, n: BELIEF_NMAX, ..Default::default() }];
    assert_eq!(pick_testimony(&[told.clone()], &used_more, |_| true), None, "the listener has already used it as much");
    let tied = vec![
        Belief { item: flint, slot: 1, n: 2.0, mean: 1.0, ..Default::default() },
        Belief { item: item(Form::Edge, Mat::Stone), slot: 1, n: 2.0, mean: 1.0, ..Default::default() },
    ];
    assert_eq!(pick_testimony(&tied, &[], |_| true), Some(0), "an equal gap keeps the earlier entry");
}

/// The meeting hands over the item the listener can make, not the one it cannot,
/// and does not move `learn_curve` or the fight fields.
#[test]
fn pass_belief_tells_only_what_the_listener_can_make() {
    let mut w = world();
    let it = item(Form::Edge, Mat::Flint);
    w.bands[1].d.kit.cut = it;
    w.bands[1].d.tk.producible.add(it);
    assert!(material_term(1, it) > 0.0);
    let ship = item(Form::Ship, Mat::Iron);
    assert!(!w.bands[1].d.tk.producible.has(ship));
    let curve = w.bands[1].learn_curve;
    w.bands[0].beliefs = vec![
        Belief { item: ship, slot: 22, n: 8.0, mean: 9.0, var: 0.1, ..Default::default() },
        Belief { item: it, slot: 1, n: 4.0, mean: 3.0, var: 0.04, fail: 0.1, labour: 0.2, fight_n: 2.0, ..Default::default() },
    ];
    w.bands[1].beliefs.clear();
    let mut src = std::mem::take(&mut w.bands[0]);
    let mut dst = std::mem::take(&mut w.bands[1]);
    let mut fx = Fx::default();
    {
        let mut pair = Pair(&mut src, &mut dst);
        w.pass_belief(&mut pair, 0, 1, &mut fx);
    }
    assert_eq!(fx.counters.beliefs_told, 1);
    assert!(dst.beliefs.iter().all(|e| e.item != ship));
    let g = dst.beliefs.iter().find(|e| e.item == it).expect("the edge was told");
    let wgt = TELL_W * 4.0 / BELIEF_NMAX;
    assert!(near(g.n, wgt), "n {}", g.n);
    assert!(near(g.mean, 3.0));
    assert_eq!(g.flags, FLAG_TOLD);
    assert_eq!(g.fight_n, 0.0);
    assert_eq!(dst.learn_curve.to_bits(), curve.to_bits());
}

fn two_knives() -> (Item, Item, ItemSet) {
    let flint = item(Form::Edge, Mat::Flint);
    let copper = item(Form::Edge, Mat::Copper);
    let mut a = ItemSet::default();
    a.add(flint);
    a.add(copper);
    (flint, copper, a)
}

fn cut_of(beliefs: &[Belief], avail: &ItemSet) -> Kit {
    let info = |_: Item| (1.0f32, 0.0f32, 0u8);
    make_kit_beliefs(avail, &info, &BandView::nominal(), &|_| 1.0, beliefs)
}

fn held(it: Item, n: f32, mean: f32, flags: u8) -> Belief {
    Belief { item: it, slot: 1, n, mean, labour: 0.05, fail: 0.0, flags, ..Default::default() }
}

/// An empty table is the old score. A lucky own-use mean takes the slot; an unlucky one does not.
#[test]
fn luck_shifts_the_knife_and_an_empty_table_stays_put() {
    let (flint, copper, avail) = two_knives();
    let info = |_: Item| (1.0f32, 0.0f32, 0u8);
    let view = BandView::nominal();
    let old = make_kit_scored(&avail, &info, &view, &|_| 1.0);
    let empty = make_kit_beliefs(&avail, &info, &view, &|_| 1.0, &[]);
    assert_eq!(old.cut, empty.cut);
    assert_eq!(old.hunt, empty.hunt);
    assert_eq!(old.game_m.to_bits(), empty.game_m.to_bits());
    assert_eq!(old.farm_m.to_bits(), empty.farm_m.to_bits());

    let unlucky = cut_of(&[held(flint, 2.0, 1.0, FLAG_OWN), held(copper, 2.0, 0.2, FLAG_OWN)], &avail);
    assert_eq!(unlucky.cut, flint, "a low copper mean keeps the flint knife");
    let lucky = cut_of(&[held(flint, 2.0, 1.0, FLAG_OWN), held(copper, 2.0, 3.0, FLAG_OWN)], &avail);
    assert_eq!(lucky.cut, copper, "the same copper, lucky, takes the slot");
    assert_eq!(lucky.trial[1], NONE, "both are adoptable, so this is a switch not a trial");
}

/// A told belief cannot displace an adopted knife. An inherited one, with high n, can.
#[test]
fn told_belief_cannot_switch_and_an_inherited_one_can() {
    let (flint, copper, avail) = two_knives();
    let told = cut_of(&[held(flint, 4.0, 0.5, FLAG_OWN), held(copper, TELL_NMAX, 5.0, FLAG_TOLD)], &avail);
    assert_eq!(told.cut, flint);
    assert_eq!(told.trial[1], copper, "hearsay is only a trial");
    let inherited = cut_of(&[held(flint, 4.0, 0.5, FLAG_OWN), held(copper, BELIEF_NMAX * BELIEF_INHERIT_N, 5.0, FLAG_INHERITED)], &avail);
    assert_eq!(inherited.cut, copper, "inherited n stays above the adopt line");
    assert_eq!(inherited.trial[1], NONE);
}

/// Maker skill and the quality of a knife the band has no belief for do not move the kit.
/// The kit moves when the observed mean does.
#[test]
fn unseen_maker_skill_and_quality_do_not_move_the_kit() {
    let (flint, copper, mut avail) = two_knives();
    let stone = item(Form::Edge, Mat::Stone);
    avail.add(stone);
    let beliefs = vec![held(flint, 3.0, 1.2, FLAG_OWN), held(stone, 3.0, 0.4, FLAG_OWN)];
    let view = BandView::nominal();
    let info = |q: f32| {
        move |it: Item| {
            if it == copper { (1.0f32, q, 0u8) } else { (1.0, 0.0, 0) }
        }
    };
    let base = make_kit_beliefs(&avail, &info(0.0), &view, &|_| 1.0, &beliefs);
    let weak = make_kit_beliefs(&avail, &info(0.0), &view, &|it| if it == copper || it == stone { 0.0 } else { 1.0 }, &beliefs);
    let fine = make_kit_beliefs(&avail, &info(0.9), &view, &|_| 1.0, &beliefs);
    let mut novice = view.clone();
    novice.user[Form::Edge as usize] = 0.05;
    novice.user[Form::Spear as usize] = 0.02;
    let shifted = make_kit_beliefs(&avail, &info(0.0), &novice, &|_| 1.0, &beliefs);
    assert_eq!(base.cut, flint);
    assert_eq!((weak.cut, weak.hunt, weak.trial), (base.cut, base.hunt, base.trial));
    assert_eq!((fine.cut, fine.hunt, fine.trial), (base.cut, base.hunt, base.trial));
    assert_eq!((shifted.cut, shifted.hunt, shifted.trial), (base.cut, base.hunt, base.trial), "user skill is not the belief");
    let mut paid = beliefs.clone();
    paid.iter_mut().find(|e| e.item == stone).unwrap().mean = 5.0;
    let after = make_kit_beliefs(&avail, &info(0.0), &view, &|_| 1.0, &paid);
    assert_eq!(after.cut, stone, "only the observed mean moves the knife");
}

/// The belief food choice does not name the true-effects account, nor maker waste or failure.
#[test]
fn belief_choice_does_not_call_tool_effects() {
    let src = include_str!("effects.rs");
    for name in ["fn choose_food_belief", "fn food_from_mean", "fn quote_of", "fn paint_belief_context", "fn make_kit_beliefs"] {
        let body = fn_source(src, name);
        assert!(!body.contains("tool_effects"), "{name}");
        assert!(!body.contains("maker_p_fail"), "{name}");
        assert!(!body.contains("maker_waste"), "{name}");
    }
}

fn fn_source(src: &str, name: &str) -> String {
    let start = src.find(name).unwrap_or_else(|| panic!("missing {name}"));
    let mut out = String::new();
    for (i, line) in src[start..].lines().enumerate() {
        if i > 0 && (line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("pub(crate) fn ") || line.starts_with("struct ") || line.starts_with("impl ")) {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Below the adopt line a better knife is only the trial. Fade pulls a written-off one back there.
#[test]
fn fade_returns_a_writeoff_as_a_trial() {
    // Obsidian's edge outranks stone on the visible term, so a stone belief pulls
    // obsidian's prior above the stone mean. Fade walks the written-off mean there.
    let stone = item(Form::Edge, Mat::Stone);
    let glass = item(Form::Edge, Mat::Obsidian);
    let mut avail = ItemSet::default();
    avail.add(stone);
    avail.add(glass);
    let mut beliefs = vec![held(stone, BELIEF_NMAX, 0.4, FLAG_OWN), held(glass, PRIOR_N + BELIEF_ADOPT_N, 0.02, FLAG_OWN | FLAG_WROTE_OFF)];
    let still = cut_of(&beliefs, &avail);
    assert_eq!(still.cut, stone);
    assert_ne!(still.trial[1], glass, "still adoptable, and worse, so not the trial");
    let mut saw = false;
    for _ in 0..8 {
        fade_unused(&mut beliefs, &[true, false]);
        let n_now = beliefs.iter().find(|e| e.item == glass).map(|e| e.n);
        let Some(n_now) = n_now else { break };
        if n_now >= PRIOR_N + BELIEF_ADOPT_N {
            continue;
        }
        let k = cut_of(&beliefs, &avail);
        if k.trial[1] == glass {
            assert_eq!(k.cut, stone, "a faded write-off is trialled, not adopted");
            saw = true;
            break;
        }
    }
    assert!(saw, "a few years of fade put the written-off knife back on trial");
    assert_eq!(KIT_TRIAL_P, 0.2, "the trial still fires at the old chance");
}

/// Own use, inside the window, below the prior: a write-off. A founder flag, a late drop, and a
/// good mean are not. The year counts the fresh bit once. A later trial clears the sticky bit.
#[test]
fn writeoff_window_and_a_later_trial() {
    use crate::world::Econ;
    let flint = item(Form::Edge, Mat::Flint);
    let stone = item(Form::Edge, Mat::Stone);
    let mut old = Kit::default();
    old.cut = flint;
    let mut new_k = Kit::default();
    new_k.cut = stone;
    let early = Belief { item: flint, slot: 1, n: PRIOR_N + 1.0, mean: 0.01, flags: FLAG_OWN, ..Default::default() };
    let mut beliefs = vec![early];
    mark_writeoffs(&mut beliefs, &old, &new_k);
    assert_eq!(beliefs[0].flags & (FLAG_WROTE_OFF | FLAG_WO_FRESH), FLAG_WROTE_OFF | FLAG_WO_FRESH);
    beliefs[0].flags &= !FLAG_WO_FRESH;
    mark_writeoffs(&mut beliefs, &old, &new_k);
    assert_eq!(beliefs[0].flags & FLAG_WO_FRESH, 0, "a second look does not count it again");

    let mut late = vec![Belief { item: flint, slot: 1, n: PRIOR_N + WRITE_OFF_SEASONS as f32 + 1.0, mean: 0.01, flags: FLAG_OWN, ..Default::default() }];
    mark_writeoffs(&mut late, &old, &new_k);
    assert_eq!(late[0].flags & FLAG_WROTE_OFF, 0);
    let mut founder = vec![Belief { item: flint, slot: 1, n: BELIEF_NMAX, mean: 0.01, flags: 0, ..Default::default() }];
    mark_writeoffs(&mut founder, &old, &new_k);
    assert_eq!(founder[0].flags, 0, "founder fill is not own use");
    let prior = food_prior(&[], 1, flint, None, None);
    let mut good = vec![Belief { item: flint, slot: 1, n: PRIOR_N + 1.0, mean: prior.mean + 1.0, flags: FLAG_OWN, ..Default::default() }];
    mark_writeoffs(&mut good, &old, &new_k);
    assert_eq!(good[0].flags & FLAG_WROTE_OFF, 0);

    assert_eq!(claim_fresh_writeoffs(&mut beliefs), 0);
    beliefs[0].flags |= FLAG_WO_FRESH;
    assert_eq!(claim_fresh_writeoffs(&mut beliefs), 1);
    assert_eq!(beliefs[0].flags & FLAG_WO_FRESH, 0);
    assert_ne!(beliefs[0].flags & FLAG_WROTE_OFF, 0);

    let mut w = world();
    let mut b = std::mem::take(&mut w.bands[0]);
    b.d.kit.cut = stone;
    b.d.kit.trial[1] = flint;
    b.beliefs = vec![
        Belief { item: stone, slot: 1, n: 4.0, mean: 1.0, labour: 0.05, flags: FLAG_OWN, ..Default::default() },
        Belief { item: flint, slot: 1, n: PRIOR_N + 1.0, mean: 0.2, labour: 0.05, flags: FLAG_OWN | FLAG_WROTE_OFF | FLAG_WO_FRESH, ..Default::default() },
    ];
    let mut fx = Fx::default();
    let ec = Econ { used: vec![stone], trial_on: 1u16 << 1, ..Default::default() };
    w.note_belief_payout(&mut b, &ec, &mut fx);
    assert_eq!(fx.counters.tools_retried, 1);
    let got = b.beliefs.iter().find(|e| e.item == flint).unwrap();
    assert_eq!(got.flags & FLAG_WROTE_OFF, 0);
    assert_eq!(got.flags & FLAG_WO_FRESH, FLAG_WO_FRESH);
}

/// The gauge shrinks as the mean walks toward the multiplier the economy would expect.
#[test]
fn belief_err_falls_as_n_grows() {
    let mut w = world();
    for b in &mut w.bands {
        b.beliefs.clear();
    }
    let it = item(Form::Edge, Mat::Flint);
    let b = &mut w.bands[0];
    b.gen = 1;
    b.use_skill = vec![1.0; NFORM];
    b.d.kit.cut = it;
    let user = band_user(b);
    let (_, q, _) = recipe_info(&b.d.tk, &b.known, it);
    let exp = expected_multiplier(1, it, user[Form::Edge as usize], q);
    assert!(exp > 1e-6, "exp {exp}");
    b.beliefs = vec![Belief { item: it, slot: 1, n: 1.0, mean: exp * 0.25, flags: FLAG_OWN, ..Default::default() }];
    let e0 = belief_err_x1000(&w.bands);
    {
        let b = &mut w.bands[0];
        for _ in 0..6 {
            update_belief(&mut b.beliefs[0], exp, 1.0, false, 0.05);
        }
    }
    let e1 = belief_err_x1000(&w.bands);
    assert!(e0 > e1, "err {e0} -> {e1}");
    assert!(e1 < e0 / 2, "err {e0} -> {e1}");
}
