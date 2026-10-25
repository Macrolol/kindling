//! Kindling (working name): desktop / web viewer and headless runner for the civilisation sim.
mod cli;
#[cfg(not(target_arch = "wasm32"))]
mod headless;
mod render;
mod saveio;
mod ui;

use civ_core::{ViewState, World};
use macroquad::prelude::*;
use render::{MapTex, View};

// a faster allocator for the many small per-band buffers, especially across threads
#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

type Loaded = (World, Option<ViewState>, String);

fn load_world_bytes(b: &[u8], name: &str) -> Result<Loaded, String> {
    let (w, v, info) = World::load_bytes(b)?;
    let mut msg = format!(
        "loaded {name}: format v{} app {} saved {} · Y{} · {} bands",
        info.format_version,
        info.app_version,
        info.created_at,
        w.tick / 4,
        w.bands.len()
    );
    if let Some(f) = info.migrated_from {
        msg += &format!(" (migrated from v{f})");
    }
    if let Some(wn) = info.warning {
        msg += &format!(" WARNING: {wn}");
    }
    Ok((w, v, msg))
}

fn main() {
    let a = cli::parse_args();
    #[cfg(not(target_arch = "wasm32"))]
    if a.threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new().num_threads(a.threads).build_global();
    }
    #[cfg(not(target_arch = "wasm32"))]
    if a.headless {
        headless::run(&a);
        return;
    }
    let conf = Conf {
        window_title: "Kindling".into(),
        window_width: 1400,
        window_height: 860,
        high_dpi: false,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, run(a));
}

fn toast(st: &mut ui::UiState, msg: String) {
    eprintln!("{msg}");
    st.toast = msg;
    st.toast_until = get_time() + 6.0;
}

fn view_state(v: &View, sel: Option<u32>, st: &ui::UiState) -> ViewState {
    ViewState { cx: v.cx, cy: v.cy, zoom: v.zoom, selected: sel, overlay: st.overlay, item: st.item }
}

/// One season on the chosen thread pool (native) or on the global one.
#[cfg(not(target_arch = "wasm32"))]
fn step(w: &mut World, pool: &Option<rayon::ThreadPool>) {
    match pool {
        Some(p) => p.install(|| w.step()),
        None => w.step(),
    }
}
#[cfg(target_arch = "wasm32")]
fn step(w: &mut World, _pool: &Option<()>) {
    w.step()
}

/// A few well-spread good-land cells (for the picker screenshot).
fn demo_starts(w: &World, n: usize) -> Vec<(u16, u16)> {
    let mut v = Vec::new();
    let (mw, mh) = (w.map.w, w.map.h);
    let mut k = 1usize;
    while v.len() < n && k < 4000 {
        // a scattered walk over the map
        let x = (k * 7919 + 13) % mw;
        let y = (k * 104729 + 7) % mh;
        k += 1;
        if w.map.start_check(x, y) == civ_core::StartCheck::Ok
            && v.iter().all(|&(a, b): &(u16, u16)| (a as i64 - x as i64).abs() + (b as i64 - y as i64).abs() > (mw as i64 / 6))
        {
            v.push((x as u16, y as u16));
        }
    }
    v
}

async fn run(a: cli::Args) {
    let mut w = World::new(cli::make_config(&a, a.seeds.0), cli::make_params(&a));
    for m in cli::check_starts(&w, &a.starts) {
        eprintln!("{m}");
    }
    let mut st = ui::UiState {
        overlay: a.overlay.min(8),
        new_seed: a.seeds.0 + 1,
        threads: a.threads,
        new_w: w.map.w,
        new_h: w.map.h,
        ..Default::default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let mut pool: Option<rayon::ThreadPool> = None;
    #[cfg(target_arch = "wasm32")]
    let pool: Option<()> = None;
    #[allow(unused)]
    let mut pool_n = a.threads;
    // the start picker's world (map only; nothing runs on it)
    let mut pv: Option<World> = None;
    let mut view = View::fit(&w);
    let mut selected: Option<u32> = None;
    let mut pending: Option<Loaded> = None;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(p) = &a.load {
        match saveio::read(p).and_then(|b| load_world_bytes(&b, p)) {
            Ok(x) => pending = Some(x),
            Err(e) => eprintln!("load failed: {e}"),
        }
    }
    // screenshot mode: run the world forward first, pick the most knowledgeable band
    if a.screenshot.is_some() && a.ui == "picker" {
        let p = World::new(cli::make_config(&a, a.seeds.0), w.params);
        let starts = demo_starts(&p, 4);
        let mut pk = ui::Picker { seed: a.seeds.0, w: p.map.w, h: p.map.h, mode: 2, starts: vec![], founders: 6, note: String::new() };
        for &(x, y) in &starts {
            pk.click(&p.map, x as usize, y as usize);
        }
        st.picker = Some(pk);
        st.show_legend = true;
        st.overlay = 0;
        pv = Some(p);
    } else if a.screenshot.is_some() && a.load.is_none() {
        for _ in 0..a.shot_years * 4 {
            w.step();
        }
        selected = w.bands.iter().max_by_key(|b| (b.n_items(), b.pop)).map(|b| b.id);
        st.paused = true;
        st.show_log = a.ui != "none";
        st.show_inspector = a.ui != "none";
        st.show_stats = a.ui.contains("stats");
        st.show_settings = a.ui.contains("settings");
        if st.overlay == 2 {
            // the most widespread item first made after the first century, else the latest
            st.item = w
                .firsts
                .iter()
                .filter(|f| f.tick > 400)
                .max_by_key(|f| w.item_bands.get(&f.item).copied().unwrap_or(0))
                .or(w.firsts.last())
                .map(|f| f.item);
        }
    }
    let mut mt = MapTex::new(&w, st.overlay, st.item);
    let mut acc = 0.0f64;
    let mut press: Option<Vec2> = None;
    let mut last_mouse = vec2(0.0, 0.0);
    let mut dragged = false;
    #[allow(unused)]
    let mut frame = 0u64;
    let mut prev_params = w.params;
    #[cfg(not(target_arch = "wasm32"))]
    let mut last_autosave = get_time();
    loop {
        frame += 1;
        if let Some((nw, vs, msg)) = pending.take() {
            w = nw;
            prev_params = w.params;
            view = View::fit(&w);
            if let Some(v) = vs {
                if v.zoom > 0.0 {
                    view = View { cx: v.cx, cy: v.cy, zoom: v.zoom };
                }
                selected = v.selected;
                st.overlay = v.overlay.min(8);
                st.item = v.item;
            }
            toast(&mut st, msg);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(b) = saveio::poll_file() {
            match load_world_bytes(&b, "file") {
                Ok(x) => pending = Some(x),
                Err(e) => toast(&mut st, format!("load failed: {e}")),
            }
        }
        let mut jump = None;
        egui_macroquad::ui(|ctx| {
            let cur = match pv.as_mut() {
                Some(p) if st.picker.is_some() => p,
                _ => &mut w,
            };
            jump = ui::draw(ctx, &mut st, cur, selected);
        });
        if st.picker.is_none() && pv.is_some() {
            // picker closed: back to the running world
            pv = None;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if st.threads != pool_n {
            pool_n = st.threads;
            pool = if pool_n == 0 { None } else { cli::make_pool(pool_n) };
            toast(&mut st, format!("sim threads: {}", if pool_n == 0 { "all cores".to_string() } else { pool_n.to_string() }));
        }
        if let Some(id) = jump {
            selected = Some(id);
            st.follow = true;
        }
        // settings changes are logged once the user lets go of the control
        if w.params != prev_params && !is_mouse_button_down(MouseButton::Left) {
            let old = prev_params;
            w.log_param_changes(&old);
            prev_params = w.params;
        }
        // ---------- input
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) || is_key_down(KeyCode::LeftSuper);
        if !st.wants_kb {
            if ctrl && is_key_pressed(KeyCode::S) {
                st.req = Some(ui::FileReq::Save);
            }
            if ctrl && is_key_pressed(KeyCode::O) {
                st.req = Some(ui::FileReq::OpenLoad);
            }
            if !ctrl {
                if is_key_pressed(KeyCode::Space) {
                    st.paused = !st.paused;
                }
                if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
                    st.speed = (st.speed * 2.0).min(400.0);
                }
                if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
                    st.speed = (st.speed / 2.0).max(0.5);
                }
                if is_key_pressed(KeyCode::F) {
                    st.follow = !st.follow;
                }
                if is_key_pressed(KeyCode::Home) || is_key_pressed(KeyCode::Key0) {
                    st.fit_view = true;
                }
                if is_key_pressed(KeyCode::X) {
                    st.fast = !st.fast;
                }
                for (k, o) in [(KeyCode::Key1, 0), (KeyCode::Key2, 1), (KeyCode::Key3, 2), (KeyCode::Key4, 3), (KeyCode::Key5, 4), (KeyCode::Key6, 5), (KeyCode::Key7, 6), (KeyCode::Key8, 7), (KeyCode::Key9, 8)] {
                    if is_key_pressed(k) {
                        st.overlay = o;
                    }
                }
            }
        }
        let zmin = {
            let cur = match pv.as_ref() {
                Some(p) if st.picker.is_some() => p,
                _ => &w,
            };
            if std::mem::take(&mut st.fit_view) {
                view = View::fit(cur);
                st.follow = false;
            }
            View::min_zoom(cur)
        };
        let m: Vec2 = mouse_position().into();
        if !st.wants_pointer {
            let (_, wy) = mouse_wheel();
            if wy != 0.0 {
                let before = view.to_world(m);
                view.zoom = (view.zoom * if wy > 0.0 { 1.15 } else { 1.0 / 1.15 }).clamp(zmin, View::MAX_ZOOM);
                let after = view.to_world(m);
                view.cx += before.x - after.x;
                view.cy += before.y - after.y;
            }
            if is_mouse_button_pressed(MouseButton::Left) {
                press = Some(m);
                dragged = false;
            }
        }
        if is_mouse_button_down(MouseButton::Left) && press.is_some() {
            let d = m - last_mouse;
            if press.is_some_and(|p| p.distance(m) > 4.0) {
                dragged = true;
            }
            if dragged {
                view.cx -= d.x / view.zoom;
                view.cy -= d.y / view.zoom;
                st.follow = false;
            }
        }
        if is_mouse_button_released(MouseButton::Left) {
            if press.is_some() && !dragged {
                match (st.picker.as_mut(), pv.as_ref()) {
                    (Some(pk), Some(p)) => {
                        let c = view.to_world(m);
                        if c.x >= 0.0 && c.y >= 0.0 {
                            pk.click(&p.map, c.x as usize, c.y as usize);
                        }
                    }
                    _ => selected = render::pick_band(&w, &view, m),
                }
            }
            press = None;
        }
        last_mouse = m;
        // ---------- files
        #[cfg(not(target_arch = "wasm32"))]
        if st.autosave_min > 0.0 && get_time() - last_autosave > st.autosave_min as f64 * 60.0 {
            last_autosave = get_time();
            let name = saveio::autosave_name(st.autosave_keep);
            let r = w
                .save_bytes(Some(&view_state(&view, selected, &st)), saveio::APP_VERSION, &saveio::now_string())
                .and_then(|b| saveio::write(&name, &b));
            match r {
                Ok(p) => toast(&mut st, format!("autosaved {p}")),
                Err(e) => toast(&mut st, format!("autosave failed: {e}")),
            }
        }
        if let Some(req) = st.req.take() {
            match req {
                ui::FileReq::Save => {
                    let name = format!("kindling-s{}-y{}-{}.{}", w.cfg.seed, w.tick / 4, saveio::stamp(), saveio::EXT);
                    let r = w.save_bytes(Some(&view_state(&view, selected, &st)), saveio::APP_VERSION, &saveio::now_string());
                    #[cfg(not(target_arch = "wasm32"))]
                    let r = r.and_then(|b| saveio::write(&name, &b));
                    #[cfg(target_arch = "wasm32")]
                    let r = r.map(|b| {
                        saveio::download(&name, &b);
                        name.clone()
                    });
                    match r {
                        Ok(p) => toast(&mut st, format!("saved {p}")),
                        Err(e) => toast(&mut st, format!("save failed: {e}")),
                    }
                }
                ui::FileReq::OpenLoad => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        st.saves = saveio::list();
                    }
                    st.show_load = true;
                }
                ui::FileReq::Load(_p) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    match saveio::read(&_p).and_then(|b| load_world_bytes(&b, &_p)) {
                        Ok(x) => {
                            pending = Some(x);
                            st.show_load = false;
                        }
                        Err(e) => toast(&mut st, format!("load failed: {e}")),
                    }
                }
                ui::FileReq::Preview { seed, w: mw, h: mh } => {
                    let mut c = cli::make_config(&a, seed);
                    c.map_w = mw.clamp(32, 2048);
                    c.map_h = mh.clamp(24, 2048);
                    c.starts.clear();
                    let p = World::new(c, w.params);
                    view = View::fit(&p);
                    st.picker = Some(ui::Picker { seed, w: p.map.w, h: p.map.h, mode: 0, starts: vec![], founders: p.cfg.founders, note: String::new() });
                    st.show_load = false;
                    st.show_legend = true;
                    if !render::static_overlay(st.overlay) && st.overlay != 8 {
                        st.overlay = 0;
                    }
                    pv = Some(p);
                }
                ui::FileReq::New { seed, w: mw, h: mh, starts, founders } => {
                    let p = w.params;
                    let mut c = cli::make_config(&a, seed);
                    c.map_w = mw.clamp(32, 2048);
                    c.map_h = mh.clamp(24, 2048);
                    c.starts = starts.clone();
                    if founders > 0 {
                        c.founders = founders;
                    }
                    w = World::new(c, p);
                    pv = None;
                    prev_params = w.params;
                    selected = None;
                    view = View::fit(&w);
                    st.show_load = false;
                    let how = if starts.is_empty() {
                        "random start".to_string()
                    } else {
                        let s: Vec<String> = starts.iter().map(|(x, y)| format!("{x},{y}")).collect();
                        format!("starts {}", s.join(";"))
                    };
                    toast(&mut st, format!("new world, seed {seed}, {}x{}, {how}", w.map.w, w.map.h));
                }
            }
        }
        // ---------- simulate (bounded work per frame so the UI stays responsive)
        if !st.paused && st.picker.is_none() {
            let t0 = get_time();
            if st.fast {
                // fast mode: as many seasons as fit in ~1/4 s, at least one
                loop {
                    step(&mut w, &pool);
                    if get_time() - t0 > 0.25 || w.bands.is_empty() {
                        break;
                    }
                }
                acc = 0.0;
            } else {
                acc += st.speed as f64 * get_frame_time().min(0.1) as f64;
                while acc >= 1.0 {
                    step(&mut w, &pool);
                    acc -= 1.0;
                    if get_time() - t0 > 0.03 {
                        acc = acc.min(1.0);
                        break;
                    }
                }
            }
        }
        if st.follow {
            if let Some(b) = selected.and_then(|id| w.band_index(id)).map(|i| &w.bands[i]) {
                view.cx = b.x as f32 + 0.5;
                view.cy = b.y as f32 + 0.5;
            }
        }
        // ---------- draw
        if let (Some(p), Some(_)) = (pv.as_ref(), st.picker.as_ref()) {
            mt.refresh(p, st.overlay, None, false);
            render::draw_world(p, &view, &mt, st.overlay, None, None, false);
        } else {
            mt.refresh(&w, st.overlay, st.item, st.fast);
            let bands = !st.fast || view.zoom >= 4.0 || w.bands.len() < 3000;
            render::draw_world(&w, &view, &mt, st.overlay, st.item, selected, bands);
        }
        if get_time() < st.toast_until {
            let tw = measure_text(&st.toast, None, 20, 1.0).width;
            draw_rectangle(screen_width() / 2.0 - tw / 2.0 - 8.0, screen_height() - 40.0, tw + 16.0, 28.0, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_text(&st.toast, screen_width() / 2.0 - tw / 2.0, screen_height() - 20.0, 20.0, WHITE);
        }
        egui_macroquad::draw();
        // start markers go on top of the windows, so the legend or the picker panel never hides one
        if let (Some(p), Some(pk)) = (pv.as_ref(), st.picker.as_ref()) {
            draw_starts(p, &view, pk, m, st.wants_pointer);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &a.screenshot {
            if frame == 90 {
                get_screen_data().export_png(path);
                eprintln!("screenshot written to {path} (Y{}, {} bands)", w.tick / 4, w.bands.len());
                std::process::exit(0);
            }
        }
        next_frame().await
    }
}

/// Start markers and the hover readout in the start picker.
fn draw_starts(w: &World, v: &View, pk: &ui::Picker, m: Vec2, over_ui: bool) {
    let r = (v.zoom * 0.5).max(5.0);
    if pk.mode == 1 {
        if let Some(&(x, y)) = pk.starts.first() {
            // the founders spread over a few cells around the start
            let a = v.to_screen(x as f32 - 3.0, y as f32 - 3.0);
            let b = v.to_screen(x as f32 + 4.0, y as f32 + 4.0);
            draw_rectangle_lines(a.x, a.y, b.x - a.x, b.y - a.y, 3.0, WHITE);
            let c = v.to_screen(x as f32 + 0.5, y as f32 + 0.5);
            draw_circle(c.x, c.y, r, Color::new(1.0, 0.85, 0.2, 1.0));
            draw_circle_lines(c.x, c.y, r, 2.0, BLACK);
        }
    } else if pk.mode == 2 {
        for (k, &(x, y)) in pk.starts.iter().enumerate() {
            let c = v.to_screen(x as f32 + 0.5, y as f32 + 0.5);
            draw_circle(c.x, c.y, r + 3.0, BLACK);
            draw_circle(c.x, c.y, r, render::lineage_color(k as u32));
            let t = format!("{}", k + 1);
            draw_text(&t, c.x + r + 3.0, c.y - r, 22.0, BLACK);
            draw_text(&t, c.x + r + 2.0, c.y - r - 1.0, 22.0, WHITE);
        }
    }
    if over_ui {
        return;
    }
    let c = v.to_world(m);
    if c.x < 0.0 || c.y < 0.0 || c.x as usize >= w.map.w || c.y as usize >= w.map.h {
        return;
    }
    let (x, y) = (c.x as usize, c.y as usize);
    let i = y * w.map.w + x;
    let (txt, col) = match w.map.start_check(x, y) {
        civ_core::StartCheck::Ok => (format!("({x},{y}) {} {:.0}C - good land", w.map.biome_at(i).name(), w.map.temp[i]), GREEN),
        civ_core::StartCheck::Poor => (format!("({x},{y}) {} {:.0}C - poor land", w.map.biome_at(i).name(), w.map.temp[i]), ORANGE),
        civ_core::StartCheck::Uninhabitable => (format!("({x},{y}) {} - uninhabitable", w.map.biome_at(i).name()), RED),
    };
    let tw = measure_text(&txt, None, 18, 1.0).width;
    draw_rectangle(m.x + 14.0, m.y + 8.0, tw + 10.0, 24.0, Color::new(0.0, 0.0, 0.0, 0.75));
    draw_text(&txt, m.x + 19.0, m.y + 25.0, 18.0, col);
}
