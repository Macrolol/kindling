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
    /// a new world: seed, map size, chosen starts (empty: random), founders (0: default)
    New {
        seed: u64,
        w: usize,
        h: usize,
        starts: Vec<(u16, u16)>,
        founders: usize,
    },
    /// generate a world's map and open the start picker on it
    Preview {
        seed: u64,
        w: usize,
        h: usize,
    },
}

/// Start picker state (the preview world itself is held by the main loop).
pub struct Picker {
    pub seed: u64,
    pub w: usize,
    pub h: usize,
    /// 0 random (as before), 1 one shared start area, 2 each lineage placed individually
    pub mode: u8,
    pub starts: Vec<(u16, u16)>,
    pub founders: usize,
    pub note: String,
}

impl Picker {
    /// A click on cell (x, y) of the preview map.
    pub fn click(&mut self, map: &civ_core::map::Map, x: usize, y: usize) {
        use civ_core::StartCheck;
        let chk = map.start_check(x, y);
        let what = if x < map.w && y < map.h {
            map.biome_at(y * map.w + x).name()
        } else {
            "off the map"
        };
        if chk == StartCheck::Uninhabitable {
            self.note = format!("({x},{y}) is {what}: nobody can live there - pick land");
            return;
        }
        if self.mode == 0 {
            self.mode = 1;
        }
        let p = (x as u16, y as u16);
        if self.mode == 1 {
            self.starts = vec![p];
        } else if self.starts.len() < 24 {
            self.starts.push(p);
        } else {
            self.note = "24 lineages at most - undo one first".into();
            return;
        }
        self.note = if chk == StartCheck::Poor {
            format!("warning: ({x},{y}) {what} is poor land (cold, hot or little food) - the founders may struggle")
        } else {
            format!("({x},{y}) {what}: good land")
        };
    }
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
    pub show_legend: bool,
    /// zoom out to the whole map on the next frame (Home / 0 / the Fit map button)
    pub fit_view: bool,
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
    /// fast mode: more sim per frame, the map redrawn every 2 s, bands drawn only zoomed in
    pub fast: bool,
    /// sim worker threads (0 = all cores; native only)
    #[allow(dead_code)]
    pub threads: usize,
    /// map size for a new world
    pub new_w: usize,
    pub new_h: usize,
    pub picker: Option<Picker>,
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
            show_legend: true,
            fit_view: false,
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
            fast: false,
            threads: 0,
            new_w: 256,
            new_h: 160,
            picker: None,
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
pub fn draw(
    ctx: &egui::Context,
    st: &mut UiState,
    w: &mut World,
    selected: Option<u32>,
) -> Option<u32> {
    if st.picker.is_some() {
        picker(ctx, st, w);
        if st.show_legend {
            legend(ctx, st, w);
        }
        st.wants_pointer = ctx.wants_pointer_input() || ctx.is_pointer_over_area();
        st.wants_kb = ctx.wants_keyboard_input();
        return None;
    }
    let mut jump = None;
    egui::TopBottomPanel::top("top").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Kindling");
            // fixed-width monospace fields so nothing in the bar shifts as numbers/seasons change
            fixed(ui, 92.0, format!("Y{:<5} {:<6}", w.tick / 4, season_name(w.season())));
            fixed(ui, 92.0, format!("pop {:<7}", w.total_pop()));
            fixed(ui, 86.0, format!("bands {:<5}", w.bands.len()));
            fixed(ui, 72.0, format!("known {:<3}", w.item_bands.len()));
            ui.separator();
            let run = if st.paused { "▶ run" } else { "⏸ pause" };
            if ui.add_sized([70.0, 20.0], egui::Button::new(run)).clicked() {
                st.paused = !st.paused;
            }
            ui.add(egui::Slider::new(&mut st.speed, 0.5..=400.0).logarithmic(true).show_value(false));
            fixed(ui, 112.0, format!("{:>6.1} seasons/s", st.speed));
            ui.toggle_value(&mut st.fast, "⚡ fast").on_hover_text(
                "Fast mode (X): as much sim per frame as possible; the map is redrawn every 2 s and bands only when zoomed in",
            );
            ui.separator();
            egui::ComboBox::from_id_salt("overlay")
                .width(110.0)
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
                    .width(170.0)
                    .selected_text(st.item.map(item_name).unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for it in items {
                            ui.selectable_value(&mut st.item, Some(it), item_name(it));
                        }
                    });
            }
            ui.separator();
            if ui.button("⛶ Fit map").on_hover_text("Zoom out to the whole map (Home or 0)").clicked() {
                st.fit_view = true;
            }
            ui.toggle_value(&mut st.show_legend, "Legend");
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

    if st.show_legend {
        legend(ctx, st, w);
    }

    if st.show_inspector {
        egui::Window::new("Inspector")
            .default_pos([8.0, 40.0])
            .default_width(420.0)
            .show(ctx, |ui| {
                inspector(ui, w, selected, st);
            });
    }
    if st.show_log {
        egui::Window::new("Events")
            .default_pos([8.0, 470.0])
            .default_size([480.0, 260.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut st.log_mode, 0, "Chronicle");
                    ui.selectable_value(&mut st.log_mode, 1, "All");
                    ui.selectable_value(&mut st.log_mode, 2, "Selected band");
                });
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let evs: Vec<_> = match st.log_mode {
                            0 => w.chronicle.iter().copied().collect(),
                            1 => w.events.iter().rev().take(400).rev().copied().collect(),
                            _ => match selected {
                                Some(id) => {
                                    let mut v: Vec<_> = w
                                        .events
                                        .iter()
                                        .rev()
                                        .filter(|e| e.band == id || e.other == id)
                                        .take(300)
                                        .copied()
                                        .collect();
                                    v.reverse();
                                    v
                                }
                                None => vec![],
                            },
                        };
                        for e in evs {
                            let txt = w.event_text(&e);
                            let r = if EventKind::from_u8(e.kind).major() {
                                ui.add(
                                    egui::Label::new(egui::RichText::new(txt).strong())
                                        .sense(egui::Sense::click()),
                                )
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
        egui::Window::new("Stats")
            .default_pos([900.0, 40.0])
            .default_width(360.0)
            .show(ctx, |ui| stats(ui, w, st));
    }
    if st.show_settings {
        egui::Window::new("Settings")
            .default_pos([900.0, 300.0])
            .default_width(340.0)
            .show(ctx, |ui| {
                settings(ui, w, st);
            });
    }
    if st.show_history {
        egui::Window::new("Settings history").show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if w.settings_log.is_empty() {
                        ui.label("no changes yet");
                    }
                    for c in &w.settings_log {
                        ui.label(format!(
                            "Y{:.1}: {} {} → {}",
                            c.tick as f32 / 4.0,
                            c.name,
                            c.old,
                            c.new
                        ));
                    }
                });
        });
    }
    if st.show_load {
        let mut open = true;
        egui::Window::new("Load / new world")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.new_seed).prefix("seed "));
                    ui.add(
                        egui::DragValue::new(&mut st.new_w)
                            .range(32..=2048)
                            .prefix("map ")
                            .suffix(" w"),
                    );
                    ui.label("×");
                    ui.add(
                        egui::DragValue::new(&mut st.new_h)
                            .range(24..=2048)
                            .suffix(" h"),
                    );
                });
                ui.horizontal(|ui| {
                    if ui
                        .button("Choose start…")
                        .on_hover_text("Show the new map and place the founders yourself")
                        .clicked()
                    {
                        st.req = Some(FileReq::Preview {
                            seed: st.new_seed,
                            w: st.new_w,
                            h: st.new_h,
                        });
                    }
                    if ui.button("New world (random start)").clicked() {
                        st.req = Some(FileReq::New {
                            seed: st.new_seed,
                            w: st.new_w,
                            h: st.new_h,
                            starts: vec![],
                            founders: 0,
                        });
                    }
                });
                if st.new_w * st.new_h > 1_000_000 {
                    ui.label("big map: slow to generate and to run (use fast mode)");
                }
                ui.separator();
                #[cfg(target_arch = "wasm32")]
                if ui.button("Open a .civsave file…").clicked() {
                    crate::saveio::pick_file();
                }
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (p, t, sz) in st.saves.clone() {
                            let label = format!(
                                "{p}  ({:.1} MB, {})",
                                sz as f64 / 1e6,
                                civ_core::save::format_unix_time(t as f64)
                            );
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
                 Space pause · +/- speed · X fast mode · 1–9 overlays · Home or 0 fit whole map · F follow selected · Ctrl+S save · Ctrl+O load\n\n\
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
    let Some(b) = selected
        .and_then(|id| w.band_index(id))
        .map(|i| &w.bands[i])
    else {
        ui.label("Click a band on the map.");
        ui.separator();
        ui.label("First discoveries:");
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for f in &w.firsts {
                    ui.label(format!(
                        "Y{:>5.1}  {}",
                        f.tick as f32 / 4.0,
                        item_name(f.item)
                    ));
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
    ui.label(format!(
        "invented {} · learned {} · forgot {}",
        b.inventions, b.learned, b.forgotten
    ));
    {
        let wq = civ_core::band::weapon_q(&b.d.kit);
        let gs: Vec<String> = b
            .grudges
            .iter()
            .map(|g| format!("band {} ({:.1})", g.id, g.w))
            .collect();
        ui.label(format!(
            "weapons {:.2}{} · grudges: {}{}",
            wq,
            if b.d.kit.hunt != civ_core::materials::NONE {
                format!(" ({})", item_name(b.d.kit.hunt))
            } else {
                String::new()
            },
            if gs.is_empty() {
                "none".into()
            } else {
                gs.join(", ")
            },
            if b.pressed > 0.05 {
                format!(" · pressed by neighbours {:.0}%", 100.0 * b.pressed)
            } else {
                String::new()
            }
        ));
    }
    {
        let ci = b.cell(w.map.w);
        let mut parts = vec![];
        if b.sow > 0.0 {
            parts.push(format!(
                "farms ({:.0}% of food from fields and herds, grain {:.0}% domesticated)",
                100.0 * b.d.farm_share,
                100.0 * b.strain
            ));
        } else {
            parts.push("forages".to_string());
        }
        if b.herd > 0.0 {
            parts.push(format!(
                "herd {:.2}/person, tameness {:.0}%",
                b.herd,
                100.0 * b.tame
            ));
        }
        parts.push(format!("settled {:.0}%", 100.0 * b.settled));
        if b.sick > 0.05 {
            parts.push(format!("SICK {:.0}%", 100.0 * b.sick));
        }
        if b.immune > 0.05 {
            parts.push(format!("immunity {:.0}%", 100.0 * b.immune));
        }
        let t = w.tend.get(ci).copied().unwrap_or(0.0);
        let f = w.fert.get(ci).copied().unwrap_or(1.0);
        parts.push(format!(
            "here: tended {:.0}%, soil {:.0}%",
            100.0 * t,
            100.0 * f
        ));
        ui.label(parts.join(" · "));
    }
    ui.checkbox(&mut st.follow, "follow");
    ui.separator();
    ui.label(format!(
        "Knows {} recipes ({} things it can make):",
        b.known.len(),
        b.n_items()
    ));
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for k in &b.known {
                let Some(r) = w.reg.recipes.get(k.recipe as usize) else {
                    continue;
                };
                let from = match k.from {
                    FROM_SELF => "own idea".to_string(),
                    FROM_PARENT => "from parent band".to_string(),
                    id => format!("from band {id}"),
                };
                let resp = ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{:<22} skill {:.2}  {}",
                                item_name(r.output),
                                k.skill,
                                from
                            ))
                            .monospace(),
                        )
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
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(200.0), 90.0),
        egui::Sense::hover(),
    );
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
        p.text(
            rect.right_top(),
            egui::Align2::RIGHT_TOP,
            format!("max {mx:.0}"),
            egui::FontId::monospace(10.0),
            *col,
        );
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
    chart(
        ui,
        "things makeable: world (gold), mean per band (blue)",
        &[
            (&items, egui::Color32::GOLD),
            (&mean, egui::Color32::LIGHT_BLUE),
        ],
    );
    chart(
        ui,
        "per year: inventions (gold), lessons (green), forgettings (red)",
        &[
            (&inv, egui::Color32::GOLD),
            (&tr, egui::Color32::LIGHT_GREEN),
            (&fg, egui::Color32::LIGHT_RED),
        ],
    );
    // lineage shares (v5 series; older saves have zeros for the years before they loaded)
    let top: Vec<f32> = h.iter().map(|y| y.top_lineage_pct).collect();
    let nlin: Vec<f32> = h.iter().map(|y| y.lineages as f32).collect();
    if top.iter().any(|&v| v > 0.0) {
        chart(
            ui,
            "largest lineage % of people (violet), lineages alive (grey)",
            &[
                (&top, egui::Color32::from_rgb(190, 140, 255)),
                (&nlin, egui::Color32::GRAY),
            ],
        );
    }
    let c = &w.counters;
    ui.label(format!(
        "experiments {} · inventions {} · lessons {} · forgotten {} · splits {} · bands died {}",
        c.experiments, c.inventions, c.transfers, c.forgets, c.splits, c.deaths
    ));
    ui.separator();
    ui.label("How many bands can make each thing now (click to map it):");
    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            for f in &w.firsts {
                let n = w.item_bands.get(&f.item).copied().unwrap_or(0);
                if ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{:<26} {:>4}  (first Y{:.0})",
                                item_name(f.item),
                                n,
                                f.tick as f32 / 4.0
                            ))
                            .monospace(),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    st.overlay = 2;
                    st.item = Some(f.item);
                }
            }
        });
}

fn settings(ui: &mut egui::Ui, w: &mut World, st: &mut UiState) {
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        ui.label("Threads");
        ui.add(egui::DragValue::new(&mut st.threads).range(0..=256))
            .on_hover_text("sim worker threads; 0 = all cores");
        ui.label(if st.threads == 0 { "(all cores)" } else { "" });
    });
    ui.label("Changes apply immediately and are recorded in the save.");
    let fields = w.params.fields();
    let mut v = serde_json::to_value(w.params).unwrap_or_default();
    let mut changed = false;
    egui::Grid::new("params").num_columns(2).show(ui, |ui| {
        for (name, val) in fields {
            ui.label(&name);
            let is_int = v.get(&name).is_some_and(|x| x.is_u64());
            let mut x = val;
            let range = if is_int {
                0.0..=200.0
            } else {
                0.0..=(val.abs() * 4.0).max(1.0)
            };
            let r = ui.add(egui::DragValue::new(&mut x).range(range).speed(if is_int {
                0.2
            } else {
                (val.abs() * 0.01).max(0.0005)
            }));
            if r.changed() && x != val {
                v[&name] = if is_int {
                    serde_json::json!(x.round().max(0.0) as u64)
                } else {
                    serde_json::json!(x)
                };
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
        ui.add(
            egui::Slider::new(&mut st.autosave_min, 0.0..=60.0)
                .text("autosave every (min, 0 = off)"),
        );
        ui.add(egui::Slider::new(&mut st.autosave_keep, 1..=10).text("autosaves kept"));
    }
}

/// A monospace label in a box of fixed width, left-aligned, so its neighbours never move.
fn fixed(ui: &mut egui::Ui, width: f32, text: String) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.add(
                egui::Label::new(egui::RichText::new(text).monospace())
                    .wrap_mode(egui::TextWrapMode::Extend),
            );
        },
    );
}

fn c32(c: [f32; 3]) -> egui::Color32 {
    let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
    egui::Color32::from_rgb(f(c[0]), f(c[1]), f(c[2]))
}

fn swatch(ui: &mut egui::Ui, c: egui::Color32, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 2.0, c);
        ui.label(text);
    });
}

/// Colour ramp bar with tick labels at fractions 0..1.
fn ramp_bar(ui: &mut egui::Ui, ticks: &[(f32, String)]) {
    let width = 280.0;
    let (r, _) = ui.allocate_exact_size(egui::vec2(width, 32.0), egui::Sense::hover());
    let p = ui.painter();
    let n = 70;
    for k in 0..n {
        let x0 = r.left() + width * k as f32 / n as f32;
        let x1 = r.left() + width * (k + 1) as f32 / n as f32 + 0.5;
        let col = c32(crate::render::ramp((k as f32 + 0.5) / n as f32));
        p.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, r.top()), egui::pos2(x1, r.top() + 14.0)),
            0.0,
            col,
        );
    }
    let txt = ui.visuals().text_color();
    for (i, (f, s)) in ticks.iter().enumerate() {
        let x = r.left() + width * f.clamp(0.0, 1.0);
        p.line_segment(
            [egui::pos2(x, r.top() + 14.0), egui::pos2(x, r.top() + 18.0)],
            egui::Stroke::new(1.0_f32, txt),
        );
        let align = if i == 0 {
            egui::Align2::LEFT_TOP
        } else if i + 1 == ticks.len() {
            egui::Align2::RIGHT_TOP
        } else {
            egui::Align2::CENTER_TOP
        };
        p.text(
            egui::pos2(x, r.top() + 18.0),
            align,
            s,
            egui::FontId::proportional(12.0),
            txt,
        );
    }
}

fn band_key(ui: &mut egui::Ui, item_mode: bool) {
    ui.separator();
    ui.label(egui::RichText::new("Bands (dots)").strong());
    ui.label("Each dot is one band of people. Bigger dot = more people. Colour = family line (bands that split from the same ancestor share a colour).");
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(r.center(), 5.0, egui::Color32::from_rgb(120, 200, 140));
        ui.painter().circle_stroke(
            r.center(),
            6.5,
            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 150, 40)),
        );
        ui.label("orange ring = keeps a fire going");
    });
    if item_mode {
        swatch(
            ui,
            egui::Color32::from_rgb(90, 90, 90),
            "grey dot = this band can't make the chosen item",
        );
    }
}

fn legend(ctx: &egui::Context, st: &UiState, w: &World) {
    use crate::render::*;
    let title = format!("Legend: {}", OVERLAYS[st.overlay as usize]);
    egui::Window::new("Legend")
        .id(egui::Id::new("legend"))
        .title_bar(true)
        .collapsible(true)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-8.0, -8.0])
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.set_width(300.0);
            ui.label(egui::RichText::new(title).strong());
            let unreached = c32([0.3, 0.3, 0.3]);
            match st.overlay {
                0 => {
                    ui.label("Natural landscape. Shading shows hills (lit from the north-west).");
                    egui::Grid::new("biomes").num_columns(2).show(ui, |ui| {
                        for (k, (b, name)) in BIOME_KEY.iter().enumerate() {
                            swatch(ui, c32(biome_rgb(*b)), name);
                            if k % 2 == 1 {
                                ui.end_row();
                            }
                        }
                        swatch(ui, c32(RIVER_RGB), "river");
                        ui.end_row();
                    });
                }
                1 => {
                    let max = w.bands.iter().map(|b| b.n_items()).max().unwrap_or(0).max(1);
                    ui.label("How many different things the bands living here can make (tools, fire, pots, food preparations...). Painted over each band's home range; the most skilled band nearby wins.");
                    ramp_bar(ui, &[(0.0, "0".into()), (0.5, format!("{}", max / 2)), (1.0, format!("{max} things (most today)"))]);
                    swatch(ui, unreached, "grey = nobody lives nearby");
                }
                2 => {
                    let name = st.item.map(item_name).unwrap_or_default();
                    let now = w.tick / 4;
                    ui.label(format!("When knowledge of \"{name}\" first reached each place. Dark = got it early, yellow = got it recently. A smooth gradient means the idea spread from band to band."));
                    ramp_bar(ui, &[(0.0, "year 0".into()), (0.5, format!("year {}", now / 2)), (1.0, format!("year {now} (now)"))]);
                    swatch(ui, unreached, "grey = has never reached here");
                }
                3 => {
                    ui.label("People living in each map cell (all bands there added up). The scale is logarithmic: each step along the bar multiplies the number of people.");
                    let t = |n: f32| (n.ln() / POP_LN_MAX, format!("{}", n.round() as u32));
                    let e = POP_LN_MAX.exp();
                    ramp_bar(ui, &[t(1.0), t(7.0), t(55.0), (1.0, format!("{}+ people", e.round() as u32))]);
                    swatch(ui, unreached, "grey = nobody lives here");
                }
                4 => {
                    ui.label("Raw materials in the ground that bands can pick up and work with. If a cell has several, the lowest one in this list is shown.");
                    for (_, col, name) in DEPOSIT_KEY {
                        swatch(ui, c32(col), name);
                    }
                    swatch(ui, c32([0.25, 0.25, 0.25]), "dark grey = nothing special");
                }
                6 => {
                    ui.label("Goods changing hands recently: each exchange marks the cells between the two bands (stone, flint, obsidian, hides, fibre, ores, metal). Bright lines are busy routes; lines over the sea are boat trips between partners. Fades within a couple of years once trade stops.");
                    let t = |f: f32| ((1.0 + f).ln() / TRADE_LN_MAX, format!("{f:.1}"));
                    let e = TRADE_LN_MAX.exp() - 1.0;
                    ramp_bar(ui, &[t(0.0), t(1.0), t(5.0), (1.0, format!("{e:.0}+ loads"))]);
                    swatch(ui, unreached, "grey = no trade passing (darker = land)");
                    let h = w.history.last();
                    if let Some(h) = h {
                        ui.label(format!("Last year: {} exchanges, {} partnerships, carried metal/obsidian on average {:.1} cells from its source.", h.trades, h.partnerships, h.goods_dist_mean));
                    }
                }
                7 => {
                    ui.label("Fighting between bands: people killed around each place recently. Fights flare where hungry bands crowd the same ground, covet a neighbour's stores or goods, or nurse a grudge; most tense meetings still end with one side backing off or paying for peace. Fades over several years once the fighting stops.");
                    let t = |f: f32| ((1.0 + f).ln() / CONFLICT_LN_MAX, format!("{f:.0}"));
                    let e = CONFLICT_LN_MAX.exp() - 1.0;
                    egui::Grid::new("cramp").num_columns(4).show(ui, |ui| {
                        for (f, lab) in [t(1.0), t(5.0), t(12.0), (1.0, format!("{e:.0}+ dead"))] {
                            swatch(ui, c32(conflict_ramp(f)), &lab);
                        }
                    });
                    swatch(ui, unreached, "grey = no recent fighting (darker = land)");
                    if let Some(h) = w.history.last() {
                        ui.label(format!(
                            "Last year: {} confrontations: {} backed off, {} paid for peace, {} fights ({} killed), {} bands driven off, {} absorbed by their victors.",
                            h.confrontations, h.avoided, h.tributes, h.fights, h.conflict_deaths, h.displaced, h.absorbed
                        ));
                    }
                    let c = &w.counters;
                    ui.label(format!("All time: {} fights, {} killed, {} bands driven off ({} by sea), {} captives taken.", c.fights, c.conflict_deaths, c.displaced, c.fled_by_sea, c.captives));
                }
                8 => {
                    ui.label("Farming. Bands that camp in the same place year after year tend the wild grasses and seeds around them; some go on to clear and sow fields, and grain kept for seed slowly becomes a domestic crop. Harvests wear out the soil unless it rests, is flooded by a river, or gets manure from herds.");
                    swatch(ui, c32(FARM_TENDED), "tended wild stands (weeding, scattering seed)");
                    swatch(ui, c32(FARM_FIELD), "cultivated fields (wild-type grain)");
                    swatch(ui, c32(FARM_DOMESTIC), "fields of domesticated grain");
                    swatch(ui, c32(FARM_EXHAUSTED), "exhausted soil (browner = more worn out)");
                    swatch(ui, c32(FARM_PASTURE), "grazing range of herding bands");
                    swatch(ui, c32([0.55, 0.55, 0.55]), "grey = no farming; lighter grey = better farmland");
                    if let Some(h) = w.history.last() {
                        ui.label(format!(
                            "Last year: {} farming bands ({} people) and {} herding bands; {} field cells ({} domesticated), mean soil fertility {:.0}%; {:.0}% of all food from fields and herds. {} epidemics, {} died of disease.",
                            h.farmers, h.farmer_pop, h.herders, h.fields, h.dom_fields, h.field_fert * 100.0, h.farm_food_share * 100.0, h.epidemics, h.disease_deaths
                        ));
                    }
                    let c = &w.counters;
                    ui.label(format!("All time: farming began independently {} times; {} bands took it up, {} gave it up; {} epidemics.", c.farm_origins, c.took_up_farming, c.abandoned_farming, c.epidemics));
                }
                _ => {
                    ui.label("Average temperature over the year. Seasons swing above and below this; cold places need fire and clothing.");
                    let f = |t: f32| ((t - TEMP_LO) / (TEMP_HI - TEMP_LO), format!("{t:.0}°C"));
                    ramp_bar(ui, &[f(TEMP_LO), f(0.0), f(15.0), f(TEMP_HI)]);
                    ui.label("Colder than -15°C shows as the darkest colour, hotter than 30°C as the brightest.");
                }
            }
            band_key(ui, st.overlay == 2);
        });
}

/// The start picker window (the map behind it is the new world's).
fn picker(ctx: &egui::Context, st: &mut UiState, w: &World) {
    let mut go = false;
    let mut back = false;
    let p = st.picker.as_mut().unwrap();
    egui::Window::new("Choose where people start").default_pos([8.0, 40.0]).default_width(380.0).show(ctx, |ui| {
        ui.label(format!("Seed {} · map {}×{}", p.seed, p.w, p.h));
        ui.horizontal(|ui| {
            ui.label("Overlay");
            egui::ComboBox::from_id_salt("pick_overlay")
                .width(120.0)
                .selected_text(OVERLAYS[st.overlay as usize])
                .show_ui(ui, |ui| {
                    for (i, n) in OVERLAYS.iter().enumerate() {
                        if matches!(i, 0 | 4 | 5 | 8) {
                            ui.selectable_value(&mut st.overlay, i as u8, *n);
                        }
                    }
                });
            ui.toggle_value(&mut st.show_legend, "Legend");
        });
        ui.separator();
        ui.radio_value(&mut p.mode, 0, "Random (as before: a rich region drawn by the seed)");
        ui.radio_value(&mut p.mode, 1, "One shared start area (click once; all lineages spread from it)");
        ui.radio_value(&mut p.mode, 2, "Place each founding lineage (click once per lineage)");
        if p.mode == 1 && p.starts.len() > 1 {
            p.starts.truncate(1);
        }
        if p.mode != 2 {
            ui.horizontal(|ui| {
                ui.label("founding bands");
                ui.add(egui::DragValue::new(&mut p.founders).range(1..=24));
            });
        }
        match p.mode {
            0 => ui.label("Click the map to choose a start instead."),
            1 => ui.label(match p.starts.first() {
                Some(&(x, y)) => format!("start area around ({x},{y})"),
                None => "click the map to place the start area".into(),
            }),
            _ => ui.label(format!("{} lineages placed (each starts on its own cell)", p.starts.len())),
        };
        if p.mode == 2 {
            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                for (k, &(x, y)) in p.starts.iter().enumerate() {
                    let i = y as usize * w.map.w + x as usize;
                    let warn = if w.map.start_check(x as usize, y as usize) == civ_core::StartCheck::Poor { "  (poor land)" } else { "" };
                    ui.label(format!("lineage {}: ({x},{y}) {}{warn}", k + 1, w.map.biome_at(i).name()));
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.button("Undo").clicked() {
                p.starts.pop();
            }
            if ui.button("Clear").clicked() {
                p.starts.clear();
            }
        });
        if !p.note.is_empty() {
            let col = if p.note.contains("nobody") || p.note.starts_with("warning") { egui::Color32::from_rgb(255, 170, 60) } else { egui::Color32::LIGHT_GREEN };
            ui.colored_label(col, &p.note);
        }
        ui.separator();
        ui.horizontal(|ui| {
            let ready = p.mode == 0 || !p.starts.is_empty();
            if ui.add_enabled(ready, egui::Button::new("Start this world")).clicked() {
                go = true;
            }
            if ui.button("Back").clicked() {
                back = true;
            }
        });
        ui.label("Sea and ice are refused. Drag to pan, wheel to zoom, Home or 0 to fit the whole map.");
    });
    if go {
        let p = st.picker.take().unwrap();
        let (starts, founders) = match p.mode {
            0 => (vec![], p.founders),
            1 => (p.starts, p.founders),
            _ => {
                let n = p.starts.len();
                (p.starts, n)
            }
        };
        st.req = Some(FileReq::New {
            seed: p.seed,
            w: p.w,
            h: p.h,
            starts,
            founders,
        });
    } else if back {
        st.picker = None;
        st.show_load = true;
    }
}
