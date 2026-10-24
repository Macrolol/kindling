//! egui windows: top bar, inspector, event log / chronicle, stats, live settings, load.
use crate::render::OVERLAYS;
use civ_core::knowledge::{FROM_PARENT, FROM_SELF};
use civ_core::materials::{item_name, Item};
use civ_core::world::EventKind;
use civ_core::{Params, World};
use egui_macroquad::egui;

pub enum FileReq {
    Save,
    OpenLoad,
    Load(String),
    New(u64),
}

pub struct UiState {
    pub paused: bool,
    /// seasons per second
    pub speed: f32,
    pub overlay: u8,
    pub item: Option<Item>,
    pub show_inspector: bool,
    pub show_log: bool,
    pub show_stats: bool,
    pub show_settings: bool,
    pub show_history: bool,
    pub show_load: bool,
    pub show_help: bool,
    /// 0 chronicle, 1 all events, 2 selected band only
    pub log_mode: u8,
    pub wants_pointer: bool,
    pub wants_kb: bool,
    pub req: Option<FileReq>,
    pub toast: String,
    pub toast_until: f64,
    pub saves: Vec<(String, u64, u64)>,
    pub new_seed: u64,
    #[allow(dead_code)]
    pub autosave_min: f32,
    #[allow(dead_code)]
    pub autosave_keep: u32,
    pub follow: bool,
}
impl Default for UiState {
    fn default() -> Self {
        UiState {
            paused: false,
            speed: 8.0,
            overlay: 0,
            item: None,
            show_inspector: true,
            show_log: true,
            show_stats: false,
            show_settings: false,
            show_history: false,
            show_load: false,
            show_help: false,
            log_mode: 0,
            wants_pointer: false,
            wants_kb: false,
            req: None,
            toast: String::new(),
            toast_until: 0.0,
            saves: vec![],
            new_seed: 2,
            autosave_min: 10.0,
            autosave_keep: 3,
            follow: false,
        }
    }
}

pub fn season_name(s: u32) -> &'static str {
    ["spring", "summer", "autumn", "winter"][(s % 4) as usize]
}

fn items_ever(w: &World) -> Vec<Item> {
    w.firsts.iter().map(|f| f.item).collect()
}

/// Draws all windows. Returns a band id to jump to, if one was clicked.
pub fn draw(ctx: &egui::Context, st: &mut UiState, w: &mut World, selected: Option<u32>) -> Option<u32> {
    let mut jump = None;
    egui::TopBottomPanel::top("top").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Kindling");
            ui.label(format!(
                "Y{} {} · pop {} · {} bands · {} things known",
                w.tick / 4,
                season_name(w.season()),
                w.total_pop(),
                w.bands.len(),
                w.item_bands.len()
            ));
            ui.separator();
            if ui.button(if st.paused { "▶ run" } else { "⏸ pause" }).clicked() {
                st.paused = !st.paused;
            }
            ui.add(egui::Slider::new(&mut st.speed, 0.5..=400.0).logarithmic(true).text("seasons/s"));
            ui.separator();
            egui::ComboBox::from_id_salt("overlay")
                .selected_text(OVERLAYS[st.overlay as usize])
                .show_ui(ui, |ui| {
                    for (i, n) in OVERLAYS.iter().enumerate() {
                        ui.selectable_value(&mut st.overlay, i as u8, *n);
                    }
                });
            if st.overlay == 2 {
                let items = items_ever(w);
                if st.item.is_none() {
                    st.item = items.first().copied();
                }
                egui::ComboBox::from_id_salt("item")
                    .selected_text(st.item.map(item_name).unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for it in items {
                            ui.selectable_value(&mut st.item, Some(it), item_name(it));
                        }
                    });
            }
            ui.separator();
            ui.toggle_value(&mut st.show_inspector, "Inspector");
            ui.toggle_value(&mut st.show_log, "Log");
            ui.toggle_value(&mut st.show_stats, "Stats");
            ui.toggle_value(&mut st.show_settings, "Settings");
            ui.toggle_value(&mut st.show_help, "Help");
            if ui.button("Save").clicked() {
                st.req = Some(FileReq::Save);
            }
            if ui.button("Load…").clicked() {
                st.req = Some(FileReq::OpenLoad);
            }
        });
    });

    if st.show_inspector {
        egui::Window::new("Inspector").default_pos([8.0, 40.0]).default_width(420.0).show(ctx, |ui| {
            inspector(ui, w, selected, st);
        });
    }
    if st.show_log {
        egui::Window::new("Events").default_pos([8.0, 470.0]).default_size([480.0, 260.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut st.log_mode, 0, "Chronicle");
                ui.selectable_value(&mut st.log_mode, 1, "All");
                ui.selectable_value(&mut st.log_mode, 2, "Selected band");
            });
            egui::ScrollArea::vertical().max_height(220.0).stick_to_bottom(true).show(ui, |ui| {
                let evs: Vec<_> = match st.log_mode {
                    0 => w.chronicle.iter().copied().collect(),
                    1 => w.events.iter().rev().take(400).rev().copied().collect(),
                    _ => match selected {
                        Some(id) => {
                            {
                                let mut v: Vec<_> = w.events.iter().rev().filter(|e| e.band == id || e.other == id).take(300).copied().collect();
                                v.reverse();
                                v
                            }
                        }
                        None => vec![],
                    },
                };
                for e in evs {
                    let txt = w.event_text(&e);
                    let r = if EventKind::from_u8(e.kind).major() {
                        ui.add(egui::Label::new(egui::RichText::new(txt).strong()).sense(egui::Sense::click()))
                    } else {
                        ui.add(egui::Label::new(txt).sense(egui::Sense::click()))
                    };
                    if r.clicked() && w.band_index(e.band).is_some() {
                        jump = Some(e.band);
                    }
                }
            });
        });
    }
    if st.show_stats {
        egui::Window::new("Stats").default_pos([900.0, 40.0]).default_width(360.0).show(ctx, |ui| stats(ui, w, st));
    }
    if st.show_settings {
        egui::Window::new("Settings").default_pos([900.0, 300.0]).default_width(340.0).show(ctx, |ui| {
            settings(ui, w, st);
        });
    }
    if st.show_history {
        egui::Window::new("Settings history").show(ctx, |ui| {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                if w.settings_log.is_empty() {
                    ui.label("no changes yet");
                }
                for c in &w.settings_log {
                    ui.label(format!("Y{:.1}: {} {} → {}", c.tick as f32 / 4.0, c.name, c.old, c.new));
                }
            });
        });
    }
    if st.show_load {
        let mut open = true;
        egui::Window::new("Load / new world").open(&mut open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut st.new_seed).prefix("seed "));
                if ui.button("New world").clicked() {
                    st.req = Some(FileReq::New(st.new_seed));
                }
            });
            ui.separator();
            #[cfg(target_arch = "wasm32")]
            if ui.button("Open a .civsave file…").clicked() {
                crate::saveio::pick_file();
            }
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (p, t, sz) in st.saves.clone() {
                    let label = format!("{p}  ({:.1} MB, {})", sz as f64 / 1e6, civ_core::save::format_unix_time(t as f64));
                    if ui.button(label).clicked() {
                        st.req = Some(FileReq::Load(p));
                    }
                }
            });
        });
        st.show_load &= open;
    }
    if st.show_help {
        egui::Window::new("Help").default_pos([400.0, 200.0]).show(ctx, |ui| {
            ui.label(
                "Drag to pan, wheel to zoom, click a band to inspect it.\n\
                 Space pause · +/- speed · 1–6 overlays · F follow selected · Ctrl+S save · Ctrl+O load\n\n\
                 Discs are bands (size ~ people, colour = founding lineage, orange ring = keeps a fire).\n\
                 Nothing is scripted: each band experiments with what it has; physics of the materials\n\
                 decides what works. Know-how spreads by teaching between neighbours and is forgotten\n\
                 when unused.",
            );
        });
    }
    st.wants_pointer = ctx.wants_pointer_input() || ctx.is_pointer_over_area();
    st.wants_kb = ctx.wants_keyboard_input();
    jump
}

fn inspector(ui: &mut egui::Ui, w: &World, selected: Option<u32>, st: &mut UiState) {
    let Some(b) = selected.and_then(|id| w.band_index(id)).map(|i| &w.bands[i]) else {
        ui.label("Click a band on the map.");
        ui.separator();
        ui.label("First discoveries:");
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for f in &w.firsts {
                ui.label(format!("Y{:>5.1}  {}", f.tick as f32 / 4.0, item_name(f.item)));
            }
        });
        return;
    };
    let ci = b.cell(w.map.w);
    ui.label(format!(
        "Band {} (lineage {}, generation {}) at ({}, {}) – {}",
        b.id,
        b.lineage,
        b.gen,
        b.x,
        b.y,
        w.map.biome_at(ci).name()
    ));
    ui.label(format!(
        "{} people · store {:.0} · curiosity {:.2} · {}",
        b.pop,
        b.store,
        b.curiosity,
        if b.flame { "keeps a fire" } else { "no fire" }
    ));
    ui.label(format!(
        "food {:.2} of need · spare time {:.2} · stress {:.2} · cold {:.2}",
        b.d.food_ratio, b.d.spare, b.d.stress, b.d.cold
    ));
    ui.label(format!("invented {} · learned {} · forgot {}", b.inventions, b.learned, b.forgotten));
    ui.checkbox(&mut st.follow, "follow");
    ui.separator();
    ui.label(format!("Knows {} recipes ({} things it can make):", b.known.len(), b.n_items()));
    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        for k in &b.known {
            let Some(r) = w.reg.recipes.get(k.recipe as usize) else { continue };
            let from = match k.from {
                FROM_SELF => "own idea".to_string(),
                FROM_PARENT => "from parent band".to_string(),
                id => format!("from band {id}"),
            };
            let resp = ui
                .add(
                    egui::Label::new(egui::RichText::new(format!("{:<22} skill {:.2}  {}", item_name(r.output), k.skill, from)).monospace())
                        .sense(egui::Sense::click()),
                )
                .on_hover_text(r.describe());
            if resp.clicked() {
                st.overlay = 2;
                st.item = Some(r.output);
            }
        }
    });
}

fn chart(ui: &mut egui::Ui, title: &str, series: &[(&[f32], egui::Color32)]) {
    ui.label(title);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(200.0), 90.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    for (s, col) in series {
        if s.len() < 2 {
            continue;
        }
        let mx = s.iter().copied().fold(1e-6f32, f32::max);
        let pts: Vec<egui::Pos2> = s
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                egui::pos2(
                    rect.left() + rect.width() * i as f32 / (s.len() - 1) as f32,
                    rect.bottom() - rect.height() * (v / mx) * 0.95,
                )
            })
            .collect();
        p.add(egui::Shape::line(pts, egui::Stroke::new(1.5f32, *col)));
        p.text(rect.right_top(), egui::Align2::RIGHT_TOP, format!("max {mx:.0}"), egui::FontId::monospace(10.0), *col);
    }
}

fn stats(ui: &mut egui::Ui, w: &World, st: &mut UiState) {
    let h = &w.history;
    let pop: Vec<f32> = h.iter().map(|y| y.pop as f32).collect();
    let items: Vec<f32> = h.iter().map(|y| y.items_world as f32).collect();
    let mean: Vec<f32> = h.iter().map(|y| y.items_mean).collect();
    let inv: Vec<f32> = h.iter().map(|y| y.inventions as f32).collect();
    let tr: Vec<f32> = h.iter().map(|y| y.transfers as f32).collect();
    let fg: Vec<f32> = h.iter().map(|y| y.forgets as f32).collect();
    chart(ui, "population", &[(&pop, egui::Color32::LIGHT_GREEN)]);
    chart(ui, "things makeable: world (gold), mean per band (blue)", &[
        (&items, egui::Color32::GOLD),
        (&mean, egui::Color32::LIGHT_BLUE),
    ]);
    chart(ui, "per year: inventions (gold), lessons (green), forgettings (red)", &[
        (&inv, egui::Color32::GOLD),
        (&tr, egui::Color32::LIGHT_GREEN),
        (&fg, egui::Color32::LIGHT_RED),
    ]);
    let c = &w.counters;
    ui.label(format!(
        "experiments {} · inventions {} · lessons {} · forgotten {} · splits {} · bands died {}",
        c.experiments, c.inventions, c.transfers, c.forgets, c.splits, c.deaths
    ));
    ui.separator();
    ui.label("How many bands can make each thing now (click to map it):");
    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for f in &w.firsts {
            let n = w.item_bands.get(&f.item).copied().unwrap_or(0);
            if ui
                .add(egui::Label::new(egui::RichText::new(format!("{:<26} {:>4}  (first Y{:.0})", item_name(f.item), n, f.tick as f32 / 4.0)).monospace()).sense(egui::Sense::click()))
                .clicked()
            {
                st.overlay = 2;
                st.item = Some(f.item);
            }
        }
    });
}

fn settings(ui: &mut egui::Ui, w: &mut World, st: &mut UiState) {
    ui.label("Changes apply immediately and are recorded in the save.");
    let fields = w.params.fields();
    let mut v = serde_json::to_value(w.params).unwrap_or_default();
    let mut changed = false;
    egui::Grid::new("params").num_columns(2).show(ui, |ui| {
        for (name, val) in fields {
            ui.label(&name);
            let is_int = v.get(&name).is_some_and(|x| x.is_u64());
            let mut x = val;
            let range = if is_int { 0.0..=200.0 } else { 0.0..=(val.abs() * 4.0).max(1.0) };
            let r = ui.add(egui::DragValue::new(&mut x).range(range).speed(if is_int { 0.2 } else { (val.abs() * 0.01).max(0.0005) }));
            if r.changed() && x != val {
                v[&name] = if is_int { serde_json::json!(x.round().max(0.0) as u64) } else { serde_json::json!(x) };
                changed = true;
            }
            ui.end_row();
        }
    });
    if changed {
        if let Ok(p) = serde_json::from_value::<Params>(v) {
            w.params = p;
        }
    }
    ui.horizontal(|ui| {
        if ui.button("Reset to defaults").clicked() {
            w.params = Params::default();
        }
        ui.toggle_value(&mut st.show_history, "History");
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.separator();
        ui.add(egui::Slider::new(&mut st.autosave_min, 0.0..=60.0).text("autosave every (min, 0 = off)"));
        ui.add(egui::Slider::new(&mut st.autosave_keep, 1..=10).text("autosaves kept"));
    }
}
