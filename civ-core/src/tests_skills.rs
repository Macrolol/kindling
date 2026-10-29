//! (skills step 1) The economy pays what `tool_effects` says, and a food slot is the item
//! that maximises food minus the food its upkeep labour would have grown.
use crate::band::{edge_q, Band, Kit};
use crate::band::{KIT_UPKEEP, QUALITY_YIELD};
use crate::effects::{
    harvest_for, kit_yields, make_kit_with_view, pitch_in_use, recipe_info, sealed_factor, slot_score, tool_effects, tool_upkeep, BandView,
    HARVEST_EPS, HARVEST_PRIOR, LABOUR_SHARE, NOMINAL_MIX, WORK_SHARE,
};
use crate::materials::*;
use crate::{Config, Params, World};

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
    let y = kit_yields(kit, &info);
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
        let yw = kit_yields(kit, &info);
        let ye = kit_yields(&bare, &info);
        let dg = yw.digest;
        let hd: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
        assert!(hd > HARVEST_EPS);
        let t: f32 = (0..5).map(|r| view.harvest[r]).sum();
        let mut h = [0.0; 5];
        for r in 0..5 {
            h[r] = (1.0 - HARVEST_PRIOR) * view.harvest[r] + HARVEST_PRIOR * NOMINAL_MIX[r] * t;
        }
        let mut food = 0.0;
        for r in 0..5 {
            let (m1, m0) = (mult(&yw, r), mult(&ye, r));
            food += h[r] * dg[r] * (m1 - m0) / m0;
        }
        let mask = b.d.tk.via(it).map(|v| v.1).unwrap_or(0);
        let fx = tool_effects(&view, kit, slot, it, &info, mask);
        assert!((fx.food - food).abs() < 1e-5, "slot {slot} effects {} formula {food}", fx.food);
        assert_eq!(fx.food_sd, 0.0);
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
    let yw = kit_yields(&k, &info);
    let ye = kit_yields(&bare, &info);
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
    let t: f32 = (0..5).map(|r| view.harvest[r]).sum();
    let mut want = [0.0; 5];
    for r in 0..5 {
        want[r] = (1.0 - HARVEST_PRIOR) * view.harvest[r] + HARVEST_PRIOR * NOMINAL_MIX[r] * t;
    }
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
    let (sk, ql, d) = info(it);
    if matches!(slot, 0 | 1 | 10 | 4 | 12 | 13) {
        let mut empty = *kit;
        set_pay(&mut empty, slot, NONE);
        let ye = kit_yields(&empty, info);
        let mut with = empty;
        set_pay(&mut with, slot, it);
        let yw = kit_yields(&with, info);
        let dg = yw.digest;
        let sum: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
        let h = if sum > HARVEST_EPS {
            let t: f32 = (0..5).map(|r| view.harvest[r]).sum();
            let mut h = [0.0; 5];
            for r in 0..5 {
                h[r] = (1.0 - HARVEST_PRIOR) * view.harvest[r] + HARVEST_PRIOR * NOMINAL_MIX[r] * t;
            }
            h
        } else {
            NOMINAL_MIX.map(|x| x * view.pop.max(1.0))
        };
        let mut food = 0.0;
        for r in 0..5 {
            let (m1, m0) = (mult(&yw, r), mult(&ye, r));
            food += h[r] * dg[r] * (m1 - m0) / m0;
        }
        let labour = tool_upkeep(view.upkeep_rate, it, d, sealed_factor(it, &with, pitch_in_use(&with, view.wet)));
        let pop = view.pop.max(1.0);
        let units = labour * LABOUR_SHARE * pop;
        let price = view.food.max(1e-6) / (WORK_SHARE * LABOUR_SHARE * pop);
        let cost = price * units;
        (food * (0.5 + 0.5 * sk) - cost, food - cost)
    } else {
        let m = mat(it);
        let up = KIT_UPKEEP * (1.15 - durability(m)) * (1.0 + 0.3 * d as f32);
        let qv = match slot {
            11 | 16 | 18 | 22 => boat_q(m),
            14 | 15 => m.props().tough + 1.0,
            _ => 0.0,
        };
        let gain = qv * (1.0 + QUALITY_YIELD * ql);
        (gain * (0.5 + 0.5 * sk) - up, gain - up)
    }
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
        BandView { harvest: [0.0; 5], food: 0.0, pop: 8.0, upkeep_rate: 0.02, wet: true, at_hand: 0 },
        BandView { harvest: [2.0, 40.0, 1.0, 5.0, 0.0], food: 30.0, pop: 40.0, upkeep_rate: 0.02, wet: true, at_hand: 0 },
        BandView { harvest: [10.0, 0.0, 25.0, 0.0, 8.0], food: 12.0, pop: 1.0, upkeep_rate: 0.05, wet: false, at_hand: 0 },
        BandView { harvest: [0.0, 0.0, 0.0, 0.0, 0.0], food: 3.0, pop: 200.0, upkeep_rate: 0.02, wet: true, at_hand: 0 },
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
