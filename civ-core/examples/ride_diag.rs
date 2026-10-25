//! Riding in a saved world: when it began and how far it spread.
//! `cargo run --release --example ride_diag -- file.civsave`
use civ_core::world::{EventKind, World};

fn main() {
    let path = std::env::args().nth(1).expect("save path");
    let (w, _, _) = World::load_bytes(&std::fs::read(&path).unwrap()).expect("load");
    for e in w.chronicle.iter().filter(|e| EventKind::from_u8(e.kind) == EventKind::FirstRiding) {
        println!("first riding Y{:.1} band {} cell {}", e.tick as f32 / 4.0, e.band, e.cell);
    }
    let riders: Vec<_> = w.bands.iter().filter(|b| b.ride > 0.0).collect();
    let mounted = riders.iter().filter(|b| w.mounted(b) > 0.5).count();
    let pop: u32 = riders.iter().map(|b| b.pop).sum();
    let herders = w.bands.iter().filter(|b| b.herd > 0.0).count();
    println!(
        "Y{:.0}: bands {} herders {} riding {} (mounted >0.5 now {}) riders' pop {} of {}",
        w.tick as f32 / 4.0, w.bands.len(), herders, riders.len(), mounted, pop, w.total_pop()
    );
}
