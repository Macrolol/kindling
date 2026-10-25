//! Save round-trip check: load a save (or run a fresh world), save, reload, step both and report
//! the first difference. `rt_diag <file.civsave> [steps]` or `rt_diag fresh <seed> <years> WxH`.
use civ_core::params::{Config, Params};
use civ_core::World;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut w = if a[1] == "fresh" {
        let (mw, mh) = a[4]
            .split_once('x')
            .map(|(x, y)| (x.parse::<usize>().unwrap(), y.parse::<usize>().unwrap()))
            .unwrap();
        let cfg = Config {
            seed: a[2].parse().unwrap(),
            map_w: mw,
            map_h: mh,
            ..Default::default()
        };
        let mut w = World::new(cfg, Params::default());
        for _ in 0..a[3].parse::<u32>().unwrap() * 4 {
            w.step();
        }
        w
    } else {
        let b = std::fs::read(&a[1]).unwrap();
        let (mut w, _, _) = World::load_bytes(&b).unwrap();
        let n: u32 = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(0);
        for _ in 0..n {
            w.step();
        }
        w
    };
    let again = w.save_bytes(None, "t", "t").unwrap();
    let (mut w2, _, _) = World::load_bytes(&again).unwrap();
    for s in 0..8 {
        w.step();
        w2.step();
        if w.state_hash() != w2.state_hash() {
            println!(
                "diverged at step {s}: bands {} vs {}",
                w.bands.len(),
                w2.bands.len()
            );
            for (x, y) in w.bands.iter().zip(&w2.bands) {
                if x.id != y.id
                    || x.pop != y.pop
                    || x.x != y.x
                    || x.y != y.y
                    || x.known.len() != y.known.len()
                {
                    println!("first band diff id {} / {}: pop {} {} pos {},{} {},{} known {} {} sow {} {} herd {} {} sick {} {} settled {} {}",
                        x.id, y.id, x.pop, y.pop, x.x, x.y, y.x, y.y, x.known.len(), y.known.len(), x.sow, y.sow, x.herd, y.herd, x.sick, y.sick, x.settled, y.settled);
                    break;
                }
            }
            return;
        }
    }
    println!("round trip OK");
}
