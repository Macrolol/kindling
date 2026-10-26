//! Event-log mix of a save: what the rolling log holds, how far back it reaches, and the
//! counters that size the history store. `cargo run --release --example event_mix -- FILE`
use civ_core::world::EventKind;
use std::collections::BTreeMap;
fn main() {
    let p = std::env::args().nth(1).expect("save file");
    let w = civ_core::World::load_bytes(&std::fs::read(&p).unwrap()).expect("load").0;
    let mut by: BTreeMap<u8, (usize, u32, u32)> = BTreeMap::new();
    for e in &w.events {
        let x = by.entry(e.kind).or_insert((0, u32::MAX, 0));
        x.0 += 1;
        x.1 = x.1.min(e.tick);
        x.2 = x.2.max(e.tick);
    }
    println!("tick {} events {} chronicle {} history {}", w.tick, w.events.len(), w.chronicle.len(), w.history.len());
    for (k, (n, a, b)) in by {
        println!("  {:?}: {n} (ticks {a}..{b})", EventKind::from_u8(k));
    }
    let c = &w.counters;
    println!("splits {} deaths {} marriages {} fights {} transfers {} forgets {} absorbed {} fled {} epidemics {} drifts {}", c.splits, c.deaths, c.marriages, c.fights, c.transfers, c.forgets, c.absorbed, c.fled, c.epidemics, c.drift_landings);
    let mut lin: BTreeMap<u32, u32> = BTreeMap::new();
    let mut lm: BTreeMap<u16, u32> = BTreeMap::new();
    for b in &w.bands {
        *lin.entry(b.lineage).or_default() += b.pop;
        *lm.entry(w.map.landmass_of(b.cell(w.map.w))).or_default() += b.pop;
    }
    println!("bands {} lineages alive {} landmasses settled {} items {}", w.bands.len(), lin.len(), lm.len(), w.item_bands.len());
}
