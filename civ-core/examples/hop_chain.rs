//! Island-hopping geography (generates worlds only, no simulation):
//! `cargo run --release --example hop_chain -- SEED_A SEED_B [W H]` -> per seed, for every other
//! continent, the shortest open-water hop that must be made on the best chain of landmasses
//! (islands as stepping stones) from the start landmass, the chain's hop count, and the direct gap.
use civ_core::params::{Config, Params};
use civ_core::world::World;
use std::collections::VecDeque;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let (mw, mh) = if a.len() >= 4 { (a[2] as usize, a[3] as usize) } else { (256, 160) };
    for seed in a[0]..=a[1] {
        let w = World::new(Config { seed, map_w: mw, map_h: mh, ..Default::default() }, Params::default());
        let m = &w.map;
        let n = m.w * m.h;
        let nl = m.landmass_size.len();
        let start = m.landmass_of(w.bands[0].cell(m.w)) as usize;
        let cont = m.continent_cells();
        // pairwise narrowest gap between landmasses (BFS over sea from each landmass, cut at 200)
        let mut gap = vec![vec![u32::MAX; nl]; nl];
        let mut dist = vec![u32::MAX; n];
        for l in 1..nl {
            if m.landmass_size[l] == 0 {
                continue;
            }
            dist.iter_mut().for_each(|d| *d = u32::MAX);
            let mut q = VecDeque::new();
            for i in 0..n {
                if m.landmass_of(i) as usize == l {
                    dist[i] = 0;
                    q.push_back(i);
                }
            }
            while let Some(i) = q.pop_front() {
                if dist[i] >= 200 {
                    continue;
                }
                let (x, y) = ((i % m.w) as i32, (i / m.w) as i32);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= m.w as i32 || ny >= m.h as i32 {
                            continue;
                        }
                        let j = ny as usize * m.w + nx as usize;
                        if dist[j] != u32::MAX {
                            continue;
                        }
                        let k = m.landmass_of(j) as usize;
                        if k == 0 {
                            dist[j] = dist[i] + 1;
                            q.push_back(j);
                        } else if k != l && k < nl {
                            gap[l][k] = gap[l][k].min(dist[i]);
                        }
                    }
                }
            }
        }
        // bottleneck path from start: minimise the largest hop, then the hop count
        let mut best = vec![(u32::MAX, u32::MAX); nl];
        best[start] = (0, 0);
        let mut done = vec![false; nl];
        loop {
            let Some(u) = (1..nl).filter(|&l| !done[l] && best[l].0 != u32::MAX).min_by_key(|&l| best[l]) else { break };
            done[u] = true;
            for v in 1..nl {
                if gap[u][v] == u32::MAX || done[v] {
                    continue;
                }
                let c = (best[u].0.max(gap[u][v]), best[u].1 + 1);
                if c < best[v] {
                    best[v] = c;
                }
            }
        }
        let mut line = format!("seed {seed}: start #{start} ({} cells)", m.landmass_size[start]);
        for l in 1..nl {
            if l != start && m.landmass_size[l] >= cont {
                line += &format!(" | #{l} {}c direct {} chain max-hop {} in {} hops", m.landmass_size[l], gap[start][l], best[l].0, best[l].1);
            }
        }
        println!("{line}");
    }
}
