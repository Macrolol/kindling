//! Map geography for sea crossings (generates worlds only, no simulation):
//! `cargo run --release --example sea_gaps -- 1 10` -> per seed, landmass sizes, which one the
//! founders start on, and the narrowest open-water gap (cells) from the start landmass to each other.
use civ_core::materials::*;
use civ_core::params::{Config, Params};
use civ_core::world::World;
use std::collections::VecDeque;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    for m in [Mat::Fibre, Mat::Wood, Mat::Hide] {
        println!("boat_q {:?} = {:.2}", m, boat_q(m));
    }
    for seed in a[0]..=a[1] {
        let w = World::new(Config { seed, ..Default::default() }, Params::default());
        let m = &w.map;
        let n = m.w * m.h;
        let start = m.landmass[w.bands[0].cell(m.w)];
        // BFS over sea cells (8-neighbour) from the start landmass's coast
        let mut dist = vec![u32::MAX; n];
        let mut q = VecDeque::new();
        for i in 0..n {
            if m.landmass[i] == start {
                dist[i] = 0;
                q.push_back(i);
            }
        }
        let mut best = vec![u32::MAX; m.landmass_size.len()];
        while let Some(i) = q.pop_front() {
            let (x, y) = ((i % m.w) as i32, (i / m.w) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= m.w as i32 || ny >= m.h as i32 {
                        continue;
                    }
                    let j = ny as usize * m.w + nx as usize;
                    let l = m.landmass[j];
                    if l != 0 && l != start {
                        best[l as usize] = best[l as usize].min(dist[i]);
                        continue;
                    }
                    if l == 0 && dist[j] == u32::MAX {
                        dist[j] = dist[i] + 1;
                        q.push_back(j);
                    }
                }
            }
        }
        let big: Vec<String> = (1..m.landmass_size.len())
            .filter(|&l| m.landmass_size[l] >= 400 && l as u16 != start)
            .map(|l| format!("L{} {} cells gap {}", l, m.landmass_size[l], best[l]))
            .collect();
        let small_gap = (1..m.landmass_size.len()).filter(|&l| l as u16 != start && m.landmass_size[l] < 400).map(|l| best[l]).min().unwrap_or(u32::MAX);
        println!("seed {seed}: start L{start} ({} cells) | continents: {} | nearest island gap {}", m.landmass_size[start as usize], big.join("; "), small_gap);
    }
}
