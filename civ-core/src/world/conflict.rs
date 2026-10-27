//! Competition and conflict between bands (v4). Nothing is scripted: when two bands meet, each
//! weighs hunger, crowding on shared ground, the other's stores and goods, and old grudges. Most
//! meetings stay peaceful. A tense one becomes a confrontation, and the pressing side then
//! backs off (avoid), or the other side flees (displacement), buys peace with a gift (negotiate),
//! or fights. A fight kills people on both sides, thins the skills of whoever loses people, and
//! may end with stores taken, captives brought home (with what they know), the loser driven off,
//! or a broken remnant absorbed. Strength is people times the spears, points and axes each band
//! already makes from its own materials.
use super::*;
use crate::band::{loss_guard, weapon_q};

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
        } else {
            // more distant relatives quarrel a little less; a shared founder long ago counts for nothing
            1.0 - 0.3 * (2.0 * a.kinship(c, self.tick)).min(1.0)
        };
        // crowding alone breeds friction only when food is short; a grudge needs no reason
        let base = overlap * (hunger * (1.0 + 0.5 * crowd) + 0.5 * lure + 0.5 * goods) + 0.6 * g;
        // (v9) in-laws quarrel less
        let wed = 1.0 - super::MARRIAGE_PEACE * a.married_to(c.id).min(1.0);
        base * (1.0 - 0.8 * ties.min(1.0)) * kin * wed
    }

    /// Fighting strength: people, their weapons, how well fed they are.
    fn power(b: &Band) -> f32 {
        b.pop as f32 * (1.0 + 1.5 * weapon_q(&b.d.kit)) * (0.6 + 0.4 * b.d.food_ratio.min(1.0))
    }

    /// A meeting that may turn hostile. Returns true when it did (no peaceful exchange follows).
    pub(super) fn confront(&self, bands: &mut Pair, a0: usize, c0: usize, d: i32, rng: &mut Rng, fx: &mut Fx) -> bool {
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
        fx.acc.confrontations += 1;
        fx.counters.confrontations += 1;
        // (v8) riders on open land strike harder
        let pa = Self::power(&bands[a]) * (1.0 + 0.3 * self.mounted(&bands[a]));
        let pc = Self::power(&bands[c]) * 1.25 * (1.0 + 0.4 * bands[c].settled) * (1.0 + 0.28 * Self::wall_at(&bands[c], self.map.w)); // on their own ground, ready (behind house walls and field banks, more so)
        let r = pa / pc.max(1.0);
        // the pressing side weighs the odds against how badly it wants this
        // (v9) and the bold press harder
        let resolve = r * (0.6 + 0.5 * t) * rng.range(0.7, 1.3) * (super::AMBITION_RESOLVE * bands[a].ambition).exp();
        if resolve < 0.9 {
            fx.acc.avoided += 1;
            fx.counters.avoided += 1;
            // the side that backed off is readier to look for other ground
            bands[a].pressed = (bands[a].pressed + 0.15).min(1.0);
            return true;
        }
        let (aid, cid) = (bands[a].id, bands[c].id);
        let cell = bands[c].cell(self.map.w) as u32;
        // the side being pressed: run, pay, or stand
        let grudge_c = bands[c].grudge_vs(&bands[a]);
        // people with fields, houses and stores to lose stand their ground more often
        if r > 1.4 && rng.chance(0.5 * (1.0 - 0.3 * bands[c].settled)) {
            // flee: leave the ground (and whatever store cannot be carried)
            let left = 0.3 * bands[c].store;
            bands[c].store -= left;
            bands[a].store += left;
            let ag = bands[a].clone();
            bands[c].add_grudge(&ag, 0.25);
            fx.counters.fled += 1;
            self.displace(bands, c, a, rng, fx);
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
            // (inventions step 8) a debt both sides will remember (and may keep count of)
            bands[a].owed = 1.0;
            bands[c].owed = 1.0;
            fx.acc.tributes += 1;
            fx.counters.tributes += 1;
            fx.push_event(EventKind::Tribute, cid, aid, cell, NONE, u32::MAX);
            return true;
        }
        self.fight(bands, a, c, r, rng, fx);
        true
    }

    fn fight(&self, bands: &mut Pair, a: usize, c: usize, r: f32, rng: &mut Rng, fx: &mut Fx) {
        let p = self.params;
        let w = self.map.w;
        let ra = r.powf(1.5);
        let a_wins = rng.chance(ra / (1.0 + ra));
        let (win, lose) = if a_wins { (a, c) } else { (c, a) };
        if bands[a].married_to(bands[c].id) > 0.2 {
            fx.counters.fights_in_law += 1;
        }
        let wq = |b: &Band| weapon_q(&b.d.kit);
        let (qw, ql) = (wq(&bands[win]), wq(&bands[lose]));
        // losers lose more; better weapons on the other side kill more
        let fl = (p.lethality * rng.range(0.05, 0.16) * (0.6 + qw)).min(0.45);
        let fw = (p.lethality * rng.range(0.01, 0.05) * (0.6 + ql)).min(0.2);
        // defenders in a settled village have walls, banks and houses to fall back behind
        // (v8) and behind a palisade more so; bows and shields cut a side's losses
        let shelter = |k: usize| {
            let home = if k == c { (1.0 - 0.4 * bands[k].settled) * (1.0 - 0.25 * Self::wall_at(&bands[k], w)) } else { 1.0 };
            home * loss_guard(&bands[k].d.kit)
        };
        let (fl, fw) = (fl * shelter(lose), fw * shelter(win));
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
        fx.acc.conflict_deaths += deaths;
        fx.counters.conflict_deaths += deaths as u64;
        // loot: food and goods the winner can use
        let take = rng.range(0.3, 0.6) * bands[lose].store;
        bands[lose].store -= take;
        bands[win].store += take;
        for &m in TRADE_MATS.iter() {
            let mi = m as usize;
            // (grain is food whoever takes it, and seed for anyone who farms)
            if bands[lose].stock[mi] > 0.2 && (bands[win].d.uses & (1 << mi) != 0 || m == Mat::Grain) {
                let q = 0.5 * bands[lose].stock[mi];
                bands[lose].stock[mi] -= q;
                bands[win].stock[mi] = (bands[win].stock[mi] + q).min(1.5);
                if m == Mat::Grain && bands[lose].strain > bands[win].strain {
                    bands[win].strain = 0.9 * bands[lose].strain;
                }
            }
        }
        // herds are driven off with the winners
        if bands[lose].herd > 0.0 && bands[win].pop > 0 {
            let animals = 0.3 * bands[lose].herd * bands[lose].pop as f32;
            bands[lose].herd *= 0.7;
            let (wh, wp) = (bands[win].herd, bands[win].pop as f32);
            let nh = wh + animals / wp.max(1.0);
            bands[win].tame = (wh * bands[win].tame + (animals / wp.max(1.0)) * bands[lose].tame) / nh.max(1e-6);
            bands[win].herd = nh;
        }
        // whatever ties there were are broken; both remember
        let (wid, lid) = (bands[win].id, bands[lose].id);
        bands[win].partners.retain(|x| x.id != lid);
        bands[lose].partners.retain(|x| x.id != wid);
        let (wb, lb) = (bands[win].clone(), bands[lose].clone());
        bands[lose].add_grudge(&wb, 0.4 + 3.0 * fl);
        bands[win].add_grudge(&lb, 0.1 + 3.0 * fw);
        let cell = bands[lose].cell(w);
        fx.ops.push(Op::Conflict(cell as u32, deaths as f32));
        fx.ops.push(Op::Conflict(bands[win].cell(w) as u32, 0.5 * deaths as f32));
        // the first fight anywhere, and each new bloodiest one, go in the chronicle
        let it = bands[win].d.kit.hunt;
        fx.ops.push(Op::Fight { win: wid, lose: lid, cell: cell as u32, item: it, deaths });
        // what becomes of the losers
        let left = bands[lose].pop;
        if left < 8 {
            // a broken remnant joins the winners, bringing what they know
            self.absorb(bands, win, lose, left, rng, fx);
            fx.counters.absorbed += 1;
            fx.acc.absorbed += 1;
            fx.push_event(EventKind::Absorbed, lid, wid, cell as u32, NONE, left);
            return;
        }
        // (v8) archers cover a retreat: fewer are caught
        let catch = if crate::band::has_bow(&bands[lose].d.kit) { 0.15 } else { 0.35 };
        if rng.chance(catch) {
            // captives taken home
            let n = ((left as f32 * rng.range(0.08, 0.2)) as u32).max(1);
            self.absorb(bands, win, lose, n, rng, fx);
            fx.counters.captives += n as u64;
        }
        if rng.chance(0.75) {
            self.displace(bands, lose, win, rng, fx);
        } else {
            bands[lose].pressed = (bands[lose].pressed + 0.4).min(1.0);
        }
    }

    /// `n` people of band `from` join band `to`; each recipe they know well enough may come with them.
    fn absorb(&self, bands: &mut Pair, to: usize, from: usize, n: u32, rng: &mut Rng, fx: &mut Fx) {
        let n = n.min(bands[from].pop);
        bands[from].pop -= n;
        bands[to].pop += n;
        // (v9) they keep their ancestry (but take the winners' name)
        let anc = bands[from].ancestry();
        let f = n as f32 / bands[to].pop.max(1) as f32;
        bands[to].mix_ancestry(&anc, f);
        let fid = bands[from].id;
        let share = (n as f32 / 10.0).min(1.0);
        let moved: Vec<Known> = bands[from].known.clone();
        let tick = self.tick;
        let tb = &mut bands[to];
        for kn in moved {
            // (inventions step 11b) a retired recipe (cold-hammered gold) is not passed on
            if !tb.knows(kn.recipe) && !self.reg.retired(kn.recipe) && rng.chance(kn.skill * 0.5 * share) {
                tb.known.push(Known { skill: kn.skill * 0.6, quality: kn.quality * super::QUALITY_TEACH, from: fid, since: tick, kin: false, ..kn });
                tb.d.known_ver += 1;
                fx.counters.transfers += 1;
                fx.acc.transfers += 1;
                tb.learned += 1;
            }
        }
    }

    /// Band `k` is driven off by band `by`: it leaves for the best ground it can find 2-6 cells
    /// away from them, or, with boats and a coast at hand, puts to sea.
    fn displace(&self, bands: &mut Pair, k: usize, by: usize, rng: &mut Rng, fx: &mut Fx) {
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
            // (v9) refugees on foot cannot cross water either
            if !m.walkable(cur, c) {
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
        fx.counters.displaced += 1;
        fx.acc.displaced += 1;
        if best.0 == cur {
            return; // nowhere better to go: they stay, wary
        }
        let from = cur as u32;
        b.x = (best.0 % self.map.w) as u16;
        b.settled *= 0.3; // fields and houses left behind
        b.y = (best.0 / self.map.w) as u16;
        if let Some(s) = sea {
            b.d.sailed_from = Some(from);
            fx.counters.voyages += 1;
            self.sea_toll(b, s, rng, fx);
            self.note_crossing(from as usize, b, fx);
            self.forget_reached_shore(b);
            let (bid, boat) = (b.id, b.d.kit.boat);
            fx.ops.push(Op::FledBySea { band: bid, by: oid, cell: best.0 as u32, boat, from });
            return;
        }
        let by_sea = 0;
        let bid = b.id;
        fx.push_event(EventKind::Fled, bid, oid, best.0 as u32, NONE, by_sea);
    }

    /// The palisade a band has at the cell it is in now (0 when it has moved away from it).
    pub(super) fn wall_at(b: &Band, w: usize) -> f32 {
        if b.wall_cell as usize == b.cell(w) { b.wall } else { 0.0 }
    }

    /// Yearly (v8): a settled band that has been attacked, and has an axe to cut stakes and a
    /// digging stick for the ditch, raises a palisade. It costs food (labour) every year and rots
    /// when no longer kept up. A wall left behind is no use to the band that left it.
    pub(super) fn build_walls(&mut self) {
        let w = self.map.w;
        for b in &mut self.bands {
            let c = b.cell(w) as u32;
            if b.wall_cell != c {
                b.wall_cell = c;
                b.wall = 0.0;
            }
            let attacked: f32 = b.grudges.iter().map(|g| g.w).sum();
            let k = &b.d.kit;
            if b.settled > 0.5 && attacked >= 0.3 && k.axe != NONE && k.dig != NONE {
                b.wall += 0.15 * (1.0 - b.wall);
            } else {
                b.wall *= 0.85;
            }
            if b.wall < 0.01 {
                b.wall = 0.0;
            }
            let upkeep = 0.05 * b.wall * b.d.need.max(0.0);
            b.store = (b.store - upkeep).max(0.0);
        }
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
