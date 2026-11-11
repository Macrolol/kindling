//! EXPERIMENT for PROPOSAL-replay.md, not used by the sim.
//!
//! Throwaway measurement of a viewing-only lossy picture codec. Loads a save,
//! extracts band rows and the five u8 layers (picture A), steps one year
//! (4 ticks) for picture B, then up to nine further years for a decade gap.
//! Encodes the deflate level-3 baseline and the four ideas in the proposal.
//! Does not write a recording and does not change library code.
//!
//! `cargo run --release -p civ-core --example replay_codec_experiment -- <save>`

use civ_core::band::Band;
use civ_core::materials::NONE;
use civ_core::world::World;
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Instant;

const LAYER_NAMES: [&str; 5] = ["tend", "dom", "fert", "trade", "conflict"];

#[derive(Clone, Debug)]
struct Dot {
    id: u32,
    x: u16,
    y: u16,
    pop: u32,
    lineage: u32,
    flags: u8,
    items: u8,
}

struct Picture {
    year: u32,
    dots: Vec<Dot>,
    layers: [Vec<u8>; 5],
    w: usize,
    h: usize,
}

fn main() {
    haar_self_check();
    qpop_self_check();

    let path = std::env::args().nth(1).expect("usage: replay_codec_experiment <save>");
    let t_load = Instant::now();
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let (mut world, _view, info) = World::load_bytes(&bytes).unwrap_or_else(|e| panic!("load: {e}"));
    world.refresh_all_toolkits();
    let load_ms = elapsed_ms(t_load);
    log(&format!(
        "loaded {path} ({:.1} MB on disk) format v{} in {load_ms:.0} ms",
        bytes.len() as f64 / 1e6,
        info.format_version
    ));

    let mut pics: Vec<Picture> = Vec::new();
    pics.push(snap(&world));
    log(&format!(
        "picture A  year {}  map {}x{}  bands {}  pop {}",
        pics[0].year,
        pics[0].w,
        pics[0].h,
        pics[0].dots.len(),
        pics[0].dots.iter().map(|d| d.pop as u64).sum::<u64>()
    ));

    let t_year = Instant::now();
    for _ in 0..4 {
        world.step();
    }
    let year_ms = elapsed_ms(t_year);
    pics.push(snap(&world));
    log(&format!(
        "picture B  year {}  bands {}  pop {}  sim_one_year_ms {year_ms:.0}",
        pics[1].year,
        pics[1].dots.len(),
        pics[1].dots.iter().map(|d| d.pop as u64).sum::<u64>()
    ));

    let mut decade_ms = 0.0;
    // One year on this map is cheap enough that nine more (a decade gap) is in budget.
    // A pathological save that already spent two minutes on one year is skipped.
    let do_decade = year_ms < 120_000.0;
    if do_decade {
        let t_dec = Instant::now();
        for k in 0..9 {
            for _ in 0..4 {
                world.step();
            }
            pics.push(snap(&world));
            log(&format!(
                "picture +{}  year {}  bands {}  sim_so_far_ms {:.0}",
                k + 2,
                pics.last().unwrap().year,
                pics.last().unwrap().dots.len(),
                elapsed_ms(t_dec)
            ));
        }
        decade_ms = elapsed_ms(t_dec);
    } else {
        log("decade gap skipped: one year was not cheap");
    }

    let a = &pics[0];
    let b = &pics[1];
    print_change_rates("A_to_B", &a.dots, &b.dots);
    print_layer_fill(b);

    println!("\n== 1 bands (deflate level 3)");
    println!(
        "{:<32} {:>10} {:>10} {:>8} {:>10} {:>10} {:>8} {:>8}",
        "method", "bytes", "raw", "max_cheb", "mean_euclid", "max_rel_pop", "mean_rel", "enc_ms"
    );
    report_bands("keyframe_packed_B", &pack_bands(&b.dots), &b.dots, &b.dots, false);
    report_bands("keyframe_msgpack_B", &pack_msgpack(&b.dots), &b.dots, &b.dots, false);
    report_bands("keyframe_qpop_B", &pack_bands_qpop(&b.dots), &decode_packed_qpop(&pack_bands_qpop(&b.dots)), &b.dots, true);

    let spike_raw = encode_spike_delta(&a.dots, &b.dots, false);
    report_bands("spike_whole_row_delta", &spike_raw, &b.dots, &b.dots, false);
    let spike_q = encode_spike_delta(&a.dots, &b.dots, true);
    let shown_q = shown_from_qpop_delta(&a.dots, &b.dots);
    report_bands("spike_whole_row_qpop", &spike_q, &shown_q, &b.dots, true);

    let (field_raw, shown_raw) = encode_field_delta(&a.dots, &b.dots, FieldMode::Exact);
    check_dots("field_exact", &shown_raw, &b.dots, true);
    report_bands("field_delta_exact", &field_raw, &shown_raw, &b.dots, true);

    let (field_q, shown_field_q) = encode_field_delta(&a.dots, &b.dots, FieldMode::Qpop);
    check_qpop_dots("field_qpop", &shown_field_q, &b.dots);
    report_bands("field_delta_qpop2", &field_q, &shown_field_q, &b.dots, true);

    if do_decade {
        println!("\n== 1b year-to-year band deltas across the decade (mean of 10 steps)");
        let mut sum_spike = 0usize;
        let mut sum_spike_q = 0usize;
        let mut sum_field_q = 0usize;
        let mut sum_kf = 0usize;
        let mut sum_q_changed = 0u64;
        let mut sum_exact_changed = 0u64;
        let mut sum_bands = 0u64;
        let n = 10usize;
        for i in 0..n {
            let prev = &pics[i];
            let next = &pics[i + 1];
            sum_kf += deflate3(&pack_bands(&next.dots)).len();
            sum_spike += deflate3(&encode_spike_delta(&prev.dots, &next.dots, false)).len();
            sum_spike_q += deflate3(&encode_spike_delta(&prev.dots, &next.dots, true)).len();
            let (fq, _) = encode_field_delta(&prev.dots, &next.dots, FieldMode::Qpop);
            sum_field_q += deflate3(&fq).len();
            let (exact_c, q_c, nb) = change_counts(&prev.dots, &next.dots);
            sum_exact_changed += exact_c;
            sum_q_changed += q_c;
            sum_bands += nb;
        }
        println!(
            "mean keyframe_packed {:>8} B   spike_whole_row {:>8} B   spike_qpop {:>8} B   field_qpop {:>8} B",
            sum_kf / n,
            sum_spike / n,
            sum_spike_q / n,
            sum_field_q / n
        );
        println!(
            "mean share of bands with any exact change {:.1}%   any change after qpop2 {:.1}%",
            100.0 * sum_exact_changed as f64 / sum_bands as f64,
            100.0 * sum_q_changed as f64 / sum_bands as f64
        );
    }

    println!("\n== 2 layers as fields (picture B, deflate level 3, error vs plain u8)");
    println!(
        "{:<28} {:>10} {:>10} {:>8} {:>10} {:>8}",
        "method", "bytes", "raw", "max_u8", "mean_u8", "enc_ms"
    );
    let base_layers = concat_layers(&b.layers);
    report_layer("baseline_u8_deflate", &base_layers, &base_layers, &base_layers);

    for ts in [4usize, 8, 16] {
        let (payload, recon) = encode_tiles_all(&b.layers, b.w, b.h, ts);
        let got = decode_tiles_all(&payload, b.w, b.h, ts, 5);
        check_layers_eq(&format!("tile{ts} recon"), &recon, &got);
        report_layer(&format!("tile{ts}_mean"), &payload, &recon, &base_layers);
    }
    for t in [0u8, 4, 8, 16] {
        let (payload, recon) = encode_quad_all(&b.layers, b.w, b.h, t);
        let got = decode_quad_all(&payload, b.w, b.h, t, 5);
        check_layers_eq(&format!("quad{t} recon"), &recon, &got);
        if t == 0 {
            check_layers_eq("quad0 lossless", &got, &base_layers);
        }
        report_layer(&format!("quad_T{t}"), &payload, &recon, &base_layers);
    }
    for q in [1i32, 2, 4, 8, 16] {
        let (payload, recon) = encode_haar_all(&b.layers, b.w, b.h, q);
        let got = decode_haar_all(&payload, b.w, b.h, q, 5);
        check_layers_eq(&format!("haar{q} recon"), &recon, &got);
        if q == 1 {
            check_layers_eq("haar_Q1 lossless", &got, &base_layers);
        }
        report_layer(&format!("haar8_Q{q}"), &payload, &recon, &base_layers);
    }

    println!("\n== 2b per layer, baseline vs smallest method with max error <= 8");
    per_layer_best(b);

    println!("\n== 3 activity (tiles, picture B vs reference A)");
    println!(
        "{:<32} {:>10} {:>8} {:>10} {:>8} {:>8} {:>8}",
        "method", "bytes", "max_u8", "mean_u8", "enc_ms", "sent", "skip"
    );
    run_activity("act16_exact_skip0", a, b, 16, 0, 0);
    run_activity("act16_lod_budget8", a, b, 16, 0, 8);
    run_activity("act16_rare_skip8_budget8", a, b, 16, 8, 8);
    run_activity("act8_lod_budget8", a, b, 8, 0, 8);
    run_activity("act8_exact_skip0", a, b, 8, 0, 0);

    if do_decade {
        let last = pics.last().unwrap();
        println!("\n== 3b activity across a decade gap (year+10 vs A) and mean of 10 year-steps");
        run_activity("decade_gap_act16_exact", a, last, 16, 0, 0);
        run_activity("decade_gap_act16_lod8", a, last, 16, 0, 8);
        run_activity("decade_gap_act16_rare8", a, last, 16, 8, 8);
        let mut sum_exact = 0usize;
        let mut sum_lod = 0usize;
        let mut sum_rare = 0usize;
        let mut sum_base = 0usize;
        for i in 0..10 {
            sum_base += deflate3(&concat_layers(&pics[i + 1].layers)).len();
            sum_exact += activity_bytes(&pics[i], &pics[i + 1], 16, 0, 0);
            sum_lod += activity_bytes(&pics[i], &pics[i + 1], 16, 0, 8);
            sum_rare += activity_bytes(&pics[i], &pics[i + 1], 16, 8, 8);
        }
        println!(
            "mean year baseline_layers {sum_base_b} B   act16_exact {sum_exact_b} B   act16_lod8 {sum_lod_b} B   act16_rare8 {sum_rare_b} B",
            sum_base_b = sum_base / 10,
            sum_exact_b = sum_exact / 10,
            sum_lod_b = sum_lod / 10,
            sum_rare_b = sum_rare / 10
        );
    }

    if do_decade {
        println!("\n== 4 viewer interpolation between year 0 and year 10, corrections for years 1..9");
        run_idea4(&pics);
    } else {
        println!("\n== 4 decade interpolation: skipped");
    }

    println!("\n== SUMMARY");
    println!("save={path}");
    println!("map={}x{} year_A={} bands_A={} pop_A={}", a.w, a.h, a.year, a.dots.len(), a.dots.iter().map(|d| d.pop as u64).sum::<u64>());
    println!("bands_B={}", b.dots.len());
    println!("sim_one_year_ms={year_ms:.1}");
    println!("sim_nine_more_years_ms={decade_ms:.1}");
    println!("decade={}", if do_decade { "yes" } else { "no" });
    println!("baseline_picture_B_bytes={}", deflate3(&pack_bands(&b.dots)).len() + deflate3(&concat_layers(&b.layers)).len());
    println!("baseline_bands_packed_B={}", deflate3(&pack_bands(&b.dots)).len());
    println!("baseline_bands_msgpack_B={}", deflate3(&pack_msgpack(&b.dots)).len());
    println!("baseline_layers_B={}", deflate3(&concat_layers(&b.layers)).len());
    let (fq, _) = encode_field_delta(&a.dots, &b.dots, FieldMode::Qpop);
    println!("field_qpop_delta_A_to_B={}", deflate3(&fq).len());
    println!("spike_qpop_delta_A_to_B={}", deflate3(&encode_spike_delta(&a.dots, &b.dots, true)).len());
    println!("spike_raw_delta_A_to_B={}", deflate3(&encode_spike_delta(&a.dots, &b.dots, false)).len());
    println!("800x800=pending");
}

fn log(s: &str) {
    println!("{s}");
    let _ = std::io::stdout().flush();
}

fn elapsed_ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn snap(w: &World) -> Picture {
    let mut dots: Vec<Dot> = w.bands.iter().map(dot_of).collect();
    dots.sort_by_key(|d| d.id);
    if dots.windows(2).any(|p| p[0].id == p[1].id) {
        panic!("duplicate band id");
    }
    let width = w.map.w;
    let height = w.map.h;
    let n = width * height;
    let layers = [
        quant_unit(&w.tend, n),
        quant_unit(&w.dom, n),
        quant_unit(&w.fert, n),
        quant_flow(&w.trade_flow, n),
        quant_flow(&w.conflict_flow, n),
    ];
    Picture { year: w.tick / 4, dots, layers, w: width, h: height }
}

fn dot_of(b: &Band) -> Dot {
    let mut flags = 0u8;
    if b.flame {
        flags |= 1;
    }
    if b.sow > 0.0 {
        flags |= 2;
    }
    if b.d.kit.boat != NONE {
        flags |= 4;
    }
    if b.herd > 0.0 {
        flags |= 8;
    }
    Dot {
        id: b.id,
        x: b.x,
        y: b.y,
        pop: b.pop,
        lineage: b.lineage,
        flags,
        items: b.n_items().min(255) as u8,
    }
}

fn quant_unit(v: &[f32], n: usize) -> Vec<u8> {
    let mut o = vec![0u8; n];
    for i in 0..n {
        let x = if i < v.len() { v[i] } else { 0.0 };
        o[i] = (x * 255.0).clamp(0.0, 255.0) as u8;
    }
    o
}

fn quant_flow(v: &[f32], n: usize) -> Vec<u8> {
    let mut o = vec![0u8; n];
    for i in 0..n {
        let mut x = if i < v.len() { v[i] } else { 0.0 };
        if x < 0.0 {
            x = 0.0;
        }
        o[i] = ((1.0 + x).ln() * 40.0).clamp(0.0, 255.0) as u8;
    }
    o
}

fn deflate3(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(3));
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn pack_bands(dots: &[Dot]) -> Vec<u8> {
    let mut out = Vec::with_capacity(dots.len() * 18);
    for d in dots {
        out.extend_from_slice(&d.id.to_le_bytes());
        out.extend_from_slice(&d.x.to_le_bytes());
        out.extend_from_slice(&d.y.to_le_bytes());
        out.extend_from_slice(&d.pop.to_le_bytes());
        out.extend_from_slice(&d.lineage.to_le_bytes());
        out.push(d.flags);
        out.push(d.items);
    }
    out
}

fn pack_bands_qpop(dots: &[Dot]) -> Vec<u8> {
    let mut q: Vec<Dot> = dots.to_vec();
    for d in &mut q {
        d.pop = qpop(d.pop);
    }
    pack_bands(&q)
}

fn decode_packed_qpop(raw: &[u8]) -> Vec<Dot> {
    let mut dots = Vec::with_capacity(raw.len() / 18);
    let mut i = 0;
    while i + 18 <= raw.len() {
        let id = u32::from_le_bytes(raw[i..i + 4].try_into().unwrap());
        let x = u16::from_le_bytes(raw[i + 4..i + 6].try_into().unwrap());
        let y = u16::from_le_bytes(raw[i + 6..i + 8].try_into().unwrap());
        let pop = u32::from_le_bytes(raw[i + 8..i + 12].try_into().unwrap());
        let lineage = u32::from_le_bytes(raw[i + 12..i + 16].try_into().unwrap());
        let flags = raw[i + 16];
        let items = raw[i + 17];
        dots.push(Dot { id, x, y, pop, lineage, flags, items });
        i += 18;
    }
    dots
}

fn pack_msgpack(dots: &[Dot]) -> Vec<u8> {
    let tuples: Vec<(u32, u16, u16, u32, u32, u8, u8)> = dots
        .iter()
        .map(|d| (d.id, d.x, d.y, d.pop, d.lineage, d.flags, d.items))
        .collect();
    rmp_serde::to_vec(&tuples).unwrap()
}

/// Two significant figures. Exact below 100. Max relative error is under 5%.
fn qpop(p: u32) -> u32 {
    if p < 100 {
        return p;
    }
    let mut scale = 1u32;
    let mut probe = p;
    while probe >= 1000 {
        probe /= 10;
        scale *= 10;
    }
    let step = scale * 10;
    let half = step / 2;
    p.saturating_add(half) / step * step
}

fn qpop_self_check() {
    for p in 0..20_000u32 {
        let q = qpop(p);
        if p == 0 {
            assert_eq!(q, 0);
            continue;
        }
        let rel = (q as f64 - p as f64).abs() / p as f64;
        assert!(rel <= 0.05 + 1e-9, "qpop {p} -> {q} rel {rel}");
    }
}

fn put_uvar(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            break;
        }
    }
}

fn put_zvar(out: &mut Vec<u8>, v: i32) {
    let u = ((v << 1) ^ (v >> 31)) as u32;
    put_uvar(out, u);
}

fn get_uvar(b: &[u8], i: &mut usize) -> u32 {
    let mut v = 0u32;
    let mut s = 0;
    loop {
        if *i >= b.len() || s > 28 {
            panic!("bad uvar");
        }
        let byte = b[*i];
        *i += 1;
        v |= u32::from(byte & 0x7f) << s;
        if byte & 0x80 == 0 {
            return v;
        }
        s += 7;
    }
}

fn get_zvar(b: &[u8], i: &mut usize) -> i32 {
    let u = get_uvar(b, i);
    ((u >> 1) as i32) ^ -((u & 1) as i32)
}

#[derive(Clone, Copy)]
enum FieldMode {
    Exact,
    Qpop,
}

/// Id-sorted field deltas. `Qpop` omits a population write when 2-sig-fig pop
/// did not change, and stores the quantised value when it did.
fn encode_field_delta(prev: &[Dot], next: &[Dot], mode: FieldMode) -> (Vec<u8>, Vec<Dot>) {
    let mut removes: Vec<u32> = Vec::new();
    let mut adds: Vec<Dot> = Vec::new();
    let mut updates: Vec<u8> = Vec::new();
    let mut n_upd = 0u32;
    let mut prev_upd = 0u32;
    let mut i = 0;
    let mut j = 0;
    while i < prev.len() || j < next.len() {
        if j >= next.len() || (i < prev.len() && prev[i].id < next[j].id) {
            removes.push(prev[i].id);
            i += 1;
        } else if i >= prev.len() || next[j].id < prev[i].id {
            let mut d = next[j].clone();
            if matches!(mode, FieldMode::Qpop) {
                d.pop = qpop(d.pop);
            }
            adds.push(d);
            j += 1;
        } else {
            let pa = &prev[i];
            let pb = &next[j];
            let (pop_a, pop_b) = match mode {
                FieldMode::Exact => (pa.pop, pb.pop),
                FieldMode::Qpop => (qpop(pa.pop), qpop(pb.pop)),
            };
            let mut mask = 0u8;
            if pa.x != pb.x {
                mask |= 1;
            }
            if pa.y != pb.y {
                mask |= 2;
            }
            if pop_a != pop_b {
                mask |= 4;
            }
            if pa.lineage != pb.lineage {
                mask |= 8;
            }
            if pa.flags != pb.flags {
                mask |= 16;
            }
            if pa.items != pb.items {
                mask |= 32;
            }
            if mask != 0 {
                put_uvar(&mut updates, pb.id.wrapping_sub(prev_upd));
                prev_upd = pb.id;
                updates.push(mask);
                if mask & 1 != 0 {
                    put_zvar(&mut updates, i32::from(pb.x) - i32::from(pa.x));
                }
                if mask & 2 != 0 {
                    put_zvar(&mut updates, i32::from(pb.y) - i32::from(pa.y));
                }
                if mask & 4 != 0 {
                    put_uvar(&mut updates, pop_b);
                }
                if mask & 8 != 0 {
                    put_uvar(&mut updates, pb.lineage);
                }
                if mask & 16 != 0 {
                    updates.push(pb.flags);
                }
                if mask & 32 != 0 {
                    updates.push(pb.items);
                }
                n_upd += 1;
            }
            i += 1;
            j += 1;
        }
    }

    let mut out = Vec::new();
    put_uvar(&mut out, removes.len() as u32);
    put_uvar(&mut out, adds.len() as u32);
    put_uvar(&mut out, n_upd);
    let mut prev_id = 0u32;
    for id in &removes {
        put_uvar(&mut out, id.wrapping_sub(prev_id));
        prev_id = *id;
    }
    prev_id = 0;
    for d in &adds {
        put_uvar(&mut out, d.id.wrapping_sub(prev_id));
        prev_id = d.id;
        put_uvar(&mut out, u32::from(d.x));
        put_uvar(&mut out, u32::from(d.y));
        put_uvar(&mut out, d.pop);
        put_uvar(&mut out, d.lineage);
        out.push(d.flags);
        out.push(d.items);
    }
    out.extend_from_slice(&updates);

    let shown = decode_field_delta(prev, &out, mode);
    (out, shown)
}

fn decode_field_delta(prev: &[Dot], bytes: &[u8], mode: FieldMode) -> Vec<Dot> {
    let mut i = 0;
    let n_rem = get_uvar(bytes, &mut i) as usize;
    let n_add = get_uvar(bytes, &mut i) as usize;
    let n_upd = get_uvar(bytes, &mut i) as usize;
    let mut map: BTreeMap<u32, Dot> = BTreeMap::new();
    for d in prev {
        let mut c = d.clone();
        if matches!(mode, FieldMode::Qpop) {
            c.pop = qpop(c.pop);
        }
        map.insert(c.id, c);
    }
    let mut prev_id = 0u32;
    for _ in 0..n_rem {
        let id = prev_id.wrapping_add(get_uvar(bytes, &mut i));
        prev_id = id;
        map.remove(&id);
    }
    prev_id = 0;
    for _ in 0..n_add {
        let id = prev_id.wrapping_add(get_uvar(bytes, &mut i));
        prev_id = id;
        let x = get_uvar(bytes, &mut i) as u16;
        let y = get_uvar(bytes, &mut i) as u16;
        let pop = get_uvar(bytes, &mut i);
        let lineage = get_uvar(bytes, &mut i);
        let flags = bytes[i];
        i += 1;
        let items = bytes[i];
        i += 1;
        map.insert(id, Dot { id, x, y, pop, lineage, flags, items });
    }
    let mut prev_upd = 0u32;
    for _ in 0..n_upd {
        let id = prev_upd.wrapping_add(get_uvar(bytes, &mut i));
        prev_upd = id;
        let mask = bytes[i];
        i += 1;
        let d = map.get_mut(&id).unwrap_or_else(|| panic!("update missing {id}"));
        if mask & 1 != 0 {
            let dx = get_zvar(bytes, &mut i);
            d.x = (i32::from(d.x) + dx) as u16;
        }
        if mask & 2 != 0 {
            let dy = get_zvar(bytes, &mut i);
            d.y = (i32::from(d.y) + dy) as u16;
        }
        if mask & 4 != 0 {
            d.pop = get_uvar(bytes, &mut i);
        }
        if mask & 8 != 0 {
            d.lineage = get_uvar(bytes, &mut i);
        }
        if mask & 16 != 0 {
            d.flags = bytes[i];
            i += 1;
        }
        if mask & 32 != 0 {
            d.items = bytes[i];
            i += 1;
        }
    }
    if i != bytes.len() {
        panic!("field delta trailing bytes {} vs {i}", bytes.len());
    }
    map.into_values().collect()
}

fn encode_spike_delta(prev: &[Dot], next: &[Dot], quant: bool) -> Vec<u8> {
    let mut removes: Vec<u32> = Vec::new();
    let mut adds: Vec<Dot> = Vec::new();
    let mut ups: Vec<Dot> = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < prev.len() || j < next.len() {
        if j >= next.len() || (i < prev.len() && prev[i].id < next[j].id) {
            removes.push(prev[i].id);
            i += 1;
        } else if i >= prev.len() || next[j].id < prev[i].id {
            let mut d = next[j].clone();
            if quant {
                d.pop = qpop(d.pop);
            }
            adds.push(d);
            j += 1;
        } else {
            let pa = &prev[i];
            let pb = &next[j];
            let pop_diff = if quant { qpop(pa.pop) != qpop(pb.pop) } else { pa.pop != pb.pop };
            let changed = pa.x != pb.x || pa.y != pb.y || pop_diff || pa.lineage != pb.lineage || pa.flags != pb.flags || pa.items != pb.items;
            if changed {
                let mut d = pb.clone();
                if quant {
                    d.pop = qpop(d.pop);
                }
                ups.push(d);
            }
            i += 1;
            j += 1;
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(removes.len() as u32).to_le_bytes());
    for id in removes {
        out.extend_from_slice(&id.to_le_bytes());
    }
    out.extend_from_slice(&(adds.len() as u32).to_le_bytes());
    out.extend_from_slice(&pack_bands(&adds));
    out.extend_from_slice(&(ups.len() as u32).to_le_bytes());
    out.extend_from_slice(&pack_bands(&ups));
    out
}

fn shown_from_qpop_delta(prev: &[Dot], next: &[Dot]) -> Vec<Dot> {
    let mut map: BTreeMap<u32, Dot> = BTreeMap::new();
    for d in prev {
        let mut c = d.clone();
        c.pop = qpop(c.pop);
        map.insert(c.id, c);
    }
    let mut i = 0;
    let mut j = 0;
    while i < prev.len() || j < next.len() {
        if j >= next.len() || (i < prev.len() && prev[i].id < next[j].id) {
            map.remove(&prev[i].id);
            i += 1;
        } else if i >= prev.len() || next[j].id < prev[i].id {
            let mut d = next[j].clone();
            d.pop = qpop(d.pop);
            map.insert(d.id, d);
            j += 1;
        } else {
            let pb = &next[j];
            let e = map.get_mut(&pb.id).unwrap();
            e.x = pb.x;
            e.y = pb.y;
            e.pop = qpop(pb.pop);
            e.lineage = pb.lineage;
            e.flags = pb.flags;
            e.items = pb.items;
            i += 1;
            j += 1;
        }
    }
    map.into_values().collect()
}

fn check_dots(label: &str, shown: &[Dot], truth: &[Dot], exact_pop: bool) {
    assert_eq!(shown.len(), truth.len(), "{label} count");
    for (s, t) in shown.iter().zip(truth.iter()) {
        assert_eq!(s.id, t.id, "{label} id");
        assert_eq!(s.x, t.x, "{label} x");
        assert_eq!(s.y, t.y, "{label} y");
        assert_eq!(s.lineage, t.lineage, "{label} lineage");
        assert_eq!(s.flags, t.flags, "{label} flags");
        assert_eq!(s.items, t.items, "{label} items");
        if exact_pop {
            assert_eq!(s.pop, t.pop, "{label} pop");
        }
    }
}

fn check_qpop_dots(label: &str, shown: &[Dot], truth: &[Dot]) {
    assert_eq!(shown.len(), truth.len(), "{label} count");
    for (s, t) in shown.iter().zip(truth.iter()) {
        assert_eq!(s.id, t.id, "{label} id");
        assert_eq!(s.x, t.x, "{label} x");
        assert_eq!(s.y, t.y, "{label} y");
        assert_eq!(s.pop, qpop(t.pop), "{label} qpop");
        assert_eq!(s.lineage, t.lineage, "{label} lineage");
        assert_eq!(s.flags, t.flags, "{label} flags");
        assert_eq!(s.items, t.items, "{label} items");
    }
}

fn pop_err(shown: &[Dot], truth: &[Dot]) -> (f64, f64, f64, f64) {
    assert_eq!(shown.len(), truth.len());
    let mut max_cheb = 0u32;
    let mut sum_euclid = 0.0f64;
    let mut max_rel = 0.0f64;
    let mut sum_rel = 0.0f64;
    let mut n_rel = 0u32;
    for (s, t) in shown.iter().zip(truth.iter()) {
        let cheb = chebyshev(s.x, s.y, t.x, t.y);
        max_cheb = max_cheb.max(cheb);
        let dx = f64::from(s.x) - f64::from(t.x);
        let dy = f64::from(s.y) - f64::from(t.y);
        sum_euclid += (dx * dx + dy * dy).sqrt();
        if t.pop > 0 {
            let rel = (s.pop as f64 - t.pop as f64).abs() / t.pop as f64;
            max_rel = max_rel.max(rel);
            sum_rel += rel;
            n_rel += 1;
        } else if s.pop != 0 {
            max_rel = max_rel.max(1.0);
            sum_rel += 1.0;
            n_rel += 1;
        }
    }
    let n = shown.len().max(1) as f64;
    let mean_rel = if n_rel == 0 { 0.0 } else { sum_rel / f64::from(n_rel) };
    (f64::from(max_cheb), sum_euclid / n, max_rel, mean_rel)
}

fn chebyshev(x0: u16, y0: u16, x1: u16, y1: u16) -> u32 {
    let dx = (i32::from(x0) - i32::from(x1)).unsigned_abs();
    let dy = (i32::from(y0) - i32::from(y1)).unsigned_abs();
    dx.max(dy)
}

fn report_bands(name: &str, payload: &[u8], shown: &[Dot], truth: &[Dot], already_shown: bool) {
    let t = Instant::now();
    let packed = deflate3(payload);
    let ms = elapsed_ms(t);
    let (max_c, mean_e, max_r, mean_r) = if already_shown {
        pop_err(shown, truth)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };
    println!(
        "{:<32} {:>10} {:>10} {:>8.1} {:>10.3} {:>10.4} {:>8.4} {:>8.2}",
        name,
        packed.len(),
        payload.len(),
        max_c,
        mean_e,
        max_r,
        mean_r,
        ms
    );
}

fn change_counts(prev: &[Dot], next: &[Dot]) -> (u64, u64, u64) {
    let mut exact = 0u64;
    let mut q = 0u64;
    let mut i = 0;
    let mut j = 0;
    while i < prev.len() || j < next.len() {
        if j >= next.len() || (i < prev.len() && prev[i].id < next[j].id) {
            exact += 1;
            q += 1;
            i += 1;
        } else if i >= prev.len() || next[j].id < prev[i].id {
            exact += 1;
            q += 1;
            j += 1;
        } else {
            let pa = &prev[i];
            let pb = &next[j];
            let any = pa.x != pb.x || pa.y != pb.y || pa.pop != pb.pop || pa.lineage != pb.lineage || pa.flags != pb.flags || pa.items != pb.items;
            let any_q = pa.x != pb.x || pa.y != pb.y || qpop(pa.pop) != qpop(pb.pop) || pa.lineage != pb.lineage || pa.flags != pb.flags || pa.items != pb.items;
            if any {
                exact += 1;
            }
            if any_q {
                q += 1;
            }
            i += 1;
            j += 1;
        }
    }
    (exact, q, next.len() as u64)
}

fn print_change_rates(label: &str, prev: &[Dot], next: &[Dot]) {
    let mut both = 0u64;
    let mut pos = 0u64;
    let mut pop = 0u64;
    let mut qpop_c = 0u64;
    let mut lin = 0u64;
    let mut flags = 0u64;
    let mut items = 0u64;
    let mut add = 0u64;
    let mut rem = 0u64;
    let mut i = 0;
    let mut j = 0;
    while i < prev.len() || j < next.len() {
        if j >= next.len() || (i < prev.len() && prev[i].id < next[j].id) {
            rem += 1;
            i += 1;
        } else if i >= prev.len() || next[j].id < prev[i].id {
            add += 1;
            j += 1;
        } else {
            both += 1;
            let pa = &prev[i];
            let pb = &next[j];
            if pa.x != pb.x || pa.y != pb.y {
                pos += 1;
            }
            if pa.pop != pb.pop {
                pop += 1;
            }
            if qpop(pa.pop) != qpop(pb.pop) {
                qpop_c += 1;
            }
            if pa.lineage != pb.lineage {
                lin += 1;
            }
            if pa.flags != pb.flags {
                flags += 1;
            }
            if pa.items != pb.items {
                items += 1;
            }
            i += 1;
            j += 1;
        }
    }
    let nb = both.max(1) as f64;
    println!(
        "{label}: both {both} add {add} rem {rem} | of both, pos {:.1}% pop {:.1}% qpop2 {:.1}% lineage {:.1}% flags {:.1}% items {:.1}%",
        100.0 * pos as f64 / nb,
        100.0 * pop as f64 / nb,
        100.0 * qpop_c as f64 / nb,
        100.0 * lin as f64 / nb,
        100.0 * flags as f64 / nb,
        100.0 * items as f64 / nb
    );
}

fn print_layer_fill(p: &Picture) {
    for (name, layer) in LAYER_NAMES.iter().zip(p.layers.iter()) {
        let n = layer.len().max(1) as f64;
        let nz = layer.iter().filter(|&&v| v != 0).count();
        let mean = layer.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
        println!("layer {name}: nonzero {:.1}% mean {mean:.2}", 100.0 * nz as f64 / n);
    }
}

fn concat_layers(layers: &[Vec<u8>; 5]) -> Vec<u8> {
    let mut out = Vec::with_capacity(layers.iter().map(|l| l.len()).sum());
    for l in layers {
        out.extend_from_slice(l);
    }
    out
}

fn layer_err(recon: &[u8], truth: &[u8]) -> (u8, f64) {
    assert_eq!(recon.len(), truth.len());
    let mut max_e = 0u8;
    let mut sum = 0u64;
    for (a, b) in recon.iter().zip(truth.iter()) {
        let d = a.abs_diff(*b);
        max_e = max_e.max(d);
        sum += u64::from(d);
    }
    let mean = sum as f64 / recon.len().max(1) as f64;
    (max_e, mean)
}

fn report_layer(name: &str, payload: &[u8], recon: &[u8], truth: &[u8]) {
    let t = Instant::now();
    let packed = deflate3(payload);
    let ms = elapsed_ms(t);
    let (max_e, mean_e) = layer_err(recon, truth);
    println!(
        "{:<28} {:>10} {:>10} {:>8} {:>10.3} {:>8.2}",
        name,
        packed.len(),
        payload.len(),
        max_e,
        mean_e,
        ms
    );
}

fn check_layers_eq(label: &str, a: &[u8], b: &[u8]) {
    assert_eq!(a.len(), b.len(), "{label} len");
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            panic!("{label} mismatch at {i}: {x} vs {y}");
        }
    }
}

fn encode_tiles_all(layers: &[Vec<u8>; 5], w: usize, h: usize, ts: usize) -> (Vec<u8>, Vec<u8>) {
    let mut payload = Vec::new();
    let mut recon = Vec::new();
    for layer in layers {
        let (p, r) = encode_tiles(layer, w, h, ts);
        put_uvar(&mut payload, p.len() as u32);
        payload.extend_from_slice(&p);
        recon.extend_from_slice(&r);
    }
    (payload, recon)
}

fn decode_tiles_all(payload: &[u8], w: usize, h: usize, ts: usize, nlayers: usize) -> Vec<u8> {
    let mut i = 0;
    let mut recon = Vec::new();
    for _ in 0..nlayers {
        let n = get_uvar(payload, &mut i) as usize;
        let r = decode_tiles(&payload[i..i + n], w, h, ts);
        i += n;
        recon.extend_from_slice(&r);
    }
    assert_eq!(i, payload.len());
    recon
}

fn encode_tiles(layer: &[u8], w: usize, h: usize, ts: usize) -> (Vec<u8>, Vec<u8>) {
    let tw = (w + ts - 1) / ts;
    let th = (h + ts - 1) / ts;
    let mut means = Vec::with_capacity(tw * th);
    let mut recon = vec![0u8; w * h];
    for ty in 0..th {
        for tx in 0..tw {
            let x0 = tx * ts;
            let y0 = ty * ts;
            let x1 = (x0 + ts).min(w);
            let y1 = (y0 + ts).min(h);
            let mut sum = 0u32;
            let mut n = 0u32;
            for y in y0..y1 {
                for x in x0..x1 {
                    sum += u32::from(layer[y * w + x]);
                    n += 1;
                }
            }
            let m = ((sum + n / 2) / n) as u8;
            means.push(m);
            for y in y0..y1 {
                for x in x0..x1 {
                    recon[y * w + x] = m;
                }
            }
        }
    }
    (means, recon)
}

fn decode_tiles(means: &[u8], w: usize, h: usize, ts: usize) -> Vec<u8> {
    let tw = (w + ts - 1) / ts;
    let th = (h + ts - 1) / ts;
    assert_eq!(means.len(), tw * th);
    let mut recon = vec![0u8; w * h];
    for ty in 0..th {
        for tx in 0..tw {
            let m = means[ty * tw + tx];
            let x0 = tx * ts;
            let y0 = ty * ts;
            let x1 = (x0 + ts).min(w);
            let y1 = (y0 + ts).min(h);
            for y in y0..y1 {
                for x in x0..x1 {
                    recon[y * w + x] = m;
                }
            }
        }
    }
    recon
}

fn encode_quad_all(layers: &[Vec<u8>; 5], w: usize, h: usize, t: u8) -> (Vec<u8>, Vec<u8>) {
    let mut payload = Vec::new();
    let mut recon = Vec::new();
    for layer in layers {
        let mut p = Vec::new();
        quad_write(&mut p, layer, w, 0, 0, w, h, t);
        let r = quad_decode(&p, w, h, t);
        put_uvar(&mut payload, p.len() as u32);
        payload.extend_from_slice(&p);
        recon.extend_from_slice(&r);
    }
    (payload, recon)
}

fn decode_quad_all(payload: &[u8], w: usize, h: usize, t: u8, nlayers: usize) -> Vec<u8> {
    let mut i = 0;
    let mut recon = Vec::new();
    for _ in 0..nlayers {
        let n = get_uvar(payload, &mut i) as usize;
        let r = quad_decode(&payload[i..i + n], w, h, t);
        i += n;
        recon.extend_from_slice(&r);
    }
    assert_eq!(i, payload.len());
    recon
}

fn quad_write(out: &mut Vec<u8>, layer: &[u8], w: usize, x: usize, y: usize, bw: usize, bh: usize, t: u8) {
    let (mn, mx, mean) = range_of(layer, w, x, y, bw, bh);
    if (bw == 1 && bh == 1) || mx - mn <= t {
        out.push(0);
        out.push(mean);
        return;
    }
    out.push(1);
    if bw >= 2 && bh >= 2 {
        let mw = bw / 2;
        let mh = bh / 2;
        quad_write(out, layer, w, x, y, mw, mh, t);
        quad_write(out, layer, w, x + mw, y, bw - mw, mh, t);
        quad_write(out, layer, w, x, y + mh, mw, bh - mh, t);
        quad_write(out, layer, w, x + mw, y + mh, bw - mw, bh - mh, t);
    } else if bw >= 2 {
        let mw = bw / 2;
        quad_write(out, layer, w, x, y, mw, bh, t);
        quad_write(out, layer, w, x + mw, y, bw - mw, bh, t);
    } else {
        let mh = bh / 2;
        quad_write(out, layer, w, x, y, bw, mh, t);
        quad_write(out, layer, w, x, y + mh, bw, bh - mh, t);
    }
}

fn quad_decode(bytes: &[u8], w: usize, h: usize, t: u8) -> Vec<u8> {
    let mut recon = vec![0u8; w * h];
    let mut i = 0;
    quad_read(bytes, &mut i, &mut recon, w, 0, 0, w, h);
    assert_eq!(i, bytes.len(), "quad trailing t={t}");
    recon
}

fn quad_read(bytes: &[u8], i: &mut usize, recon: &mut [u8], w: usize, x: usize, y: usize, bw: usize, bh: usize) {
    let tag = bytes[*i];
    *i += 1;
    if tag == 0 {
        let mean = bytes[*i];
        *i += 1;
        for yy in y..y + bh {
            for xx in x..x + bw {
                recon[yy * w + xx] = mean;
            }
        }
        return;
    }
    assert_eq!(tag, 1);
    if bw >= 2 && bh >= 2 {
        let mw = bw / 2;
        let mh = bh / 2;
        quad_read(bytes, i, recon, w, x, y, mw, mh);
        quad_read(bytes, i, recon, w, x + mw, y, bw - mw, mh);
        quad_read(bytes, i, recon, w, x, y + mh, mw, bh - mh);
        quad_read(bytes, i, recon, w, x + mw, y + mh, bw - mw, bh - mh);
    } else if bw >= 2 {
        let mw = bw / 2;
        quad_read(bytes, i, recon, w, x, y, mw, bh);
        quad_read(bytes, i, recon, w, x + mw, y, bw - mw, bh);
    } else {
        let mh = bh / 2;
        quad_read(bytes, i, recon, w, x, y, bw, mh);
        quad_read(bytes, i, recon, w, x, y + mh, bw, bh - mh);
    }
}

fn range_of(layer: &[u8], w: usize, x: usize, y: usize, bw: usize, bh: usize) -> (u8, u8, u8) {
    let mut mn = 255u8;
    let mut mx = 0u8;
    let mut sum = 0u32;
    let mut n = 0u32;
    for yy in y..y + bh {
        let row = yy * w;
        for xx in x..x + bw {
            let v = layer[row + xx];
            mn = mn.min(v);
            mx = mx.max(v);
            sum += u32::from(v);
            n += 1;
        }
    }
    let mean = ((sum + n / 2) / n) as u8;
    (mn, mx, mean)
}

fn haar_self_check() {
    let mut a = [10, 6, 7, 10, 0, 255, 128, 3];
    let orig = a;
    haar1_fwd(&mut a);
    haar1_inv(&mut a);
    assert_eq!(a, orig);
    let mut b = [0i32; 64];
    for i in 0..64 {
        b[i] = ((i * 17) % 251) as i32;
    }
    let o = b;
    haar2_fwd(&mut b);
    haar2_inv(&mut b);
    assert_eq!(b, o);
    let mut block = [0i32; 64];
    for i in 0..64 {
        block[i] = (i % 8) as i32 * 3;
    }
    let src = block;
    haar2_fwd(&mut block);
    for c in block.iter_mut().skip(1) {
        *c = quant_i(*c, 1);
    }
    for c in block.iter_mut().skip(1) {
        *c = dequant_i(*c, 1);
    }
    haar2_inv(&mut block);
    assert_eq!(block, src);
}

fn haar1_fwd(a: &mut [i32]) {
    let mut len = a.len();
    let mut tmp = vec![0i32; a.len()];
    while len > 1 {
        let half = len / 2;
        for i in 0..half {
            let l = a[2 * i];
            let r = a[2 * i + 1];
            let d = r - l;
            tmp[i] = l + d.div_euclid(2);
            tmp[half + i] = d;
        }
        a[..len].copy_from_slice(&tmp[..len]);
        len = half;
    }
}

fn haar1_inv(a: &mut [i32]) {
    let n = a.len();
    let mut len = 2;
    let mut tmp = vec![0i32; n];
    while len <= n {
        let half = len / 2;
        for i in 0..half {
            let avg = a[i];
            let d = a[half + i];
            let l = avg - d.div_euclid(2);
            tmp[2 * i] = l;
            tmp[2 * i + 1] = l + d;
        }
        a[..len].copy_from_slice(&tmp[..len]);
        len *= 2;
    }
}

fn haar2_fwd(b: &mut [i32; 64]) {
    for r in 0..8 {
        haar1_fwd(&mut b[r * 8..r * 8 + 8]);
    }
    for c in 0..8 {
        let mut col = [0i32; 8];
        for r in 0..8 {
            col[r] = b[r * 8 + c];
        }
        haar1_fwd(&mut col);
        for r in 0..8 {
            b[r * 8 + c] = col[r];
        }
    }
}

fn haar2_inv(b: &mut [i32; 64]) {
    for c in 0..8 {
        let mut col = [0i32; 8];
        for r in 0..8 {
            col[r] = b[r * 8 + c];
        }
        haar1_inv(&mut col);
        for r in 0..8 {
            b[r * 8 + c] = col[r];
        }
    }
    for r in 0..8 {
        haar1_inv(&mut b[r * 8..r * 8 + 8]);
    }
}

fn quant_i(d: i32, q: i32) -> i32 {
    if q <= 1 {
        return d;
    }
    let half = q / 2;
    if d >= 0 {
        (d + half) / q
    } else {
        -((-d + half) / q)
    }
}

fn dequant_i(c: i32, q: i32) -> i32 {
    if q <= 1 {
        c
    } else {
        c * q
    }
}

fn encode_haar_all(layers: &[Vec<u8>; 5], w: usize, h: usize, q: i32) -> (Vec<u8>, Vec<u8>) {
    let mut payload = Vec::new();
    let mut recon = Vec::new();
    for layer in layers {
        let p = encode_haar(layer, w, h, q);
        let r = decode_haar(&p, w, h, q);
        put_uvar(&mut payload, p.len() as u32);
        payload.extend_from_slice(&p);
        recon.extend_from_slice(&r);
    }
    (payload, recon)
}

fn decode_haar_all(payload: &[u8], w: usize, h: usize, q: i32, nlayers: usize) -> Vec<u8> {
    let mut i = 0;
    let mut recon = Vec::new();
    for _ in 0..nlayers {
        let n = get_uvar(payload, &mut i) as usize;
        let r = decode_haar(&payload[i..i + n], w, h, q);
        i += n;
        recon.extend_from_slice(&r);
    }
    assert_eq!(i, payload.len());
    recon
}

fn encode_haar(layer: &[u8], w: usize, h: usize, q: i32) -> Vec<u8> {
    let pw = (w + 7) & !7;
    let ph = (h + 7) & !7;
    let mut out = Vec::new();
    let mut y = 0;
    while y < ph {
        let mut x = 0;
        while x < pw {
            let mut block = [0i32; 64];
            for r in 0..8 {
                for c in 0..8 {
                    let xx = (x + c).min(w.saturating_sub(1));
                    let yy = (y + r).min(h.saturating_sub(1));
                    block[r * 8 + c] = i32::from(layer[yy * w + xx]);
                }
            }
            haar2_fwd(&mut block);
            let dc = block[0];
            let mut idx = [0u8; 63];
            let mut coef = [0i32; 63];
            let mut n = 0usize;
            for k in 1..64 {
                let cq = quant_i(block[k], q);
                if cq != 0 {
                    idx[n] = (k - 1) as u8;
                    coef[n] = cq;
                    n += 1;
                }
            }
            put_zvar(&mut out, dc);
            out.push(n as u8);
            for k in 0..n {
                out.push(idx[k]);
                put_zvar(&mut out, coef[k]);
            }
            x += 8;
        }
        y += 8;
    }
    out
}

fn decode_haar(bytes: &[u8], w: usize, h: usize, q: i32) -> Vec<u8> {
    let pw = (w + 7) & !7;
    let ph = (h + 7) & !7;
    let mut recon = vec![0u8; w * h];
    let mut i = 0;
    let mut y = 0;
    while y < ph {
        let mut x = 0;
        while x < pw {
            let mut block = [0i32; 64];
            block[0] = get_zvar(bytes, &mut i);
            let n = bytes[i] as usize;
            i += 1;
            for _ in 0..n {
                let k = bytes[i] as usize;
                i += 1;
                let c = get_zvar(bytes, &mut i);
                block[k + 1] = dequant_i(c, q);
            }
            haar2_inv(&mut block);
            for r in 0..8 {
                for c in 0..8 {
                    let xx = x + c;
                    let yy = y + r;
                    if xx < w && yy < h {
                        recon[yy * w + xx] = block[r * 8 + c].clamp(0, 255) as u8;
                    }
                }
            }
            x += 8;
        }
        y += 8;
    }
    assert_eq!(i, bytes.len(), "haar trailing");
    recon
}

fn run_activity(name: &str, prev: &Picture, next: &Picture, tile: usize, skip: u8, budget: u8) {
    let t = Instant::now();
    let (payload, recon, sent, skipped) = encode_activity_all(&next.layers, &prev.layers, next.w, next.h, tile, skip, budget);
    let packed = deflate3(&payload);
    let ms = elapsed_ms(t);
    let truth = concat_layers(&next.layers);
    let got = decode_activity_all(&payload, &prev.layers, next.w, next.h, tile, skip, budget);
    check_layers_eq(name, &recon, &got);
    if skip == 0 && budget == 0 {
        check_layers_eq(&format!("{name} lossless"), &got, &truth);
    }
    let (max_e, mean_e) = layer_err(&got, &truth);
    println!(
        "{:<32} {:>10} {:>8} {:>10.3} {:>8.2} {:>8} {:>8}",
        name,
        packed.len(),
        max_e,
        mean_e,
        ms,
        sent,
        skipped
    );
}

fn activity_bytes(prev: &Picture, next: &Picture, tile: usize, skip: u8, budget: u8) -> usize {
    let (payload, _, _, _) = encode_activity_all(&next.layers, &prev.layers, next.w, next.h, tile, skip, budget);
    deflate3(&payload).len()
}

fn encode_activity_all(
    cur: &[Vec<u8>; 5],
    reference: &[Vec<u8>; 5],
    w: usize,
    h: usize,
    tile: usize,
    skip: u8,
    budget: u8,
) -> (Vec<u8>, Vec<u8>, u32, u32) {
    let mut payload = Vec::new();
    let mut recon = Vec::new();
    let mut sent = 0u32;
    let mut skipped = 0u32;
    for (c, r) in cur.iter().zip(reference.iter()) {
        let (p, rec, s, k) = encode_activity(c, r, w, h, tile, skip, budget);
        put_uvar(&mut payload, p.len() as u32);
        payload.extend_from_slice(&p);
        recon.extend_from_slice(&rec);
        sent += s;
        skipped += k;
    }
    (payload, recon, sent, skipped)
}

fn decode_activity_all(payload: &[u8], reference: &[Vec<u8>; 5], w: usize, h: usize, tile: usize, skip: u8, budget: u8) -> Vec<u8> {
    let _ = (skip, budget);
    let mut i = 0;
    let mut recon = Vec::new();
    for r in reference {
        let n = get_uvar(payload, &mut i) as usize;
        let got = decode_activity(&payload[i..i + n], r, w, h, tile);
        i += n;
        recon.extend_from_slice(&got);
    }
    assert_eq!(i, payload.len());
    recon
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Skip = 0,
    Mean = 1,
    Quarter = 2,
    Full = 3,
}

fn encode_activity(cur: &[u8], reference: &[u8], w: usize, h: usize, tile: usize, skip_max: u8, budget: u8) -> (Vec<u8>, Vec<u8>, u32, u32) {
    let tw = (w + tile - 1) / tile;
    let th = (h + tile - 1) / tile;
    let ntiles = tw * th;
    let mut modes = vec![Mode::Skip; ntiles];
    let mut payloads: Vec<Vec<u8>> = Vec::with_capacity(ntiles);
    let mut recon = vec![0u8; w * h];
    let mut sent = 0u32;
    let mut skipped = 0u32;
    for ty in 0..th {
        for tx in 0..tw {
            let x0 = tx * tile;
            let y0 = ty * tile;
            let bw = (w - x0).min(tile);
            let bh = (h - y0).min(tile);
            let max_diff = tile_max_diff(cur, reference, w, x0, y0, bw, bh);
            let (mode, body) = if max_diff <= skip_max {
                (Mode::Skip, Vec::new())
            } else {
                choose_lod(cur, w, x0, y0, bw, bh, budget)
            };
            if mode == Mode::Skip {
                skipped += 1;
                paint_from(reference, &mut recon, w, x0, y0, bw, bh);
            } else {
                sent += 1;
                paint_mode(cur, &mut recon, w, x0, y0, bw, bh, mode, &body);
            }
            modes[ty * tw + tx] = mode;
            payloads.push(body);
        }
    }
    let mut out = Vec::new();
    out.push(tile as u8);
    out.push(skip_max);
    out.push(budget);
    out.extend_from_slice(&(w as u16).to_le_bytes());
    out.extend_from_slice(&(h as u16).to_le_bytes());
    let mut packed_modes = vec![0u8; (ntiles + 3) / 4];
    for (k, mode) in modes.iter().enumerate() {
        packed_modes[k / 4] |= (*mode as u8) << ((k % 4) * 2);
    }
    out.extend_from_slice(&packed_modes);
    for body in &payloads {
        out.extend_from_slice(body);
    }
    (out, recon, sent, skipped)
}

fn decode_activity(bytes: &[u8], reference: &[u8], w: usize, h: usize, tile: usize) -> Vec<u8> {
    let mut i = 0;
    let tile_b = bytes[i];
    i += 1;
    assert_eq!(tile_b as usize, tile);
    i += 2; // skip, budget
    let ww = u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as usize;
    i += 2;
    let hh = u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as usize;
    i += 2;
    assert_eq!((ww, hh), (w, h));
    let tw = (w + tile - 1) / tile;
    let th = (h + tile - 1) / tile;
    let ntiles = tw * th;
    let mode_bytes = (ntiles + 3) / 4;
    let mode_src = &bytes[i..i + mode_bytes];
    i += mode_bytes;
    let mut recon = vec![0u8; w * h];
    for ty in 0..th {
        for tx in 0..tw {
            let k = ty * tw + tx;
            let mode_n = (mode_src[k / 4] >> ((k % 4) * 2)) & 3;
            let mode = match mode_n {
                0 => Mode::Skip,
                1 => Mode::Mean,
                2 => Mode::Quarter,
                _ => Mode::Full,
            };
            let x0 = tx * tile;
            let y0 = ty * tile;
            let bw = (w - x0).min(tile);
            let bh = (h - y0).min(tile);
            match mode {
                Mode::Skip => paint_from(reference, &mut recon, w, x0, y0, bw, bh),
                Mode::Mean => {
                    let m = bytes[i];
                    i += 1;
                    paint_const(&mut recon, w, x0, y0, bw, bh, m);
                }
                Mode::Quarter => {
                    let mut qs = [0u8; 4];
                    qs.copy_from_slice(&bytes[i..i + 4]);
                    i += 4;
                    paint_quarters(&mut recon, w, x0, y0, bw, bh, &qs);
                }
                Mode::Full => {
                    let n = bw * bh;
                    let body = &bytes[i..i + n];
                    i += n;
                    let mut p = 0;
                    for yy in y0..y0 + bh {
                        for xx in x0..x0 + bw {
                            recon[yy * w + xx] = body[p];
                            p += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(i, bytes.len(), "activity trailing");
    recon
}

fn choose_lod(cur: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize, budget: u8) -> (Mode, Vec<u8>) {
    let mean = rect_mean(cur, w, x0, y0, bw, bh);
    if rect_max_dev(cur, w, x0, y0, bw, bh, mean) <= budget {
        return (Mode::Mean, vec![mean]);
    }
    if bw >= 2 && bh >= 2 {
        let qs = quarter_means(cur, w, x0, y0, bw, bh);
        if quarter_max_dev(cur, w, x0, y0, bw, bh, &qs) <= budget {
            return (Mode::Quarter, qs.to_vec());
        }
    }
    let mut full = Vec::with_capacity(bw * bh);
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            full.push(cur[y * w + x]);
        }
    }
    (Mode::Full, full)
}

fn tile_max_diff(a: &[u8], b: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize) -> u8 {
    let mut m = 0u8;
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            m = m.max(a[y * w + x].abs_diff(b[y * w + x]));
        }
    }
    m
}

fn rect_mean(layer: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize) -> u8 {
    let mut sum = 0u32;
    let mut n = 0u32;
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            sum += u32::from(layer[y * w + x]);
            n += 1;
        }
    }
    ((sum + n / 2) / n) as u8
}

fn rect_max_dev(layer: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize, mean: u8) -> u8 {
    let mut m = 0u8;
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            m = m.max(layer[y * w + x].abs_diff(mean));
        }
    }
    m
}

fn quarter_means(layer: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize) -> [u8; 4] {
    let mw = bw / 2;
    let mh = bh / 2;
    let rects = [(0, 0, mw, mh), (mw, 0, bw - mw, mh), (0, mh, mw, bh - mh), (mw, mh, bw - mw, bh - mh)];
    let mut qs = [0u8; 4];
    for (k, (dx, dy, rw, rh)) in rects.iter().enumerate() {
        if *rw == 0 || *rh == 0 {
            qs[k] = 0;
        } else {
            qs[k] = rect_mean(layer, w, x0 + dx, y0 + dy, *rw, *rh);
        }
    }
    qs
}

fn quarter_max_dev(layer: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize, qs: &[u8; 4]) -> u8 {
    let mw = bw / 2;
    let mh = bh / 2;
    let rects = [(0, 0, mw, mh), (mw, 0, bw - mw, mh), (0, mh, mw, bh - mh), (mw, mh, bw - mw, bh - mh)];
    let mut m = 0u8;
    for (k, (dx, dy, rw, rh)) in rects.iter().enumerate() {
        if *rw == 0 || *rh == 0 {
            continue;
        }
        m = m.max(rect_max_dev(layer, w, x0 + dx, y0 + dy, *rw, *rh, qs[k]));
    }
    m
}

fn paint_from(src: &[u8], dst: &mut [u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize) {
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            dst[y * w + x] = src[y * w + x];
        }
    }
}

fn paint_const(dst: &mut [u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize, v: u8) {
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            dst[y * w + x] = v;
        }
    }
}

fn paint_quarters(dst: &mut [u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize, qs: &[u8; 4]) {
    let mw = bw / 2;
    let mh = bh / 2;
    let rects = [(0, 0, mw, mh), (mw, 0, bw - mw, mh), (0, mh, mw, bh - mh), (mw, mh, bw - mw, bh - mh)];
    for (k, (dx, dy, rw, rh)) in rects.iter().enumerate() {
        if *rw == 0 || *rh == 0 {
            continue;
        }
        paint_const(dst, w, x0 + dx, y0 + dy, *rw, *rh, qs[k]);
    }
}

fn paint_mode(cur: &[u8], dst: &mut [u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize, mode: Mode, body: &[u8]) {
    match mode {
        Mode::Skip => {}
        Mode::Mean => paint_const(dst, w, x0, y0, bw, bh, body[0]),
        Mode::Quarter => {
            let mut qs = [0u8; 4];
            qs.copy_from_slice(&body[..4]);
            paint_quarters(dst, w, x0, y0, bw, bh, &qs);
        }
        Mode::Full => {
            let mut p = 0;
            for y in y0..y0 + bh {
                for x in x0..x0 + bw {
                    dst[y * w + x] = body[p];
                    let _ = cur;
                    p += 1;
                }
            }
        }
    }
}

fn per_layer_best(p: &Picture) {
    println!(
        "{:<10} {:>10} {:>10} {:>10} {:>8} {:>10}",
        "layer", "base_B", "best_B", "best_max", "best_mean", "method"
    );
    for (li, name) in LAYER_NAMES.iter().enumerate() {
        let layer = &p.layers[li];
        let base = deflate3(layer).len();
        let mut best_bytes = base;
        let mut best_name = "baseline".to_string();
        let mut best_max = 0u8;
        let mut best_mean = 0.0;
        let mut consider = |label: &str, payload: &[u8], recon: &[u8]| {
            let (max_e, mean_e) = layer_err(recon, layer);
            if max_e > 8 {
                return;
            }
            let n = deflate3(payload).len();
            if n < best_bytes {
                best_bytes = n;
                best_name = label.to_string();
                best_max = max_e;
                best_mean = mean_e;
            }
        };
        for ts in [4usize, 8, 16] {
            let (payload, recon) = encode_tiles(layer, p.w, p.h, ts);
            consider(&format!("tile{ts}"), &payload, &recon);
        }
        for t in [0u8, 4, 8, 16] {
            let mut payload = Vec::new();
            quad_write(&mut payload, layer, p.w, 0, 0, p.w, p.h, t);
            let recon = quad_decode(&payload, p.w, p.h, t);
            consider(&format!("quad_T{t}"), &payload, &recon);
        }
        for q in [1i32, 2, 4, 8, 16] {
            let payload = encode_haar(layer, p.w, p.h, q);
            let recon = decode_haar(&payload, p.w, p.h, q);
            consider(&format!("haar_Q{q}"), &payload, &recon);
        }
        println!(
            "{:<10} {:>10} {:>10} {:>10} {:>8.3} {:>10}",
            name, base, best_bytes, best_max, best_mean, best_name
        );
    }
}

fn run_idea4(pics: &[Picture]) {
    let kf0 = &pics[0];
    let kf1 = &pics[10];
    let span = 10i32;
    println!(
        "keyframes: year {} and year {}   budget viewing: pos<=1 cell, rel pop<=10%, layer<=8 u8",
        kf0.year, kf1.year
    );
    println!(
        "{:<16} {:>10} {:>10} {:>10} {:>8} {:>10} {:>8} {:>8} {:>8} {:>8}",
        "year", "band_B", "layer_B", "sum_B", "max_cheb", "mean_euclid", "max_rel", "mean_rel", "max_u8", "mean_u8"
    );
    let mut sum_corr = 0usize;
    let mut sum_band = 0usize;
    let mut sum_layer = 0usize;
    let mut worst_pos = 0.0f64;
    let mut worst_rel = 0.0f64;
    let mut worst_layer = 0u8;
    for y in 1..10 {
        let truth = &pics[y];
        let pred = predict_bands(&kf0.dots, &kf1.dots, y as i32, span);
        let (band_payload, shown) = encode_tol_delta(&pred, &truth.dots, 1, 0.10);
        let band_def = deflate3(&band_payload);
        let (max_c, mean_e, max_r, mean_r) = pop_err(&shown, &truth.dots);
        assert!(max_c <= 1.0 + 1e-6, "pos budget year {y}: {max_c}");
        assert!(max_r <= 0.10 + 1e-6, "pop budget year {y}: {max_r}");
        let mut lerp_layers: [Vec<u8>; 5] = std::array::from_fn(|_| Vec::new());
        for li in 0..5 {
            lerp_layers[li] = lerp_layer(&kf0.layers[li], &kf1.layers[li], y as i32, span);
        }
        let (layer_payload, recon, _, _) = encode_activity_all(&truth.layers, &lerp_layers, truth.w, truth.h, 16, 8, 8);
        let got = decode_activity_all(&layer_payload, &lerp_layers, truth.w, truth.h, 16, 8, 8);
        check_layers_eq("idea4 layers", &recon, &got);
        let truth_cat = concat_layers(&truth.layers);
        let (max_l, mean_l) = layer_err(&got, &truth_cat);
        assert!(max_l <= 8, "layer budget year {y}: {max_l}");
        let layer_def = deflate3(&layer_payload);
        let sum = band_def.len() + layer_def.len();
        sum_corr += sum;
        sum_band += band_def.len();
        sum_layer += layer_def.len();
        worst_pos = worst_pos.max(max_c);
        worst_rel = worst_rel.max(max_r);
        worst_layer = worst_layer.max(max_l);
        println!(
            "{:<16} {:>10} {:>10} {:>10} {:>8.1} {:>10.3} {:>8.4} {:>8.4} {:>8} {:>8.3}",
            format!("corr_y{y}"),
            band_def.len(),
            layer_def.len(),
            sum,
            max_c,
            mean_e,
            max_r,
            mean_r,
            max_l,
            mean_l
        );
    }
    // Interpolation alone, no corrections: error on bands present in both keyframes, plus layer lerp.
    println!("\ninterpolation alone (no corrections), bands matched by id in both keyframes");
    println!(
        "{:<16} {:>8} {:>8} {:>10} {:>8} {:>8} {:>10} {:>8} {:>8}",
        "year", "matched", "ghosts", "missing", "max_cheb", "mean_euclid", "max_rel", "max_u8", "mean_u8"
    );
    for y in 1..10 {
        let truth = &pics[y];
        let pred = predict_bands(&kf0.dots, &kf1.dots, y as i32, span);
        let (max_c, mean_e, max_r, _mean_r, matched) = matched_err(&pred, &truth.dots);
        let ghosts = pred.len().saturating_sub(matched);
        let missing = truth.dots.len().saturating_sub(matched);
        let lerp = lerp_layer(&kf0.layers[0], &kf1.layers[0], y as i32, span);
        // All five layers concatenated via activity-free lerp.
        let mut cat_l = Vec::new();
        let mut cat_t = Vec::new();
        for li in 0..5 {
            cat_l.extend(lerp_layer(&kf0.layers[li], &kf1.layers[li], y as i32, span));
            cat_t.extend_from_slice(&truth.layers[li]);
        }
        let _ = lerp;
        let (max_l, mean_l) = layer_err(&cat_l, &cat_t);
        println!(
            "{:<16} {:>8} {:>8} {:>10} {:>8.1} {:>10.3} {:>8.4} {:>8} {:>8.3}",
            format!("lerp_y{y}"),
            matched,
            ghosts,
            missing,
            max_c,
            mean_e,
            max_r,
            max_l,
            mean_l
        );
    }

    let kf_bands = deflate3(&pack_bands(&kf1.dots)).len();
    let kf_layers = deflate3(&concat_layers(&kf1.layers)).len();
    let kf = kf_bands + kf_layers;
    let mut sum_year = 0usize;
    for p in pics.iter().take(11).skip(1) {
        sum_year += deflate3(&pack_bands(&p.dots)).len() + deflate3(&concat_layers(&p.layers)).len();
    }
    println!("\nidea4 totals over years 1..9 corrections + the year-10 keyframe, amortised per year of a decade");
    println!("mean correction bytes/year (9 years) band {} layer {} sum {}", sum_band / 9, sum_layer / 9, sum_corr / 9);
    println!("one keyframe (year 10) bands {kf_bands} layers {kf_layers} sum {kf}");
    println!(
        "amortised bytes/year = (1 keyframe + 9 corrections) / 10 = {}",
        (kf + sum_corr) / 10
    );
    println!("mean full picture years 1..10 = {}", sum_year / 10);
    println!("worst corrected max_cheb {worst_pos:.1} max_rel {worst_rel:.4} max_layer_u8 {worst_layer}");
    // Tight budget as a second measured point: pos 0, rel 5%, layer 2.
    println!("\ntight budget: pos 0, rel pop <= 5%, layer <= 2 u8 (mean of years 1..9)");
    let mut tight_sum = 0usize;
    let mut tight_max_pos = 0.0f64;
    let mut tight_max_rel = 0.0f64;
    let mut tight_max_l = 0u8;
    for y in 1..10 {
        let truth = &pics[y];
        let pred = predict_bands(&kf0.dots, &kf1.dots, y as i32, span);
        let (band_payload, shown) = encode_tol_delta(&pred, &truth.dots, 0, 0.05);
        let (max_c, _, max_r, _) = pop_err(&shown, &truth.dots);
        assert!(max_c <= 0.0 + 1e-6);
        assert!(max_r <= 0.05 + 1e-6);
        let mut lerp_layers: [Vec<u8>; 5] = std::array::from_fn(|_| Vec::new());
        for li in 0..5 {
            lerp_layers[li] = lerp_layer(&kf0.layers[li], &kf1.layers[li], y as i32, span);
        }
        let (layer_payload, _, _, _) = encode_activity_all(&truth.layers, &lerp_layers, truth.w, truth.h, 16, 2, 2);
        tight_sum += deflate3(&band_payload).len() + deflate3(&layer_payload).len();
        tight_max_pos = tight_max_pos.max(max_c);
        tight_max_rel = tight_max_rel.max(max_r);
        let got = decode_activity_all(&layer_payload, &lerp_layers, truth.w, truth.h, 16, 2, 2);
        let (max_l, _) = layer_err(&got, &concat_layers(&truth.layers));
        tight_max_l = tight_max_l.max(max_l);
    }
    println!(
        "tight mean correction bytes/year {}   worst max_cheb {:.1} max_rel {:.4} max_u8 {}",
        tight_sum / 9,
        tight_max_pos,
        tight_max_rel,
        tight_max_l
    );
}

fn predict_bands(a: &[Dot], b: &[Dot], y: i32, span: i32) -> Vec<Dot> {
    let mut mb: BTreeMap<u32, &Dot> = BTreeMap::new();
    for d in b {
        mb.insert(d.id, d);
    }
    let mut ma: BTreeMap<u32, &Dot> = BTreeMap::new();
    for d in a {
        ma.insert(d.id, d);
    }
    let mut ids: Vec<u32> = Vec::new();
    ids.extend(ma.keys().copied());
    for id in mb.keys() {
        if !ma.contains_key(id) {
            ids.push(*id);
        }
    }
    ids.sort_unstable();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        match (ma.get(&id), mb.get(&id)) {
            (Some(pa), Some(pb)) => {
                let nearer_end = y * 2 >= span;
                let hold = if nearer_end { *pb } else { *pa };
                out.push(Dot {
                    id,
                    x: lerp_u16(pa.x, pb.x, y, span),
                    y: lerp_u16(pa.y, pb.y, y, span),
                    pop: lerp_u32(pa.pop, pb.pop, y, span),
                    lineage: hold.lineage,
                    flags: hold.flags,
                    items: hold.items,
                });
            }
            (Some(pa), None) => out.push((*pa).clone()),
            (None, Some(pb)) => out.push((*pb).clone()),
            (None, None) => {}
        }
    }
    out
}

fn lerp_u16(a: u16, b: u16, y: i32, span: i32) -> u16 {
    lerp_round(i32::from(a), i32::from(b), y, span) as u16
}

fn lerp_u32(a: u32, b: u32, y: i32, span: i32) -> u32 {
    let a = i64::from(a);
    let b = i64::from(b);
    let y = i64::from(y);
    let span = i64::from(span);
    let num = (b - a) * y;
    let v = if num >= 0 { a + (num + span / 2) / span } else { a + (num - span / 2) / span };
    v.max(0) as u32
}

fn lerp_round(a: i32, b: i32, y: i32, span: i32) -> i32 {
    let num = (b - a) * y;
    if num >= 0 {
        a + (num + span / 2) / span
    } else {
        a + (num - span / 2) / span
    }
}

fn lerp_layer(a: &[u8], b: &[u8], y: i32, span: i32) -> Vec<u8> {
    a.iter()
        .zip(b.iter())
        .map(|(&u, &v)| lerp_round(i32::from(u), i32::from(v), y, span).clamp(0, 255) as u8)
        .collect()
}

/// Corrections against a prediction. Position is sent when Chebyshev error
/// exceeds `pos_budget` (then stored exactly). Population is sent when relative
/// error exceeds `rel_budget`. Lineage, flags and items are sent when they differ.
fn encode_tol_delta(pred: &[Dot], truth: &[Dot], pos_budget: u32, rel_budget: f64) -> (Vec<u8>, Vec<Dot>) {
    let mut removes: Vec<u32> = Vec::new();
    let mut adds: Vec<Dot> = Vec::new();
    let mut updates: Vec<u8> = Vec::new();
    let mut n_upd = 0u32;
    let mut prev_upd = 0u32;
    let mut i = 0;
    let mut j = 0;
    while i < pred.len() || j < truth.len() {
        if j >= truth.len() || (i < pred.len() && pred[i].id < truth[j].id) {
            removes.push(pred[i].id);
            i += 1;
        } else if i >= pred.len() || truth[j].id < pred[i].id {
            adds.push(truth[j].clone());
            j += 1;
        } else {
            let pa = &pred[i];
            let pb = &truth[j];
            let mut mask = 0u8;
            if chebyshev(pa.x, pa.y, pb.x, pb.y) > pos_budget {
                mask |= 1;
                mask |= 2;
            }
            let rel = if pb.pop == 0 {
                if pa.pop == 0 { 0.0 } else { 1.0 }
            } else {
                (pa.pop as f64 - pb.pop as f64).abs() / pb.pop as f64
            };
            if rel > rel_budget {
                mask |= 4;
            }
            if pa.lineage != pb.lineage {
                mask |= 8;
            }
            if pa.flags != pb.flags {
                mask |= 16;
            }
            if pa.items != pb.items {
                mask |= 32;
            }
            if mask != 0 {
                put_uvar(&mut updates, pb.id.wrapping_sub(prev_upd));
                prev_upd = pb.id;
                updates.push(mask);
                if mask & 1 != 0 {
                    put_zvar(&mut updates, i32::from(pb.x) - i32::from(pa.x));
                }
                if mask & 2 != 0 {
                    put_zvar(&mut updates, i32::from(pb.y) - i32::from(pa.y));
                }
                if mask & 4 != 0 {
                    put_uvar(&mut updates, pb.pop);
                }
                if mask & 8 != 0 {
                    put_uvar(&mut updates, pb.lineage);
                }
                if mask & 16 != 0 {
                    updates.push(pb.flags);
                }
                if mask & 32 != 0 {
                    updates.push(pb.items);
                }
                n_upd += 1;
            }
            i += 1;
            j += 1;
        }
    }
    let mut out = Vec::new();
    put_uvar(&mut out, removes.len() as u32);
    put_uvar(&mut out, adds.len() as u32);
    put_uvar(&mut out, n_upd);
    let mut prev_id = 0u32;
    for id in &removes {
        put_uvar(&mut out, id.wrapping_sub(prev_id));
        prev_id = *id;
    }
    prev_id = 0;
    for d in &adds {
        put_uvar(&mut out, d.id.wrapping_sub(prev_id));
        prev_id = d.id;
        put_uvar(&mut out, u32::from(d.x));
        put_uvar(&mut out, u32::from(d.y));
        put_uvar(&mut out, d.pop);
        put_uvar(&mut out, d.lineage);
        out.push(d.flags);
        out.push(d.items);
    }
    out.extend_from_slice(&updates);
    let shown = decode_field_delta(pred, &out, FieldMode::Exact);
    (out, shown)
}

fn matched_err(pred: &[Dot], truth: &[Dot]) -> (f64, f64, f64, f64, usize) {
    let mut shown = Vec::new();
    let mut real = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < pred.len() && j < truth.len() {
        if pred[i].id < truth[j].id {
            i += 1;
        } else if truth[j].id < pred[i].id {
            j += 1;
        } else {
            shown.push(pred[i].clone());
            real.push(truth[j].clone());
            i += 1;
            j += 1;
        }
    }
    let n = shown.len();
    if n == 0 {
        return (0.0, 0.0, 0.0, 0.0, 0);
    }
    let (a, b, c, d) = pop_err(&shown, &real);
    (a, b, c, d, n)
}
