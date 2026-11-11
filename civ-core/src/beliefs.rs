//! (skills step 4) What a band believes a tool does.
//!
//! Priors use visible material terms and the band's own beliefs. Updates use
//! an outcome the caller already observed. This module does not call
//! `tool_effects`. The season loop does not call it until a later sub-part.

use crate::effects::motion_cos;
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
pub const FLAG_OWN: u8 = 1;
pub const FLAG_INHERITED: u8 = 2;
pub const FLAG_TOLD: u8 = 4;
pub const FLAG_SEEN: u8 = 8;

/// A prior for one item in one slot, before any season of use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoodPrior {
    pub n: f32,
    pub mean: f32,
    pub var: f32,
    pub fail: f32,
}

/// `learn_curve` and the mean after the first season of own use.
/// `first_mean` is 0 until that season. It is not a saved field.
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

/// Update `b` and, when this season completes the first one of own use, remember
/// its mean. Blend `learn` when `n` reaches the cap.
pub fn observe(b: &mut crate::band::Belief, learn: &mut LearnState, x: f32, w: f32, failed: bool, labour_obs: f32) {
    let n0 = if b.n.is_finite() { b.n } else { 0.0 };
    let crossed = update_belief(b, x, w, failed, labour_obs);
    let own0 = (n0 - PRIOR_N).max(0.0);
    let own1 = (b.n - PRIOR_N).max(0.0);
    if learn.first_mean == 0.0 && own0 < 1.0 && own1 >= 1.0 {
        learn.first_mean = b.mean;
    }
    if crossed {
        learn.curve = blend_learn_curve(learn.curve, b.mean, learn.first_mean);
    }
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
