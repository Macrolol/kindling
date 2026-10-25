//! Headless multi-seed runner: runs each seed in turn (one process, sequential) and prints
//! per-seed stats plus cross-seed comparisons.
use crate::cli::{make_config, make_params, Args};
use civ_core::analysis::{order_distance, order_distance_free, summarize, Summary};
use civ_core::materials::Item;
use civ_core::World;
use std::time::Instant;

pub fn run(a: &Args) {
    eprintln!("threads: {}", rayon::current_num_threads());
    let mut sums: Vec<Summary> = Vec::new();
    let mut orders: Vec<Vec<Item>> = Vec::new();
    let mut confl: Vec<ConflictStats> = Vec::new();
    for seed in a.seeds.0..=a.seeds.1 {
        let t0 = Instant::now();
        let mut w = if let Some(p) = &a.load {
            let b = std::fs::read(p).expect("read save");
            World::load_bytes(&b).expect("load").0
        } else {
            let w = World::new(make_config(a, seed), make_params(a));
            for m in crate::cli::check_starts(&w, &a.starts) {
                eprintln!("seed {seed}: {m}");
            }
            w
        };
        let t_gen = t0.elapsed().as_secs_f32();
        if std::env::var("KINDLING_PROF").is_ok() {
            w.prof = Some(Box::default());
        }
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
        if let Some(p) = &w.prof {
            let parts: Vec<String> = civ_core::world::Prof::NAMES.iter().zip(p.t.iter()).map(|(n, t)| format!("{}: {:.1}s", n.trim(), t)).collect();
            eprintln!("  seed {seed} profile: {} | pairs in range {:.1}M, meetings {:.1}M", parts.join(" | "), w.counters.pairs_seen as f64 / 1e6, w.counters.meetings as f64 / 1e6);
        }
        let s = summarize(&w);
        if let Some(p) = &a.save {
            let bytes = w.save_bytes(None, env!("CARGO_PKG_VERSION"), "headless").unwrap();
            std::fs::write(p, bytes).unwrap();
        }
        if a.json {
            println!("{}", serde_json::to_string(&s).unwrap());
        } else {
            println!(
                "seed {seed}: Y{:.0} pop {} bands {} range {} cells | items now {} (mean/band {:.1}) | stage now: {}, ever: {} | exp {} inv {} learned {} forgot {} | splits {} died {} | lineage-found {:.0}% of known, {:.1}% of acquisitions | {:.1}s (gen {:.1}s)",
                s.years, s.pop, s.bands, s.range_cells, s.items_now, s.items_mean, s.stage_now, s.stage_ever, s.experiments, s.inventions,
                s.transfers, s.forgets, s.splits, s.deaths, 100.0 * s.self_invented_frac, 100.0 * s.own_find_share, el, t_gen
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
                    Some(y) => format!(
                        "#{} {}c gap {} Y{:.0} pop {}{}",
                        c.id,
                        c.cells,
                        c.gap.map(|g| g.to_string()).unwrap_or("-".into()),
                        y,
                        c.pop_now,
                        how_reached(c)
                    ),
                    None => format!("#{} {}c gap {} never", c.id, c.cells, c.gap.map(|g| g.to_string()).unwrap_or("-".into())),
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
            println!("    started in: {}", s.origin);
            let q = &s.sea;
            println!(
                "    sea: first boat {} | voyages {} | sightings {} | drifts {} (ashore as new bands {}, got home {}) | scouting {} (found {}, lost {}) | landed on another landmass {} | disasters {} lost {} people ({:.0} per landing) | bands with boats now {} (seacraft {:.2})",
                q.first_boat.map(|y| format!("Y{y:.0}")).unwrap_or("never".into()),
                q.voyages, q.sightings, q.drifts, q.drift_landings, q.drift_returns, q.scouting, q.scout_finds, q.scouts_lost, q.crossings, q.sea_disasters, q.lost_at_sea,
                q.lost_at_sea as f32 / q.crossings.max(1) as f32, q.bands_with_boats, q.seacraft_mean
            );
            println!(
                "    seafaring (v9): seacraft p50 {:.2} p90 {:.2} max {:.2} | sail-usable bands {} | coastal boat range p90 {:.1} cells | far-shore word held by {:.1}% of bands ({:.1}% within range)",
                q.seacraft_p50, q.seacraft_p90, q.seacraft_max, q.sail_bands, q.range_p90, 100.0 * q.rumour_share, 100.0 * q.rumour_in_reach_share
            );
            let l = &s.lineages;
            let bl: Vec<String> = l
                .by_landmass
                .iter()
                .map(|x| format!("#{} {} people: top L{} {:.0}%, eff {:.1} (ancestry {:.1})", x.landmass, x.pop, x.top_lineage, x.top_pct, x.effective, x.ancestry_effective))
                .collect();
            println!("    lineages: alive {} | largest {:.0}% | effective {:.2} | by landmass: {}", l.alive, l.top_pct, l.effective, bl.join("; "));
            println!(
                "    ancestry (v9): largest founder {:.0}% of descent | effective founders {:.2} | {} weddings, {} fights between in-laws",
                l.ancestry_top_pct, l.ancestry_effective, s.motives.marriages, s.motives.fights_in_law
            );
            let mo = &s.motives;
            println!(
                "    motives (v9): splits {} ({} after dissent) | dissent mean {:.2} | curiosity {:.2} (overseas {:.2}) | ambition {:+.2} (overseas {:+.2}, {} bands) | land ever camped {:.0}%",
                mo.splits, mo.dissent_splits, mo.dissent_mean, mo.curiosity_mean, mo.curiosity_overseas, mo.ambition_mean, mo.ambition_overseas, mo.overseas_bands, mo.explored_pct
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
        let cf = conflict_stats(&w);
        if !a.json {
            println!(
                "    conflict: {} confrontations ({} avoided, {} paid off, {} fled) | {} fights ({:.1}/century), {} killed = {:.3}%/yr of people (lost at sea {}) | {} displaced ({} by sea), {} absorbed, {} captives | by century fights {:?} | lineages {} (largest {:.0}% of people; by century {:?}) | disease deaths {:.3}%/yr | pop by century {:?}",
                cf.confrontations, cf.avoided, cf.tributes, cf.fled, cf.fights, cf.fights_per_century, cf.deaths, cf.death_rate_pct,
                w.counters.lost_at_sea, cf.displaced, cf.fled_by_sea, cf.absorbed, cf.captives, cf.fights_by_century, cf.lineages, cf.top_lineage_pct, cf.lineage_by_century, cf.disease_rate_pct, cf.pop_by_century
            );
        }
        if !a.json {
            let g = &s.farming;
            let yo = |y: Option<f32>| y.map(|y| format!("Y{y:.0}")).unwrap_or("never".into());
            let og: Vec<String> = g.origins.iter().map(|(y, b)| format!("Y{y:.0} {b}")).collect();
            let fc: Vec<String> = w
                .history
                .iter()
                .enumerate()
                .filter(|(k, _)| (k + 1) % 100 == 0)
                .map(|(_, h)| format!("{}/{} {:.0}%", h.farmer_pop, h.pop, 100.0 * h.farm_food_share))
                .collect();
            println!(
                "    farming: first fields {} | independent origins {} [{}] | domestic grain {} | herding {} | plough {} | first epidemic {} | took up {} gave up {} | epidemics {} disease deaths {}",
                yo(g.first_cultivation), g.origins.len(), og.join("; "), yo(g.first_domestic_grain), yo(g.first_herding), yo(g.first_plough),
                yo(g.first_epidemic), g.took_up, g.abandoned, g.epidemics, g.disease_deaths
            );
            println!(
                "    farming now: {} farming bands ({} people) vs {} foraging bands ({} people) | landmasses farming {} foraging {} | {} herding | fields {} ({} domestic) soil {:.0}% | food from farming {:.0}% (peak {:.0}%) | people/occupied cell farmers {:.0} foragers {:.0} | by century farmer pop/pop food% [{}]",
                g.farmers, g.farmer_pop, g.forager_bands, g.forager_pop, g.landmasses_farming, g.landmasses_foraging, g.herders, g.fields, g.dom_fields,
                100.0 * g.field_fert, 100.0 * g.farm_food_share, 100.0 * g.peak_farm_food_share, g.farmer_density, g.forager_density, fc.join(", ")
            );
        }
        confl.push(cf);
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
        let mut fs = Vec::new();
        for i in 0..sums.len() {
            for j in i + 1..sums.len() {
                let dep = |s: &Summary| {
                    let m: std::collections::HashMap<u16, Vec<u16>> = s.firsts.iter().map(|f| (f.item, f.needs.clone())).collect();
                    move |x: u16, y: u16| m.get(&x).map(|v| v.contains(&y)).unwrap_or(false)
                };
                let (da, db) = (dep(&sums[i]), dep(&sums[j]));
                if let Some(d) = order_distance_free(&orders[i], &orders[j], &da, &db) {
                    fs.push(d);
                }
            }
        }
        let fmean = fs.iter().sum::<f32>() / fs.len().max(1) as f32;
        println!("\n== {} seeds, {} years", sums.len(), a.years);
        println!("discovery-order distance between seeds (Kendall tau, 0 same .. 1 reversed): mean {:.2} over {} pairs", mean, ds.len());
        println!("  same, over item pairs with no prerequisite link in either seed (diagnostic): mean {:.2}", fmean);
        let ls: Vec<String> = sums
            .iter()
            .map(|s| {
                let n = s.continents.iter().filter(|c| c.landfall_year.is_some()).count();
                let second = s.continents.iter().filter_map(|c| c.landfall_year).filter(|&y| y > 0.0).reduce(f32::min);
                format!("s{} {}/{}{}", s.seed, n, s.continents.len(), second.map(|y| format!(" (2nd Y{y:.0})")).unwrap_or_default())
            })
            .collect();
        let multi = sums.iter().filter(|s| s.continents.iter().filter(|c| c.landfall_year.is_some()).count() >= 2).count();
        let possible = sums.iter().filter(|s| s.continents.len() >= 2).count();
        println!("  settled a second continent: {multi}/{} seeds ({possible} have more than one) | {}", sums.len(), ls.join(", "));
        {
            // (v9) only boat-borne landfalls count as crossings; walks over straits are listed apart
            fn first(s: &Summary, sea: bool) -> Option<&civ_core::analysis::LandmassSummary> {
                s.continents
                    .iter()
                    .filter(|c| c.landfall_year.map_or(false, |y| y > 0.0) && (c.by_sea == Some(true)) == sea && c.by_sea.is_some())
                    .min_by(|a, b| a.landfall_year.partial_cmp(&b.landfall_year).unwrap())
            }
            let fmt = |s: &Summary, c: &civ_core::analysis::LandmassSummary| {
                format!("s{} Y{:.0} (gap {})", s.seed, c.landfall_year.unwrap_or(0.0), c.gap_settled.map(|g| g.to_string()).unwrap_or("-".into()))
            };
            let boat: Vec<String> = sums.iter().filter_map(|s| first(s, true).map(|c| fmt(s, c))).collect();
            let foot: Vec<String> = sums.iter().filter_map(|s| first(s, false).map(|c| fmt(s, c))).collect();
            let total: usize = sums.iter().map(|s| s.continents.iter().filter(|c| c.crossed() && c.landfall_year.map_or(false, |y| y > 0.0)).count()).sum();
            let early: Vec<String> = sums
                .iter()
                .filter_map(|s| first(s, true).filter(|c| c.landfall_year.unwrap_or(0.0) < 100.0).map(|c| fmt(s, c)))
                .collect();
            println!(
                "  boat-borne crossings to another continent: {}/{} seeds, {total} continents in all | first per seed: {} | before Y100: {} | reached on foot: {}",
                boat.len(),
                sums.len(),
                if boat.is_empty() { "none".to_string() } else { boat.join(", ") },
                if early.is_empty() { "none".to_string() } else { early.join(", ") },
                if foot.is_empty() { "none".to_string() } else { foot.join(", ") }
            );
        }
        {
            let lost: u64 = sums.iter().map(|s| s.sea.lost_at_sea).sum();
            let land: u64 = sums.iter().map(|s| s.sea.crossings).sum();
            let voy: u64 = sums.iter().map(|s| s.sea.voyages).sum();
            let sc: u64 = sums.iter().map(|s| s.sea.scouting).sum();
            let sf: u64 = sums.iter().map(|s| s.sea.scout_finds).sum();
            let early: Vec<String> = sums
                .iter()
                .filter_map(|s| s.continents.iter().filter_map(|c| c.landfall_year).filter(|&y| y > 0.0).reduce(f32::min).filter(|&y| y < 100.0).map(|y| format!("s{} Y{y:.0}", s.seed)))
                .collect();
            println!(
                "  sea (all seeds): voyages {voy} | landed on another landmass {land} | lost at sea {lost} ({:.0} per landing) | scouting {sc} (found {sf}) | second continent before Y100: {}",
                lost as f32 / land.max(1) as f32,
                if early.is_empty() { "none".to_string() } else { early.join(", ") }
            );
        }
        let own: Vec<f32> = sums.iter().map(|s| 100.0 * s.own_find_share).collect();
        let kin: Vec<f32> = sums.iter().map(|s| 100.0 * s.self_invented_frac).collect();
        println!(
            "  recipes found by experiment: {:.1}% of acquisitions (mean); known recipes found in-lineage at end: {:.0}% (mean)",
            own.iter().sum::<f32>() / own.len() as f32,
            kin.iter().sum::<f32>() / kin.len() as f32
        );
        for st in civ_core::analysis::STAGES {
            let n = sums.iter().filter(|s| s.stage_ever == st).count();
            let m = sums.iter().filter(|s| s.stage_now == st).count();
            if n + m > 0 {
                println!("  stage {st:<14} reached (highest ever) {n:>3}   holding now {m:>3}");
            }
        }
        {
            let n = sums.len();
            let farmed = sums.iter().filter(|s| s.farming.first_cultivation.is_some()).count();
            let mut fy: Vec<f32> = sums.iter().filter_map(|s| s.farming.first_cultivation).collect();
            fy.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let origins: usize = sums.iter().map(|s| s.farming.origins.len()).sum();
            let farming_now = sums.iter().filter(|s| s.farming.farmers > 0).count();
            let both = sums.iter().filter(|s| s.farming.farmers > 0 && s.farming.forager_pop > 0).count();
            let dom = sums.iter().filter(|s| s.farming.first_domestic_grain.is_some()).count();
            let herd = sums.iter().filter(|s| s.farming.first_herding.is_some()).count();
            let epi = sums.iter().filter(|s| s.farming.epidemics > 0).count();
            let fp: u32 = sums.iter().map(|s| s.farming.farmer_pop).sum();
            let tp: u32 = sums.iter().map(|s| s.pop).sum();
            let dd: u64 = sums.iter().map(|s| s.farming.disease_deaths).sum();
            let fd: Vec<String> = sums.iter().map(|s| format!("s{} {:.0}/{:.0}", s.seed, s.farming.farmer_density, s.farming.forager_density)).collect();
            println!(
                "  farming: fields in {farmed}/{n} seeds (first-field years {:?}), {origins} independent origins in all | farming at end {farming_now}/{n}, with foragers alongside {both}/{n} | domestic grain {dom}/{n} | herding {herd}/{n} | epidemics {epi}/{n} ({dd} disease deaths) | farmers {:.0}% of all people at end",
                fy.iter().map(|y| y.round() as i32).collect::<Vec<_>>(),
                100.0 * fp as f32 / tp.max(1) as f32
            );
            println!("  people per occupied cell at end, farmers/foragers: {}", fd.join(", "));
            let pop_sum: u32 = tp;
            println!("  total population at end (sum over seeds): {pop_sum}");
        }
        let alive = sums.iter().filter(|s| s.pop > 0).count();
        println!("  alive at end: {alive}/{}", sums.len());
        let n = confl.len() as f32;
        let mean = |f: &dyn Fn(&ConflictStats) -> f32| confl.iter().map(f).sum::<f32>() / n;
        println!(
            "  conflict (mean per seed): {:.1} fights/century, {:.0} killed = {:.3}%/yr of people, {:.0} displaced ({:.1} by sea), {:.1} absorbed, {:.0} paid off, {:.0} avoided | largest lineage {:.0}% of people (max {:.0}%), lineages alive {:.1}",
            mean(&|c| c.fights_per_century), mean(&|c| c.deaths as f32), mean(&|c| c.death_rate_pct), mean(&|c| c.displaced as f32),
            mean(&|c| c.fled_by_sea as f32), mean(&|c| c.absorbed as f32), mean(&|c| c.tributes as f32), mean(&|c| c.avoided as f32),
            mean(&|c| c.top_lineage_pct), confl.iter().map(|c| c.top_lineage_pct).fold(0.0, f32::max), mean(&|c| c.lineages as f32)
        );
        {
            let el: Vec<f32> = sums.iter().map(|s| s.lineages.effective).collect();
            let lt: Vec<f32> = sums.iter().filter_map(|s| s.lineages.by_landmass.first().map(|x| x.top_pct)).collect();
            let cents = confl.iter().map(|c| c.eff_by_century.len()).max().unwrap_or(0);
            let ec: Vec<String> = (0..cents).map(|k| format!("{:.1}", confl.iter().map(|c| c.eff_by_century.get(k).copied().unwrap_or(0.0)).sum::<f32>() / n)).collect();
            println!(
                "  lineages (v9): effective number {:.2} (mean; min {:.2}) | top lineage on the most populous landmass {:.0}% (mean; max {:.0}%) | effective by century (mean): [{}]",
                el.iter().sum::<f32>() / el.len().max(1) as f32,
                el.iter().copied().fold(f32::MAX, f32::min),
                lt.iter().sum::<f32>() / lt.len().max(1) as f32,
                lt.iter().copied().fold(0.0, f32::max),
                ec.join(", ")
            );
        }
        let cents = confl.iter().map(|c| c.fights_by_century.len()).max().unwrap_or(0);
        let fc: Vec<String> = (0..cents).map(|k| format!("{:.0}", confl.iter().map(|c| c.fights_by_century.get(k).copied().unwrap_or(0)).sum::<u32>() as f32 / n)).collect();
        let pc: Vec<String> = (0..cents).map(|k| format!("{:.0}", confl.iter().map(|c| c.pop_by_century.get(k).copied().unwrap_or(0)).sum::<u32>() as f32 / n)).collect();
        let lc: Vec<String> = (0..cents).map(|k| format!("{:.0}", confl.iter().map(|c| c.lineage_by_century.get(k).copied().unwrap_or(0)).sum::<u32>() as f32 / n)).collect();
        println!("  fights per century (mean): [{}] | population at end of each century (mean): [{}] | largest lineage % by century (mean): [{}] | disease deaths {:.3}%/yr", fc.join(", "), pc.join(", "), lc.join(", "), mean(&|c| c.disease_rate_pct));
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

/// How people first reached a continent, for the per-seed line: " [sea, 17 from settled]",
/// " [foot]", or " [?]" for saves from before v9 (and for homelands).
fn how_reached(c: &civ_core::analysis::LandmassSummary) -> String {
    match c.by_sea {
        Some(true) => format!(" [sea, {} from settled]", c.gap_settled.map(|g| g.to_string()).unwrap_or("-".into())),
        Some(false) => " [foot]".to_string(),
        None if c.landfall_year == Some(0.0) => String::new(),
        None => " [?]".to_string(),
    }
}

struct ConflictStats {
    confrontations: u64,
    avoided: u64,
    tributes: u64,
    fled: u64,
    fights: u64,
    fights_per_century: f32,
    deaths: u64,
    /// conflict deaths per year as a share of the population (mean over the run)
    death_rate_pct: f32,
    displaced: u64,
    fled_by_sea: u64,
    absorbed: u64,
    captives: u64,
    fights_by_century: Vec<u32>,
    pop_by_century: Vec<u32>,
    lineages: u32,
    top_lineage_pct: f32,
    /// largest lineage share (%) at the end of each century
    lineage_by_century: Vec<u32>,
    /// (v9) effective number of lineages at the end of each century (0 before v9 saves)
    eff_by_century: Vec<f32>,
    /// disease deaths per year as a share of the population
    disease_rate_pct: f32,
}

fn conflict_stats(w: &World) -> ConflictStats {
    let c = &w.counters;
    let years = (w.tick as f32 / 4.0).max(1.0);
    let person_years: f64 = w.history.iter().map(|h| h.pop as f64).sum();
    let cents = (w.history.len() + 99) / 100;
    let mut fbc = vec![0u32; cents];
    let mut pbc = vec![0u32; cents];
    let mut lbc = vec![0u32; cents];
    let mut ebc = vec![0f32; cents];
    let mut dd = 0u64;
    for (k, h) in w.history.iter().enumerate() {
        fbc[k / 100] += h.fights;
        pbc[k / 100] = h.pop;
        lbc[k / 100] = h.top_lineage_pct.round() as u32;
        ebc[k / 100] = h.eff_lineages;
        dd += h.disease_deaths as u64;
    }
    let mut lin: std::collections::BTreeMap<u32, u32> = Default::default();
    for b in &w.bands {
        *lin.entry(b.lineage).or_default() += b.pop;
    }
    let tot = w.total_pop().max(1) as f32;
    ConflictStats {
        confrontations: c.confrontations,
        avoided: c.avoided,
        tributes: c.tributes,
        fled: c.fled,
        fights: c.fights,
        fights_per_century: c.fights as f32 / years * 100.0,
        deaths: c.conflict_deaths,
        death_rate_pct: if person_years > 0.0 { (100.0 * c.conflict_deaths as f64 / person_years) as f32 } else { 0.0 },
        displaced: c.displaced,
        fled_by_sea: c.fled_by_sea,
        absorbed: c.absorbed,
        captives: c.captives,
        fights_by_century: fbc,
        pop_by_century: pbc,
        lineages: lin.len() as u32,
        top_lineage_pct: 100.0 * lin.values().copied().max().unwrap_or(0) as f32 / tot,
        lineage_by_century: lbc,
        eff_by_century: ebc,
        disease_rate_pct: if person_years > 0.0 { (100.0 * dd as f64 / person_years) as f32 } else { 0.0 },
    }
}
