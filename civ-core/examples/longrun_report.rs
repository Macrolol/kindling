//! Read-only long-run report of a saved world (conflict, population, knowledge, seafaring):
//! `cargo run --release --example longrun_report -- file.civsave`
use civ_core::materials::*;
use civ_core::world::{EventKind, World};
use std::collections::BTreeMap;

fn main() {
    let path = std::env::args().nth(1).expect("save path");
    let bytes = std::fs::read(&path).unwrap();
    let (w, _view, info) = World::load_bytes(&bytes).expect("load");
    println!("== {path}");
    println!(
        "format v{} app {} migrated_from {:?} | year {:.1} | map {}x{} seed {} | bands {} pop {}",
        info.format_version,
        info.app_version,
        info.migrated_from,
        w.tick as f32 / 4.0,
        w.map.w,
        w.map.h,
        w.cfg.seed,
        w.bands.len(),
        w.total_pop()
    );
    println!("settings now: {:?}", w.params);
    println!("settings change log ({}):", w.settings_log.len());
    for c in &w.settings_log {
        println!(
            "   Y{:.1} {} {} -> {}",
            c.tick as f32 / 4.0,
            c.name,
            c.old,
            c.new
        );
    }
    let c = &w.counters;
    println!("counters: fights {} conflict_deaths {} confrontations {} avoided {} tributes {} fled {} displaced {} fled_by_sea {} absorbed {} captives {} max_fight_deaths {} | voyages {} sea_disasters {} lost_at_sea {} drifts {} drift_landings {} splits {} deaths(band ends) {}",
        c.fights, c.conflict_deaths, c.confrontations, c.avoided, c.tributes, c.fled, c.displaced, c.fled_by_sea, c.absorbed, c.captives, c.max_fight_deaths,
        c.voyages, c.sea_disasters, c.lost_at_sea, c.drifts, c.drift_landings, c.splits, c.deaths);

    // events in the chronicle by kind and century
    let mut by: BTreeMap<(u32, u8), u32> = BTreeMap::new();
    for e in &w.chronicle {
        *by.entry((e.tick / 400, e.kind)).or_default() += 1;
    }
    println!(
        "history len {} years, chronicle {} entries, recent events {}",
        w.history.len(),
        w.chronicle.len(),
        w.events.len()
    );

    println!("\n-- per century: pop mean/end | bands | fights | killed | killed %/yr | displaced | absorbed | tributes | avoided | confront | trades | partnerships(end) | inv | forgets | items_world(end) | items_mean(end) | max items");
    for ch in w.history.chunks(100) {
        let n = ch.len() as f32;
        let l = ch.last().unwrap();
        let pop = ch.iter().map(|h| h.pop as f32).sum::<f32>() / n;
        let s = |f: &dyn Fn(&civ_core::world::YearStats) -> u32| {
            ch.iter().map(|h| f(h) as u64).sum::<u64>()
        };
        let killed = s(&|h| h.conflict_deaths);
        println!("   Y{:>4}-{:<4} {:>6.0}/{:<6} {:>5} | f {:>5} k {:>6} {:.3}% | disp {:>5} abs {:>4} trib {:>5} avoid {:>6} conf {:>6} | trades {:>7} part {:>5} | inv {:>4} forg {:>7} | items {:>3} mean {:>5.1} max {:>3}",
            ch[0].year, l.year, pop, l.pop, l.bands, s(&|h| h.fights), killed, 100.0 * killed as f32 / n / pop.max(1.0),
            s(&|h| h.displaced), s(&|h| h.absorbed), s(&|h| h.tributes), s(&|h| h.avoided), s(&|h| h.confrontations),
            s(&|h| h.trades), l.partnerships, s(&|h| h.inventions), s(&|h| h.forgets), l.items_world, l.items_mean, l.items_max);
    }

    println!("\n-- chronicle kinds by century (major events)");
    let mut cents: Vec<u32> = by.keys().map(|k| k.0).collect();
    cents.dedup();
    for cc in cents {
        let parts: Vec<String> = by
            .iter()
            .filter(|(k, _)| k.0 == cc)
            .map(|(k, v)| format!("{:?} {}", EventKind::from_u8(k.1), v))
            .collect();
        println!("   C{:>2}: {}", cc, parts.join(", "));
    }

    println!("\n-- world firsts:");
    for f in &w.firsts {
        println!("   Y{:>7.1} {}", f.tick as f32 / 4.0, item_name(f.item));
    }
    println!(
        "\n-- items lost worldwide (times): {}",
        w.item_lost
            .iter()
            .map(|(i, n)| format!("{} x{}", item_name(*i), n))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let held: Vec<String> = w
        .item_bands
        .iter()
        .filter(|(i, _)| matches!(mat(**i), Mat::Copper | Mat::Tin | Mat::Bronze | Mat::Iron))
        .map(|(i, n)| format!("{} {}", item_name(*i), n))
        .collect();
    println!("metal items, bands able now: {}", held.join(", "));

    // landfall
    println!("\n-- landmasses: first arrival (Y):");
    let mut sizes: BTreeMap<u32, u32> = BTreeMap::new();
    for i in 0..w.map.w * w.map.h {
        if let Some(&l) = w.map.landmass.get(i) {
            if l > 0 {
                *sizes.entry(l as u32).or_default() += 1;
            }
        }
    }
    let mut occ: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
    for b in &w.bands {
        let l = w.map.landmass.get(b.cell(w.map.w)).copied().unwrap_or(0) as u32;
        let e = occ.entry(l).or_default();
        e.0 += 1;
        e.1 += b.pop;
    }
    for (i, &t) in w.landfall.iter().enumerate() {
        let id = i as u32;
        let sz = sizes.get(&id).copied().unwrap_or(0);
        if sz < 30 && t == u32::MAX {
            continue;
        }
        let (nb, np) = occ.get(&id).copied().unwrap_or((0, 0));
        println!(
            "   landmass {:>3} {:>6} cells | arrived {} | now {} bands {} people",
            id,
            sz,
            if t == u32::MAX {
                "never".into()
            } else {
                format!("Y{:.0}", t as f32 / 4.0)
            },
            nb,
            np
        );
    }

    // largest lineage share by century (yearly series, recorded from v5 on)
    let lt: Vec<String> = w
        .history
        .chunks(100)
        .map(|c| {
            let v: Vec<f32> = c
                .iter()
                .map(|y| y.top_lineage_pct)
                .filter(|&p| p > 0.0)
                .collect();
            if v.is_empty() {
                "-".into()
            } else {
                format!(
                    "{:.0}%/{}",
                    v.iter().sum::<f32>() / v.len() as f32,
                    c.last().map(|y| y.lineages).unwrap_or(0)
                )
            }
        })
        .collect();
    println!(
        "\n-- largest lineage share / lineages alive, by century: {}",
        lt.join(" ")
    );
    // lineages and grudges now
    let mut lin: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
    for b in &w.bands {
        let e = lin.entry(b.lineage).or_default();
        e.0 += 1;
        e.1 += b.pop;
    }
    let tp = w.total_pop().max(1) as f32;
    let mut lv: Vec<_> = lin.iter().collect();
    lv.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
    println!(
        "\n-- lineages alive {}: {}",
        lv.len(),
        lv.iter()
            .take(8)
            .map(|(l, (nb, np))| format!("L{} {:.1}% ({} bands)", l, 100.0 * *np as f32 / tp, nb))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let nb = w.bands.len().max(1) as f32;
    let with = w.bands.iter().filter(|b| !b.grudges.is_empty()).count();
    let tot: usize = w.bands.iter().map(|b| b.grudges.len()).sum();
    let wsum: f32 = w
        .bands
        .iter()
        .flat_map(|b| b.grudges.iter().map(|g| g.w))
        .sum();
    let same: usize = w
        .bands
        .iter()
        .map(|b| b.grudges.iter().filter(|g| g.lineage == b.lineage).count())
        .sum();
    let full = w.bands.iter().filter(|b| b.grudges.len() >= 6).count();
    let strong = w
        .bands
        .iter()
        .flat_map(|b| b.grudges.iter())
        .filter(|g| g.w > 1.0)
        .count();
    let mut wh = [0u32; 5];
    for g in w.bands.iter().flat_map(|b| b.grudges.iter()) {
        wh[((g.w / 0.4) as usize).min(4)] += 1;
    }
    let alive: std::collections::HashSet<u32> = w.bands.iter().map(|b| b.id).collect();
    let vs_alive = w
        .bands
        .iter()
        .flat_map(|b| b.grudges.iter())
        .filter(|g| alive.contains(&g.id))
        .count();
    let mutual = w
        .bands
        .iter()
        .flat_map(|b| b.grudges.iter().map(move |g| (b.id, g.id)))
        .filter(|&(a, o)| {
            w.bands
                .iter()
                .find(|x| x.id == o)
                .map(|x| x.grudges.iter().any(|g| g.id == a))
                .unwrap_or(false)
        })
        .count();
    println!("-- grudges now: {:.0}% of bands hold any, {:.2} per band, mean weight {:.2}, at cap (6) {} bands, weight>1 {}, histogram by 0.4 {:?}, against own lineage {:.0}%, against bands still alive {:.0}%, mutual {:.0}%",
        100.0 * with as f32 / nb, tot as f32 / nb, wsum / tot.max(1) as f32, full, strong, wh, 100.0 * same as f32 / tot.max(1) as f32, 100.0 * vs_alive as f32 / tot.max(1) as f32, 100.0 * mutual as f32 / tot.max(1) as f32);
    let pressed: f32 = w.bands.iter().map(|b| b.pressed).sum::<f32>() / nb;
    println!("-- mean pressed {:.3}", pressed);

    // population every 25 years, and yearly fight-death rate variability per century
    println!("\n-- pop every 25y:");
    let pts: Vec<String> = w
        .history
        .iter()
        .filter(|h| h.year % 25 == 0)
        .map(|h| format!("{}:{}", h.year, h.pop))
        .collect();
    for ch in pts.chunks(16) {
        println!("   {}", ch.join(" "));
    }
    println!("\n-- per century: yearly killed %/pop  min / median / max, lag-1 autocorr, max yearly pop drop %");
    for ch in w.history.chunks(100) {
        let mut r: Vec<f32> = ch
            .iter()
            .map(|h| 100.0 * h.conflict_deaths as f32 / h.pop.max(1) as f32)
            .collect();
        let m = r.iter().sum::<f32>() / r.len() as f32;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..r.len() {
            den += (r[i] - m).powi(2);
            if i > 0 {
                num += (r[i] - m) * (r[i - 1] - m);
            }
        }
        let drop = ch
            .windows(2)
            .map(|x| 100.0 * (x[0].pop as f32 - x[1].pop as f32) / x[0].pop.max(1) as f32)
            .fold(0.0f32, f32::max);
        r.sort_by(|a, b| a.total_cmp(b));
        println!(
            "   Y{:>4}: {:.2} / {:.2} / {:.2}  ac1 {:.2}  max drop {:.1}%",
            ch[0].year,
            r[0],
            r[r.len() / 2],
            r[r.len() - 1],
            num / den.max(1e-9),
            drop
        );
    }
    // spread of key items: cells where a band first could make it, by century and landmass
    let nl = w.landfall.len();
    for name in [
        "copper (metal)",
        "bronze (metal)",
        "bronze knife",
        "fired pottery",
        "dugout canoe",
        "skin boat",
    ] {
        let Some((it, v)) = w.cell_first.iter().find(|(i, _)| item_name(**i) == name) else {
            continue;
        };
        let _ = it;
        let mut by: BTreeMap<(u32, u16), u32> = BTreeMap::new();
        for (i, &t) in v.iter().enumerate() {
            if t != u32::MAX {
                *by.entry((t / 400, w.map.landmass[i])).or_default() += 1;
            }
        }
        let mut cum = vec![0u32; nl.max(8)];
        let mut line = Vec::new();
        let mut cents: Vec<u32> = by.keys().map(|k| k.0).collect();
        cents.dedup();
        for c in cents {
            for ((cc, l), n) in &by {
                if *cc == c {
                    if (*l as usize) < cum.len() {
                        cum[*l as usize] += n;
                    }
                }
            }
            line.push(format!(
                "C{}:[{}]",
                c,
                (1..5)
                    .map(|l| cum[l].to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        println!(
            "-- {name}: cumulative cells reached by end of century [landmass 1,2,3,4]: {}",
            line.join(" ")
        );
    }
    // per landmass now: items and metal
    let bronze = w
        .cell_first
        .keys()
        .find(|i| item_name(**i) == "bronze (metal)")
        .copied();
    let copper = w
        .cell_first
        .keys()
        .find(|i| item_name(**i) == "copper (metal)")
        .copied();
    println!("\n-- per landmass now: bands, mean items known, bands knowing copper / bronze metal, mean kit tools");
    let mut pl: BTreeMap<u16, (u32, f32, u32, u32)> = BTreeMap::new();
    for b in &w.bands {
        let l = w.map.landmass[b.cell(w.map.w)];
        let e = pl.entry(l).or_default();
        e.0 += 1;
        e.1 += b.known.len() as f32;
        if copper
            .map(|c| {
                b.known
                    .iter()
                    .any(|k| w.reg.recipes[k.recipe as usize].output == c)
            })
            .unwrap_or(false)
        {
            e.2 += 1;
        }
        if bronze
            .map(|c| {
                b.known
                    .iter()
                    .any(|k| w.reg.recipes[k.recipe as usize].output == c)
            })
            .unwrap_or(false)
        {
            e.3 += 1;
        }
    }
    for (l, (n, k, cu, br)) in pl {
        println!(
            "   landmass {l}: {n} bands, items {:.1}, copper {cu}, bronze {br}",
            k / n.max(1) as f32
        );
    }
    // recent fights: do the same pairs keep fighting?
    let fights: Vec<_> = w
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Fight as u8)
        .collect();
    let mut pairs: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for e in &fights {
        let k = (e.band.min(e.other), e.band.max(e.other));
        *pairs.entry(k).or_default() += 1;
    }
    let rep = pairs.values().filter(|&&n| n > 1).count();
    let maxp = pairs.values().copied().max().unwrap_or(0);
    let span = fights
        .first()
        .zip(fights.last())
        .map(|(a, b)| (b.tick - a.tick) as f32 / 4.0)
        .unwrap_or(0.0);
    println!("\n-- recent event log: {} fights over {:.1} years, {} distinct pairs, {} pairs fought more than once, most by one pair {}", fights.len(), span, pairs.len(), rep, maxp);

    // ore cells per landmass (copper / tin)
    let mut ore: BTreeMap<u16, (u32, u32)> = BTreeMap::new();
    for i in 0..w.map.deposits.len() {
        let l = w.map.landmass[i];
        if l == 0 {
            continue;
        }
        let e = ore.entry(l).or_default();
        if w.map.deposits[i] & civ_core::map::D_COPPER != 0 {
            e.0 += 1;
        }
        if w.map.deposits[i] & civ_core::map::D_TIN != 0 {
            e.1 += 1;
        }
    }
    println!(
        "-- ore cells per landmass (copper, tin): {:?}",
        ore.iter().filter(|(l, _)| **l <= 5).collect::<Vec<_>>()
    );
}
#[allow(dead_code)]
fn _unused() {}
