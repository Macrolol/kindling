//! Map generator statistics: biome mix, landmasses, generation time.
use civ_core::map::{Biome, Map, NBIOME};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let w: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(256);
    let h: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);
    let s0: u64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let s1: u64 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(10);
    let mut tot = [0f64; NBIOME];
    for seed in s0..=s1 {
        let t = std::time::Instant::now();
        let m = Map::generate(w, h, seed);
        let el = t.elapsed().as_secs_f32();
        let land: Vec<usize> = (0..w * h).filter(|&i| m.is_land(i)).collect();
        let mut c = [0usize; NBIOME];
        for &i in &land { c[m.biome[i] as usize] += 1; }
        let cut = m.continent_cells();
        let conts: Vec<u32> = m.landmass_size.iter().skip(1).copied().filter(|&s| s >= cut).collect();
        let islands = m.landmass_size.iter().skip(1).filter(|&&s| s < cut && s >= 3).count();
        let cu = land.iter().filter(|&&i| m.has(i, civ_core::map::D_COPPER)).count();
        let sn = land.iter().filter(|&&i| m.has(i, civ_core::map::D_TIN)).count();
        let ob = land.iter().filter(|&&i| m.has(i, civ_core::map::D_OBSIDIAN)).count();
        print!("seed {seed:>2}: land {:.0}% lat {:.0}..{:.0} | continents {:?} islands {} | cu {cu} sn {sn} obs {ob} | {:.2}s |", 100.0 * land.len() as f32 / (w * h) as f32, m.lat_n, m.lat_s, conts, islands, el);
        for b in 1..NBIOME {
            let f = c[b] as f64 / land.len() as f64;
            tot[b] += f;
            print!(" {} {:.0}", Biome::from_u8(b as u8).name().get(..4).unwrap_or("ice"), 100.0 * f);
        }
        println!();
    }
    print!("mean %:");
    for b in 1..NBIOME { print!(" {} {:.1}", Biome::from_u8(b as u8).name(), 100.0 * tot[b] / (s1 - s0 + 1) as f64); }
    println!();
}
