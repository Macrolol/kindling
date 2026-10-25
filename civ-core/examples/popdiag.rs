//! Read-only diagnostics for the population plateau. Does not change sim behaviour.
//! cargo run --release -p civ-core --example popdiag -- <seed> <years> <every>
use civ_core::map::NBIOME;
use civ_core::params::{Config, Params};
use civ_core::World;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let years: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(500);
    let every: u32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(25);
    let mut w = World::new(
        Config {
            seed,
            ..Default::default()
        },
        Params::default(),
    );
    let m = &w.map;
    let n = m.w * m.h;
    // analytic max sustainable yield per cell per year (stock units), from the regrow() equations at p = k/2
    let mut cap_people = vec![0f32; n];
    let mut land = 0;
    for i in 0..n {
        if !m.is_land(i) {
            continue;
        }
        land += 1;
        let mut y = 0.0;
        let mut need = 0.0;
        for s in 0..4 {
            let t = m.temp_season(i, s);
            let g = ((t - 3.0) / 12.0).clamp(0.0, 1.0);
            let kp = m.k_plants[i];
            y += (0.5 * g * 0.25 + 0.03 * g) * kp - 0.12 * (1.0 - g) * 0.5 * kp;
            y += (0.12 * 0.25 + 0.01) * m.k_game[i];
            y += (0.3 * 0.25 + 0.02) * m.k_fish[i];
            need += 1.0 + ((8.0 - t) / 14.0).max(0.0); // no clothing
        }
        cap_people[i] = y.max(0.0) / need;
    }
    let total_cap: f32 = cap_people.iter().sum();
    println!("seed {seed}: land cells {land}, analytic sustainable capacity (no tools, no cooking, no clothing) = {:.0} people", total_cap);
    let names = [
        "ocean", "ice", "tundra", "taiga", "steppe", "grass", "tempfor", "desert", "savanna",
        "tropfor", "mount",
    ];
    let mut bcap = [0f32; NBIOME];
    let mut bcells = [0u32; NBIOME];
    for i in 0..n {
        if m.is_land(i) {
            let b = m.biome_at(i) as usize;
            bcap[b] += cap_people[i];
            bcells[b] += 1;
        }
    }
    print!("  capacity by biome (cells / people):");
    for b in 1..NBIOME {
        if bcells[b] > 0 {
            print!(" {} {}/{:.0}", names[b], bcells[b], bcap[b]);
        }
    }
    println!();
    println!("  year   pop bands mean/band | occ.cells  inRange%  pop/cap(inRange) | food<1 %  meanFood spare stress cold | plants/K game/K fish/K (in range) | splits/yr deaths/yr | kit plants game fish cook protect");
    let mut last_splits = 0;
    let mut last_deaths = 0;
    for yr in 1..=years {
        for _ in 0..4 {
            w.step();
        }
        if yr % every != 0 {
            continue;
        }
        let m = &w.map;
        let mut occ = vec![false; n];
        let mut inr = vec![false; n];
        for b in &w.bands {
            let (x, y) = (b.x as i32, b.y as i32);
            occ[b.cell(m.w)] = true;
            for dy in -1..=1i32 {
                for dx in -1..=1i32 {
                    let (xx, yy) = (x + dx, y + dy);
                    if xx >= 0 && yy >= 0 && (xx as usize) < m.w && (yy as usize) < m.h {
                        let j = yy as usize * m.w + xx as usize;
                        if m.is_land(j) {
                            inr[j] = true;
                        }
                    }
                }
            }
        }
        let occn = occ.iter().filter(|&&v| v).count();
        let inrn = inr.iter().filter(|&&v| v).count();
        let cap_in: f32 = (0..n).filter(|&i| inr[i]).map(|i| cap_people[i]).sum();
        let (mut sp, mut sg, mut sf, mut kp, mut kg, mut kf) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
        for i in 0..n {
            if inr[i] {
                sp += w.plants[i];
                sg += w.game[i];
                sf += w.fish[i];
                kp += m.k_plants[i];
                kg += m.k_game[i];
                kf += m.k_fish[i];
            }
        }
        let nb = w.bands.len().max(1) as f32;
        let pop = w.total_pop();
        let hungry = w.bands.iter().filter(|b| b.d.food_ratio < 0.999).count() as f32 / nb * 100.0;
        let mf = w.bands.iter().map(|b| b.d.food_ratio).sum::<f32>() / nb;
        let msp = w.bands.iter().map(|b| b.d.spare).sum::<f32>() / nb;
        let mst = w.bands.iter().map(|b| b.d.stress).sum::<f32>() / nb;
        let mc = w.bands.iter().map(|b| b.d.cold).sum::<f32>() / nb;
        let k = |f: &dyn Fn(&civ_core::band::Band) -> f32| {
            w.bands.iter().map(|b| f(b)).sum::<f32>() / nb
        };
        let splits = w.counters.splits - last_splits;
        let deaths = w.counters.deaths - last_deaths;
        last_splits = w.counters.splits;
        last_deaths = w.counters.deaths;
        println!("  {:4} {:6} {:5} {:6.1}   | {:6} {:7.1}% {:8.2}          | {:6.1}% {:8.2} {:5.2} {:6.2} {:4.2} | {:6.2} {:6.2} {:6.2}            | {:7.1} {:7.1}    | {:.2} {:.2} {:.2} {:.2} {:.2}",
            yr, pop, w.bands.len(), pop as f32 / nb, occn, inrn as f32 / land as f32 * 100.0, pop as f32 / cap_in.max(1.0),
            hungry, mf, msp, mst, mc, sp / kp.max(1.0), sg / kg.max(1.0), sf / kf.max(1.0),
            splits as f32 / every as f32, deaths as f32 / every as f32,
            k(&|b| b.d.kit.plants_m), k(&|b| b.d.kit.game_m), k(&|b| b.d.kit.fish_m), k(&|b| b.d.kit.digest_mean()), k(&|b| b.d.kit.protect));
    }
    // final: where people are vs capacity, per biome
    let m = &w.map;
    let mut bpop = [0u32; NBIOME];
    for b in &w.bands {
        bpop[m.biome_at(b.cell(m.w)) as usize] += b.pop;
    }
    print!("  final pop by biome (people / capacity):");
    for b in 1..NBIOME {
        if bcells[b] > 0 {
            print!(" {} {}/{:.0}", names[b], bpop[b], bcap[b]);
        }
    }
    println!();
    // band size distribution and food ratio
    let mut sizes: Vec<u32> = w.bands.iter().map(|b| b.pop).collect();
    sizes.sort();
    let q = |f: f32| sizes[((sizes.len() - 1) as f32 * f) as usize];
    println!(
        "  band size p10 {} p50 {} p90 {} max {}",
        q(0.1),
        q(0.5),
        q(0.9),
        q(1.0)
    );
}
