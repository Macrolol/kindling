//! The world and its seasonal step.
//!
//! Each season: renewables regrow -> each band forages with what it can make (food, spare time,
//! stress) -> births / deaths -> practice / forgetting -> fire keeping -> experiments (invention) ->
//! moving -> contacts between nearby bands (teaching / copying, fire sharing) -> splits, merges and
//! deaths -> statistics.
use crate::band::{make_kit, Band};
use crate::knowledge::*;
use crate::map::*;
use crate::materials::*;
use crate::params::{Config, ParamChange, Params};
use crate::physics::{self, Action, Ctx, Inputs};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

pub const EVENT_CAP: usize = 40_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EventKind {
    /// first band in the world able to make an item
    FirstInvention = 0,
    /// a band found a recipe by its own experiment (not the first in the world)
    Invention,
    Learned,
    Forgot,
    /// no band in the world can make the item any more
    LostWorldwide,
    FireCaptured,
    FireLost,
    Split,
    Merged,
    Died,
    Founded,
    /// the world's first band to keep fire (captured from a wildfire)
    FirstFire,
}
impl EventKind {
    pub fn from_u8(x: u8) -> EventKind {
        use EventKind::*;
        [FirstInvention, Invention, Learned, Forgot, LostWorldwide, FireCaptured, FireLost, Split, Merged, Died, Founded, FirstFire]
            [(x as usize).min(11)]
    }
    pub fn major(self) -> bool {
        matches!(self, EventKind::FirstInvention | EventKind::LostWorldwide | EventKind::FirstFire)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Event {
    pub tick: u32,
    pub kind: u8,
    pub band: u32,
    pub other: u32,
    pub cell: u32,
    pub item: Item,
    pub recipe: u32,
}

/// First time any band could make an item.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct First {
    pub item: Item,
    pub tick: u32,
    pub band: u32,
    pub cell: u32,
    /// recipe index, or u32::MAX for a captured wildfire
    pub recipe: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct YearStats {
    pub year: u32,
    pub pop: u32,
    pub bands: u32,
    /// distinct items at least one band can make
    pub items_world: u32,
    pub items_mean: f32,
    pub items_max: u32,
    pub experiments: u32,
    pub inventions: u32,
    pub transfers: u32,
    pub forgets: u32,
    pub with_fire: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Counters {
    pub experiments: u64,
    pub inventions: u64,
    pub transfers: u64,
    pub forgets: u64,
    pub splits: u64,
    pub deaths: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct World {
    pub cfg: Config,
    pub params: Params,
    pub map: Map,
    pub tick: u32,
    pub rng_state: u64,
    pub next_id: u32,
    pub plants: Vec<f32>,
    pub game: Vec<f32>,
    pub fish: Vec<f32>,
    pub bands: Vec<Band>,
    pub reg: Registry,
    pub events: VecDeque<Event>,
    pub chronicle: Vec<Event>,
    pub firsts: Vec<First>,
    /// item -> per cell: first tick a band in that cell could make it (u32::MAX = never)
    #[serde(with = "crate::save::pairs")]
    pub cell_first: BTreeMap<Item, Vec<u32>>,
    pub history: Vec<YearStats>,
    pub settings_log: Vec<ParamChange>,
    pub counters: Counters,
    /// item -> number of bands able to make it (last year)
    #[serde(with = "crate::save::pairs")]
    pub item_bands: BTreeMap<Item, u32>,
    #[serde(skip)]
    pub occ: Vec<Vec<u32>>,
    /// people per cell (snapshot used for crowding)
    #[serde(skip)]
    pub occ_pop: Vec<u32>,
    #[serde(skip)]
    rng: Option<Rng>,
    #[serde(skip)]
    year_acc: YearStats,
}

fn raw(m: Mat) -> Item {
    item(Form::Raw, m)
}

impl World {
    pub fn new(cfg: Config, params: Params) -> World {
        let map = Map::generate(cfg.map_w, cfg.map_h, cfg.seed);
        let n = map.w * map.h;
        let mut w = World {
            plants: map.k_plants.clone(),
            game: map.k_game.clone(),
            fish: map.k_fish.clone(),
            map,
            cfg,
            params,
            rng_state: 0,
            next_id: 1,
            ..Default::default()
        };
        w.occ = vec![Vec::new(); n];
        let mut rng = Rng::new(w.cfg.seed);
        // founders: one region with mild climate and plenty of food
        let mut cands: Vec<usize> = (0..n)
            .filter(|&i| w.map.is_land(i) && w.map.temp[i] > 16.0 && w.map.temp[i] < 27.0)
            .filter(|&i| w.map.k_plants[i] + w.map.k_game[i] > 70.0)
            .collect();
        if cands.is_empty() {
            cands = (0..n).filter(|&i| w.map.is_land(i) && w.map.k_plants[i] > 5.0).collect();
        }
        let origin = cands[rng.below(cands.len())];
        let (ox, oy) = (origin % w.map.w, origin / w.map.w);
        for k in 0..w.cfg.founders {
            let mut pos = origin;
            for _ in 0..20 {
                let x = (ox as i32 + rng.below(7) as i32 - 3).clamp(0, w.map.w as i32 - 1) as usize;
                let y = (oy as i32 + rng.below(7) as i32 - 3).clamp(0, w.map.h as i32 - 1) as usize;
                let i = y * w.map.w + x;
                if w.map.is_land(i) {
                    pos = i;
                    break;
                }
            }
            let id = w.next_id;
            w.next_id += 1;
            let b = Band {
                id,
                lineage: id,
                parent: 0,
                gen: 0,
                x: (pos % w.map.w) as u16,
                y: (pos / w.map.w) as u16,
                pop: w.cfg.founder_pop + rng.below(8) as u32,
                store: 5.0,
                curiosity: (1.0 + 0.25 * rng.gauss()).clamp(0.4, 1.8),
                flame: false,
                stock: vec![0.0; NRAW],
                ..Default::default()
            };
            let _ = k;
            w.push_event(EventKind::Founded, id, 0, pos as u32, NONE, u32::MAX);
            w.bands.push(b);
        }
        w.rng_state = rng.0;
        w
    }

    pub fn year(&self) -> f32 {
        self.tick as f32 / 4.0
    }
    pub fn season(&self) -> u32 {
        self.tick % 4
    }
    pub fn total_pop(&self) -> u32 {
        self.bands.iter().map(|b| b.pop).sum()
    }
    pub fn band_index(&self, id: u32) -> Option<usize> {
        self.bands.iter().position(|b| b.id == id)
    }

    fn push_event(&mut self, kind: EventKind, band: u32, other: u32, cell: u32, it: Item, recipe: u32) {
        let e = Event { tick: self.tick, kind: kind as u8, band, other, cell, item: it, recipe };
        if kind.major() {
            self.chronicle.push(e);
        }
        if self.events.len() >= EVENT_CAP {
            self.events.pop_front();
        }
        self.events.push_back(e);
    }

    pub fn log_param_changes(&mut self, old: &Params) {
        let a: BTreeMap<String, f64> = old.fields().into_iter().collect();
        for (k, v) in self.params.fields() {
            if let Some(&o) = a.get(&k) {
                if (o - v).abs() > 1e-9 {
                    self.settings_log.push(ParamChange { tick: self.tick, name: k, old: o, new: v });
                }
            }
        }
    }

    /// Rebuild transient state after loading.
    pub fn after_load(&mut self) {
        self.reg.rebuild_index();
        let n = self.map.w * self.map.h;
        self.occ = vec![Vec::new(); n];
        for b in &mut self.bands {
            if b.stock.len() < NRAW {
                b.stock.resize(NRAW, 0.0);
            }
        }
        // derived toolkits so a freshly loaded (paused) world can be inspected
        let mut bands = std::mem::take(&mut self.bands);
        for b in &mut bands {
            self.refresh_toolkit(b);
        }
        self.bands = bands;
    }

    // ------------------------------------------------------------------ step
    pub fn step(&mut self) {
        let mut rng = self.rng.take().unwrap_or(Rng(self.rng_state));
        if self.occ.len() != self.map.w * self.map.h {
            self.after_load();
        }
        self.regrow();
        self.build_occupancy();
        let mut bands = std::mem::take(&mut self.bands);
        for bi in 0..bands.len() {
            self.band_season(&mut bands[bi], &mut rng);
        }
        self.contacts(&mut bands, &mut rng);
        self.demography(&mut bands, &mut rng);
        self.bands = bands;
        self.track_knowledge();
        self.tick += 1;
        if self.tick % 4 == 0 {
            self.year_stats();
        }
        self.rng_state = rng.0;
        self.rng = Some(rng);
    }

    fn regrow(&mut self) {
        let s = self.season();
        let r = self.params.regen;
        let m = &self.map;
        for i in 0..m.w * m.h {
            if !m.is_land(i) {
                continue;
            }
            let t = m.temp_season(i, s);
            let g = ((t - 3.0) / 12.0).clamp(0.0, 1.0);
            let k = m.k_plants[i];
            if k > 0.0 {
                let p = &mut self.plants[i];
                *p += r * (0.5 * g * *p * (1.0 - *p / k) + 0.03 * k * g) - 0.12 * (1.0 - g) * *p;
                *p = p.clamp(0.0, k);
            }
            let k = m.k_game[i];
            if k > 0.0 {
                let p = &mut self.game[i];
                *p += r * (0.12 * *p * (1.0 - *p / k) + 0.01 * k);
                *p = p.clamp(0.0, k);
            }
            let k = m.k_fish[i];
            if k > 0.0 {
                let p = &mut self.fish[i];
                *p += r * (0.3 * *p * (1.0 - *p / k) + 0.02 * k);
                *p = p.clamp(0.0, k);
            }
        }
    }

    fn build_occupancy(&mut self) {
        for v in &mut self.occ {
            v.clear();
        }
        let w = self.map.w;
        self.occ_pop = vec![0; self.occ.len()];
        for (k, b) in self.bands.iter().enumerate() {
            self.occ[b.cell(w)].push(k as u32);
            self.occ_pop[b.cell(w)] += b.pop;
        }
    }

    /// People in `cell` other than band `b` (from the last occupancy snapshot).
    fn crowd(&self, cell: usize, b: &Band) -> u32 {
        let own = if cell == b.cell(self.map.w) { b.pop } else { 0 };
        self.occ_pop.get(cell).copied().unwrap_or(0).saturating_sub(own)
    }

    /// Raw items directly at hand for a band (current cell + carried stock).
    fn raws_at(&self, b: &Band) -> ItemSet {
        let i = b.cell(self.map.w);
        let m = &self.map;
        let mut s = ItemSet::default();
        let st = |mm: Mat| b.stock.get(mm as usize).copied().unwrap_or(0.0) > 0.05;
        if m.k_wood[i] > 2.0 || st(Mat::Wood) {
            s.add(raw(Mat::Wood));
        }
        if m.k_fibre[i] > 2.0 || st(Mat::Fibre) {
            s.add(raw(Mat::Fibre));
        }
        for (d, mm) in DEPOSITS {
            if m.has(i, d) || st(mm) {
                s.add(raw(mm));
            }
        }
        for mm in [Mat::Hide, Mat::Bone] {
            if st(mm) {
                s.add(raw(mm));
            }
        }
        if self.game[i] > 5.0 {
            s.add(raw(Mat::Meat));
        }
        if self.plants[i] > 5.0 {
            s.add(raw(Mat::Plants));
        }
        if self.fish[i] > 3.0 {
            s.add(raw(Mat::Fish));
        }
        s
    }

    fn refresh_toolkit(&self, b: &mut Band) {
        let raws = self.raws_at(b);
        let key = (raws, b.flame, b.d.known_ver);
        if b.d.tk_key != Some(key) {
            b.d.tk = closure(&self.reg, &b.known, &raws, b.flame);
            b.d.kit = make_kit(&b.d.tk.avail);
            b.d.tk_key = Some(key);
        }
    }

    fn band_season(&mut self, b: &mut Band, rng: &mut Rng) {
        let p = self.params;
        let w = self.map.w;
        let ci = b.cell(w);
        let s = self.season();
        if b.stock.len() < NRAW {
            b.stock.resize(NRAW, 0.0);
        }
        for k in &mut b.known {
            k.used = false;
        }
        self.refresh_toolkit(b);
        let kit = b.d.kit;
        // ---------------- economy
        let pop = b.pop.max(1) as f32;
        let labour = 0.55 * pop;
        let t = self.map.temp_season(ci, s);
        let cold = p.cold_scale * ((8.0 - t) / 14.0).max(0.0) * (1.0 - kit.protect);
        let need = pop * (1.0 + cold);
        // tool upkeep
        let mut upkeep = 0.0;
        let mut needed = 0u32;
        let used = kit.used_items();
        for &it in &used {
            if it == NONE {
                continue;
            }
            let (_, mask, depth) = b.d.tk.via.get(&it).copied().unwrap_or((0, 0, 0));
            needed |= mask;
            if !matches!(form(it), Form::Fire | Form::Cooked) {
                upkeep += p.upkeep * (1.15 - durability(mat(it))) * (1.0 + 0.3 * depth as f32);
            } else {
                upkeep += 0.01;
            }
            mark_used(&self.reg, &mut b.known, &b.d.tk, it);
        }
        let upkeep_l = upkeep * labour;
        // home range: own cell + neighbours at reduced weight
        let (x, y) = (b.x as usize, b.y as usize);
        let mut cells: [(usize, f32); 9] = [(ci, 1.0); 9];
        let mut nc = 1;
        for (nx, ny) in nbrs8(x, y, w, self.map.h) {
            let j = ny * w + nx;
            if self.map.is_land(j) {
                cells[nc] = (j, 0.3);
                nc += 1;
            }
        }
        let cells = &cells[..nc];
        let ys = [
            cells.iter().map(|&(c, wt)| wt * self.plants[c]).sum::<f32>(),
            cells.iter().map(|&(c, wt)| wt * self.game[c]).sum::<f32>(),
            cells.iter().map(|&(c, wt)| wt * self.fish[c]).sum::<f32>(),
        ];
        let ks = [3.0 * kit.plants_m * p.forage_eff, 2.5 * kit.game_m * p.forage_eff, 2.5 * kit.fish_m * p.forage_eff];
        let wsum: f32 = (0..3).map(|r| ys[r] * ks[r]).sum::<f32>().max(1e-6);
        let harvest = |e: f32| -> [f32; 3] {
            let mut h = [0f32; 3];
            for r in 0..3 {
                if ys[r] > 0.01 {
                    let er = e * ys[r] * ks[r] / wsum;
                    h[r] = ys[r] * (1.0 - (-ks[r] * er / ys[r]).exp());
                }
            }
            h
        };
        let cook = kit.cook_m;
        let winter_next = s == 2 || (s == 1 && self.map.tamp[ci] > 8.0);
        let vq = if kit.vessel == NONE { 0.0 } else { vessel_q(mat(kit.vessel)) };
        let store_target = if winter_next { need * 0.6 * vq } else { 0.0 };
        let target = (need + store_target - b.store * 0.5).max(need * 0.5);
        let max_e = (labour * 0.85 - upkeep_l).max(labour * 0.3);
        let food_of = |e: f32| harvest(e).iter().sum::<f32>() * cook;
        let e_food = if food_of(max_e) <= target {
            max_e
        } else {
            let (mut lo, mut hi) = (0.0f32, max_e);
            for _ in 0..14 {
                let mid = 0.5 * (lo + hi);
                if food_of(mid) < target {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            hi
        };
        let h = harvest(e_food);
        let stocks: [&mut Vec<f32>; 3] = [&mut self.plants, &mut self.game, &mut self.fish];
        for (r, st) in stocks.into_iter().enumerate() {
            if ys[r] > 0.01 {
                let phi = (h[r] / ys[r]).min(1.0);
                for &(c, wt) in cells {
                    st[c] -= phi * wt * st[c];
                }
            }
        }
        let food = h.iter().sum::<f32>() * cook;
        let spoil = 0.35 - 0.25 * vq;
        b.store *= 1.0 - spoil;
        let food_ratio = if food >= need {
            b.store = (b.store + food - need).min(pop * 1.5);
            1.0
        } else {
            let take = b.store.min(need - food);
            b.store -= take;
            ((food + take) / need).min(1.0)
        };
        // hides and bones from hunting
        let hunted = h[1];
        b.stock[Mat::Hide as usize] = (b.stock[Mat::Hide as usize] + hunted * 0.01).min(1.5);
        b.stock[Mat::Bone as usize] = (b.stock[Mat::Bone as usize] + hunted * 0.01).min(1.5);
        let spare = ((labour - e_food - upkeep_l) / labour).clamp(0.0, 1.0);
        let stress = ((1.0 - food_ratio) * 2.0 + 0.15 * cold).clamp(0.0, 1.0);
        b.d.food_ratio = food_ratio;
        b.d.spare = spare;
        b.d.stress = stress;
        b.d.cold = cold;
        b.d.need = need;
        b.d.upkeep = upkeep;
        b.d.needed = needed;
        // ---------------- materials: gather what is here, use up what tools consume
        for (d, mm) in DEPOSITS {
            let mi = mm as usize;
            let want = if needed & (1 << mi) != 0 { 1.2 } else { 0.4 };
            if self.map.has(ci, d) {
                b.stock[mi] = b.stock[mi].max(want);
            }
        }
        if self.map.k_wood[ci] > 2.0 {
            b.stock[Mat::Wood as usize] = b.stock[Mat::Wood as usize].max(if needed & 1 != 0 { 1.0 } else { 0.3 });
        }
        if self.map.k_fibre[ci] > 2.0 {
            b.stock[Mat::Fibre as usize] = b.stock[Mat::Fibre as usize].max(0.6);
        }
        for mi in 0..NRAW {
            let present = match ALL_MATS[mi] {
                Mat::Wood => self.map.k_wood[ci] > 2.0,
                Mat::Fibre => self.map.k_fibre[ci] > 2.0,
                mm => DEPOSITS.iter().any(|&(d, m2)| m2 == mm && self.map.has(ci, d)),
            };
            let mut use_ = 0.02;
            if needed & (1 << mi) != 0 && !present {
                use_ += 0.06;
            }
            b.stock[mi] = (b.stock[mi] - use_).max(0.0);
        }
        // ---------------- demography (births / deaths; splits happen later)
        let fed = ((food_ratio - 0.75) / 0.25).clamp(0.0, 1.0);
        let births = rng.poisson(pop * p.birth_rate * fed * (0.8 + 0.4 * spare));
        let deaths = rng.poisson(pop * p.death_rate + pop * p.starve_rate * (1.0 - food_ratio).powf(1.5));
        b.pop = (b.pop + births).saturating_sub(deaths);
        // ---------------- fire keeping
        let can_make_fire = b.d.tk.producible.has(item(Form::Fire, Mat::Wood));
        if can_make_fire {
            b.flame = true;
        } else if b.flame && rng.chance(p.flame_loss) {
            b.flame = false;
            self.push_event(EventKind::FireLost, b.id, 0, ci as u32, NONE, u32::MAX);
        }
        let dry = (1.0 - self.map.rain[ci]).clamp(0.0, 1.0);
        let dry_s = (dry + if s == 1 { 0.2 } else if s == 3 { -0.2 } else { 0.0 }).clamp(0.0, 1.0);
        if !b.flame && self.map.k_wood[ci] + self.map.k_plants[ci] > 10.0 {
            let pf = p.wildfire_rate * dry_s * if s == 1 { 2.0 } else { 0.6 };
            if rng.chance(pf) && rng.chance((0.3 * b.curiosity).min(0.9)) {
                b.flame = true;
                let fire = item(Form::Fire, Mat::Wood);
                let first = !self.firsts.iter().any(|f| f.item == fire);
                if first {
                    self.firsts.push(First { item: fire, tick: self.tick, band: b.id, cell: ci as u32, recipe: u32::MAX });
                    self.push_event(EventKind::FirstFire, b.id, 0, ci as u32, fire, u32::MAX);
                } else {
                    self.push_event(EventKind::FireCaptured, b.id, 0, ci as u32, fire, u32::MAX);
                }
            }
        }
        // ---------------- experiments
        self.refresh_toolkit(b);
        let lam = p.invent_rate * b.curiosity * (0.15 + spare + p.stress_invent * stress) * (pop / 25.0).sqrt();
        let n = rng.poisson(lam).min(6);
        b.d.experiments = n;
        for _ in 0..n {
            self.experiment(b, rng, dry_s);
        }
        // ---------------- practice / forgetting
        let floss = p.forget_rate * (1.0 + p.small_band_loss / pop.sqrt());
        let mut forgot: Vec<u32> = Vec::new();
        for k in &mut b.known {
            if k.used {
                k.skill += p.practice * (1.0 - k.skill);
            } else {
                k.skill -= floss;
            }
            if k.skill < 0.04 {
                forgot.push(k.recipe);
            }
        }
        if !forgot.is_empty() {
            b.known.retain(|k| k.skill >= 0.04);
            b.d.known_ver += 1;
            b.forgotten += forgot.len() as u32;
            self.counters.forgets += forgot.len() as u64;
            self.year_acc.forgets += forgot.len() as u32;
            for r in forgot {
                let it = self.reg.recipes[r as usize].output;
                self.push_event(EventKind::Forgot, b.id, 0, ci as u32, it, r);
            }
        }
        // ---------------- movement
        self.refresh_toolkit(b);
        self.maybe_move(b, rng);
    }

    /// One experiment: pick a primitive action and things at hand, see what physics says.
    fn experiment(&mut self, b: &mut Band, rng: &mut Rng, dryness: f32) {
        self.counters.experiments += 1;
        self.year_acc.experiments += 1;
        let pool: Vec<Item> = b
            .d
            .tk
            .avail
            .iter()
            .filter(|&i| !matches!(form(i), Form::Cooked | Form::Fire | Form::Kiln))
            .collect();
        if pool.is_empty() {
            return;
        }
        let fires: Vec<Item> = b.d.tk.avail.iter().filter(|&i| form(i) == Form::Fire).collect();
        let kilns: Vec<Item> = b.d.tk.avail.iter().filter(|&i| form(i) == Form::Kiln).collect();
        // things near a fire also get combined with it (feeding, building around)
        let mut cpool = pool.clone();
        cpool.extend(fires.iter().copied());
        let pick = |rng: &mut Rng, v: &[Item]| v[rng.below(v.len())];
        // salience: things that already work as tools get tried as tools; hard cobbles too
        let tool_w: Vec<f32> = pool
            .iter()
            .map(|&i| match form(i) {
                Form::Hammer | Form::Edge | Form::Point => 3.0,
                Form::Raw if mat(i).props().hard >= 5.0 => 1.5,
                _ => 0.3,
            })
            .collect();
        let roll = rng.f32();
        let (action, inputs): (Action, Inputs) = if !fires.is_empty() && roll < 0.15 {
            let t1 = pick(rng, &pool);
            let t2 = if rng.chance(0.25) { pick(rng, &pool) } else { NONE };
            if t1 == t2 {
                return;
            }
            let k = if !kilns.is_empty() && rng.chance(0.6) { pick(rng, &kilns) } else { NONE };
            (Action::Heat, [t1, t2, pick(rng, &fires), k])
        } else if roll < 0.6 {
            let t = pick(rng, &pool);
            let tool = if rng.chance(0.35) { NONE } else { pool[rng.weighted(&tool_w).unwrap_or(0)] };
            if t == tool {
                return;
            }
            (Action::Shape, [t, tool, NONE, NONE])
        } else {
            // a band that has lost (or never kept) its flame fiddles with burnables,
            // trying to get one back: goal-driven tinkering, still judged by physics
            let fuels: Vec<Item> = pool.iter().copied().filter(|&i| mat(i).props().fuel_t > 0.0).collect();
            let src = if !b.flame && fuels.len() >= 2 && rng.chance(0.4) { &fuels } else { &cpool };
            let a = pick(rng, src);
            let bb = pick(rng, src);
            let c = if rng.chance(0.45) { pick(rng, src) } else { NONE };
            if a == bb || a == c || bb == c {
                return;
            }
            (Action::Combine, [a, bb, c, NONE])
        };
        let inputs = physics::canonical(action, inputs);
        // tinkering with things keeps the know-how for them alive
        for &i in inputs.iter() {
            if i != NONE {
                mark_used(&self.reg, &mut b.known, &b.d.tk, i);
            }
        }
        let ctx = Ctx { dryness };
        let Some(out) = physics::apply(action, &inputs, &ctx) else { return };
        if b.d.tk.producible.has(out.out) || !rng.chance(out.p) {
            return;
        }
        let ci = b.cell(self.map.w) as u32;
        let rid = match self.reg.find(action, &inputs) {
            Some(r) => r,
            None => self.reg.insert(Recipe {
                action: action as u8,
                inputs,
                output: out.out,
                p: out.p,
                first_tick: self.tick,
                first_band: b.id,
                first_cell: ci,
                inventions: 0,
                transfers: 0,
            }),
        };
        if b.knows(rid) {
            return;
        }
        self.reg.recipes[rid as usize].inventions += 1;
        b.known.push(Known { recipe: rid, skill: 0.35, since: self.tick, from: FROM_SELF, used: true });
        b.d.known_ver += 1;
        b.inventions += 1;
        self.counters.inventions += 1;
        self.year_acc.inventions += 1;
        let first = !self.firsts.iter().any(|f| f.item == out.out);
        if first {
            self.firsts.push(First { item: out.out, tick: self.tick, band: b.id, cell: ci, recipe: rid });
            self.push_event(EventKind::FirstInvention, b.id, 0, ci, out.out, rid);
        } else {
            self.push_event(EventKind::Invention, b.id, 0, ci, out.out, rid);
        }
        self.refresh_toolkit(b);
    }

    fn cell_score(&self, b: &Band, c: usize) -> f32 {
        let m = &self.map;
        if !m.is_land(c) {
            return -1.0;
        }
        let kit = &b.d.kit;
        let pot = |j: usize| 3.0 * kit.plants_m * self.plants[j] + 2.5 * kit.game_m * self.game[j] + 2.5 * kit.fish_m * self.fish[j];
        let (x, y) = (c % m.w, c / m.w);
        let mut nb = 0.0f32;
        let mut cnt = 0.0f32;
        for (nx, ny) in nbrs4(x, y, m.w, m.h) {
            let j = ny * m.w + nx;
            if m.is_land(j) {
                nb += pot(j);
                cnt += 1.0;
            }
        }
        let mut s = pot(c) + 0.6 * nb / cnt.max(1.0) * 2.0;
        let crowd = self.crowd(c, b) as f32;
        s /= 1.0 + crowd / (b.pop as f32 + 10.0);
        let winter = m.temp_season(c, 3);
        let cold = self.params.cold_scale * ((8.0 - winter) / 14.0).max(0.0) * (1.0 - kit.protect);
        s /= 1.0 + 1.5 * cold;
        // materials the band's tools need and is running out of
        for (d, mm) in DEPOSITS {
            let mi = mm as usize;
            if b.d.needed & (1 << mi) != 0 && b.stock[mi] < 0.4 && m.has(c, d) {
                s *= 1.35;
            }
        }
        s
    }

    fn maybe_move(&mut self, b: &mut Band, rng: &mut Rng) {
        let m = &self.map;
        let (x, y) = (b.x as usize, b.y as usize);
        let cur = b.cell(m.w);
        let mut best = cur;
        let mut best_s = self.cell_score(b, cur) * self.params.move_inertia;
        let mut cands: Vec<usize> = nbrs8(x, y, m.w, m.h).map(|(a, c)| c * m.w + a).collect();
        // occasionally scout further
        let dx = rng.below(7) as i32 - 3;
        let dy = rng.below(7) as i32 - 3;
        let (sx, sy) = (x as i32 + dx, y as i32 + dy);
        if sx >= 0 && sy >= 0 && (sx as usize) < m.w && (sy as usize) < m.h {
            cands.push(sy as usize * m.w + sx as usize);
        }
        for c in cands {
            let (cx, cy) = (c % m.w, c / m.w);
            let dist = (cx as i32 - x as i32).abs().max((cy as i32 - y as i32).abs()) as f32;
            let sc = self.cell_score(b, c) / (1.0 + 0.08 * (dist - 1.0).max(0.0));
            if sc > best_s {
                best_s = sc;
                best = c;
            }
        }
        if best != cur {
            b.x = (best % m.w) as u16;
            b.y = (best / m.w) as u16;
        }
    }

    // ------------------------------------------------------------------ contacts
    fn contacts(&mut self, bands: &mut [Band], rng: &mut Rng) {
        let p = self.params;
        let w = self.map.w as i32;
        let h = self.map.h as i32;
        let r = p.teach_radius as i32;
        // rebuild occupancy after movement
        for v in &mut self.occ {
            v.clear();
        }
        self.occ_pop = vec![0; self.occ.len()];
        for (k, b) in bands.iter().enumerate() {
            self.occ[b.cell(w as usize)].push(k as u32);
            self.occ_pop[b.cell(w as usize)] += b.pop;
        }
        let mut pairs: Vec<(usize, usize, i32)> = Vec::new();
        for (a, b) in bands.iter().enumerate() {
            let (x, y) = (b.x as i32, b.y as i32);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    for &k in &self.occ[(ny * w + nx) as usize] {
                        let k = k as usize;
                        if k > a {
                            pairs.push((a, k, dx.abs().max(dy.abs())));
                        }
                    }
                }
            }
        }
        for (a, c, d) in pairs {
            let kin = if bands[a].lineage == bands[c].lineage { 1.3 } else { 1.0 };
            let pc = p.teach_rate * kin / (1.0 + d as f32);
            if !rng.chance(pc) {
                continue;
            }
            self.exchange(bands, a, c, rng);
            self.exchange(bands, c, a, rng);
        }
    }

    /// `src` shows `dst` one thing it knows (if `dst` can use it, it is more likely to stick).
    fn exchange(&mut self, bands: &mut [Band], src: usize, dst: usize, rng: &mut Rng) {
        // fire can be shared
        if bands[src].flame && !bands[dst].flame && rng.chance(0.5) {
            bands[dst].flame = true;
        }
        let cands: Vec<(u32, f32)> = bands[src]
            .known
            .iter()
            .filter(|k| k.skill >= 0.25)
            .filter(|k| {
                let out = self.reg.recipes[k.recipe as usize].output;
                !bands[dst].d.tk.producible.has(out) && !bands[dst].knows(k.recipe)
            })
            .map(|k| (k.recipe, k.skill))
            .collect();
        if cands.is_empty() {
            return;
        }
        let (rid, skill) = cands[rng.below(cands.len())];
        let r = &self.reg.recipes[rid as usize];
        let relevant = r.inputs.iter().all(|&i| bands[dst].d.tk.avail.has(i));
        let pl = skill * if relevant { 0.8 } else { 0.25 };
        if !rng.chance(pl) {
            return;
        }
        let out = r.output;
        let srcid = bands[src].id;
        let d = &mut bands[dst];
        d.known.push(Known { recipe: rid, skill: 0.3, since: self.tick, from: srcid, used: true });
        d.d.known_ver += 1;
        d.learned += 1;
        let (did, dc) = (d.id, d.cell(self.map.w) as u32);
        self.reg.recipes[rid as usize].transfers += 1;
        self.counters.transfers += 1;
        self.year_acc.transfers += 1;
        self.push_event(EventKind::Learned, did, srcid, dc, out, rid);
        let tk_dirty = &mut bands[dst];
        self.refresh_toolkit(tk_dirty);
    }

    // ------------------------------------------------------------------ splits, merges, deaths
    fn demography(&mut self, bands: &mut Vec<Band>, rng: &mut Rng) {
        let p = self.params;
        let w = self.map.w;
        let h = self.map.h;
        let mut newb: Vec<Band> = Vec::new();
        let n0 = bands.len();
        for k in 0..n0 {
            let thresh = p.split_size * (0.8 + 0.4 * rng.f32());
            if (bands[k].pop as f32) <= thresh {
                continue;
            }
            let b = &bands[k];
            // daughter settles in the best nearby cell
            let (x, y) = (b.x as i32, b.y as i32);
            let mut best = (b.cell(w), f32::MIN);
            for _ in 0..8 {
                let nx = x + rng.below(5) as i32 - 2;
                let ny = y + rng.below(5) as i32 - 2;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let c = ny as usize * w + nx as usize;
                let s = self.cell_score(b, c);
                if s > best.1 {
                    best = (c, s);
                }
            }
            let dpop = (b.pop as f32 * rng.range(0.35, 0.5)) as u32;
            let id = self.next_id;
            self.next_id += 1;
            let known: Vec<Known> = b
                .known
                .iter()
                .filter(|_| true)
                .filter_map(|kn| {
                    if rng.chance(0.55 + 0.45 * kn.skill) {
                        Some(Known { skill: kn.skill * 0.9, from: FROM_PARENT, since: self.tick, ..*kn })
                    } else {
                        None
                    }
                })
                .collect();
            let d = Band {
                id,
                lineage: b.lineage,
                parent: b.id,
                gen: b.gen + 1,
                x: (best.0 % w) as u16,
                y: (best.0 / w) as u16,
                pop: dpop,
                store: b.store * 0.4,
                curiosity: (b.curiosity * (0.12 * rng.gauss()).exp()).clamp(0.3, 2.5),
                flame: b.flame,
                known,
                stock: b.stock.clone(),
                born: self.tick,
                ..Default::default()
            };
            let bm = &mut bands[k];
            bm.pop -= dpop;
            bm.store *= 0.6;
            self.counters.splits += 1;
            self.push_event(EventKind::Split, bm.id, id, best.0 as u32, NONE, u32::MAX);
            newb.push(d);
        }
        // tiny bands merge with a neighbour or vanish
        for k in 0..n0 {
            if bands[k].pop >= 4 {
                continue;
            }
            let (x, y) = (bands[k].x as i32, bands[k].y as i32);
            let target = (0..n0).find(|&j| {
                j != k && bands[j].pop >= 4 && (bands[j].x as i32 - x).abs() <= 1 && (bands[j].y as i32 - y).abs() <= 1
            });
            let (id, cell) = (bands[k].id, bands[k].cell(w) as u32);
            if let Some(j) = target {
                let moved: Vec<Known> = bands[k].known.clone();
                let pop = bands[k].pop;
                let tid = bands[j].id;
                let tb = &mut bands[j];
                tb.pop += pop;
                for kn in moved {
                    if !tb.knows(kn.recipe) && rng.chance(kn.skill * 0.6) {
                        tb.known.push(Known { skill: kn.skill * 0.7, from: id, since: self.tick, ..kn });
                        tb.d.known_ver += 1;
                    }
                }
                self.push_event(EventKind::Merged, id, tid, cell, NONE, u32::MAX);
            } else {
                self.push_event(EventKind::Died, id, 0, cell, NONE, u32::MAX);
                self.counters.deaths += 1;
            }
            bands[k].pop = 0;
        }
        bands.retain(|b| b.pop > 0);
        for mut d in newb {
            self.refresh_toolkit(&mut d);
            bands.push(d);
        }
    }

    // ------------------------------------------------------------------ statistics
    fn track_knowledge(&mut self) {
        let w = self.map.w;
        let n = self.map.w * self.map.h;
        let tick = self.tick;
        for b in &self.bands {
            let c = b.cell(w);
            let mut av = b.d.tk.producible;
            if b.flame {
                av.add(item(Form::Fire, Mat::Wood));
            }
            for it in av.iter() {
                let v = self.cell_first.entry(it).or_insert_with(|| vec![u32::MAX; n]);
                if v[c] == u32::MAX {
                    v[c] = tick;
                }
            }
        }
    }

    fn year_stats(&mut self) {
        let mut counts: BTreeMap<Item, u32> = BTreeMap::new();
        let mut sum = 0u64;
        let mut mx = 0u32;
        let mut with_fire = 0;
        for b in &self.bands {
            let mut av = b.d.tk.producible;
            if b.flame {
                av.add(item(Form::Fire, Mat::Wood));
                with_fire += 1;
            }
            let c = av.count();
            sum += c as u64;
            mx = mx.max(c);
            for it in av.iter() {
                *counts.entry(it).or_default() += 1;
            }
        }
        // anything that was known last year and now by nobody is lost to the world
        let lost: Vec<Item> = self
            .item_bands
            .iter()
            .filter(|(it, &c)| c > 0 && !counts.contains_key(it))
            .map(|(&it, _)| it)
            .collect();
        for it in lost {
            self.push_event(EventKind::LostWorldwide, 0, 0, 0, it, u32::MAX);
        }
        self.item_bands = counts;
        let acc = std::mem::take(&mut self.year_acc);
        let nb = self.bands.len() as u32;
        self.history.push(YearStats {
            year: self.tick / 4,
            pop: self.total_pop(),
            bands: nb,
            items_world: self.item_bands.len() as u32,
            items_mean: if nb > 0 { sum as f32 / nb as f32 } else { 0.0 },
            items_max: mx,
            with_fire,
            ..acc
        });
    }

    /// Deterministic hash of the simulation state (for regression checks).
    pub fn state_hash(&self) -> u64 {
        let mut hsh: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            hsh ^= v;
            hsh = hsh.wrapping_mul(0x100_0000_01b3);
        };
        mix(self.tick as u64);
        mix(self.rng_state);
        for b in &self.bands {
            mix(b.id as u64);
            mix(b.pop as u64);
            mix(((b.x as u64) << 16) | b.y as u64);
            mix(b.known.len() as u64);
        }
        mix(self.reg.recipes.len() as u64);
        hsh
    }

    pub fn event_text(&self, e: &Event) -> String {
        let yr = e.tick / 4;
        let it = if e.item == NONE { String::new() } else { item_name(e.item) };
        let how = |r: u32| -> String {
            if r == u32::MAX || r as usize >= self.reg.recipes.len() {
                String::new()
            } else {
                format!(" ({})", self.reg.recipes[r as usize].describe())
            }
        };
        let (cx, cy) = (e.cell as usize % self.map.w.max(1), e.cell as usize / self.map.w.max(1));
        match EventKind::from_u8(e.kind) {
            EventKind::FirstInvention => format!("Y{yr}: band {} at ({cx},{cy}) is the first to make {it}{}", e.band, how(e.recipe)),
            EventKind::Invention => format!("Y{yr}: band {} independently worked out {it}{}", e.band, how(e.recipe)),
            EventKind::Learned => format!("Y{yr}: band {} learned {it} from band {}", e.band, e.other),
            EventKind::Forgot => format!("Y{yr}: band {} forgot how to make {it}", e.band),
            EventKind::LostWorldwide => format!("Y{yr}: nobody in the world can make {it} any more"),
            EventKind::FireCaptured => format!("Y{yr}: band {} took fire from a wildfire", e.band),
            EventKind::FirstFire => format!("Y{yr}: band {} at ({cx},{cy}) is the first to keep fire, taken from a wildfire", e.band),
            EventKind::FireLost => format!("Y{yr}: band {}'s fire went out", e.band),
            EventKind::Split => format!("Y{yr}: band {} split; band {} moved to ({cx},{cy})", e.band, e.other),
            EventKind::Merged => format!("Y{yr}: band {} dwindled and joined band {}", e.band, e.other),
            EventKind::Died => format!("Y{yr}: band {} died out at ({cx},{cy})", e.band),
            EventKind::Founded => format!("Y{yr}: band {} founded at ({cx},{cy})", e.band),
        }
    }
}
