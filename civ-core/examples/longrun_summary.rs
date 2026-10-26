//! Compact long-run summary of saved worlds, one block per save: population, war and disease
//! deaths, farmers and the largest lineage per century, landfalls, key firsts, riding.
//! `cargo run --release --example longrun_summary -- a.civsave b.civsave ...`
use civ_core::materials::*;
use civ_core::world::{EventKind, World};

const KEY: [&str; 14] = [
    "fire", "fired pottery", "dugout canoe", "outrigger (any)", "plank boat", "salt", "copper (metal)",
    "tin (metal)", "bronze (metal)", "iron bloom", "wrought iron", "sinew-strung bow", "carrying frame (plant fibre)", "bone beads",
];

fn main() {
    for path in std::env::args().skip(1) {
        let (w, _, _) = World::load_bytes(&std::fs::read(&path).unwrap()).expect("load");
        println!("== seed {} | map {}x{} | Y{:.0} | pop {} bands {}", w.cfg.seed, w.map.w, w.map.h, w.tick as f32 / 4.0, w.total_pop(), w.bands.len());
        println!("   century   pop(end)  war%/yr  dis%/yr  farmers%  top-lineage%  epidemics");
        for ch in w.history.chunks(100) {
            let n = ch.len() as f32;
            let pm = ch.iter().map(|h| h.pop as f32).sum::<f32>() / n;
            let k: u64 = ch.iter().map(|h| h.conflict_deaths as u64).sum();
            let d: u64 = ch.iter().map(|h| h.disease_deaths as u64).sum();
            let ep: u64 = ch.iter().map(|h| h.epidemics as u64).sum();
            let l = ch.last().unwrap();
            let lt: Vec<f32> = ch.iter().map(|y| y.top_lineage_pct).filter(|&p| p > 0.0).collect();
            println!(
                "   Y{:>4}-{:<4} {:>9} {:>7.3} {:>8.3} {:>8.0} {:>12.0} {:>10}",
                ch[0].year, l.year, l.pop, 100.0 * k as f32 / n / pm.max(1.0), 100.0 * d as f32 / n / pm.max(1.0),
                100.0 * l.farmer_pop as f32 / l.pop.max(1) as f32, lt.iter().sum::<f32>() / lt.len().max(1) as f32, ep
            );
        }
        let lf: Vec<String> = w.landfall.iter().filter(|&&t| t != u32::MAX && t > 0).map(|&t| format!("Y{:.0}", t as f32 / 4.0)).collect();
        println!("   landfalls on new landmasses: {} [{}] | crossings {} voyages {} lost at sea {}", lf.len(), lf.join(" "), w.counters.crossings, w.counters.voyages, w.counters.lost_at_sea);
        let firsts: Vec<String> = KEY
            .iter()
            .map(|k| match w.firsts.iter().filter(|f| if *k == "outrigger (any)" { form(f.item) == Form::Outrigger } else { item_name(f.item) == *k }).map(|f| f.tick).min() {
                Some(t) => format!("{k} Y{:.0}", t as f32 / 4.0),
                None => format!("{k} -"),
            })
            .collect();
        println!("   firsts: {}", firsts.join(", "));
        let ev = |k: EventKind| w.chronicle.iter().filter(|e| EventKind::from_u8(e.kind) == k).map(|e| format!("Y{:.0}", e.tick as f32 / 4.0)).next().unwrap_or("-".into());
        let riders: Vec<_> = w.bands.iter().filter(|b| b.ride > 0.0).collect();
        let rp: u32 = riders.iter().map(|b| b.pop).sum();
        println!(
            "   first herding {} plough {} epidemic {} riding {} | riding bands now {} ({:.0}% of people)",
            ev(EventKind::FirstHerding), ev(EventKind::FirstPlough), ev(EventKind::FirstEpidemic), ev(EventKind::FirstRiding),
            riders.len(), 100.0 * rp as f32 / w.total_pop().max(1) as f32
        );
    }
}
