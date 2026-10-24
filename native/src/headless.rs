//! Headless multi-seed runner: runs each seed in turn (one process, sequential) and prints
//! per-seed stats plus cross-seed comparisons.
use crate::cli::{make_config, make_params, Args};
use civ_core::analysis::{order_distance, summarize, Summary};
use civ_core::materials::Item;
use civ_core::World;
use std::time::Instant;

pub fn run(a: &Args) {
    let mut sums: Vec<Summary> = Vec::new();
    let mut orders: Vec<Vec<Item>> = Vec::new();
    for seed in a.seeds.0..=a.seeds.1 {
        let t0 = Instant::now();
        let mut w = if let Some(p) = &a.load {
            let b = std::fs::read(p).expect("read save");
            World::load_bytes(&b).expect("load").0
        } else {
            World::new(make_config(a, seed), make_params(a))
        };
        let t_gen = t0.elapsed().as_secs_f32();
        let ticks = a.years * 4;
        for t in 0..ticks {
            w.step();
            if a.report > 0 && (t + 1) % (a.report * 4) == 0 {
                if let Some(h) = w.history.last() {
                    eprintln!(
                        "  seed {seed} Y{:>4} pop {:>6} bands {:>4} items world {:>3} mean {:>5.1} max {:>3} fire {:>4} exp {:>4} inv {:>3} learn {:>4} forget {:>3}",
                        h.year, h.pop, h.bands, h.items_world, h.items_mean, h.items_max, h.with_fire, h.experiments, h.inventions, h.transfers, h.forgets
                    );
                }
            }
            if w.bands.is_empty() {
                break;
            }
        }
        let el = t0.elapsed().as_secs_f32();
        let s = summarize(&w);
        if let Some(p) = &a.save {
            let bytes = w.save_bytes(None, env!("CARGO_PKG_VERSION"), "headless").unwrap();
            std::fs::write(p, bytes).unwrap();
        }
        if a.json {
            println!("{}", serde_json::to_string(&s).unwrap());
        } else {
            println!(
                "seed {seed}: Y{:.0} pop {} bands {} range {} cells | items now {} (mean/band {:.1}) | stage now: {}, ever: {} | exp {} inv {} learned {} forgot {} | splits {} died {} | self-invented {:.0}% | {:.1}s (gen {:.1}s)",
                s.years, s.pop, s.bands, s.range_cells, s.items_now, s.items_mean, s.stage_now, s.stage_ever, s.experiments, s.inventions,
                s.transfers, s.forgets, s.splits, s.deaths, 100.0 * s.self_invented_frac, el, t_gen
            );
            for f in &s.firsts {
                if !a.verbose && f.bands_now == 0 && f.inventions <= 1 && f.transfers == 0 {
                    continue;
                }
                println!(
                    "    Y{:>5.1} {:<34} inv {:>3} taught {:>4} bands now {:>4} cells {:>5} r {:>5} speed {:>5}  [{}]",
                    f.year_first,
                    f.name,
                    f.inventions,
                    f.transfers,
                    f.bands_now,
                    f.cells_reached,
                    f.diffusion_r.map(|r| format!("{r:.2}")).unwrap_or("-".into()),
                    f.spread_speed.map(|r| format!("{r:.1}")).unwrap_or("-".into()),
                    f.how
                );
            }
            let cs: Vec<String> = s
                .continents
                .iter()
                .map(|c| match c.landfall_year {
                    Some(y) => format!("#{} {}c Y{:.0} pop {}", c.id, c.cells, y, c.pop_now),
                    None => format!("#{} {}c never", c.id, c.cells),
                })
                .collect();
            println!(
                "    continents settled {}/{}: {} | islands reached {}/{}",
                s.continents.iter().filter(|c| c.landfall_year.is_some()).count(),
                s.continents.len(),
                cs.join(", "),
                s.islands_reached,
                s.islands
            );
            let t = &s.trade;
            let ft: Vec<String> = t.first_traded.iter().map(|(y, m)| format!("{m} Y{y:.0}")).collect();
            println!(
                "    trade: {} exchanges, volume {:.0}, food shared {:.0}, mean dist {:.1} | last yr {} trades, {} partnerships, carried metal/obsidian {:.1} cells from source (max ever {:.0}) | first: {}",
                t.trades, t.volume, t.food_shared, t.mean_dist, t.trades_last, t.partnerships_last, t.goods_dist_mean_last, t.goods_dist_max_ever, ft.join(", ")
            );
            if !s.lost_worldwide.is_empty() {
                println!("    lost worldwide: {}", s.lost_worldwide.join(", "));
            }
        }
        orders.push(s.firsts.iter().map(|f| f.item).collect());
        sums.push(s);
    }
    if sums.len() > 1 && !a.json {
        let mut ds = Vec::new();
        for i in 0..orders.len() {
            for j in i + 1..orders.len() {
                if let Some(d) = order_distance(&orders[i], &orders[j]) {
                    ds.push(d);
                }
            }
        }
        let mean = ds.iter().sum::<f32>() / ds.len().max(1) as f32;
        println!("\n== {} seeds, {} years", sums.len(), a.years);
        println!("discovery-order distance between seeds (Kendall tau, 0 same .. 1 reversed): mean {:.2} over {} pairs", mean, ds.len());
        for st in civ_core::analysis::STAGES {
            let n = sums.iter().filter(|s| s.stage_ever == st).count();
            let m = sums.iter().filter(|s| s.stage_now == st).count();
            if n + m > 0 {
                println!("  stage {st:<14} reached (highest ever) {n:>3}   holding now {m:>3}");
            }
        }
        let alive = sums.iter().filter(|s| s.pop > 0).count();
        println!("  alive at end: {alive}/{}", sums.len());
        let rs: Vec<f32> = sums.iter().flat_map(|s| s.firsts.iter().filter_map(|f| f.diffusion_r)).collect();
        if !rs.is_empty() {
            println!("  distance-vs-arrival correlation over {} item spreads: mean {:.2}", rs.len(), rs.iter().sum::<f32>() / rs.len() as f32);
        }
        let mut firsts_by_item: std::collections::BTreeMap<String, Vec<f32>> = Default::default();
        for s in &sums {
            for f in &s.firsts {
                firsts_by_item.entry(f.name.clone()).or_default().push(f.year_first);
            }
        }
        println!("  items found (in how many seeds; median year of first discovery):");
        let mut v: Vec<_> = firsts_by_item.into_iter().collect();
        v.sort_by_key(|(_, ys)| std::cmp::Reverse(ys.len()));
        for (name, mut ys) in v {
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            println!("    {:<36} {:>3} seeds  median Y{:>6.1}", name, ys.len(), ys[ys.len() / 2]);
        }
    }
}
