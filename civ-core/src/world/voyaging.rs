//! (inventions) Voyaging: what a crossing party carries with it and the crossing ledger. Step 0
//! only measures (no behaviour or RNG change); later steps add provisions, pilotage, sky lore
//! and ships here.

use super::*;

/// What a crossing party had with it, recorded with each landfall (`LandfallRec::tech`) and
/// carried in `Derived::sailed_tech`. Bits for later inventions are reserved now so the ledger
/// keeps one meaning across versions.
pub const TECH_OUTRIGGER: u16 = 1 << 0;
/// a sail the crew had the seamanship to use (seacraft > 0.4)
pub const TECH_SAIL: u16 = 1 << 1;
pub const TECH_PLANK: u16 = 1 << 2;
/// food put by for the trip (store >= half the people, or a plank boat)
pub const TECH_PROVISIONED: u16 = 1 << 3;
pub const TECH_SHIP: u16 = 1 << 4;
pub const TECH_JAR: u16 = 1 << 5;
pub const TECH_PILOT: u16 = 1 << 6;
pub const TECH_SKY: u16 = 1 << 7;
pub const TECH_ROUTE: u16 = 1 << 8;
/// blown there by a storm while fishing, not a planned voyage
pub const TECH_ADRIFT: u16 = 1 << 15;
pub const TECH_NAMES: [(u16, &str); 10] = [
    (TECH_OUTRIGGER, "outrigger"),
    (TECH_SAIL, "sail"),
    (TECH_PLANK, "plank"),
    (TECH_PROVISIONED, "provisioned"),
    (TECH_SHIP, "ship"),
    (TECH_JAR, "jar"),
    (TECH_PILOT, "pilot"),
    (TECH_SKY, "sky"),
    (TECH_ROUTE, "route"),
    (TECH_ADRIFT, "adrift"),
];

/// The names of the bits set in a `TECH_*` mask.
pub fn tech_names(t: u16) -> Vec<String> {
    TECH_NAMES.iter().filter(|(b, _)| t & b != 0).map(|(_, n)| n.to_string()).collect()
}

/// A crossing below this many open-water cells is never reported as a record (straits).
pub const LEG_RECORD_MIN: f32 = 3.0;

impl World {
    /// What band `b` takes to sea with it on a planned voyage (`TECH_*` bits).
    pub(crate) fn voyage_tech(b: &Band, prepared: bool) -> u16 {
        let k = &b.d.kit;
        let mut t = 0;
        if k.outrigger != NONE {
            t |= TECH_OUTRIGGER;
        }
        if k.sail != NONE && b.seacraft > 0.4 {
            t |= TECH_SAIL;
        }
        if k.plank != NONE {
            t |= TECH_PLANK;
        }
        if prepared {
            t |= TECH_PROVISIONED;
        }
        if k.jar != NONE {
            t |= TECH_JAR;
        }
        t
    }

    /// Book a crossing to another landmass of `sea` open-water cells in the season's counters.
    pub(crate) fn ledger_crossing(sea: f32, fx: &mut Fx) {
        let cells = sea.round().max(0.0) as u64;
        fx.counters.crossing_cells += cells;
        fx.counters.longest_leg = fx.counters.longest_leg.max(cells);
    }

    /// (inventions step 0) After the season's moves: the longest crossing to another landmass
    /// this season, if it beats every one before it, is a major event.
    pub(crate) fn track_leg_record(&mut self) {
        let w = self.map.w;
        let mut best: Option<(f32, u32, u32, u32, Item)> = None;
        for b in &self.bands {
            let Some(f) = b.d.sailed_from else { continue };
            let c = b.cell(w);
            if b.pop == 0 || self.map.landmass_of(f as usize) == self.map.landmass_of(c) {
                continue;
            }
            let leg = b.d.sailed_sea;
            if best.map_or(true, |x| leg > x.0) {
                let boat = if b.d.sailed_tech & TECH_ADRIFT != 0 { NONE } else { b.d.kit.boat };
                best = Some((leg, b.id, c as u32, self.map.landmass_of(c) as u32, boat));
            }
        }
        if let Some((leg, id, c, l, boat)) = best {
            if leg >= LEG_RECORD_MIN && leg > self.leg_record {
                self.leg_record = leg;
                self.push_event(EventKind::LongestCrossing, id, l, c, boat, leg.round() as u32);
            }
        }
    }
}

impl World {
    /// (inventions step 0) The seafaring columns of this year's `YearStats`.
    pub(crate) fn sea_year_stats(&self, ys: &mut YearStats) {
        let mut xs: Vec<f32> = Vec::new();
        let (mut sail, mut rumour) = (0u32, 0u32);
        for b in &self.bands {
            if b.far_shore.is_some() {
                rumour += 1;
            }
            if b.d.kit.boat_q > 0.0 {
                xs.push(b.seacraft);
                if b.d.kit.sail != NONE && b.seacraft > 0.4 {
                    sail += 1;
                }
            }
        }
        ys.boat_bands = xs.len() as u32;
        ys.sail_bands = sail;
        ys.rumour_share = if self.bands.is_empty() { 0.0 } else { rumour as f32 / self.bands.len() as f32 };
        if !xs.is_empty() {
            ys.seacraft_mean = xs.iter().sum::<f32>() / xs.len() as f32;
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys.seacraft_p90 = xs[(xs.len() * 9 / 10).min(xs.len() - 1)];
        }
        ys.leg_record = self.leg_record;
        ys.pitch_bands = self.bands.iter().filter(|b| b.d.kit.pitch != NONE).count() as u32;
        ys.jar_bands = self.bands.iter().filter(|b| b.d.kit.jar != NONE).count() as u32;
    }
}
