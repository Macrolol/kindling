//! Where the wild ancestors of crops and herd animals live, per seed (generates worlds only):
//! `cargo run --release --example wild_map -- 1 10` -> share of land cells with wild grain / wild
//! herds, how many separate patches, and how far (cells) the nearest patch lies from the founders.
use civ_core::params::{Config, Params};
use civ_core::world::farming::{crop_potential, wild_progenitors, WILD_GRAIN, WILD_HERD};
use civ_core::world::World;

fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    for seed in a[0]..=a[1] {
        let w = World::new(Config { seed, ..Default::default() }, Params::default());
        let m = &w.map;
        let n = m.w * m.h;
        let start = w.bands[0].cell(m.w);
        let start_lm = m.landmass[start];
        let wild: Vec<u8> = (0..n).map(|i| wild_progenitors(m, i)).collect();
        let land = (0..n).filter(|&i| m.is_land(i)).count();
        let good = (0..n).filter(|&i| crop_potential(m, i) > 0.3).count();
        let mut line = format!("seed {seed}: land {land}, grain-capable (pot>0.3) {good}");
        for (bit, name) in [(WILD_GRAIN, "wild grain"), (WILD_HERD, "wild herds")] {
            let cells: Vec<usize> = (0..n).filter(|&i| wild[i] & bit != 0).collect();
            let on_start = cells.iter().filter(|&&i| m.landmass[i] == start_lm).count();
            // patches: 8-connected components
            let mut seen = vec![false; n];
            let mut patches = 0;
            for &c in &cells {
                if seen[c] {
                    continue;
                }
                patches += 1;
                let mut st = vec![c];
                seen[c] = true;
                while let Some(i) = st.pop() {
                    let (x, y) = ((i % m.w) as i32, (i / m.w) as i32);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (nx, ny) = (x + dx, y + dy);
                            if nx < 0 || ny < 0 || nx >= m.w as i32 || ny >= m.h as i32 {
                                continue;
                            }
                            let j = ny as usize * m.w + nx as usize;
                            if !seen[j] && wild[j] & bit != 0 {
                                seen[j] = true;
                                st.push(j);
                            }
                        }
                    }
                }
            }
            let (sx, sy) = ((start % m.w) as i32, (start / m.w) as i32);
            let near = cells
                .iter()
                .filter(|&&i| m.landmass[i] == start_lm)
                .map(|&i| ((i % m.w) as i32 - sx).abs().max(((i / m.w) as i32 - sy).abs()))
                .min();
            line += &format!(
                " | {name}: {} cells ({:.0}% of land), {patches} patches, {on_start} on the start landmass, nearest {}",
                cells.len(),
                100.0 * cells.len() as f32 / land as f32,
                near.map(|d| format!("{d} cells")).unwrap_or("none".into())
            );
        }
        println!("{line}");
    }
}
