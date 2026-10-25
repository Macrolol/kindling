//! Read-only diagnosis of a saved world: `cargo run --release --example inspect_save -- file.civsave`
use civ_core::knowledge::Registry;
use civ_core::map::{DEPOSITS, D_COPPER, D_TIN};
use civ_core::materials::*;
use civ_core::physics::{apply, canonical, Action, Ctx};
use civ_core::world::World;
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let path = std::env::args().nth(1).expect("save path");
    let bytes = std::fs::read(&path).unwrap();
    let (mut w, _view, info) = World::load_bytes(&bytes).expect("load");
    w.refresh_all_toolkits();
    println!("== {path}");
    println!("format v{} app {} created {} migrated_from {:?} warning {:?}", info.format_version, info.app_version, info.created_at, info.migrated_from, info.warning);
    let yr = w.tick as f32 / 4.0;
    println!("year {:.1} | map {}x{} seed {} founders {} | bands {} pop {}", yr, w.map.w, w.map.h, w.cfg.seed, w.cfg.founders, w.bands.len(), w.bands.iter().map(|b| b.pop).sum::<u32>());
    println!("\n-- settings now: {:?}", w.params);
    println!("-- settings change log ({} entries):", w.settings_log.len());
    for c in &w.settings_log {
        println!("   Y{:.1} {} {} -> {}", c.tick as f32 / 4.0, c.name, c.old, c.new);
    }
    println!("\n-- world firsts ({}):", w.firsts.len());
    for f in &w.firsts {
        println!("   Y{:>7.1} {}", f.tick as f32 / 4.0, item_name(f.item));
    }
    // what exists now
    let mut have: BTreeMap<Item, u32> = BTreeMap::new();
    for b in &w.bands {
        for it in b.d.tk.avail.iter() {
            *have.entry(it).or_default() += 1;
        }
    }
    let known_items: BTreeSet<Item> = w.firsts.iter().map(|f| f.item).collect();
    println!("\n-- items any band can make/has now ({}):", have.len());
    let mut v: Vec<_> = have.iter().filter(|(i, _)| form(**i) != Form::Raw).collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (i, n) in v {
        println!("   {:<32} {} bands", item_name(*i), n);
    }
    let lost: Vec<String> = known_items.iter().filter(|i| !have.contains_key(i)).map(|&i| item_name(i)).collect();
    println!("-- found once but held by no band now: {}", lost.join(", "));

    // history by century
    println!("\n-- per century: mean pop, bands, experiments/yr, exp per band-yr, inventions, of which world-firsts, learned, forgot, items_world(end), items_mean(end)");
    let mut firsts_by_c: BTreeMap<u32, u32> = BTreeMap::new();
    for f in &w.firsts {
        *firsts_by_c.entry(f.tick / 400).or_default() += 1;
    }
    for c in w.history.chunks(100) {
        let n = c.len() as f32;
        let pop = c.iter().map(|h| h.pop as f32).sum::<f32>() / n;
        let bands = c.iter().map(|h| h.bands as f32).sum::<f32>() / n;
        let ex = c.iter().map(|h| h.experiments as f32).sum::<f32>();
        let inv: u32 = c.iter().map(|h| h.inventions).sum();
        let tr: u32 = c.iter().map(|h| h.transfers).sum();
        let fo: u32 = c.iter().map(|h| h.forgets).sum();
        let l = c.last().unwrap();
        let cent = c[0].year / 100;
        println!(
            "   Y{:>4}-{:<4} pop {:>7.0} bands {:>5.0} exp/yr {:>7.0} per band-yr {:>5.2} inv {:>5} firsts {:>3} learned {:>7} forgot {:>7} items {:>3} mean {:>5.1}",
            c[0].year, l.year, pop, bands, ex / n, ex / n / bands.max(1.0), inv, firsts_by_c.get(&cent).copied().unwrap_or(0), tr, fo, l.items_world, l.items_mean
        );
    }

    // deposits
    let m = &w.map;
    let mw = m.w;
    println!("\n-- deposits (land cells):");
    for (d, mm) in DEPOSITS {
        let cells: Vec<usize> = (0..mw * m.h).filter(|&i| m.has(i, d)).collect();
        let mut banded = 0;
        let mut pop = 0;
        let mut dists = Vec::new();
        for &c in &cells {
            let (x, y) = ((c % mw) as i32, (c / mw) as i32);
            let mut best = i32::MAX;
            for b in &w.bands {
                let dd = (b.x as i32 - x).abs().max((b.y as i32 - y).abs());
                best = best.min(dd);
                if dd == 0 {
                    banded += 1;
                    pop += b.pop;
                }
            }
            dists.push(best);
        }
        dists.sort();
        let med = dists.get(dists.len() / 2).copied().unwrap_or(-1);
        println!("   {:<10?} {:>5} cells | band-on-cell {:>4} (pop {:>6}) | nearest band median {} cells, within 3 cells: {}", mm, cells.len(), banded, pop, med, dists.iter().filter(|&&d| d <= 3).count());
    }
    // which bands could even try smelting: on/holding ore, with what fires/kilns
    let cu_ore = item(Form::Raw, Mat::CopperOre);
    let tin_ore = item(Form::Raw, Mat::TinOre);
    let mut s_ore = 0;
    let mut s_ore_char = 0;
    let mut s_ore_char_kiln = 0;
    let mut s_tin = 0;
    let mut kilns_seen: BTreeMap<String, u32> = BTreeMap::new();
    let mut fires_seen: BTreeMap<String, u32> = BTreeMap::new();
    for b in &w.bands {
        let a = &b.d.tk.avail;
        for it in a.iter() {
            if form(it) == Form::Kiln {
                *kilns_seen.entry(item_name(it)).or_default() += 1;
            }
            if form(it) == Form::Fire {
                *fires_seen.entry(item_name(it)).or_default() += 1;
            }
        }
        if a.has(cu_ore) {
            s_ore += 1;
            let charf = a.iter().any(|i| form(i) == Form::Fire && mat(i).props().fuel_t >= 900.0);
            if charf {
                s_ore_char += 1;
                if a.iter().any(|i| form(i) == Form::Kiln) {
                    s_ore_char_kiln += 1;
                }
            }
        }
        if a.has(tin_ore) {
            s_tin += 1;
        }
    }
    println!("   fires held: {:?}", fires_seen);
    println!("   kilns held: {:?}", kilns_seen);
    println!("   bands with copper ore at hand {} | + a >=900C fire {} | + a kiln {} | tin ore at hand {}", s_ore, s_ore_char, s_ore_char_kiln, s_tin);
    for mm in [Mat::Wood, Mat::Charcoal] {
        for l in [None, Some(Mat::Stone), Some(Mat::Clay)] {
            println!("   fire_temp({:?}, {:?}) = {:.0}  (copper ore reduce_t {:.0}, tin ore {:.0}, bronze melt {:.0})", mm, l, fire_temp(mm, l), Mat::CopperOre.props().reduce_t, Mat::TinOre.props().reduce_t, Mat::Bronze.props().melt);
        }
    }
    // ore stock in band stores (traded/carried) and how far from source
    let mut held = [0.0f32; NMAT];
    let mut far = [0.0f32; NMAT];
    for b in &w.bands {
        for (mi, &s) in b.stock.iter().enumerate() {
            if s > 0.05 && mi < NMAT {
                held[mi] += s;
                if let Some(&o) = b.origin.get(mi) {
                    if o != u32::MAX {
                        let (ox, oy) = ((o as usize % mw) as i32, (o as usize / mw) as i32);
                        let d = (b.x as i32 - ox).abs().max((b.y as i32 - oy).abs()) as f32;
                        far[mi] = far[mi].max(d);
                    }
                }
            }
        }
    }
    for mm in [Mat::CopperOre, Mat::TinOre, Mat::Copper, Mat::Tin, Mat::Obsidian, Mat::Charcoal] {
        println!("   stock held now {:<10?} {:>8.1} units, farthest from source {} cells", mm, held[mm as usize], far[mm as usize]);
    }
    let t = civ_core::analysis::summarize(&w);
    let ft: Vec<String> = t.trade.first_traded.iter().map(|(y, m)| format!("{m} Y{y:.0}")).collect();
    println!("   first traded: {}", ft.join(", "));
    println!("   max carried metal/obsidian distance ever {:.0}", t.trade.goods_dist_max_ever);

    // reachability: everything physics allows from all raw materials in the world, under all fires/kilns
    reach(&w.reg, &known_items);
    let _ = (D_COPPER, D_TIN);
}

fn reach(reg: &Registry, known: &BTreeSet<Item>) {
    let mut set: BTreeSet<Item> = (0..NRAW).map(|k| item(Form::Raw, ALL_MATS[k])).collect();
    set.insert(item(Form::Fire, Mat::Wood));
    let mut best: BTreeMap<Item, (f32, String, usize)> = BTreeMap::new();
    for round in 0..8 {
        let v: Vec<Item> = set.iter().copied().collect();
        let mut new = Vec::new();
        let mut consider = |a: Action, inp: [Item; 4], new: &mut Vec<Item>| {
            let inp = canonical(a, inp);
            for ctx in [Ctx { dryness: 0.6, water: true }, Ctx { dryness: 0.6, water: false }] {
                if let Some(o) = apply(a, &inp, &ctx) {
                    let e = best.entry(o.out).or_insert((0.0, String::new(), round));
                    if o.p > e.0 {
                        let names: Vec<String> = inp.iter().filter(|&&i| i != NONE).map(|&i| item_name(i)).collect();
                        *e = (o.p, format!("{} {}", a.name(), names.join(" + ")), e.2);
                    }
                    if !set.contains(&o.out) {
                        new.push(o.out);
                    }
                }
            }
        };
        let fires: Vec<Item> = v.iter().copied().filter(|&i| form(i) == Form::Fire).collect();
        let kilns: Vec<Item> = v.iter().copied().filter(|&i| form(i) == Form::Kiln).collect();
        let pool: Vec<Item> = v.iter().copied().filter(|&i| !matches!(form(i), Form::Fire | Form::Kiln | Form::Cooked)).collect();
        let mut cpool = pool.clone();
        cpool.extend(fires.iter().copied());
        for &t in &pool {
            consider(Action::Shape, [t, NONE, NONE, NONE], &mut new);
            for &tool in &pool {
                if tool != t {
                    consider(Action::Shape, [t, tool, NONE, NONE], &mut new);
                }
            }
        }
        for (i, &a) in cpool.iter().enumerate() {
            for (j, &b) in cpool.iter().enumerate().skip(i + 1) {
                consider(Action::Combine, [a, b, NONE, NONE], &mut new);
                for &c in cpool.iter().skip(j + 1) {
                    consider(Action::Combine, [a, b, c, NONE], &mut new);
                }
            }
        }
        for &f in &fires {
            for k in std::iter::once(NONE).chain(kilns.iter().copied()) {
                for (i, &a) in pool.iter().enumerate() {
                    consider(Action::Heat, [a, NONE, f, k], &mut new);
                    for &b in pool.iter().skip(i + 1) {
                        consider(Action::Heat, [a, b, f, k], &mut new);
                    }
                }
            }
        }
        let before = set.len();
        set.extend(new);
        if set.len() == before {
            break;
        }
    }
    let _ = reg;
    println!("\n-- reachable by physics from the world's raw materials: {} items (non-raw); never found in this world:", best.len());
    for (it, (p, how, _)) in &best {
        if !known.contains(it) && form(*it) != Form::Raw || (form(*it) == Form::Raw && !known.contains(it) && !matches!(mat(*it), m if (m as usize) < NRAW)) {
            println!("   {:<32} p {:.2}  via {}", item_name(*it), p, how);
        }
    }
}
