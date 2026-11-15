//! Diagnostic only. Not part of the simulation and not run by `cargo test`.
//! Prints material-quality formulas and nominal slot scores so caps and ties
//! are visible. It asserts nothing and does not touch a golden hash.
//!
//! `cargo test --release -p civ-core audit_ -- --ignored --nocapture`

use crate::band::{edge_q, weapon_q, Kit, KIT_UPKEEP};
use crate::effects::{slot_score, BandView};
use crate::materials::*;
use crate::world::World;

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
