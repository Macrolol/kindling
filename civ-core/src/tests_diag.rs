//! Ignored diagnostics. Not part of the sim; `cargo test` skips them.
//!
//! `diag_copper_over_steel_axe` (skills step 3 fix 2): bands whose fitted kit
//! holds a copper axe while a steel axe is makeable. Numbers only — the test
//! does not assert a ranking.
//!
//! `DIAG_SAVE` is the snapshot path. If it is unset, `DIAG_SAVE_DEFAULT` is used.

use crate::band::{loss_guard, weapon_q, Kit, KIT_UPKEEP};
use crate::effects::{
    harvest_for, kit_yields, make_kit_among_view, maker_p_fail, maker_waste, recipe_info, slot_score, tool_effects, BandView, LABOUR_SHARE,
    WORK_SHARE,
};
use crate::knowledge::{Known, Toolkit};
use crate::materials::*;
use crate::World;

/// Seed 6, 256x160, year 350, main `f8d80dd`. Replaced only if that snapshot
/// has too few copper-axe-over-steel bands to measure.
const DIAG_SAVE_DEFAULT: &str = "/workspace/step3-saves/main_seed6_y350_256x160.civsave";

/// Probe tie and fight weight, copied from `probe_diffusion` so this test does
/// not reach into that module. A gap within `PROBE_TIE` is a tie there.
const PROBE_TIE: f32 = 0.02;
const PROBE_FIGHT_W: f32 = 0.1;

/// Axe is best[] slot 4 (`CHOOSE_ORDER`), payoff index 3.
const AXE_SLOT: usize = 4;

const SLOT_NAMES: [&str; 12] = [
    "hunt", "cut", "point", "axe", "boat", "net", "trap", "bow", "shield", "outrigger", "plank", "ship",
];

#[derive(Clone, Copy, Default)]
struct Agg {
    n: u64,
    food_cu: f64,
    food_st: f64,
    labour_cu: f64,
    labour_st: f64,
    cur_cu: f64,
    cur_st: f64,
    rank_cu: u64,
    rank_st: u64,
    rank_tie: u64,
    depth_cu: f64,
    depth_st: f64,
    ueff: f64,
    skill_cu: f64,
    skill_st: f64,
    qual_cu: f64,
    qual_st: f64,
    d_plants: f64,
    d_farm: f64,
    d_food: f64,
    d_up: f64,
    d_fight: f64,
    d_protect: f64,
    d_other: f64,
    d_tv: f64,
    d_tv_forced: f64,
    probe_st: u64,
    own_cu_probe_st: u64,
    forced_changed: u64,
    forced_none: u64,
    slot_chg: [u64; 12],
    stale: u64,
    raw_plant: f64,
    raw_farm: f64,
    w_plant: f64,
    w_farm: f64,
    sow: f64,
    score_mismatch: u64,
}

struct Sample {
    food_cu: f32,
    food_st: f32,
    labour_cu: f32,
    labour_st: f32,
    cur_cu: f32,
    cur_st: f32,
    rank: i8,
    depth_cu: f32,
    depth_st: f32,
    ueff: f32,
    skill_cu: f32,
    skill_st: f32,
    qual_cu: f32,
    qual_st: f32,
    d_plants: f32,
    d_farm: f32,
    d_food: f32,
    d_up: f32,
    d_fight: f32,
    d_protect: f32,
    d_other: f32,
    d_tv: f32,
    d_tv_forced: f32,
    probe_st: bool,
    forced_changed: bool,
    forced_none: bool,
    slot_chg: [bool; 12],
    stale: bool,
    raw_plant: f32,
    raw_farm: f32,
    w_plant: f32,
    w_farm: f32,
    sow: f32,
    score_mismatch: bool,
}

fn acc(a: &mut Agg, s: &Sample) {
    a.n += 1;
    a.food_cu += s.food_cu as f64;
    a.food_st += s.food_st as f64;
    a.labour_cu += s.labour_cu as f64;
    a.labour_st += s.labour_st as f64;
    a.cur_cu += s.cur_cu as f64;
    a.cur_st += s.cur_st as f64;
    match s.rank {
        1 => a.rank_cu += 1,
        -1 => a.rank_st += 1,
        _ => a.rank_tie += 1,
    }
    a.depth_cu += s.depth_cu as f64;
    a.depth_st += s.depth_st as f64;
    a.ueff += s.ueff as f64;
    a.skill_cu += s.skill_cu as f64;
    a.skill_st += s.skill_st as f64;
    a.qual_cu += s.qual_cu as f64;
    a.qual_st += s.qual_st as f64;
    a.d_plants += s.d_plants as f64;
    a.d_farm += s.d_farm as f64;
    a.d_food += s.d_food as f64;
    a.d_up += s.d_up as f64;
    a.d_fight += s.d_fight as f64;
    a.d_protect += s.d_protect as f64;
    a.d_other += s.d_other as f64;
    a.d_tv += s.d_tv as f64;
    a.d_tv_forced += s.d_tv_forced as f64;
    if s.probe_st {
        a.probe_st += 1;
        if s.rank >= 0 {
            a.own_cu_probe_st += 1;
        }
    }
    if s.forced_changed {
        a.forced_changed += 1;
    }
    if s.forced_none {
        a.forced_none += 1;
    }
    for j in 0..12 {
        if s.slot_chg[j] {
            a.slot_chg[j] += 1;
        }
    }
    if s.stale {
        a.stale += 1;
    }
    a.raw_plant += s.raw_plant as f64;
    a.raw_farm += s.raw_farm as f64;
    a.w_plant += s.w_plant as f64;
    a.w_farm += s.w_farm as f64;
    a.sow += s.sow as f64;
    if s.score_mismatch {
        a.score_mismatch += 1;
    }
}

fn mean(s: f64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { s / n as f64 }
}

fn pct(k: u64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { 100.0 * k as f64 / n as f64 }
}

/// Pieces of `probe_diffusion::true_value`. `plants` and `farm` stay the raw
/// multipliers. `food` is the unweighted sum (five multipliers plus digest)
/// times the worst food-slot keep `1 - p_fail`. `upkeep` is `KIT_UPKEEP` times
/// `maker_waste` at each distinct payoff tool's chain skill. At chain skill 1
/// both factors are 1. `other` is boat soundness plus fine work.
struct Tv {
    food: f32,
    plants: f32,
    farm: f32,
    protect: f32,
    other: f32,
    upkeep: f32,
    fight: f32,
}

impl Tv {
    fn value(&self) -> f32 {
        self.food + self.protect + self.other - self.upkeep + self.fight
    }
}

fn tv_of(k: &Kit, tk: &Toolkit, known: &[Known]) -> Tv {
    let mut up = 0.0;
    let mut seen: [Item; 12] = [NONE; 12];
    let mut keep = 1.0;
    for j in 0..12 {
        let it = k.slot_item(j);
        if it == NONE {
            continue;
        }
        // Same food slots as `probe_diffusion::true_value`: hunt, cut, point, axe, net, trap.
        if matches!(j, 0 | 1 | 2 | 3 | 5 | 6) {
            let one = 1.0 - maker_p_fail(tk.chain_m(it));
            if one < keep {
                keep = one;
            }
        }
        if seen[..j].contains(&it) {
            continue;
        }
        seen[j] = it;
        let d = recipe_info(tk, known, it).2;
        let base = KIT_UPKEEP * (1.15 - durability(mat(it))) * (1.0 + 0.3 * d as f32);
        up += base * maker_waste(tk.chain_m(it));
    }
    let mults = k.plants_m + k.game_m + k.fish_m + k.small_m + k.farm_m;
    let digest: f32 = k.digest.iter().sum();
    Tv {
        food: (mults + digest) * keep,
        plants: k.plants_m,
        farm: k.farm_m,
        protect: k.protect,
        other: k.boat_q + k.fine,
        upkeep: up,
        fight: PROBE_FIGHT_W * (weapon_q(k) + 1.0 - loss_guard(k)),
    }
}

fn with_axe(k: &Kit, axe: Item) -> Kit {
    let mut c = *k;
    c.axe = axe;
    c
}

fn without(s: ItemSet, x: Item) -> ItemSet {
    let mut o = ItemSet::default();
    for i in s.iter() {
        if i != x {
            o.add(i);
        }
    }
    o
}

/// Probe `force`: drop whatever the axe slot picks until the steel axe is the pick.
fn force_steel(view: &BandView, tk: &Toolkit, known: &[Known], steel: Item) -> Option<Kit> {
    let mut avail = tk.avail;
    for _ in 0..16 {
        let k = make_kit_among_view(view, &avail, tk, known);
        if k.axe == steel {
            return Some(k);
        }
        if k.axe == NONE {
            return None;
        }
        avail = without(avail, k.axe);
    }
    None
}

fn priced(view: &BandView, food: f32, labour: f32) -> f32 {
    let pop = view.pop.max(1.0);
    let price = view.food.max(1e-6) / (WORK_SHARE * LABOUR_SHARE * pop);
    food - price * labour * LABOUR_SHARE * pop
}

fn print_agg(name: &str, a: &Agg) {
    let n = a.n;
    if n == 0 {
        println!("{name:18} n=0");
        return;
    }
    let m = |s: f64| mean(s, n);
    println!(
        "{name:18} n={n:6}  raw plant/farm {:.3}/{:.3}  w plant/farm {:.3}/{:.3}  sow {:.3}",
        m(a.raw_plant),
        m(a.raw_farm),
        m(a.w_plant),
        m(a.w_farm),
        m(a.sow)
    );
    println!(
        "  food st-cu {:+.5}  labour cu/st {:.5}/{:.5}  cur cu-st {:+.5}  rank cu/st/tie {}/{}/{} ({:.1}/{:.1}/{:.1}%)",
        m(a.food_st - a.food_cu),
        m(a.labour_cu),
        m(a.labour_st),
        m(a.cur_cu - a.cur_st),
        a.rank_cu,
        a.rank_st,
        a.rank_tie,
        pct(a.rank_cu, n),
        pct(a.rank_st, n),
        pct(a.rank_tie, n)
    );
    println!(
        "  depth cu/st {:.2}/{:.2}  u_eff(Axe) {:.3}  skill cu/st {:.3}/{:.3}  qual cu/st {:.3}/{:.3}",
        m(a.depth_cu),
        m(a.depth_st),
        m(a.ueff),
        m(a.skill_cu),
        m(a.skill_st),
        m(a.qual_cu),
        m(a.qual_st)
    );
    println!(
        "  probe same-slot st-cu: food {:+.5} (plants {:+.5} farm {:+.5}) upkeep {:+.5} fight {:+.5} protect {:+.5} other {:+.5} value {:+.5}",
        m(a.d_food),
        m(a.d_plants),
        m(a.d_farm),
        m(a.d_up),
        m(a.d_fight),
        m(a.d_protect),
        m(a.d_other),
        m(a.d_tv)
    );
    println!(
        "  probe prefers steel (same-slot value > {PROBE_TIE}) {:.1}%  of which own still ranks copper or tie {:.1}%  forced value st-cu {:+.5}",
        pct(a.probe_st, n),
        pct(a.own_cu_probe_st, n),
        m(a.d_tv_forced)
    );
    println!(
        "  forced steel changes another slot {:.1}%  force failed {:.1}%  stale axe {:.1}%  score mismatch {:.1}%",
        pct(a.forced_changed, n),
        pct(a.forced_none, n),
        pct(a.stale, n),
        pct(a.score_mismatch, n)
    );
}

/// Copper axe held while a steel axe is in `tk.avail`.
///
/// Harvest split, on the saved season's raw harvest (not the smoothed weights):
/// "farms" is farm share >= 0.10 or `sow >= 0.2`; "many plants" is plant share
/// >= 0.35. "either" is the union, "neither" is few plants and little farming.
///
/// Own score is `slot_score` cur of each axe in the fitted kit with only that
/// slot swapped (the same context `choose_food` uses). Positive `cur cu-st`
/// means the band ranks the copper axe above the steel one. The probe parts
/// are `true_value`'s: food times the worst food-slot keep, `KIT_UPKEEP` times
/// `maker_waste` at the chain skill, and `PROBE_FIGHT_W` times fight. Plants
/// and farm in the print stay the raw multipliers. `value` is steel minus copper.
#[test]
#[ignore]
fn diag_copper_over_steel_axe() {
    let path = std::env::var("DIAG_SAVE").unwrap_or_else(|_| DIAG_SAVE_DEFAULT.to_string());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (w, _, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load {path}: {e}"));
    let copper = item(Form::Axe, Mat::Copper);
    let steel = item(Form::Axe, Mat::Steel);
    let term = |m: Mat| 0.5 * cut_q(m) + 0.5 * durability(m);
    println!(
        "save {path} format v{} year {:.0} bands {} pop {}",
        info.format_version,
        w.year(),
        w.bands.len(),
        w.total_pop()
    );
    println!(
        "copper cut_q {:.3} dur {:.3} axe_term {:.3} | steel cut_q {:.3} dur {:.3} axe_term {:.3}",
        cut_q(Mat::Copper),
        durability(Mat::Copper),
        term(Mat::Copper),
        cut_q(Mat::Steel),
        durability(Mat::Steel),
        term(Mat::Steel)
    );
    println!(
        "order: hunt, cut, point, axe, net, trap, then hulls. axe deps (food_deps_changed slot 4): cut, pitch. cut is scored before axe in both passes; a hull that turns pitch on is chosen in pass 1, and pass 2 rescores the axe after that. stale = the held axe is not slot_score's strict argmax in the final kit (a tie keeps the earlier item id; copper's id is lower)."
    );

    let mut all = Agg::default();
    let mut farms = Agg::default();
    let mut plants = Agg::default();
    let mut either = Agg::default();
    let mut neither = Agg::default();

    for b in &w.bands {
        let kit = b.d.kit;
        if kit.axe != copper {
            continue;
        }
        let tk = &b.d.tk;
        if !tk.avail.has(steel) {
            continue;
        }
        let view = w.band_view(b);
        let known = &b.known;
        let info_of = |it: Item| recipe_info(tk, known, it);
        let mask = |it: Item| tk.via(it).map(|v| v.1).unwrap_or(0);
        let fx_cu = tool_effects(&view, &kit, AXE_SLOT, copper, &info_of, mask(copper));
        let fx_st = tool_effects(&view, &kit, AXE_SLOT, steel, &info_of, mask(steel));
        let cur_cu = slot_score(&view, &kit, AXE_SLOT, copper, &info_of).0;
        let cur_st = slot_score(&view, &kit, AXE_SLOT, steel, &info_of).0;
        let rank = if cur_cu > cur_st {
            1
        } else if cur_st > cur_cu {
            -1
        } else {
            0
        };
        let (skill_cu, qual_cu, depth_cu) = info_of(copper);
        let (skill_st, qual_st, depth_st) = info_of(steel);
        let ueff = view.user[Form::Axe as usize];

        let y_cu = kit_yields(&with_axe(&kit, copper), &info_of, &view.user);
        let y_st = kit_yields(&with_axe(&kit, steel), &info_of, &view.user);
        let p_cu = tv_of(&y_cu, tk, known);
        let p_st = tv_of(&y_st, tk, known);
        let d_tv = p_st.value() - p_cu.value();

        let (forced_none, forced_changed, slot_chg, d_tv_forced) = match force_steel(&view, tk, known, steel) {
            None => (true, false, [false; 12], 0.0),
            Some(fk) => {
                let mut chg = [false; 12];
                let mut any = false;
                for j in 0..12 {
                    if j == 3 {
                        continue;
                    }
                    if fk.slot_item(j) != kit.slot_item(j) {
                        chg[j] = true;
                        any = true;
                    }
                }
                let yf = kit_yields(&fk, &info_of, &view.user);
                let gap = tv_of(&yf, tk, known).value() - p_cu.value();
                (false, any, chg, gap)
            }
        };

        let mut best = f32::NEG_INFINITY;
        let mut best_it = NONE;
        for it in tk.avail.iter() {
            if form(it) != Form::Axe {
                continue;
            }
            let cur = slot_score(&view, &kit, AXE_SLOT, it, &info_of).0;
            if cur > best {
                best = cur;
                best_it = it;
            }
        }
        let stale = best_it != copper;

        let hsum: f32 = view.harvest.iter().copied().sum();
        let raw_plant = if hsum > 1e-6 { view.harvest[0] / hsum } else { 0.0 };
        let raw_farm = if hsum > 1e-6 { view.harvest[4] / hsum } else { 0.0 };
        let dg = kit.digest;
        let h = harvest_for(&view, &dg);
        let mut ws = 0.0;
        let mut ww = [0.0; 5];
        for r in 0..5 {
            ww[r] = h[r] * dg[r];
            ws += ww[r];
        }
        let w_plant = if ws > 0.0 { ww[0] / ws } else { 0.0 };
        let w_farm = if ws > 0.0 { ww[4] / ws } else { 0.0 };

        let mismatch = (priced(&view, fx_cu.food, fx_cu.labour) - cur_cu).abs() > 1e-3
            || (priced(&view, fx_st.food, fx_st.labour) - cur_st).abs() > 1e-3;

        let sample = Sample {
            food_cu: fx_cu.food,
            food_st: fx_st.food,
            labour_cu: fx_cu.labour,
            labour_st: fx_st.labour,
            cur_cu,
            cur_st,
            rank,
            depth_cu: depth_cu as f32,
            depth_st: depth_st as f32,
            ueff,
            skill_cu,
            skill_st,
            qual_cu,
            qual_st,
            d_plants: p_st.plants - p_cu.plants,
            d_farm: p_st.farm - p_cu.farm,
            d_food: p_st.food - p_cu.food,
            d_up: p_st.upkeep - p_cu.upkeep,
            d_fight: p_st.fight - p_cu.fight,
            d_protect: p_st.protect - p_cu.protect,
            d_other: p_st.other - p_cu.other,
            d_tv,
            d_tv_forced,
            probe_st: d_tv > PROBE_TIE,
            forced_changed,
            forced_none,
            slot_chg,
            stale,
            raw_plant,
            raw_farm,
            w_plant,
            w_farm,
            sow: b.sow,
            score_mismatch: mismatch,
        };
        let does_farm = raw_farm >= 0.10 || b.sow >= 0.2;
        let many_plants = raw_plant >= 0.35;
        acc(&mut all, &sample);
        if does_farm {
            acc(&mut farms, &sample);
        }
        if many_plants {
            acc(&mut plants, &sample);
        }
        if does_farm || many_plants {
            acc(&mut either, &sample);
        } else {
            acc(&mut neither, &sample);
        }
    }

    println!("--- copper axe held, steel axe available ---");
    println!("split: farms = raw farm share >= 0.10 or sow >= 0.2; many plants = raw plant share >= 0.35");
    print_agg("all", &all);
    print_agg("farms", &farms);
    print_agg("many plants", &plants);
    print_agg("farms or plants", &either);
    print_agg("few / little farm", &neither);
    println!("forced-steel other-slot changes (all):");
    for j in 0..12 {
        if j == 3 || all.slot_chg[j] == 0 {
            continue;
        }
        println!("  {} {} ({:.1}%)", SLOT_NAMES[j], all.slot_chg[j], pct(all.slot_chg[j], all.n));
    }
    println!("cases {}", all.n);
}
