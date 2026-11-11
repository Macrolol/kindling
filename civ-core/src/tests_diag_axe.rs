//! Ignored diagnostic for the steel-axe question. Not part of the sim.
//!
//! `diag_axe_breakeven` scores each makeable axe the way `choose_food` does
//! after skills step 3 (`slot_score_m` at `tk.chain_m`), on the same snapshot
//! as `diag_copper_over_steel_axe`. It prints the food gain, the labour
//! charged (base upkeep, maker waste, failure), and how far that gap is from
//! a tie. It does not assert a ranking and it does not change a hash.
//!
//! `DIAG_SAVE` is the snapshot (the step-2 seed-6 year-350 save, refit on load
//! by the step-3 kit choice). If `/tmp/axe-seed5-y300.civsave` exists, that
//! world is reported too.

use crate::effects::{
    harvest_for, kit_yields, maker_p_fail, maker_waste, recipe_info, slot_score, slot_score_m, tool_effects, tool_effects_m, BandView,
    WORK_SHARE,
};
use crate::knowledge::{Known, Toolkit};
use crate::materials::*;
use crate::tests_diag::{with_axe, tv_of, PROBE_TIE};
use crate::World;

/// Axe is best[] slot 4 (`CHOOSE_ORDER`).
const AXE_SLOT: usize = 4;
/// Step 5's `LAB_FLOOR`: labour price with every hand spare, as a share of the full price.
const LAB_FLOOR: f32 = 0.2;
const FRESH_SAVE: &str = "/tmp/axe-seed5-y300.civsave";

const TAGS: [&str; 9] = ["none", "flint", "stone", "obsidian", "bone", "copper", "bronze", "iron", "steel"];

#[derive(Clone, Copy)]
struct Scored {
    food_raw: f32,
    food_kept: f32,
    lab_base: f32,
    lab: f32,
    pfail: f32,
    waste: f32,
    chain_m: f32,
    depth: f32,
    skill: f32,
    cur_m: f32,
    cur_1: f32,
}

#[derive(Clone, Copy)]
struct AxRow {
    food_gain: f32,
    food_raw: f32,
    lab_h: f32,
    lab_s: f32,
    base_h: f32,
    base_s: f32,
    waste_h: f32,
    waste_s: f32,
    pfail_h: f32,
    pfail_s: f32,
    m_h: f32,
    m_s: f32,
    depth_h: f32,
    depth_s: f32,
    skill_h: f32,
    skill_s: f32,
    /// Other axe's chain-skill score minus steel's. Positive: the band keeps the other axe.
    cur_m: f32,
    cur_1: f32,
    priced_gap: f32,
    k_max: f32,
    g_min: f32,
    /// Steel's charged labour must fall to this fraction of itself before steel ties.
    w_max: f32,
    slack: f32,
    spare: f32,
    w_farm: f32,
    w_plant: f32,
    raw_farm: f32,
    sow: f32,
    ueff: f32,
    food_per: f32,
    pop: f32,
    d_tv: f32,
    keep_h: f32,
    keep_s: f32,
    re_tv: f32,
    stock_cu: f32,
    stock_fe: f32,
    stock_st: f32,
    stock_ch: f32,
    farm: bool,
    deep: bool,
    practised: bool,
    mismatch: bool,
    alt: &'static str,
}

fn tag_of(it: Item) -> &'static str {
    if it == NONE || form(it) != Form::Axe {
        return "none";
    }
    match mat(it) {
        Mat::Flint => "flint",
        Mat::Stone => "stone",
        Mat::Obsidian => "obsidian",
        Mat::Bone => "bone",
        Mat::Copper => "copper",
        Mat::Bronze => "bronze",
        Mat::Iron => "iron",
        Mat::Steel => "steel",
        _ => "other",
    }
}

fn tag_ix(name: &str) -> Option<usize> {
    TAGS.iter().position(|t| *t == name)
}

fn score_axe(view: &BandView, kit: &crate::band::Kit, tk: &Toolkit, known: &[Known], it: Item) -> Scored {
    let info = |x: Item| recipe_info(tk, known, x);
    let m = tk.chain_m(it);
    let fx1 = tool_effects(view, kit, AXE_SLOT, it, &info, 0);
    let fxm = tool_effects_m(view, kit, AXE_SLOT, it, &info, 0, m);
    let (skill, _, depth) = info(it);
    Scored {
        food_raw: fx1.food,
        food_kept: fxm.food,
        lab_base: fx1.labour,
        lab: fxm.labour,
        pfail: if m == 1.0 { 0.0 } else { maker_p_fail(m) },
        waste: maker_waste(m),
        chain_m: m,
        depth: depth as f32,
        skill,
        cur_m: slot_score_m(view, kit, AXE_SLOT, it, &info, m).0,
        cur_1: slot_score(view, kit, AXE_SLOT, it, &info).0,
    }
}

fn food_keep(k: &crate::band::Kit, tk: &Toolkit) -> f32 {
    let mut keep = 1.0;
    for j in [0usize, 1, 2, 3, 5, 6] {
        let it = k.slot_item(j);
        if it == NONE {
            continue;
        }
        let one = 1.0 - maker_p_fail(tk.chain_m(it));
        if one < keep {
            keep = one;
        }
    }
    keep
}

fn mean_of(rows: &[AxRow], f: impl Fn(&AxRow) -> f32) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(|r| f(r) as f64).sum::<f64>() / rows.len() as f64
}

fn mean_finite(rows: &[AxRow], f: impl Fn(&AxRow) -> f32) -> f64 {
    let mut s = 0.0;
    let mut n = 0u64;
    for r in rows {
        let x = f(r);
        if x.is_finite() {
            s += x as f64;
            n += 1;
        }
    }
    if n == 0 { 0.0 } else { s / n as f64 }
}

fn percentile(rows: &[AxRow], f: impl Fn(&AxRow) -> f32, p: f32) -> f32 {
    let mut v: Vec<f32> = rows.iter().map(f).filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return f32::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((v.len() - 1) as f32 * p).round() as usize;
    v[idx.min(v.len() - 1)]
}

fn pct_where(rows: &[AxRow], f: impl Fn(&AxRow) -> bool) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    100.0 * rows.iter().filter(|r| f(r)).count() as f64 / rows.len() as f64
}

fn price_factor(gap: f32, lab_s: f32, lab_h: f32) -> f32 {
    let d = lab_s - lab_h;
    if d.abs() <= 1e-8 { f32::NAN } else { gap / d }
}

/// Drop the `(1 + 0.3 * depth)` factor. Waste and the wear term stay.
fn gap_nodepth(r: &AxRow) -> f32 {
    let pf = price_factor(r.priced_gap, r.lab_s, r.lab_h);
    if !pf.is_finite() {
        return f32::NAN;
    }
    let df = |d: f32| 1.0 + 0.3 * d;
    (r.lab_s / df(r.depth_s) - r.lab_h / df(r.depth_h)) * pf
}

/// Charge the steel axe the other axe's chain depth, and leave everything else.
fn gap_match_depth(r: &AxRow) -> f32 {
    let pf = price_factor(r.priced_gap, r.lab_s, r.lab_h);
    if !pf.is_finite() {
        return f32::NAN;
    }
    let df = |d: f32| 1.0 + 0.3 * d;
    let lab_s = r.lab_s * df(r.depth_h) / df(r.depth_s);
    (lab_s - r.lab_h) * pf
}

fn beats(food_gain: f32, gap: f32) -> bool {
    gap.is_finite() && food_gain > gap
}

fn emit(label: &str, rows: &[AxRow]) {
    let n = rows.len();
    if n == 0 {
        println!("{label} n=0");
        return;
    }
    let mf = |f: fn(&AxRow) -> f32| mean_of(rows, f);
    let k10 = percentile(rows, |r| r.k_max, 0.10);
    let k50 = percentile(rows, |r| r.k_max, 0.50);
    let k90 = percentile(rows, |r| r.k_max, 0.90);
    let rank_h = rows.iter().filter(|r| r.cur_m > 0.0).count();
    let rank_s = rows.iter().filter(|r| r.cur_m < 0.0).count();
    let rank_t = n - rank_h - rank_s;
    println!(
        "{label} n={n} fk={:+.4} fr={:+.4} lab={:.4}/{:.4} base={:.4}/{:.4} waste={:.3}/{:.3} pfail={:.3}/{:.3} m={:.3}/{:.3} dep={:.2}/{:.2} sk={:.3}/{:.3} cur={:+.4} cur1={:+.4} rank={rank_h}/{rank_s}/{rank_t} gap={:.4} k={:.3}[{k10:.2}/{k50:.2}/{k90:.2}] g={:.2} wmax={:.3} slack%={:.1} nodep%={:.1} matchd%={:.1} m1%={:.1} spare={:.3} wf={:.3} wp={:.3} sow={:.3} rf={:.3} fpc={:.2} pop={:.1} ueff={:.3} tv={:+.4} pref%={:.1} re={:+.4} re%={:.1} keep={:.3}/{:.3} stock={:.2}/{:.2}/{:.2}/{:.2} mis%={:.1}",
        mf(|r| r.food_gain),
        mf(|r| r.food_raw),
        mf(|r| r.lab_h),
        mf(|r| r.lab_s),
        mf(|r| r.base_h),
        mf(|r| r.base_s),
        mf(|r| r.waste_h),
        mf(|r| r.waste_s),
        mf(|r| r.pfail_h),
        mf(|r| r.pfail_s),
        mf(|r| r.m_h),
        mf(|r| r.m_s),
        mf(|r| r.depth_h),
        mf(|r| r.depth_s),
        mf(|r| r.skill_h),
        mf(|r| r.skill_s),
        mf(|r| r.cur_m),
        mf(|r| r.cur_1),
        mf(|r| r.priced_gap),
        mean_finite(rows, |r| r.k_max),
        mean_finite(rows, |r| r.g_min),
        mean_finite(rows, |r| r.w_max),
        pct_where(rows, |r| beats(r.food_gain, r.slack * r.priced_gap)),
        pct_where(rows, |r| beats(r.food_gain, gap_nodepth(r))),
        pct_where(rows, |r| beats(r.food_gain, gap_match_depth(r))),
        pct_where(rows, |r| r.cur_1 < 0.0),
        mf(|r| r.spare),
        mf(|r| r.w_farm),
        mf(|r| r.w_plant),
        mf(|r| r.sow),
        mf(|r| r.raw_farm),
        mf(|r| r.food_per),
        mf(|r| r.pop),
        mf(|r| r.ueff),
        mf(|r| r.d_tv),
        pct_where(rows, |r| r.d_tv > PROBE_TIE),
        mf(|r| r.re_tv),
        pct_where(rows, |r| r.re_tv > PROBE_TIE),
        mf(|r| r.keep_h),
        mf(|r| r.keep_s),
        mf(|r| r.stock_cu),
        mf(|r| r.stock_fe),
        mf(|r| r.stock_st),
        mf(|r| r.stock_ch),
        pct_where(rows, |r| r.mismatch),
    );
}

fn emit_splits(pair: &str, rows: &[AxRow]) {
    if rows.is_empty() {
        println!("{pair} n=0");
        return;
    }
    let mut hist = std::collections::BTreeMap::<&str, u64>::new();
    for r in rows {
        *hist.entry(r.alt).or_insert(0) += 1;
    }
    let parts: Vec<String> = hist.iter().map(|(k, v)| format!("{k}:{v}")).collect();
    println!("{pair} alts {}", parts.join(" "));
    emit(&format!("{pair} all"), rows);
    let farms: Vec<AxRow> = rows.iter().copied().filter(|r| r.farm).collect();
    let wild: Vec<AxRow> = rows.iter().copied().filter(|r| !r.farm).collect();
    let hi: Vec<AxRow> = rows.iter().copied().filter(|r| r.practised).collect();
    let lo: Vec<AxRow> = rows.iter().copied().filter(|r| !r.practised).collect();
    let deep: Vec<AxRow> = rows.iter().copied().filter(|r| r.deep).collect();
    let shallow: Vec<AxRow> = rows.iter().copied().filter(|r| !r.deep).collect();
    emit(&format!("{pair} farms"), &farms);
    emit(&format!("{pair} not-farms"), &wild);
    emit(&format!("{pair} steel-m>=0.85"), &hi);
    emit(&format!("{pair} steel-m<0.85"), &lo);
    emit(&format!("{pair} depth>=8"), &deep);
    emit(&format!("{pair} depth<8"), &shallow);
}

fn stock_at(b: &crate::band::Band, m: Mat) -> f32 {
    b.stock.get(m as usize).copied().unwrap_or(0.0)
}

fn pair_row(view: &BandView, kit: &crate::band::Kit, tk: &Toolkit, known: &[Known], sow: f32, spare_in: f32, pop: f32, stock: [f32; 4], held_it: Item, steel_it: Item, other: Scored, steel: Scored) -> AxRow {
    let pf = view.food.max(1e-6) / WORK_SHARE;
    let food_gain = steel.food_kept - other.food_kept;
    let food_raw = steel.food_raw - other.food_raw;
    let priced_gap = (steel.lab - other.lab) * pf;
    let cur_m = other.cur_m - steel.cur_m;
    let cur_1 = other.cur_1 - steel.cur_1;
    let k_max = if priced_gap.abs() > 1e-8 { food_gain / priced_gap } else { f32::NAN };
    let g_min = if food_gain.abs() > 1e-8 { priced_gap / food_gain } else { f32::NAN };
    let p_s = steel.lab * pf;
    let w_max = if p_s > 1e-8 { (food_gain + other.lab * pf) / p_s } else { f32::NAN };
    let spare = spare_in.clamp(0.0, 1.0);
    let slack = LAB_FLOOR + (1.0 - LAB_FLOOR) * (1.0 - spare);
    let hsum: f32 = view.harvest.iter().copied().sum();
    let raw_farm = if hsum > 1e-6 { view.harvest[4] / hsum } else { 0.0 };
    let dg = kit.digest;
    let h = harvest_for(view, &dg);
    let mut ws = 0.0;
    let mut ww = [0.0; 5];
    for r in 0..5 {
        ww[r] = h[r] * dg[r];
        ws += ww[r];
    }
    let w_plant = if ws > 0.0 { ww[0] / ws } else { 0.0 };
    let w_farm = if ws > 0.0 { ww[4] / ws } else { 0.0 };
    let info = |x: Item| recipe_info(tk, known, x);
    let y_h = kit_yields(&with_axe(kit, held_it), &info, &view.user);
    let y_s = kit_yields(&with_axe(kit, steel_it), &info, &view.user);
    let p_h = tv_of(&y_h, tk, known);
    let p_s = tv_of(&y_s, tk, known);
    let d_plants = p_s.plants - p_h.plants;
    let d_farm = p_s.farm - p_h.farm;
    let d_up = p_s.upkeep - p_h.upkeep;
    let d_fight = p_s.fight - p_h.fight;
    let d_protect = p_s.protect - p_h.protect;
    let d_tv = p_s.value() - p_h.value();
    let keep_h = food_keep(&y_h, tk);
    let keep_s = food_keep(&y_s, tk);
    // Farm and plants at the band's own harvest weights, times the held kit's keep.
    // Upkeep, fight and protect stay the probe's. This is the yardstick with the
    // unweighted farm line taken out, not a second decision.
    let re_tv = (w_plant * d_plants + w_farm * d_farm) * keep_h - d_up + d_fight + d_protect;
    let expect = priced_gap - food_gain;
    AxRow {
        food_gain,
        food_raw,
        lab_h: other.lab,
        lab_s: steel.lab,
        base_h: other.lab_base,
        base_s: steel.lab_base,
        waste_h: other.waste,
        waste_s: steel.waste,
        pfail_h: other.pfail,
        pfail_s: steel.pfail,
        m_h: other.chain_m,
        m_s: steel.chain_m,
        depth_h: other.depth,
        depth_s: steel.depth,
        skill_h: other.skill,
        skill_s: steel.skill,
        cur_m,
        cur_1,
        priced_gap,
        k_max,
        g_min,
        w_max,
        slack,
        spare,
        w_farm,
        w_plant,
        raw_farm,
        sow,
        ueff: view.user[Form::Axe as usize],
        food_per: view.food / view.pop.max(1.0),
        pop,
        d_tv,
        keep_h,
        keep_s,
        re_tv,
        stock_cu: stock[0],
        stock_fe: stock[1],
        stock_st: stock[2],
        stock_ch: stock[3],
        farm: raw_farm >= 0.10 || sow >= 0.2,
        deep: steel.depth >= 8.0,
        practised: steel.chain_m >= 0.85,
        mismatch: (cur_m - expect).abs() > 1e-3,
        alt: tag_of(held_it),
    }
}

fn report_save(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (w, _, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load {path}: {e}"));
    let steel_it = item(Form::Axe, Mat::Steel);
    let term = |m: Mat| 0.5 * cut_q(m) + 0.5 * durability(m);
    println!(
        "=== {path} format v{} year {:.0} bands {} pop {} ===",
        info.format_version,
        w.year(),
        w.bands.len(),
        w.total_pop()
    );
    println!(
        "axe_term copper {:.3} bronze {:.3} iron {:.3} steel {:.3} | dur {:.3}/{:.3}/{:.3}/{:.3} | cut {:.3}/{:.3}/{:.3}/{:.3}",
        term(Mat::Copper),
        term(Mat::Bronze),
        term(Mat::Iron),
        term(Mat::Steel),
        durability(Mat::Copper),
        durability(Mat::Bronze),
        durability(Mat::Iron),
        durability(Mat::Steel),
        cut_q(Mat::Copper),
        cut_q(Mat::Bronze),
        cut_q(Mat::Iron),
        cut_q(Mat::Steel)
    );
    println!(
        "columns: fk/fr food gain steel-other kept/raw; lab/base/waste/pfail/m/dep/sk are other/steel; cur is other-steel at chain skill (positive keeps the other); cur1 is the same at maker skill 1; rank is other/steel/tie; gap is priced extra labour; k is fk/gap (p10/p50/p90) so steel ties when extra labour is scaled to k; g is gap/fk; wmax is the fraction steel's own labour must fall to; slack/nodep/matchd/m1 are % who would then take steel; wf/wp decision weights; rf raw farm share; fpc food per person; tv probe value steel-other; pref% probe prefers steel by >{PROBE_TIE}; re is the same probe with plants and farm at the band's weights; stock is copper/iron/steel/charcoal; mis% is cur disagreeing with food-minus-priced-labour"
    );

    let mut held = [0u64; 9];
    let mut other_held = 0u64;
    let mut n_avail = 0u64;
    let mut pop_avail = 0u64;
    let mut m_sum = 0.0f64;
    let mut m_min = 2.0f32;
    let mut m_max = -1.0f32;
    let mut m_one = 0u64;
    let mut groups: Vec<(&str, Vec<AxRow>)> = Vec::new();
    // parallel to TAGS, plus "steel-vs-best" at the end
    let mut buckets: Vec<Vec<AxRow>> = (0..TAGS.len() + 1).map(|_| Vec::new()).collect();

    for b in &w.bands {
        let tk = &b.d.tk;
        if !tk.avail.has(steel_it) {
            continue;
        }
        n_avail += 1;
        pop_avail += b.pop as u64;
        let m_steel = tk.chain_m(steel_it);
        m_sum += m_steel as f64;
        m_min = m_min.min(m_steel);
        m_max = m_max.max(m_steel);
        if m_steel == 1.0 {
            m_one += 1;
        }
        let kit = b.d.kit;
        let held_it = kit.axe;
        match tag_ix(tag_of(held_it)) {
            Some(i) => held[i] += 1,
            None => other_held += 1,
        }
        let view = w.band_view(b);
        let known = &b.known;
        let mut axes: Vec<(Item, Scored)> = tk
            .avail
            .iter()
            .filter(|&it| form(it) == Form::Axe)
            .map(|it| (it, score_axe(&view, &kit, tk, known, it)))
            .collect();
        let Some(steel_sc) = axes.iter().find(|(it, _)| *it == steel_it).map(|(_, s)| *s) else {
            continue;
        };
        let stock = [stock_at(b, Mat::Copper), stock_at(b, Mat::Iron), stock_at(b, Mat::Steel), stock_at(b, Mat::Charcoal)];
        let pop = b.pop.max(1) as f32;
        if held_it != steel_it {
            let Some(other_sc) = axes.iter().find(|(it, _)| *it == held_it).map(|(_, s)| *s) else {
                continue;
            };
            if held_it == NONE || form(held_it) != Form::Axe {
                continue;
            }
            let row = pair_row(&view, &kit, tk, known, b.sow, b.d.spare, pop, stock, held_it, steel_it, other_sc, steel_sc);
            if let Some(i) = tag_ix(tag_of(held_it)) {
                buckets[i].push(row);
            }
        } else {
            axes.retain(|(it, _)| *it != steel_it);
            let Some((alt_it, alt_sc)) = axes.into_iter().max_by(|a, b| a.1.cur_m.partial_cmp(&b.1.cur_m).unwrap_or(std::cmp::Ordering::Equal)) else {
                continue;
            };
            let row = pair_row(&view, &kit, tk, known, b.sow, b.d.spare, pop, stock, alt_it, steel_it, alt_sc, steel_sc);
            buckets[TAGS.len()].push(row);
        }
    }

    let mean_m = if n_avail == 0 { 0.0 } else { m_sum / n_avail as f64 };
    println!(
        "census steel-available {n_avail} pop {pop_avail} of bands {} | held none {} flint {} stone {} obsidian {} bone {} copper {} bronze {} iron {} steel {} other {other_held} | steel chain_m mean {mean_m:.3} min {m_min:.3} max {m_max:.3} frac_at_1 {:.1}%",
        w.bands.len(),
        held[0], held[1], held[2], held[3], held[4], held[5], held[6], held[7], held[8],
        if n_avail == 0 { 0.0 } else { 100.0 * m_one as f64 / n_avail as f64 }
    );
    for (i, name) in TAGS.iter().enumerate() {
        if *name == "steel" || buckets[i].is_empty() {
            continue;
        }
        groups.push((*name, std::mem::take(&mut buckets[i])));
    }
    if !buckets[TAGS.len()].is_empty() {
        groups.push(("steel-vs-best", std::mem::take(&mut buckets[TAGS.len()])));
    }
    for (name, rows) in &groups {
        emit_splits(name, rows);
    }
}

/// Copper, iron and the other axes a band keeps while a steel axe is makeable,
/// scored at chain skill, with the break-even on the extra labour.
#[test]
#[ignore]
fn diag_axe_breakeven() {
    let path = std::env::var("DIAG_SAVE").unwrap_or_else(|_| crate::tests_diag::DIAG_SAVE_DEFAULT.to_string());
    report_save(&path);
    if std::path::Path::new(FRESH_SAVE).is_file() {
        report_save(FRESH_SAVE);
    } else {
        println!("fresh save absent: {FRESH_SAVE}");
    }
}
