//! Ignored diagnostics for the durability proposal. Not part of the sim.
//!
//! `durability_v2_orders_edges_and_v1_formula_stays` runs in the normal suite.
//! The `diag_durability_*` tests are `#[ignore]`. They print; they do not
//! assert a ranking and they do not change a hash.
//!
//! `DIAG_SAVE` is the snapshot (the same one as the steel-axe diagnostic).

#[cfg(feature = "durability-v2")]
use crate::band::Kit;
use crate::effects::{fills_payoff, recipe_info, slot_score_m, BandView, PAYOFF_SLOTS};
#[cfg(feature = "durability-v2")]
use crate::effects::slot_score;
use crate::materials::*;
use crate::World;
use std::collections::BTreeMap;

#[cfg(feature = "durability-v2")]
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

/// The default build still uses the old formula. Non-edge classes keep their own
/// lives. An edge is not one of those lives: the feature-on sim uses the split
/// wear, and that order is `split_model_orders_retention_break_and_price`.
#[test]
fn durability_v2_orders_edges_and_v1_formula_stays() {
    // The bug the proposal is answering: the old function still caps copper above steel.
    assert!(durability(Mat::Copper) > durability(Mat::Steel));
    assert!((durability(Mat::Copper) - 1.0).abs() < 1e-6);
    assert!((durability(Mat::Bronze) - 1.0).abs() < 1e-6);
    assert!((durability(Mat::Iron) - 1.0).abs() < 1e-6);
    assert!(repair_factor(WearClass::Edge, Mat::Copper) < repair_factor(WearClass::Edge, Mat::Steel));
    assert!(repair_factor(WearClass::Edge, Mat::Steel) < repair_factor(WearClass::Edge, Mat::Flint));
    let classes = [
        WearClass::Impact, WearClass::Vessel, WearClass::Cordage, WearClass::Hull, WearClass::Thermal,
        WearClass::Record, WearClass::Structure, WearClass::Stock,
    ];
    for &m in &ALL_MATS {
        for class in classes {
            let d = class_durability(class, m);
            assert!(d >= 0.0 && d < 1.15, "{class:?} {} life {d} leaves the wear term", m.name());
            let r = repair_factor(class, m);
            assert!(r > 0.5 && r <= 1.0, "{class:?} {} repair {r}", m.name());
        }
        // The edge arm is not a life. The sim does not read it.
        assert_eq!(class_durability(WearClass::Edge, m).to_bits(), 0.0f32.to_bits());
    }
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
            assert_eq!(food_edge_q(m).to_bits(), cut_q(m).to_bits());
            assert_eq!(axe_food_score(m).to_bits(), (0.5 * cut_q(m) + 0.5 * durability(m)).to_bits());
            for &f in &ALL_FORMS {
                let it = item(f, m);
                assert_eq!(upkeep_wear(it).to_bits(), (1.15 - durability(m)).to_bits(), "{}", item_name(it));
            }
        }
    }
    #[cfg(feature = "durability-v2")]
    {
        assert_eq!(score_durability(Mat::Steel).to_bits(), edge_toughness(Mat::Steel).to_bits());
        assert_eq!(food_edge_q(Mat::Copper).to_bits(), split_working_cut(Mat::Copper).to_bits());
        assert_eq!(axe_food_score(Mat::Copper).to_bits(), split_axe_score(Mat::Copper).to_bits());
        let it = item(Form::Axe, Mat::Copper);
        assert_eq!(upkeep_wear(it).to_bits(), split_edge_wear(Mat::Copper).to_bits());
        // A cord is not an edge: the split model does not rewrite it.
        let cord = item(Form::Cord, Mat::Flax);
        let cord_wear = (1.15 - class_durability(WearClass::Cordage, Mat::Flax)) * repair_factor(WearClass::Cordage, Mat::Flax);
        assert_eq!(upkeep_wear(cord).to_bits(), cord_wear.to_bits());
        assert_eq!(record_durability(Mat::Bone).to_bits(), class_durability(WearClass::Record, Mat::Bone).to_bits());
        // Malleability does not add to the split edge. Copper is softer and more
        // malleable than iron, and its retention is still shorter.
        assert!(edge_retention(Mat::Copper) < edge_retention(Mat::Iron));
        assert!(edge_toughness(Mat::Gold) < edge_toughness(Mat::Copper) || edge_retention(Mat::Gold) < edge_retention(Mat::Copper));
    }
}

/// Feature on: the priced edge at chain depth 0, nominal band. A glass edge
/// must not beat iron, bronze or steel. Flint still beats soft copper as a knife.
/// A point fishes on presence, so its order is wear: copper ahead of flint.
#[cfg(feature = "durability-v2")]
fn assert_priced_edge_orders(label: &str) {
    let view = BandView::nominal();
    let info = |_: Item| (1.0f32, 0.0, 0u8);
    let price = |slot: usize, f: Form, m: Mat| {
        let it = item(f, m);
        slot_score(&view, &Kit::default(), slot, it, &info).0
    };
    let order = |slot: usize, f: Form, mats: &[Mat], what: &str| {
        let mut prev = f32::INFINITY;
        let mut prev_m = "";
        let mut shown = String::new();
        for &m in mats {
            let p = price(slot, f, m);
            shown.push_str(&format!(" {}={:.4}", m.name(), p));
            assert!(p < prev, "{label} {what}: {prev_m} {prev:.4} should beat {} {p:.4};{shown}", m.name());
            prev = p;
            prev_m = m.name();
        }
    };
    let cut = [Mat::Steel, Mat::Iron, Mat::Bronze, Mat::Flint, Mat::Copper, Mat::Obsidian];
    order(1, Form::Edge, &cut, "knife");
    // Food is the same for every point. Copper wears less than flint, so it prices higher.
    let point = [Mat::Steel, Mat::Iron, Mat::Bronze, Mat::Copper, Mat::Flint, Mat::Obsidian];
    order(10, Form::Point, &point, "point");
    let axe = [Mat::Steel, Mat::Iron, Mat::Bronze, Mat::Copper, Mat::Stone, Mat::Flint, Mat::Obsidian];
    order(4, Form::Axe, &axe, "axe");
    assert!(price(1, Form::Edge, Mat::Flint) > price(1, Form::Edge, Mat::Stone), "{label}: flint knife over stone");
    assert!(price(1, Form::Edge, Mat::Copper) > price(1, Form::Edge, Mat::Obsidian), "{label}: copper knife over obsidian");
    assert!(price(10, Form::Point, Mat::Copper) > price(10, Form::Point, Mat::Obsidian), "{label}: copper point over obsidian");
    assert!(price(0, Form::Spear, Mat::Iron) > price(0, Form::Spear, Mat::Obsidian), "{label}: iron spear over obsidian");
    assert!(price(0, Form::Spear, Mat::Steel) > price(0, Form::Spear, Mat::Iron), "{label}: steel spear over iron");
}

#[cfg(feature = "durability-v2")]
#[test]
fn working_edge_orders_knife_axe_and_point() {
    assert_priced_edge_orders("split");
}

/// Split account, direct orders. Retention is hardness and acuity only.
/// Malleability cheapens a hone and never lengthens the interval. A knapped
/// edge's upkeep is mostly replacement. Copper's resharpen labour and its
/// average cut both sit clearly the wrong side of iron.
#[cfg(feature = "durability-v2")]
#[test]
fn split_model_orders_retention_break_and_price() {
    let ret = edge_retention;
    assert!(ret(Mat::Copper) < ret(Mat::Bronze) && ret(Mat::Bronze) < ret(Mat::Iron) && ret(Mat::Iron) < ret(Mat::Steel));
    assert!(ret(Mat::Gold) < ret(Mat::Copper), "softer than copper, however malleable");
    // Gold is more malleable than copper. Retention does not reward that.
    assert!(Mat::Gold.props().malleable > Mat::Copper.props().malleable);
    let resh = resharpen_labour;
    assert!(resh(Mat::Copper) > resh(Mat::Iron) * 1.5, "copper is honed clearly more often than iron");
    assert!(resh(Mat::Iron) > resh(Mat::Steel), "iron is honed more often than steel");
    assert!(resh(Mat::Copper) > resh(Mat::Steel) * 2.0, "copper against steel is a larger hone gap");
    assert!(split_working_cut(Mat::Copper) < cut_q(Mat::Copper) * 0.55, "copper's working cut is well below its fresh cut");
    assert!(split_working_cut(Mat::Copper) < split_working_cut(Mat::Iron) * 0.7, "copper cuts clearly less than iron between hones");
    assert!(split_working_cut(Mat::Iron) < split_working_cut(Mat::Steel));
    assert!(split_working_cut(Mat::Flint) > split_working_cut(Mat::Copper), "flint stays keener than soft copper");
    let br = break_labour;
    assert!(br(Mat::Obsidian) > br(Mat::Flint) && br(Mat::Flint) > br(Mat::Stone) && br(Mat::Stone) > br(Mat::Copper));
    assert!(br(Mat::Obsidian) > resh(Mat::Obsidian), "obsidian's cost is mostly replacement");
    assert!(br(Mat::Flint) > resh(Mat::Flint), "flint's cost is mostly replacement");
    assert!(resh(Mat::Copper) > br(Mat::Copper), "copper's cost is mostly the hone");
    assert!(resh(Mat::Iron) > br(Mat::Iron));
    assert!(split_edge_wear(Mat::Steel) < split_edge_wear(Mat::Iron));
    assert!(split_edge_wear(Mat::Iron) < split_edge_wear(Mat::Bronze));
    assert!(split_edge_wear(Mat::Bronze) < split_edge_wear(Mat::Copper));
    assert!(split_edge_wear(Mat::Copper) < split_edge_wear(Mat::Stone));
    assert!(split_edge_wear(Mat::Stone) < split_edge_wear(Mat::Flint));
    assert!(split_edge_wear(Mat::Flint) < split_edge_wear(Mat::Obsidian));
    // Repair still gets cheaper as malleability rises, and that is the only place it helps.
    assert!(resharpen_cost(Mat::Copper) < resharpen_cost(Mat::Iron));
    assert!(resharpen_cost(Mat::Iron) < resharpen_cost(Mat::Flint));
    assert!((resharpen_cost(Mat::Obsidian) - 1.0).abs() < 1e-6, "a glass edge is replaced, not hammered");
    for &m in &ALL_MATS {
        assert!(split_edge_wear(m) >= 0.0, "{}", m.name());
        assert!(split_working_cut(m) >= 0.0, "{}", m.name());
        assert!(edge_retention(m) >= 0.0, "{}", m.name());
    }
    assert_priced_edge_orders("split-direct");
}

fn axe_before(m: Mat) -> f32 {
    0.5 * cut_q(m) + 0.5 * durability(m)
}

fn edge_before(m: Mat) -> f32 {
    0.55 * cut_q(m) + 0.45 * durability(m)
}

fn edge_after(m: Mat) -> f32 {
    0.55 * cut_q(m) + 0.45 * edge_toughness(m)
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

/// (a) Every material: today's durability, the split edge, and the non-edge class lives.
#[test]
#[ignore]
fn diag_durability_table() {
    println!(
        "PROPOSAL diagnostic. feature durability-v2 is {}.",
        if cfg!(feature = "durability-v2") { "ON (sim uses the split model)" } else { "OFF (sim uses v1; this table still prints the split model)" }
    );
    println!("Edges are not a class life. work is split_working_cut. wear is split_edge_wear. axe is split_axe_score.");
    println!("eq is the fight blend: before is 0.55*cut+0.45*durability, after is 0.55*cut+0.45*toughness.");
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "material", "old", "work", "resh", "brk", "wear", "axeOld", "axeNew", "eqOld", "eqNew", "impact", "vessel", "cord", "hull", "thermal", "record", "struct", "stock"
    );
    for &m in &ALL_MATS {
        println!(
            "{:<14} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3}",
            m.name(),
            durability(m),
            split_working_cut(m),
            resharpen_labour(m),
            break_labour(m),
            split_edge_wear(m),
            axe_before(m),
            split_axe_score(m),
            edge_before(m),
            edge_after(m),
            class_durability(WearClass::Impact, m),
            class_durability(WearClass::Vessel, m),
            class_durability(WearClass::Cordage, m),
            class_durability(WearClass::Hull, m),
            class_durability(WearClass::Thermal, m),
            class_durability(WearClass::Record, m),
            class_durability(WearClass::Structure, m),
            class_durability(WearClass::Stock, m),
        );
    }
    println!("ties (same life, and the same inputs — expected where the props match):");
    print_ties("impact", |m| class_durability(WearClass::Impact, m));
    print_ties("vessel", |m| class_durability(WearClass::Vessel, m));
    print_ties("cord", |m| class_durability(WearClass::Cordage, m));
    print_ties("hull", |m| class_durability(WearClass::Hull, m));
    print_ties("thermal", |m| class_durability(WearClass::Thermal, m));
    print_ties("record", |m| class_durability(WearClass::Record, m));
    print_ties("structure", |m| class_durability(WearClass::Structure, m));
    print_ties("stock", |m| class_durability(WearClass::Stock, m));
    println!("tool edges  oldDur/oldWear/splitWear/axeOld/axeNew/work/repair");
    for m in [Mat::Obsidian, Mat::Flint, Mat::Stone, Mat::Bone, Mat::Copper, Mat::Bronze, Mat::Iron, Mat::Steel, Mat::Gold, Mat::Tin] {
        println!(
            "  {:<10} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3} {:7.3}",
            m.name(),
            durability(m),
            old_wear(m),
            split_edge_wear(m),
            axe_before(m),
            split_axe_score(m),
            split_working_cut(m),
            repair_factor(WearClass::Edge, m)
        );
    }
    println!("other slot wear (old is one number; new depends on the form's class)");
    println!("  shield=impact  bow/net/garment=cord  boat=hull  vessel=vessel  hammer=impact  mark is record life");
    for m in [Mat::Wood, Mat::Hide, Mat::Fibre, Mat::Flax, Mat::Wool, Mat::Ceramic, Mat::Clay, Mat::Stone, Mat::Copper] {
        println!(
            "  {:<10} oldWear {:7.3} | impact {:7.3} cord {:7.3} hull {:7.3} vessel {:7.3} thermal {:7.3} mark {:7.3}",
            m.name(),
            old_wear(m),
            new_wear(WearClass::Impact, m),
            new_wear(WearClass::Cordage, m),
            new_wear(WearClass::Hull, m),
            new_wear(WearClass::Vessel, m),
            new_wear(WearClass::Thermal, m),
            mark_score(m, true)
        );
    }
    print_accounts();
}

fn ratio(a: f32, b: f32) -> f32 {
    if b.abs() < 1e-9 { f32::NAN } else { a / b }
}

fn print_ratio_row(label: &str, v: impl Fn(Mat) -> f32) {
    let pairs = [
        ("st/cu", Mat::Steel, Mat::Copper),
        ("st/fe", Mat::Steel, Mat::Iron),
        ("fe/br", Mat::Iron, Mat::Bronze),
        ("br/cu", Mat::Bronze, Mat::Copper),
        ("cu/sn", Mat::Copper, Mat::Stone),
        ("fe/cu", Mat::Iron, Mat::Copper),
        ("fl/ob", Mat::Flint, Mat::Obsidian),
        ("st/sn", Mat::Steel, Mat::Stone),
        ("br/sn", Mat::Bronze, Mat::Stone),
    ];
    print!("  {label:<16}");
    for (name, a, b) in pairs {
        print!(" {name} {:6.3}", ratio(v(a), v(b)));
    }
    println!();
}

/// Split model, every material, plus before/after ratios for the tool edges.
/// Pure functions, so the feature-off binary prints them too.
fn print_accounts() {
    println!("SPLIT every material. fresh=cut_q. work=split_working_cut. interval=resharpen_interval.");
    println!("resh=resharpen_labour. brk=break_labour. wear=split_edge_wear. axe=split_axe_score. knife=work.");
    println!("plants=0.1*axe (the plants coefficient). farm=0.2*axe.");
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "material", "fresh", "work", "interval", "resh", "brk", "wear", "axe", "knife", "plants", "farm"
    );
    for &m in &ALL_MATS {
        let axe = split_axe_score(m);
        let work = split_working_cut(m);
        println!(
            "{:<14} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3}",
            m.name(),
            cut_q(m),
            work,
            resharpen_interval(m),
            resharpen_labour(m),
            break_labour(m),
            split_edge_wear(m),
            axe,
            work,
            0.1 * axe,
            0.2 * axe
        );
    }
    println!("RATIOS a/b. before = today's formula (fresh cut, durability as the only 'life', wear=1.15-dur, axe=0.5*cut+0.5*dur, knife=fresh).");
    println!("after uses the split functions. plants and farm are 0.1*axe and 0.2*axe.");
    let before_wear = old_wear;
    let before_axe = axe_before;
    println!("fresh cut (unchanged: cut_q, before = after)");
    print_ratio_row("fresh", cut_q);
    println!("edge life / resharpen interval");
    print_ratio_row("before-life", durability);
    print_ratio_row("split-interval", resharpen_interval);
    println!("toughness / break (before has no separate break; column is old wear, lower is better so the ratio flips)");
    print_ratio_row("before-wear", before_wear);
    print_ratio_row("split-break", break_labour);
    print_ratio_row("split-resh", resharpen_labour);
    println!("working cut");
    print_ratio_row("before-work", cut_q);
    print_ratio_row("split-work", split_working_cut);
    println!("total upkeep (wear)");
    print_ratio_row("before-upkeep", before_wear);
    print_ratio_row("split-upkeep", split_edge_wear);
    println!("axe score and the food coefficients that multiply it");
    print_ratio_row("before-axe", before_axe);
    print_ratio_row("split-axe", split_axe_score);
    print_ratio_row("before-plants", |m| 0.1 * before_axe(m));
    print_ratio_row("split-plants", |m| 0.1 * split_axe_score(m));
    print_ratio_row("before-farm", |m| 0.2 * before_axe(m));
    print_ratio_row("split-farm", |m| 0.2 * split_axe_score(m));
    println!("knife score (the food term: fresh cut before, split_working_cut after)");
    print_ratio_row("before-knife", cut_q);
    print_ratio_row("split-knife", split_working_cut);
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

#[cfg(feature = "durability-v2")]
fn bump(map: &mut BTreeMap<String, (u64, u64)>, key: String, farm: bool) {
    let e = map.entry(key).or_insert((0, 0));
    if farm {
        e.0 += 1;
    } else {
        e.1 += 1;
    }
}

fn score_item(view: &BandView, kit: &crate::band::Kit, tk: &crate::knowledge::Toolkit, known: &[crate::knowledge::Known], slot: usize, it: Item) -> f32 {
    slot_score_m(view, kit, slot, it, &|x| recipe_info(tk, known, x), tk.chain_m(it)).0
}

/// (b) Which payoff slot (and the mark) would change material under the split
/// model, on the loaded save. Feature on, each slot is scored at today's formula
/// and at the split model. Feature off, only the live v1 pick is counted.
#[test]
#[ignore]
fn diag_durability_flips() {
    let Some(path) = crate::tests_diag::diag_save_or_skip() else {
        return;
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (w, _, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load {path}: {e}"));
    println!(
        "FLIPS save {path} format v{} year {:.0} bands {} pop {} live-model {}",
        info.format_version,
        w.year(),
        w.bands.len(),
        w.total_pop(),
        durability_account_name()
    );
    #[cfg(feature = "durability-v2")]
    let mut flips: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut axe_v1: BTreeMap<String, u64> = BTreeMap::new();
    let mut axe_split: BTreeMap<String, u64> = BTreeMap::new();
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
            let mut rows_v1: Vec<(Item, f32)> = Vec::new();
            let mut rows_sp: Vec<(Item, f32)> = Vec::new();
            for it in tk.avail.iter() {
                if !fills_payoff(it, j) {
                    continue;
                }
                let live = score_item(&view, kit, tk, known, slot, it);
                #[cfg(feature = "durability-v2")]
                let (v1, sp) = {
                    let v1 = with_v1_model(|| score_item(&view, kit, tk, known, slot, it));
                    (v1, live)
                };
                #[cfg(not(feature = "durability-v2"))]
                let (v1, sp) = (live, live);
                scorer_n += 1;
                let expect = if live_v2() { sp } else { v1 };
                if (live - expect).abs() > 1e-3 {
                    scorer_bad += 1;
                }
                rows_v1.push((it, v1));
                rows_sp.push((it, sp));
            }
            if rows_v1.is_empty() {
                continue;
            }
            let pick_v1 = argmax(&rows_v1);
            let pick_sp = argmax(&rows_sp);
            kit_n += 1;
            let held = kit.slot_item(j);
            let live_pick = if live_v2() { pick_sp } else { pick_v1 };
            if live_pick != held {
                kit_mismatch += 1;
            }
            if j == 3 {
                *axe_v1.entry(item_name(pick_v1)).or_insert(0) += 1;
                *axe_split.entry(item_name(pick_sp)).or_insert(0) += 1;
            }
            #[cfg(feature = "durability-v2")]
            if pick_v1 != pick_sp {
                let key = format!("{} {} -> {}", SLOT_NAMES[j], item_name(pick_v1), item_name(pick_sp));
                bump(&mut flips, key, farm);
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
                bump(&mut flips, key, farm);
                #[cfg(not(feature = "durability-v2"))]
                {
                    let _ = (key, farm);
                }
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
    #[cfg(not(feature = "durability-v2"))]
    {
        let _ = axe_split;
        println!("feature off: the split flip table is not scored in this binary. Re-run with --features durability-v2.");
    }
    #[cfg(feature = "durability-v2")]
    {
        println!("axe census split:");
        for (k, n) in &axe_split {
            println!("  {n:6} {k}");
        }
        println!("FLIPS v1 -> split  (farm / forage).");
        if flips.is_empty() {
            println!("  none");
        }
        let mut bands = 0u64;
        for (k, (farm, wild)) in &flips {
            bands += farm + wild;
            println!("  {farm:5} farm  {wild:5} forage  {k}");
        }
        println!("SUMMARY flip rows {} band-slots {bands}", flips.len());
    }
}
