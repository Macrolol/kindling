//! Convert a save (any supported format) to the current JSON form, for diffs and checks:
//! `cargo run --release -p civ-core --example save_to_json -- IN.civsave OUT.json`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let t = std::time::Instant::now();
    let (w, view, info) = civ_core::World::load_bytes(&std::fs::read(&a[1]).expect("read")).expect("load");
    eprintln!("loaded v{} in {:.2}s", info.format_version, t.elapsed().as_secs_f32());
    std::fs::write(&a[2], w.save_json(view.as_ref(), "dump", "dump").expect("json")).expect("write");
}
