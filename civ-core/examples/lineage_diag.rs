//! Read-only diagnostics for single-lineage dominance (v7). Does not change sim behaviour.
//! cargo run --release -p civ-core --example lineage_diag -- <seed> <years> <every>
//! Per lineage and window: share of people, bands, landmasses held, farming/settled share,
//! weapons, food, growth, fights won/lost against other lineages and against its own,
//! and people taken in from broken remnants of other lineages.
use civ_core::band::weapon_q;
use civ_core::params::{Config, Params};
use civ_core::world::EventKind;
use civ_core::World;
use std::collections::BTreeMap;

#[derive(Default, Clone)]
struct W {
    won_x: u32,
    lost_x: u32,
    won_own: u32,
    absorbed_in: u32,
    absorbed_out: u32,
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(7);
    let years: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(500);
    let every: u32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    // optional: run another seed first in the same process (checks for hidden global state)
    if let Some(pre) = std::env::var("PRESEED").ok().and_then(|s| s.parse::<u64>().ok()) {
        let mut p = World::new(Config { seed: pre, ..Default::default() }, Params::default());
        for _ in 0..years * 4 {
            p.step();
        }
        println!("pre-seed {pre}: pop {}", p.total_pop());
    }
    let mut w = World::new(Config { seed, ..Default::default() }, Params::default());
    let mut lin_of: BTreeMap<u32, u32> = BTreeMap::new();
    let mut win: BTreeMap<u32, W> = BTreeMap::new();
    let mut last_pop: BTreeMap<u32, u32> = BTreeMap::new();
    let mut seen_tick = 0u32;
    println!("seed {seed}");
    for yr in 1..=years {
        for _ in 0..4 {
            for b in &w.bands {
                lin_of.insert(b.id, b.lineage);
            }
            w.step();
            for b in &w.bands {
                lin_of.insert(b.id, b.lineage);
            }
            for e in w.events.iter().filter(|e| e.tick >= seen_tick) {
                let (lw, ll) = (lin_of.get(&e.band).copied(), lin_of.get(&e.other).copied());
                let (Some(lw), Some(ll)) = (lw, ll) else { continue };
                if e.kind == EventKind::Fight as u8 || e.kind == EventKind::BloodiestFight as u8 || e.kind == EventKind::FirstFight as u8 {
                    if lw == ll {
                        win.entry(lw).or_default().won_own += 1;
                    } else {
                        win.entry(lw).or_default().won_x += 1;
                        win.entry(ll).or_default().lost_x += 1;
                    }
                } else if e.kind == EventKind::Absorbed as u8 && lw != ll {
                    // band = loser, other = winner, recipe = people
                    win.entry(ll).or_default().absorbed_in += e.recipe;
                    win.entry(lw).or_default().absorbed_out += e.recipe;
                }
            }
            seen_tick = w.tick;
        }
        if yr % every != 0 {
            continue;
        }
        let total = w.total_pop().max(1) as f32;
        let mw = w.map.w;
        #[derive(Default)]
        struct S { pop: u32, bands: u32, lm: BTreeMap<u16, u32>, farm: f32, settled: f32, wq: f32, food: f32, gen: f32, boat: f32 }
        let mut by: BTreeMap<u32, S> = BTreeMap::new();
        for b in &w.bands {
            let s = by.entry(b.lineage).or_default();
            let p = b.pop as f32;
            s.pop += b.pop;
            s.bands += 1;
            *s.lm.entry(w.map.landmass_of(b.cell(mw))).or_default() += b.pop;
            s.farm += p * (b.sow > 0.05) as u8 as f32;
            s.settled += p * b.settled;
            s.wq += p * weapon_q(&b.d.kit);
            s.food += p * b.d.food_ratio.min(1.5);
            s.gen += p * b.gen as f32;
            s.boat += p * (b.d.kit.boat_q > 0.0) as u8 as f32;
        }
        println!("Y{yr} pop {} bands {}", w.total_pop(), w.bands.len());
        for (l, s) in &by {
            let p = s.pop.max(1) as f32;
            let g = last_pop.get(l).map(|&o| (s.pop as f32 / o.max(1) as f32).powf(1.0 / every as f32) - 1.0).unwrap_or(0.0);
            let lm: Vec<String> = s.lm.iter().filter(|(_, &v)| v as f32 > 0.02 * p).map(|(k, v)| format!("#{k}:{:.0}%", 100.0 * *v as f32 / p)).collect();
            let x = win.get(l).cloned().unwrap_or_default();
            println!(
                "  L{l:<4} share {:5.1}% pop {:7} bands {:5} growth {:+.2}%/yr | farm {:3.0}% settled {:.2} weapon {:.2} food {:.2} boat {:3.0}% gen {:5.1} | won/lost vs others {}/{} (own {}) | absorbed in/out {}/{} | lands {}",
                100.0 * s.pop as f32 / total, s.pop, s.bands, 100.0 * g,
                100.0 * s.farm / p, s.settled / p, s.wq / p, s.food / p, 100.0 * s.boat / p, s.gen / p,
                x.won_x, x.lost_x, x.won_own, x.absorbed_in, x.absorbed_out, lm.join(" ")
            );
        }
        last_pop = by.iter().map(|(k, s)| (*k, s.pop)).collect();
        win.clear();
    }
}
