//! Competition and conflict between bands (v4). Nothing is scripted: when two bands meet, each
//! weighs hunger, crowding on shared ground, the other's stores and goods, and old grudges. Most
//! meetings stay peaceful. A tense one becomes a confrontation, and the pressing side then
//! backs off (avoid), or the other side flees (displacement), buys peace with a gift (negotiate),
//! or fights. A fight kills people on both sides, thins the skills of whoever loses people, and
//! may end with stores taken, captives brought home (with what they know), the loser driven off,
//! or a broken remnant absorbed. Strength is people times the spears, points and axes each band
//! already makes from its own materials.
use super::*;
use crate::band::weapon_q;

impl World {
    /// How strongly `a` is moved to press `c` when they meet `d` cells apart.
    fn tension(&self, a: &Band, c: &Band, d: i32) -> f32 {
        let overlap = match d {
            0 => 1.0,
            1 => 0.6,
            _ => 0.3,
        };
        let need = a.d.need.max(1.0);
        let hunger = (1.0 - a.d.food_ratio).clamp(0.0, 1.0) + 0.4 * (1.0 - a.store / need).clamp(0.0, 1.0);
        let crowd = (self.crowd(a.cell(self.map.w), a) as f32 / (a.pop as f32 + 10.0)).min(2.0);
        // what the other band has that we lack: food put by, goods we use and are out of
        let lure = (c.store / c.d.need.max(1.0)).clamp(0.0, 2.0) * hunger;
        let mut goods = 0.0f32;
        for &m in TRADE_MATS.iter() {
            let mi = m as usize;
            if a.d.needed & (1 << mi) != 0 && a.stock[mi] < 0.2 && c.stock[mi] > 0.5 {
                goods = 0.3;
            }
        }
        let g = a.grudge_vs(c);
        let ties = a.partner(c.id).map(|p| p.strength).unwrap_or(0.0);
        let kin = if a.close_kin(c) {
            0.2
        } else if a.lineage == c.lineage {
            0.7
        } else {
            1.0
        };
        // crowding alone breeds friction only when food is short; a grudge needs no reason
        let base = overlap * (hunger * (1.0 + 0.5 * crowd) + 0.5 * lure + 0.5 * goods) + 0.6 * g;
        base * (1.0 - 0.8 * ties.min(1.0)) * kin
    }

    /// Fighting strength: people, their weapons, how well fed they are.
    fn power(b: &Band) -> f32 {
        b.pop as f32 * (1.0 + 1.5 * weapon_q(&b.d.kit)) * (0.6 + 0.4 * b.d.food_ratio.min(1.0))
    }

    /// A meeting that may turn hostile. Returns true when it did (no peaceful exchange follows).
    pub(super) fn confront(&mut self, bands: &mut [Band], a0: usize, c0: usize, d: i32, rng: &mut Rng) -> bool {
        let p = self.params;
        if p.aggression <= 0.0 {
            return false;
        }
        let ta = self.tension(&bands[a0], &bands[c0], d);
        let tc = self.tension(&bands[c0], &bands[a0], d);
        let (a, c, t) = if ta >= tc { (a0, c0, ta) } else { (c0, a0, tc) };
        if !rng.chance((0.05 * p.aggression * t).min(0.5)) {
            return false;
        }
        self.year_acc.confrontations += 1;
        self.counters.confrontations += 1;
        let pa = Self::power(&bands[a]);
        let pc = Self::power(&bands[c]) * 1.25; // on their own ground, ready
        let r = pa / pc.max(1.0);
        // the pressing side weighs the odds against how badly it wants this
        let resolve = r * (0.6 + 0.5 * t) * rng.range(0.7, 1.3);
        if resolve < 0.9 {
            self.year_acc.avoided += 1;
            self.counters.avoided += 1;
            // the side that backed off is readier to look for other ground
            bands[a].pressed = (bands[a].pressed + 0.15).min(1.0);
            return true;
        }
        let (aid, cid) = (bands[a].id, bands[c].id);
        let cell = bands[c].cell(self.map.w) as u32;
        // the side being pressed: run, pay, or stand
        let grudge_c = bands[c].grudge_vs(&bands[a]);
        if r > 1.4 && rng.chance(0.5) {
            // flee: leave the ground (and whatever store cannot be carried)
            let left = 0.3 * bands[c].store;
            bands[c].store -= left;
            bands[a].store += left;
            let ag = bands[a].clone();
            bands[c].add_grudge(&ag, 0.25);
            self.counters.fled += 1;
            self.displace(bands, c, a, rng);
            return true;
        }
        let (cs, cn) = (bands[c].store, bands[c].d.need.max(1.0));
        if cs > 0.5 * cn && grudge_c < 0.5 && rng.chance(0.6 * bands[c].d.food_ratio.min(1.0)) {
            // negotiate: food (and a good the other side lacks) as the price of peace
            let gift = 0.25 * cs;
            bands[c].store -= gift;
            bands[a].store += gift;
            for &m in TRADE_MATS.iter() {
                let mi = m as usize;
                if bands[a].stock[mi] < 0.2 && bands[c].stock[mi] > 0.6 {
                    let q = 0.3;
                    bands[c].stock[mi] -= q;
                    bands[a].stock[mi] = (bands[a].stock[mi] + q).min(1.5);
                    break;
                }
            }
            let tick = self.tick;
            bands[a].bond(cid, 0.1, 0.0, tick);
            bands[c].bond(aid, 0.1, 0.0, tick);
            self.year_acc.tributes += 1;
            self.counters.tributes += 1;
            self.push_event(EventKind::Tribute, cid, aid, cell, NONE, u32::MAX);
            return true;
        }
        self.fight(bands, a, c, r, rng);
        true
    }

    fn fight(&mut self, bands: &mut [Band], a: usize, c: usize, r: f32, rng: &mut Rng) {
        let p = self.params;
        let w = self.map.w;
        let ra = r.powf(1.5);
        let a_wins = rng.chance(ra / (1.0 + ra));
        let (win, lose) = if a_wins { (a, c) } else { (c, a) };
        let wq = |b: &Band| weapon_q(&b.d.kit);
        let (qw, ql) = (wq(&bands[win]), wq(&bands[lose]));
        // losers lose more; better weapons on the other side kill more
        let fl = (p.lethality * rng.range(0.05, 0.16) * (0.6 + qw)).min(0.45);
        let fw = (p.lethality * rng.range(0.01, 0.05) * (0.6 + ql)).min(0.2);
        let dl = ((bands[lose].pop as f32 * fl).round() as u32).max(1).min(bands[lose].pop);
        let dw = ((bands[win].pop as f32 * fw).round() as u32).min(bands[win].pop.saturating_sub(1));
        for (k, dead) in [(lose, dl), (win, dw)] {
            let b = &mut bands[k];
            let f = dead as f32 / b.pop.max(1) as f32;
            b.pop -= dead;
            // the dead take what they knew best with them: skills thin, and a thinly held
            // craft may then be forgotten in the usual way
            for kn in &mut b.known {
                if rng.chance(f) {
                    kn.skill *= 1.0 - 0.7 * rng.f32();
                }
            }
            b.d.known_ver += 1;
        }
        let deaths = dl + dw;
        self.year_acc.fights += 1;
        self.year_acc.conflict_deaths += deaths;
        self.counters.fights += 1;
        self.counters.conflict_deaths += deaths as u64;
        // loot: food and goods the winner can use
        let take = rng.range(0.3, 0.6) * bands[lose].store;
        bands[lose].store -= take;
        bands[win].store += take;
        for &m in TRADE_MATS.iter() {
            let mi = m as usize;
            if bands[lose].stock[mi] > 0.2 && (bands[win].d.uses & (1 << mi) != 0) {
                let q = 0.5 * bands[lose].stock[mi];
                bands[lose].stock[mi] -= q;
                bands[win].stock[mi] = (bands[win].stock[mi] + q).min(1.5);
            }
        }
        // whatever ties there were are broken; both remember
        let (wid, lid) = (bands[win].id, bands[lose].id);
        bands[win].partners.retain(|x| x.id != lid);
        bands[lose].partners.retain(|x| x.id != wid);
        let (wb, lb) = (bands[win].clone(), bands[lose].clone());
        bands[lose].add_grudge(&wb, 0.4 + 3.0 * fl);
        bands[win].add_grudge(&lb, 0.1 + 3.0 * fw);
        let cell = bands[lose].cell(w);
        if let Some(f) = self.conflict_flow.get_mut(cell) {
            *f += deaths as f32;
        }
        if let Some(f) = self.conflict_flow.get_mut(bands[win].cell(w)) {
            *f += 0.5 * deaths as f32;
        }
        // the first fight anywhere, and each new bloodiest one, go in the chronicle
        let it = bands[win].d.kit.hunt;
        if self.counters.fights == 1 {
            self.push_event(EventKind::FirstFight, wid, lid, cell as u32, it, deaths);
        } else if deaths >= 10 && deaths as u64 > self.counters.max_fight_deaths {
            self.push_event(EventKind::BloodiestFight, wid, lid, cell as u32, it, deaths);
        } else {
            self.push_event(EventKind::Fight, wid, lid, cell as u32, it, deaths);
        }
        self.counters.max_fight_deaths = self.counters.max_fight_deaths.max(deaths as u64);
        // what becomes of the losers
        let left = bands[lose].pop;
        if left < 8 {
            // a broken remnant joins the winners, bringing what they know
            self.absorb(bands, win, lose, left, rng);
            self.counters.absorbed += 1;
            self.year_acc.absorbed += 1;
            self.push_event(EventKind::Absorbed, lid, wid, cell as u32, NONE, left);
            return;
        }
        if rng.chance(0.35) {
            // captives taken home
            let n = ((left as f32 * rng.range(0.08, 0.2)) as u32).max(1);
            self.absorb(bands, win, lose, n, rng);
            self.counters.captives += n as u64;
        }
        if rng.chance(0.75) {
            self.displace(bands, lose, win, rng);
        } else {
            bands[lose].pressed = (bands[lose].pressed + 0.4).min(1.0);
        }
    }

    /// `n` people of band `from` join band `to`; each recipe they know well enough may come with them.
    fn absorb(&mut self, bands: &mut [Band], to: usize, from: usize, n: u32, rng: &mut Rng) {
        let n = n.min(bands[from].pop);
        bands[from].pop -= n;
        bands[to].pop += n;
        let fid = bands[from].id;
        let share = (n as f32 / 10.0).min(1.0);
        let moved: Vec<Known> = bands[from].known.clone();
        let tick = self.tick;
        let tb = &mut bands[to];
        for kn in moved {
            if !tb.knows(kn.recipe) && rng.chance(kn.skill * 0.5 * share) {
                tb.known.push(Known { skill: kn.skill * 0.6, from: fid, since: tick, kin: false, ..kn });
                tb.d.known_ver += 1;
                self.counters.transfers += 1;
                self.year_acc.transfers += 1;
                tb.learned += 1;
            }
        }
    }

    /// Band `k` is driven off by band `by`: it leaves for the best ground it can find 2-6 cells
    /// away from them, or, with boats and a coast at hand, puts to sea.
    fn displace(&mut self, bands: &mut [Band], k: usize, by: usize, rng: &mut Rng) {
        let m = &self.map;
        let (w, h) = (m.w as i32, m.h as i32);
        let b = &bands[k];
        let (x, y) = (b.x as i32, b.y as i32);
        let (ex, ey) = (bands[by].x as i32, bands[by].y as i32);
        let (ax, ay) = ((x - ex) as f32, (y - ey) as f32);
        let cur = b.cell(m.w);
        let mut best = (cur, f32::MIN);
        for _ in 0..14 {
            let dx = rng.below(13) as i32 - 6;
            let dy = rng.below(13) as i32 - 6;
            if dx.abs().max(dy.abs()) < 2 {
                continue;
            }
            // away from (or at least not towards) the enemy
            if (ax != 0.0 || ay != 0.0) && (dx as f32 * ax + dy as f32 * ay) < 0.0 {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let c = (ny * w + nx) as usize;
            if !m.is_land(c) {
                continue;
            }
            let s = self.cell_score(b, c);
            if s > best.1 {
                best = (c, s);
            }
        }
        let mut sea = None;
        if b.d.kit.boat_q > 0.0 && self.near_water(cur) && rng.chance(0.4 + 0.5 * b.pressed) {
            // a coastal band with boats looks to the water: a known far shore first, then any
            // bearing that does not lead back past the enemy; the first landing found is taken
            for i in 0..4 {
                let aim = if i == 0 { b.far_shore } else { None };
                if i == 0 && aim.is_none() {
                    continue;
                }
                let Some((c, s)) = self.voyage(b, rng, aim) else { continue };
                let (lx, ly) = ((c % m.w) as f32 - x as f32, (c / m.w) as f32 - y as f32);
                if aim.is_none() && (ax != 0.0 || ay != 0.0) && lx * ax + ly * ay < 0.0 {
                    continue;
                }
                // refugees take a poorer landing than settlers would
                let sc = self.cell_score(b, c) / (1.0 + 0.03 * s);
                if sc > 0.6 * best.1 {
                    best = (c, sc);
                    sea = Some(s);
                }
                break;
            }
        }
        let oid = bands[by].id;
        let b = &mut bands[k];
        b.pressed = (b.pressed + 0.6).min(1.0);
        self.counters.displaced += 1;
        self.year_acc.displaced += 1;
        if best.0 == cur {
            return; // nowhere better to go: they stay, wary
        }
        let from = cur as u32;
        b.x = (best.0 % self.map.w) as u16;
        b.y = (best.0 / self.map.w) as u16;
        let mut by_sea = 0;
        if let Some(s) = sea {
            self.counters.voyages += 1;
            self.counters.fled_by_sea += 1;
            self.sea_toll(b, s, rng);
            self.forget_reached_shore(b);
            by_sea = 1;
            if self.counters.fled_by_sea == 1 {
                let (bid, boat) = (b.id, b.d.kit.boat);
                self.push_event(EventKind::FirstSeaRefuge, bid, oid, best.0 as u32, boat, from);
                return;
            }
        }
        let bid = b.id;
        self.push_event(EventKind::Fled, bid, oid, best.0 as u32, NONE, by_sea);
    }

    /// Yearly: grudges fade (half gone in about 8 years at the default memory), pressure eases.
    pub(super) fn fade_grudges(&mut self) {
        let keep = self.params.grudge_memory.clamp(0.0, 1.0);
        for b in &mut self.bands {
            for g in &mut b.grudges {
                g.w *= keep;
            }
            b.grudges.retain(|g| g.w >= 0.05);
        }
    }
}
