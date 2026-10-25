//! Read-only: why do herds not start? Counts bands standing on wild-herd ground and which of the
//! start conditions they meet. `cargo run --release --example herd_diag -- <seed> <years> [WxH]`
use civ_core::materials::NONE;
use civ_core::params::{Config, Params};
use civ_core::world::farming::WILD_HERD;
use civ_core::World;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a[1].parse().unwrap();
    let years: u32 = a[2].parse().unwrap();
    let mut cfg = Config { seed, ..Default::default() };
    if let Some(m) = a.get(3) {
        let (x, y) = m.split_once('x').unwrap();
        cfg.map_w = x.parse().unwrap();
        cfg.map_h = y.parse().unwrap();
    }
    let mut w = World::new(cfg, Params::default());
    for y in 1..=years {
        for _ in 0..4 {
            w.step();
        }
        if y % 50 == 0 {
            let mw = w.map.w;
            let (mut on, mut cord, mut gp, mut game, mut settled, mut all) = (0, 0, 0, 0, 0, 0);
            let mut sett = 0.0;
            for b in &w.bands {
                let ci = b.cell(mw);
                if w.wild.get(ci).copied().unwrap_or(0) & WILD_HERD == 0 {
                    continue;
                }
                on += 1;
                sett += b.settled;
                let c1 = b.d.kit.cord != NONE;
                let c2 = w.graze_pot[ci] > 0.4;
                let c3 = w.game[ci] > 3.0;
                let c4 = b.settled > 0.1 || b.sow > 0.0;
                cord += c1 as u32;
                gp += c2 as u32;
                game += c3 as u32;
                settled += c4 as u32;
                all += (c1 && c2 && c3 && c4) as u32;
            }
            println!(
                "Y{y}: bands {} | on wild-herd ground {on} (mean settled {:.3}) | cord {cord} gp>0.4 {gp} game>5 {game} settled/sow {settled} | all four {all} | herding bands {}",
                w.bands.len(),
                sett / on.max(1) as f32,
                w.bands.iter().filter(|b| b.herd > 0.0).count()
            );
        }
    }
}
