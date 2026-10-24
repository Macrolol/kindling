//! World map generation: fBm heightmap shaped into continents, temperature from latitude and
//! altitude with seasonal swing, rainfall carried by westerly winds with rain shadows, rivers from
//! flow accumulation over a depression-filled surface, Whittaker-style biomes, and mineral deposits
//! placed by "geology" noise fields (flint in sediments, copper/tin in uplands, obsidian near
//! volcanic hotspots, clay along rivers).
use crate::materials::Mat;
use crate::rng::{hash3, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Biome {
    Ocean = 0,
    Ice,
    Tundra,
    Taiga,
    Steppe,
    Grassland,
    TempForest,
    Desert,
    Savanna,
    TropForest,
    Mountain,
}
pub const NBIOME: usize = 11;
impl Biome {
    pub fn from_u8(x: u8) -> Biome {
        use Biome::*;
        [Ocean, Ice, Tundra, Taiga, Steppe, Grassland, TempForest, Desert, Savanna, TropForest, Mountain]
            [(x as usize).min(NBIOME - 1)]
    }
    pub fn name(self) -> &'static str {
        use Biome::*;
        match self {
            Ocean => "ocean",
            Ice => "ice",
            Tundra => "tundra",
            Taiga => "taiga",
            Steppe => "steppe",
            Grassland => "grassland",
            TempForest => "temperate forest",
            Desert => "desert",
            Savanna => "savanna",
            TropForest => "tropical forest",
            Mountain => "mountain",
        }
    }
    /// (plants, game, wood, fibre) capacity per cell, before rainfall / noise modulation.
    pub fn capacity(self) -> [f32; 4] {
        use Biome::*;
        match self {
            Ocean => [0.0, 0.0, 0.0, 0.0],
            Ice => [0.0, 2.0, 0.0, 0.0],
            Tundra => [12.0, 45.0, 1.0, 2.0],
            Taiga => [25.0, 50.0, 30.0, 4.0],
            Steppe => [30.0, 60.0, 2.0, 10.0],
            Grassland => [50.0, 70.0, 6.0, 14.0],
            TempForest => [65.0, 45.0, 30.0, 10.0],
            Desert => [8.0, 8.0, 0.5, 2.0],
            Savanna => [65.0, 85.0, 8.0, 14.0],
            TropForest => [90.0, 35.0, 30.0, 14.0],
            Mountain => [12.0, 25.0, 8.0, 2.0],
        }
    }
}

/// Deposit bits.
pub const D_STONE: u8 = 1;
pub const D_FLINT: u8 = 2;
pub const D_OBSIDIAN: u8 = 4;
pub const D_CLAY: u8 = 8;
pub const D_COPPER: u8 = 16;
pub const D_TIN: u8 = 32;
pub const DEPOSITS: [(u8, Mat); 6] = [
    (D_STONE, Mat::Stone),
    (D_FLINT, Mat::Flint),
    (D_OBSIDIAN, Mat::Obsidian),
    (D_CLAY, Mat::Clay),
    (D_COPPER, Mat::CopperOre),
    (D_TIN, Mat::TinOre),
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Map {
    pub w: usize,
    pub h: usize,
    pub seed: u64,
    /// metres; < 0 = sea
    pub height: Vec<f32>,
    /// annual mean temperature, deg C
    pub temp: Vec<f32>,
    /// seasonal half-amplitude, deg C
    pub tamp: Vec<f32>,
    /// 0..1
    pub rain: Vec<f32>,
    /// accumulated river flow (rain units)
    pub flow: Vec<f32>,
    pub river: Vec<bool>,
    pub biome: Vec<u8>,
    pub deposits: Vec<u8>,
    /// renewable capacities
    pub k_plants: Vec<f32>,
    pub k_game: Vec<f32>,
    pub k_fish: Vec<f32>,
    pub k_wood: Vec<f32>,
    pub k_fibre: Vec<f32>,
}

// ---------------------------------------------------------------- noise
fn lattice(seed: u64, x: i64, y: i64) -> f32 {
    (hash3(seed, x, y) >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
}
fn smooth(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
pub fn vnoise(seed: u64, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - x0), smooth(y - y0));
    let (xi, yi) = (x0 as i64, y0 as i64);
    let a = lattice(seed, xi, yi);
    let b = lattice(seed, xi + 1, yi);
    let c = lattice(seed, xi, yi + 1);
    let d = lattice(seed, xi + 1, yi + 1);
    let ab = a + (b - a) * fx;
    let cd = c + (d - c) * fx;
    ab + (cd - ab) * fy
}
pub fn fbm(seed: u64, x: f32, y: f32, oct: u32) -> f32 {
    let (mut s, mut amp, mut f, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..oct {
        s += amp * vnoise(seed.wrapping_add(o as u64 * 7919), x * f, y * f);
        norm += amp;
        amp *= 0.5;
        f *= 2.03;
    }
    s / norm
}

impl Map {
    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }
    #[inline]
    pub fn is_land(&self, i: usize) -> bool {
        self.height[i] >= 0.0
    }
    pub fn biome_at(&self, i: usize) -> Biome {
        Biome::from_u8(self.biome[i])
    }
    /// latitude in degrees for row y (north at the top)
    pub fn lat(&self, y: usize) -> f32 {
        65.0 - 60.0 * y as f32 / (self.h.max(2) - 1) as f32
    }
    /// temperature in season s (0 = spring, 1 = summer, 2 = autumn, 3 = winter)
    pub fn temp_season(&self, i: usize, s: u32) -> f32 {
        let f = [0.0, 1.0, 0.0, -1.0][(s % 4) as usize];
        self.temp[i] + self.tamp[i] * f
    }
    pub fn has(&self, i: usize, d: u8) -> bool {
        self.deposits[i] & d != 0
    }

    pub fn generate(w: usize, h: usize, seed: u64) -> Map {
        let n = w * h;
        let mut rng = Rng::new(seed ^ 0x6d61_7067_656e);
        let sc = 1.0 / w.max(h) as f32;
        // ---- continents: a few gaussian blobs + fBm, falling off at the map border
        let nb = 2 + rng.below(3);
        let blobs: Vec<(f32, f32, f32)> = (0..nb)
            .map(|_| (rng.range(0.2, 0.8) * w as f32, rng.range(0.2, 0.8) * h as f32, rng.range(0.18, 0.32) * w as f32))
            .collect();
        let mut hraw = vec![0f32; n];
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f32 * sc, y as f32 * sc);
                let mut blob = 0f32;
                for &(bx, by, r) in &blobs {
                    let d2 = ((x as f32 - bx).powi(2) + (y as f32 - by).powi(2)) / (r * r);
                    blob = blob.max((-d2).exp());
                }
                let edge = {
                    let ex = (x as f32 / w as f32 - 0.5).abs() * 2.0;
                    let ey = (y as f32 / h as f32 - 0.5).abs() * 2.0;
                    (ex.max(ey) - 0.75).max(0.0) * 2.5
                };
                let base = fbm(seed, fx * 5.0, fy * 5.0, 6);
                let ridge = 1.0 - fbm(seed ^ 0xabc, fx * 4.0, fy * 4.0, 5).abs() * 2.2;
                let ridge = ridge.max(0.0).powi(3);
                hraw[y * w + x] = 0.55 * base + 0.65 * blob + 0.35 * ridge * blob - edge;
            }
        }
        let mut sorted = hraw.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let sea = sorted[(n as f32 * 0.58) as usize];
        let hmax = sorted[n - 1];
        let height: Vec<f32> = hraw
            .iter()
            .map(|&v| if v >= sea { (v - sea) / (hmax - sea).max(1e-3) * 3800.0 } else { (v - sea) * 3000.0 - 1.0 })
            .collect();
        let mut m = Map { w, h, seed, height, ..Default::default() };
        // ---- distance to sea (for continentality), BFS
        let mut dsea = vec![u32::MAX; n];
        let mut q = std::collections::VecDeque::new();
        for i in 0..n {
            if !m.is_land(i) {
                dsea[i] = 0;
                q.push_back(i);
            }
        }
        while let Some(i) = q.pop_front() {
            let (x, y) = (i % w, i / w);
            for (nx, ny) in nbrs4(x, y, w, h) {
                let j = ny * w + nx;
                if dsea[j] == u32::MAX {
                    dsea[j] = dsea[i] + 1;
                    q.push_back(j);
                }
            }
        }
        // ---- temperature
        m.temp = vec![0.0; n];
        m.tamp = vec![0.0; n];
        for y in 0..h {
            let lat = m.lat(y);
            for x in 0..w {
                let i = y * w + x;
                let alt = m.height[i].max(0.0);
                let t = 28.0 - 0.6 * (lat - 8.0).max(0.0) - 0.0065 * alt + 1.5 * fbm(seed ^ 0x7e, x as f32 * sc * 6.0, y as f32 * sc * 6.0, 3);
                let cont = (dsea[i] as f32 / 12.0).min(1.0);
                m.temp[i] = t;
                m.tamp[i] = (1.5 + 0.22 * lat) * (0.6 + 0.6 * cont);
            }
        }
        // ---- rainfall: westerly moisture sweep with orographic rain and rain shadow
        m.rain = vec![0.0; n];
        for y in 0..h {
            let lat = m.lat(y);
            let latf = lat_rain(lat);
            // trade winds (below 30 deg) blow from the east; westerlies above
            let east_wind = lat < 30.0;
            let mut moist = 1.0f32;
            for k in 0..w {
                let x = if east_wind { w - 1 - k } else { k };
                let i = y * w + x;
                if !m.is_land(i) {
                    moist = (moist + 0.25).min(1.0);
                    m.rain[i] = moist * latf;
                    continue;
                }
                let prev = if east_wind { if x + 1 < w { x + 1 } else { x } } else { x.saturating_sub(1) };
                let up = (m.height[i] - m.height[y * w + prev].max(0.0)).max(0.0) / 400.0;
                let r = moist * (0.10 + 0.5 * up.min(1.5));
                m.rain[i] = (moist * 0.85 + r) * latf;
                moist = (moist - r * 0.7 - 0.012).max(0.08);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let nz = fbm(seed ^ 0x4a1, x as f32 * sc * 7.0, y as f32 * sc * 7.0, 4);
                m.rain[i] = (m.rain[i] + 0.25 * nz).clamp(0.0, 1.0);
            }
        }
        // ---- rivers: priority-flood depression filling, then steepest-descent accumulation
        let mut filled = m.height.clone();
        let mut done = vec![false; n];
        let mut pq: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        for i in 0..n {
            if !m.is_land(i) {
                done[i] = true;
                for (nx, ny) in nbrs8(i % w, i / w, w, h) {
                    let j = ny * w + nx;
                    if m.is_land(j) && !done[j] {
                        done[j] = true;
                        pq.push(Reverse(((filled[j] * 100.0) as i64, j)));
                    }
                }
            }
        }
        while let Some(Reverse((_, i))) = pq.pop() {
            for (nx, ny) in nbrs8(i % w, i / w, w, h) {
                let j = ny * w + nx;
                if !done[j] {
                    done[j] = true;
                    if filled[j] <= filled[i] {
                        filled[j] = filled[i] + 0.5;
                    }
                    pq.push(Reverse(((filled[j] * 100.0) as i64, j)));
                }
            }
        }
        let mut order: Vec<usize> = (0..n).filter(|&i| m.is_land(i)).collect();
        order.sort_by(|&a, &b| filled[b].partial_cmp(&filled[a]).unwrap());
        m.flow = m.rain.iter().map(|&r| r * 0.5).collect();
        for &i in &order {
            let (x, y) = (i % w, i / w);
            let mut best = i;
            for (nx, ny) in nbrs8(x, y, w, h) {
                let j = ny * w + nx;
                if filled[j] < filled[best] {
                    best = j;
                }
            }
            if best != i && m.is_land(best) {
                let f = m.flow[i];
                m.flow[best] += f;
            }
        }
        let mut lf: Vec<f32> = order.iter().map(|&i| m.flow[i]).collect();
        lf.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let thr = if lf.is_empty() { 1.0 } else { lf[(lf.len() as f32 * 0.93) as usize].max(2.0) };
        m.river = (0..n).map(|i| m.is_land(i) && m.flow[i] >= thr).collect();
        // ---- biomes
        m.biome = (0..n)
            .map(|i| {
                if !m.is_land(i) {
                    return Biome::Ocean as u8;
                }
                let (t, r) = (m.temp[i], m.rain[i]);
                let b = if m.height[i] > 2700.0 {
                    Biome::Mountain
                } else if t < -6.0 {
                    Biome::Ice
                } else if t < 0.0 {
                    Biome::Tundra
                } else if t < 6.0 {
                    if r > 0.3 { Biome::Taiga } else { Biome::Tundra }
                } else if t < 17.0 {
                    if r < 0.18 { Biome::Desert } else if r < 0.32 { Biome::Steppe } else if r < 0.48 { Biome::Grassland } else { Biome::TempForest }
                } else if r < 0.16 {
                    Biome::Desert
                } else if r < 0.5 {
                    Biome::Savanna
                } else {
                    Biome::TropForest
                };
                b as u8
            })
            .collect();
        // ---- deposits
        m.deposits = vec![0; n];
        let mut volc: Vec<usize> = Vec::new();
        let mut mount: Vec<usize> = (0..n).filter(|&i| m.height[i] > 1800.0).collect();
        if mount.is_empty() {
            mount = (0..n).filter(|&i| m.height[i] > 900.0).collect();
        }
        for _ in 0..(1 + rng.below(4)) {
            if !mount.is_empty() {
                volc.push(mount[rng.below(mount.len())]);
            }
        }
        let mut cu_cells = 0;
        let mut tin_thr = 0.72f32;
        loop {
            let mut tin = 0;
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    if !m.is_land(i) {
                        continue;
                    }
                    let (fx, fy) = (x as f32 * sc, y as f32 * sc);
                    let hgt = m.height[i];
                    let mut d = 0u8;
                    let rough = fbm(seed ^ 0x51, fx * 12.0, fy * 12.0, 3);
                    if hgt > 250.0 || m.river[i] || rough > -0.1 {
                        d |= D_STONE;
                    }
                    let sed = fbm(seed ^ 0xf1, fx * 9.0, fy * 9.0, 4);
                    if sed > 0.28 && hgt < 1400.0 {
                        d |= D_FLINT;
                    }
                    let wet_low = m.rain[i] > 0.6 && hgt < 400.0;
                    let near_river = m.river[i] || nbrs8(x, y, w, h).any(|(a, b)| m.river[b * w + a]);
                    if (near_river && hgt < 1500.0 && fbm(seed ^ 0xc1, fx * 15.0, fy * 15.0, 2) > -0.2) || wet_low {
                        d |= D_CLAY;
                    }
                    let cu = fbm(seed ^ 0xc0, fx * 10.0, fy * 10.0, 4);
                    if cu > 0.42 && hgt > 400.0 {
                        d |= D_COPPER;
                    }
                    let sn = fbm(seed ^ 0x50, fx * 11.0, fy * 11.0, 4);
                    if sn > tin_thr && hgt > 200.0 {
                        d |= D_TIN;
                        tin += 1;
                    }
                    for &v in &volc {
                        let (vx, vy) = ((v % w) as i64, (v / w) as i64);
                        let dd = (vx - x as i64).abs().max((vy - y as i64).abs());
                        if dd <= 3 && hash3(seed ^ 0x0b, x as i64, y as i64) % 3 == 0 {
                            d |= D_OBSIDIAN;
                        }
                    }
                    m.deposits[i] = d;
                }
            }
            cu_cells = cu_cells.max(0);
            if tin >= 12 || tin_thr < 0.4 {
                break;
            }
            tin_thr -= 0.04;
        }
        let _ = cu_cells;
        // ---- renewable capacities
        m.k_plants = vec![0.0; n];
        m.k_game = vec![0.0; n];
        m.k_fish = vec![0.0; n];
        m.k_wood = vec![0.0; n];
        m.k_fibre = vec![0.0; n];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !m.is_land(i) {
                    continue;
                }
                let cap = m.biome_at(i).capacity();
                let nz = 1.0 + 0.3 * fbm(seed ^ 0x99, x as f32 * sc * 14.0, y as f32 * sc * 14.0, 3);
                let wet = 0.6 + 0.8 * m.rain[i].min(1.0);
                m.k_plants[i] = cap[0] * nz * wet.min(1.3);
                m.k_game[i] = cap[1] * (2.0 - nz).max(0.5);
                m.k_wood[i] = cap[2] * nz;
                m.k_fibre[i] = cap[3] * nz;
                let coast = nbrs8(x, y, w, h).any(|(a, b)| !m.is_land(b * w + a));
                let mut fish = 0.0;
                if m.river[i] {
                    fish += 15.0 + 10.0 * (m.flow[i] / thr).ln().max(0.0);
                }
                if coast {
                    fish += 25.0;
                }
                if m.temp[i] < -4.0 {
                    fish *= 0.5;
                }
                m.k_fish[i] = fish.min(60.0);
                if m.river[i] {
                    m.k_plants[i] *= 1.25;
                }
            }
        }
        m
    }
}

fn lat_rain(lat: f32) -> f32 {
    // equatorial wet belt, subtropical dry belt (~25-32 deg), mid-latitude wet, polar dry
    let pts = [(0.0, 1.0), (12.0, 0.9), (24.0, 0.45), (32.0, 0.5), (45.0, 0.95), (58.0, 0.85), (70.0, 0.5)];
    for k in 0..pts.len() - 1 {
        let (a, va) = pts[k];
        let (b, vb) = pts[k + 1];
        if lat <= b {
            let t = ((lat - a) / (b - a)).clamp(0.0, 1.0);
            return va + (vb - va) * t;
        }
    }
    0.5
}

pub fn nbrs4(x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = (usize, usize)> {
    const D: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    D.iter().filter_map(move |&(dx, dy)| {
        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
        (nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h).then_some((nx as usize, ny as usize))
    })
}
pub fn nbrs8(x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = (usize, usize)> {
    const D: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)];
    D.iter().filter_map(move |&(dx, dy)| {
        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
        (nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h).then_some((nx as usize, ny as usize))
    })
}
