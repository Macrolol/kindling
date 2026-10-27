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
/// food put by for the trip in full (store >= half the people, or a plank boat)
pub const TECH_PROVISIONED: u16 = 1 << 3;
pub const TECH_SHIP: u16 = 1 << 4;
pub const TECH_JAR: u16 = 1 << 5;
pub const TECH_PILOT: u16 = 1 << 6;
pub const TECH_SKY: u16 = 1 << 7;
pub const TECH_ROUTE: u16 = 1 << 8;
/// (inventions step 5b) a planned expedition: a ship crew with stores for the trip that sailed
/// past the boat's usual range, as far as its food and water would take it
pub const TECH_EXPEDITION: u16 = 1 << 9;
/// blown there by a storm while fishing, not a planned voyage
pub const TECH_ADRIFT: u16 = 1 << 15;
pub const TECH_NAMES: [(u16, &str); 11] = [
    (TECH_OUTRIGGER, "outrigger"),
    (TECH_SAIL, "sail"),
    (TECH_PLANK, "plank"),
    (TECH_PROVISIONED, "provisioned"),
    (TECH_SHIP, "ship"),
    (TECH_JAR, "jar"),
    (TECH_PILOT, "pilot"),
    (TECH_SKY, "sky"),
    (TECH_ROUTE, "route"),
    (TECH_EXPEDITION, "expedition"),
    (TECH_ADRIFT, "adrift"),
];

/// The names of the bits set in a `TECH_*` mask.
pub fn tech_names(t: u16) -> Vec<String> {
    TECH_NAMES.iter().filter(|(b, _)| t & b != 0).map(|(_, n)| n.to_string()).collect()
}

/// (inventions step 2) Endurance: how many open-water cells a crew can cross before its food
/// and water run out. Everyone has this much (fish caught on the way, rain, a few days' food);
/// it is set so every leg the old boats made routinely (up to 18 cells) is unchanged.
pub const ENDURANCE_BASE: f32 = 18.0;
/// Food that keeps (salted, dried, smoked, cheese, grain, food in sealed jars), put by in full,
/// adds up to this much more...
pub const ENDURANCE_FOOD: f32 = 10.0;
/// ...but only as far as the water lasts: skins and gourds carry this much beyond the base,
pub const WATER_SKINS: f32 = 4.0;
/// sealed jars this much.
pub const WATER_JARS: f32 = 10.0;
/// A plank boat has room for more of both.
pub const ENDURANCE_PLANK: f32 = 4.0;
/// Past endurance the crew starves and thirsts: survival falls as exp(-k * over^2).
pub const ENDURANCE_TOLL: f32 = 0.03;
/// (inventions step 5) a ship's endurance multiplier, and how much a well-built one adds
/// (x (1 + SHIP_QUALITY x quality)): about 1.5x at the usual quality 0.33, 1.68x at best
pub const SHIP_ENDURANCE: f32 = 1.4;
pub const SHIP_QUALITY: f32 = 0.2;
/// (inventions step 5) range multiplier of a ship under a usable sail
pub const SHIP_RANGE: f32 = 1.25;
/// (inventions step 5b) Planned expeditions: a ship crew whose stores feed everyone aboard for
/// the trip (store >= half the people) and whose food keeps this well (`preserved`) can push on
/// as far as its food and water last (its endurance), not just as far as it would row or sail
/// on an ordinary move.
pub const EXPEDITION_PRESERVED: f32 = 0.8;
/// (inventions step 5b) Deep-water seamanship: each open-water cell a ship crew sails past
/// `PILOT_OUT` (out of sight of land) teaches this much seacraft...
pub const DEEP_SEA_LESSON: f32 = 0.004;
/// ...and pilotage is learnt this many times as fast as in a small boat, over the whole leg.
pub const DEEP_PILOT: f32 = 2.0;
/// Drying racks cut spoilage of fresh food (meat and fish) by this share; cold cellars too.
pub const RACK_KEEP: f32 = 0.25;
pub const CELLAR_KEEP: f32 = 0.3;

/// (inventions step 3) Pilotage: at pilot 1, land is read from signs this many cells off.
pub const PILOT_SIGNS: f32 = 10.0;
/// A crew this many cells out has lost sight of land; longer legs teach pilotage...
pub const PILOT_OUT: f32 = 6.0;
/// ...this much per open-water cell crossed (up to `PILOT_CELLS_MAX` cells),
pub const PILOT_PER_CELL: f32 = 0.003;
pub const PILOT_CELLS_MAX: f32 = 25.0;
/// a scouting party that comes home this much, and a drift party that finds its way home (or
/// comes ashore somewhere new) this much.
pub const PILOT_SCOUT: f32 = 0.03;
pub const PILOT_DRIFT: f32 = 0.03;
/// Fishers whose grounds lie out of sight of land (`1 + 3q + 3 q xp` > `PILOT_OUT`) learn this
/// much per season, times their fish share.
pub const PILOT_FISH: f32 = 0.002;
/// Pilotage not used fades by this factor each season (half gone in about 87 years; as seacraft).
pub const PILOT_FADE: f32 = 0.998;
/// A pilot keeps word of a far shore up to (1 + ROUTE_KEEP x pilot) times as long, as sailing
/// directions...
pub const ROUTE_KEEP: f32 = 2.0;
/// ...and tells it to neighbours up to (1 + PILOT_TELL x pilot) times as far off.
pub const PILOT_TELL: f32 = 0.5;
/// (inventions step 9) charts: a band with records and pilotage both at least `CHART_MIN` keeps
/// its sailing directions as a chart (stick chart, recited directions with tally aids). While
/// it still sails (has a boat), its route does not fade, and it teaches the route to the people
/// it meets or visits (partners too) up to `CHART_TELL` times as far and with chance
/// `CHART_TEACH_P` (plain word: 0.3); the hearer gets it as fresh directions.
pub const CHART_MIN: f32 = 0.4;
pub const CHART_TELL: f32 = 1.5;
pub const CHART_TEACH_P: f32 = 0.6;
/// (inventions step 9) `World::farm_flags` bit: the world's first chart has been noted
pub const FLAG_CHART: u32 = 1 << 10;
/// (inventions step 4) Sky lore. Settled farmers (sow and settled above `SKY_FARM_MIN`) learn
/// this much per season, times sow x settled...
pub const SKY_FARM: f32 = 0.003;
pub const SKY_FARM_MIN: f32 = 0.3;
/// ...and crews this much per open-water cell of a leg out of sight of land (nights at sea
/// steered by the stars; up to `PILOT_CELLS_MAX` cells), times pilot / `SKY_PILOT` up to 1.
pub const SKY_PER_CELL: f32 = 0.003;
/// What is not used fades by this factor each season (as seacraft and pilotage).
pub const SKY_FADE: f32 = 0.998;
/// At sea, sky lore counts as far as the crew can use it: fully from this pilotage up, in
/// proportion below it (so sky lore alone steers nobody).
pub const SKY_PILOT: f32 = 0.3;
/// An aimed heading's error shrinks by up to this share (0.15 -> 0.06 for seasoned crews).
pub const SKY_HEADING: f32 = 0.6;
/// Fields sown by the calendar yield up to this much more (the plough-like effort factor).
pub const SKY_SOW: f32 = 0.05;
/// A crossing counts as steered by the stars (ledger `sky` bit) from this effective sky lore.
pub const SKY_TECH_MIN: f32 = 0.2;

/// Signs are looked up only this many steps from land.
const SIGN_MAX: u8 = 12;

/// (inventions step 3) Per cell: 8-neighbour steps to the nearest land (0 on land, 255 beyond
/// `SIGN_MAX`) and that land cell (u32::MAX if none within reach). Derived from the map.
#[derive(Clone, Default)]
pub struct SeaSigns {
    pub dist: Vec<u8>,
    pub near: Vec<u32>,
}

impl SeaSigns {
    pub fn build(m: &Map) -> Self {
        let n = m.w * m.h;
        let mut dist = vec![255u8; n];
        let mut near = vec![u32::MAX; n];
        let mut q = std::collections::VecDeque::new();
        for i in 0..n {
            if m.is_land(i) {
                dist[i] = 0;
                near[i] = i as u32;
                q.push_back(i);
            }
        }
        while let Some(i) = q.pop_front() {
            if dist[i] >= SIGN_MAX {
                continue;
            }
            let (x, y) = (i % m.w, i / m.w);
            for (a, bb) in nbrs8(x, y, m.w, m.h) {
                let j = bb * m.w + a;
                if dist[j] == 255 {
                    dist[j] = dist[i] + 1;
                    near[j] = near[i];
                    q.push_back(j);
                }
            }
        }
        SeaSigns { dist, near }
    }
}

/// (inventions step 5) bit of `World::farm_flags` (world firsts with no item of their own) set
/// once a ship has carried people to another landmass
pub const FLAG_SHIP_CROSSED: u32 = 1 << 8;

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
        if Self::sail_usable(k, b.seacraft) {
            t |= TECH_SAIL;
        }
        if k.plank != NONE {
            t |= TECH_PLANK;
        }
        if k.ship != NONE {
            t |= TECH_SHIP;
        }
        if prepared {
            t |= TECH_PROVISIONED;
        }
        if k.jar != NONE {
            t |= TECH_JAR;
        }
        if Self::sky_at_sea(b) >= SKY_TECH_MIN {
            t |= TECH_SKY;
        }
        t
    }

    /// (inventions step 4) Band `b` learns something of the sky: sky lore moves `gain` of the
    /// way to 1.
    pub(crate) fn learn_sky(b: &mut Band, gain: f32) {
        b.sky = (b.sky + gain * (1.0 - b.sky)).min(1.0);
    }

    /// (inventions step 4) Sky lore as it counts at sea: only as far as the crew has the
    /// pilotage to use it on the water.
    pub fn sky_at_sea(b: &Band) -> f32 {
        b.sky * (b.pilot / SKY_PILOT).min(1.0)
    }

    /// (inventions step 4) Heading error (radians, one sigma) of an aimed course: seasoned
    /// seafarers steer half as wide (v8), and star paths steady it further.
    pub fn heading_error(b: &Band) -> f32 {
        let base = if b.seacraft > 0.6 { 0.15 } else { 0.3 };
        base * (1.0 - SKY_HEADING * Self::sky_at_sea(b))
    }

    /// (inventions step 3) The sea-signs field, built on first use.
    pub fn sea_signs(&self) -> &SeaSigns {
        self.signs.get_or_init(|| SeaSigns::build(&self.map))
    }

    /// (inventions step 3) Band `b` learns something of the signs of land: pilotage moves
    /// `gain` of the way to 1.
    pub(crate) fn learn_pilot(b: &mut Band, gain: f32) {
        b.pilot = (b.pilot + gain * (1.0 - b.pilot)).min(1.0);
    }

    /// (inventions step 9) Whether band `b` keeps a chart: records and pilotage both at least
    /// `CHART_MIN`, and a boat to use it in.
    pub fn charts(b: &Band) -> bool {
        b.records >= CHART_MIN && b.pilot >= CHART_MIN && b.d.kit.boat_q > 0.0
    }

    /// (inventions step 3) Years band `b` keeps word of a far shore nobody has confirmed.
    pub(crate) fn rumour_keep(b: &Band) -> f32 {
        RUMOUR_YEARS as f32 * (1.0 + ROUTE_KEEP * b.pilot)
    }

    /// (inventions step 2) Drying racks: people with an edge and cord, where the air is dry or
    /// freezing in winter, dry meat and fish on racks so it keeps.
    pub(crate) fn dries_food(&self, b: &Band, c: usize) -> bool {
        let k = &b.d.kit;
        k.cut != NONE && k.cord != NONE && (self.map.rain[c] < 0.35 || self.map.temp_winter(c) < 0.0)
    }

    /// (inventions step 2) Cold cellars: settled people with a digging stick, where the winters
    /// freeze hard, dig a pit below the frost line that stays cold into summer.
    pub(crate) fn cold_cellar(&self, b: &Band, c: usize) -> bool {
        b.settled > 0.5 && b.d.kit.dig != NONE && self.map.temp_winter(c) < -5.0
    }

    /// (inventions step 2) How well band `b`'s food keeps on a trip (0..1): the best of what it
    /// has (a cellar does not travel).
    pub(crate) fn preserved(&self, b: &Band, home: usize) -> f32 {
        let k = &b.d.kit;
        let mut p: f32 = 0.0;
        if b.stock.get(Mat::Salt as usize).copied().unwrap_or(0.0) > 0.1 || self.dries_food(b, home) {
            p = 1.0;
        }
        if b.d.farm_share > 0.3 {
            p = p.max(1.0); // dry grain
        }
        if k.jar != NONE {
            p = p.max(0.8);
        }
        if b.tame > 0.8 && b.herd > 0.1 && k.vessel != NONE && mat(k.vessel) == Mat::Ceramic {
            p = p.max(0.8); // cheese
        }
        if k.smoke {
            p = p.max(0.6);
        }
        p
    }

    /// (inventions step 2) Band `b`'s provisions for a trip from `home`: the share (0..1) of
    /// the food a trip wants that is put by (half a season's food per person; a plank boat
    /// always has room for it), and its endurance in open-water cells.
    pub(crate) fn provisions(&self, b: &Band, home: usize) -> (f32, f32) {
        let k = &b.d.kit;
        let plank = k.plank != NONE;
        let fed = if plank { 1.0 } else { (b.store / (0.5 * b.pop.max(1) as f32)).clamp(0.0, 1.0) };
        let food = ENDURANCE_FOOD * self.preserved(b, home) * fed;
        let water = if k.jar != NONE { WATER_JARS } else { WATER_SKINS };
        let e = ENDURANCE_BASE + food.min(water) + if plank { ENDURANCE_PLANK } else { 0.0 };
        (fed, e * Self::ship_endurance(k))
    }

    /// (inventions step 5b) Whether a sail is woven matting of plant fibre rather than cloth.
    pub fn mat_sail(sail: Item) -> bool {
        sail != NONE && mat(sail) == Mat::Fibre
    }

    /// (v8) Whether a band can use its sail: people who know the sea well enough to use the wind
    /// (seacraft above 0.4); (inventions step 5b) a mat sail only on a plank hull or a ship, since
    /// a dugout or canoe cannot carry a mast and a heavy mat.
    pub fn sail_usable(k: &Kit, xp: f32) -> bool {
        k.sail != NONE && xp > 0.4 && (!Self::mat_sail(k.sail) || k.plank != NONE || k.ship != NONE)
    }

    /// (inventions step 5b) How far a planned expedition can go: a ship crew whose stores feed
    /// everyone aboard (store >= half the people) with food that keeps (`EXPEDITION_PRESERVED`)
    /// sails as far as its food and water last. `None` for anyone else.
    pub(crate) fn expedition_reach(&self, b: &Band) -> Option<f32> {
        if b.d.kit.ship == NONE || b.store < 0.5 * b.pop.max(1) as f32 {
            return None;
        }
        let home = b.cell(self.map.w);
        if self.preserved(b, home) < EXPEDITION_PRESERVED {
            return None;
        }
        Some(self.provisions(b, home).1)
    }

    /// (inventions step 5) A ship's hold carries a bigger party's food and water: endurance x
    /// `SHIP_ENDURANCE`, and a well-built one (recipe quality q) x (1 + `SHIP_QUALITY` x q) more.
    pub fn ship_endurance(k: &Kit) -> f32 {
        if k.ship == NONE {
            1.0
        } else {
            SHIP_ENDURANCE * (1.0 + SHIP_QUALITY * k.ship_q.clamp(0.0, 1.0))
        }
    }

    /// (inventions step 2) Survival factor for a leg of `sea` cells by a crew whose food and
    /// water last `endurance` cells: 1 within it, falling steeply beyond.
    pub fn endurance_factor(sea: f32, endurance: f32) -> f32 {
        let over = (sea - endurance).max(0.0);
        (-ENDURANCE_TOLL * over * over).exp()
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
        // (inventions step 5) the world's first crossing to another landmass made by a ship
        if self.farm_flags & FLAG_SHIP_CROSSED == 0 {
            let first = self.bands.iter().find(|b| {
                b.pop > 0
                    && b.d.sailed_tech & TECH_SHIP != 0
                    && b.d.sailed_tech & TECH_ADRIFT == 0
                    && b.d.sailed_from.is_some_and(|f| self.map.landmass_of(f as usize) != self.map.landmass_of(b.cell(w)))
            });
            if let Some(b) = first {
                let (id, c, ship, leg) = (b.id, b.cell(w), b.d.kit.ship, b.d.sailed_sea);
                self.farm_flags |= FLAG_SHIP_CROSSED;
                self.push_event(EventKind::FirstShip, id, self.map.landmass_of(c) as u32, c as u32, ship, leg.round() as u32);
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
        let (mut sail, mut rumour, mut mat_sail) = (0u32, 0u32, 0u32);
        for b in &self.bands {
            if b.far_shore.is_some() {
                rumour += 1;
            }
            if b.d.kit.boat_q > 0.0 {
                xs.push(b.seacraft);
                if Self::sail_usable(&b.d.kit, b.seacraft) {
                    sail += 1;
                }
                if b.d.kit.sail != NONE && Self::mat_sail(b.d.kit.sail) {
                    mat_sail += 1;
                }
            }
        }
        ys.boat_bands = xs.len() as u32;
        ys.sail_bands = sail;
        ys.mat_sail_bands = mat_sail;
        ys.rumour_share = if self.bands.is_empty() { 0.0 } else { rumour as f32 / self.bands.len() as f32 };
        if !xs.is_empty() {
            ys.seacraft_mean = xs.iter().sum::<f32>() / xs.len() as f32;
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys.seacraft_p90 = xs[(xs.len() * 9 / 10).min(xs.len() - 1)];
        }
        ys.leg_record = self.leg_record;
        ys.pitch_bands = self.bands.iter().filter(|b| b.d.kit.pitch != NONE).count() as u32;
        ys.jar_bands = self.bands.iter().filter(|b| b.d.kit.jar != NONE).count() as u32;
        let w = self.map.w;
        let mut ps: Vec<f32> = self.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).map(|b| b.pilot).collect();
        ys.pilot_bands = self.bands.iter().filter(|b| b.pilot > 0.1).count() as u32;
        // (inventions step 9) bands keeping a chart, and those of them with a route on it
        ys.chart_bands = self.bands.iter().filter(|b| Self::charts(b)).count() as u32;
        ys.charted_routes = self.bands.iter().filter(|b| b.far_shore.is_some() && Self::charts(b)).count() as u32;
        if !ps.is_empty() {
            ys.pilot_mean = ps.iter().sum::<f32>() / ps.len() as f32;
            ps.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys.pilot_p90 = ps[(ps.len() * 9 / 10).min(ps.len() - 1)];
        }
        ys.sky_bands = self.bands.iter().filter(|b| b.sky > 0.1).count() as u32;
        let mean = |v: Vec<f32>| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
        ys.sky_farm_mean = mean(self.bands.iter().filter(|b| b.sow > 0.0).map(|b| b.sky).collect());
        ys.sky_sea_mean = mean(self.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).map(|b| b.sky).collect());
        ys.sky_at_sea_mean = mean(self.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).map(Self::sky_at_sea).collect());
        ys.ship_bands = self.bands.iter().filter(|b| b.d.kit.ship != NONE).count() as u32;
        ys.ship_q_mean = mean(self.bands.iter().filter(|b| b.d.kit.ship != NONE).map(|b| b.d.kit.ship_q).collect());
        ys.dried_bands = self.bands.iter().filter(|b| self.dries_food(b, b.cell(w))).count() as u32;
        ys.cellar_bands = self.bands.iter().filter(|b| self.cold_cellar(b, b.cell(w))).count() as u32;
        let mut es: Vec<f32> = self.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).map(|b| self.provisions(b, b.cell(w)).1).collect();
        if !es.is_empty() {
            es.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys.endurance_p50 = es[es.len() / 2];
            ys.endurance_p90 = es[(es.len() * 9 / 10).min(es.len() - 1)];
        }
    }
}
