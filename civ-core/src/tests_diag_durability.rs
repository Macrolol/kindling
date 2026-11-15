//! Ignored diagnostics for the durability proposal. Not part of the sim.
//!
//! `durability_v2_orders_edges_and_v1_formula_stays` runs in the normal suite.
//! The `diag_durability_*` tests are `#[ignore]`. They print; they do not
//! assert a ranking and they do not change a hash.
//!
//! `DIAG_SAVE` is the snapshot (the same one as the steel-axe diagnostic).

use crate::band::KIT_UPKEEP;
use crate::effects::{
    fills_payoff, recipe_info, slot_score_m, tool_effects_m, user_factor, BandView, LABOUR_SHARE, PAYOFF_SLOTS, WORK_SHARE,
};
use crate::materials::*;
use crate::tests_diag::DIAG_SAVE_DEFAULT;
use crate::World;
use std::collections::BTreeMap;

const FOOD_EPS: f32 = 1e-6;
const SLOT_NAMES: [&str; 12] = [
    "hunt", "cut", "point", "axe", "boat", "net", "trap", "bow", "shield", "outrigger", "plank", "ship",
];

fn bits(x: f32) -> u32 {
    x.to_bits()
}

fn props_bits(xs: &[f32]) -> Vec<u32> {
    xs.iter().copied().map(bits).collect()
}

/// Two materials may share a life only when the properties that formula reads are the same.
fn assert_same_inputs(label: &str, life: impl Fn(Mat) -> f32, key: impl Fn(Mat) -> Vec<u32>) {
    let mut rows: Vec<(u32, Vec<u32>, Mat)> = Vec::new();
    for &m in &ALL_MATS {
        let b = bits(life(m));
        let k = key(m);
        for (ob, ok, other) in &rows {
            if *ob == b && ok != &k {
                panic!("{label}: {} and {} share a life ({}) but not the inputs", m.name(), other.name(), life(m));
            }
        }
        rows.push((b, k, m));
    }
}

fn edge_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.hard, p.sharp, p.malleable, p.tough, p.fracture])
}

fn impact_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.tough, p.fracture, p.hard, p.malleable])
}

fn vessel_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.fibrous, p.tough, p.fracture, p.hard])
}

fn cord_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.tough, p.fibrous])
}

fn hull_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.tough, p.hard])
}

fn thermal_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    if p.refractory > 0.0 {
        props_bits(&[1.0, p.refractory, p.tough])
    } else if p.melt > 0.0 {
        props_bits(&[2.0, p.melt])
    } else {
        props_bits(&[3.0, p.tough])
    }
}

fn record_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.hard, p.tough])
}

fn structure_key(m: Mat) -> Vec<u32> {
    let p = m.props();
    props_bits(&[p.tough, p.fracture, p.hard])
}

fn stock_key(m: Mat) -> Vec<u32> {
    props_bits(&[m.props().tough])
}

fn net_edge_wear(m: Mat) -> f32 {
    let life = class_durability(WearClass::Edge, m);
    (1.15 - life) * repair_factor(WearClass::Edge, m)
}

/// The proposal's edge order, and that the default build still uses the old formula.
#[test]
fn durability_v2_orders_edges_and_v1_formula_stays() {
    let e = |m: Mat| class_durability(WearClass::Edge, m);
    assert!(e(Mat::Obsidian) < e(Mat::Flint), "obsidian chips before flint");
    assert!(e(Mat::Flint) < e(Mat::Copper), "flint chips before copper dulls");
    assert!(e(Mat::Copper) < e(Mat::Bronze), "copper dulls before bronze");
    assert!(e(Mat::Bronze) < e(Mat::Iron), "iron a bit ahead of bronze");
    assert!(e(Mat::Iron) < e(Mat::Steel), "steel keeps an edge");
    assert!(e(Mat::Steel) > e(Mat::Iron) * 1.15, "steel is clearly ahead of iron, not a rounding");
    assert!(e(Mat::Copper) < e(Mat::Stone) && e(Mat::Stone) < e(Mat::Bronze), "a ground stone edge outlasts soft copper and not bronze");
    // The bug the proposal is answering: the old function still caps copper above steel.
    assert!(durability(Mat::Copper) > durability(Mat::Steel));
    assert!((durability(Mat::Copper) - 1.0).abs() < 1e-6);
    assert!((durability(Mat::Bronze) - 1.0).abs() < 1e-6);
    assert!((durability(Mat::Iron) - 1.0).abs() < 1e-6);
    let w = net_edge_wear;
    assert!(w(Mat::Steel) < w(Mat::Iron) && w(Mat::Iron) < w(Mat::Bronze) && w(Mat::Bronze) < w(Mat::Copper));
    assert!(w(Mat::Copper) < w(Mat::Flint) && w(Mat::Flint) < w(Mat::Obsidian));
    assert!(repair_factor(WearClass::Edge, Mat::Copper) < repair_factor(WearClass::Edge, Mat::Steel));
    assert!(repair_factor(WearClass::Edge, Mat::Steel) < repair_factor(WearClass::Edge, Mat::Flint));
    for &m in &ALL_MATS {
        for class in [WearClass::Edge, WearClass::Impact, WearClass::Vessel, WearClass::Cordage, WearClass::Hull, WearClass::Thermal, WearClass::Record, WearClass::Structure, WearClass::Stock] {
            let d = class_durability(class, m);
            assert!(d >= 0.0 && d < 1.15, "{class:?} {} life {d} leaves the wear term", m.name());
            let r = repair_factor(class, m);
            assert!(r > 0.5 && r <= 1.0, "{class:?} {} repair {r}", m.name());
        }
    }
    assert_same_inputs("edge", |m| class_durability(WearClass::Edge, m), edge_key);
    assert_same_inputs("impact", |m| class_durability(WearClass::Impact, m), impact_key);
    assert_same_inputs("vessel", |m| class_durability(WearClass::Vessel, m), vessel_key);
    assert_same_inputs("cord", |m| class_durability(WearClass::Cordage, m), cord_key);
    assert_same_inputs("hull", |m| class_durability(WearClass::Hull, m), hull_key);
    assert_same_inputs("thermal", |m| class_durability(WearClass::Thermal, m), thermal_key);
    assert_same_inputs("record", |m| class_durability(WearClass::Record, m), record_key);
    assert_same_inputs("structure", |m| class_durability(WearClass::Structure, m), structure_key);
    assert_same_inputs("stock", |m| class_durability(WearClass::Stock, m), stock_key);
    #[cfg(not(feature = "durability-v2"))]
    {
        for &m in &ALL_MATS {
            assert_eq!(score_durability(m).to_bits(), durability(m).to_bits());
            assert_eq!(record_durability(m).to_bits(), durability(m).to_bits());
            for &f in &ALL_FORMS {
                let it = item(f, m);
                assert_eq!(upkeep_wear(it).to_bits(), (1.15 - durability(m)).to_bits(), "{}", item_name(it));
            }
        }
    }
    #[cfg(feature = "durability-v2")]
    {
        assert_eq!(score_durability(Mat::Steel).to_bits(), class_durability(WearClass::Edge, Mat::Steel).to_bits());
        let it = item(Form::Axe, Mat::Copper);
        let expect = (1.15 - class_durability(WearClass::Edge, Mat::Copper)) * repair_factor(WearClass::Edge, Mat::Copper);
        assert_eq!(upkeep_wear(it).to_bits(), expect.to_bits());
        assert_eq!(record_durability(Mat::Bone).to_bits(), class_durability(WearClass::Record, Mat::Bone).to_bits());
    }
}

fn axe_score(m: Mat, v2: bool) -> f32 {
    let d = if v2 { class_durability(WearClass::Edge, m) } else { durability(m) };
    0.5 * cut_q(m) + 0.5 * d
}

fn edge_score(m: Mat, v2: bool) -> f32 {
    let d = if v2 { class_durability(WearClass::Edge, m) } else { durability(m) };
    0.55 * cut_q(m) + 0.45 * d
}

fn mark_score(m: Mat, v2: bool) -> f32 {
    let d = if v2 { class_durability(WearClass::Record, m) } else { durability(m) };
    d + if m == Mat::Ceramic { 1.0 } else { 0.0 }
}

fn old_wear(m: Mat) -> f32 {
    1.15 - durability(m)
}

fn new_wear(class: WearClass, m: Mat) -> f32 {
    (1.15 - class_durability(class, m)) * repair_factor(class, m)
}

/// (a) Every material: old durability, new life and wear by class, axe and edge
/// scores, mark score, repair.
#[test]
#[ignore]
fn diag_durability_table() {
    println!("PROPOSAL diagnostic. feature durability-v2 is {}", if cfg!(feature = "durability-v2") { "ON (sim uses v2)" } else { "OFF (sim uses v1; this table still prints v2)" });
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "material", "old", "edge", "impact", "vessel", "cord", "hull", "thermal", "record", "struct", "stock", "repair", "oldWear", "edgeWear", "axeOld", "axeNew", "eqOld", "eqNew", "markNew"
    );
    for &m in &ALL_MATS {
        let r = repair_factor(WearClass::Edge, m);
        println!(
            "{:<14} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3}",
            m.name(),
            durability(m),
            class_durability(WearClass::Edge, m),
            class_durability(WearClass::Impact, m),
            class_durability(WearClass::Vessel, m),
            class_durability(WearClass::Cordage, m),
            class_durability(WearClass::Hull, m),
            class_durability(WearClass::Thermal, m),
            class_durability(WearClass::Record, m),
            class_durability(WearClass::Structure, m),
            class_durability(WearClass::Stock, m),
            r,
            old_wear(m),
            new_wear(WearClass::Edge, m),
            axe_score(m, false),
            axe_score(m, true),
            edge_score(m, false),
            edge_score(m, true),
            mark_score(m, true)
        );
    }
    println!("ties (same life, and the same inputs — expected where the props match):");
    print_ties("edge", |m| class_durability(WearClass::Edge, m));
    print_ties("impact", |m| class_durability(WearClass::Impact, m));
    print_ties("vessel", |m| class_durability(WearClass::Vessel, m));
    print_ties("cord", |m| class_durability(WearClass::Cordage, m));
    print_ties("hull", |m| class_durability(WearClass::Hull, m));
    print_ties("thermal", |m| class_durability(WearClass::Thermal, m));
    print_ties("record", |m| class_durability(WearClass::Record, m));
    print_ties("structure", |m| class_durability(WearClass::Structure, m));
    print_ties("stock", |m| class_durability(WearClass::Stock, m));
    println!("ANCHOR edges  oldDur/newLife/oldWear/newWear/axeOld/axeNew/repair");
    for m in [Mat::Obsidian, Mat::Flint, Mat::Stone, Mat::Bone, Mat::Copper, Mat::Bronze, Mat::Iron, Mat::Steel, Mat::Gold, Mat::Tin] {
        println!(
            "  {:<10} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3}",
            m.name(),
            durability(m),
            class_durability(WearClass::Edge, m),
            old_wear(m),
            new_wear(WearClass::Edge, m),
            axe_score(m, false),
            axe_score(m, true),
            repair_factor(WearClass::Edge, m)
        );
    }
    println!("other slot wear (old is one number; new depends on the form's class)");
    println!("  shield=impact  bow/net/garment=cord  boat=hull  vessel=vessel  hammer=impact  mark score above");
    for m in [Mat::Wood, Mat::Hide, Mat::Fibre, Mat::Flax, Mat::Wool, Mat::Ceramic, Mat::Clay, Mat::Stone, Mat::Copper] {
        println!(
            "  {:<10} oldWear {:7.3} | impact {:7.3} cord {:7.3} hull {:7.3} vessel {:7.3} thermal {:7.3}",
            m.name(),
            old_wear(m),
            new_wear(WearClass::Impact, m),
            new_wear(WearClass::Cordage, m),
            new_wear(WearClass::Hull, m),
            new_wear(WearClass::Vessel, m),
            new_wear(WearClass::Thermal, m)
        );
    }
}

fn print_ties(label: &str, life: impl Fn(Mat) -> f32) {
    let mut map: BTreeMap<u32, Vec<&'static str>> = BTreeMap::new();
    for &m in &ALL_MATS {
        map.entry(bits(life(m))).or_default().push(m.name());
    }
    for names in map.values() {
        if names.len() > 1 {
            println!("  {label}: {}", names.join(", "));
        }
    }
}

fn live_v2() -> bool {
    cfg!(feature = "durability-v2")
}

fn wear_model(it: Item, v2: bool) -> f32 {
    if !v2 {
        return old_wear(mat(it));
    }
    let class = wear_class(form(it));
    new_wear(class, mat(it))
}

fn price_of(food: f32, labour: f32, view: &BandView) -> f32 {
    let pop = view.pop.max(1.0);
    let units = labour * LABOUR_SHARE * pop;
    let price = view.food.max(FOOD_EPS) / (WORK_SHARE * LABOUR_SHARE * pop);
    food - price * units
}

fn nonfood_price(slot: usize, it: Item, u: f32, ql: f32, d: u8, v2: bool) -> f32 {
    let m = mat(it);
    let term = match slot {
        11 | 16 | 18 | 22 => boat_q(m),
        14 | 15 => m.props().tough + 1.0,
        _ => 0.0,
    };
    let up = KIT_UPKEEP * wear_model(it, v2) * (1.0 + 0.3 * d as f32);
    term * (1.0 + crate::band::QUALITY_YIELD * ql) * user_factor(u) - up
}

fn food_pair(view: &BandView, kit: &crate::band::Kit, tk: &crate::knowledge::Toolkit, known: &[crate::knowledge::Known], slot: usize, it: Item) -> (f32, f32, f32) {
    let info = |x: Item| recipe_info(tk, known, x);
    let mskill = tk.chain_m(it);
    let fx = tool_effects_m(view, kit, slot, it, &info, 0, mskill);
    let live = live_v2();
    let wear_l = wear_model(it, live);
    let wear_o = wear_model(it, !live);
    let lab_o = if wear_l > 1e-8 { fx.labour * wear_o / wear_l } else { fx.labour };
    let food_o = if slot == 4 {
        let t_l = axe_score(mat(it), live);
        let t_o = axe_score(mat(it), !live);
        if t_l > 1e-8 { fx.food * t_o / t_l } else { fx.food }
    } else {
        fx.food
    };
    let p_l = price_of(fx.food, fx.labour, view);
    let p_o = price_of(food_o, lab_o, view);
    let (p1, p2) = if live { (p_o, p_l) } else { (p_l, p_o) };
    let scored = slot_score_m(view, kit, slot, it, &info, mskill).0;
    (p1, p2, scored)
}

fn argmax(rows: &[(Item, f32)]) -> Item {
    let mut best = NONE;
    let mut s = f32::NEG_INFINITY;
    for &(it, sc) in rows {
        if sc > s {
            s = sc;
            best = it;
        }
    }
    best
}

fn is_farm(sow: f32, harvest: &[f32; 5]) -> bool {
    let hsum: f32 = harvest.iter().copied().sum();
    let raw = if hsum > 1e-6 { harvest[4] / hsum } else { 0.0 };
    raw >= 0.10 || sow >= 0.2
}

fn bump(map: &mut BTreeMap<String, (u64, u64)>, key: String, farm: bool) {
    let e = map.entry(key).or_insert((0, 0));
    if farm {
        e.0 += 1;
    } else {
        e.1 += 1;
    }
}

/// (b) Which payoff slot (and the mark) would change material under v2, on the
/// loaded save. v1 and v2 are both scored here; the feature only changes which
/// one the sim itself uses. One slot at a time, in the kit as loaded.
#[test]
#[ignore]
fn diag_durability_flips() {
    let path = std::env::var("DIAG_SAVE").unwrap_or_else(|_| DIAG_SAVE_DEFAULT.to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (w, _, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load {path}: {e}"));
    println!(
        "FLIPS save {path} format v{} year {:.0} bands {} pop {} live-model {}",
        info.format_version,
        w.year(),
        w.bands.len(),
        w.total_pop(),
        if live_v2() { "v2" } else { "v1" }
    );
    let mut flips: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut axe_v1: BTreeMap<String, u64> = BTreeMap::new();
    let mut axe_v2: BTreeMap<String, u64> = BTreeMap::new();
    let mut scorer_bad = 0u64;
    let mut scorer_n = 0u64;
    let mut kit_mismatch = 0u64;
    let mut kit_n = 0u64;
    let mut farm_n = 0u64;
    let mut forage_n = 0u64;
    let mut sample: Vec<String> = Vec::new();
    for b in &w.bands {
        let view = w.band_view(b);
        let farm = is_farm(b.sow, &view.harvest);
        if farm {
            farm_n += 1;
        } else {
            forage_n += 1;
        }
        let tk = &b.d.tk;
        let known = &b.known;
        let kit = &b.d.kit;
        for (j, &slot) in PAYOFF_SLOTS.iter().enumerate() {
            let mut rows1: Vec<(Item, f32)> = Vec::new();
            let mut rows2: Vec<(Item, f32)> = Vec::new();
            for it in tk.avail.iter() {
                if !fills_payoff(it, j) {
                    continue;
                }
                let (p1, p2, scored) = if matches!(slot, 0 | 1 | 10 | 4 | 12 | 13) {
                    food_pair(&view, kit, tk, known, slot, it)
                } else {
                    let (_sk, ql, d) = recipe_info(tk, known, it);
                    let u = view.user[form(it) as usize];
                    let p1 = nonfood_price(slot, it, u, ql, d, false);
                    let p2 = nonfood_price(slot, it, u, ql, d, true);
                    let scored = slot_score_m(&view, kit, slot, it, &|x| recipe_info(tk, known, x), tk.chain_m(it)).0;
                    (p1, p2, scored)
                };
                let live = if live_v2() { p2 } else { p1 };
                scorer_n += 1;
                if (live - scored).abs() > 1e-3 {
                    scorer_bad += 1;
                }
                rows1.push((it, p1));
                rows2.push((it, p2));
            }
            if rows1.is_empty() {
                continue;
            }
            let a = argmax(&rows1);
            let c = argmax(&rows2);
            kit_n += 1;
            let held = kit.slot_item(j);
            let live_pick = if live_v2() { c } else { a };
            if live_pick != held {
                kit_mismatch += 1;
            }
            if j == 3 {
                *axe_v1.entry(item_name(a)).or_insert(0) += 1;
                *axe_v2.entry(item_name(c)).or_insert(0) += 1;
            }
            if a != c {
                let key = format!("{} {} -> {}", SLOT_NAMES[j], item_name(a), item_name(c));
                bump(&mut flips, key.clone(), farm);
                if sample.len() < 24 && SLOT_NAMES[j] == "axe" {
                    sample.push(format!("band {} {} {key}", b.id, if farm { "farm" } else { "forage" }));
                }
            }
        }
        let mut marks1: Vec<(Item, f32)> = Vec::new();
        let mut marks2: Vec<(Item, f32)> = Vec::new();
        for it in tk.avail.iter() {
            if form(it) != Form::Mark {
                continue;
            }
            marks1.push((it, mark_score(mat(it), false)));
            marks2.push((it, mark_score(mat(it), true)));
        }
        if !marks1.is_empty() {
            let a = argmax(&marks1);
            let c = argmax(&marks2);
            if a != c {
                let key = format!("mark {} -> {}", item_name(a), item_name(c));
                bump(&mut flips, key, farm);
            }
        }
    }
    println!("bands farm {farm_n} forage {forage_n}");
    println!("scorer vs slot_score mismatches {scorer_bad} of {scorer_n} (above 0.001)");
    println!("live pick vs loaded kit mismatches {kit_mismatch} of {kit_n} (a later pass can move a slot; not a flip)");
    println!("axe census v1:");
    for (k, n) in &axe_v1 {
        println!("  {n:6} {k}");
    }
    println!("axe census v2:");
    for (k, n) in &axe_v2 {
        println!("  {n:6} {k}");
    }
    println!("FLIPS from -> to  (farm / forage). Empty means that slot did not change.");
    if flips.is_empty() {
        println!("  none");
    }
    for (k, (farm, wild)) in &flips {
        println!("  {farm:5} farm  {wild:5} forage  {k}");
    }
    println!("sample of axe flips (at most 24):");
    for line in &sample {
        println!("  {line}");
    }
}
