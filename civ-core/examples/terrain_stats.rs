use civ_core::map::{nbrs8, Map};
fn main() {
    for seed in [1u64, 4, 7] {
        let m = Map::generate(256, 160, seed);
        let mut hs = vec![];
        let mut sl = vec![];
        for i in 0..m.w * m.h {
            if !m.is_land(i) { continue; }
            hs.push(m.height[i]);
            let (x, y) = (i % m.w, i / m.w);
            let s = nbrs8(x, y, m.w, m.h).map(|(a, b)| (m.height[b * m.w + a].max(0.0) - m.height[i]).abs()).fold(0.0f32, f32::max);
            sl.push(s);
        }
        hs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sl.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |v: &Vec<f32>, q: f32| v[((v.len() - 1) as f32 * q) as usize];
        println!("seed {seed} land {} h p10 {:.0} p50 {:.0} p90 {:.0} p99 {:.0} max {:.0} | slope p50 {:.0} p90 {:.0} p99 {:.0} | river {}", hs.len(), p(&hs,0.1), p(&hs,0.5), p(&hs,0.9), p(&hs,0.99), hs[hs.len()-1], p(&sl,0.5), p(&sl,0.9), p(&sl,0.99), m.river.iter().filter(|&&r| r).count());
    }
}
