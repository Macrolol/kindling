//! (skills step 1) What a tool does for a band, and the kit choice that reads it.
//!
//! The economy's yields and the decision used to disagree: blades were ranked by `edge_q`
//! (sharp and lasting) while the food they bring depends only on `cut_q`, and toughness was
//! counted again as upkeep. `tool_effects` is the one account. For a given kit the multipliers
//! are exactly today's (`apply_base` then `apply_quality`); only which item fills a slot changes.
//!
//! Food slots are ranked by food gained minus the food the tool's upkeep labour would have
//! grown. Recipe skill no longer discounts that decision (skills step 2): the user factor
//! scales the material term, and recipe skill still sets quality. `edge_q` is not used for
//! food. Non-food payoff slots keep the old `KIT_UPKEEP` ranking, with the user factor in
//! place of the recipe-skill discount.
use crate::band::{edge_q, Kit, KIT_UPKEEP, QUALITY_BOAT, QUALITY_PROTECT, QUALITY_YIELD};
use crate::knowledge::{Known, Toolkit};
use crate::materials::*;
use crate::params::Params;
use crate::rng::hash3;
use crate::world::{PITCH_CAULK, PITCH_GLUE};
use std::sync::OnceLock;

/// Share of the people who work. `World::economy` sets `labour = LABOUR_SHARE * pop`.
pub const LABOUR_SHARE: f32 = 0.55;
/// Of that labour, the share that can go to feeding (`max_e` uses `labour * WORK_SHARE`).
pub const WORK_SHARE: f32 = 0.85;

/// Below this, last season's harvest counts as "no history" and the nominal mix is used.
pub const HARVEST_EPS: f32 = 1e-3;
/// Per-person mix when a band has no harvest yet: plants, game, fish, small game, crops.
/// Sums to 1, so `NOMINAL_MIX * pop` is `pop` units of food before digest.
pub const NOMINAL_MIX: [f32; 5] = [0.3, 0.3, 0.25, 0.15, 0.0];
/// One season's harvest swings with the season, so the food term shrinks it halfway
/// toward the long-run nominal mix at the same total. This damps seasonal kit
/// flip-flopping without new saved state.
pub const HARVEST_PRIOR: f32 = 0.5;
/// (skills step 2) Starting `Band::learn_curve`. Founders, daughter bands and the
/// pre-v16 fill-in all use this prior (the skills proposal's 1.3). Later steps
/// move it per band.
pub const LC_PRIOR: f32 = 1.3;
/// (skills step 2) User factor of a form the band has never handled.
pub const USE_NOVICE: f32 = 0.5;
/// (skills step 2) Founders start here on the forms of their starting kit (pass 3).
pub const USE_FOUNDER: f32 = 0.7;
/// (skills step 2) One season of use closes this fraction of the gap to 1 (pass 3).
pub const USE_PRACTICE: f32 = 0.08;
/// (skills step 2) An unused form fades by `USE_FADE_X * floss` a season (pass 3).
pub const USE_FADE_X: f32 = 0.5;
/// (skills step 2) Cosine of two motion profiles is scaled by this. `T`'s diagonal is 1.
pub const USE_TRANSFER: f32 = 0.6;
/// (skills step 2) A daughter inherits this share of the parent's `use_skill` (pass 3).
pub const USE_INHERIT: f32 = 0.9;
/// (skills step 2) Chance a close tie teaches one form in a year (pass 3).
pub const USE_TEACH_P: f32 = 0.2;
/// (skills step 2) Fraction of the gap to the teacher that one lesson closes (pass 3).
pub const USE_TEACH_SHARE: f32 = 0.15;
/// (skills step 2) Half-width of a novice's season-to-season yield factor at `u_eff` 0.
pub const NOVICE_SD: f32 = 0.3;
const NOMINAL_POP: f32 = 25.0;
const FOOD_EPS: f32 = 1e-6;
/// All-ones effective skill: the nominal view, and a trial's potential.
const USER_FULL: [f32; NFORM] = [1.0; NFORM];
/// Hash salt for the novice yield draw. Not the kit-trial salt.
const NOVICE_SALT: u64 = 0x4E6F_7669_6365_5A01;
/// Motion axes, proposal order: cut, chop, thrust, throw, aim, fend, knot, dig, paddle, sail, pound.
const MOTION_N: usize = 11;

struct MotionRow {
    form: u8,
    w: [f32; MOTION_N],
}

/// One row per form that trains a motion. A form with no row has no profile.
const MOTION_ROWS: &[MotionRow] = &[
    MotionRow { form: Form::Edge as u8, w: [1.0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Axe as u8, w: [0.3, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.3] },
    MotionRow { form: Form::Spear as u8, w: [0.2, 0.0, 1.0, 0.5, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Point as u8, w: [0.0, 0.0, 0.3, 0.4, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Bow as u8, w: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Shield as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Net as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Trap as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Cord as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.6, 0.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Shaft as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0] },
    MotionRow { form: Form::Boat as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0] },
    MotionRow { form: Form::Outrigger as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.3, 0.0] },
    MotionRow { form: Form::Plank as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.7, 0.0, 0.0] },
    MotionRow { form: Form::Ship as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.6, 0.6, 0.0] },
    MotionRow { form: Form::Sail as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0] },
    MotionRow { form: Form::Hammer as u8, w: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0] },
];

/// `T(f, g)`. Diagonal is 1. Off-diagonal is `USE_TRANSFER * cos`, or 0 if either form has no profile.
pub(crate) fn transfer_table() -> &'static [[f32; NFORM]; NFORM] {
    static T: OnceLock<[[f32; NFORM]; NFORM]> = OnceLock::new();
    T.get_or_init(|| {
        let mut prof = [None; NFORM];
        let mut seen = [false; NFORM];
        for row in MOTION_ROWS {
            let i = row.form as usize;
            assert!(i < NFORM && !seen[i], "duplicate motion profile");
            seen[i] = true;
            prof[i] = Some(row.w);
        }
        let mut nrm = [0.0f32; NFORM];
        for f in 0..NFORM {
            if let Some(p) = prof[f] {
                let mut s = 0.0;
                for w in p {
                    s += w * w;
                }
                nrm[f] = s.sqrt();
            }
        }
        let mut t = [[0.0f32; NFORM]; NFORM];
        for f in 0..NFORM {
            t[f][f] = 1.0;
            for g in (f + 1)..NFORM {
                let v = match (prof[f], prof[g]) {
                    (Some(pf), Some(pg)) if pf == pg => USE_TRANSFER,
                    (Some(pf), Some(pg)) => {
                        let mut dot = 0.0;
                        for i in 0..MOTION_N {
                            dot += pf[i] * pg[i];
                        }
                        let mut c = dot / (nrm[f] * nrm[g]);
                        if c > 1.0 {
                            c = 1.0;
                        }
                        if c < 0.0 {
                            c = 0.0;
                        }
                        USE_TRANSFER * c
                    }
                    _ => 0.0,
                };
                t[f][g] = v;
                t[g][f] = v;
            }
        }
        t
    })
}

/// Effective skill with each form. An empty slice (a band not yet filled) is all 1.0.
/// Any other length must be `NFORM`.
pub(crate) fn u_eff(use_skill: &[f32]) -> [f32; NFORM] {
    if use_skill.is_empty() {
        return USER_FULL;
    }
    assert!(use_skill.len() == NFORM, "use_skill must be empty or NFORM, got {}", use_skill.len());
    // Sparse walk over the non-zero off-diagonal entries of `T` (same maxima as the full
    // NFORM x NFORM loop: the diagonal adds `use_skill[g]` itself and zero entries add 0).
    let mut out = [0.0f32; NFORM];
    out.copy_from_slice(use_skill);
    for &(f, g, t) in transfer_pairs() {
        let v = t * use_skill[f as usize];
        if v > out[g as usize] {
            out[g as usize] = v;
        }
    }
    out
}

/// Non-zero off-diagonal entries `(f, g, T(f, g))` of `transfer_table`, in (f, g) order.
fn transfer_pairs() -> &'static [(u8, u8, f32)] {
    static P: OnceLock<Vec<(u8, u8, f32)>> = OnceLock::new();
    P.get_or_init(|| {
        let t = transfer_table();
        let mut v = Vec::new();
        for f in 0..NFORM {
            for g in 0..NFORM {
                if f != g && t[f][g] != 0.0 {
                    v.push((f as u8, g as u8, t[f][g]));
                }
            }
        }
        v
    })
}

/// `USE_NOVICE + (1 - USE_NOVICE) * u`, written so `u == 1` is 1.0 bit-exactly.
#[inline]
pub(crate) fn user_factor(u: f32) -> f32 {
    1.0 - (1.0 - USE_NOVICE) * (1.0 - u)
}

fn quiet(_: usize) -> f32 {
    1.0
}

/// Deterministic draw in [-1, 1), the same mapping the map noise uses.
pub(crate) fn novice_z(seed: u64, band: u32, tick: u32, slot: usize) -> f32 {
    let h = hash3(seed ^ NOVICE_SALT, band as i64, (tick as i64) * 32 + slot as i64);
    (h >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
}

/// `1 + NOVICE_SD*(1-u)*z` for the tool in `slot`. Empty slot, or `u == 1`, is exactly 1.
pub(crate) fn novice_factor(k: &Kit, user: &[f32; NFORM], seed: u64, band: u32, tick: u32, slot: usize) -> f32 {
    let it = match slot {
        0 => k.hunt,
        1 => k.cut,
        2 => k.dig,
        4 => k.axe,
        10 => k.point,
        12 => k.net,
        13 => k.trap,
        _ => NONE,
    };
    if it == NONE {
        return 1.0;
    }
    let u = user[form(it) as usize];
    let z = novice_z(seed, band, tick, slot);
    1.0 + NOVICE_SD * (1.0 - u) * z
}

/// Five food multipliers (plants, game, fish, small, farm), quality included.
/// `noise(slot)` is 1 on the kit's stored multipliers and the novice factor in the economy.
pub(crate) fn food_mults<Q, N>(k: &Kit, user: &[f32; NFORM], qual: Q, noise: N) -> [f32; 5]
where
    Q: Fn(Item) -> f32,
    N: Fn(usize) -> f32,
{
    yields(&yargs_of(k, usize::MAX, &|it| (0.0, qual(it), 0), user, &noise))
}

/// best[] index of each payoff slot, in `Kit::slot_item` order.
#[cfg(test)]
pub(crate) const PAYOFF_SLOTS: [usize; 12] = [0, 1, 10, 4, 11, 12, 13, 14, 15, 16, 18, 22];
/// Food slots, then the rest. Pass 1 fills each from an empty payoff kit; pass 2 re-chooses
/// each with the others already set. A cut is optimal on the final kit (nothing chosen after
/// it changes its food). Hunt is not: cut is filled again after hunt.
const CHOOSE_ORDER: [usize; 12] = [0, 1, 10, 4, 12, 13, 11, 14, 15, 16, 18, 22];

/// What putting `it` in a slot does, beyond the kit with that slot empty.
/// `food_sd` is the novice spread on a food-slot tool (`p_fail` stays 0 until a later step).
/// `fight` is informational: the decision does not read it. `labour` is the economy's fraction
/// of the labour pool (the economy multiplies by `LABOUR_SHARE * pop`). `mats` is the marginal
/// stock draw, up to four materials, index order, unused entries `(0, 0)`.
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
    /// Effective user skill per form (`u_eff`). Nominal callers are all 1.0.
    pub user: [f32; NFORM],
}

impl BandView {
    /// Callers with no band (`make_kit` in tests). Food equals pop, so the price is finite.
    /// User skill is all 1.0, so yields match the pre-user-factor formulas.
    pub fn nominal() -> Self {
        let pop = NOMINAL_POP;
        BandView {
            harvest: NOMINAL_MIX.map(|x| x * pop),
            food: pop,
            pop,
            upkeep_rate: Params::default().upkeep,
            wet: false,
            at_hand: 0,
            user: USER_FULL,
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

/// Harvest the food term is linearised around. With history, one season is shrunk halfway
/// toward `NOMINAL_MIX` at the same total (`HARVEST_PRIOR`). A season with no digested
/// harvest (a band just loaded from before harvest was saved, or not yet run) uses the
/// nominal mix scaled by *this* band's pop, so food and labour move together.
pub(crate) fn harvest_for(view: &BandView, dg: &[f32; 5]) -> [f32; 5] {
    let sum: f32 = (0..5).map(|r| view.harvest[r] * dg[r]).sum();
    if sum > HARVEST_EPS {
        // plain sum, index order — not the digested sum above
        let t: f32 = (0..5).map(|r| view.harvest[r]).sum();
        let mut h = [0.0; 5];
        for r in 0..5 {
            h[r] = (1.0 - HARVEST_PRIOR) * view.harvest[r] + HARVEST_PRIOR * NOMINAL_MIX[r] * t;
        }
        h
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
    /// User factor of the bow. Unused when `bow` is false. Exactly 1.0 keeps the old bonus.
    bow_uf: f32,
    q_hunt: f32,
    q_cut: f32,
    q_net: f32,
    q_trap: f32,
    q_vessel: f32,
    q_dig: f32,
}

/// Plants, game, fish, small, farm, before recipe quality. Same operations, in the same
/// order, as the food lines of `apply_base` (which calls this).
#[inline(always)]
fn yields_base(a: &YArgs) -> [f32; 5] {
    let plants = 1.0 + 0.35 * a.dig + 0.3 * a.vq + 0.1 * a.hammer + 0.1 * a.axe;
    let mut game = (1.0 + 1.4 * a.hunt_q + 0.15 * a.cord) * (1.0 + 0.35 * a.cut + 0.05 * a.hammer);
    let fish = 1.0 + 0.8 * a.hunt_q + 0.4 * a.vq + 0.3 * a.point + 0.2 * a.cord + 0.9 * a.netq;
    let mut small = 0.35 + 0.4 * a.hunt_q + 1.6 * a.trapq + 0.4 * a.netq;
    if a.bow {
        // `bow_uf == 1` keeps the old literals. `1 + 0.2*1` is not the f32 closest to 1.2.
        let (bg, bs) = if a.bow_uf == 1.0 { (1.2, 1.15) } else { (1.0 + 0.2 * a.bow_uf, 1.0 + 0.15 * a.bow_uf) };
        game *= bg;
        small *= bs;
    }
    let farm = 0.5 + 0.3 * a.dig + 0.35 * a.cut + 0.2 * a.axe;
    [plants, game, fish, small, farm]
}

/// `yields_base` then the food lines of `apply_quality`, same operations.
#[inline(always)]
fn yields(a: &YArgs) -> [f32; 5] {
    let mut m = yields_base(a);
    let y = |x: f32| 1.0 + QUALITY_YIELD * x;
    m[1] *= y(a.q_hunt);
    m[2] *= y(a.q_net.max(a.q_hunt));
    m[3] *= y(a.q_trap.max(a.q_net));
    m[0] *= y(0.5 * (a.q_vessel + a.q_dig));
    m[4] *= y(0.5 * (a.q_dig + a.q_cut));
    m
}

/// User factor times the novice factor for `slot`. `noise` is 1 on the decision path.
fn scale<N: Fn(usize) -> f32>(user: &[f32; NFORM], it: Item, slot: usize, noise: &N) -> f32 {
    if it == NONE {
        return 1.0;
    }
    user_factor(user[form(it) as usize]) * noise(slot)
}

/// The kit with payoff `slot` treated as empty (`usize::MAX` clears nothing). Hunt, cut,
/// point, axe, net, trap and dig terms are multiplied by the user factor (and `noise`).
fn yargs_of<F, N>(k: &Kit, slot: usize, info: &F, user: &[f32; NFORM], noise: &N) -> YArgs
where
    F: Fn(Item) -> (f32, f32, u8),
    N: Fn(usize) -> f32,
{
    let q = [
        qual(info, if slot == 0 { NONE } else { k.hunt }),
        qual(info, if slot == 1 { NONE } else { k.cut }),
        qual(info, if slot == 12 { NONE } else { k.net }),
        qual(info, if slot == 13 { NONE } else { k.trap }),
        qual(info, k.vessel),
        qual(info, k.dig),
    ];
    yargs_rescaled(k, slot, user, noise, q)
}

/// The recipe qualities `yargs_of` read into `a`, in `yargs_rescaled` order.
fn quals_of(a: &YArgs) -> [f32; 6] {
    [a.q_hunt, a.q_cut, a.q_net, a.q_trap, a.q_vessel, a.q_dig]
}

/// `yargs_of` with the recipe qualities `q` already read (hunt, cut, net, trap, vessel, dig;
/// they do not depend on the user), so a second user can reuse them.
fn yargs_rescaled<N>(k: &Kit, slot: usize, user: &[f32; NFORM], noise: &N, q: [f32; 6]) -> YArgs
where
    N: Fn(usize) -> f32,
{
    let hunt = if slot == 0 { NONE } else { k.hunt };
    let cut = if slot == 1 { NONE } else { k.cut };
    let point = if slot == 10 { NONE } else { k.point };
    let axe = if slot == 4 { NONE } else { k.axe };
    let net = if slot == 12 { NONE } else { k.net };
    let trap = if slot == 13 { NONE } else { k.trap };
    YArgs {
        hunt_q: hunt_q_of(hunt) * scale(user, hunt, 0, noise),
        cut: cut_term(cut) * scale(user, cut, 1, noise),
        vq: if k.vessel == NONE { 0.0 } else { vessel_q(mat(k.vessel)) },
        dig: if k.dig == NONE { 0.0 } else { scale(user, k.dig, 2, noise) },
        axe: axe_term(axe) * scale(user, axe, 4, noise),
        cord: if k.cord == NONE { 0.0 } else { 1.0 },
        hammer: if k.hammer == NONE { 0.0 } else { 1.0 },
        point: if point == NONE { 0.0 } else { scale(user, point, 10, noise) },
        netq: mesh_term(net) * scale(user, net, 12, noise),
        trapq: mesh_term(trap) * scale(user, trap, 13, noise),
        bow: k.bow != NONE && point != NONE,
        bow_uf: if k.bow == NONE { 1.0 } else { user_factor(user[Form::Bow as usize]) },
        q_hunt: q[0],
        q_cut: q[1],
        q_net: q[2],
        q_trap: q[3],
        q_vessel: q[4],
        q_dig: q[5],
    }
}

/// One producible item, with the unscaled material terms the choice reuses across passes.
/// `sk` is `u_eff` of the form, not recipe skill. `hunt_q` / `term` match `apply_base` before the user factor.
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
fn yields_with<N: Fn(usize) -> f32>(slot: usize, base: &YArgs, c: &Cand, bow_item: bool, user: &[f32; NFORM], noise: &N) -> [f32; 5] {
    let mut a = *base;
    let s = scale(user, c.it, slot, noise);
    match slot {
        0 => {
            a.hunt_q = c.hunt_q * s;
            a.q_hunt = c.ql;
        }
        1 => {
            a.cut = c.term * s;
            a.q_cut = c.ql;
        }
        // every point fills the slot the same way: the multiplier is presence, not material
        10 => {
            a.point = s;
            a.bow = bow_item;
        }
        4 => a.axe = c.term * s,
        12 => {
            a.netq = c.term * s;
            a.q_net = c.ql;
        }
        13 => {
            a.trapq = c.term * s;
            a.q_trap = c.ql;
        }
        _ => {}
    }
    yields(&a)
}

/// Glue on a haft (hunt, axe) when pitch is in use. A knife, point, net or trap is glued only
/// when it is the same item as the kit's hunt or axe (a flint blade that is also the hunting
/// tool), exactly as the economy's `sealed_factor` charges it. Same result as `sealed_factor`
/// for `it` in `slot` of `k` (a food-slot form cannot also be a hull).
fn food_sealed(slot: usize, it: Item, k: &Kit, pitch: bool) -> f32 {
    if pitch && (slot == 0 || slot == 4 || it == k.hunt || it == k.axe) { PITCH_GLUE } else { 1.0 }
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
/// Every food slot also depends on hunt, axe and pitch: a candidate that is the same item as
/// the hunt or axe is glued (`food_sealed`).
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
        1 => then.hunt != now.hunt || then.point != now.point || then.bow != now.bow || then.axe != now.axe || pitch_then != pitch_now,
        // fish: hunt, net; a bow that gains its point also moves game (cut) and small (trap)
        10 => {
            then.hunt != now.hunt
                || then.net != now.net
                || then.bow != now.bow
                || then.cut != now.cut
                || then.trap != now.trap
                || then.axe != now.axe
                || pitch_then != pitch_now
        }
        // farm: cut; glue: pitch
        4 => then.cut != now.cut || pitch_then != pitch_now,
        // fish: hunt, point; small: trap, bow×point
        12 => then.hunt != now.hunt || then.point != now.point || then.trap != now.trap || then.bow != now.bow || then.axe != now.axe || pitch_then != pitch_now,
        // small: hunt, net, bow×point
        13 => then.hunt != now.hunt || then.net != now.net || then.point != now.point || then.bow != now.bow || then.axe != now.axe || pitch_then != pitch_now,
        _ => true,
    }
}

/// Food minus the food the upkeep labour would have grown. Recipe skill does not scale it.
/// `labour` is the economy's fraction; price is food per unit of working labour.
fn price_of(food: f32, labour: f32, view: &BandView) -> f32 {
    let pop = view.pop.max(1.0);
    let units = labour * LABOUR_SHARE * pop;
    let price = view.food.max(FOOD_EPS) / (WORK_SHARE * LABOUR_SHARE * pop);
    food - price * units
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

/// Material term, user factor and recipe quality. `info` supplies quality; recipe skill
/// and depth are not applied here. `user` all 1.0 reproduces the pre-factor multipliers.
pub fn kit_yields<F: Fn(Item) -> (f32, f32, u8)>(k: &Kit, info: &F, user: &[f32; NFORM]) -> Kit {
    let mut y = *k;
    apply_base(&mut y, user);
    apply_quality(&mut y, &|it| info(it).1);
    y
}

/// Multipliers from the items in the kit, before recipe quality. Food lines go through
/// `yields_base` (user factor on hunt, cut, point, axe, net, trap and dig). Boat, digest
/// and protection stay unscaled; the axe inside protection is the material term, not `uf`.
pub(crate) fn apply_base(k: &mut Kit, user: &[f32; NFORM]) {
    let q = |it: Item, f: &dyn Fn(Mat) -> f32| if it == NONE { 0.0 } else { f(mat(it)) };
    let axe = q(k.axe, &|m| 0.5 * cut_q(m) + 0.5 * durability(m));
    let hammer = if k.hammer == NONE { 0.0 } else { 1.0 };
    let fire = if k.fire == NONE { 0.0 } else { 1.0 };
    let kiln = if k.kiln == NONE { 0.0 } else { 1.0 };
    let ceramic = if k.vessel != NONE && mat(k.vessel) == Mat::Ceramic { 1.0 } else { 0.0 };
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
    let m = yields_base(&yargs_of(k, usize::MAX, &|_| (0.0, 0.0, 0), user, &quiet));
    k.plants_m = m[0];
    k.game_m = m[1];
    k.fish_m = m[2];
    k.small_m = m[3];
    k.farm_m = m[4];
    let ck = |j: usize, g: f32| if k.cooked[j] != NONE { g } else { 1.0 };
    // grain: boiled into porridge (pots help) or ground on a quern and baked
    let grain = if k.cooked_grain != NONE { 1.3 + 0.15 * ceramic + 0.1 * hammer } else { 1.0 + 0.1 * hammer };
    k.digest = [ck(0, 1.25 + 0.15 * ceramic), ck(1, 1.2), ck(2, 1.15), ck(1, 1.2), grain];
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
        let base = yargs_of(kit, slot, info, &view.user, &quiet);
        let m0 = yields(&base);
        let m1 = yields_with(slot, &base, &c, kit.bow != NONE, &view.user, &quiet);
        let food = food_acc(&w, &m0, &m1);
        let mut with = *kit;
        set_slot(&mut with, slot, it);
        let labour = tool_upkeep(view.upkeep_rate, it, c.d, sealed_factor(it, &with, pitch_in_use(&with, view.wet)));
        (food, labour)
    } else {
        let mut empty = *kit;
        set_slot(&mut empty, slot, NONE);
        let ye = kit_yields(&empty, info, &view.user);
        let mut with = empty;
        set_slot(&mut with, slot, it);
        let yw = kit_yields(&with, info, &view.user);
        let food = food_delta(view, &yw, &ye);
        let labour = tool_upkeep(view.upkeep_rate, it, info(it).2, sealed_factor(it, &with, pitch_in_use(&with, view.wet)));
        (food, labour)
    };
    let mut with = *kit;
    set_slot(&mut with, slot, it);
    // Novice spread is `1 + NOVICE_SD*(1-u)*z` on the tool term, z uniform on [-1, 1]
    // (sd 1/sqrt(3)). Counted here as that relative width times |food|, and added onto
    // `food_sd`. Later steps add their own contributions the same way (a sum, not a
    // root-sum-square). Only the food-slot tools carry it; `p_fail` stays 0.
    let food_sd = if it != NONE && is_food_slot(slot) {
        let u = view.user[form(it) as usize];
        NOVICE_SD * (1.0 - u) / 3.0f32.sqrt() * food.abs()
    } else {
        0.0
    };
    Effects {
        food,
        food_sd,
        labour,
        mats: marginal_mats(if it == NONE { 0 } else { mask }, view.at_hand),
        p_fail: 0.0,
        fight: fight_of(slot, it, &with),
    }
}

/// The decision's current and full-skill scores for `it` in `slot`, given the other slots
/// in `kit`. Food: `cur` uses `view.user`, `full` is the same yields at user 1 (trial
/// potential). Both are food minus the food that labour would have grown; recipe skill does
/// not scale them. The other payoff slots keep `KIT_UPKEEP`, with `uf` in place of the old
/// recipe-skill discount. A hull is still ranked by soundness.
pub fn slot_score<F: Fn(Item) -> (f32, f32, u8)>(view: &BandView, kit: &Kit, slot: usize, it: Item, info: &F) -> (f32, f32) {
    let (_sk, ql, d) = info(it);
    let u = if it == NONE { 1.0 } else { view.user[form(it) as usize] };
    if is_food_slot(slot) {
        let c = cand_new(it, u, ql, d);
        let w = weights(view, &digest_of(kit));
        let pitch = pitch_in_use(kit, view.wet);
        let labour = tool_upkeep(view.upkeep_rate, it, d, food_sealed(slot, it, kit, pitch));
        let food_at = |user: &[f32; NFORM]| {
            let base = yargs_of(kit, slot, info, user, &quiet);
            let m0 = yields(&base);
            let m1 = yields_with(slot, &base, &c, kit.bow != NONE, user, &quiet);
            food_acc(&w, &m0, &m1)
        };
        (price_of(food_at(&view.user), labour, view), price_of(food_at(&USER_FULL), labour, view))
    } else {
        nonfood_scores(slot, it, u, ql, d)
    }
}

fn nonfood_qv(slot: usize, m: Mat) -> f32 {
    match slot {
        11 | 16 | 18 | 22 => boat_q(m),
        14 | 15 => m.props().tough + 1.0,
        _ => 0.0,
    }
}

/// Ranking for a boat, bow, shield, outrigger, plank or ship: material term times recipe
/// quality, times the user factor, minus `KIT_UPKEEP`. `full` is the same at `u_eff` 1.
/// Not the real upkeep (a hull is not ranked by caulking).
fn nonfood_scores(slot: usize, it: Item, u_eff: f32, ql: f32, d: u8) -> (f32, f32) {
    let m = mat(it);
    nonfood_from(nonfood_qv(slot, m), u_eff, ql, 1.15 - durability(m), 1.0 + 0.3 * d as f32)
}

fn nonfood_from(term: f32, u_eff: f32, ql: f32, dur_term: f32, depth_f: f32) -> (f32, f32) {
    let up = KIT_UPKEEP * dur_term * depth_f;
    let gain = term * (1.0 + QUALITY_YIELD * ql);
    let uf = user_factor(u_eff);
    (gain * uf - up, gain - up)
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
    let base = yargs_of(k, slot, info, &view.user, &quiet);
    let m0 = yields(&base);
    let base_f = yargs_rescaled(k, slot, &USER_FULL, &quiet, quals_of(&base));
    let m0_f = yields(&base_f);
    let pitch = pitch_in_use(k, view.wet);
    // one food for every point: presence changes the multipliers, the material does not,
    // and every point is the same form so it has the same user factor
    let point_food = if slot == 10 {
        let mut a = base;
        a.point = user_factor(view.user[Form::Point as usize]);
        a.bow = k.bow != NONE;
        let mut af = base_f;
        af.point = 1.0;
        af.bow = k.bow != NONE;
        Some((food_acc(w, &m0, &yields(&a)), food_acc(w, &m0_f, &yields(&af))))
    } else {
        None
    };
    let bow_item = k.bow != NONE;
    for list in lists {
        for c in *list {
            let (food, food_f) = if let Some(pair) = point_food {
                pair
            } else {
                (
                    food_acc(w, &m0, &yields_with(slot, &base, c, bow_item, &view.user, &quiet)),
                    food_acc(w, &m0_f, &yields_with(slot, &base_f, c, bow_item, &USER_FULL, &quiet)),
                )
            };
            let labour = tool_upkeep(view.upkeep_rate, c.it, c.d, food_sealed(slot, c.it, k, pitch));
            consider(price_of(food, labour, view), price_of(food_f, labour, view), c.it, &mut best_cur, &mut best_it, &mut pot);
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
            let (_sk, ql, d) = info(it);
            buckets[bi].push(cand_new(it, view.user[f as usize], ql, d));
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
    apply_base(&mut k, &view.user);
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
