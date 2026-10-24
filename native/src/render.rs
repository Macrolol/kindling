//! Map rendering: a per-cell texture (terrain or an overlay) plus bands drawn as discs.
use civ_core::map::{Biome, D_CLAY, D_COPPER, D_FLINT, D_OBSIDIAN, D_STONE, D_TIN};
use civ_core::materials::Item;
use civ_core::World;
use macroquad::prelude::*;

pub const OVERLAYS: [&str; 6] = ["terrain", "know-how", "item spread", "population", "deposits", "temperature"];

/// Camera: centre in cell coordinates, zoom in screen pixels per cell.
pub struct View {
    pub cx: f32,
    pub cy: f32,
    pub zoom: f32,
}
impl View {
    pub fn fit(w: &World) -> View {
        let z = (screen_width() / w.map.w as f32).min((screen_height() - 30.0) / w.map.h as f32);
        View { cx: w.map.w as f32 / 2.0, cy: w.map.h as f32 / 2.0, zoom: z.max(1.0) }
    }
    pub fn to_screen(&self, x: f32, y: f32) -> Vec2 {
        vec2((x - self.cx) * self.zoom + screen_width() / 2.0, (y - self.cy) * self.zoom + screen_height() / 2.0)
    }
    pub fn to_world(&self, p: Vec2) -> Vec2 {
        vec2((p.x - screen_width() / 2.0) / self.zoom + self.cx, (p.y - screen_height() / 2.0) / self.zoom + self.cy)
    }
}

fn biome_rgb(b: Biome) -> [f32; 3] {
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

/// Builds the RGBA pixels of the map for an overlay.
pub fn map_pixels(w: &World, overlay: u8, item: Option<Item>) -> Vec<u8> {
    let m = &w.map;
    let n = m.w * m.h;
    let mut px = vec![0u8; n * 4];
    // per-cell aggregates from bands
    let mut pop = vec![0u32; n];
    let mut know = vec![0u32; n];
    for b in &w.bands {
        let c = b.cell(m.w);
        pop[c] += b.pop;
        // know-how is shown over each band's home range (2 cells), not just its own cell
        let (bx, by) = (b.x as i32, b.y as i32);
        for dy in -2..=2i32 {
            for dx in -2..=2i32 {
                let (x, y) = (bx + dx, by + dy);
                if x >= 0 && y >= 0 && (x as usize) < m.w && (y as usize) < m.h && dx * dx + dy * dy <= 5 {
                    let j = y as usize * m.w + x as usize;
                    know[j] = know[j].max(b.n_items());
                }
            }
        }
    }
    let max_know = know.iter().copied().max().unwrap_or(1).max(1) as f32;
    let cf = item.and_then(|it| w.cell_first.get(&it));
    let now = w.tick.max(1) as f32;
    for y in 0..m.h {
        for x in 0..m.w {
            let i = m.idx(x, y);
            let land = m.is_land(i);
            // hillshade from the height gradient (light from the north-west)
            let hgt = |xx: usize, yy: usize| m.height[m.idx(xx.min(m.w - 1), yy.min(m.h - 1))].max(0.0);
            let dz = hgt(x + 1, y + 1) - hgt(x.saturating_sub(1), y.saturating_sub(1));
            let shade = (1.0 - dz / 900.0).clamp(0.65, 1.25);
            let mut c = biome_rgb(m.biome_at(i));
            if land && m.river[i] {
                c = lerp3(c, [0.25, 0.45, 0.75], 0.75);
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
                2 if land => match cf.map(|v| v[i]) {
                    Some(t) if t != u32::MAX => ramp(t as f32 / now),
                    _ => grey(c, 0.55),
                },
                3 if land => {
                    if pop[i] > 0 {
                        ramp((pop[i] as f32).ln() / 6.0)
                    } else {
                        grey(c, 0.55)
                    }
                }
                4 if land => {
                    let d = m.deposits[i];
                    let mut g = grey(c, 0.45);
                    for (bit, col) in [
                        (D_STONE, [0.55, 0.55, 0.6]),
                        (D_CLAY, [0.75, 0.45, 0.3]),
                        (D_FLINT, [0.95, 0.95, 0.85]),
                        (D_OBSIDIAN, [0.15, 0.1, 0.25]),
                        (D_COPPER, [0.15, 0.85, 0.6]),
                        (D_TIN, [0.55, 0.75, 1.0]),
                    ] {
                        if d & bit != 0 {
                            g = col;
                        }
                    }
                    g
                }
                5 => ramp((m.temp[i] + 15.0) / 45.0),
                _ => c,
            };
            let o = (y * m.w + x) * 4;
            px[o] = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
            px[o + 1] = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
            px[o + 2] = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
            px[o + 3] = 255;
        }
    }
    px
}

pub struct MapTex {
    pub tex: Texture2D,
    pub key: (u32, u8, Option<Item>, usize),
}
impl MapTex {
    pub fn new(w: &World, overlay: u8, item: Option<Item>) -> MapTex {
        let px = map_pixels(w, overlay, item);
        let tex = Texture2D::from_rgba8(w.map.w as u16, w.map.h as u16, &px);
        tex.set_filter(FilterMode::Nearest);
        MapTex { tex, key: (w.tick, overlay, item, w.map.w * w.map.h) }
    }
    pub fn refresh(&mut self, w: &World, overlay: u8, item: Option<Item>) {
        let key = (w.tick, overlay, item, w.map.w * w.map.h);
        if key != self.key {
            *self = MapTex::new(w, overlay, item);
        }
    }
}

pub fn draw_world(w: &World, v: &View, mt: &MapTex, overlay: u8, item: Option<Item>, selected: Option<u32>) {
    clear_background(Color::from_rgba(12, 18, 30, 255));
    let p0 = v.to_screen(0.0, 0.0);
    draw_texture_ex(
        &mt.tex,
        p0.x,
        p0.y,
        WHITE,
        DrawTextureParams { dest_size: Some(vec2(w.map.w as f32 * v.zoom, w.map.h as f32 * v.zoom)), ..Default::default() },
    );
    // bands: disc area ~ population; colour = founding lineage; orange ring = keeps fire
    for b in &w.bands {
        let p = v.to_screen(b.x as f32 + 0.5, b.y as f32 + 0.5);
        if p.x < -20.0 || p.y < -20.0 || p.x > screen_width() + 20.0 || p.y > screen_height() + 20.0 {
            continue;
        }
        let r = ((b.pop as f32).sqrt() * 0.09 * v.zoom).clamp(1.5, 3.0 * v.zoom);
        let mut col = lineage_color(b.lineage);
        if let Some(it) = item {
            if overlay == 2 && !b.d.tk.producible.has(it) {
                col = Color::new(0.35, 0.35, 0.35, 0.8);
            }
        }
        draw_circle(p.x, p.y, r, col);
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
        let r = ((b.pop as f32).sqrt() * 0.09 * v.zoom).clamp(1.5, 3.0 * v.zoom) + 4.0;
        let d = s.distance(p);
        if d <= r && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, b.id));
        }
    }
    best.map(|x| x.1)
}
