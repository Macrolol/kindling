//! Load a save (read-only), run it forward N years in the current build, print new world firsts.
//! `cargo run --release --example continue_save -- file.civsave 300`
use civ_core::materials::*;
use civ_core::world::World;

fn main() {
    let path = std::env::args().nth(1).expect("save path");
    let years: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let bytes = std::fs::read(&path).unwrap();
    let (mut w, _view, info) = World::load_bytes(&bytes).expect("load");
    let n0 = w.firsts.len();
    println!(
        "trade overlay after load: {} of {} cells non-zero",
        w.trade_flow.iter().filter(|&&q| q > 0.0).count(),
        w.trade_flow.len()
    );
    let y0 = w.tick as f32 / 4.0;
    println!(
        "loaded {path}: file v{} migrated_from {:?} | Y{:.1} bands {} | {} firsts",
        info.format_version,
        info.migrated_from,
        y0,
        w.bands.len(),
        n0
    );
    let t = std::time::Instant::now();
    for i in 0..years * 4 {
        w.step();
        if i % 400 == 399 {
            println!(
                "  Y{:.0} bands {} pop {} firsts {} ({:.0}s)",
                w.tick as f32 / 4.0,
                w.bands.len(),
                w.bands.iter().map(|b| b.pop).sum::<u32>(),
                w.firsts.len(),
                t.elapsed().as_secs_f32()
            );
        }
    }
    println!("new firsts after Y{:.0}:", y0);
    for f in &w.firsts[n0..] {
        println!("   Y{:>7.1} {}", f.tick as f32 / 4.0, item_name(f.item));
    }
}
