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

async fn run(a: cli::Args) {
    let mut w = World::new(cli::make_config(&a, a.seeds.0), cli::make_params(&a));
    let mut st = ui::UiState { overlay: a.overlay.min(8), new_seed: a.seeds.0 + 1, ..Default::default() };
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
    if a.screenshot.is_some() && a.load.is_none() {
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
            jump = ui::draw(ctx, &mut st, &mut w, selected);
        });
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
                for (k, o) in [(KeyCode::Key1, 0), (KeyCode::Key2, 1), (KeyCode::Key3, 2), (KeyCode::Key4, 3), (KeyCode::Key5, 4), (KeyCode::Key6, 5), (KeyCode::Key7, 6), (KeyCode::Key8, 7), (KeyCode::Key9, 8)] {
                    if is_key_pressed(k) {
                        st.overlay = o;
                    }
                }
            }
        }
        let m: Vec2 = mouse_position().into();
        if !st.wants_pointer {
            let (_, wy) = mouse_wheel();
            if wy != 0.0 {
                let before = view.to_world(m);
                view.zoom = (view.zoom * if wy > 0.0 { 1.15 } else { 1.0 / 1.15 }).clamp(1.0, 80.0);
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
                selected = render::pick_band(&w, &view, m);
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
                ui::FileReq::New(seed) => {
                    let p = w.params;
                    w = World::new(cli::make_config(&a, seed), p);
                    prev_params = w.params;
                    selected = None;
                    view = View::fit(&w);
                    st.show_load = false;
                    toast(&mut st, format!("new world, seed {seed}"));
                }
            }
        }
        // ---------- simulate (bounded work per frame so the UI stays responsive)
        if !st.paused {
            acc += st.speed as f64 * get_frame_time().min(0.1) as f64;
            let t0 = get_time();
            while acc >= 1.0 {
                w.step();
                acc -= 1.0;
                if get_time() - t0 > 0.03 {
                    acc = acc.min(1.0);
                    break;
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
        mt.refresh(&w, st.overlay, st.item);
        render::draw_world(&w, &view, &mt, st.overlay, st.item, selected);
        if get_time() < st.toast_until {
            let tw = measure_text(&st.toast, None, 20, 1.0).width;
            draw_rectangle(screen_width() / 2.0 - tw / 2.0 - 8.0, screen_height() - 40.0, tw + 16.0, 28.0, Color::new(0.0, 0.0, 0.0, 0.7));
            draw_text(&st.toast, screen_width() / 2.0 - tw / 2.0, screen_height() - 20.0, 20.0, WHITE);
        }
        egui_macroquad::draw();
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
