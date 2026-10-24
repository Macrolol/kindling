//! The farming stage (v5). Nothing here is scripted: no band is told to farm.
//!
//! * Camps leave a mark. People who keep coming back to the same stands weed, replant and
//!   protect them (`tend` up to 0.4), which raises the wild plant yield there.
//! * A band that leans on tended stands of seed plants, has a digging stick, and is hungry,
//!   crowded or curious may start clearing and sowing fields (`Band::sow`). Fields (`tend` above
//!   0.4) carry a standing crop that grows in the warm seasons and dies back in the cold.
//! * Harvesting with sickles and resowing the kept seed selects for plants that hold their grain
//!   (`dom`); the band carries its strain as seed grain, which spreads in trade and raids.
//! * Cropping drains soil fertility (`fert`); rivers, fallow and manure bring it back.
//! * Bands that live off fields stay put (`Band::settled`), store more (grain keeps), have
//!   children closer together, and crowd - which breeds crowd disease, worse with herds close by.
//! * Bands that hunt herd game on open grassland may start keeping animals (`Band::herd`), which
//!   slowly tame; tame herds give wool in cool lands and pull ards for farmers with axes.
//! * Bands give it all up again when it stops paying (moved to poor land, soil worn out, seed
//!   eaten in a famine).
use super::*;

const FLAG_PLOUGH: u32 = 1;
const FLAG_HERD: u32 = 2;

/// Standing crop of a full, fertile, fully domestic field of potential 1 (food units per cell).
pub const CROP_K: f32 = 300.0;

/// How well grain plants grow in a cell (0 = not at all). Big-seeded annuals belong to lands with
/// a marked bad season, enough water and warmth; rivers water and renew the ground.
pub fn crop_potential(m: &Map, i: usize) -> f32 {
    if !m.is_land(i) {
        return 0.0;
    }
    use crate::map::Biome::*;
    let base = match m.biome_at(i) {
        Grassland => 1.0,
        Savanna => 0.9,
        Steppe => 0.7,
        TempForest => 0.6,
        TropForest => 0.35,
        Taiga => 0.15,
        Mountain => 0.1,
        Desert | Tundra | Ice | Ocean => 0.0,
    };
    if base == 0.0 {
        return 0.0;
    }
    // a marked dry or cold season suits annual grasses; the ever-wet tropics less so
    let season = 0.6 + 0.4 * (m.tamp[i] / 12.0).min(1.0);
    let water = if m.river[i] { 1.0 } else { ((m.rain[i] - 0.15) / 0.35).clamp(0.0, 1.0) };
    let warm = ((m.temp[i] - 4.0) / 8.0).clamp(0.0, 1.0);
    base * season * water * warm * if m.river[i] { 1.3 } else { 1.0 }
}

/// Bit flags for where the wild ancestors of crops and herd animals live (derived from the map,
/// not saved). Big-seeded wild grasses that can be brought into cultivation and wild animals
/// placid and social enough to be kept grow only in some patches of the world; outside them a
/// band can still farm or herd, but only with seed or animals passed on by others.
pub const WILD_GRAIN: u8 = 1;
pub const WILD_HERD: u8 = 2;
pub fn wild_progenitors(m: &Map, i: usize) -> u8 {
    if !m.is_land(i) {
        return 0;
    }
    let (x, y) = ((i % m.w) as f32, (i / m.w) as f32);
    let mut f = 0;
    if crop_potential(m, i) > 0.3 && crate::map::fbm(m.seed ^ 0x6a41_9e37, x / 22.0, y / 22.0, 3) > 0.28 {
        f |= WILD_GRAIN;
    }
    if graze_potential(m, i) >= 0.4 && crate::map::fbm(m.seed ^ 0x1f3d_5c81, x / 22.0, y / 22.0, 3) > 0.25 {
        f |= WILD_HERD;
    }
    f
}

/// How well a cell carries grazing herds (0..1).
pub fn graze_potential(m: &Map, i: usize) -> f32 {
    if !m.is_land(i) {
        return 0.0;
    }
    use crate::map::Biome::*;
    match m.biome_at(i) {
        Steppe | Grassland => 1.0,
        Savanna => 0.8,
        Mountain => 0.5,
        Tundra | TempForest => 0.4,
        Taiga => 0.3,
        Desert => 0.2,
        TropForest => 0.1,
        Ice | Ocean => 0.0,
    }
}

/// Share of a cell that is cleared, sown field (from its `tend`).
#[inline]
pub fn cultivated(td: f32) -> f32 {
    ((td - 0.4) / 0.6).clamp(0.0, 1.0)
}

/// Wild plant capacity of a cell given how tended it is: tended stands yield more, fields
/// replace the wild plants.
#[inline]
pub fn wild_plant_cap(k: f32, td: f32) -> f32 {
    k * (1.0 + 0.3 * td.min(0.4) / 0.4) * (1.0 - 0.6 * cultivated(td))
}

/// The season's farming terms for a band (see `World::economy`).
#[derive(Clone, Copy, Debug)]
pub struct FarmTerms {
    /// share of labour taken by field work and herding
    pub labour: f32,
    /// field work multiplier from an animal-drawn ard
    pub plough: f32,
    /// food from the herd this season
    pub herd_food: f32,
    /// weighted cultivated area in the home range (cells)
    pub fields: f32,
}

impl World {
    /// Size the per-cell farming layers (older saves start fresh) and derive the potentials.
    pub(crate) fn ensure_farm_layers(&mut self) {
        let n = self.map.w * self.map.h;
        for v in [&mut self.camp, &mut self.tend, &mut self.dom, &mut self.crop] {
            if v.len() != n {
                *v = vec![0.0; n];
            }
        }
        if self.fert.len() != n {
            self.fert = vec![1.0; n];
        }
        if self.crop_pot.len() != n || self.wild.len() != n {
            self.crop_pot = (0..n).map(|i| crop_potential(&self.map, i)).collect();
            self.graze_pot = (0..n).map(|i| graze_potential(&self.map, i)).collect();
            self.wild = (0..n).map(|i| wild_progenitors(&self.map, i)).collect();
        }
    }

    /// The crop a cell would carry at full growth.
    pub fn crop_cap(&self, i: usize) -> f32 {
        let cult = cultivated(self.tend.get(i).copied().unwrap_or(0.0));
        if cult <= 0.0 {
            return 0.0;
        }
        CROP_K * self.params.farm_yield * self.crop_pot[i] * cult * (0.45 + 0.55 * self.dom[i]) * self.fert[i]
    }

    /// What a farming band would make of a cell (for choosing where to live): its fields, or the
    /// fields it could clear there.
    pub(super) fn farm_prospect(&self, b: &Band, i: usize) -> f32 {
        let pot = self.crop_pot.get(i).copied().unwrap_or(0.0);
        if pot <= 0.0 {
            return 0.0;
        }
        let cult = cultivated(self.tend[i]).max(0.35);
        CROP_K * self.params.farm_yield * pot * cult * (0.45 + 0.55 * self.dom[i].max(b.strain)) * self.fert[i]
    }

    /// Seasonal change in the farming layers: crops grow and die back, untended fields go back to
    /// the wild, soils rest and recover.
    pub(super) fn regrow_fields(&mut self) {
        let s = self.season();
        let n = self.map.w * self.map.h;
        if self.tend.len() != n {
            self.ensure_farm_layers();
        }
        let fy = CROP_K * self.params.farm_yield;
        let m = &self.map;
        let pot = &self.crop_pot;
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            const CH: usize = 4096;
            if n >= 65536 && [self.camp.len(), self.dom.len(), self.fert.len(), self.crop.len(), pot.len()].iter().all(|&l| l == n) {
                self.camp
                    .par_chunks_mut(CH)
                    .zip(self.tend.par_chunks_mut(CH))
                    .zip(self.dom.par_chunks_mut(CH))
                    .zip(self.fert.par_chunks_mut(CH))
                    .zip(self.crop.par_chunks_mut(CH))
                    .enumerate()
                    .for_each(|(ci, ((((ca, te), dm), fe), cr))| {
                        for j in 0..ca.len() {
                            let i = ci * CH + j;
                            field_cell(m, s, fy, pot[i], i, &mut ca[j], &mut te[j], &mut dm[j], &mut fe[j], &mut cr[j]);
                        }
                    });
                return;
            }
        }
        for i in 0..n {
            let (mut ca, mut te, mut dm, mut fe, mut cr) = (self.camp[i], self.tend[i], self.dom[i], self.fert[i], self.crop[i]);
            field_cell(m, s, fy, pot[i], i, &mut ca, &mut te, &mut dm, &mut fe, &mut cr);
            (self.camp[i], self.tend[i], self.dom[i], self.fert[i], self.crop[i]) = (ca, te, dm, fe, cr);
        }
    }

    /// Labour, food and plough terms of a band's fields and herd this season.
    pub(super) fn farm_terms(&self, b: &Band, cells: &[(usize, f32)], t: f32) -> FarmTerms {
        let mut ft = FarmTerms { labour: 0.0, plough: 1.0, herd_food: 0.0, fields: 0.0 };
        if b.sow > 0.0 && !self.tend.is_empty() {
            let f: f32 = cells.iter().map(|&(c, wt)| wt * cultivated(self.tend.get(c).copied().unwrap_or(0.0))).sum();
            ft.fields = f;
            if t > 4.0 {
                // clearing, weeding, watering, keeping birds and beasts off
                ft.labour += 0.12 * b.sow * (f / 1.6).min(1.0);
            }
            if b.tame > 0.6 && b.herd > 0.15 && b.d.kit.axe != NONE {
                ft.plough = 1.4;
            }
        }
        if b.herd > 0.0 && !self.graze_pot.is_empty() {
            let gp = self.graze_pot[b.cell(self.map.w)].max(0.3);
            ft.herd_food = b.pop as f32 * b.herd * 0.22 * (0.4 + 0.6 * b.tame) * gp * self.params.forage_eff;
            ft.labour += 0.08 * b.herd.min(1.0);
        }
        ft
    }

    /// No other band's fields within `r` cells of `c` (so a start here is independent)?
    fn no_fields_near(&self, c: usize, r: i32) -> bool {
        let (w, h) = (self.map.w as i32, self.map.h as i32);
        let (x, y) = ((c % self.map.w) as i32, (c / self.map.w) as i32);
        for dy in -r..=r {
            for dx in -r..=r {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                if self.tend[(ny * w + nx) as usize] > 0.45 {
                    return false;
                }
            }
        }
        true
    }

    /// A band's season with its stands, fields and animals (after the harvest is taken).
    pub(super) fn farm_season(&self, b: &mut Band, ec: &Econ, rng: &mut Rng, fx: &mut Fx) {
        let n = self.map.w * self.map.h;
        if self.tend.len() != n || b.pop == 0 {
            return;
        }
        let p = self.params;
        let w = self.map.w;
        let ci = b.cell(w);
        let pop = b.pop as f32;
        let kit = b.d.kit;
        let dg = kit.digest;
        let food = ec.food.max(1e-3);
        let plant_share = (ec.h[0] * dg[0] / food).clamp(0.0, 1.0);
        let crop_share = (ec.h[4] * dg[4] / food).clamp(0.0, 1.0);
        let farm_share = (ec.farm_food / food).clamp(0.0, 1.0);
        let pot = self.crop_pot[ci];
        // ---- camp memory and the tending of wild stands
        // (the shared farm layers are read as they stood at the start of the season and the
        // changes buffered; `tend_ci` / `dom_ci` / `fert_ci` track this band's own cell)
        let dcamp = (pop / 30.0).min(2.0) * (0.5 + plant_share);
        fx.cell(Layer::Camp, ci, dcamp);
        let camp = self.camp[ci] + dcamp;
        let campf = camp / (camp + 20.0);
        let mut tend_ci = self.tend[ci];
        for &(c, wt) in &ec.cells {
            if !self.map.is_land(c) {
                continue;
            }
            let t0 = self.tend[c];
            let mut td = t0;
            if td < 0.4 {
                td += p.farm_rate * 0.03 * wt * campf * plant_share * (0.4 - td);
            }
            if b.sow > 0.0 && self.crop_pot[c] > 0.05 {
                // clearing and sowing: the home cell most, the edges of the range less
                td += 0.25 * b.sow * wt * (1.0 - td) * (0.5 + 0.5 * kit.farm_m.min(1.5) / 1.5);
            }
            td = td.min(1.0);
            fx.cell(Layer::Tend, c, td - t0);
            if c == ci {
                tend_ci = td;
            }
        }
        // settling in: people who stay put build lasting houses, stores, field walls
        // (living off fields and herds most; a long-used camp a little; it fades if neither)
        b.settled = (b.settled * 0.99 + (0.03 * farm_share + 0.002 * campf) * (1.0 - b.settled)).clamp(0.0, 1.0);
        // ---- starting to farm (organic: tended stands + digging stick + push to do more)
        if b.sow <= 0.0 {
            let seed = b.strain > 0.0 && b.stock[Mat::Grain as usize] > 0.05;
            let wild = self.wild[ci] & WILD_GRAIN != 0;
            if pot > 0.2 && kit.dig != NONE && ((wild && tend_ci > 0.25) || seed) {
                let crowd = self.crowd(ci, b) as f32 / (pop + 10.0);
                // nobody takes on field work unless foraging is getting hard (hunger, crowding)
                let push = (b.d.stress + crowd).min(1.5);
                let ch = if seed {
                    p.farm_rate * 0.005 * pot * b.curiosity * (0.2 + push)
                } else {
                    p.farm_rate * 0.0006 * tend_ci * pot * b.curiosity * push * push
                };
                if rng.chance(ch) {
                    b.sow = 0.3;
                    b.settled = b.settled.max(0.2);
                    fx.counters.farm_starts += 1;
                    if !seed && self.no_fields_near(ci, 12) {
                        fx.ops.push(Op::FarmOrigin { band: b.id, cell: ci as u32 });
                    } else {
                        fx.counters.took_up_farming += 1;
                        fx.push_event(EventKind::TookUpFarming, b.id, 0, ci as u32, NONE, u32::MAX);
                    }
                }
            }
        } else {
            // ---- practice: farming that pays deepens; farming that does not fades
            if crop_share > 0.08 && pot > 0.2 {
                b.sow = (b.sow + 0.02 * (1.0 - b.sow)).min(1.0);
            } else if pot > 0.2 {
                b.sow -= 0.004;
            } else {
                b.sow -= 0.03; // poor ground for grain: the know-how lapses within a generation
            }
            // ---- harvest: selection for non-shattering grain, soil drawn down
            if ec.ys[4] > 0.01 {
                let phi = (ec.h[4] / ec.ys[4]).min(1.0);
                let sickle = if kit.cut != NONE { 1.0 } else { 0.5 };
                let strain = if b.stock[Mat::Grain as usize] > 0.05 { b.strain } else { 0.0 };
                let mut dom_ci = self.dom[ci];
                for &(c, wt) in &ec.cells {
                    let cult = cultivated(self.tend.get(c).copied().unwrap_or(0.0));
                    if cult <= 0.0 {
                        continue;
                    }
                    let d0 = self.dom[c];
                    let mut d = d0;
                    d += p.farm_rate * 0.02 * sickle * phi.max(0.2) * wt * cult * (1.0 - d);
                    if strain > d {
                        d += 0.3 * wt * (strain - d);
                    }
                    d = d.min(1.0);
                    fx.cell(Layer::Dom, c, d - d0);
                    if c == ci {
                        dom_ci = d;
                    }
                    let f = self.fert[c];
                    fx.cell(Layer::Fert, c, -p.soil_drain * 0.05 * phi * wt * cult * f);
                }
                let was = b.strain;
                b.strain = (b.strain * 0.999).max(dom_ci);
                if was < 0.6 && b.strain >= 0.6 && !self.firsts.iter().any(|f| f.item == raw(Mat::Grain)) {
                    fx.first(raw(Mat::Grain), b.id, ci as u32, u32::MAX, EventKind::FirstDomestic, None);
                }
                if ec.h[4] > 0.02 * pop {
                    let g = Mat::Grain as usize;
                    b.stock[g] = b.stock[g].max(0.6);
                    b.origin[g] = ci as u32;
                }
            }
            // flax: a fibre crop taken into the fields where it grows wild
            let fl = Mat::Flax as usize;
            if b.sow > 0.4 && pot > 0.1 {
                let t = self.map.temp[ci];
                if b.stock[fl] > 0.05 {
                    b.stock[fl] = b.stock[fl].max(0.8);
                } else if self.map.k_fibre[ci] > 2.0 && (5.0..20.0).contains(&t) && rng.chance(p.farm_rate * 0.004 * b.sow) {
                    b.stock[fl] = 0.8;
                    b.origin[fl] = ci as u32;
                    if !self.firsts.iter().any(|f| f.item == raw(Mat::Flax)) {
                        fx.first(raw(Mat::Flax), b.id, ci as u32, u32::MAX, EventKind::FirstDomestic, None);
                    }
                }
            }
            if b.sow < 0.05 {
                b.sow = 0.0;
                fx.counters.abandoned_farming += 1;
                fx.push_event(EventKind::AbandonedFarming, b.id, 0, ci as u32, NONE, u32::MAX);
            }
            if b.tame > 0.6 && b.herd > 0.15 && kit.axe != NONE && self.farm_flags & FLAG_PLOUGH == 0 {
                fx.ops.push(Op::Flag { flag: FLAG_PLOUGH, kind: EventKind::FirstPlough, band: b.id, cell: ci as u32 });
            }
        }
        // seed grain: kept back while farming, eaten in a famine, slowly used otherwise
        let g = Mat::Grain as usize;
        if b.stock[g] > 0.0 {
            let loss = if ec.food_ratio < 0.6 { 0.25 } else if b.sow > 0.0 { 0.004 } else { 0.02 };
            b.stock[g] = (b.stock[g] - loss).max(0.0);
            if b.stock[g] <= 0.0 {
                b.origin[g] = u32::MAX;
                b.strain = 0.0;
            }
        }
        // ---- herds
        let gp = self.graze_pot[ci];
        if b.herd <= 0.0 {
            let game_share = ((ec.h[1] * dg[1]) / food).clamp(0.0, 1.0);
            // keeping animals takes people who stay near them (settled, tied to fields) or
            // hunters who live off one herd and start to follow and manage it
            if kit.cord != NONE && self.wild[ci] & WILD_HERD != 0 && game_share > 0.1 && gp > 0.4 && self.game[ci] > 3.0 && (b.settled > 0.1 || b.sow > 0.0 || game_share > 0.3) {
                let ch = p.herd_rate * 0.002 * game_share * gp * b.curiosity * (0.5 + b.settled + b.d.stress);
                if rng.chance(ch) {
                    b.herd = 0.05;
                    b.tame = 0.05;
                    fx.counters.herds_started += 1;
                    if self.farm_flags & FLAG_HERD == 0 {
                        fx.ops.push(Op::Flag { flag: FLAG_HERD, kind: EventKind::FirstHerding, band: b.id, cell: ci as u32 });
                    }
                }
            }
        } else {
            let fields = if b.sow > 0.0 { 0.3 * (tend_ci - 0.4).max(0.0) } else { 0.0 };
            // open grazing carries a herd; woodland hardly does
            let hk = 0.9 * (gp - 0.25).max(0.0) / 0.75 + fields;
            if hk < 0.03 {
                b.herd *= 0.9; // nothing to graze: the animals are eaten or wander off
            } else {
                b.herd += 0.1 * b.herd * (1.0 - b.herd / hk);
            }
            if ec.food_ratio < 0.85 {
                b.herd *= 0.85; // breeding stock eaten
            }
            b.tame = (b.tame + 0.004 * (1.0 - b.tame)).min(1.0);
            // dung on the fields
            let f = self.fert[ci];
            fx.cell(Layer::Fert, ci, 0.01 * b.herd.min(1.0) * (1.0 - f));
            let hb = 0.01 * b.herd.min(1.0);
            for mm in [Mat::Hide, Mat::Bone] {
                b.stock[mm as usize] = (b.stock[mm as usize] + hb).min(1.5);
            }
            // fleece: tame sheep and goats in cool lands grow wool worth spinning
            if b.tame > 0.4 && self.map.temp_winter(ci) < 10.0 {
                let wl = Mat::Wool as usize;
                b.stock[wl] = b.stock[wl].max(0.8);
                b.origin[wl] = ci as u32;
                if !self.firsts.iter().any(|f| f.item == raw(Mat::Wool)) {
                    fx.first(raw(Mat::Wool), b.id, ci as u32, u32::MAX, EventKind::FirstDomestic, None);
                }
            }
            if b.herd < 0.01 {
                b.herd = 0.0;
                b.tame = 0.0;
                fx.counters.herds_lost += 1;
            }
        }
    }

    /// Crowd diseases: endemic deaths that rise steeply with density, sedentism and herds, and
    /// outbreaks that sweep through people with no immunity.
    pub(super) fn disease_season(&self, b: &mut Band, rng: &mut Rng, fx: &mut Fx) {
        let p = self.params;
        if p.disease <= 0.0 || b.pop == 0 {
            return;
        }
        let ci = b.cell(self.map.w);
        let dens = self.occ_pop.get(ci).copied().unwrap_or(0).max(b.pop) as f32;
        let pop = b.pop as f32;
        let hz = 1.0 + 1.5 * b.herd.min(1.0);
        let hs = 1.2 * (b.settled - 0.15).max(0.0) + 0.3 * b.herd.min(1.0);
        let endemic = p.disease * 0.001 * (dens / 100.0).powf(1.5) * (0.1 + b.settled) * hz * (1.0 - 0.5 * b.immune);
        let mut dead = rng.poisson(pop * endemic);
        if b.sick < 0.05 {
            // outbreaks need crowds that stay put (or live with animals)
            let ch = p.disease * 0.003 * (dens / 80.0 - 0.5).max(0.0) * hs * (1.0 - b.immune);
            if rng.chance(ch) {
                b.sick = rng.range(0.6, 1.0);
                fx.ops.push(Op::Epidemic { band: b.id, from: 0, cell: ci as u32 });
            }
        }
        if b.sick >= 0.05 {
            dead += rng.poisson(pop * 0.06 * b.sick * (1.0 - 0.7 * b.immune));
            b.immune = (b.immune + 0.4 * b.sick * (1.0 - b.immune)).min(1.0);
            b.sick *= 0.5;
            if b.sick < 0.05 {
                b.sick = 0.0;
            }
        }
        b.immune *= 0.997;
        let dead = dead.min(b.pop);
        b.pop -= dead;
        fx.counters.disease_deaths += dead as u64;
        fx.acc.disease_deaths += dead;
    }

    /// A meeting can carry sickness from a band in the middle of an outbreak.
    pub(super) fn spread_disease(&self, bands: &mut [Band], a: usize, c: usize, rng: &mut Rng, fx: &mut Fx) {
        for (x, y) in [(a, c), (c, a)] {
            let s = bands[y].settled;
            if bands[x].sick > 0.15 && bands[y].sick < 0.05 && rng.chance(0.5 * (1.0 - bands[y].immune) * (0.3 + 0.7 * s)) {
                bands[y].sick = 0.8 * bands[x].sick;
                let from = bands[x].id;
                let (yid, yc) = (bands[y].id, bands[y].cell(self.map.w));
                fx.ops.push(Op::Epidemic { band: yid, from, cell: yc as u32 });
            }
        }
    }

    /// `src` may show `dst` how to farm or hand over breeding animals.
    pub(super) fn share_farming(&self, bands: &mut [Band], src: usize, dst: usize, rng: &mut Rng, fx: &mut Fx) {
        let dc = bands[dst].cell(self.map.w);
        let (ssow, sstrain, sgrain) = (bands[src].sow, bands[src].strain, bands[src].stock[Mat::Grain as usize]);
        let dpot = self.crop_pot.get(dc).copied().unwrap_or(0.0);
        // seeing it done is not enough: people take up field work only on good ground, and
        // mostly when foraging is getting hard for them
        let push = if ssow > 0.3 && bands[dst].sow <= 0.0 && dpot > 0.25 {
            (bands[dst].d.stress + self.crowd(dc, &bands[dst]) as f32 / (bands[dst].pop as f32 + 10.0)).min(1.5)
        } else {
            0.0
        };
        if push > 0.0 && bands[dst].d.kit.dig != NONE && rng.chance(0.03 * ssow * dpot * push) {
            let d = &mut bands[dst];
            d.sow = 0.25;
            if sgrain > 0.3 && sstrain > d.strain {
                d.strain = 0.95 * sstrain;
                d.stock[Mat::Grain as usize] = d.stock[Mat::Grain as usize].max(0.5);
            }
            fx.counters.farm_starts += 1;
            fx.counters.took_up_farming += 1;
            let (did, sid) = (d.id, bands[src].id);
            fx.push_event(EventKind::TookUpFarming, did, sid, dc as u32, NONE, u32::MAX);
        }
        let (sh, st) = (bands[src].herd, bands[src].tame);
        if sh > 0.3 && bands[dst].herd <= 0.0 && bands[dst].d.kit.cord != NONE && self.graze_pot.get(dc).copied().unwrap_or(0.0) > 0.6 && rng.chance(0.004) {
            bands[dst].herd = 0.05;
            bands[dst].tame = 0.9 * st;
            fx.counters.herds_started += 1;
        }
    }
}

/// One cell's seasonal change in the farming layers.
#[inline]
#[allow(clippy::too_many_arguments)]
fn field_cell(m: &Map, s: u32, fy: f32, pot: f32, i: usize, camp: &mut f32, tend: &mut f32, dom: &mut f32, fert: &mut f32, crop: &mut f32) {
    if !m.is_land(i) {
        return;
    }
    *camp *= 0.985;
    let td = *tend;
    if td > 0.0 {
        // weeds and scrub take back what is not worked
        let nt = if td > 0.4 { 0.4 + (td - 0.4) * 0.94 } else { td * 0.995 };
        *tend = if nt < 1e-3 { 0.0 } else { nt };
    }
    if *dom > 0.0 && *tend < 0.4 {
        // untended crops go feral
        *dom *= 0.985;
        if *dom < 1e-3 {
            *dom = 0.0;
        }
    }
    let f = *fert;
    if f < 1.0 {
        let river = if m.river[i] { 0.03 } else { 0.0 };
        *fert = (f + (0.006 + river) * (1.0 - f)).min(1.0);
    }
    let cult = cultivated(*tend);
    if cult > 0.0 || *crop > 0.0 {
        let t = m.temp_season(i, s);
        let g = ((t - 6.0) / 12.0).clamp(0.0, 1.0);
        let kc = fy * pot * cult * (0.45 + 0.55 * *dom) * *fert;
        let c = crop;
        if g > 0.15 {
            *c += 0.6 * g * (kc - *c).max(0.0);
        } else {
            *c *= 0.6;
        }
        *c = c.min(kc.max(0.0));
        if *c < 1e-3 {
            *c = 0.0;
        }
    }
}
