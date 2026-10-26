//! Load a save, re-save it in the current format, load that again and check nothing changed:
//! `cargo run --release -p civ-core --example save_roundtrip -- IN.civsave [OUT.civsave]`
//! Prints sizes and load / save times; exits 1 if the reloaded world differs (compared as JSON).
use std::time::Instant;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let inb = std::fs::read(&a[1]).expect("read");
    let t = Instant::now();
    let (w, view, info) = civ_core::World::load_bytes(&inb).expect("load");
    let t_load = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let out = w.save_bytes(view.as_ref(), "roundtrip", "roundtrip").expect("save");
    let t_save = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let (w2, view2, _) = civ_core::World::load_bytes(&out).expect("reload");
    let t_reload = t.elapsed().as_secs_f64();
    if let Some(p) = a.get(2) {
        std::fs::write(p, &out).expect("write");
    }
    let j1 = w.save_json(view.as_ref(), "x", "x").unwrap();
    let j2 = w2.save_json(view2.as_ref(), "x", "x").unwrap();
    let same = j1 == j2 && w.state_hash() == w2.state_hash();
    println!(
        "in v{} {:.2} MB, load {:.2}s | out v{} {:.2} MB, save {:.2}s, load {:.2}s | {:.1}% of the input | identical: {same}",
        info.format_version,
        inb.len() as f64 / 1e6,
        t_load,
        civ_core::save::FORMAT_VERSION,
        out.len() as f64 / 1e6,
        t_save,
        t_reload,
        100.0 * out.len() as f64 / inb.len() as f64
    );
    if !same {
        std::process::exit(1);
    }
}
