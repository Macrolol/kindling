//! (diagnostic, read-only) How inventions spread, how long bands keep them, and why they are
//! dropped. An ignored test that runs worlds and only *reads* them after each season: it never
//! touches world state or the rng, so the runs it watches are exactly the ones `kindling
//! --headless` makes for the same seed and map (see docs/PROGRESS-diffusion.md).
//!
//! `PROBE_SEEDS=1,2 PROBE_YEARS=1000 PROBE_W=128 PROBE_H=80 PROBE_OUT=probe \
//!  cargo test --release -p civ-core probe_diffusion -- --ignored --nocapture`
//!
//! Writes per seed into PROBE_OUT (default `probe-out`):
//! - `items-<W>x<H>-seed<S>.csv`: one row per output item ever made by a recipe (spread,
//!   lifetimes, origins, forget reasons, use, quality against the incumbent);
//! - `series-<W>x<H>-seed<S>.csv`: every `PROBE_EVERY` years (default 10), per item: bands
//!   that know a recipe for it, can make it now, carry it in their kit, used it this season;
//! - `forgets-<W>x<H>-seed<S>.csv`: every forgetting of a recipe by a living band (year, band
//!   size, how it was learned, years known, seasons used, the reason it was idle, skill and
//!   quality history).
//!
//! Idle reasons (each season a known recipe was not practised), in this order:
//! - `no_input`: an input is not at hand (materials, or an upstream tool the band lacks);
//! - `redundant`: another known recipe already makes the same item (the older one is used);
//! - `idle_slot`: the item *is* the kit's pick for its job but the job was not done (a boat or
//!   sail away from the water, a net with no fish, an ornament, pitch with nothing to seal, a
//!   tally with nothing to count);
//! - `displaced`: the item could fill a kit job but a better item fills it (an old tool after a
//!   new one);
//! - `unneeded`: an intermediate nothing the band uses is made from.
//!
//! (skills step 0, docs/PROPOSAL-skills.md 3.8) Tool choices are also scored against the true
//! value of a kit (`true_value`: food multipliers, boat soundness, protection and fine work,
//! less the upkeep of the payoff-slot tools, plus `PROBE_FIGHT_W` x fight strength), against
//! its food part only (`food`), and against the band's own valuation (`own_value`: the slot pay
//! `make_kit_with` ranks by). Gaps within `PROBE_TIE` are ties.
//! - **adoption**: when a slot's pick changes from A to B (both tools), B's kit is compared
//!   with the kit the band would have with A kept (A forced back into the slot); if A can no
//!   longer be made the change is "forced";
//! - **held** (yearly, `worse_held_share`): band-years an item filled a slot while another
//!   makeable item for that slot gave a kit of higher true value;
//! - "believed wrong" needs beliefs (step 4) and reads NA until then.
//! Extra files: `skills-<W>x<H>-seed<S>.csv` (every `PROBE_EVERY` years: these figures for the
//! period, the skills YearStats fields and counters, NA for what later steps add) and
//! `pairs-<W>x<H>-seed<S>.csv` (which item was held or adopted over which better one).
use crate::band::{edge_q, loss_guard, make_kit, make_kit_among, make_kit_for, refine_kit, weapon_q, Kit, KIT_UPKEEP, QUALITY_YIELD};
use crate::knowledge::{closure, Known, Toolkit, FROM_PARENT, FROM_SELF};
use crate::materials::*;
use crate::{Config, Params, World};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;

const R_NO_INPUT: usize = 0;
const R_REDUNDANT: usize = 1;
const R_IDLE_SLOT: usize = 2;
const R_DISPLACED: usize = 3;
const R_UNNEEDED: usize = 4;
const REASONS: [&str; 5] = ["no_input", "redundant", "idle_slot", "displaced", "unneeded"];
// origins of a band's knowing a recipe
const O_SELF: usize = 0;
const O_PARENT: usize = 1;
const O_TAUGHT: usize = 2;
const O_START: usize = 3;
const ORIGINS: [&str; 4] = ["invented", "inherited", "taught", "start"];

#[derive(Clone, Default)]
struct Track {
    since: u32,
    origin: usize,
    used: u32,
    seasons: u32,
    idle: [u32; 5],
    streak: [u32; 5],
    streak_len: u32,
    arrived_ok: Option<bool>,
    max_skill: f32,
    max_q: f32,
}

#[derive(Clone, Default)]
struct ItemAgg {
    first_year: Option<f32>,
    peak_known: u32,
    peak_year: f32,
    peak_share: f32,
    last_known: u32,
    last_make: u32,
    last_kit: u32,
    extinctions: u32,
    // band-seasons
    bs_known: u64,
    bs_make: u64,
    bs_kit: u64,
    bs_used: u64,
    bs_share_sum: f64,
    seasons_alive: u32,
    gained: [u64; 4],
    taught_ok: u64,
    taught_bad: u64,
    forgets: [u64; 5],
    forgets_band_gone: u64,
    forgets_by_origin: [u64; 4],
    forget_years_sum: f64,
    forget_never_used: u64,
    forget_taught_20y: u64,
    // yearly incumbent comparison, for bands whose kit carries the item
    inc_n: u64,
    inc_none: u64,
    q_new_sum: f64,
    q_inc_sum: f64,
    q_lower: u64,
    worse_refined: u64,
    gain_sum: f64,
    gain_raw_sum: f64,
    // (skills step 0) adoption of this item into a slot, against the item it replaced
    adopt_n: u64,
    adopt_forced: u64,
    /// [worse, tie, better] by true food, true value, own valuation
    adopt_food: [u64; 3],
    adopt_true: [u64; 3],
    adopt_own: [u64; 3],
    /// band-years this item filled a slot, and those with a makeable item of higher true value
    /// (or higher food) for the slot
    held_n: u64,
    held_worse_true: u64,
    held_worse_food: u64,
    held_gap_sum: f64,
}

/// (skills step 0) gaps within this count as ties
const PROBE_TIE: f32 = 0.02;
/// (skills step 0) weight of fight strength (weapons + guard) in the probe's true value
const PROBE_FIGHT_W: f32 = 0.1;

/// (skills step 0) the worth `make_kit_with` gives item `it` in payoff slot `j`
/// (`Kit::slot_item` order), or None if it cannot fill the slot
fn slot_qv(it: Item, j: usize) -> Option<f32> {
    let m = mat(it);
    Some(match (form(it), j) {
        (Form::Spear, 0) => 0.35 + 0.65 * edge_q(m),
        (Form::Point, 0) => 0.35 * edge_q(m),
        (Form::Point, 2) => edge_q(m),
        (Form::Edge, 0) => 0.2 * edge_q(m),
        (Form::Edge, 1) => edge_q(m),
        (Form::Axe, 3) => 0.5 * cut_q(m) + 0.5 * durability(m),
        (Form::Boat, 4) | (Form::Outrigger, 9) | (Form::Plank, 10) | (Form::Ship, 11) => boat_q(m),
        (Form::Net, 5) | (Form::Trap, 6) => mesh_q(m),
        (Form::Bow, 7) | (Form::Shield, 8) => m.props().tough + 1.0,
        _ => return None,
    })
}

/// (skill, quality, chain depth) of the recipe behind an item, as `make_kit_among` reads it
fn info(tk: &Toolkit, known: &[Known], it: Item) -> (f32, f32, u8) {
    match tk.via(it) {
        Some((i, _, d)) if i != u16::MAX => known.get(i as usize).map(|kn| (kn.skill.clamp(0.0, 1.0), kn.quality.clamp(0.0, 1.0), d)).unwrap_or((1.0, 0.0, d)),
        Some((_, _, d)) => (1.0, 0.0, d),
        None => (1.0, 0.0, 0),
    }
}

fn upkeep(it: Item, d: u8) -> f32 {
    KIT_UPKEEP * (1.15 - durability(mat(it))) * (1.0 + 0.3 * d as f32)
}

/// food part of a (refined) kit's payoff
fn food(k: &Kit) -> f32 {
    k.plants_m + k.game_m + k.fish_m + k.small_m + k.farm_m + k.digest.iter().sum::<f32>()
}

/// (skills step 0) layer-1-like true value of a refined kit: food, boat, protection, fine
/// work, less the upkeep of its payoff-slot tools (each tool once), plus fight strength
fn true_value(k: &Kit, tk: &Toolkit, known: &[Known]) -> f32 {
    let mut up = 0.0;
    let mut seen: [Item; 12] = [NONE; 12];
    for j in 0..12 {
        let it = k.slot_item(j);
        if it == NONE || seen[..j].contains(&it) {
            continue;
        }
        seen[j] = it;
        up += upkeep(it, info(tk, known, it).2);
    }
    food(k) + k.boat_q + k.protect + k.fine - up + PROBE_FIGHT_W * (weapon_q(k) + 1.0 - loss_guard(k))
}

/// (skills step 0) the band's own valuation: the slot pay `make_kit_with` ranks by, summed
fn own_value(k: &Kit, tk: &Toolkit, known: &[Known]) -> f32 {
    (0..12)
        .map(|j| {
            let it = k.slot_item(j);
            match (it != NONE).then(|| slot_qv(it, j)).flatten() {
                Some(qv) => {
                    let (sk, ql, d) = info(tk, known, it);
                    qv * (1.0 + QUALITY_YIELD * ql) * (0.5 + 0.5 * sk) - upkeep(it, d)
                }
                None => 0.0,
            }
        })
        .sum()
}

/// (skills step 0) the kit the band would choose with `a` in slot `j`: the items it would
/// pick there instead are taken away one by one until `a` is the pick (None if it never is)
fn force(tk: &Toolkit, known: &[Known], j: usize, a: Item) -> Option<Kit> {
    let mut avail = tk.avail;
    for _ in 0..16 {
        let k = make_kit_among(&avail, tk, known);
        let p = k.slot_item(j);
        if p == a {
            return Some(k);
        }
        if p == NONE {
            return None;
        }
        avail = set_without(&avail, p);
    }
    None
}

/// 0 worse, 1 tie, 2 better (`d` = chosen minus alternative)
fn cmp3(d: f32) -> usize {
    if d < -PROBE_TIE {
        0
    } else if d <= PROBE_TIE {
        1
    } else {
        2
    }
}

/// (skills step 0) totals for a period of the skills CSV, and for the run
#[derive(Clone, Default)]
struct Period {
    adopt_n: u64,
    adopt_forced: u64,
    adopt_food: [u64; 3],
    adopt_true: [u64; 3],
    adopt_own: [u64; 3],
    held_n: u64,
    held_worse_true: u64,
    held_worse_food: u64,
}
impl Period {
    fn add(&mut self, p: &Period) {
        for k in 0..3 {
            self.adopt_food[k] += p.adopt_food[k];
            self.adopt_true[k] += p.adopt_true[k];
            self.adopt_own[k] += p.adopt_own[k];
        }
        self.adopt_n += p.adopt_n;
        self.adopt_forced += p.adopt_forced;
        self.held_n += p.held_n;
        self.held_worse_true += p.held_worse_true;
        self.held_worse_food += p.held_worse_food;
    }
}

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
}

/// What the kit does for a band, summed (food multipliers, boat soundness, protection). The
/// D1-D6 probe measure, kept for the `worse_refined_share` / `gain_mean` columns so they stay
/// comparable with earlier runs; skills step 0 scores choices with `true_value` instead.
fn payoff(k: &Kit) -> f32 {
    k.plants_m + k.game_m + k.fish_m + k.small_m + k.farm_m + k.boat_q + k.protect + k.digest.iter().sum::<f32>() + k.fine
}

fn set_without(s: &ItemSet, x: Item) -> ItemSet {
    let mut o = ItemSet::default();
    for i in s.iter() {
        if i != x {
            o.add(i);
        }
    }
    o
}

/// The toolkit a band has right now, recomputed (indices consistent with `known`).
fn toolkit(w: &World, b: &crate::band::Band) -> Toolkit {
    let tk = &b.d.tk;
    let mut raws = ItemSet::default();
    for i in tk.avail.iter() {
        if !tk.producible.has(i) {
            raws.add(i);
        }
    }
    closure(&w.reg, &b.known, &raws, false)
}

fn kit_items(k: &Kit) -> HashSet<Item> {
    k.used_items().iter().copied().filter(|&i| i != NONE).collect()
}

#[test]
#[ignore]
fn probe_diffusion() {
    let seeds: Vec<u64> = std::env::var("PROBE_SEEDS").unwrap_or("1".into()).split(',').map(|s| s.trim().parse().unwrap()).collect();
    let years: u32 = env("PROBE_YEARS", 500);
    let (mw, mh): (usize, usize) = (env("PROBE_W", 256), env("PROBE_H", 160));
    let every: u32 = env("PROBE_EVERY", 10);
    let out = std::env::var("PROBE_OUT").unwrap_or("probe-out".into());
    std::fs::create_dir_all(&out).unwrap();
    for seed in seeds {
        run_seed(seed, years, mw, mh, every, &out);
    }
}

fn run_seed(seed: u64, years: u32, mw: usize, mh: usize, every: u32, out: &str) {
    let t0 = std::time::Instant::now();
    let mut w = World::new(Config { seed, map_w: mw, map_h: mh, ..Default::default() }, Params::default());
    let tag = format!("{mw}x{mh}-seed{seed}");
    let mut series = std::io::BufWriter::new(std::fs::File::create(format!("{out}/series-{tag}.csv")).unwrap());
    writeln!(series, "year,bands,pop,item,name,known,make,kit,used").unwrap();
    let mut fcsv = std::io::BufWriter::new(std::fs::File::create(format!("{out}/forgets-{tag}.csv")).unwrap());
    writeln!(fcsv, "year,band,pop,recipe,item,name,origin,years_known,seasons,used,reason,{},arrived_ok,max_skill,max_q", REASONS.map(|r| format!("idle_{r}")).join(",")).unwrap();
    // (skills step 0)
    let mut skcsv = std::io::BufWriter::new(std::fs::File::create(format!("{out}/skills-{tag}.csv")).unwrap());
    let sk_counters: Vec<&str> = crate::world::Counters::NAMES.iter().copied().skip_while(|n| *n != "kit_switches").take_while(|n| *n != "max_fight_deaths").collect();
    let sk_fields: Vec<String> = crate::data::year_fields(&crate::world::YearStats::default()).into_iter().map(|(k, _)| k).skip_while(|k| k != "use_skill_edge").collect();
    writeln!(
        skcsv,
        "year,bands,pop,adopt_n,adopt_forced,adopt_worse_food,adopt_tie_food,adopt_worse_true,adopt_tie_true,adopt_worse_own,adopt_tie_own,adopt_believed_wrong,held_n,worse_held_share,worse_held_food_share,trial_rate_pshort_lo,trial_rate_pshort_mid,trial_rate_pshort_hi,belief_err_n1,belief_err_n5,belief_err_n20,fight_belief_err_n1,fight_belief_err_n5,fight_belief_err_n20,inv_payout,inv_decision_blind,inv_perturb,{},{}",
        sk_fields.join(","),
        sk_counters.iter().map(|n| format!("c_{n}")).collect::<Vec<_>>().join(",")
    )
    .unwrap();
    let mut prev_pick: HashMap<u32, [Item; 12]> = HashMap::new();
    let mut period = Period::default();
    let mut tot = Period::default();
    let mut pairs: BTreeMap<(&'static str, Item, Item), (u64, f64)> = BTreeMap::new();
    let mut tracks: HashMap<(u32, u32), Track> = HashMap::new();
    let mut agg: BTreeMap<Item, ItemAgg> = BTreeMap::new();
    let mut prev_bands: HashSet<u32> = w.bands.iter().map(|b| b.id).collect();
    let mut last_tick = 0;
    let mut first = true;
    for t in 0..years * 4 {
        w.step();
        last_tick = t;
        if w.bands.is_empty() {
            break;
        }
        let tick = w.tick;
        let yr = tick as f32 / 4.0;
        let year_end = tick % 4 == 0;
        let nb = w.bands.len() as u32;
        let mut seen: HashSet<(u32, u32)> = HashSet::with_capacity(tracks.len());
        // per item this season: bands knowing / making / kit / used
        let mut cnt: BTreeMap<Item, [u32; 4]> = BTreeMap::new();
        let alive: HashSet<u32> = w.bands.iter().map(|b| b.id).collect();
        for b in &w.bands {
            let tk = toolkit(&w, b);
            // (diffusion D3) the kit the band relies on, as the world chooses it
            let kit = make_kit_for(&tk, &b.known);
            let ki = kit_items(&kit);
            // (skills step 0) adoption: a slot's pick changed since last season
            let picks: [Item; 12] = std::array::from_fn(|j| kit.slot_item(j));
            let mut now_vals: Option<(f32, f32, f32)> = None;
            if let Some(pp) = prev_pick.get(&b.id) {
                for j in 0..12 {
                    let (a, bn) = (pp[j], picks[j]);
                    if a == bn || a == NONE || bn == NONE {
                        continue;
                    }
                    let ag = agg.entry(bn).or_default();
                    ag.adopt_n += 1;
                    period.adopt_n += 1;
                    let forced = if tk.avail.has(a) { force(&tk, &b.known, j, a) } else { None };
                    let Some(mut ka) = forced else {
                        ag.adopt_forced += 1;
                        period.adopt_forced += 1;
                        continue;
                    };
                    let (f0, t0v, o0) = *now_vals.get_or_insert_with(|| {
                        let mut kr = kit;
                        refine_kit(&mut kr, &tk, &b.known);
                        (food(&kr), true_value(&kr, &tk, &b.known), own_value(&kit, &tk, &b.known))
                    });
                    let oa = own_value(&ka, &tk, &b.known);
                    refine_kit(&mut ka, &tk, &b.known);
                    let ta = true_value(&ka, &tk, &b.known);
                    let (cf, ct, co) = (cmp3(f0 - food(&ka)), cmp3(t0v - ta), cmp3(o0 - oa));
                    ag.adopt_food[cf] += 1;
                    ag.adopt_true[ct] += 1;
                    ag.adopt_own[co] += 1;
                    period.adopt_food[cf] += 1;
                    period.adopt_true[ct] += 1;
                    period.adopt_own[co] += 1;
                    if ct == 0 {
                        let e = pairs.entry(("adopt_worse_true", bn, a)).or_default();
                        e.0 += 1;
                        e.1 += (ta - t0v) as f64;
                    }
                }
            }
            prev_pick.insert(b.id, picks);
            let mut item_seen: HashSet<Item> = HashSet::new();
            let mut item_used: HashSet<Item> = HashSet::new();
            for (i, k) in b.known.iter().enumerate() {
                let r = &w.reg.recipes[k.recipe as usize];
                let o = r.output;
                let key = (b.id, k.recipe);
                seen.insert(key);
                let tr = tracks.entry(key).or_insert_with(|| {
                    let origin = if first {
                        O_START
                    } else if k.from == FROM_SELF {
                        O_SELF
                    } else if k.from == FROM_PARENT {
                        O_PARENT
                    } else {
                        O_TAUGHT
                    };
                    let a = agg.entry(o).or_default();
                    a.gained[origin] += 1;
                    if a.first_year.is_none() {
                        a.first_year = Some(r.first_tick as f32 / 4.0);
                    }
                    Track { since: tick, origin, ..Default::default() }
                });
                tr.seasons += 1;
                tr.max_skill = tr.max_skill.max(k.skill);
                tr.max_q = tr.max_q.max(k.quality);
                let makes = tk.via(o).map(|v| v.0 as usize == i).unwrap_or(false);
                if tr.arrived_ok.is_none() && tr.origin == O_TAUGHT {
                    tr.arrived_ok = Some(tk.producible.has(o));
                    let a = agg.entry(o).or_default();
                    if tk.producible.has(o) {
                        a.taught_ok += 1;
                    } else {
                        a.taught_bad += 1;
                    }
                }
                item_seen.insert(o);
                if k.used {
                    tr.used += 1;
                    tr.streak = [0; 5];
                    tr.streak_len = 0;
                    item_used.insert(o);
                } else {
                    let reason = if !r.inputs.iter().all(|&x| x == NONE || tk.avail.has(x)) {
                        R_NO_INPUT
                    } else if !makes {
                        R_REDUNDANT
                    } else if ki.contains(&o) {
                        R_IDLE_SLOT
                    } else if kit_items(&make_kit(&{
                        let mut s = ItemSet::default();
                        s.add(o);
                        s
                    }))
                    .contains(&o)
                    {
                        R_DISPLACED
                    } else {
                        R_UNNEEDED
                    };
                    tr.idle[reason] += 1;
                    tr.streak[reason] += 1;
                    tr.streak_len += 1;
                }
            }
            for &o in &item_seen {
                let c = cnt.entry(o).or_default();
                c[0] += 1;
                if tk.producible.has(o) {
                    c[1] += 1;
                }
                if ki.contains(&o) {
                    c[2] += 1;
                }
                if item_used.contains(&o) {
                    c[3] += 1;
                }
            }
            // yearly: every recipe-made kit item against what the band would use without it
            if year_end {
                let mut kr = kit;
                refine_kit(&mut kr, &tk, &b.known);
                // (skills step 0) held: is another makeable item better in any slot?
                let (f0, t0v) = (food(&kr), true_value(&kr, &tk, &b.known));
                for j in 0..12 {
                    let x = picks[j];
                    if x == NONE {
                        continue;
                    }
                    let (mut best_t, mut best_f, mut best_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY, NONE);
                    for y in tk.avail.iter() {
                        if y == x || slot_qv(y, j).is_none() {
                            continue;
                        }
                        if let Some(mut ky) = force(&tk, &b.known, j, y) {
                            refine_kit(&mut ky, &tk, &b.known);
                            let tv = true_value(&ky, &tk, &b.known);
                            if tv > best_t {
                                (best_t, best_y) = (tv, y);
                            }
                            best_f = best_f.max(food(&ky));
                        }
                    }
                    let ag = agg.entry(x).or_default();
                    ag.held_n += 1;
                    period.held_n += 1;
                    if best_t - t0v > PROBE_TIE {
                        ag.held_worse_true += 1;
                        ag.held_gap_sum += (best_t - t0v) as f64;
                        period.held_worse_true += 1;
                        let e = pairs.entry(("held_worse_true", x, best_y)).or_default();
                        e.0 += 1;
                        e.1 += (best_t - t0v) as f64;
                    }
                    if best_f - f0 > PROBE_TIE {
                        ag.held_worse_food += 1;
                        period.held_worse_food += 1;
                    }
                }
                let q = |tk: &Toolkit, it: Item| -> f32 {
                    tk.via(it).and_then(|(i, _, _)| b.known.get(i as usize)).map(|kn: &Known| kn.quality).unwrap_or(0.0)
                };
                for &x in &ki {
                    if !tk.producible.has(x) || form(x) == Form::Cooked || form(x) == Form::Fire {
                        continue;
                    }
                    let avail2 = set_without(&tk.avail, x);
                    let kit2 = make_kit_among(&avail2, &tk, &b.known);
                    let mut kr2 = kit2;
                    refine_kit(&mut kr2, &tk, &b.known);
                    let k2 = kit_items(&kit2);
                    let rep: Vec<Item> = k2.iter().copied().filter(|i| !ki.contains(i)).collect();
                    let a = agg.entry(x).or_default();
                    if rep.is_empty() {
                        a.inc_none += 1;
                        continue;
                    }
                    let inc = rep[0];
                    let (qn, qi) = (q(&tk, x), q(&tk, inc));
                    a.inc_n += 1;
                    a.q_new_sum += qn as f64;
                    a.q_inc_sum += qi as f64;
                    if qn < qi {
                        a.q_lower += 1;
                    }
                    let gain = payoff(&kr) - payoff(&kr2);
                    let gain_raw = payoff(&kit) - payoff(&kit2);
                    a.gain_sum += gain as f64;
                    a.gain_raw_sum += gain_raw as f64;
                    if gain < 0.0 {
                        a.worse_refined += 1;
                    }
                }
            }
        }
        // what was lost this season
        let gone: Vec<(u32, u32)> = tracks.keys().filter(|k| !seen.contains(k)).copied().collect();
        for key in gone {
            let tr = tracks.remove(&key).unwrap();
            let o = w.reg.recipes[key.1 as usize].output;
            let a = agg.entry(o).or_default();
            if !alive.contains(&key.0) {
                a.forgets_band_gone += 1;
                continue;
            }
            let reason = if tr.streak_len == 0 {
                // dropped while practised: cannot come from forgetting
                usize::MAX
            } else {
                (0..5).max_by_key(|&r| (tr.streak[r], 5 - r)).unwrap()
            };
            let yrs = (tick - tr.since) as f32 / 4.0;
            if reason < 5 {
                a.forgets[reason] += 1;
            }
            a.forgets_by_origin[tr.origin] += 1;
            a.forget_years_sum += yrs as f64;
            if tr.used <= 1 {
                a.forget_never_used += 1;
            }
            if tr.origin == O_TAUGHT && yrs <= 20.0 {
                a.forget_taught_20y += 1;
            }
            let pop = w.bands.iter().find(|b| b.id == key.0).map(|b| b.pop).unwrap_or(0);
            writeln!(
                fcsv,
                "{yr},{},{pop},{},{o},\"{}\",{},{yrs},{},{},{},{},{},{:.3},{:.3}",
                key.0,
                key.1,
                item_name(o),
                ORIGINS[tr.origin],
                tr.seasons,
                tr.used,
                if reason < 5 { REASONS[reason] } else { "other" },
                tr.idle.map(|x| x.to_string()).join(","),
                tr.arrived_ok.map(|x| x as u8 as i32).unwrap_or(-1),
                tr.max_skill,
                tr.max_q
            )
            .unwrap();
        }
        // world aggregates
        let pop = w.total_pop();
        for (&o, a) in agg.iter_mut() {
            let c = cnt.get(&o).copied().unwrap_or([0; 4]);
            if a.last_known > 0 && c[0] == 0 {
                a.extinctions += 1;
            }
            a.last_known = c[0];
            a.last_make = c[1];
            a.last_kit = c[2];
            a.bs_known += c[0] as u64;
            a.bs_make += c[1] as u64;
            a.bs_kit += c[2] as u64;
            a.bs_used += c[3] as u64;
            if c[0] > 0 {
                let share = c[0] as f32 / nb as f32;
                a.bs_share_sum += share as f64;
                a.seasons_alive += 1;
                if c[0] > a.peak_known {
                    a.peak_known = c[0];
                    a.peak_year = yr;
                }
                if share > a.peak_share {
                    a.peak_share = share;
                }
            }
        }
        prev_pick.retain(|id, _| alive.contains(id));
        if year_end && (tick / 4) % every == 0 {
            let p = std::mem::take(&mut period);
            tot.add(&p);
            let na = p.adopt_n - p.adopt_forced;
            let d = |x: u64, n: u64| if n > 0 { format!("{:.4}", x as f64 / n as f64) } else { "NA".to_string() };
            let ys: Vec<String> = match w.history.last() {
                Some(h) => crate::data::year_fields(h).into_iter().skip_while(|(k, _)| k != "use_skill_edge").map(|(_, v)| v).collect(),
                None => vec!["NA".to_string(); sk_fields.len()],
            };
            let cv: Vec<String> = crate::world::Counters::NAMES.iter().zip(w.counters.values()).filter(|(n, _)| sk_counters.contains(*n)).map(|(_, v)| v.to_string()).collect();
            writeln!(
                skcsv,
                "{},{nb},{pop},{},{},{},{},{},{},{},{},NA,{},{},{},NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,{},{}",
                tick / 4,
                p.adopt_n,
                p.adopt_forced,
                d(p.adopt_food[0], na),
                d(p.adopt_food[1], na),
                d(p.adopt_true[0], na),
                d(p.adopt_true[1], na),
                d(p.adopt_own[0], na),
                d(p.adopt_own[1], na),
                p.held_n,
                d(p.held_worse_true, p.held_n),
                d(p.held_worse_food, p.held_n),
                ys.join(","),
                cv.join(",")
            )
            .unwrap();
            for (&o, c) in &cnt {
                writeln!(series, "{},{nb},{pop},{o},\"{}\",{},{},{},{}", tick / 4, item_name(o), c[0], c[1], c[2], c[3]).unwrap();
            }
        }
        prev_bands = alive;
        first = false;
        if year_end && (tick / 4) % 100 == 0 {
            eprintln!("  {tag} year {} bands {nb} pop {pop} tracked {} ({:.0}s)", tick / 4, tracks.len(), t0.elapsed().as_secs_f32());
        }
    }
    let _ = prev_bands;
    let nb = w.bands.len().max(1) as f32;
    let mut f = std::io::BufWriter::new(std::fs::File::create(format!("{out}/items-{tag}.csv")).unwrap());
    writeln!(
        f,
        "item,name,form,mat,first_year,inventions,transfers,peak_known,peak_year,peak_share,final_known,final_share,final_make,final_kit,mean_share_alive,seasons_alive,extinctions,bs_known,bs_make,bs_kit,bs_used,gained_invented,gained_inherited,gained_taught,gained_start,taught_ok,taught_bad,{},forgets_band_gone,forgets_invented,forgets_inherited,forgets_taught,mean_years_known_at_forget,forget_never_used,forget_taught_within_20y,inc_n,inc_none,q_new_mean,q_inc_mean,q_lower_share,worse_refined_share,gain_mean,gain_raw_mean,adopt_n,adopt_forced,adopt_worse_food,adopt_tie_food,adopt_worse_true,adopt_tie_true,adopt_worse_own,adopt_tie_own,adopt_believed_wrong,held_n,worse_held_share,worse_held_food_share,held_gap_mean",
        REASONS.map(|r| format!("forget_{r}")).join(",")
    )
    .unwrap();
    for (&o, a) in &agg {
        let rec: Vec<&crate::knowledge::Recipe> = w.reg.recipes.iter().filter(|r| r.output == o).collect();
        let inv: u32 = rec.iter().map(|r| r.inventions).sum();
        let trn: u32 = rec.iter().map(|r| r.transfers).sum();
        let nf: u64 = a.forgets.iter().sum::<u64>() + a.forgets_by_origin.iter().sum::<u64>() * 0;
        let nfo: u64 = a.forgets_by_origin.iter().sum();
        let d = |x: f64, n: u64| if n > 0 { x / n as f64 } else { f64::NAN };
        let _ = nf;
        writeln!(
            f,
            "{o},\"{}\",{:?},{:?},{},{inv},{trn},{},{},{:.4},{},{:.4},{},{},{:.4},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.4},{:.4},{},{},{},{},{},{},{},{},NA,{},{:.4},{:.4},{:.4}",
            item_name(o),
            form(o),
            mat(o),
            a.first_year.map(|y| format!("{y}")).unwrap_or_default(),
            a.peak_known,
            a.peak_year,
            a.peak_share,
            a.last_known,
            a.last_known as f32 / nb,
            a.last_make,
            a.last_kit,
            d(a.bs_share_sum, a.seasons_alive as u64),
            a.seasons_alive,
            a.extinctions,
            a.bs_known,
            a.bs_make,
            a.bs_kit,
            a.bs_used,
            a.gained[0],
            a.gained[1],
            a.gained[2],
            a.gained[3],
            a.taught_ok,
            a.taught_bad,
            a.forgets.map(|x| x.to_string()).join(","),
            a.forgets_band_gone,
            a.forgets_by_origin[0],
            a.forgets_by_origin[1],
            a.forgets_by_origin[2],
            d(a.forget_years_sum, nfo),
            a.forget_never_used,
            a.forget_taught_20y,
            a.inc_n,
            a.inc_none,
            d(a.q_new_sum, a.inc_n),
            d(a.q_inc_sum, a.inc_n),
            d(a.q_lower as f64, a.inc_n),
            d(a.worse_refined as f64, a.inc_n),
            d(a.gain_sum, a.inc_n),
            d(a.gain_raw_sum, a.inc_n),
            a.adopt_n,
            a.adopt_forced,
            a.adopt_food[0],
            a.adopt_food[1],
            a.adopt_true[0],
            a.adopt_true[1],
            a.adopt_own[0],
            a.adopt_own[1],
            a.held_n,
            d(a.held_worse_true as f64, a.held_n),
            d(a.held_worse_food as f64, a.held_n),
            d(a.held_gap_sum, a.held_worse_true),
        )
        .unwrap();
    }
    let mut pf = std::io::BufWriter::new(std::fs::File::create(format!("{out}/pairs-{tag}.csv")).unwrap());
    writeln!(pf, "kind,item,name,other,other_name,n,gap_mean").unwrap();
    for (&(kind, x, y), &(n, g)) in &pairs {
        writeln!(pf, "{kind},{x},\"{}\",{y},\"{}\",{n},{:.4}", item_name(x), if y == NONE { "-".to_string() } else { item_name(y) }, g / n.max(1) as f64).unwrap();
    }
    // the last partial period counts in the totals too
    tot.add(&period);
    let sh = |x: u64, n: u64| if n > 0 { 100.0 * x as f64 / n as f64 } else { f64::NAN };
    let na = tot.adopt_n - tot.adopt_forced;
    println!(
        "  skills split {tag}: adoptions {} (forced {}) | worse in true food {:.1}% (tie {:.1}%) | worse in true value {:.1}% (tie {:.1}%) | worse by own valuation {:.1}% (tie {:.1}%) | believed wrong NA | held band-years {} worse_held_share {:.1}% (food {:.1}%)",
        tot.adopt_n,
        tot.adopt_forced,
        sh(tot.adopt_food[0], na),
        sh(tot.adopt_food[1], na),
        sh(tot.adopt_true[0], na),
        sh(tot.adopt_true[1], na),
        sh(tot.adopt_own[0], na),
        sh(tot.adopt_own[1], na),
        tot.held_n,
        sh(tot.held_worse_true, tot.held_n),
        sh(tot.held_worse_food, tot.held_n)
    );
    println!("probe_diffusion {tag}: {} years, {} bands, pop {}, {} recipes, {:.0}s", (last_tick + 1) / 4, w.bands.len(), w.total_pop(), w.reg.recipes.len(), t0.elapsed().as_secs_f32());
    println!("  state_hash {:016x}", w.state_hash());
}

/// Read-only: where hides and bones come from. Per year: bands, share whose kit chain needs
/// hide, share holding hide stock (> 0.05), mean big-game harvest of hide-needing bands, share of
/// them hunting >= 8 (the break-even of `hunted*0.01` gain vs 0.02+0.06 use when needed and,
/// as for hide/bone always, not "present"), and total game left vs year 0.
/// Env as `probe_diffusion` (PROBE_SEEDS, PROBE_YEARS, PROBE_W, PROBE_H).
#[test]
#[ignore]
fn probe_hide() {
    let seeds: Vec<u64> = env("PROBE_SEEDS", "1".to_string()).split(',').filter_map(|s| s.trim().parse().ok()).collect();
    let (years, mw, mh): (u32, usize, usize) = (env("PROBE_YEARS", 500), env("PROBE_W", 256), env("PROBE_H", 160));
    for seed in seeds {
        let mut w = World::new(Config { seed, map_w: mw, map_h: mh, ..Default::default() }, Params::default());
        let g0: f32 = w.game.iter().sum();
        println!("seed {seed}: year bands need_hide has_hide mean_hunt(needers) hunt>=8 game_left");
        let hide = Mat::Hide as usize;
        for t in 0..years * 4 {
            w.step();
            if w.bands.is_empty() {
                break;
            }
            if (t + 1) % 200 != 0 {
                continue;
            }
            let nb = w.bands.len() as f32;
            let needers: Vec<_> = w.bands.iter().filter(|b| b.d.needed & (1 << hide) != 0).collect();
            let has = w.bands.iter().filter(|b| b.stock[hide] > 0.05).count() as f32;
            let nn = needers.len().max(1) as f32;
            let hunt: f32 = needers.iter().map(|b| b.d.harvest[1] + 0.3 * b.d.harvest[3]).sum::<f32>() / nn;
            let ok = needers.iter().filter(|b| b.d.harvest[1] + 0.3 * b.d.harvest[3] >= 8.0).count() as f32 / nn;
            let g: f32 = w.game.iter().sum();
            println!("  {:5.0} {:5} {:.2} {:.2} {:6.2} {:.2} {:.2}", w.tick as f32 / 4.0, w.bands.len(), needers.len() as f32 / nb, has / nb, hunt, ok, g / g0);
        }
    }
}
