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
        [
            Ocean, Ice, Tundra, Taiga, Steppe, Grassland, TempForest, Desert, Savanna, TropForest,
            Mountain,
        ][(x as usize).min(NBIOME - 1)]
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
/// bog / laterite / ironstone iron ore (map generator 4)
pub const D_IRON: u8 = 64;
pub const DEPOSITS: [(u8, Mat); 7] = [
    (D_STONE, Mat::Stone),
    (D_FLINT, Mat::Flint),
    (D_OBSIDIAN, Mat::Obsidian),
    (D_CLAY, Mat::Clay),
    (D_COPPER, Mat::CopperOre),
    (D_TIN, Mat::TinOre),
    (D_IRON, Mat::IronOre),
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
    /// small game (birds, hares, rodents): fast-breeding, mostly caught with snares and traps
    pub k_small: Vec<f32>,
    /// latitude of the top and bottom rows (degrees, negative = southern hemisphere)
    #[serde(default = "lat_n_default")]
    pub lat_n: f32,
    #[serde(default = "lat_s_default")]
    pub lat_s: f32,
    /// generator version: 1 = stage-1 maps (one hemisphere, no offshore fish, no small game)
    #[serde(default = "gen_v1")]
    pub gen_version: u32,
    /// landmass id per cell (0 = sea; 1 = largest landmass, 2 = next...). Derived, not saved.
    #[serde(skip)]
    pub landmass: Vec<u16>,
    /// cells per landmass (index = id; [0] unused)
    #[serde(skip)]
    pub landmass_size: Vec<u32>,
}
fn lat_n_default() -> f32 {
    65.0
}
fn lat_s_default() -> f32 {
    5.0
}
fn gen_v1() -> u32 {
    1
}
pub const GEN_VERSION: u32 = 4; // 3: placer tin in streams below tin lodes; 4: iron ore

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
    /// Can people start at (x, y)? Sea, ice and off-map cells: no. Cold, dry or foodless land:
    /// yes, with a warning.
    pub fn start_check(&self, x: usize, y: usize) -> crate::params::StartCheck {
        use crate::params::StartCheck;
        if x >= self.w || y >= self.h {
            return StartCheck::Uninhabitable;
        }
        let i = y * self.w + x;
        if !self.is_land(i) || matches!(self.biome_at(i), Biome::Ice) {
            return StartCheck::Uninhabitable;
        }
        let food = self.k_plants[i] + self.k_game[i] + 0.5 * self.k_fish[i];
        if self.temp[i] <= 3.0 || self.temp[i] >= 30.0 || food <= 38.0 {
            return StartCheck::Poor;
        }
        StartCheck::Ok
    }

    pub fn biome_at(&self, i: usize) -> Biome {
        Biome::from_u8(self.biome[i])
    }
    /// latitude in degrees for row y (north at the top)
    pub fn lat(&self, y: usize) -> f32 {
        self.lat_n + (self.lat_s - self.lat_n) * y as f32 / (self.h.max(2) - 1) as f32
    }
    /// The local season at cell i when the world (northern) season is s: seasons are reversed
    /// south of the equator. 0 = spring, 1 = summer, 2 = autumn, 3 = winter.
    #[inline]
    pub fn local_season(&self, i: usize, s: u32) -> u32 {
        if self.lat(i / self.w.max(1)) < 0.0 {
            (s + 2) % 4
        } else {
            s % 4
        }
    }
    /// temperature in world season s at cell i (local seasons, so hemispheres are opposite)
    pub fn temp_season(&self, i: usize, s: u32) -> f32 {
        let f = [0.0, 1.0, 0.0, -1.0][self.local_season(i, s) as usize];
        self.temp[i] + self.tamp[i] * f
    }
    /// coldest-season temperature
    pub fn temp_winter(&self, i: usize) -> f32 {
        self.temp[i] - self.tamp[i]
    }
    pub fn landmass_of(&self, i: usize) -> u16 {
        self.landmass.get(i).copied().unwrap_or(0)
    }
    /// Label connected landmasses (8-connected land), largest first. Cheap; run at generation and
    /// after loading.
    pub fn label_landmasses(&mut self) {
        let (w, h) = (self.w, self.h);
        let n = w * h;
        let mut lab = vec![0u32; n];
        let mut sizes: Vec<u32> = vec![0];
        let mut stack = Vec::new();
        for s0 in 0..n {
            if !self.is_land(s0) || lab[s0] != 0 {
                continue;
            }
            let id = sizes.len() as u32;
            let mut cnt = 0;
            lab[s0] = id;
            stack.push(s0);
            while let Some(i) = stack.pop() {
                cnt += 1;
                for (nx, ny) in nbrs8(i % w, i / w, w, h) {
                    let j = ny * w + nx;
                    if self.is_land(j) && lab[j] == 0 {
                        lab[j] = id;
                        stack.push(j);
                    }
                }
            }
            sizes.push(cnt);
        }
        let mut order: Vec<u32> = (1..sizes.len() as u32).collect();
        order.sort_by_key(|&k| std::cmp::Reverse(sizes[k as usize]));
        let mut remap = vec![0u16; sizes.len()];
        for (rank, &k) in order.iter().enumerate() {
            remap[k as usize] = (rank + 1).min(u16::MAX as usize) as u16;
        }
        self.landmass = lab.iter().map(|&l| remap[l as usize]).collect();
        self.landmass_size = vec![0; order.len() + 1];
        for (rank, &k) in order.iter().enumerate() {
            self.landmass_size[rank + 1] = sizes[k as usize];
        }
    }
    /// Landmasses big enough to call continents (cells).
    pub const CONTINENT_CELLS: u32 = 400;
    /// Derived layers that stage-1 (gen 1) maps lack: small-game capacity and offshore fish on
    /// the continental shelf. Deterministic from the saved map, so old saves gain them on load.
    pub fn upgrade_legacy(&mut self) {
        let n = self.w * self.h;
        if self.k_small.len() != n {
            self.k_small = (0..n)
                .map(|i| {
                    if self.is_land(i) {
                        small_capacity(self.biome_at(i), self.rain[i])
                    } else {
                        0.0
                    }
                })
                .collect();
        }
        if self.gen_version < 2 {
            self.add_shelf_fish();
            self.gen_version = 2;
        }
        if self.gen_version < 4 {
            self.add_iron();
            self.gen_version = 4;
        }
    }
    /// Iron ore is common but useless until a fire is hot enough: bog ore in cool wet lowlands,
    /// lateritic crusts in the wet-dry tropics, ironstone in sedimentary uplands. Deterministic
    /// from the map, so older maps gain it on load without changing anything else.
    fn add_iron(&mut self) {
        let (w, h) = (self.w, self.h);
        let sc = 1.0 / 160.0;
        let seed = self.seed;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !self.is_land(i) || self.deposits.len() <= i {
                    continue;
                }
                let (fx, fy) = (x as f32 * sc, y as f32 * sc);
                let nz = fbm(seed ^ 0x1e0e, fx * 13.0, fy * 13.0, 3);
                let hgt = self.height[i];
                let b = self.biome_at(i);
                let bog = self.rain[i] > 0.7
                    && hgt < 350.0
                    && matches!(b, Biome::Taiga | Biome::TempForest | Biome::Tundra)
                    && nz > 0.05;
                let laterite = matches!(b, Biome::Savanna | Biome::TropForest) && nz > 0.25;
                let ironstone = hgt > 250.0 && hgt < 2000.0 && nz > 0.45;
                if bog || laterite || ironstone {
                    self.deposits[i] |= D_IRON;
                }
            }
        }
    }
    /// Fish on the shallow sea near coasts (reachable only by boat).
    fn add_shelf_fish(&mut self) {
        let (w, h) = (self.w, self.h);
        let n = w * h;
        let mut dl = vec![u32::MAX; n];
        let mut q = std::collections::VecDeque::new();
        for i in 0..n {
            if self.is_land(i) {
                dl[i] = 0;
                q.push_back(i);
            }
        }
        while let Some(i) = q.pop_front() {
            if dl[i] >= 6 {
                continue;
            }
            for (nx, ny) in nbrs4(i % w, i / w, w, h) {
                let j = ny * w + nx;
                if dl[j] == u32::MAX {
                    dl[j] = dl[i] + 1;
                    q.push_back(j);
                }
            }
        }
        for i in 0..n {
            if self.is_land(i) || dl[i] == u32::MAX {
                continue;
            }
            // shelf seas are rich, open ocean poor; cold upwelling seas richer still
            let d = dl[i] as f32;
            let shelf = (1.0 - (d - 1.0) / 5.0).clamp(0.0, 1.0);
            let cold = if self.temp[i] < 12.0 { 1.25 } else { 1.0 };
            let ice = if self.temp[i] < -4.0 { 0.3 } else { 1.0 };
            self.k_fish[i] = 40.0 * shelf * cold * ice;
        }
    }
    pub fn has(&self, i: usize, d: u8) -> bool {
        self.deposits[i] & d != 0
    }

    pub fn generate(w: usize, h: usize, seed: u64) -> Map {
        let n = w * h;
        let mut rng = Rng::new(seed ^ 0x6d61_7067_656e);
        // noise frequencies are per cell (not per map), so bigger maps have more features, not bigger ones
        let sc = 1.0 / 160.0;
        // ---- landmasses. Each continent is a randomly sized, stretched and rotated blob whose
        // outline is roughened by domain-warped noise; centres are spread apart so oceans
        // separate them. Island arcs (chains of small blobs along a curve) add stepping stones.
        let area = (w * h) as f32;
        let land_frac = rng.range(0.27, 0.40);
        let nc = 2 + rng.below(5); // 2..6 continents
                                   // random sizes (a few big, some small), normalised to the land budget
        let mut sizes: Vec<f32> = (0..nc).map(|_| rng.range(0.15, 1.0).powf(1.6)).collect();
        let tot: f32 = sizes.iter().sum();
        for s in &mut sizes {
            *s = *s / tot * land_frac * area * 1.15;
        }
        sizes.sort_by(|a, b| b.partial_cmp(a).unwrap());
        struct Blob {
            x: f32,
            y: f32,
            r: f32,
            ax: f32,
            rot: f32,
            amp: f32,
        }
        let mut blobs: Vec<Blob> = Vec::new();
        for &sz in &sizes {
            let r = (sz / std::f32::consts::PI).sqrt();
            let ax = rng.range(1.0, 2.4);
            let rot = rng.range(0.0, std::f32::consts::PI);
            let mut best = (0.5 * w as f32, 0.5 * h as f32, f32::MIN);
            for _ in 0..60 {
                let mx = (r * 0.6 + 4.0).min(0.45 * w as f32);
                let my = (r * 0.6 + 4.0).min(0.45 * h as f32);
                let cx = rng.range(mx, w as f32 - mx);
                let cy = rng.range(my, h as f32 - my);
                // clearance to the nearest existing continent, relative to the combined radii
                let mut clear = f32::MAX;
                for bl in &blobs {
                    let d = ((cx - bl.x).powi(2) + (cy - bl.y).powi(2)).sqrt();
                    clear = clear.min(d - 1.05 * (r * ax.sqrt() + bl.r * bl.ax.sqrt()));
                }
                if clear > best.2 {
                    best = (cx, cy, clear);
                }
                if clear > 6.0 {
                    break;
                }
            }
            blobs.push(Blob {
                x: best.0,
                y: best.1,
                r,
                ax,
                rot,
                amp: 1.0,
            });
        }
        // island arcs
        let narcs = rng.below(4);
        for _ in 0..narcs {
            let (mut x, mut y) = (
                rng.range(0.1, 0.9) * w as f32,
                rng.range(0.1, 0.9) * h as f32,
            );
            let mut dir = rng.range(0.0, std::f32::consts::TAU);
            let k = 4 + rng.below(9);
            let step = rng.range(4.0, 8.0);
            let turn = rng.range(-0.25, 0.25);
            for _ in 0..k {
                let r = rng.range(1.5, 4.5);
                blobs.push(Blob {
                    x,
                    y,
                    r,
                    ax: rng.range(1.0, 1.8),
                    rot: rng.range(0.0, 3.14),
                    amp: 0.85,
                });
                dir += turn + rng.range(-0.2, 0.2);
                x += dir.cos() * step;
                y += dir.sin() * step;
            }
        }
        let mut hraw = vec![0f32; n];
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f32 * sc, y as f32 * sc);
                // domain warp roughens coasts into peninsulas, bays and inlets
                let wx = x as f32 + 14.0 * fbm(seed ^ 0x3a, fx * 6.0, fy * 6.0, 4);
                let wy = y as f32 + 14.0 * fbm(seed ^ 0x3b, fx * 6.0, fy * 6.0, 4);
                let mut cont = -1.0f32;
                for bl in &blobs {
                    let (dx, dy) = (wx - bl.x, wy - bl.y);
                    let (c, s) = (bl.rot.cos(), bl.rot.sin());
                    let u = (dx * c + dy * s) / (bl.r * bl.ax.sqrt());
                    let v = (-dx * s + dy * c) * bl.ax.sqrt() / bl.r;
                    let d = (u * u + v * v).sqrt();
                    cont = cont.max(bl.amp * (1.0 - d));
                }
                let edge = {
                    let ex = (x as f32 / w as f32 - 0.5).abs() * 2.0;
                    let ey = (y as f32 / h as f32 - 0.5).abs() * 2.0;
                    (ex.max(ey) - 0.85).max(0.0) * 6.0
                };
                let base = fbm(seed, fx * 7.0, fy * 7.0, 6);
                let ridge = 1.0 - fbm(seed ^ 0xabc, fx * 5.0, fy * 5.0, 5).abs() * 2.2;
                let ridge = ridge.max(0.0).powi(3);
                let inland = cont.clamp(0.0, 1.0);
                hraw[y * w + x] = cont + 0.32 * base + 0.5 * ridge * inland.sqrt() - edge;
            }
        }
        let mut sorted = hraw.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let sea = sorted[((n as f32) * (1.0 - land_frac)) as usize];
        let hmax = sorted[n - 1];
        let height: Vec<f32> = hraw
            .iter()
            .map(|&v| {
                if v >= sea {
                    ((v - sea) / (hmax - sea).max(1e-3)).powf(1.3) * 4200.0
                } else {
                    (v - sea) * 3000.0 - 1.0
                }
            })
            .collect();
        // climate: latitude span and a global warm/cold offset vary by seed
        let lat_n = rng.range(48.0, 72.0);
        let lat_s = rng.range(-58.0, -8.0);
        let climate = rng.range(-2.5, 2.5);
        let mut m = Map {
            w,
            h,
            seed,
            height,
            lat_n,
            lat_s,
            gen_version: GEN_VERSION,
            ..Default::default()
        };
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
                let t = lat_temp(lat.abs()) + climate - 0.0065 * alt
                    + 1.5 * fbm(seed ^ 0x7e, x as f32 * sc * 6.0, y as f32 * sc * 6.0, 3);
                let cont = (dsea[i] as f32 / 12.0).min(1.0);
                m.temp[i] = t;
                m.tamp[i] = (1.5 + 0.22 * lat.abs()) * (0.6 + 0.6 * cont);
            }
        }
        // ---- rainfall: westerly moisture sweep with orographic rain and rain shadow
        m.rain = vec![0.0; n];
        for y in 0..h {
            let lat = m.lat(y);
            let latf = lat_rain(lat.abs());
            // trade winds (below 30 deg) blow from the east; westerlies above
            let east_wind = lat.abs() < 30.0;
            let mut moist = 1.0f32;
            for k in 0..w {
                let x = if east_wind { w - 1 - k } else { k };
                let i = y * w + x;
                if !m.is_land(i) {
                    moist = (moist + 0.25).min(1.0);
                    m.rain[i] = moist * latf;
                    continue;
                }
                let prev = if east_wind {
                    if x + 1 < w {
                        x + 1
                    } else {
                        x
                    }
                } else {
                    x.saturating_sub(1)
                };
                let up = (m.height[i] - m.height[y * w + prev].max(0.0)).max(0.0) / 400.0;
                let r = moist * (0.10 + 0.5 * up.min(1.5));
                m.rain[i] = (moist * 0.85 + r) * latf;
                // plants return much of the rain to the air (moisture recycling), so interiors
                // dry out over ~25 cells rather than a handful
                moist = (moist - r * 0.3 - 0.003).max(0.15);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let nz = fbm(seed ^ 0x4a1, x as f32 * sc * 7.0, y as f32 * sc * 7.0, 4);
                // storms and monsoons also bring rain from the nearest sea, whatever the wind
                let near_sea = (-(dsea[i] as f32) / 14.0).exp() * lat_rain(m.lat(y).abs());
                let r = 0.65 * m.rain[i] + 0.4 * near_sea;
                m.rain[i] = (r + 0.25 * nz).clamp(0.0, 1.0);
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
        // where each cell drains to (used again for placer deposits below)
        let mut recv = vec![usize::MAX; n];
        for &i in &order {
            let (x, y) = (i % w, i / w);
            let mut best = i;
            for (nx, ny) in nbrs8(x, y, w, h) {
                let j = ny * w + nx;
                if filled[j] < filled[best] {
                    best = j;
                }
            }
            if best != i {
                recv[i] = best;
            }
            if best != i && m.is_land(best) {
                let f = m.flow[i];
                m.flow[best] += f;
            }
        }
        let mut lf: Vec<f32> = order.iter().map(|&i| m.flow[i]).collect();
        lf.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let thr = if lf.is_empty() {
            1.0
        } else {
            lf[(lf.len() as f32 * 0.93) as usize].max(2.0)
        };
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
                    if r > 0.3 {
                        Biome::Taiga
                    } else {
                        Biome::Tundra
                    }
                } else if t < 17.0 {
                    if r < 0.17 {
                        Biome::Desert
                    } else if r < 0.28 {
                        Biome::Steppe
                    } else if r < 0.42 {
                        Biome::Grassland
                    } else {
                        Biome::TempForest
                    }
                } else if r < 0.16 {
                    Biome::Desert
                } else if r < 0.46 {
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
        let nvolc = 1 + rng.below(3) + (area / 16000.0) as usize;
        for _ in 0..nvolc {
            if !mount.is_empty() {
                volc.push(mount[rng.below(mount.len())]);
            }
        }
        // island arcs are volcanic
        for bl in blobs.iter().filter(|b| b.amp < 1.0) {
            let i = (bl.y.clamp(0.0, h as f32 - 1.0) as usize) * w
                + bl.x.clamp(0.0, w as f32 - 1.0) as usize;
            if m.is_land(i) && rng.chance(0.3) {
                volc.push(i);
            }
        }
        let land_cells = (0..n).filter(|&i| m.is_land(i)).count();
        let tin_target = (land_cells / 550).max(8);
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
                    let near_river =
                        m.river[i] || nbrs8(x, y, w, h).any(|(a, b)| m.river[b * w + a]);
                    if (near_river
                        && hgt < 1500.0
                        && fbm(seed ^ 0xc1, fx * 15.0, fy * 15.0, 2) > -0.2)
                        || wet_low
                    {
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
            if tin >= tin_target || tin_thr < 0.4 {
                break;
            }
            tin_thr -= 0.04;
        }
        let _ = cu_cells;
        // placer tin: cassiterite is heavy and nearly indestructible, so streams draining a tin
        // lode carry it down and drop it in their gravels where the current slackens. Most early
        // tin was panned from such gravels, far from the lode itself and often where people
        // already lived by the river.
        let lodes: Vec<usize> = (0..n).filter(|&i| m.deposits[i] & D_TIN != 0).collect();
        for &l in &lodes {
            let mut c = l;
            for step in 0..30 {
                let nx = recv[c];
                if nx == usize::MAX || !m.is_land(nx) {
                    break;
                }
                c = nx;
                if step >= 1
                    && m.river[c]
                    && m.height[c] < m.height[l]
                    && hash3(seed ^ 0x7a, (c % w) as i64, (c / w) as i64) % 2 == 0
                {
                    m.deposits[c] |= D_TIN;
                }
            }
        }
        m.add_iron();
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
                let nz =
                    1.0 + 0.3 * fbm(seed ^ 0x99, x as f32 * sc * 14.0, y as f32 * sc * 14.0, 3);
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
        m.k_small = (0..n)
            .map(|i| {
                if m.is_land(i) {
                    small_capacity(m.biome_at(i), m.rain[i])
                } else {
                    0.0
                }
            })
            .collect();
        m.add_shelf_fish();
        m.label_landmasses();
        m
    }
}

/// Annual mean sea-level temperature by |latitude| (roughly Earth's zonal means).
fn lat_temp(lat: f32) -> f32 {
    let pts = [
        (0.0, 27.0),
        (15.0, 26.0),
        (30.0, 20.0),
        (45.0, 11.5),
        (60.0, 1.5),
        (75.0, -11.0),
        (90.0, -24.0),
    ];
    for k in 0..pts.len() - 1 {
        let (a, va) = pts[k];
        let (b, vb) = pts[k + 1];
        if lat <= b {
            let t = ((lat - a) / (b - a)).clamp(0.0, 1.0);
            return va + (vb - va) * t;
        }
    }
    -24.0
}
/// Small-game capacity by biome, scaled by rainfall.
pub fn small_capacity(b: Biome, rain: f32) -> f32 {
    use Biome::*;
    let base = match b {
        Ocean | Ice => 0.0,
        Tundra => 10.0,
        Taiga => 18.0,
        Steppe => 22.0,
        Grassland => 26.0,
        TempForest => 26.0,
        Desert => 5.0,
        Savanna => 28.0,
        TropForest => 30.0,
        Mountain => 8.0,
    };
    base * (0.7 + 0.6 * rain.clamp(0.0, 1.0))
}
fn lat_rain(lat: f32) -> f32 {
    // equatorial wet belt, subtropical dry belt (~25-32 deg), mid-latitude wet, polar dry
    let pts = [
        (0.0, 1.0),
        (12.0, 0.9),
        (24.0, 0.45),
        (32.0, 0.5),
        (45.0, 0.95),
        (58.0, 0.85),
        (70.0, 0.5),
    ];
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
        (nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h)
            .then_some((nx as usize, ny as usize))
    })
}
pub fn nbrs8(x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = (usize, usize)> {
    const D: [(i32, i32); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ];
    D.iter().filter_map(move |&(dx, dy)| {
        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
        (nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h)
            .then_some((nx as usize, ny as usize))
    })
}
