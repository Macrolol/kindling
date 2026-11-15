//! Ignored diagnostics for the durability proposal. Not part of the sim.
//!
//! `durability_v2_orders_edges_and_v1_formula_stays` runs in the normal suite.
//! The `diag_durability_*` tests are `#[ignore]`. They print; they do not
//! assert a ranking and they do not change a hash.
//!
//! `DIAG_SAVE` is the snapshot (the same one as the steel-axe diagnostic).

use crate::band::Kit;
use crate::effects::{fills_payoff, recipe_info, slot_score, slot_score_m, BandView, PAYOFF_SLOTS};
use crate::materials::*;
use crate::tests_diag::DIAG_SAVE_DEFAULT;
use crate::World;
use std::collections::BTreeMap;

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

fn assert_edge_order(life: impl Fn(Mat) -> f32) {
    assert!(life(Mat::Obsidian) < life(Mat::Flint), "obsidian chips before flint");
    assert!(life(Mat::Flint) < life(Mat::Copper), "flint chips before copper dulls");
    assert!(life(Mat::Copper) < life(Mat::Stone), "a ground stone edge outlasts soft copper");
    assert!(life(Mat::Stone) < life(Mat::Bronze), "stone does not outlast bronze");
    assert!(life(Mat::Bronze) < life(Mat::Iron), "iron a bit ahead of bronze");
    assert!(life(Mat::Iron) < life(Mat::Steel), "steel keeps an edge");
}

/// The proposal's edge order, and that the default build still uses the old formula.
#[test]
fn durability_v2_orders_edges_and_v1_formula_stays() {
    let raw = raw_edge_life;
    let e = |m: Mat| class_durability(WearClass::Edge, m);
    assert_edge_order(raw);
    assert_edge_order(e);
    // The 15% gap is the raw formula. Compression keeps the order and shrinks it.
    assert!(raw(Mat::Steel) > raw(Mat::Iron) * 1.15, "raw steel is clearly ahead of iron, not a rounding");
    assert!(e(Mat::Steel) > e(Mat::Iron) + 0.04, "compressed steel is still clearly ahead of iron");
    assert!((EDGE_LIFE_SPREAD - 0.50).abs() < 1e-6, "the sim's spread is the recommended one");
    assert!((edge_life_at(Mat::Steel, 1.0) - raw(Mat::Steel)).abs() < 1e-6);
    let wq = working_edge_q;
    assert!(wq(Mat::Steel) > wq(Mat::Iron) && wq(Mat::Iron) > wq(Mat::Flint), "working edge: steel > iron > flint");
    assert!(wq(Mat::Iron) > wq(Mat::Obsidian) && wq(Mat::Bronze) > wq(Mat::Obsidian), "glass does not outrank a metal blade");
    assert!(wq(Mat::Flint) > wq(Mat::Copper), "flint is still keener than soft copper");
    assert!(wq(Mat::Flint) > wq(Mat::Stone), "a flint edge out-cuts dull stone");
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

/// Feature on: the priced working edge at chain depth 0, nominal band. A glass edge
/// must not beat iron, bronze or steel. Flint still beats soft copper as a knife.
#[cfg(feature = "durability-v2")]
#[test]
fn working_edge_orders_knife_axe_and_point() {
    let view = BandView::nominal();
    let info = |_: Item| (1.0f32, 0.0, 0u8);
    let price = |slot: usize, f: Form, m: Mat| {
        let it = item(f, m);
        slot_score(&view, &Kit::default(), slot, it, &info).0
    };
    let order = |slot: usize, f: Form, mats: &[Mat], label: &str| {
        let mut prev = f32::INFINITY;
        let mut prev_m = "";
        let mut shown = String::new();
        for &m in mats {
            let p = price(slot, f, m);
            shown.push_str(&format!(" {}={:.4}", m.name(), p));
            assert!(p < prev, "{label}: {prev_m} {prev:.4} should beat {} {p:.4};{shown}", m.name());
            prev = p;
            prev_m = m.name();
        }
    };
    // Knife and point: keenness discounted by life. Flint stays ahead of soft copper.
    let cut = [Mat::Steel, Mat::Iron, Mat::Bronze, Mat::Flint, Mat::Copper, Mat::Obsidian];
    order(1, Form::Edge, &cut, "knife");
    order(10, Form::Point, &cut, "point");
    // Axe: half keenness, half compressed life, then repair. Copper is a working axe
    // again; flint and obsidian chip. Stone (a ground celt) sits between copper and flint.
    let axe = [Mat::Steel, Mat::Iron, Mat::Bronze, Mat::Copper, Mat::Stone, Mat::Flint, Mat::Obsidian];
    order(4, Form::Axe, &axe, "axe");
    assert!(price(1, Form::Edge, Mat::Flint) > price(1, Form::Edge, Mat::Stone), "flint knife over stone");
    assert!(price(1, Form::Edge, Mat::Copper) > price(1, Form::Edge, Mat::Obsidian), "copper knife over obsidian");
    assert!(price(10, Form::Point, Mat::Copper) > price(10, Form::Point, Mat::Obsidian), "copper point over obsidian");
    assert!(price(0, Form::Spear, Mat::Iron) > price(0, Form::Spear, Mat::Obsidian), "iron spear over obsidian");
    assert!(price(0, Form::Spear, Mat::Steel) > price(0, Form::Spear, Mat::Iron), "steel spear over iron");
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
    println!(
        "PROPOSAL diagnostic. feature durability-v2 is {}. EDGE_LIFE_SPREAD {:.2} anchor {:.2} keep_floor {:.2}",
        if cfg!(feature = "durability-v2") { "ON (sim uses v2)" } else { "OFF (sim uses v1; this table still prints v2)" },
        EDGE_LIFE_SPREAD,
        EDGE_LIFE_ANCHOR,
        EDGE_KEEP_FLOOR
    );
    println!("edge column is compressed life (upkeep and the axe). rawEdge is spread 1. workQ is working_edge_q (raw life, not compressed).");
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "material", "old", "rawEdge", "edge", "impact", "vessel", "cord", "hull", "thermal", "record", "struct", "stock", "repair", "oldWear", "edgeWear", "axeOld", "axeNew", "eqOld", "eqNew", "markNew", "workQ"
    );
    for &m in &ALL_MATS {
        let r = repair_factor(WearClass::Edge, m);
        println!(
            "{:<14} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3}",
            m.name(),
            durability(m),
            raw_edge_life(m),
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
            mark_score(m, true),
            working_edge_q(m)
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
    println!("SPREAD compare (life / edgeWear / axe). workQ does not depend on spread.");
    for spread in [1.0f32, 0.50, 0.35] {
        println!("  spread {spread:.2}");
        for m in [Mat::Obsidian, Mat::Flint, Mat::Stone, Mat::Copper, Mat::Bronze, Mat::Iron, Mat::Steel] {
            let life = edge_life_at(m, spread);
            let wear = (1.15 - life) * repair_factor(WearClass::Edge, m);
            let axe = 0.5 * cut_q(m) + 0.5 * life;
            println!("    {:<10} life {:7.3} wear {:7.3} axe {:7.3} workQ {:7.3}", m.name(), life, wear, axe, working_edge_q(m));
        }
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

fn spread_moves_food(slot: usize) -> bool {
    matches!(slot, 0 | 1 | 4 | 10)
}

fn score_item(view: &BandView, kit: &crate::band::Kit, tk: &crate::knowledge::Toolkit, known: &[crate::knowledge::Known], slot: usize, it: Item) -> f32 {
    slot_score_m(view, kit, slot, it, &|x| recipe_info(tk, known, x), tk.chain_m(it)).0
}

/// (b) Which payoff slot (and the mark) would change material under v2, on the
/// loaded save. Feature on, each edge slot is scored at the old formula and at
/// edge-life spreads 1, 0.50 and 0.35. Feature off, only the live v1 pick is
/// counted (the v2 food path is not in that binary).
#[test]
#[ignore]
fn diag_durability_flips() {
    let path = std::env::var("DIAG_SAVE").unwrap_or_else(|_| DIAG_SAVE_DEFAULT.to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (w, _, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load {path}: {e}"));
    println!(
        "FLIPS save {path} format v{} year {:.0} bands {} pop {} live-model {} spread {:.2}",
        info.format_version,
        w.year(),
        w.bands.len(),
        w.total_pop(),
        if live_v2() { "v2" } else { "v1" },
        EDGE_LIFE_SPREAD
    );
    // 0 = v1, 1 = spread 1, 2 = spread 0.50, 3 = spread 0.35
    let labels = ["v1", "spread1.00", "spread0.50", "spread0.35"];
    let mut flips: [BTreeMap<String, (u64, u64)>; 3] = [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    let mut axe_c: [BTreeMap<String, u64>; 4] = [BTreeMap::new(), BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    let mut scorer_bad = 0u64;
    let mut scorer_n = 0u64;
    let mut kit_mismatch = 0u64;
    let mut kit_n = 0u64;
    let mut farm_n = 0u64;
    let mut forage_n = 0u64;
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
            let mut rows: [Vec<(Item, f32)>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
            for it in tk.avail.iter() {
                if !fills_payoff(it, j) {
                    continue;
                }
                let live = score_item(&view, kit, tk, known, slot, it);
                #[cfg(feature = "durability-v2")]
                let ps = {
                    let v1 = with_durability_model(true, 1.0, || score_item(&view, kit, tk, known, slot, it));
                    let s1 = if spread_moves_food(slot) {
                        with_durability_model(false, 1.0, || score_item(&view, kit, tk, known, slot, it))
                    } else {
                        live
                    };
                    let s50 = if spread_moves_food(slot) {
                        with_durability_model(false, 0.50, || score_item(&view, kit, tk, known, slot, it))
                    } else if (EDGE_LIFE_SPREAD - 0.50).abs() < 1e-6 {
                        live
                    } else {
                        with_durability_model(false, 0.50, || score_item(&view, kit, tk, known, slot, it))
                    };
                    let s35 = if spread_moves_food(slot) {
                        with_durability_model(false, 0.35, || score_item(&view, kit, tk, known, slot, it))
                    } else {
                        live
                    };
                    [v1, s1, s50, s35]
                };
                #[cfg(not(feature = "durability-v2"))]
                let ps = [live, live, live, live];
                scorer_n += 1;
                let expect = if live_v2() { ps[2] } else { ps[0] };
                if (live - expect).abs() > 1e-3 {
                    scorer_bad += 1;
                }
                for i in 0..4 {
                    rows[i].push((it, ps[i]));
                }
            }
            if rows[0].is_empty() {
                continue;
            }
            let picks: [Item; 4] = [argmax(&rows[0]), argmax(&rows[1]), argmax(&rows[2]), argmax(&rows[3])];
            kit_n += 1;
            let held = kit.slot_item(j);
            let live_pick = if live_v2() { picks[2] } else { picks[0] };
            if live_pick != held {
                kit_mismatch += 1;
            }
            if j == 3 {
                for i in 0..4 {
                    *axe_c[i].entry(item_name(picks[i])).or_insert(0) += 1;
                }
            }
            #[cfg(feature = "durability-v2")]
            {
                for i in 0..3 {
                    if picks[0] != picks[i + 1] {
                        let key = format!("{} {} -> {}", SLOT_NAMES[j], item_name(picks[0]), item_name(picks[i + 1]));
                        bump(&mut flips[i], key, farm);
                    }
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
                #[cfg(feature = "durability-v2")]
                {
                    for i in 0..3 {
                        bump(&mut flips[i], key.clone(), farm);
                    }
                }
                #[cfg(not(feature = "durability-v2"))]
                {
                    let _ = (key, farm);
                }
            }
        }
    }
    let _ = flips[0].len();
    println!("bands farm {farm_n} forage {forage_n}");
    println!("scorer vs slot_score mismatches {scorer_bad} of {scorer_n} (above 0.001)");
    println!("live pick vs loaded kit mismatches {kit_mismatch} of {kit_n} (a later pass can move a slot; not a flip)");
    for (i, label) in labels.iter().enumerate() {
        println!("axe census {label}:");
        for (k, n) in &axe_c[i] {
            println!("  {n:6} {k}");
        }
    }
    #[cfg(not(feature = "durability-v2"))]
    println!("feature off: v2 flip tables are not scored in this binary. Re-run with --features durability-v2.");
    #[cfg(feature = "durability-v2")]
    {
        for (i, label) in ["spread1.00", "spread0.50", "spread0.35"].iter().enumerate() {
            println!("FLIPS v1 -> {label}  (farm / forage).");
            if flips[i].is_empty() {
                println!("  none");
            }
            for (k, (farm, wild)) in &flips[i] {
                println!("  {farm:5} farm  {wild:5} forage  {k}");
            }
        }
        println!("SUMMARY flip rows (not band totals):");
        for (i, label) in ["spread1.00", "spread0.50", "spread0.35"].iter().enumerate() {
            let mut bands = 0u64;
            for (farm, wild) in flips[i].values() {
                bands += farm + wild;
            }
            println!("  {label} rows {} band-slots {bands}", flips[i].len());
        }
    }
}
