//! What makes up a save: each top-level `World` field (and each band and map field) encoded on
//! its own the way saves are (named MessagePack, deflate level 6), raw and compressed.
//! `cargo run --release --example save_breakdown -- file.civsave`
use civ_core::world::World;
use serde::Serialize;
use std::io::Write;

fn sz<T: Serialize + ?Sized>(v: &T) -> (usize, usize) {
    let raw = rmp_serde::to_vec_named(v).unwrap();
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
    e.write_all(&raw).unwrap();
    (raw.len(), e.finish().unwrap().len())
}

fn main() {
    let path = std::env::args().nth(1).expect("save path");
    let bytes = std::fs::read(&path).unwrap();
    let (w, _, info) = World::load_bytes(&bytes).expect("load");
    if std::env::var("V10").is_ok() {
        v10_parts(&w);
        return;
    }
    println!("{path}: {:.1} MB on disk, format v{}, map {}x{}, {} bands, year {}", bytes.len() as f64 / 1e6, info.format_version, w.map.w, w.map.h, w.bands.len(), w.tick / 4);
    let mut rows: Vec<(String, (usize, usize))> = Vec::new();
    macro_rules! f {
        ($($name:ident),*) => { $( rows.push((stringify!($name).to_string(), sz(&w.$name))); )* };
    }
    f!(cfg, params, map, plants, game, fish, small, bands, reg, events, chronicle, firsts, history, settings_log, counters, item_bands, landfall, camped, landfalls, item_peak, item_lost, trade_flow, conflict_flow, camp, tend, dom, fert, crop, farm_starts, archive);
    rows.push(("cell_first".into(), sz(&w.cell_first.iter().collect::<Vec<_>>())));
    let total: usize = rows.iter().map(|r| r.1 .1).sum();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1 .1));
    println!("{:<16} {:>10} {:>10} {:>6}", "field", "raw MB", "deflate MB", "share");
    for (n, (r, c)) in &rows {
        if *c > 1000 {
            println!("{:<16} {:>10.2} {:>10.2} {:>5.1}%", n, *r as f64 / 1e6, *c as f64 / 1e6, 100.0 * *c as f64 / total as f64);
        }
    }
    println!("sum of parts (deflated separately): {:.1} MB", total as f64 / 1e6);
    // bands, field by field
    let mut br: Vec<(&str, (usize, usize))> = Vec::new();
    macro_rules! b {
        ($($name:ident),*) => { $( br.push((stringify!($name), sz(&w.bands.iter().map(|b| &b.$name).collect::<Vec<_>>()))); )* };
    }
    b!(known, stock, origin, partners, craft, grudges, ancestry, anc, lineage, x, pop, store, far_shore);
    let carry: Vec<_> = w.bands.iter().map(|b| b.d.demand.len()).collect();
    br.sort_by_key(|r| std::cmp::Reverse(r.1 .1));
    println!("-- bands (selected fields; carry demand len {:?})", carry.first());
    for (n, (r, c)) in &br {
        println!("  {:<14} {:>10.2} {:>10.2}", n, *r as f64 / 1e6, *c as f64 / 1e6);
    }
    let nk: usize = w.bands.iter().map(|b| b.known.len()).sum();
    let np: usize = w.bands.iter().map(|b| b.partners.len()).sum();
    let ng: usize = w.bands.iter().map(|b| b.grudges.len()).sum();
    println!("  known entries {nk} ({:.0}/band), partners {np}, grudges {ng}", nk as f64 / w.bands.len().max(1) as f64);
    let m = &w.map;
    let mut mr: Vec<(&str, (usize, usize))> = vec![
        ("height", sz(&m.height)), ("temp", sz(&m.temp)), ("tamp", sz(&m.tamp)), ("rain", sz(&m.rain)), ("flow", sz(&m.flow)),
        ("river", sz(&m.river)), ("biome", sz(&m.biome)), ("deposits", sz(&m.deposits)), ("k_plants", sz(&m.k_plants)),
        ("k_game", sz(&m.k_game)), ("k_fish", sz(&m.k_fish)), ("k_wood", sz(&m.k_wood)), ("k_fibre", sz(&m.k_fibre)), ("k_small", sz(&m.k_small)),
    ];
    mr.sort_by_key(|r| std::cmp::Reverse(r.1 .1));
    println!("-- map layers");
    for (n, (r, c)) in &mr {
        println!("  {:<14} {:>10.2} {:>10.2}", n, *r as f64 / 1e6, *c as f64 / 1e6);
    }
}

/// Sizes of the v10 compact parts (`civ_core::codec`), section by section.
#[allow(dead_code)]
pub fn v10_parts(w: &civ_core::World) {
    use std::io::Write;
    let defl = |b: &[u8]| {
        let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
        e.write_all(b).unwrap();
        e.finish().unwrap().len() as f64 / 1e6
    };
    let blob = civ_core::codec::encode_bands(&w.bands).unwrap();
    let mut r = civ_core::codec::Rd::new(&blob);
    r.u().unwrap();
    println!("-- v10 bands blob: {:.2} MB raw, {:.2} MB deflated", blob.len() as f64 / 1e6, defl(&blob));
    for name in ["rows", "known", "partners", "craft"] {
        let s = r.bytes().unwrap();
        println!("  {name:<10} {:>8.2} {:>8.2}", s.len() as f64 / 1e6, defl(s));
    }
    let m = civ_core::codec::encode_map(&w.map).unwrap();
    println!("-- v10 map blob: {:.2} MB raw, {:.2} MB deflated", m.len() as f64 / 1e6, defl(&m));
}
