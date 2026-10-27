//! (inventions step 10) Feasts and ale. A band with food stored beyond what it needs may host a
//! neighbour it meets at a feast, giving part of that surplus away. Standing comes from what is
//! actually given (~ the square root of the food, so a big, rich band stands out without one
//! band taking all), and fades as before. Ale, grain left to work in a sealed jar, is brewed by
//! grain farmers where the warm season is warm enough: it makes a little more of the grain and
//! a feast with ale draws twice the crowd. Nothing is capped, dated or boosted.

use super::*;

/// A host keeps this many seasons of its own need in store; only what lies beyond is surplus.
pub const FEAST_KEEP: f32 = 1.0;
/// Share of the surplus given at one feast.
pub const FEAST_SHARE: f32 = 0.25;
/// The guests eat at most this many seasons of their own need.
pub const FEAST_EAT: f32 = 1.0;
/// Smallest gift (person-seasons of food) worth calling a feast.
pub const FEAST_MIN: f32 = 1.0;
/// Standing for giving food, at a feast or to the hungry: this x sqrt(food given).
pub const PRESTIGE_GIVE: f32 = 0.004;
/// Tie between host and guest from a feast (x the draw).
pub const FEAST_TIE: f32 = 0.1;
/// A feast with ale draws this many times the crowd: standing and tie x this.
pub const ALE_DRAW: f32 = 2.0;
/// Brewing makes this much more of the grain (its food value x(1 + this)).
pub const ALE_FOOD: f32 = 0.05;
/// Ale is brewed only where the warmest season reaches this (deg C).
pub const ALE_WARM: f32 = 18.0;
/// `farm_flags` bit: the world's first feast has been reported.
pub const FLAG_FEAST: u32 = 1 << 11;

/// Pearson correlation of two equal-length series (0 when either does not vary).
pub fn pearson(x: &[f64], y: &[f64]) -> f32 {
    let n = x.len().min(y.len());
    if n < 3 {
        return 0.0;
    }
    let (mx, my) = (x[..n].iter().sum::<f64>() / n as f64, y[..n].iter().sum::<f64>() / n as f64);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (dx, dy) = (x[i] - mx, y[i] - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return 0.0;
    }
    (sxy / (sxx * syy).sqrt()) as f32
}

/// Standing earned for giving `food` person-seasons away (x `draw` for a feast with ale).
pub fn give_prestige(food: f32, draw: f32) -> f32 {
    PRESTIGE_GIVE * food.max(0.0).sqrt() * draw
}

impl World {
    /// Whether the warmest season at cell `c` is warm enough for grain to work into ale.
    pub fn ale_warm(&self, c: usize) -> bool {
        self.map.temp.get(c).copied().unwrap_or(0.0) + self.map.tamp.get(c).copied().unwrap_or(0.0) >= ALE_WARM
    }
    /// A band brews: it knows ale, sows grain and has seed in store, in warm enough country.
    pub fn brews(&self, b: &Band) -> bool {
        b.d.kit.ale != NONE && b.sow > 0.0 && b.stock[Mat::Grain as usize] > 0.05 && self.ale_warm(b.cell(self.map.w))
    }
    /// Food a band could give at a feast: what it has stored beyond `FEAST_KEEP` seasons of its
    /// need, while it is eating well (0 otherwise).
    pub fn feast_surplus(b: &Band) -> f32 {
        if b.pop == 0 || b.d.food_ratio < 1.0 {
            return 0.0;
        }
        (b.store - FEAST_KEEP * b.d.need.max(1.0)).max(0.0)
    }

    /// Two neighbours at peace: the one with more to spare may hold a feast for the other.
    pub(crate) fn feast(&self, bands: &mut Pair, a: usize, c: usize, fx: &mut Fx) {
        let (sa, sc) = (Self::feast_surplus(&bands[a]), Self::feast_surplus(&bands[c]));
        let (x, y, sur) = if sa >= sc { (a, c, sa) } else { (c, a, sc) };
        if sur <= 0.0 {
            return;
        }
        let give = (FEAST_SHARE * sur).min(FEAST_EAT * bands[y].d.need.max(1.0));
        if give < FEAST_MIN {
            return;
        }
        let ale = self.brews(&bands[x]);
        let draw = if ale { ALE_DRAW } else { 1.0 };
        bands[x].store -= give;
        bands[y].store += give;
        bands[x].prestige += give_prestige(give, draw);
        let (ix, iy, tick) = (bands[x].id, bands[y].id, self.tick);
        bands[x].bond(iy, FEAST_TIE * draw, 0.0, tick);
        bands[y].bond(ix, FEAST_TIE * draw, 0.0, tick);
        fx.counters.feasts += 1;
        if ale {
            fx.counters.ale_feasts += 1;
        }
        fx.acc.feasts += 1;
        fx.acc.feast_food += give;
        let cell = bands[x].cell(self.map.w) as u32;
        fx.ops.push(Op::Feast { host: ix, guest: iy, cell, food: give, ale });
    }

    /// (history store) a feast; the first one in the world is a major event.
    pub(crate) fn note_feast(&mut self, host: u32, guest: u32, cell: u32, food: f32, ale: bool) {
        let it = if ale { item(Form::Jar, Mat::Grain) } else { NONE };
        let food = food.round().max(0.0) as u32;
        if self.farm_flags & FLAG_FEAST == 0 {
            self.farm_flags |= FLAG_FEAST;
            self.push_event(EventKind::FirstFeast, host, guest, cell, it, food);
        }
        let e = Event { tick: self.tick, kind: EventKind::Feast as u8, band: host, other: guest, cell, item: it, recipe: food };
        self.archive.note(&e);
    }

    /// Year-end feast, ale and prestige figures.
    pub(crate) fn feast_year_stats(&self, ys: &mut YearStats) {
        ys.ale_bands = self.bands.iter().filter(|b| b.d.kit.ale != NONE).count() as u32;
        ys.brew_bands = self.bands.iter().filter(|b| self.brews(b)).count() as u32;
        let p: Vec<f64> = self.bands.iter().map(|b| b.prestige.max(0.0) as f64).collect();
        let n: Vec<f64> = self.bands.iter().map(|b| b.pop as f64).collect();
        ys.prestige_size_r = pearson(&p, &n);
        let tot: f64 = p.iter().sum();
        ys.prestige_top_pct = if tot > 0.0 { (100.0 * p.iter().cloned().fold(0.0, f64::max) / tot) as f32 } else { 0.0 };
        ys.prestige_gini = crate::analysis::gini(&self.bands.iter().map(|b| b.prestige).collect::<Vec<_>>());
    }
}
