//! Map rendering: a per-cell texture (terrain or an overlay) plus bands drawn as discs.
use civ_core::map::{Biome, D_CLAY, D_COPPER, D_FLINT, D_IRON, D_OBSIDIAN, D_STONE, D_TIN};
use civ_core::materials::Item;
use civ_core::World;
use macroquad::prelude::*;

/// Biomes in legend order with plain names.
pub const BIOME_KEY: [(Biome, &str); 11] = [
    (Biome::Ocean, "ocean"),
    (Biome::Ice, "ice"),
    (Biome::Tundra, "tundra"),
    (Biome::Taiga, "taiga (cold forest)"),
    (Biome::Steppe, "steppe (dry grass)"),
    (Biome::Grassland, "grassland"),
    (Biome::TempForest, "temperate forest"),
    (Biome::Desert, "desert"),
    (Biome::Savanna, "savanna"),
    (Biome::TropForest, "tropical forest"),
    (Biome::Mountain, "mountain"),
];
pub const RIVER_RGB: [f32; 3] = [0.25, 0.45, 0.75];
/// Deposit colours, in drawing order (a later one wins when a cell has several).
pub const DEPOSIT_KEY: [(u32, [f32; 3], &str); 7] = [
    (D_STONE as u32, [0.55, 0.55, 0.6], "hard stone (cobbles for hammers)"),
    (D_CLAY as u32, [0.75, 0.45, 0.3], "clay (pots, kiln lining)"),
    (D_FLINT as u32, [0.95, 0.95, 0.85], "flint (sharp flakes)"),
    (D_OBSIDIAN as u32, [0.15, 0.1, 0.25], "obsidian (sharpest edges)"),
    (D_COPPER as u32, [0.15, 0.85, 0.6], "copper ore"),
    (D_TIN as u32, [0.55, 0.75, 1.0], "tin ore"),
    (D_IRON as u32, [0.7, 0.25, 0.15], "iron ore (bog and red earth)"),
];
/// Temperature overlay range (°C) and population overlay scale (ln people per cell).
pub const TEMP_LO: f32 = -15.0;
pub const TEMP_HI: f32 = 30.0;
pub const POP_LN_MAX: f32 = 6.0;
/// Trade overlay scale: ln(1 + recent goods through a cell).
pub const TRADE_LN_MAX: f32 = 3.0;

/// Conflict overlay scale: ln(1 + people recently killed in fights around a cell).
pub const CONFLICT_LN_MAX: f32 = 3.5;

pub const OVERLAYS: [&str; 9] = ["terrain", "know-how", "item spread", "population", "deposits", "temperature", "trade", "conflict", "farming"];

/// Farming overlay colours.
pub const FARM_TENDED: [f32; 3] = [0.45, 0.75, 0.4];
pub const FARM_FIELD: [f32; 3] = [0.85, 0.85, 0.25];
pub const FARM_DOMESTIC: [f32; 3] = [1.0, 0.65, 0.05];
pub const FARM_EXHAUSTED: [f32; 3] = [0.5, 0.3, 0.15];
pub const FARM_PASTURE: [f32; 3] = [0.75, 0.55, 0.85];

/// Farming overlay colour of cell `i`, or None for no farming (then shaded by crop potential).
pub fn farm_rgb(w: &World, i: usize) -> Option<[f32; 3]> {
    let t = w.tend.get(i).copied().unwrap_or(0.0);
    let d = w.dom.get(i).copied().unwrap_or(0.0);
    let f = w.fert.get(i).copied().unwrap_or(1.0);
    if t > 0.4 {
        let base = if d > 0.6 { FARM_DOMESTIC } else { lerp3(FARM_FIELD, FARM_DOMESTIC, (d / 0.6) * 0.5) };
        // worn-out soil browns the field
        Some(lerp3(FARM_EXHAUSTED, base, ((f - 0.3) / 0.5).clamp(0.0, 1.0)))
    } else if t > 0.08 {
        Some(lerp3([0.3, 0.42, 0.3], FARM_TENDED, ((t - 0.08) / 0.32).clamp(0.0, 1.0)))
    } else {
        None
    }
}

/// Camera: centre in cell coordinates, zoom in screen pixels per cell.
pub struct View {
    pub cx: f32,
    pub cy: f32,
    pub zoom: f32,
}
impl View {
    /// Zoom (pixels per cell) at which the whole map fits the window, below the top bar.
    pub fn fit_zoom(w: &World) -> f32 {
        let z = (screen_width() / w.map.w as f32).min((screen_height() - 30.0) / w.map.h as f32);
        z.clamp(0.05, Self::MAX_ZOOM)
    }
    /// Furthest zoom-out allowed: half the fit zoom, so the whole map (any size) always fits.
    pub fn min_zoom(w: &World) -> f32 {
        (Self::fit_zoom(w) * 0.5).min(1.0)
    }
    pub const MAX_ZOOM: f32 = 80.0;
    /// The whole map, centred below the top bar (Home / 0 / "Fit map").
    pub fn fit(w: &World) -> View {
        View { cx: w.map.w as f32 / 2.0, cy: w.map.h as f32 / 2.0 - 15.0 / Self::fit_zoom(w), zoom: Self::fit_zoom(w) }
    }
    pub fn to_screen(&self, x: f32, y: f32) -> Vec2 {
        vec2((x - self.cx) * self.zoom + screen_width() / 2.0, (y - self.cy) * self.zoom + screen_height() / 2.0)
    }
    pub fn to_world(&self, p: Vec2) -> Vec2 {
        vec2((p.x - screen_width() / 2.0) / self.zoom + self.cx, (p.y - screen_height() / 2.0) / self.zoom + self.cy)
    }
}

/// Dark red to orange to pale yellow: how much blood was shed here lately.
pub fn conflict_ramp(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [0.35 + 0.65 * t.min(0.5) * 2.0, 0.05 + 0.9 * (t - 0.35).max(0.0) / 0.65, 0.05 + 0.6 * (t - 0.7).max(0.0) / 0.3]
}

pub fn biome_rgb(b: Biome) -> [f32; 3] {
    use Biome::*;
    match b {
        Ocean => [0.11, 0.22, 0.38],
        Ice => [0.90, 0.93, 0.96],
        Tundra => [0.62, 0.64, 0.58],
        Taiga => [0.24, 0.40, 0.33],
        Steppe => [0.70, 0.67, 0.45],
        Grassland => [0.50, 0.65, 0.33],
        TempForest => [0.25, 0.50, 0.25],
        Desert => [0.85, 0.76, 0.53],
        Savanna => [0.72, 0.68, 0.36],
        TropForest => [0.12, 0.42, 0.20],
        Mountain => [0.52, 0.48, 0.45],
    }
}
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
/// Ramp from dark violet through teal to yellow (early -> late or low -> high).
pub fn ramp(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    let stops = [[0.27, 0.0, 0.33], [0.23, 0.32, 0.55], [0.13, 0.57, 0.55], [0.37, 0.79, 0.38], [0.99, 0.91, 0.14]];
    let f = t * 4.0;
    let i = (f.floor() as usize).min(3);
    lerp3(stops[i], stops[i + 1], f - i as f32)
}

pub fn lineage_color(lineage: u32) -> Color {
    let h = ((lineage.wrapping_mul(2654435761) >> 8) % 360) as f32 / 360.0;
    macroquad::color::hsl_to_rgb(h, 0.75, 0.6)
}

/// Overlays that do not change as the world runs (only redrawn when switched to).
pub fn static_overlay(overlay: u8) -> bool {
    matches!(overlay, 0 | 4 | 5)
}

/// Builds the RGBA pixels of the map for an overlay (rows in parallel on native builds).
pub fn map_pixels(w: &World, overlay: u8, item: Option<Item>) -> Vec<u8> {
    let m = &w.map;
    let n = m.w * m.h;
    let mut px = vec![0u8; n * 4];
    // per-cell aggregates from bands (only what the overlay shows)
    let mut pop = vec![0u32; if overlay == 3 { n } else { 0 }];
    let mut know = vec![0u32; if overlay == 1 { n } else { 0 }];
    let mut herd = vec![false; if overlay == 8 { n } else { 0 }];
    if matches!(overlay, 1 | 3 | 8) {
        for b in &w.bands {
            let c = b.cell(m.w);
            if overlay == 3 {
                pop[c] += b.pop;
            }
            if overlay == 8 && b.herd > 0.05 {
                // grazing range of a herding band
                let (bx, by) = (b.x as i32, b.y as i32);
                for dy in -1..=1i32 {
                    for dx in -1..=1i32 {
                        let (x, y) = (bx + dx, by + dy);
                        if x >= 0 && y >= 0 && (x as usize) < m.w && (y as usize) < m.h {
                            herd[y as usize * m.w + x as usize] = true;
                        }
                    }
                }
            }
            if overlay == 1 {
                // know-how is shown over each band's home range (2 cells), not just its own cell
                let (bx, by) = (b.x as i32, b.y as i32);
                let nk = b.n_items();
                for dy in -2..=2i32 {
                    for dx in -2..=2i32 {
                        let (x, y) = (bx + dx, by + dy);
                        if x >= 0 && y >= 0 && (x as usize) < m.w && (y as usize) < m.h && dx * dx + dy * dy <= 5 {
                            let j = y as usize * m.w + x as usize;
                            know[j] = know[j].max(nk);
                        }
                    }
                }
            }
        }
    }
    let max_know = know.iter().copied().max().unwrap_or(1).max(1) as f32;
    let cf = item.and_then(|it| w.cell_first.get(&it));
    let now = w.tick.max(1) as f32;
    let fill_row = |y: usize, row: &mut [u8]| {
        for x in 0..m.w {
            let i = m.idx(x, y);
            let land = m.is_land(i);
            // hillshade from the height gradient (light from the north-west)
            let hgt = |xx: usize, yy: usize| m.height[m.idx(xx.min(m.w - 1), yy.min(m.h - 1))].max(0.0);
            let dz = hgt(x + 1, y + 1) - hgt(x.saturating_sub(1), y.saturating_sub(1));
            let shade = (1.0 - dz / 900.0).clamp(0.65, 1.25);
            let mut c = biome_rgb(m.biome_at(i));
            if land && m.river[i] {
                c = lerp3(c, RIVER_RGB, 0.75);
            }
            if land {
                c = [c[0] * shade, c[1] * shade, c[2] * shade];
            }
            let grey = |c: [f32; 3], k: f32| {
                let g = (c[0] + c[1] + c[2]) / 3.0 * k;
                [g, g, g]
            };
            let c = match overlay {
                1 if land => {
                    if know[i] > 0 {
                        ramp(know[i] as f32 / max_know)
                    } else {
                        grey(c, 0.55)
                    }
                }
                2 if land => match cf.map(|v| v.get(i)) {
                    Some(t) if t != u32::MAX => ramp(t as f32 / now),
                    _ => grey(c, 0.55),
                },
                3 if land => {
                    if pop[i] > 0 {
                        ramp((pop[i] as f32).ln() / POP_LN_MAX)
                    } else {
                        grey(c, 0.55)
                    }
                }
                4 if land => {
                    let d = m.deposits[i];
                    let mut g = grey(c, 0.45);
                    for (bit, col, _) in DEPOSIT_KEY {
                        if d as u32 & bit != 0 {
                            g = col;
                        }
                    }
                    g
                }
                5 => ramp((m.temp[i] - TEMP_LO) / (TEMP_HI - TEMP_LO)),
                6 => {
                    let f = w.trade_flow.get(i).copied().unwrap_or(0.0);
                    if f > 0.02 {
                        ramp((1.0 + f).ln() / TRADE_LN_MAX)
                    } else if land {
                        grey(c, 0.45)
                    } else {
                        grey(c, 0.8)
                    }
                }
                7 => {
                    let f = w.conflict_flow.get(i).copied().unwrap_or(0.0);
                    if f > 0.05 {
                        conflict_ramp((1.0 + f).ln() / CONFLICT_LN_MAX)
                    } else if land {
                        grey(c, 0.45)
                    } else {
                        grey(c, 0.8)
                    }
                }
                8 if land => match farm_rgb(w, i) {
                    Some(fc) => fc,
                    None => {
                        let p = w.crop_pot.get(i).copied().unwrap_or(0.0);
                        let mut g = grey(c, 0.35 + 0.35 * (p / 1.3).min(1.0));
                        if herd[i] {
                            g = lerp3(g, FARM_PASTURE, 0.6);
                        }
                        g
                    }
                },
                _ => c,
            };
            let o = x * 4;
            row[o] = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
            row[o + 1] = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
            row[o + 2] = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
            row[o + 3] = 255;
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        px.par_chunks_mut(m.w * 4).enumerate().for_each(|(y, row)| fill_row(y, row));
    }
    #[cfg(target_arch = "wasm32")]
    for (y, row) in px.chunks_mut(m.w * 4).enumerate() {
        fill_row(y, row);
    }
    px
}

/// The map texture, rebuilt when the overlay or the world changes. On big maps a rebuild is
/// costly, so while the world runs it is redrawn at most every `min_dt` seconds (adaptive: a few
/// times the last rebuild's cost, so drawing never takes more than a fraction of the frame), and
/// overlays that do not change over time (terrain, deposits, temperature) are never redrawn.
pub struct MapTex {
    pub tex: Texture2D,
    pub key: (u32, u8, Option<Item>, usize, u64),
    /// wall time of the last rebuild, and how long it took (s)
    pub at: f64,
    pub cost: f64,
}
impl MapTex {
    fn key(w: &World, overlay: u8, item: Option<Item>) -> (u32, u8, Option<Item>, usize, u64) {
        let tick = if static_overlay(overlay) { 0 } else { w.tick };
        (tick, overlay, item, w.map.w * w.map.h, w.cfg.seed)
    }
    pub fn new(w: &World, overlay: u8, item: Option<Item>) -> MapTex {
        let t0 = get_time();
        let px = map_pixels(w, overlay, item);
        let tex = Texture2D::from_rgba8(w.map.w as u16, w.map.h as u16, &px);
        tex.set_filter(FilterMode::Nearest);
        let now = get_time();
        MapTex { tex, key: Self::key(w, overlay, item), at: now, cost: now - t0 }
    }
    /// `fast`: fast mode (redraw the map at most every 2 s).
    pub fn refresh(&mut self, w: &World, overlay: u8, item: Option<Item>, fast: bool) {
        let key = Self::key(w, overlay, item);
        if key == self.key {
            return;
        }
        let same_view = (key.1, key.2, key.3, key.4) == (self.key.1, self.key.2, self.key.3, self.key.4);
        if same_view {
            let min_dt = if fast { 2.0 } else { (4.0 * self.cost).min(0.5) };
            if get_time() - self.at < min_dt {
                return;
            }
        }
        let t0 = get_time();
        let px = map_pixels(w, overlay, item);
        if key.3 == self.key.3 && self.tex.width() as usize == w.map.w {
            self.tex.update_from_bytes(w.map.w as u32, w.map.h as u32, &px);
        } else {
            self.tex = Texture2D::from_rgba8(w.map.w as u16, w.map.h as u16, &px);
            self.tex.set_filter(FilterMode::Nearest);
        }
        let now = get_time();
        self.key = key;
        self.at = now;
        self.cost = now - t0;
    }
}

/// `bands`: draw the bands (off in the start picker; in fast mode only when zoomed in or few).
pub fn draw_world(w: &World, v: &View, mt: &MapTex, overlay: u8, item: Option<Item>, selected: Option<u32>, bands: bool) {
    clear_background(Color::from_rgba(12, 18, 30, 255));
    // under one pixel per cell, sample the map smoothly (each pixel blends the cells it covers)
    // instead of picking one cell per pixel, which shimmers when panning a zoomed-out big map
    mt.tex.set_filter(if v.zoom < 1.0 { FilterMode::Linear } else { FilterMode::Nearest });
    let p0 = v.to_screen(0.0, 0.0);
    draw_texture_ex(
        &mt.tex,
        p0.x,
        p0.y,
        WHITE,
        DrawTextureParams { dest_size: Some(vec2(w.map.w as f32 * v.zoom, w.map.h as f32 * v.zoom)), ..Default::default() },
    );
    if !bands {
        return;
    }
    // bands: disc area ~ population; colour = founding lineage; orange ring = keeps fire. With
    // tens of thousands on screen the outlines are left off (they would be a pixel wide anyway)
    let crowded = w.bands.len() > 15000 && v.zoom < 3.0;
    for b in &w.bands {
        let p = v.to_screen(b.x as f32 + 0.5, b.y as f32 + 0.5);
        if p.x < -20.0 || p.y < -20.0 || p.x > screen_width() + 20.0 || p.y > screen_height() + 20.0 {
            continue;
        }
        let r = band_radius(b.pop, v.zoom);
        let mut col = lineage_color(b.lineage);
        if let Some(it) = item {
            if overlay == 2 && !b.d.tk.producible.has(it) {
                col = Color::new(0.35, 0.35, 0.35, 0.8);
            }
        }
        if v.zoom < 1.0 {
            // far out: a dot is all that shows; squares are much cheaper than 20-segment discs
            draw_rectangle(p.x - r, p.y - r, 2.0 * r, 2.0 * r, col);
            if Some(b.id) == selected {
                draw_circle_lines(p.x, p.y, r + 4.0, 2.0, WHITE);
            }
            continue;
        }
        draw_circle(p.x, p.y, r, col);
        if crowded {
            if Some(b.id) == selected {
                draw_circle_lines(p.x, p.y, r + 4.0, 2.0, WHITE);
            }
            continue;
        }
        if b.flame {
            draw_circle_lines(p.x, p.y, r + 1.0, 1.2, Color::from_rgba(255, 150, 40, 230));
        } else {
            draw_circle_lines(p.x, p.y, r, 0.8, Color::from_rgba(20, 20, 20, 200));
        }
        if Some(b.id) == selected {
            draw_circle_lines(p.x, p.y, r + 4.0, 2.0, WHITE);
        }
    }
}

/// Band under a screen point (largest population wins), within a few pixels.
pub fn pick_band(w: &World, v: &View, p: Vec2) -> Option<u32> {
    let mut best: Option<(f32, u32)> = None;
    for b in &w.bands {
        let s = v.to_screen(b.x as f32 + 0.5, b.y as f32 + 0.5);
        let r = band_radius(b.pop, v.zoom) + 4.0;
        let d = s.distance(p);
        if d <= r && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, b.id));
        }
    }
    best.map(|x| x.1)
}

/// On-screen radius of a band's disc: area ~ people, at least 1.5 px (also when zoomed far out).
pub fn band_radius(pop: u32, zoom: f32) -> f32 {
    ((pop as f32).sqrt() * 0.09 * zoom).clamp(1.5, (3.0 * zoom).max(1.5))
}
