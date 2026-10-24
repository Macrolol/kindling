//! Command-line arguments.
use civ_core::{Config, Params};

pub struct Args {
    pub headless: bool,
    pub seeds: (u64, u64),
    pub years: u32,
    pub map: Option<(usize, usize)>,
    pub config: Option<String>,
    pub load: Option<String>,
    pub save: Option<String>,
    pub json: bool,
    pub verbose: bool,
    pub report: u32,
    pub screenshot: Option<String>,
    pub shot_years: u32,
    pub overlay: u8,
    pub ui: String,
}

pub const USAGE: &str = "\
kindling [--seed S] [--map WxH] [--config params.json] [--load file.civsave]
  desktop:  [--screenshot out.png [--years N] [--overlay 0-7] [--ui inspector|log|stats|settings]]
  headless: --headless [--seeds A..B | --seed S] [--years N] [--report K] [--json] [--verbose]
            [--save out.civsave]";

pub fn parse_args() -> Args {
    let mut a = Args {
        headless: false,
        seeds: (1, 1),
        years: 300,
        map: None,
        config: None,
        load: None,
        save: None,
        json: false,
        verbose: false,
        report: 0,
        screenshot: None,
        shot_years: 0,
        overlay: 1,
        ui: String::new(),
    };
    let v: Vec<String> = std::env::args().collect();
    let mut years_set = false;
    let mut i = 1;
    while i < v.len() {
        let next = v.get(i + 1).cloned().unwrap_or_default();
        let mut skip = 1;
        match v[i].as_str() {
            "--headless" => {
                a.headless = true;
                skip = 0
            }
            "--json" => {
                a.json = true;
                skip = 0
            }
            "--verbose" => {
                a.verbose = true;
                skip = 0
            }
            "--seed" => {
                let s = next.parse().unwrap_or(1);
                a.seeds = (s, s)
            }
            "--seeds" => {
                let mut it = next.split("..");
                let s0 = it.next().and_then(|x| x.parse().ok()).unwrap_or(1);
                let s1 = it.next().and_then(|x| x.parse().ok()).unwrap_or(s0);
                a.seeds = (s0, s1.max(s0))
            }
            "--years" => {
                a.years = next.parse().unwrap_or(300);
                years_set = true
            }
            "--report" => a.report = next.parse().unwrap_or(0),
            "--map" => {
                let mut it = next.split('x');
                if let (Some(Ok(w)), Some(Ok(h))) = (it.next().map(|x| x.parse()), it.next().map(|x| x.parse())) {
                    a.map = Some((w, h));
                }
            }
            "--config" => a.config = Some(next),
            "--load" => a.load = Some(next),
            "--save" => a.save = Some(next),
            "--screenshot" => a.screenshot = Some(next),
            "--overlay" => a.overlay = next.parse().unwrap_or(1),
            "--ui" => a.ui = next,
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0)
            }
            x => {
                eprintln!("unknown argument {x}\n{USAGE}");
                std::process::exit(2)
            }
        }
        i += 1 + skip;
    }
    a.shot_years = if years_set { a.years } else { 250 };
    a
}

pub fn make_config(a: &Args, seed: u64) -> Config {
    let mut c = Config { seed, ..Default::default() };
    if let Some((w, h)) = a.map {
        c.map_w = w.clamp(32, 600);
        c.map_h = h.clamp(24, 400);
    }
    c
}

pub fn make_params(a: &Args) -> Params {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(p) = &a.config {
        match std::fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|s| Params::from_json_compat(&s)) {
            Ok(x) => return x,
            Err(e) => eprintln!("config {p}: {e} (using defaults)"),
        }
    }
    let _ = a;
    Params::default()
}
