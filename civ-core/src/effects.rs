//! (skills step 1) What a tool does for a band, and the kit choice that reads it.
//!
//! The economy's yields and the decision used to disagree: blades were ranked by `edge_q`
//! (sharp and lasting) while the food they bring depends only on `cut_q`, and toughness was
//! counted again as upkeep. `tool_effects` is the one account. For a given kit the multipliers
//! are exactly today's (`apply_base` then `apply_quality`); only which item fills a slot changes.
//!
//! Food slots are ranked by food gained minus the food the tool's upkeep labour would have
//! grown. The skill discount stays on that decision (it is fixed at step 2, not here). `edge_q`
//! is not used. Non-food payoff slots keep the old `KIT_UPKEEP` ranking.
use crate::band::{edge_q, has_bow, Kit, KIT_UPKEEP, QUALITY_BOAT, QUALITY_PROTECT, QUALITY_YIELD};
use crate::knowledge::{Known, Toolkit};
use crate::materials::*;
use crate::params::Params;
use crate::world::{PITCH_CAULK, PITCH_GLUE};

/// Share of the people who work. `World::economy` sets `labour = LABOUR_SHARE * pop`.
pub const LABOUR_SHARE: f32 = 0.55;
/// Of that labour, the share that can go to feeding (`max_e` uses `labour * WORK_SHARE`).
pub const WORK_SHARE: f32 = 0.85;

/// Below this, last season's harvest counts as "no history" and the nominal mix is used.
pub const HARVEST_EPS: f32 = 1e-3;
/// Per-person mix when a band has no harvest yet: plants, game, fish, small game, crops.
/// Sums to 1, so `NOMINAL_MIX * pop` is `pop` units of food before digest.
pub const NOMINAL_MIX: [f32; 5] = [0.3, 0.3, 0.25, 0.15, 0.0];
const NOMINAL_POP: f32 = 25.0;
const FOOD_EPS: f32 = 1e-6;

/// best[] index of each payoff slot, in `Kit::slot_item` order.
pub(crate) const PAYOFF_SLOTS: [usize; 12] = [0, 1, 10, 4, 11, 12, 13, 14, 15, 16, 18, 22];
/// Food slots, then the rest. Pass 1 fills each from an empty payoff kit; pass 2 re-chooses
/// each with the others already set. A cut is optimal on the final kit (nothing chosen after
/// it changes its food). Hunt is not: cut is filled again after hunt.
const CHOOSE_ORDER: [usize; 12] = [0, 1, 10, 4, 12, 13, 11, 14, 15, 16, 18, 22];

/// What putting `it` in a slot does, beyond the kit with that slot empty.
/// `food_sd` and `p_fail` stay 0 until a later step. `fight` is informational: the decision
/// does not read it. `labour` is the economy's fraction of the labour pool (the economy
/// multiplies by `LABOUR_SHARE * pop`). `mats` is the marginal stock draw, up to four
/// materials, index order, unused entries `(0, 0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    pub food: f32,
    pub food_sd: f32,
    pub labour: f32,
    pub mats: [(u8, f32); 4],
    pub p_fail: f32,
    pub fight: f32,
}

/// What the decision may read. All of it is saved (`Derived` harvest and food, pop) or
/// re-derived (`near_water`, `mat_at_hand` with this season's hunt still unknown).
#[derive(Clone, Copy, Debug)]
pub struct BandView {
    pub harvest: [f32; 5],
    pub food: f32,
    pub pop: f32,
    /// `Params::upkeep` for this world.
    pub upkeep_rate: f32,
    pub wet: bool,
    /// Bit `i` set when material `i` is at hand. Hide and bone are not, unless a deposit
    /// says so: this season's hunt has not happened yet (`mat_at_hand` with `hunted = 0`).
    pub at_hand: u32,
}

impl BandView {
    /// Callers with no band (`make_kit` in tests). Food equals pop, so the price is finite.
    pub fn nominal() -> Self {
        let pop = NOMINAL_POP;
        BandView {
            harvest: NOMINAL_MIX.map(|x| x * pop),
            food: pop,
            pop,
            upkeep_rate: Params::default().upkeep,
            wet: false,
            at_hand: 0,
        }
    }
}

/// (skill, quality, chain depth) of the recipe behind an item, as the kit choice reads it.
pub fn recipe_info(tk: &Toolkit, known: &[Known], it: Item) -> (f32, f32, u8) {
    match tk.via(it) {
        Some((i, _, d)) if i != u16::MAX => {
            known.get(i as usize).map(|kn| (kn.skill.clamp(0.0, 1.0), kn.quality.clamp(0.0, 1.0), d)).unwrap_or((1.0, 0.0, d))
        }
        Some((_, _, d)) => (1.0, 0.0, d),
        None => (1.0, 0.0, 0),
    }
}

/// Pitch is boiled only for a hull by the water, or a jar to seal. While it is about, hafts
/// are glued too. Same predicate the economy uses.
pub fn pitch_in_use(kit: &Kit, wet: bool) -> bool {
    let craft = [kit.boat, kit.outrigger, kit.sail, kit.plank, kit.ship].iter().any(|&i| i != NONE);
    kit.pitch != NONE && ((wet && craft) || kit.jar != NONE)
}

/// Caulk on a hull, glue on a haft, otherwise 1. `pitch` is `pitch_in_use` of the kit the
/// item is worn in (the kit *with* the candidate, so a boat chosen already can glue a spear).
pub fn sealed_factor(it: Item, kit: &Kit, pitch: bool) -> f32 {
    if !pitch {
        return 1.0;
    }
    let craft = it == kit.boat || it == kit.outrigger || it == kit.sail || it == kit.plank || it == kit.ship;
    if craft {
        PITCH_CAULK
    } else if it == kit.hunt || it == kit.axe {
        PITCH_GLUE
    } else {
        1.0
    }
}

/// One item's upkeep in the economy's units (a fraction of the labour pool, before
/// `* LABOUR_SHARE * pop`). Ornaments are kept for show. A fire or a cooked meal is the
/// flat 0.01. Anything else is `sealed * rate * (1.15 - durability) * (1 + 0.3 * depth)`.
///
/// Trials call this with `sealed = 1`. They never got the pitch discount, and a given kit's
/// trial wear stays what it was.
pub fn tool_upkeep(rate: f32, it: Item, depth: u8, sealed: f32) -> f32 {
    if it == NONE || form(it) == Form::Ornament {
        return 0.0;
    }
    if matches!(form(it), Form::Fire | Form::Cooked) {
        return 0.01;
    }
    sealed * rate * (1.15 - durability(mat(it))) * (1.0 + 0.3 * depth as f32)
}

fn is_food_slot(slot: usize) -> bool {
    matches!(slot, 0 | 1 | 10 | 4 | 12 | 13)
}

/// `Kit::slot_item` index of a payoff best[] slot.
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

fn set_slot(k: &mut Kit, slot: usize, it: Item) {
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

fn mult(k: &Kit, r: usize) -> f32 {
    match r {
        0 => k.plants_m,
        1 => k.game_m,
        2 => k.fish_m,
        3 => k.small_m,
        _ => k.farm_m,
    }
}

/// Harvest the food term is linearised around. A season with no digested harvest (a band just
/// loaded from before harvest was saved, or not yet run) uses the nominal mix scaled by *this*
/// band's pop, so food and labour move together.
fn harvest_for(view: &BandView, dg: &[f32; 5]) -> [f32; 5] {
    let sum: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
    if sum > HARVEST_EPS {
        view.harvest
    } else {
        let pop = view.pop.max(1.0);
        NOMINAL_MIX.map(|x| x * pop)
    }
}

fn food_delta(view: &BandView, with: &Kit, empty: &Kit) -> f32 {
    // Digest does not move when a food-slot item changes (it hangs off cooking, the pot and
    // the hammer). Use the with-kit's digest; the resources are summed in index order.
    let dg = with.digest;
    let h = harvest_for(view, &dg);
    let mut food = 0.0;
    for r in 0..5 {
        let m0 = mult(empty, r);
        let m1 = mult(with, r);
        food += h[r] * dg[r] * (m1 - m0) / m0;
    }
    food
}

/// Digest from cooking, the pot and the hammer. Food-slot items do not touch it, so a kit
/// choice computes it once. Same expression as `apply_base`.
fn digest_of(k: &Kit) -> [f32; 5] {
    let ceramic = if k.vessel != NONE && mat(k.vessel) == Mat::Ceramic { 1.0 } else { 0.0 };
    let hammer = if k.hammer == NONE { 0.0 } else { 1.0 };
    let ck = |j: usize, g: f32| if k.cooked[j] != NONE { g } else { 1.0 };
    let grain = if k.cooked_grain != NONE { 1.3 + 0.15 * ceramic + 0.1 * hammer } else { 1.0 + 0.1 * hammer };
    [ck(0, 1.25 + 0.15 * ceramic), ck(1, 1.2), ck(2, 1.15), ck(1, 1.2), grain]
}

/// `h[r] * dg[r]` for `food_delta`, in the same multiply, so a later
/// `w[r] * (m1 - m0) / m0` is `((h * dg) * (m1 - m0)) / m0`.
fn weights(view: &BandView, dg: &[f32; 5]) -> [f32; 5] {
    let h = harvest_for(view, dg);
    let mut w = [0.0; 5];
    for r in 0..5 {
        w[r] = h[r] * dg[r];
    }
    w
}

fn food_acc(w: &[f32; 5], m0: &[f32; 5], m1: &[f32; 5]) -> f32 {
    let mut food = 0.0;
    for r in 0..5 {
        food += w[r] * (m1[r] - m0[r]) / m0[r];
    }
    food
}

fn qual<F: Fn(Item) -> (f32, f32, u8)>(info: &F, it: Item) -> f32 {
    if it == NONE { 0.0 } else { info(it).1 }
}

fn hunt_q_of(it: Item) -> f32 {
    if it == NONE {
        0.0
    } else {
        let m = mat(it);
        match form(it) {
            Form::Spear => 0.35 + 0.65 * cut_q(m),
            Form::Point => 0.35 * cut_q(m),
            _ => 0.2 * cut_q(m),
        }
    }
}

fn cut_term(it: Item) -> f32 {
    if it == NONE { 0.0 } else { cut_q(mat(it)) }
}

fn axe_term(it: Item) -> f32 {
    if it == NONE {
        0.0
    } else {
        let m = mat(it);
        0.5 * cut_q(m) + 0.5 * durability(m)
    }
}

fn mesh_term(it: Item) -> f32 {
    if it == NONE { 0.0 } else { mesh_q(mat(it)) }
}

/// Inputs of the five food multipliers. Built for the kit with the slot under choice empty;
/// a candidate overwrites only the fields that slot owns.
#[derive(Clone, Copy)]
struct YArgs {
    hunt_q: f32,
    cut: f32,
    vq: f32,
    dig: f32,
    axe: f32,
    cord: f32,
    hammer: f32,
    point: f32,
    netq: f32,
    trapq: f32,
    /// `has_bow`: the bow is in the kit and the point slot is filled.
    bow: bool,
    q_hunt: f32,
    q_cut: f32,
    q_net: f32,
    q_trap: f32,
    q_vessel: f32,
    q_dig: f32,
}

/// Plants, game, fish, small, farm. Same operations, in the same order, as `apply_base`
/// (multipliers, then the bow factors) followed by the food lines of `apply_quality`.
#[inline(always)]
fn yields(a: &YArgs) -> [f32; 5] {
    let mut plants = 1.0 + 0.35 * a.dig + 0.3 * a.vq + 0.1 * a.hammer + 0.1 * a.axe;
    let mut game = (1.0 + 1.4 * a.hunt_q + 0.15 * a.cord) * (1.0 + 0.35 * a.cut + 0.05 * a.hammer);
    let mut fish = 1.0 + 0.8 * a.hunt_q + 0.4 * a.vq + 0.3 * a.point + 0.2 * a.cord + 0.9 * a.netq;
    let mut small = 0.35 + 0.4 * a.hunt_q + 1.6 * a.trapq + 0.4 * a.netq;
    if a.bow {
        game *= 1.2;
        small *= 1.15;
    }
    let mut farm = 0.5 + 0.3 * a.dig + 0.35 * a.cut + 0.2 * a.axe;
    let y = |x: f32| 1.0 + QUALITY_YIELD * x;
    game *= y(a.q_hunt);
    fish *= y(a.q_net.max(a.q_hunt));
    small *= y(a.q_trap.max(a.q_net));
    plants *= y(0.5 * (a.q_vessel + a.q_dig));
    farm *= y(0.5 * (a.q_dig + a.q_cut));
    [plants, game, fish, small, farm]
}

/// The kit with payoff `slot` treated as empty. Other slots stay as they are.
fn yargs_empty<F: Fn(Item) -> (f32, f32, u8)>(k: &Kit, slot: usize, info: &F) -> YArgs {
    let hunt = if slot == 0 { NONE } else { k.hunt };
    let cut = if slot == 1 { NONE } else { k.cut };
    let point = if slot == 10 { NONE } else { k.point };
    let axe = if slot == 4 { NONE } else { k.axe };
    let net = if slot == 12 { NONE } else { k.net };
    let trap = if slot == 13 { NONE } else { k.trap };
    YArgs {
        hunt_q: hunt_q_of(hunt),
        cut: cut_term(cut),
        vq: if k.vessel == NONE { 0.0 } else { vessel_q(mat(k.vessel)) },
        dig: if k.dig == NONE { 0.0 } else { 1.0 },
        axe: axe_term(axe),
        cord: if k.cord == NONE { 0.0 } else { 1.0 },
        hammer: if k.hammer == NONE { 0.0 } else { 1.0 },
        point: if point == NONE { 0.0 } else { 1.0 },
        netq: mesh_term(net),
        trapq: mesh_term(trap),
        bow: k.bow != NONE && point != NONE,
        q_hunt: qual(info, hunt),
        q_cut: qual(info, cut),
        q_net: qual(info, net),
        q_trap: qual(info, trap),
        q_vessel: qual(info, k.vessel),
        q_dig: qual(info, k.dig),
    }
}

/// One producible item, with the material terms the choice reuses across passes.
/// `hunt_q` / `term` are exactly the expressions `apply_base` would recompute.
#[derive(Clone, Copy)]
struct Cand {
    it: Item,
    d: u8,
    sk: f32,
    ql: f32,
    hunt_q: f32,
    term: f32,
    dur_term: f32,
    depth_f: f32,
}

fn cand_new(it: Item, sk: f32, ql: f32, d: u8) -> Cand {
    let m = mat(it);
    let dur = durability(m);
    let cq = cut_q(m);
    let (hunt_q, term) = match form(it) {
        Form::Spear => (0.35 + 0.65 * cq, 0.0),
        Form::Point => (0.35 * cq, 0.0),
        Form::Edge => (0.2 * cq, cq),
        Form::Axe => (0.0, 0.5 * cq + 0.5 * dur),
        Form::Net | Form::Trap => (0.0, mesh_q(m)),
        Form::Boat | Form::Outrigger | Form::Plank | Form::Ship => (0.0, boat_q(m)),
        Form::Bow | Form::Shield => (0.0, m.props().tough + 1.0),
        _ => (0.0, 0.0),
    };
    Cand { it, d, sk, ql, hunt_q, term, dur_term: 1.15 - dur, depth_f: 1.0 + 0.3 * d as f32 }
}

#[inline(always)]
fn yields_with(slot: usize, base: &YArgs, c: &Cand, bow_item: bool) -> [f32; 5] {
    let mut a = *base;
    match slot {
        0 => {
            a.hunt_q = c.hunt_q;
            a.q_hunt = c.ql;
        }
        1 => {
            a.cut = c.term;
            a.q_cut = c.ql;
        }
        // every point fills the slot the same way: the multiplier is presence, not material
        10 => {
            a.point = 1.0;
            a.bow = bow_item;
        }
        4 => a.axe = c.term,
        12 => {
            a.netq = c.term;
            a.q_net = c.ql;
        }
        13 => {
            a.trapq = c.term;
            a.q_trap = c.ql;
        }
        _ => {}
    }
    yields(&a)
}

/// Glue on a haft (hunt, axe) when pitch is in use; a knife, point, net or trap is not glued.
/// Same result as `sealed_factor` for an item that fills `slot` (its form cannot also be a hull).
fn food_sealed(slot: usize, pitch: bool) -> f32 {
    if pitch && (slot == 0 || slot == 4) { PITCH_GLUE } else { 1.0 }
}

fn bucket_of(f: Form) -> Option<usize> {
    Some(match f {
        Form::Edge => 0,
        Form::Point => 1,
        Form::Spear => 2,
        Form::Axe => 3,
        Form::Net => 4,
        Form::Trap => 5,
        Form::Boat => 6,
        Form::Bow => 7,
        Form::Shield => 8,
        Form::Outrigger => 9,
        Form::Plank => 10,
        Form::Ship => 11,
        _ => return None,
    })
}

fn food_ix(slot: usize) -> usize {
    match slot {
        0 => 0,
        1 => 1,
        10 => 2,
        4 => 3,
        12 => 4,
        13 => 5,
        _ => 0,
    }
}

/// Whether pass 2 can pick a different item than pass 1. Compared are the other slots that
/// enter this slot's multipliers (or, for hunt and axe, whether pitch started caulking a hull
/// and so glues hafts). The slot's own item is not a dependency: it is cleared before scoring.
fn food_deps_changed(slot: usize, then: &Kit, now: &Kit, pitch_then: bool, pitch_now: bool) -> bool {
    match slot {
        // game: cut, bow×point; fish: point, net; small: net, trap, bow×point; glue: pitch
        0 => {
            then.cut != now.cut
                || then.point != now.point
                || then.net != now.net
                || then.trap != now.trap
                || then.bow != now.bow
                || pitch_then != pitch_now
        }
        // game: hunt, bow×point; farm: axe
        1 => then.hunt != now.hunt || then.point != now.point || then.bow != now.bow || then.axe != now.axe,
        // fish: hunt, net; a bow that gains its point also moves game (cut) and small (trap)
        10 => {
            then.hunt != now.hunt
                || then.net != now.net
                || then.bow != now.bow
                || then.cut != now.cut
                || then.trap != now.trap
        }
        // farm: cut; glue: pitch
        4 => then.cut != now.cut || pitch_then != pitch_now,
        // fish: hunt, point; small: trap, bow×point
        12 => then.hunt != now.hunt || then.point != now.point || then.trap != now.trap || then.bow != now.bow,
        // small: hunt, net, bow×point
        13 => then.hunt != now.hunt || then.net != now.net || then.point != now.point || then.bow != now.bow,
        _ => true,
    }
}

/// Food the upkeep labour would have grown, and the same with the skill discount on the
/// food only. `labour` is the economy's fraction; price is food per unit of working labour.
fn priced(food: f32, labour: f32, sk: f32, view: &BandView) -> (f32, f32) {
    let pop = view.pop.max(1.0);
    let units = labour * LABOUR_SHARE * pop;
    let price = view.food.max(FOOD_EPS) / (WORK_SHARE * LABOUR_SHARE * pop);
    let cost = price * units;
    (food * (0.5 + 0.5 * sk) - cost, food - cost)
}

fn marginal_mats(mask: u32, at_hand: u32) -> [(u8, f32); 4] {
    let mut out = [(0u8, 0.0); 4];
    let mut n = 0;
    for mi in 0..NMAT {
        if mask & (1 << mi) == 0 || at_hand & (1 << mi) != 0 {
            continue;
        }
        let mm = ALL_MATS[mi];
        // The season loop does not draw these. Base wear (0.006 / 0.02) is not a marginal.
        if mm.is_food() || mm == Mat::Charcoal || mm == Mat::Ceramic || mm == Mat::Pitch {
            continue;
        }
        if n >= 4 {
            break;
        }
        let metal = mm.is_metal() || mm == Mat::Gold;
        out[n] = (mi as u8, if metal { 0.02 } else { 0.06 });
        n += 1;
    }
    out
}

/// Fight terms, additive. A knife in the cut slot is not a weapon (`fight == 0`) even when
/// the same edge is also the hunt weapon. Bow and shield add, they do not multiply: both is
/// 0.3 + 0.25, not `1 - loss_guard`. `edge_q` is read only here and in `weapon_q`.
fn fight_of(slot: usize, it: Item, kit: &Kit) -> f32 {
    if it == NONE {
        return 0.0;
    }
    match slot {
        0 => {
            let m = mat(it);
            match form(it) {
                Form::Spear => 0.35 + 0.65 * edge_q(m),
                Form::Point => 0.35 * edge_q(m),
                Form::Edge => 0.2 * edge_q(m),
                _ => 0.0,
            }
        }
        1 => 0.0,
        4 => 0.3 * (0.5 * cut_q(mat(it)) + 0.5 * durability(mat(it))),
        // points already chosen: a bow without them does not keep anyone back
        14 => {
            if kit.point != NONE {
                0.3
            } else {
                0.0
            }
        }
        15 => 0.25,
        _ => 0.0,
    }
}

/// Material term and recipe quality, nothing else (no user or maker factor yet).
/// `info` supplies quality; skill and depth are not applied here.
pub fn kit_yields<F: Fn(Item) -> (f32, f32, u8)>(k: &Kit, info: &F) -> Kit {
    let mut y = *k;
    apply_base(&mut y);
    apply_quality(&mut y, &|it| info(it).1);
    y
}

/// Multipliers from the items in the kit, before recipe quality. The tail of the old
/// `make_kit_with`, same operations in the same order.
pub(crate) fn apply_base(k: &mut Kit) {
    let q = |it: Item, f: &dyn Fn(Mat) -> f32| if it == NONE { 0.0 } else { f(mat(it)) };
    let hunt_q = if k.hunt == NONE {
        0.0
    } else {
        let m = mat(k.hunt);
        match form(k.hunt) {
            Form::Spear => 0.35 + 0.65 * cut_q(m),
            Form::Point => 0.35 * cut_q(m),
            _ => 0.2 * cut_q(m),
        }
    };
    let cut = q(k.cut, &cut_q);
    let vq = q(k.vessel, &vessel_q);
    let dig = if k.dig == NONE { 0.0 } else { 1.0 };
    let axe = q(k.axe, &|m| 0.5 * cut_q(m) + 0.5 * durability(m));
    let cord = if k.cord == NONE { 0.0 } else { 1.0 };
    let hammer = if k.hammer == NONE { 0.0 } else { 1.0 };
    let point = if k.point == NONE { 0.0 } else { 1.0 };
    let fire = if k.fire == NONE { 0.0 } else { 1.0 };
    let kiln = if k.kiln == NONE { 0.0 } else { 1.0 };
    let ceramic = if k.vessel != NONE && mat(k.vessel) == Mat::Ceramic { 1.0 } else { 0.0 };
    let netq = q(k.net, &mesh_q);
    let trapq = q(k.trap, &mesh_q);
    k.boat_q = q(k.boat, &boat_q);
    // an outrigger canoe or a plank boat is a boat too, and a plank hull is the soundest
    k.boat_q = k.boat_q.max(q(k.outrigger, &boat_q));
    if k.plank != NONE {
        k.boat_q = k.boat_q.max((boat_q(mat(k.plank)) + 0.25).min(1.0));
    }
    // a ship is a plank hull too: no sounder, only bigger and better found
    if k.ship != NONE {
        k.boat_q = k.boat_q.max((boat_q(mat(k.ship)) + 0.25).min(1.0));
    }
    k.plants_m = 1.0 + 0.35 * dig + 0.3 * vq + 0.1 * hammer + 0.1 * axe;
    k.game_m = (1.0 + 1.4 * hunt_q + 0.15 * cord) * (1.0 + 0.35 * cut + 0.05 * hammer);
    k.fish_m = 1.0 + 0.8 * hunt_q + 0.4 * vq + 0.3 * point + 0.2 * cord + 0.9 * netq;
    // small game: hard to catch by hand, easy with snares and nets
    k.small_m = 0.35 + 0.4 * hunt_q + 1.6 * trapq + 0.4 * netq;
    // a bow reaches fast game and birds a spear cannot
    if has_bow(k) {
        k.game_m *= 1.2;
        k.small_m *= 1.15;
    }
    let ck = |j: usize, g: f32| if k.cooked[j] != NONE { g } else { 1.0 };
    // grain: boiled into porridge (pots help) or ground on a quern and baked
    let grain = if k.cooked_grain != NONE { 1.3 + 0.15 * ceramic + 0.1 * hammer } else { 1.0 + 0.1 * hammer };
    k.digest = [ck(0, 1.25 + 0.15 * ceramic), ck(1, 1.2), ck(2, 1.15), ck(1, 1.2), grain];
    // field work: digging stick to plant, a sharp edge as a sickle, an axe to clear ground
    k.farm_m = 0.5 + 0.3 * dig + 0.35 * cut + 0.2 * axe;
    k.smoke = fire > 0.0 && (k.cooked[1] != NONE || k.cooked[2] != NONE);
    let _ = kiln;
    let gq = q(k.garment, &garment_q);
    k.protect = (0.35 * fire + 0.5 * gq + 0.05 * kiln + 0.05 * axe * fire).min(0.92);
}

/// Recipe quality on top of `apply_base`. A quality of 0 multiplies by `1.0`, which is
/// bit-identical for a finite yield, so a plain kit and a refined one agree when nothing
/// has been refined. `make_kit` does not call this; callers still `refine_kit` once.
fn apply_quality<Q: Fn(Item) -> f32>(k: &mut Kit, q: &Q) {
    let boat = q(k.boat).max(q(k.outrigger)).max(q(k.plank)).max(q(k.ship));
    k.ship_q = if k.ship != NONE { q(k.ship) } else { 0.0 };
    if k.boat_q > 0.0 {
        k.boat_q = (k.boat_q * (1.0 + QUALITY_BOAT * boat)).min(1.0);
    }
    let y = |x: f32| 1.0 + QUALITY_YIELD * x;
    k.game_m *= y(q(k.hunt));
    k.fish_m *= y(q(k.net).max(q(k.hunt)));
    k.small_m *= y(q(k.trap).max(q(k.net)));
    k.plants_m *= y(0.5 * (q(k.vessel) + q(k.dig)));
    k.farm_m *= y(0.5 * (q(k.dig) + q(k.cut)));
    if k.garment != NONE {
        k.protect = (k.protect + QUALITY_PROTECT * q(k.garment)).min(0.92);
    }
    k.fine = q(k.ornament);
}

/// Apply recipe quality to a kit from `make_kit` / `make_kit_for_view`. Once only: a second
/// call would scale the yields again.
pub fn refine_kit(k: &mut Kit, tk: &Toolkit, known: &[Known]) {
    apply_quality(k, &|it| recipe_info(tk, known, it).1);
}

/// `slot` is the chooser's best[] index (hunt 0, cut 1, point 10, axe 4, net 12, trap 13,
/// boat 11, bow 14, shield 15, outrigger 16, plank 18, ship 22), not `Kit::slot_item`'s `j`
/// except where the two numberings meet (hunt, cut). `kit` is the kit with the other slots
/// already chosen; this slot is overwritten. `mask` is `tk.via(it).1` (0 if unknown).
pub fn tool_effects<F: Fn(Item) -> (f32, f32, u8)>(view: &BandView, kit: &Kit, slot: usize, it: Item, info: &F, mask: u32) -> Effects {
    let (food, labour) = if it == NONE {
        (0.0, 0.0)
    } else if is_food_slot(slot) {
        // Same multipliers as `kit_yields` of the empty slot and of the slot filled, but only
        // the five food numbers. A bow or a hull still goes through the full kit below.
        let c = cand_new(it, 0.0, info(it).1, info(it).2);
        let w = weights(view, &digest_of(kit));
        let base = yargs_empty(kit, slot, info);
        let m0 = yields(&base);
        let m1 = yields_with(slot, &base, &c, kit.bow != NONE);
        let food = food_acc(&w, &m0, &m1);
        let mut with = *kit;
        set_slot(&mut with, slot, it);
        let labour = tool_upkeep(view.upkeep_rate, it, c.d, sealed_factor(it, &with, pitch_in_use(&with, view.wet)));
        (food, labour)
    } else {
        let mut empty = *kit;
        set_slot(&mut empty, slot, NONE);
        let ye = kit_yields(&empty, info);
        let mut with = empty;
        set_slot(&mut with, slot, it);
        let yw = kit_yields(&with, info);
        let food = food_delta(view, &yw, &ye);
        let labour = tool_upkeep(view.upkeep_rate, it, info(it).2, sealed_factor(it, &with, pitch_in_use(&with, view.wet)));
        (food, labour)
    };
    let mut with = *kit;
    set_slot(&mut with, slot, it);
    Effects {
        food,
        food_sd: 0.0,
        labour,
        mats: marginal_mats(if it == NONE { 0 } else { mask }, view.at_hand),
        p_fail: 0.0,
        fight: fight_of(slot, it, &with),
    }
}

/// The decision's discounted and undiscounted scores for `it` in `slot`, given the other
/// slots in `kit`. Food slots use the same food and labour as `tool_effects` (food minus the
/// food that labour would have grown). The other payoff slots keep `KIT_UPKEEP`, not the real
/// upkeep: a hull is still ranked by soundness, and the probe's `true_value` still subtracts
/// `KIT_UPKEEP`.
pub fn slot_score<F: Fn(Item) -> (f32, f32, u8)>(view: &BandView, kit: &Kit, slot: usize, it: Item, info: &F) -> (f32, f32) {
    let (sk, ql, d) = info(it);
    if is_food_slot(slot) {
        let c = cand_new(it, sk, ql, d);
        let w = weights(view, &digest_of(kit));
        let base = yargs_empty(kit, slot, info);
        let m0 = yields(&base);
        let m1 = yields_with(slot, &base, &c, kit.bow != NONE);
        let food = food_acc(&w, &m0, &m1);
        let pitch = pitch_in_use(kit, view.wet);
        let labour = tool_upkeep(view.upkeep_rate, it, d, food_sealed(slot, pitch));
        priced(food, labour, sk, view)
    } else {
        nonfood_scores(slot, it, sk, ql, d)
    }
}

fn nonfood_qv(slot: usize, m: Mat) -> f32 {
    match slot {
        11 | 16 | 18 | 22 => boat_q(m),
        14 | 15 => m.props().tough + 1.0,
        _ => 0.0,
    }
}

/// Old ranking for a boat, bow, shield, outrigger, plank or ship: material term times recipe
/// quality, minus `KIT_UPKEEP`. Not the real upkeep (a hull is not ranked by caulking).
fn nonfood_scores(slot: usize, it: Item, sk: f32, ql: f32, d: u8) -> (f32, f32) {
    let m = mat(it);
    nonfood_from(nonfood_qv(slot, m), sk, ql, 1.15 - durability(m), 1.0 + 0.3 * d as f32)
}

fn nonfood_from(term: f32, sk: f32, ql: f32, dur_term: f32, depth_f: f32) -> (f32, f32) {
    let up = KIT_UPKEEP * dur_term * depth_f;
    let gain = term * (1.0 + QUALITY_YIELD * ql);
    (gain * (0.5 + 0.5 * sk) - up, gain - up)
}

/// Whether `it` can fill payoff slot `j` (`Kit::slot_item` order).
pub fn fills_payoff(it: Item, j: usize) -> bool {
    if it == NONE {
        return false;
    }
    matches!(
        (form(it), j),
        (Form::Spear, 0)
            | (Form::Point, 0)
            | (Form::Point, 2)
            | (Form::Edge, 0)
            | (Form::Edge, 1)
            | (Form::Axe, 3)
            | (Form::Boat, 4)
            | (Form::Outrigger, 9)
            | (Form::Plank, 10)
            | (Form::Ship, 11)
            | (Form::Net, 5)
            | (Form::Trap, 6)
            | (Form::Bow, 7)
            | (Form::Shield, 8)
    )
}

fn consider(cur: f32, full: f32, it: Item, best_cur: &mut f32, best_it: &mut Item, pot: &mut [(f32, Item); 2]) {
    // strict: a tie keeps the earlier item, and candidates arrive in item-id order
    if cur > *best_cur {
        *best_cur = cur;
        *best_it = it;
    }
    if full > pot[0].0 {
        pot[1] = pot[0];
        pot[0] = (full, it);
    } else if full > pot[1].0 {
        pot[1] = (full, it);
    }
}

fn write_choice(k: &mut Kit, slot: usize, best_it: Item, best_cur: f32, pot: [(f32, Item); 2], record_trial: bool) {
    // a negative score still fills the slot when anything can
    set_slot(k, slot, best_it);
    if record_trial {
        let ch = if pot[0].1 != best_it { pot[0] } else { pot[1] };
        k.trial[trial_j(slot)] = if best_it != NONE && ch.1 != NONE && ch.0 > best_cur { ch.1 } else { NONE };
    }
}

fn choose_nonfood(k: &mut Kit, slot: usize, list: &[Cand], record_trial: bool) {
    let mut best_cur = f32::NEG_INFINITY;
    let mut best_it = NONE;
    let mut pot = [(f32::NEG_INFINITY, NONE); 2];
    for c in list {
        let (cur, full) = nonfood_from(c.term, c.sk, c.ql, c.dur_term, c.depth_f);
        consider(cur, full, c.it, &mut best_cur, &mut best_it, &mut pot);
    }
    write_choice(k, slot, best_it, best_cur, pot, record_trial);
}

/// Food-slot choice. `lists` are the candidate forms in item-id order (an edge before a point
/// before a spear). Empty-slot multipliers are computed once. A point's food does not depend
/// on which point it is, so that food is computed once too.
fn choose_food<F: Fn(Item) -> (f32, f32, u8)>(k: &mut Kit, slot: usize, lists: &[&[Cand]], info: &F, view: &BandView, w: &[f32; 5], record_trial: bool) {
    let mut best_cur = f32::NEG_INFINITY;
    let mut best_it = NONE;
    let mut pot = [(f32::NEG_INFINITY, NONE); 2];
    let n: usize = lists.iter().map(|l| l.len()).sum();
    if n == 0 {
        write_choice(k, slot, NONE, best_cur, pot, record_trial);
        return;
    }
    let base = yargs_empty(k, slot, info);
    let m0 = yields(&base);
    let sealed = food_sealed(slot, pitch_in_use(k, view.wet));
    // one food for every point: presence changes the multipliers, the material does not
    let point_food = if slot == 10 {
        let mut a = base;
        a.point = 1.0;
        a.bow = k.bow != NONE;
        Some(food_acc(w, &m0, &yields(&a)))
    } else {
        None
    };
    let bow_item = k.bow != NONE;
    for list in lists {
        for c in *list {
            let food = if let Some(food) = point_food {
                food
            } else {
                food_acc(w, &m0, &yields_with(slot, &base, c, bow_item))
            };
            let labour = tool_upkeep(view.upkeep_rate, c.it, c.d, sealed);
            let (cur, full) = priced(food, labour, c.sk, view);
            consider(cur, full, c.it, &mut best_cur, &mut best_it, &mut pot);
        }
    }
    write_choice(k, slot, best_it, best_cur, pot, record_trial);
}

struct Bucket {
    n: u8,
    v: [Cand; NMAT],
}

impl Bucket {
    fn new() -> Self {
        Self {
            n: 0,
            v: [Cand { it: NONE, d: 0, sk: 0.0, ql: 0.0, hunt_q: 0.0, term: 0.0, dur_term: 0.0, depth_f: 0.0 }; NMAT],
        }
    }
    fn push(&mut self, c: Cand) {
        if (self.n as usize) < NMAT {
            self.v[self.n as usize] = c;
            self.n += 1;
        }
    }
    fn as_slice(&self) -> &[Cand] {
        &self.v[..self.n as usize]
    }
}

/// Choose the kit. Non-payoff slots (dig, pot, cord, ...) are settled first, exactly as they
/// were. Payoff slots then take two passes: the first starts empty and fills each with the
/// others as chosen so far; the second re-chooses a food slot only when another slot it reads
/// has changed (a hull, bow or shield does not read the kit, so it is scored once). Trials are
/// marked on the last scoring of each slot. Quality is not baked in (`refine_kit` does that).
pub fn make_kit_with_view<F: Fn(Item) -> (f32, f32, u8)>(avail: &ItemSet, info: &F, view: &BandView) -> Kit {
    let mut k = Kit::default();
    let mut best = [0f32; 24];
    // one bucket per payoff form, filled in item-id order. A form has at most one item per material.
    let mut buckets = [
        Bucket::new(), Bucket::new(), Bucket::new(), Bucket::new(), Bucket::new(), Bucket::new(),
        Bucket::new(), Bucket::new(), Bucket::new(), Bucket::new(), Bucket::new(), Bucket::new(),
    ];
    for it in avail.iter() {
        let (f, m) = (form(it), mat(it));
        if let Some(bi) = bucket_of(f) {
            let (sk, ql, d) = info(it);
            buckets[bi].push(cand_new(it, sk, ql, d));
            continue;
        }
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
            // a cloth sail is kept before a mat one: it serves any hull
            Form::Sail => upd(17, m.props().tough + if m == Mat::Fibre { 0.0 } else { 10.0 }, &mut k.sail),
            Form::Frame => upd(19, vessel_q(m) + 0.1, &mut k.frame),
            // a prized material (gold) is worn before any hard one
            Form::Ornament => upd(20, m.props().hard + 1.0 + 10.0 * m.props().prized, &mut k.ornament),
            // a jar of grain left to work is ale, not a store jar
            Form::Jar if m == Mat::Grain => k.ale = it,
            Form::Jar => upd(21, vessel_q(m), &mut k.jar),
            // the mark that lasts: fired token, bone, stick, raw clay
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
    // digest does not depend on payoff slots; the weights are the same for every candidate
    let w = weights(view, &digest_of(&k));
    let edge = buckets[0].as_slice();
    let point = buckets[1].as_slice();
    let spear = buckets[2].as_slice();
    let axe = buckets[3].as_slice();
    let net = buckets[4].as_slice();
    let trap = buckets[5].as_slice();
    // taken before each food slot is filled, so pass 2 can see whether its inputs moved.
    // The slice array is owned: a `match` arm's `&[...]` temporary would die at the end of
    // the `let` and the choice would read freed stack.
    let mut snap = [Kit::default(); 6];
    let mut snap_pitch = [false; 6];
    let food_lists = |slot: usize| -> [&[Cand]; 3] {
        match slot {
            // edge, then point, then spear: item id rises with the form
            0 => [edge, point, spear],
            1 => [edge, &[], &[]],
            10 => [point, &[], &[]],
            4 => [axe, &[], &[]],
            12 => [net, &[], &[]],
            _ => [trap, &[], &[]],
        }
    };
    for &slot in &CHOOSE_ORDER {
        if is_food_slot(slot) {
            let i = food_ix(slot);
            snap[i] = k;
            snap_pitch[i] = pitch_in_use(&k, view.wet);
            let lists = food_lists(slot);
            choose_food(&mut k, slot, &lists, info, view, &w, true);
        } else {
            let list = match slot {
                11 => buckets[6].as_slice(),
                14 => buckets[7].as_slice(),
                15 => buckets[8].as_slice(),
                16 => buckets[9].as_slice(),
                18 => buckets[10].as_slice(),
                _ => buckets[11].as_slice(),
            };
            // a hull, bow or shield does not read the rest of the kit, so this scoring is final
            choose_nonfood(&mut k, slot, list, true);
        }
    }
    let pitch_now = pitch_in_use(&k, view.wet);
    for &slot in &CHOOSE_ORDER {
        if !is_food_slot(slot) {
            continue;
        }
        let lists = food_lists(slot);
        let n: usize = lists.iter().map(|l| l.len()).sum();
        // one candidate (or none) cannot be overtaken; an unchanged context scores the same
        if n <= 1 {
            continue;
        }
        let i = food_ix(slot);
        if !food_deps_changed(slot, &snap[i], &k, snap_pitch[i], pitch_now) {
            continue;
        }
        choose_food(&mut k, slot, &lists, info, view, &w, true);
    }
    apply_base(&mut k);
    k
}

/// `make_kit_with_view` on the nominal view (no band).
pub fn make_kit_with<F: Fn(Item) -> (f32, f32, u8)>(avail: &ItemSet, info: &F) -> Kit {
    make_kit_with_view(avail, info, &BandView::nominal())
}

/// Best item per task from what can be produced, as if every recipe were fully practised.
pub fn make_kit(avail: &ItemSet) -> Kit {
    make_kit_with(avail, &|_| (1.0, 0.0, 0))
}

/// The kit a band relies on, ranked for `view` (last season's harvest and food, its pop,
/// whether it is by water).
pub fn make_kit_for_view(view: &BandView, tk: &Toolkit, known: &[Known]) -> Kit {
    make_kit_among_view(view, &tk.avail, tk, known)
}

/// `make_kit_for_view` choosing only among `avail` (a subset of `tk.avail`; for probes).
pub fn make_kit_among_view(view: &BandView, avail: &ItemSet, tk: &Toolkit, known: &[Known]) -> Kit {
    make_kit_with_view(avail, &|it| recipe_info(tk, known, it), view)
}

/// Nominal-view `make_kit_for_view`. Tests and the probe's displaced check use this.
pub fn make_kit_for(tk: &Toolkit, known: &[Known]) -> Kit {
    make_kit_for_view(&BandView::nominal(), tk, known)
}

/// Nominal-view `make_kit_among_view`.
pub fn make_kit_among(avail: &ItemSet, tk: &Toolkit, known: &[Known]) -> Kit {
    make_kit_among_view(&BandView::nominal(), avail, tk, known)
}
