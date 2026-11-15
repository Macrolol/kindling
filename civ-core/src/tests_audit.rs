//! Diagnostic only. Not part of the simulation and not run by `cargo test`.
//! Prints material-quality formulas and nominal slot scores so caps and ties
//! are visible. It asserts nothing and does not touch a golden hash.
//!
//! `cargo test --release -p civ-core audit_ -- --ignored --nocapture`

use crate::band::{edge_q, weapon_q, Kit, KIT_UPKEEP};
use crate::effects::{slot_score, BandView};
use crate::map::Map;
use crate::materials::*;
use crate::world::farming::{cultivated, wild_plant_cap, CROP_K};
use crate::world::{prestige_bias, want, MARRIAGE_PEACE, TRADE_MATS, World};
use std::collections::HashMap;

fn raw_cut(p: &Props) -> f32 {
    (0.6 * p.sharp + 0.4 * p.hard) / 10.0
}
fn raw_dur(p: &Props) -> f32 {
    (p.tough + 0.5 * p.malleable) / 12.0
}
fn raw_boat(p: &Props) -> f32 {
    (p.tough + p.float.max(p.seal)) / 14.0
}
fn raw_mesh(p: &Props) -> f32 {
    (p.tough + p.fibrous) / 15.0
}
fn raw_garm(m: Mat, p: &Props) -> f32 {
    if m == Mat::Flax {
        return 0.55;
    }
    (p.tough + p.fibrous) / 12.0
}

fn flag(raw: f32, clamped: f32) -> &'static str {
    if (raw - clamped).abs() > 1e-4 {
        if clamped >= 0.999 {
            "CAP"
        } else if clamped <= 0.051 {
            "FLOOR"
        } else {
            "CLAMP"
        }
    } else if (clamped - 1.0).abs() < 1e-4 {
        "AT1"
    } else {
        ""
    }
}

fn protect(gq: f32, fire: f32, kiln: f32, axe: f32) -> f32 {
    (0.35 * fire + 0.5 * gq + 0.05 * kiln + 0.05 * axe * fire).min(0.92)
}

/// Every material through the quality formulas, then nominal slot scores.
#[test]
#[ignore]
fn audit_formula_caps() {
    println!("=== material qualities (CAP/FLOOR = clamp moved the value, AT1 = landed on 1) ===");
    println!(
        "{:<14} {:>7} {:>7} {:>5} {:>7} {:>7} {:>5} {:>7} {:>6} {:>7} {:>6} {:>7} {:>6} {:>7} {:>6} {:>7} {:>6}",
        "mat", "cut", "cutR", "cflg", "dur", "durR", "dflg", "wear", "edge", "axe", "boat", "bR", "bflg", "mesh", "mflg", "garm", "gflg"
    );
    for &m in ALL_MATS.iter() {
        let p = m.props();
        let (cq, dq, bq, mq, gq) = (cut_q(m), durability(m), boat_q(m), mesh_q(m), garment_q(m));
        let (cr, dr, br, mr, gr) = (raw_cut(p), raw_dur(p), raw_boat(p), raw_mesh(p), raw_garm(m, p));
        println!(
            "{:<14} {:7.3} {:7.3} {:>5} {:7.3} {:7.3} {:>5} {:7.3} {:6.3} {:7.3} {:6.3} {:7.3} {:>6} {:7.3} {:>6} {:7.3} {:>6}",
            p.name,
            cq,
            cr,
            flag(cr, cq),
            dq,
            dr,
            flag(dr, dq),
            1.15 - dq,
            edge_q(m),
            0.5 * cq + 0.5 * dq,
            bq,
            br,
            flag(br, bq),
            mq,
            flag(mr, mq),
            gq,
            flag(gr, gq)
        );
    }

    println!("\n=== other slot scores (not clamped inside the formula) ===");
    println!(
        "{:<14} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>8}",
        "mat", "cord", "hammer", "sail", "orn", "kiln", "fuel", "mark", "plankQ", "shipQ"
    );
    for &m in ALL_MATS.iter() {
        let p = m.props();
        let sail = p.tough + if m == Mat::Fibre { 0.0 } else { 10.0 };
        let orn = p.hard + 1.0 + 10.0 * p.prized;
        let mark = durability(m) + if m == Mat::Ceramic { 1.0 } else { 0.0 };
        let plank = (boat_q(m) + 0.25).min(1.0);
        println!(
            "{:<14} {:6.2} {:6.2} {:6.2} {:6.2} {:6.2} {:6.0} {:6.3} {:8.3} {:8.3}",
            p.name,
            p.tough + 1.0,
            p.hard,
            sail,
            orn,
            p.refractory,
            p.fuel_t,
            mark,
            plank,
            plank
        );
    }

    println!("\n=== ties at 3 decimals (all materials) ===");
    // (label, values)
    let series: Vec<(&str, Vec<(String, f32)>)> = vec![
        ("cut_q", ALL_MATS.iter().map(|&m| (m.name().into(), cut_q(m))).collect()),
        ("durability", ALL_MATS.iter().map(|&m| (m.name().into(), durability(m))).collect()),
        ("wear 1.15-dur", ALL_MATS.iter().map(|&m| (m.name().into(), 1.15 - durability(m))).collect()),
        ("edge_q", ALL_MATS.iter().map(|&m| (m.name().into(), edge_q(m))).collect()),
        ("axe 0.5cut+0.5dur", ALL_MATS.iter().map(|&m| (m.name().into(), 0.5 * cut_q(m) + 0.5 * durability(m))).collect()),
        ("vessel_q", ALL_MATS.iter().map(|&m| (m.name().into(), vessel_q(m))).collect()),
        ("boat_q", ALL_MATS.iter().map(|&m| (m.name().into(), boat_q(m))).collect()),
        ("mesh_q", ALL_MATS.iter().map(|&m| (m.name().into(), mesh_q(m))).collect()),
        ("garment_q", ALL_MATS.iter().map(|&m| (m.name().into(), garment_q(m))).collect()),
        ("bow/shield tough+1", ALL_MATS.iter().map(|&m| (m.name().into(), m.props().tough + 1.0)).collect()),
        ("ornament hard+1+10prized", ALL_MATS.iter().map(|&m| (m.name().into(), m.props().hard + 1.0 + 10.0 * m.props().prized)).collect()),
    ];
    for (label, mut rows) in series {
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        println!("-- {label}");
        let mut i = 0;
        while i < rows.len() {
            let mut j = i + 1;
            while j < rows.len() && (rows[j].1 - rows[i].1).abs() < 0.001 {
                j += 1;
            }
            if j - i >= 2 {
                let names: Vec<&str> = rows[i..j].iter().map(|r| r.0.as_str()).collect();
                let bound = if (rows[i].1 - 1.0).abs() < 1e-3 {
                    " AT_CAP"
                } else if rows[i].1.abs() < 1e-3 || (rows[i].1 - 0.05).abs() < 1e-3 {
                    " AT_FLOOR"
                } else {
                    ""
                };
                println!("  tie {names:?} = {:.3}{bound}", rows[i].1);
            }
            i = j;
        }
    }

    println!("\n=== weapon_q of a lone spear / lone axe, and edge order ===");
    println!("{:<14} {:>8} {:>8} {:>8}", "mat", "spearW", "axeW", "edge");
    for &m in ALL_MATS.iter() {
        let mut ks = Kit::default();
        ks.hunt = item(Form::Spear, m);
        let mut ka = Kit::default();
        ka.axe = item(Form::Axe, m);
        println!("{:<14} {:8.3} {:8.3} {:8.3}", m.name(), weapon_q(&ks), weapon_q(&ka), edge_q(m));
    }

    println!("\n=== fire_temp(fuel, lining) ===");
    let fuels = [Mat::Wood, Mat::Fibre, Mat::Charcoal, Mat::ResinWood];
    let linings = [None, Some(Mat::Stone), Some(Mat::Clay), Some(Mat::Ceramic)];
    print!("{:<14}", "fuel");
    for lin in linings {
        print!(" {:>12}", lin.map(|m| m.name()).unwrap_or("open"));
    }
    println!();
    for f in fuels {
        print!("{:<14}", f.name());
        for lin in linings {
            print!(" {:12.0}", fire_temp(f, lin));
        }
        println!();
    }

    println!("\n=== protect = min(0.92, 0.35*fire + 0.5*gq + 0.05*kiln + 0.05*axe*fire) ===");
    println!("{:<14} {:>8} {:>10} {:>10} {:>12}", "garment", "gq", "fire", "fire+kiln", "fire+kiln+axe");
    for &m in ALL_MATS.iter() {
        let gq = garment_q(m);
        let a = protect(gq, 1.0, 0.0, 0.0);
        let b = protect(gq, 1.0, 1.0, 0.0);
        let c = protect(gq, 1.0, 1.0, 1.0);
        let mark = if (c - 0.92).abs() < 1e-4 { "CAP" } else { "" };
        println!("{:<14} {:8.3} {:10.3} {:10.3} {:8.3} {}", m.name(), gq, a, b, c, mark);
    }

    println!("\n=== sea: survival hazard (1.4 - q - 0.25*xp).max(0.3), fish radius 1+(2q).round() ===");
    println!("{:<14} {:>6} {:>8} {:>8} {:>8} {:>6} {:>8}", "mat", "boat", "haz0", "haz1", "range0", "rad", "seaR0");
    for &m in [Mat::Wood, Mat::Hide, Mat::Fibre, Mat::Flax, Mat::ResinWood, Mat::Pitch].iter() {
        let q = boat_q(m);
        let haz = |xp: f32| (1.4 - q - 0.25 * xp).max(0.3);
        let rad = 1.0 + (2.0 * q).round();
        let sea_r = World::sea_range(q, 0.0);
        println!(
            "{:<14} {:6.3} {:8.3} {:8.3} {:8.1} {:6.0} {:8.2}",
            m.name(),
            q,
            haz(0.0),
            haz(1.0),
            sea_r,
            rad,
            sea_r
        );
    }
    let q_plank = (boat_q(Mat::Wood) + 0.25).min(1.0);
    println!(
        "wood plank q {:.3} haz0 {:.3} haz1 {:.3} fish-rad {} sea_range0 {:.2}",
        q_plank,
        (1.4 - q_plank).max(0.3),
        (1.4 - q_plank - 0.25).max(0.3),
        1.0 + (2.0 * q_plank).round(),
        World::sea_range(q_plank, 0.0)
    );

    println!("\n=== nominal slot_score cur (empty kit, quality 0, user 1) depth 0 and 8 ===");
    println!("score is food minus priced upkeep labour. Higher is better. Ties listed after.");
    let view = BandView::nominal();
    let kit = Kit::default();
    let slots: &[(usize, &str, Form)] = &[
        (0, "hunt/spear", Form::Spear),
        (0, "hunt/point", Form::Point),
        (0, "hunt/edge", Form::Edge),
        (1, "cut/edge", Form::Edge),
        (4, "axe", Form::Axe),
        (12, "net", Form::Net),
        (13, "trap", Form::Trap),
        (11, "boat", Form::Boat),
        (14, "bow", Form::Bow),
        (15, "shield", Form::Shield),
    ];
    let focus = [
        Mat::Wood, Mat::Stone, Mat::Flint, Mat::Obsidian, Mat::Bone, Mat::Fibre, Mat::Hide, Mat::Copper, Mat::Tin, Mat::Bronze, Mat::Flax,
        Mat::Wool, Mat::Iron, Mat::Ceramic, Mat::Clay, Mat::Steel, Mat::Gold, Mat::Pitch,
    ];
    for &(slot, name, form) in slots {
        println!("-- {name} slot {slot}");
        let mut rows: Vec<(String, f32, f32)> = Vec::new();
        for &m in focus.iter() {
            let it = item(form, m);
            let info0 = |_: Item| (1.0f32, 0.0f32, 0u8);
            let info8 = |_: Item| (1.0f32, 0.0f32, 8u8);
            let s0 = slot_score(&view, &kit, slot, it, &info0).0;
            let s8 = slot_score(&view, &kit, slot, it, &info8).0;
            rows.push((m.name().into(), s0, s8));
            println!("  {:<14} d0 {:8.4}  d8 {:8.4}", m.name(), s0, s8);
        }
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        println!("  order d0: {}", rows.iter().map(|r| format!("{}:{:.3}", r.0, r.1)).collect::<Vec<_>>().join(" > "));
        let mut i = 0;
        while i < rows.len() {
            let mut j = i + 1;
            while j < rows.len() && (rows[j].1 - rows[i].1).abs() < 0.0005 {
                j += 1;
            }
            if j - i >= 2 {
                let names: Vec<&str> = rows[i..j].iter().map(|r| r.0.as_str()).collect();
                println!("  TIE d0 {names:?} = {:.4}", rows[i].1);
            }
            i = j;
        }
        let _ = KIT_UPKEEP;
    }
}

/// Copy of `regrow_cell` (private on `World`). Applied once, in this test only, so
/// `economy` sees start-of-season wild stocks. Does not change the library.
fn audit_regrow_cell(m: &Map, tend: &[f32], s: u32, r: f32, i: usize, pl: &mut f32, ga: &mut f32, fi: &mut f32, sm: &mut f32) {
    if !m.is_land(i) {
        let k = m.k_fish[i];
        if k > 0.0 {
            *fi += r * (0.3 * *fi * (1.0 - *fi / k) + 0.02 * k);
            *fi = (*fi).clamp(0.0, k);
        }
        return;
    }
    let t = m.temp_season(i, s);
    let g = ((t - 3.0) / 12.0).clamp(0.0, 1.0);
    let mut k = m.k_plants[i];
    if let Some(&td) = tend.get(i) {
        if td > 0.0 {
            k = wild_plant_cap(k, td);
        }
    }
    if k > 0.0 {
        *pl += r * (0.5 * g * *pl * (1.0 - *pl / k) + 0.03 * k * g) - 0.12 * (1.0 - g) * *pl;
        *pl = (*pl).clamp(0.0, k);
    }
    let k = m.k_game[i];
    if k > 0.0 {
        *ga += r * (0.12 * *ga * (1.0 - *ga / k) + 0.01 * k);
        *ga = (*ga).clamp(0.0, k);
    }
    let k = m.k_fish[i];
    if k > 0.0 {
        *fi += r * (0.3 * *fi * (1.0 - *fi / k) + 0.02 * k);
        *fi = (*fi).clamp(0.0, k);
    }
    let k = m.k_small.get(i).copied().unwrap_or(0.0);
    if k > 0.0 {
        *sm += r * (0.45 * g.max(0.2) * *sm * (1.0 - *sm / k) + 0.02 * k);
        *sm = (*sm).clamp(0.0, k);
    }
}

struct Summ {
    v: Vec<f32>,
}
impl Summ {
    fn new() -> Self {
        Summ { v: Vec::new() }
    }
    fn add(&mut self, x: f32) {
        self.v.push(x);
    }
    fn report(&mut self, name: &str) {
        let n = self.v.len();
        if n == 0 {
            println!("  {name}: n=0");
            return;
        }
        self.v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let q = |p: f64| self.v[((n - 1) as f64 * p).round() as usize];
        let sum: f64 = self.v.iter().map(|x| *x as f64).sum();
        println!(
            "  {name}: n={n} min {:.4} p10 {:.4} p50 {:.4} p90 {:.4} max {:.4} mean {:.4}",
            self.v[0],
            q(0.10),
            q(0.50),
            q(0.90),
            self.v[n - 1],
            sum / n as f64
        );
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-4
}

/// How many bands (or partner-edges, or cells) sit on each non-material bound.
/// Read-only apart from one copied wild-stock regrow on this test's `World`, so
/// `economy` plans a season against start-of-season stocks. Asserts nothing.
#[test]
#[ignore]
fn audit_bounds_in_play() {
    let path = std::env::var("DIAG_SAVE").unwrap_or_else(|_| "/workspace/step3-saves/main_seed6_y350_256x160.civsave".to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (mut w, _, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load {path}: {e}"));
    let live: Vec<usize> = w.bands.iter().enumerate().filter(|(_, b)| b.pop > 0).map(|(i, _)| i).collect();
    let nb = live.len() as u64;
    println!(
        "save {path} format v{} year {:.2} tick {} season {} bands {} live {} pop {}",
        info.format_version,
        w.year(),
        w.tick,
        w.season(),
        w.bands.len(),
        nb,
        w.total_pop()
    );

    let mut id_at: HashMap<u32, usize> = HashMap::with_capacity(w.bands.len());
    for (i, b) in w.bands.iter().enumerate() {
        id_at.insert(b.id, i);
    }
    let ncell = w.map.w * w.map.h;
    let mut occ_pop = vec![0u32; ncell];
    let mut occ_n = vec![0u32; ncell];
    for &i in &live {
        let c = w.bands[i].cell(w.map.w);
        if c < ncell {
            occ_pop[c] += w.bands[i].pop;
            occ_n[c] += 1;
        }
    }

    let mut fr = Summ::new();
    let mut fed1 = 0u64;
    let mut fed0 = 0u64;
    let mut fed_mid = 0u64;
    let mut birth_all = Summ::new();
    let mut birth_fed = Summ::new();
    let mut spare_fed = Summ::new();
    let mut sed_fed = Summ::new();
    let mut fr_hungry = Summ::new();
    let mut stress0 = 0u64;
    let mut stress1 = 0u64;
    let mut stress_raw_over = 0u64;
    let mut fr_stress = Summ::new();
    let mut stress = Summ::new();
    let mut spare = Summ::new();
    let mut spare0 = 0u64;
    let mut spare1 = 0u64;
    let mut lam_s = Summ::new();
    let mut lam_over = 0u64;
    let mut exp6 = 0u64;
    let mut exp_n = 0u64;
    let mut curiosity = Summ::new();
    let mut cur_lo = 0u64;
    let mut cur_hi = 0u64;
    let mut ambition_lo = 0u64;
    let mut ambition_hi = 0u64;
    let mut push1 = 0u64;
    let mut push_raw = Summ::new();
    let mut wq_s = Summ::new();
    let mut cas_cap = 0u64;
    let mut cas_raw = Summ::new();
    let mut boats = 0u64;
    let mut haz_floor = 0u64;
    let mut radius = [0u64; 8];
    let mut radius_flat = 0u64;
    let mut xp_floor = [0u64; 4];
    let mut xp_n = [0u64; 4];
    let mut range_s = Summ::new();
    let mut protect_cap = 0u64;
    let mut upkeep_s = Summ::new();
    let mut upkeep_floor = 0u64;
    let mut upkeep_alone = 0u64;
    let mut cell_n = Summ::new();
    let mut cell_over = 0u64;
    let mut outrigger_n = 0u64;
    let mut boat_q_s = Summ::new();
    let mut depth_f = Summ::new();
    let mut axe_n = [0u64; NMAT];
    let mut axe_d = [0u64; NMAT];
    let mut hunt_n = [0u64; NMAT];
    let mut hunt_d = [0u64; NMAT];
    let mut skill_n = 0u64;
    let mut skill_out = 0u64;
    let mut skill_top = 0u64;
    let mut qual_stuck = 0u64;
    let mut qual_n = 0u64;
    let mut use_n = 0u64;
    let mut use_top = 0u64;
    let mut use_out = 0u64;
    let mut chain_n = 0u64;
    let mut chain_floor = 0u64;
    let mut chain_top = 0u64;
    let mut tame_n = 0u64;
    let mut tame_top = 0u64;
    let mut herd_over = 0u64;
    let mut herd_s = Summ::new();
    let mut wool_n = 0u64;
    let mut wool_stock = Summ::new();
    let mut wool_herd = Summ::new();
    let mut spoil_hot = 0u64;
    let mut temp_s = Summ::new();
    let p = w.params;
    let s_done = w.tick.wrapping_sub(1) % 4;
    let teach_r = p.teach_radius as i32;

    for &i in &live {
        let b = &w.bands[i];
        let pop = b.pop as f32;
        let food_r = b.d.food_ratio;
        fr.add(food_r);
        let fed = ((food_r - 0.75) / 0.25).clamp(0.0, 1.0);
        let sed = 1.0 + 0.6 * b.settled * b.d.farm_share;
        let factor = fed * (0.8 + 0.4 * b.d.spare) * sed;
        birth_all.add(factor);
        if near(fed, 1.0) {
            fed1 += 1;
            birth_fed.add(factor);
            spare_fed.add(b.d.spare);
            sed_fed.add(sed);
        } else if fed <= 1e-6 {
            fed0 += 1;
            fr_hungry.add(food_r);
        } else {
            fed_mid += 1;
        }
        let raw_st = (1.0 - food_r) * 2.0 + 0.15 * b.d.cold;
        stress.add(b.d.stress);
        if near(b.d.stress, 0.0) {
            stress0 += 1;
        }
        if near(b.d.stress, 1.0) || b.d.stress >= 1.0 - 1e-4 {
            stress1 += 1;
            fr_stress.add(food_r);
        }
        if raw_st > 1.0 + 1e-4 {
            stress_raw_over += 1;
        }
        spare.add(b.d.spare);
        if b.d.spare <= 1e-4 {
            spare0 += 1;
        }
        if b.d.spare >= 1.0 - 1e-4 {
            spare1 += 1;
        }
        let lam = p.invent_rate * b.curiosity * (0.15 + b.d.spare + p.stress_invent * b.d.stress) * (pop / 25.0).sqrt();
        lam_s.add(lam);
        if lam >= 6.0 {
            lam_over += 1;
        }
        exp_n += 1;
        if b.d.experiments >= 6 {
            exp6 += 1;
        }
        curiosity.add(b.curiosity);
        if b.curiosity <= 0.301 {
            cur_lo += 1;
        }
        if b.curiosity >= 2.499 {
            cur_hi += 1;
        }
        if b.ambition <= -0.999 {
            ambition_lo += 1;
        }
        if b.ambition >= 0.999 {
            ambition_hi += 1;
        }
        let c = b.cell(w.map.w);
        let others = occ_pop.get(c).copied().unwrap_or(0).saturating_sub(b.pop) as f32;
        let crowd = others / (pop + 10.0);
        let praw = b.d.stress + 0.5 * crowd + b.pressed;
        push_raw.add(praw);
        if praw >= 1.0 - 1e-4 {
            push1 += 1;
        }
        let wq = weapon_q(&b.d.kit);
        wq_s.add(wq);
        let raw_loss = p.lethality * 0.16 * (0.6 + wq);
        cas_raw.add(raw_loss);
        if raw_loss > 0.45 {
            cas_cap += 1;
        }
        let ci = c;
        if ci < w.map.temp.len() {
            let t = w.map.temp_season(ci, s_done);
            temp_s.add(t);
            if t >= 25.0 {
                spoil_hot += 1;
            }
        }
        if b.d.kit.boat_q > 0.0 {
            boats += 1;
            boat_q_s.add(b.d.kit.boat_q);
            if b.d.kit.outrigger != NONE {
                outrigger_n += 1;
            }
            let q = World::boat_safety(&b.d.kit);
            let xp = b.seacraft;
            let raw_h = 1.4 - q - 0.25 * xp;
            if raw_h <= 0.3 + 1e-4 {
                haz_floor += 1;
            }
            let bin = if xp < 0.25 {
                0
            } else if xp < 0.5 {
                1
            } else if xp < 0.75 {
                2
            } else {
                3
            };
            xp_n[bin] += 1;
            if raw_h <= 0.3 + 1e-4 {
                xp_floor[bin] += 1;
            }
            let rad = 1 + (2.0 * b.d.kit.boat_q).round() as i32;
            let ri = rad.clamp(0, 7) as usize;
            radius[ri] += 1;
            if b.d.kit.boat_q >= 0.75 {
                radius_flat += 1;
            }
            range_s.add(World::sea_range(b.d.kit.boat_q, xp));
        }
        if b.d.kit.protect >= 0.919 {
            protect_cap += 1;
        }
        upkeep_s.add(b.d.upkeep);
        let farm_l = 0.12 * b.sow + 0.08 * b.herd.min(1.0);
        if b.d.upkeep + farm_l > 0.55 {
            upkeep_floor += 1;
        }
        if b.d.upkeep > 0.55 {
            upkeep_alone += 1;
        }
        let share = occ_n.get(c).copied().unwrap_or(1).saturating_sub(1);
        cell_n.add(share as f32);
        if share as f32 * p.teach_rate > 4.0 {
            cell_over += 1;
        }
        for j in 0..12 {
            let it = b.d.kit.slot_item(j);
            if it == NONE {
                continue;
            }
            let d = b.d.tk.via(it).map(|v| v.2).unwrap_or(0);
            depth_f.add(1.0 + 0.3 * d as f32);
            let cm = b.d.tk.chain_m(it);
            chain_n += 1;
            if cm <= 0.051 {
                chain_floor += 1;
            }
            if cm >= 0.999 {
                chain_top += 1;
            }
        }
        if b.d.kit.axe != NONE {
            let mi = mat(b.d.kit.axe) as usize;
            if mi < NMAT {
                axe_n[mi] += 1;
                axe_d[mi] += b.d.tk.via(b.d.kit.axe).map(|v| v.2 as u64).unwrap_or(0);
            }
        }
        if b.d.kit.hunt != NONE {
            let mi = mat(b.d.kit.hunt) as usize;
            if mi < NMAT {
                hunt_n[mi] += 1;
                hunt_d[mi] += b.d.tk.via(b.d.kit.hunt).map(|v| v.2 as u64).unwrap_or(0);
            }
        }
        for k in &b.known {
            skill_n += 1;
            if k.skill < -1e-4 || k.skill > 1.0 + 1e-4 {
                skill_out += 1;
            }
            if k.skill >= 0.99 {
                skill_top += 1;
            }
            if k.quality > 0.0 {
                qual_n += 1;
                if (k.skill - k.quality).abs() <= 0.01 || k.quality >= k.skill - 1e-4 {
                    qual_stuck += 1;
                }
            }
        }
        for &u in &b.use_skill {
            use_n += 1;
            if u < -1e-4 || u > 1.0 + 1e-4 {
                use_out += 1;
            }
            if u >= 0.99 {
                use_top += 1;
            }
        }
        if b.herd > 0.0 {
            herd_s.add(b.herd);
            tame_n += 1;
            if b.tame >= 0.99 {
                tame_top += 1;
            }
            if b.herd > 1.0 {
                herd_over += 1;
            }
            if b.tame > 0.4 && ci < w.map.temp.len() && w.map.temp_winter(ci) < 10.0 {
                wool_n += 1;
                wool_herd.add(b.herd);
                wool_stock.add(b.stock.get(Mat::Wool as usize).copied().unwrap_or(0.0));
            }
        }
    }

    println!("--- births fed=((food_ratio-0.75)/0.25).clamp(0,1) ---");
    println!(
        "  live {nb}  fed=1 {:.2}%  fed=0 (food_ratio<=0.75) {:.2}%  open {:.2}%",
        100.0 * fed1 as f64 / nb as f64,
        100.0 * fed0 as f64 / nb as f64,
        100.0 * fed_mid as f64 / nb as f64
    );
    fr.report("food_ratio saved");
    birth_all.report("birth factor all");
    birth_fed.report("birth factor where fed=1");
    spare_fed.report("spare where fed=1");
    sed_fed.report("sed 1+0.6*settled*farm_share where fed=1");
    fr_hungry.report("food_ratio where fed=0");
    println!("--- stress ((1-food_ratio)*2+0.15*cold).clamp(0,1) ---");
    println!(
        "  stress=0 {:.2}%  stress=1 {:.2}%  raw>1 {:.2}%",
        100.0 * stress0 as f64 / nb as f64,
        100.0 * stress1 as f64 / nb as f64,
        100.0 * stress_raw_over as f64 / nb as f64
    );
    stress.report("stress");
    fr_stress.report("food_ratio where stress=1");
    spare.report("spare");
    println!("  spare=0 {:.2}%  spare=1 {:.2}%", 100.0 * spare0 as f64 / nb as f64, 100.0 * spare1 as f64 / nb as f64);
    println!("--- tinkering poisson(lam).min(6) ---");
    println!(
        "  experiments>=6 {:.2}%  lam>=6 {:.2}%",
        100.0 * exp6 as f64 / exp_n.max(1) as f64,
        100.0 * lam_over as f64 / nb as f64
    );
    lam_s.report("lam");
    curiosity.report("curiosity");
    println!(
        "  curiosity<=0.30 {:.2}%  curiosity>=2.50 {:.2}%  ambition<=-1 {:.2}%  ambition>=1 {:.2}%",
        100.0 * cur_lo as f64 / nb as f64,
        100.0 * cur_hi as f64 / nb as f64,
        100.0 * ambition_lo as f64 / nb as f64,
        100.0 * ambition_hi as f64 / nb as f64
    );
    println!("--- migration push (stress+0.5*crowd+pressed) clamp 0..1 ---");
    println!("  raw>=1 {:.2}%", 100.0 * push1 as f64 / nb as f64);
    push_raw.report("push raw");
    println!("--- conflict casualty cap 0.45 (lethality {:.2}) ---", p.lethality);
    wq_s.report("weapon_q");
    cas_raw.report("loser loss at rng high 0.16, before min(0.45)");
    println!("  bands whose high-end loss exceeds 0.45: {cas_cap}");
    println!("--- sea hazard floor 0.3 and fish-radius round ---");
    println!(
        "  boats {boats}  outrigger {:.2}%  hazard raw<=0.3 {:.2}%  boat_q>=0.75 (radius bin of a plank) {:.2}%",
        100.0 * outrigger_n as f64 / boats.max(1) as f64,
        100.0 * haz_floor as f64 / boats.max(1) as f64,
        100.0 * radius_flat as f64 / boats.max(1) as f64
    );
    boat_q_s.report("boat_q");
    println!("  radius histogram 1+round(2*boat_q): {radius:?}");
    println!("  floor by seacraft bin [0,.25) [.25,.5) [.5,.75) [.75,1]: num {xp_floor:?} den {xp_n:?}");
    range_s.report("sea_range");
    println!("--- skills, chain, upkeep depth, protection ---");
    println!(
        "  recipes {skill_n} outside 0..1 {skill_out} skill>=0.99 {:.2}%  quality within 0.01 of skill {:.2}% of {qual_n}",
        100.0 * skill_top as f64 / skill_n.max(1) as f64,
        100.0 * qual_stuck as f64 / qual_n.max(1) as f64
    );
    println!(
        "  use_skill n {use_n} outside 0..1 {use_out} >=0.99 {:.2}%",
        100.0 * use_top as f64 / use_n.max(1) as f64
    );
    println!(
        "  payoff chain_m n {chain_n} <=0.051 {:.2}%  >=0.999 {:.2}%",
        100.0 * chain_floor as f64 / chain_n.max(1) as f64,
        100.0 * chain_top as f64 / chain_n.max(1) as f64
    );
    depth_f.report("1+0.3*depth on payoff tools");
    upkeep_s.report("saved tool upkeep fraction");
    println!(
        "  upkeep>0.55 {:.2}%  upkeep+farm_labour_upper>0.55 {:.2}%  protect>=0.919 {:.2}%",
        100.0 * upkeep_alone as f64 / nb as f64,
        100.0 * upkeep_floor as f64 / nb as f64,
        100.0 * protect_cap as f64 / nb as f64
    );
    cell_n.report("other bands on the same cell");
    println!(
        "  same-cell others * teach_rate > 4: {:.2}%",
        100.0 * cell_over as f64 / nb as f64
    );
    println!("  axe material mean depth (n):");
    for mi in 0..NMAT {
        if axe_n[mi] > 0 {
            println!("    {:<12} n {:5} mean depth {:.2}", ALL_MATS[mi].name(), axe_n[mi], axe_d[mi] as f64 / axe_n[mi] as f64);
        }
    }
    println!("  hunt material mean depth (n):");
    for mi in 0..NMAT {
        if hunt_n[mi] > 20 {
            println!("    {:<12} n {:5} mean depth {:.2}", ALL_MATS[mi].name(), hunt_n[mi], hunt_d[mi] as f64 / hunt_n[mi] as f64);
        }
    }
    println!("--- herds, wool refill, spoil temperature ---");
    println!(
        "  herders {tame_n} herd>1 {:.2}%  tame>=0.99 {:.2}%  cool-land wool bands {wool_n}",
        100.0 * herd_over as f64 / tame_n.max(1) as f64,
        100.0 * tame_top as f64 / tame_n.max(1) as f64
    );
    herd_s.report("herd per person");
    wool_herd.report("herd where wool is refilled");
    wool_stock.report("wool stock where refilled");
    println!("  seasonal temp>=25 (spoil base pinned) {:.2}%", 100.0 * spoil_hot as f64 / nb as f64);
    temp_s.report("temp of the season just run");

    // Partner edges: prestige bias, learn ceiling, tension, trade lot.
    let mut edges = 0u64;
    let mut bias_hi = 0u64;
    let mut bias_lo = 0u64;
    let mut learn_offers = 0u64;
    let mut learn_capped = 0u64;
    let mut show_capped = 0u64;
    let mut bias_s = Summ::new();
    let mut ten_n = 0u64;
    let mut ten_cap = 0u64;
    let mut ten_s = Summ::new();
    let tick = w.tick;
    for &i in &live {
        let pids: Vec<u32> = w.bands[i].partners.iter().map(|pt| pt.id).collect();
        for pid in pids {
            let Some(&j) = id_at.get(&pid) else { continue };
            if w.bands[j].pop == 0 {
                continue;
            }
            edges += 1;
            let src = &w.bands[i];
            let dst = &w.bands[j];
            let raw_b = 1.0 + 0.5 * (src.prestige - dst.prestige);
            let bias = prestige_bias(src, dst);
            bias_s.add(bias);
            if raw_b >= 2.0 - 1e-4 {
                bias_hi += 1;
            }
            if raw_b <= 0.5 + 1e-4 {
                bias_lo += 1;
            }
            if 0.3 * bias >= 0.9 - 1e-4 {
                show_capped += 1;
            }
            for kn in &src.known {
                if kn.skill < 0.25 {
                    continue;
                }
                learn_offers += 1;
                if kn.skill * 0.8 * bias >= 0.95 - 1e-4 {
                    learn_capped += 1;
                }
            }
            let d = (src.x as i32 - dst.x as i32).abs().max((src.y as i32 - dst.y as i32).abs());
            if d <= 2 {
                ten_n += 1;
                let overlap = match d {
                    0 => 1.0,
                    1 => 0.6,
                    _ => 0.3,
                };
                let need = src.d.need.max(1.0);
                let hunger = (1.0 - src.d.food_ratio).clamp(0.0, 1.0) + 0.4 * (1.0 - src.store / need).clamp(0.0, 1.0);
                let cell = src.cell(w.map.w);
                let others = occ_pop.get(cell).copied().unwrap_or(0).saturating_sub(src.pop) as f32;
                let crowd = (others / (src.pop as f32 + 10.0)).min(2.0);
                let lure = (dst.store / dst.d.need.max(1.0)).clamp(0.0, 2.0) * hunger;
                let mut goods = 0.0f32;
                for &m in TRADE_MATS.iter() {
                    let mi = m as usize;
                    if src.d.needed & (1 << mi) != 0 && src.stock.get(mi).copied().unwrap_or(0.0) < 0.2 && dst.stock.get(mi).copied().unwrap_or(0.0) > 0.5 {
                        goods = 0.3;
                    }
                }
                let g = src.grudge_vs(dst);
                let ties = src.partner(dst.id).map(|p| p.strength).unwrap_or(0.0);
                let kin = if src.close_kin(dst) {
                    0.2
                } else {
                    1.0 - 0.3 * (2.0 * src.kinship(dst, tick)).min(1.0)
                };
                let base = overlap * (hunger * (1.0 + 0.5 * crowd) + 0.5 * lure + 0.5 * goods) + 0.6 * g;
                let wed = 1.0 - MARRIAGE_PEACE * src.married_to(dst.id).min(1.0);
                let t = base * (1.0 - 0.8 * ties.min(1.0)) * kin * wed;
                ten_s.add(t);
                if 0.05 * p.aggression * t >= 0.5 - 1e-4 {
                    ten_cap += 1;
                }
            }
        }
    }
    println!("--- prestige bias and learn ceiling, on stored partner edges ---");
    println!(
        "  edges {edges}  raw bias>=2 {:.2}%  raw bias<=0.5 {:.2}%  show_quality 0.3*bias>=0.9 {:.2}%",
        100.0 * bias_hi as f64 / edges.max(1) as f64,
        100.0 * bias_lo as f64 / edges.max(1) as f64,
        100.0 * show_capped as f64 / edges.max(1) as f64
    );
    println!(
        "  teacher recipes skill>=0.25: {learn_offers}  would hit learn min(0.95) at this bias {:.2}%",
        100.0 * learn_capped as f64 / learn_offers.max(1) as f64
    );
    bias_s.report("prestige_bias");
    println!("--- tension cap, partner edges within 2 cells (chance min 0.5 at t>=10 if aggression=1) ---");
    println!("  n {ten_n}  t>=10 {:.2}%", 100.0 * ten_cap as f64 / ten_n.max(1) as f64);
    ten_s.report("tension");

    // Trade stocks and the per-visit lot cap.
    println!("--- stock cap 1.5, want zero at stock>=1.3, visit lot (stock-0.2).min(0.3*frame) ---");
    for &m in [Mat::Copper, Mat::Tin, Mat::Bronze, Mat::Iron, Mat::Steel, Mat::Gold, Mat::Flint, Mat::Obsidian, Mat::Hide, Mat::Fibre, Mat::Wool, Mat::Grain, Mat::Flax, Mat::Salt].iter() {
        let mi = m as usize;
        let mut hold = 0u64;
        let mut at_cap = 0u64;
        let mut worth0 = 0u64;
        let mut lot_caps = 0u64;
        let mut lot_n = 0u64;
        let mut st = Summ::new();
        for &i in &live {
            let b = &w.bands[i];
            let s = b.stock.get(mi).copied().unwrap_or(0.0);
            if s <= 0.0 {
                continue;
            }
            hold += 1;
            st.add(s);
            if s >= 1.49 {
                at_cap += 1;
            }
            if want(b, mi).1 <= 1e-4 && s >= 1.3 - 1e-3 {
                worth0 += 1;
            }
            if s >= 0.3 {
                lot_n += 1;
                let fr = if b.d.kit.frame != NONE { 1.0 + vessel_q(mat(b.d.kit.frame)) } else { 1.0 };
                if s - 0.2 > 0.3 * fr + 1e-4 {
                    lot_caps += 1;
                }
            }
        }
        if hold == 0 {
            continue;
        }
        println!(
            "  {:<10} holders {:5} at>=1.49 {:.1}%  worth-already-0 {:.1}%  lot-capped {:.1}% of {:5}",
            m.name(),
            hold,
            100.0 * at_cap as f64 / hold as f64,
            100.0 * worth0 as f64 / hold as f64,
            100.0 * lot_caps as f64 / lot_n.max(1) as f64,
            lot_n
        );
        st.report(&format!("{} stock", m.name()));
    }

    // Neighbours inside the teach radius, versus MAX_MEETINGS = 4.
    let mut neigh = Summ::new();
    let mut neigh_over4 = 0u64;
    let mut neigh_over13 = 0u64;
    let ww = w.map.w as i32;
    let hh = w.map.h as i32;
    let mut at_cell: Vec<u32> = vec![0; ncell];
    for &i in &live {
        let c = w.bands[i].cell(w.map.w);
        if c < ncell {
            at_cell[c] += 1;
        }
    }
    for &i in &live {
        let b = &w.bands[i];
        let (x, y) = (b.x as i32, b.y as i32);
        let mut nbor = 0u32;
        for dy in -teach_r..=teach_r {
            for dx in -teach_r..=teach_r {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= ww || ny >= hh {
                    continue;
                }
                nbor += at_cell[(ny as usize) * w.map.w + nx as usize];
            }
        }
        nbor = nbor.saturating_sub(1);
        neigh.add(nbor as f32);
        if nbor > 4 {
            neigh_over4 += 1;
        }
        if nbor > 13 {
            neigh_over13 += 1;
        }
    }
    println!("--- meetings cap 4, neighbours inside teach_radius {teach_r} ---");
    println!(
        "  bands with >4 neighbours {:.2}%  with >13 (0.3*n > 4) {:.2}%",
        100.0 * neigh_over4 as f64 / nb as f64,
        100.0 * neigh_over13 as f64 / nb as f64
    );
    neigh.report("neighbours in teach radius");

    // Climate clamps on farmer cells and on tended field cells.
    let mut farm_b = 0u64;
    let mut water1 = 0u64;
    let mut water0 = 0u64;
    let mut warm1 = 0u64;
    let mut warm0 = 0u64;
    let mut river_n = 0u64;
    let mut season1 = 0u64;
    let mut rain_s = Summ::new();
    for &i in &live {
        let b = &w.bands[i];
        if b.sow <= 0.2 {
            continue;
        }
        let ci = b.cell(w.map.w);
        if ci >= w.map.rain.len() {
            continue;
        }
        farm_b += 1;
        rain_s.add(w.map.rain[ci]);
        if w.map.river[ci] {
            river_n += 1;
        } else {
            let water = (w.map.rain[ci] - 0.15) / 0.35;
            if water >= 1.0 {
                water1 += 1;
            }
            if water <= 0.0 {
                water0 += 1;
            }
        }
        let warm = (w.map.temp[ci] - 4.0) / 8.0;
        if warm >= 1.0 {
            warm1 += 1;
        }
        if warm <= 0.0 {
            warm0 += 1;
        }
        if w.map.tamp[ci] / 12.0 >= 1.0 {
            season1 += 1;
        }
    }
    let mut fields = 0u64;
    let mut field_full = 0u64;
    let mut field_water1 = 0u64;
    let mut kc_s = Summ::new();
    let fy = CROP_K * w.params.farm_yield;
    for i in 0..ncell.min(w.tend.len()) {
        let cult = cultivated(w.tend[i]);
        if cult <= 0.0 {
            continue;
        }
        fields += 1;
        let p0 = w.crop_pot.get(i).copied().unwrap_or(0.0);
        let o = w.opened.get(i).copied().unwrap_or(0.0);
        let q = w.open_pot.get(i).copied().unwrap_or(p0);
        let pot = if o > 0.0 { p0 + o * (q - p0) } else { p0 };
        let dom = w.dom.get(i).copied().unwrap_or(0.0);
        let fert = w.fert.get(i).copied().unwrap_or(1.0);
        let kc = fy * pot * cult * (0.45 + 0.55 * dom) * fert;
        let crop = w.crop.get(i).copied().unwrap_or(0.0);
        if kc > 1.0 {
            kc_s.add((crop / kc).min(2.0));
            if crop >= kc - 1.0 {
                field_full += 1;
            }
        }
        if !w.map.river.get(i).copied().unwrap_or(false) {
            let water = (w.map.rain[i] - 0.15) / 0.35;
            if water >= 1.0 {
                field_water1 += 1;
            }
        }
    }
    println!("--- farming climate clamps, bands with sow>0.2 ({farm_b}), field cells {fields} ---");
    println!(
        "  farmer cells: river {:.1}%  non-river water>=1 {:.1}%  water<=0 {:.1}%  warm>=1 {:.1}%  warm<=0 {:.1}%  tamp>=12 {:.1}%",
        100.0 * river_n as f64 / farm_b.max(1) as f64,
        100.0 * water1 as f64 / farm_b.max(1) as f64,
        100.0 * water0 as f64 / farm_b.max(1) as f64,
        100.0 * warm1 as f64 / farm_b.max(1) as f64,
        100.0 * warm0 as f64 / farm_b.max(1) as f64,
        100.0 * season1 as f64 / farm_b.max(1) as f64
    );
    rain_s.report("rain at farmer camps");
    println!(
        "  field cells non-river water>=1 {:.1}%  crop at capacity {:.1}%",
        100.0 * field_water1 as f64 / fields.max(1) as f64,
        100.0 * field_full as f64 / fields.max(1) as f64
    );
    kc_s.report("crop/kc on fields");

    // Carrying capacity of wild stocks, as saved (post-harvest) and the share at the ceiling.
    let mut pk = Summ::new();
    let mut pk_hi = 0u64;
    let mut pk_lo = 0u64;
    let mut pk_n = 0u64;
    let mut seen_cell = vec![0u8; ncell];
    for &i in &live {
        let c = w.bands[i].cell(w.map.w);
        if c >= ncell || seen_cell[c] == 1 {
            continue;
        }
        seen_cell[c] = 1;
        let k = w.map.k_plants.get(c).copied().unwrap_or(0.0);
        if k <= 1.0 {
            continue;
        }
        let v = w.plants.get(c).copied().unwrap_or(0.0) / k;
        pk_n += 1;
        pk.add(v);
        if v >= 0.95 {
            pk_hi += 1;
        }
        if v <= 0.05 {
            pk_lo += 1;
        }
    }
    println!("--- wild plant stock/k on occupied cells, as saved (after harvest, before regrow) ---");
    println!("  n {pk_n}  >=0.95 {:.2}%  <=0.05 {:.2}%", 100.0 * pk_hi as f64 / pk_n.max(1) as f64, 100.0 * pk_lo as f64 / pk_n.max(1) as f64);
    pk.report("plants/k");

    // One copied regrow, then a read-only economy plan. This is the start of the next season.
    let s_next = w.season();
    let regen = w.params.regen;
    let n = w.map.w * w.map.h;
    if w.plants.len() == n && w.game.len() == n && w.fish.len() == n {
        let tend = w.tend.clone();
        for i in 0..n {
            let mut pl = w.plants[i];
            let mut ga = w.game[i];
            let mut fi = w.fish[i];
            let mut smv = if i < w.small.len() { w.small[i] } else { 0.0 };
            audit_regrow_cell(&w.map, &tend, s_next, regen, i, &mut pl, &mut ga, &mut fi, &mut smv);
            w.plants[i] = pl;
            w.game[i] = ga;
            w.fish[i] = fi;
            if i < w.small.len() {
                w.small[i] = smv;
            }
        }
    }
    println!("--- harvest phi = h/ys after one copied wild regrow, economy season {s_next} (crops not regrown) ---");
    let names = ["plants", "game", "fish", "small", "farm"];
    let mut phi_n = [0u64; 5];
    let mut phi_80 = [0u64; 5];
    let mut phi_95 = [0u64; 5];
    let mut phi_sum = [0f64; 5];
    let mut store_n = 0u64;
    let mut store_cap_n = 0u64;
    let mut fed_now = 0u64;
    let mut cap_s = Summ::new();
    for &i in &live {
        let b = &w.bands[i];
        let ec = w.economy(b, s_next);
        store_n += 1;
        if ec.food_ratio >= 1.0 - 1e-4 {
            fed_now += 1;
        }
        if ec.fin.store_cap > 1.0 && ec.food + 1e-2 >= ec.need && ec.store_after >= ec.fin.store_cap - 0.05 {
            store_cap_n += 1;
            cap_s.add(ec.fin.store_cap / b.pop.max(1) as f32);
        } else if ec.fin.store_cap > 1.0 {
            cap_s.add(ec.fin.store_cap / b.pop.max(1) as f32);
        }
        for r in 0..5 {
            if ec.ys[r] > 0.01 {
                let phi = (ec.h[r] / ec.ys[r]).min(1.0);
                phi_n[r] += 1;
                phi_sum[r] += phi as f64;
                if phi >= 0.80 {
                    phi_80[r] += 1;
                }
                if phi >= 0.95 {
                    phi_95[r] += 1;
                }
            }
        }
    }
    for r in 0..5 {
        println!(
            "  {:<8} bands-with-stock {:5}  mean phi {:.3}  phi>=0.80 {:.1}%  phi>=0.95 {:.1}%",
            names[r],
            phi_n[r],
            if phi_n[r] == 0 { 0.0 } else { phi_sum[r] / phi_n[r] as f64 },
            100.0 * phi_80[r] as f64 / phi_n[r].max(1) as f64,
            100.0 * phi_95[r] as f64 / phi_n[r].max(1) as f64
        );
    }
    println!(
        "  economy food_ratio==1 {:.2}% of live (saved was {:.2}%)  surplus sitting on store cap {:.2}%",
        100.0 * fed_now as f64 / store_n.max(1) as f64,
        100.0 * fed1 as f64 / nb as f64,
        100.0 * store_cap_n as f64 / store_n.max(1) as f64
    );
    cap_s.report("store_cap per person");
    println!("=== SUMMARY (fractions of live bands unless noted) ===");
    println!("fed=1 {:.2}%  fed=0 {:.2}%  open {:.2}%", 100.0 * fed1 as f64 / nb as f64, 100.0 * fed0 as f64 / nb as f64, 100.0 * fed_mid as f64 / nb as f64);
    println!("stress=1 {:.2}%  raw_stress>1 {:.2}%  spare=0 {:.2}%  spare=1 {:.2}%", 100.0 * stress1 as f64 / nb as f64, 100.0 * stress_raw_over as f64 / nb as f64, 100.0 * spare0 as f64 / nb as f64, 100.0 * spare1 as f64 / nb as f64);
    println!("lam>=6 {:.2}%  experiments>=6 {:.2}%", 100.0 * lam_over as f64 / nb as f64, 100.0 * exp6 as f64 / exp_n.max(1) as f64);
    println!("push raw>=1 {:.2}%  casualty high-end over 0.45: {cas_cap}", 100.0 * push1 as f64 / nb as f64);
    println!(
        "boats {boats} outrigger {:.2}% hazard floor {:.2}%  q>=0.75 {:.2}%  upkeep>0.55 {:.2}%",
        100.0 * outrigger_n as f64 / boats.max(1) as f64,
        100.0 * haz_floor as f64 / boats.max(1) as f64,
        100.0 * radius_flat as f64 / boats.max(1) as f64,
        100.0 * upkeep_alone as f64 / nb as f64
    );
    println!("same-cell others * teach_rate > 4: {:.2}%", 100.0 * cell_over as f64 / nb as f64);
    println!("partner edges {edges} bias>=2 {:.2}%  bias<=0.5 {:.2}%  learn offers capped {:.2}%  show cap {:.2}%", 100.0 * bias_hi as f64 / edges.max(1) as f64, 100.0 * bias_lo as f64 / edges.max(1) as f64, 100.0 * learn_capped as f64 / learn_offers.max(1) as f64, 100.0 * show_capped as f64 / edges.max(1) as f64);
    println!("tension n {ten_n} t>=10 {:.2}%", 100.0 * ten_cap as f64 / ten_n.max(1) as f64);
    println!("neighbours >4 {:.2}%  >13 {:.2}%", 100.0 * neigh_over4 as f64 / nb as f64, 100.0 * neigh_over13 as f64 / nb as f64);
    println!("herd>1 {:.2}% of herders  wool bands {wool_n}  spoil t>=25 {:.2}%", 100.0 * herd_over as f64 / tame_n.max(1) as f64, 100.0 * spoil_hot as f64 / nb as f64);
    println!("skill>=0.99 {:.2}%  outside 0..1 {skill_out}  protect cap {:.2}%  effort floor {:.2}%", 100.0 * skill_top as f64 / skill_n.max(1) as f64, 100.0 * protect_cap as f64 / nb as f64, 100.0 * upkeep_floor as f64 / nb as f64);
    println!("farmer camps {farm_b} field cells {fields} crop at kc {:.2}%", 100.0 * field_full as f64 / fields.max(1) as f64);
    println!("plants/k >=0.95 {:.2}%  <=0.05 {:.2}% (saved, occupied)", 100.0 * pk_hi as f64 / pk_n.max(1) as f64, 100.0 * pk_lo as f64 / pk_n.max(1) as f64);
    println!("next-season food_ratio==1 {:.2}%  on store cap {:.2}%", 100.0 * fed_now as f64 / store_n.max(1) as f64, 100.0 * store_cap_n as f64 / store_n.max(1) as f64);
    for r in 0..5 {
        println!(
            "phi {} mean {:.3} >=0.80 {:.1}% >=0.95 {:.1}% (n {})",
            names[r],
            if phi_n[r] == 0 { 0.0 } else { phi_sum[r] / phi_n[r] as f64 },
            100.0 * phi_80[r] as f64 / phi_n[r].max(1) as f64,
            100.0 * phi_95[r] as f64 / phi_n[r].max(1) as f64,
            phi_n[r]
        );
    }
    println!("--- end ---");
}
