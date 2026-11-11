//! (skills step 4) What a band believes a tool does.
//!
//! Priors use visible material terms and the band's own beliefs. Updates use
//! an outcome the caller already observed. This module does not call
//! `tool_effects`. Founders and a load of an empty table are filled once from
//! the kit. The season loop records payouts, fades unused entries, copies
//! them to a daughter at a split, and hears one belief from a close tie. It
//! does not read beliefs when choosing a kit.

use crate::effects::{motion_cos, user_factor, NOVICE_SD};
use crate::materials::{boat_q, cut_q, durability, form, mat, mesh_q, valid_item, Form, Item};

/// Entries a band can hold.
pub const BELIEF_CAP: usize = 24;
/// Cap on effective observations. Old seasons fade past it.
pub const BELIEF_NMAX: f32 = 8.0;
/// Own seasons of use before a full switch (otherwise a trial only).
pub const BELIEF_ADOPT_N: f32 = 1.0;
/// Floor on believed variance, as a share of mean squared.
pub const BELIEF_VAR_MIN: f32 = 0.0025;
/// Yearly factor on `n` of an unused belief.
pub const BELIEF_FADE: f32 = 0.8;
/// Below this after a fade, the entry is dropped.
pub const BELIEF_DROP_N: f32 = 0.1;
/// Daughter's share of the parent's `n`.
pub const BELIEF_INHERIT_N: f32 = 0.6;
/// Variance of an inherited or told belief is multiplied by `1 + BELIEF_WIDEN`.
pub const BELIEF_WIDEN: f32 = 0.5;
/// Chance a year that a close tie passes one belief.
pub const BELIEF_TELL_P: f32 = 0.2;
/// Weight of that testimony, times the partner's `n / BELIEF_NMAX`.
pub const TELL_W: f32 = 0.5;
/// Hearsay never pushes `n` past this.
pub const TELL_NMAX: f32 = 1.0;
/// Prior strength of the believed failure chance.
pub const BELIEF_FAIL_N0: f32 = 2.0;
/// Weight of a prior against one season of own use.
pub const PRIOR_N: f32 = 0.5;
/// Pull of the band's average ratio when few similar tools are known.
pub const PRIOR_N0: f32 = 1.0;
/// Prior spread, as a coefficient of variation.
pub const PRIOR_CV: f32 = 0.25;
/// Extra prior spread for a tool unlike anything used. Full novelty multiplies var by `1 + PRIOR_NOVEL`.
pub const PRIOR_NOVEL: f32 = 2.0;
/// Believed failure chance with no same-technique experience.
pub const PRIOR_FAIL: f32 = 0.15;
/// Blend rate of `learn_curve` when a belief reaches `BELIEF_NMAX`. `LC_PRIOR` lives in `effects`.
pub const LC_RATE: f32 = 0.2;
/// Weight of finding out, in the value of a trial (step 5).
pub const INFO_W: f32 = 0.3;
/// Seasons after first use in which a drop below the prior counts as a write-off.
pub const WRITE_OFF_SEASONS: u32 = 4;

/// Own use, copied at a split, heard from a partner, seen on a band that won.
/// `FLAG_YEAR` is set by a payout update and cleared at the yearly fade. It is
/// a bit of the saved flags byte, not a new field, so a mid-year save fades
/// the same entries.
pub const FLAG_OWN: u8 = 1;
pub const FLAG_INHERITED: u8 = 2;
pub const FLAG_TOLD: u8 = 4;
pub const FLAG_SEEN: u8 = 8;
pub const FLAG_YEAR: u8 = 16;

/// Share of a slot a trial records. The economy still pays the incumbent in
/// full (splitting the food is step 5); the tried tool is an extra observation
/// at this weight.
pub const TRIAL_SHARE: f32 = 0.25;

/// A prior for one item in one slot, before any season of use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoodPrior {
    pub n: f32,
    pub mean: f32,
    pub var: f32,
    pub fail: f32,
}

/// `learn_curve` for a unit test. `first_mean` is not used by the season loop:
/// the climb is folded into the saved curve (see `fold_learn_curve`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LearnState {
    pub curve: f32,
    pub first_mean: f32,
}

impl LearnState {
    pub fn new() -> Self {
        Self { curve: crate::effects::LC_PRIOR, first_mean: 0.0 }
    }
}

/// What `remember` did. `stored` is false when the table was full of kit items,
/// or the `(item, slot)` was already there (nothing is evicted in that case).
#[derive(Clone, Debug, PartialEq)]
pub struct Remembered {
    pub evicted: Option<crate::band::Belief>,
    pub stored: bool,
}

/// Visible property term of `it` in payoff slot `slot` (proposal `material_term`).
/// Not a food total and not `tool_effects`. Point slot 10 is presence, not material.
pub fn material_term(slot: usize, it: Item) -> f32 {
    if !valid_item(it) {
        return 0.0;
    }
    let m = mat(it);
    let cq = cut_q(m);
    match slot {
        0 => match form(it) {
            Form::Spear => 0.35 + 0.65 * cq,
            Form::Point => 0.35 * cq,
            Form::Edge => 0.2 * cq,
            _ => 0.0,
        },
        1 => {
            if form(it) == Form::Edge {
                cq
            } else {
                0.0
            }
        }
        10 => {
            if form(it) == Form::Point {
                1.0
            } else {
                0.0
            }
        }
        4 => {
            if form(it) == Form::Axe {
                0.5 * cq + 0.5 * durability(m)
            } else {
                0.0
            }
        }
        12 | 13 => {
            if matches!(form(it), Form::Net | Form::Trap) {
                mesh_q(m)
            } else {
                0.0
            }
        }
        11 | 16 | 18 | 22 => {
            if matches!(form(it), Form::Boat | Form::Outrigger | Form::Plank | Form::Ship) {
                boat_q(m)
            } else {
                0.0
            }
        }
        14 | 15 => {
            if matches!(form(it), Form::Bow | Form::Shield) {
                m.props().tough + 1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

/// Prior for `it` in `slot`. `fail_same` is the band's believed failure rate for
/// the same technique, if it has one. `except` is an entry to leave out (the
/// belief being faded).
pub fn food_prior(beliefs: &[crate::band::Belief], slot: u8, it: Item, fail_same: Option<f32>, except: Option<usize>) -> FoodPrior {
    let want = if valid_item(it) { form(it) as u8 } else { u8::MAX };
    let mut num = 0.0;
    let mut den = 0.0;
    let mut all_n = 0.0;
    let mut all_r = 0.0;
    let mut max_s = 0.0;
    for (i, b) in beliefs.iter().enumerate() {
        if except == Some(i) || !valid_item(b.item) || !(b.n > 0.0) {
            continue;
        }
        let pv = material_term(b.slot as usize, b.item);
        if !(pv > 0.0) {
            continue;
        }
        let rho_j = b.mean / pv;
        if !rho_j.is_finite() {
            continue;
        }
        all_n += b.n;
        all_r += b.n * rho_j;
        let s = motion_cos(form(b.item) as u8, want);
        if s > max_s {
            max_s = s;
        }
        num += s * b.n * rho_j;
        den += s * b.n;
    }
    let rho_all = if all_n > 0.0 { all_r / all_n } else { 1.0 };
    let rho = (num + PRIOR_N0 * rho_all) / (den + PRIOR_N0);
    let mean = material_term(slot as usize, it) * rho;
    let novel = 1.0 + PRIOR_NOVEL * (1.0 - max_s);
    let var = (PRIOR_CV * mean) * (PRIOR_CV * mean) * novel;
    FoodPrior { n: PRIOR_N, mean, var, fail: fail_same.unwrap_or(PRIOR_FAIL) }
}

/// `n`-weighted mean failure of the beliefs `same` accepts. `None` when none qualify.
pub fn technique_fail(beliefs: &[crate::band::Belief], same: impl Fn(&crate::band::Belief) -> bool) -> Option<f32> {
    let mut n = 0.0;
    let mut s = 0.0;
    for b in beliefs {
        if !(b.n > 0.0) || !same(b) {
            continue;
        }
        n += b.n;
        s += b.n * b.fail;
    }
    if n > 0.0 { Some(s / n) } else { None }
}

/// One observation. `x` is the realised multiplier, `w` its weight, `failed` the
/// make roll, `labour_obs` the upkeep seen. Returns whether `n` reached
/// `BELIEF_NMAX` on this call. `w <= 0` changes nothing.
pub fn update_belief(b: &mut crate::band::Belief, x: f32, w: f32, failed: bool, labour_obs: f32) -> bool {
    if !(w > 0.0) || !x.is_finite() {
        return false;
    }
    let n = if b.n.is_finite() && b.n > 0.0 { b.n } else { 0.0 };
    let k = w / (n + w);
    let mean0 = b.mean;
    let mean = mean0 + k * (x - mean0);
    let var = (1.0 - k) * (b.var + k * (x - mean0) * (x - mean0));
    b.mean = mean;
    b.var = var.max(BELIEF_VAR_MIN * mean * mean);
    let strength = BELIEF_FAIL_N0 + n;
    let obs = if failed { 1.0 } else { 0.0 };
    if b.fail.is_finite() {
        b.fail = (b.fail * strength + w * obs) / (strength + w);
    }
    if labour_obs.is_finite() && b.labour.is_finite() {
        b.labour += k * (labour_obs - b.labour);
    }
    let crossed = n < BELIEF_NMAX && n + w >= BELIEF_NMAX;
    b.n = (n + w).min(BELIEF_NMAX);
    b.flags |= FLAG_OWN;
    crossed
}

/// `learn_curve` blended toward `mean_now / mean_first`. A non-positive or
/// non-finite first mean leaves the curve alone.
pub fn blend_learn_curve(curve: f32, mean_now: f32, mean_first: f32) -> f32 {
    if !(mean_first > 0.0) || !mean_now.is_finite() {
        return curve;
    }
    let ratio = mean_now / mean_first;
    if !ratio.is_finite() || ratio < 0.0 {
        return curve;
    }
    curve + LC_RATE * (ratio - curve)
}

/// Fold one own-use improvement into `learn_curve`. No per-belief baseline.
///
/// The proposal blends once, at `LC_RATE`, toward `mean_cap / mean_first` when
/// `n` reaches the cap. That first-season mean is not saved. While own-use
/// weight `n - PRIOR_N` is already at least 1 (so this update is past the
/// baseline) and `n` is still below the cap, add `ln(mean_after / mean_before)`
/// on a log scale. A full climb from that season to the cap (`SPAN` of `n`)
/// moves `ln(curve)` about `LC_RATE` of the way to `ln(ratio)`. At the cap,
/// and on the update that first completes a season, the curve is left alone.
/// A climb back after fade teaches again, because `n` is below the cap.
pub fn fold_learn_curve(curve: f32, mean_before: f32, mean_after: f32, n_before: f32, n_after: f32) -> f32 {
    let own = n_before - PRIOR_N;
    if !(own >= 1.0) || !(n_before < BELIEF_NMAX) || !(curve > 0.0) {
        return curve;
    }
    if !(mean_before > 0.0) || !(mean_after > 0.0) {
        return curve;
    }
    let ratio = mean_after / mean_before;
    if !ratio.is_finite() || !(ratio > 0.0) {
        return curve;
    }
    let span = BELIEF_NMAX - PRIOR_N - 1.0;
    let dn = (n_after.min(BELIEF_NMAX) - n_before).max(0.0);
    if !(span > 0.0) || !(dn > 0.0) {
        return curve;
    }
    let frac = (dn / span).min(1.0);
    let ln_new = curve.ln() * (1.0 - LC_RATE * frac) + LC_RATE * ratio.ln();
    let out = ln_new.exp();
    if out.is_finite() && out > 0.0 { out } else { curve }
}

/// Update `b` and fold the improvement into `learn.curve` when this observation
/// is past the first season and still under the cap. `first_mean` is not read.
pub fn observe(b: &mut crate::band::Belief, learn: &mut LearnState, x: f32, w: f32, failed: bool, labour_obs: f32) {
    let n0 = if b.n.is_finite() { b.n } else { 0.0 };
    let mean0 = b.mean;
    update_belief(b, x, w, failed, labour_obs);
    learn.curve = fold_learn_curve(learn.curve, mean0, b.mean, n0, b.n);
}

/// Record one payout. No entry yet: store the prior (if the table has room),
/// then update. Returns `(formed, evicted)`. A duplicate that is already
/// stored is updated, not formed again. `FLAG_YEAR` marks it used this year.
pub fn note_payout(
    beliefs: &mut Vec<crate::band::Belief>,
    curve: &mut f32,
    slot: u8,
    it: Item,
    x: f32,
    w: f32,
    failed: bool,
    labour_obs: f32,
    fail_same: Option<f32>,
    in_kit: impl Fn(&crate::band::Belief) -> bool,
) -> (bool, bool) {
    if !(w > 0.0) || !x.is_finite() || !valid_item(it) || !(material_term(slot as usize, it) > 0.0) {
        return (false, false);
    }
    let mut formed = false;
    let mut evicted = false;
    let idx = if let Some(i) = beliefs.iter().position(|e| e.item == it && e.slot == slot) {
        i
    } else {
        let bel = belief_from_prior(slot, it, food_prior(beliefs, slot, it, fail_same, None));
        let rec = remember(beliefs, bel, in_kit);
        evicted = rec.evicted.is_some();
        if !rec.stored {
            return (false, evicted);
        }
        formed = true;
        beliefs.len() - 1
    };
    let n0 = beliefs[idx].n;
    let mean0 = beliefs[idx].mean;
    update_belief(&mut beliefs[idx], x, w, failed, labour_obs);
    beliefs[idx].flags |= FLAG_YEAR;
    let mean1 = beliefs[idx].mean;
    let n1 = beliefs[idx].n;
    *curve = fold_learn_curve(*curve, mean0, mean1, n0, n1);
    (formed, evicted)
}

/// Drop the entry with the smallest `n` for which `in_kit` is false.
/// Equal `n` keeps the earlier entry. `None` when every entry is in the kit.
pub fn evict_smallest(beliefs: &mut Vec<crate::band::Belief>, in_kit: impl Fn(&crate::band::Belief) -> bool) -> Option<crate::band::Belief> {
    let mut best: Option<usize> = None;
    for (i, b) in beliefs.iter().enumerate() {
        if in_kit(b) {
            continue;
        }
        match best {
            None => best = Some(i),
            Some(j) if b.n < beliefs[j].n => best = Some(i),
            _ => {}
        }
    }
    best.map(|i| beliefs.remove(i))
}

/// Append `bel` unless that `(item, slot)` is already stored. At the cap, evict
/// first. Kit entries are not evicted.
pub fn remember(beliefs: &mut Vec<crate::band::Belief>, bel: crate::band::Belief, in_kit: impl Fn(&crate::band::Belief) -> bool) -> Remembered {
    if beliefs.iter().any(|b| b.item == bel.item && b.slot == bel.slot) {
        return Remembered { evicted: None, stored: false };
    }
    let mut evicted = None;
    if beliefs.len() >= BELIEF_CAP {
        evicted = evict_smallest(beliefs, &in_kit);
        if beliefs.len() >= BELIEF_CAP {
            return Remembered { evicted, stored: false };
        }
    }
    beliefs.push(bel);
    Remembered { evicted, stored: true }
}

/// Once a year. `used[i]` keeps that entry as it is. Every other entry has
/// `n *= BELIEF_FADE` and its mean and var pulled toward a fresh prior by the
/// share of `n` lost. Faded `n` below `BELIEF_DROP_N` is removed.
pub fn fade_unused(beliefs: &mut Vec<crate::band::Belief>, used: &[bool]) {
    assert_eq!(used.len(), beliefs.len(), "fade flags must match the belief table");
    let mut out = Vec::with_capacity(beliefs.len());
    for (i, b) in beliefs.iter().enumerate() {
        if used[i] {
            out.push(b.clone());
            continue;
        }
        let p = food_prior(beliefs, b.slot, b.item, None, Some(i));
        let n2 = b.n * BELIEF_FADE;
        if n2 < BELIEF_DROP_N {
            continue;
        }
        let mut c = b.clone();
        let lost = 1.0 - BELIEF_FADE;
        c.n = n2;
        c.mean = b.mean + lost * (p.mean - b.mean);
        c.var = b.var + lost * (p.var - b.var);
        if c.var < 0.0 {
            c.var = 0.0;
        }
        out.push(c);
    }
    *beliefs = out;
}

/// Copy `src` onto a daughter. `n` is multiplied by `BELIEF_INHERIT_N` and
/// `var` by `1 + BELIEF_WIDEN` (then the variance floor). `FLAG_INHERITED` is
/// set; the parent's other flags, including the year bit and the fight fields,
/// are kept. `learn_curve` is copied, and a curve that was never set (0)
/// becomes `LC_PRIOR`. Returns how many entries were copied.
pub fn inherit_beliefs(dst: &mut Vec<crate::band::Belief>, curve: &mut f32, src: &[crate::band::Belief], src_curve: f32) -> u32 {
    dst.clear();
    for b in src {
        let mut c = b.clone();
        c.n *= BELIEF_INHERIT_N;
        c.var *= 1.0 + BELIEF_WIDEN;
        let floor = BELIEF_VAR_MIN * c.mean * c.mean;
        if c.mean.is_finite() && c.var < floor {
            c.var = floor;
        }
        c.flags |= FLAG_INHERITED;
        dst.push(c);
    }
    *curve = if src_curve > 0.0 { src_curve } else { crate::effects::LC_PRIOR };
    src.len() as u32
}

/// Partner entry the listener should hear, if any. The partner must have used
/// it more (`n` strictly higher on the same item and slot), the visible term
/// must be positive, and `can` must say the listener can make it or nearly
/// make it. The greatest gap in `n` wins; a tie keeps the earlier entry.
pub fn pick_testimony(src: &[crate::band::Belief], dst: &[crate::band::Belief], mut can: impl FnMut(Item) -> bool) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, e) in src.iter().enumerate() {
        if !(e.n > 0.0) || !valid_item(e.item) || !(material_term(e.slot as usize, e.item) > 0.0) {
            continue;
        }
        let have = dst.iter().find(|d| d.item == e.item && d.slot == e.slot).map(|d| d.n).unwrap_or(0.0);
        if !(e.n > have) || !can(e.item) {
            continue;
        }
        let adv = e.n - have;
        if best.map_or(true, |(_, a)| adv > a) {
            best = Some((i, adv));
        }
    }
    best.map(|(i, _)| i)
}

/// Fold `told` in as one pseudo-observation. No entry yet: start from nothing
/// and let the weight land on the partner's numbers (not a fresh prior).
/// Returns `(accepted, evicted)`. A full table of kit items accepts nothing.
pub fn accept_testimony(
    beliefs: &mut Vec<crate::band::Belief>,
    told: &crate::band::Belief,
    w: f32,
    in_kit: impl Fn(&crate::band::Belief) -> bool,
) -> (bool, bool) {
    if !(w > 0.0) || !told.mean.is_finite() || !valid_item(told.item) {
        return (false, false);
    }
    if let Some(i) = beliefs.iter().position(|e| e.item == told.item && e.slot == told.slot) {
        apply_testimony(&mut beliefs[i], told, w);
        return (true, false);
    }
    let mut bel = crate::band::Belief { item: told.item, slot: told.slot, ..Default::default() };
    apply_testimony(&mut bel, told, w);
    let rec = remember(beliefs, bel, in_kit);
    (rec.stored, rec.evicted.is_some())
}

/// One told observation. Mean moves toward `told.mean` with weight `w`.
/// Variance blends toward the partner's variance widened by `1 + BELIEF_WIDEN`.
/// `n` grows by `w` but never past `TELL_NMAX`; an `n` already above that stays.
/// Not own use: `FLAG_TOLD` only, and the caller does not touch `learn_curve`.
fn apply_testimony(b: &mut crate::band::Belief, told: &crate::band::Belief, w: f32) {
    let n = if b.n.is_finite() && b.n > 0.0 { b.n } else { 0.0 };
    let k = w / (n + w);
    let mean0 = if b.mean.is_finite() { b.mean } else { 0.0 };
    let mean = mean0 + k * (told.mean - mean0);
    b.mean = mean;
    let widened = if told.var.is_finite() && told.var > 0.0 { told.var * (1.0 + BELIEF_WIDEN) } else { 0.0 };
    let var0 = if b.var.is_finite() && b.var > 0.0 { b.var } else { 0.0 };
    let var = (1.0 - k) * var0 + k * widened;
    let floor = BELIEF_VAR_MIN * mean * mean;
    b.var = if var.is_finite() && var > floor { var } else { floor };
    if told.fail.is_finite() {
        let f0 = if b.fail.is_finite() { b.fail } else { 0.0 };
        b.fail = f0 + k * (told.fail - f0);
    }
    if told.labour.is_finite() {
        let l0 = if b.labour.is_finite() { b.labour } else { 0.0 };
        b.labour = l0 + k * (told.labour - l0);
    }
    if n < TELL_NMAX {
        b.n = (n + w).min(TELL_NMAX);
    }
    b.flags |= FLAG_TOLD;
}

/// Chooser slot (`best[]` index) of each payoff slot, in `Kit::slot_item` order.
pub const PAYOFF_SLOT: [u8; 12] = [0, 1, 10, 4, 11, 12, 13, 14, 15, 16, 18, 22];

/// Food payoff slots. The economy's novice draw applies to these and not to a hull or a bow.
pub fn food_payoff(slot: u8) -> bool {
    matches!(slot, 0 | 1 | 10 | 4 | 12 | 13)
}

/// Expected multiplier of `it` in `slot`: visible term, user factor, recipe quality.
/// The novice draw averages to 1, so it is not in the mean. Not a food total.
pub fn expected_multiplier(slot: usize, it: Item, u: f32, quality: f32) -> f32 {
    material_term(slot, it) * user_factor(u) * (1.0 + crate::band::QUALITY_YIELD * quality)
}

/// That multiplier times the novice factor the economy already applied (`noise`).
/// `noise` is 1 on a slot the economy does not draw.
pub fn realised_multiplier(slot: usize, it: Item, u: f32, quality: f32, noise: f32) -> f32 {
    expected_multiplier(slot, it, u, quality) * noise
}

/// Variance of that multiplier. Food slots use the novice draw (`z` uniform on
/// [-1, 1), sd `1/sqrt(3)`). Other slots are not noised, so only the floor remains.
pub fn novice_var(mean: f32, u: f32, slot: u8) -> f32 {
    let floor = BELIEF_VAR_MIN * mean * mean;
    if !food_payoff(slot) || !mean.is_finite() {
        return floor;
    }
    let s = NOVICE_SD * (1.0 - u) / 3.0f32.sqrt();
    let spread = (mean * s) * (mean * s);
    if spread > floor { spread } else { floor }
}

/// Numbers for one kit slot's starting belief. `u` is the user skill the mean uses.
pub struct PayoffStart {
    pub slot: u8,
    pub item: Item,
    pub u: f32,
    pub quality: f32,
    pub fail: f32,
    pub labour: f32,
}

/// Starting belief for a founder or an old save. Flags and fight fields stay 0:
/// this is the allowed starting state, not a season of observation.
pub fn starter_belief(n: f32, s: &PayoffStart) -> crate::band::Belief {
    let mean = expected_multiplier(s.slot as usize, s.item, s.u, s.quality);
    crate::band::Belief {
        item: s.item,
        slot: s.slot,
        flags: 0,
        n,
        mean,
        var: novice_var(mean, s.u, s.slot),
        fail: s.fail,
        labour: s.labour,
        ..Default::default()
    }
}

/// A belief filled from a prior. Fight fields stay 0. Labour starts at 0 until an update.
pub fn belief_from_prior(slot: u8, it: Item, p: FoodPrior) -> crate::band::Belief {
    crate::band::Belief {
        item: it,
        slot,
        flags: 0,
        n: p.n,
        mean: p.mean,
        var: p.var,
        fail: p.fail,
        labour: 0.0,
        ..Default::default()
    }
}
