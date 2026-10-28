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
use crate::band::{make_kit, refine_kit, Kit};
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
}

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
}

/// What the kit does for a band, summed (food multipliers, boat soundness, protection).
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
            let kit = make_kit(&tk.avail);
            let ki = kit_items(&kit);
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
                let q = |tk: &Toolkit, it: Item| -> f32 {
                    tk.via(it).and_then(|(i, _, _)| b.known.get(i as usize)).map(|kn: &Known| kn.quality).unwrap_or(0.0)
                };
                for &x in &ki {
                    if !tk.producible.has(x) || form(x) == Form::Cooked || form(x) == Form::Fire {
                        continue;
                    }
                    let avail2 = set_without(&tk.avail, x);
                    let kit2 = make_kit(&avail2);
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
        if year_end && (tick / 4) % every == 0 {
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
        "item,name,form,mat,first_year,inventions,transfers,peak_known,peak_year,peak_share,final_known,final_share,final_make,final_kit,mean_share_alive,seasons_alive,extinctions,bs_known,bs_make,bs_kit,bs_used,gained_invented,gained_inherited,gained_taught,gained_start,taught_ok,taught_bad,{},forgets_band_gone,forgets_invented,forgets_inherited,forgets_taught,mean_years_known_at_forget,forget_never_used,forget_taught_within_20y,inc_n,inc_none,q_new_mean,q_inc_mean,q_lower_share,worse_refined_share,gain_mean,gain_raw_mean",
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
            "{o},\"{}\",{:?},{:?},{},{inv},{trn},{},{},{:.4},{},{:.4},{},{},{:.4},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.4},{:.4}",
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
        )
        .unwrap();
    }
    println!("probe_diffusion {tag}: {} years, {} bands, pop {}, {} recipes, {:.0}s", (last_tick + 1) / 4, w.bands.len(), w.total_pop(), w.reg.recipes.len(), t0.elapsed().as_secs_f32());
    println!("  state_hash {:016x}", w.state_hash());
}
