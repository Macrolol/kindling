//! The world and its seasonal step.
//!
//! Each season: renewables regrow -> each band forages with what it can make (food, spare time,
//! stress) -> births / deaths -> practice / forgetting -> fire keeping -> experiments (invention) ->
//! moving -> contacts between nearby bands (teaching / copying, fire sharing) -> splits, merges and
//! deaths -> statistics.
use crate::archive::{Archive, EventLog, Frame, LineageRow, LoreRow, RegionRow, GRID};
use crate::band::{make_kit_for, refine_kit, Band, Grudge, Kit, Partner, CRAFT_LEN, KIT_TRIAL_P};
use crate::knowledge::*;
use crate::map::*;
use crate::materials::*;
use crate::params::{Config, ParamChange, Params, StartCheck};
use crate::physics::{self, Action, Ctx, Inputs};
use crate::rng::{hash3, Rng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod cell_first;
pub use cell_first::CellFirst;
mod conflict;
pub(crate) mod fx;
use fx::{Fx, Layer, Op, Pair};
/// Most meetings a band has time for in one season (v5).
const MAX_MEETINGS: u8 = 4;
pub mod farming;
pub mod terrain;
pub mod voyaging;
pub use voyaging::*;
pub mod feasting;
pub use feasting::*;
pub use farming::{crop_potential, graze_potential};

/// Size of the old single event ring (formats 1-9); now `archive::RING_CAPS` per class.
pub const EVENT_CAP: usize = 40_000;
/// (v9) Years a rumour of a far shore lasts without anyone seeing that land again.
pub const RUMOUR_YEARS: u32 = 50;
/// (v9) Seamanship: a completed voyage moves seacraft this share of the way to 1, plus this
/// much per cell of open water crossed (up to 10 cells).
pub const VOYAGE_LESSON: f32 = 0.03;
pub const VOYAGE_LESSON_PER_CELL: f32 = 0.01;
/// (v9) ... a scouting party that comes home, and a visit to partners across the water.
pub const SCOUT_LESSON: f32 = 0.03;
pub const SEA_VISIT_LESSON: f32 = 0.02;
/// (v9) Chance per meeting that a better seafarer teaches a coastal band with boats (doubled
/// between kin), and the share of the gap closed.
pub const SEA_TEACH_P: f32 = 0.1;
pub const SEA_TEACH_SHARE: f32 = 0.25;
/// (inventions step 1) upkeep of boats, outriggers, sails and plank hulls caulked with pitch
pub const PITCH_CAULK: f32 = 0.6;
/// (inventions step 1) upkeep of a hafted weapon or axe glued with pitch
pub const PITCH_GLUE: f32 = 0.9;
/// (inventions step 1) spoilage of stored food in sealed jars, and the extra store they hold
/// (per person)
pub const JAR_SPOIL: f32 = 0.8;
/// (inventions step 8) records: a season's growth (share of the way to 1) while a band keeps
/// count with a mark; the seasonal fade; and what full records do: stored food spoils this
/// share less, unused recipes are forgotten this share more slowly, and a trade tie keeps
/// `0.8 + RECORDS_TIE` of its strength a year instead of 0.8
pub const RECORDS_GAIN: f32 = 0.04;
pub const RECORDS_FADE: f32 = 0.97;
pub const RECORDS_SPOIL: f32 = 0.05;
pub const RECORDS_FORGET: f32 = 0.25;
pub const RECORDS_TIE: f32 = 0.1;
/// (inventions step 8b) close ties. A band's yearly care for its ties is a budget of
/// `TIE_CARE` (widened by `RECORDS_SPAN` x records), shared out by how much each tie already
/// means (trade strength + marriage + kinship): a tie's share of full upkeep is
/// `TIE_CARE * w / sum(w^2)`, at most 1, so strong ties are kept up in full and reinforce
/// themselves, while loose ones get little care and fade towards `LOOSE_KEEP` a year (they
/// survive a year or two unless used). No cap and no set number of close ties.
pub const TIE_CARE: f32 = 4.0;
pub const RECORDS_SPAN: f32 = 0.5;
pub const LOOSE_KEEP: f32 = 0.3;
/// (inventions step 8b) a trade tie this strong counts as close: food for the hungry and
/// gifts pass along it (as between kin)
pub const CLOSE_TIE: f32 = 0.3;
/// (inventions step 8) what is worth keeping count of: a store of at least this many seasons'
/// food, at least `RECORDS_PARTNERS` partners with ties of `RECORDS_PARTNER_TIE` or more, a
/// herd, or tribute owed or owing (`Band::owed` above 0.2)
pub const RECORDS_STORE: f32 = 2.0;
pub const RECORDS_PARTNERS: usize = 4;
pub const RECORDS_PARTNER_TIE: f32 = 0.5;
/// (inventions step 8) `FirstRecords` fires when a band's records first reach this
pub const RECORDS_FIRST: f32 = 0.3;
/// (inventions step 8) `farm_flags` bit: the first band keeping records has been reported
pub const FLAG_RECORDS: u32 = 1 << 9;
pub const JAR_STORE: f32 = 0.2;
/// (v9) Ambition (log scale) is clamped to +-this, drifts by this sd at each split, and founders
/// start with this sd.
pub const AMBITION_MAX: f32 = 1.0;
pub const AMBITION_DRIFT: f32 = 0.12;
pub const AMBITION_FOUNDER_SD: f32 = 0.3;
/// (v9) How far past its boats' safe range a bold crew will push (range x e^(this x ambition)),
/// and how much harder it presses a quarrel (resolve x e^(this x ambition)).
pub const AMBITION_REACH: f32 = 0.25;
pub const AMBITION_RESOLVE: f32 = 0.15;
/// (v9) Dissent follows the band's friction at this rate a season; above `DISSENT_SPLIT` a band
/// of at least `DISSENT_MIN_SIZE` x split size may break up early (chance `DISSENT_SPLIT_P` x the
/// excess a season). Those who leave in anger go up to `DISSENT_RADIUS` cells, and take the
/// bolder, more curious tail (`EMIGRANT_TAIL`: shift in log units per sd).
pub const DISSENT_RATE: f32 = 0.1;
pub const DISSENT_SPLIT: f32 = 0.5;
pub const DISSENT_MIN_SIZE: f32 = 0.6;
pub const DISSENT_SPLIT_P: f32 = 0.5;
pub const DISSENT_RADIUS: i32 = 4;
pub const EMIGRANT_TAIL: f32 = 0.1;
/// (v9) Wanderlust: a cell nobody has camped in (or not for `NOVELTY_YEARS`) looks better by up
/// to `NOVELTY_PULL` x curiosity x youth (pop / split size, capped at 1), never more than
/// `NOVELTY_MAX`. A curious crew that has heard of a far shore and knows the sea takes ship
/// more often (`FAR_SHORE_LURE` x (curiosity - 1) x seacraft a season).
pub const NOVELTY_YEARS: f32 = 100.0;
pub const NOVELTY_PULL: f32 = 0.15;
pub const NOVELTY_MAX: f32 = 0.3;
pub const FAR_SHORE_LURE: f32 = 0.1;
/// (v9) Marriage: at a peaceful meeting of two bands of 10+ people, chance `MARRIAGE_P` that
/// `MARRIAGE_SHARE` of the smaller band's people (at least 1) marry each way. Each wedding adds
/// `MARRIAGE_TIE` to both sides' marriage ties (which fade x`MARRIAGE_FADE` a year), eases old
/// grudges between them (x(1 - `MARRIAGE_FORGIVE`)), and in-laws quarrel less (tension
/// x(1 - `MARRIAGE_PEACE` x ties)).
pub const MARRIAGE_P: f32 = 0.1;
pub const MARRIAGE_SHARE: f32 = 0.03;
pub const MARRIAGE_TIE: f32 = 0.25;
pub const MARRIAGE_FADE: f32 = 0.95;
pub const MARRIAGE_FORGIVE: f32 = 0.5;
pub const MARRIAGE_PEACE: f32 = 0.5;
/// (v9) Play and refinement: each season a band makes Poisson(`REFINE_RATE` x curiosity x
/// (0.1 + spare labour) x sqrt(pop / 25)) attempts to improve a recipe it used, each raising
/// that recipe's quality by `REFINE_STEP` of the way to its skill (diminishing returns; a recipe
/// is never done better than it is known). Quality fades x(1 - `QUALITY_FADE` x the forgetting
/// rate) a season, so small bands lose finesse first. Learners take `QUALITY_TEACH` of the
/// teacher's quality with a new recipe; at a meeting a band may also pick up some of a better
/// way to make something it already makes (`QUALITY_SHOW_P`, closing `QUALITY_SHOW` of the gap).
/// Daughters keep `QUALITY_SPLIT` of it.
pub const REFINE_RATE: f32 = 2.0;
pub const REFINE_STEP: f32 = 0.15;
pub const QUALITY_FADE: f32 = 0.5;
pub const QUALITY_TEACH: f32 = 0.5;
pub const QUALITY_SHOW_P: f32 = 0.3;
pub const QUALITY_SHOW: f32 = 0.3;
pub const QUALITY_SPLIT: f32 = 0.9;
/// (v9) Prestige: x`PRESTIGE_FADE` a season, plus, for a band that makes ornaments,
/// `PRESTIGE_FINE` x (0.5 + the ornament recipe's quality) a season (step 10b: x sqrt(size /
/// `ORNAMENT_SEEN`), see `ornament_prestige`); `PRESTIGE_VOYAGE` for
/// coming ashore alive on another landmass. (inventions step 10) Giving food, at a feast or to
/// the hungry, earns `PRESTIGE_GIVE` x the food given (see `feasting`); this replaced the
/// share-of-spare-labour term (`PRESTIGE_SURPLUS`) and the flat gift (`PRESTIGE_GIFT`).
/// A band learns from a more prestigious one more readily (chance
/// x(1 + `PRESTIGE_LEARN` x difference), within 0.5..2), weddings are likelier with prestigious
/// bands (x(1 + `PRESTIGE_MARRY` x their mean)), and the ambitious take ship for glory more
/// often (`PRESTIGE_LURE` x ambition a season, for those who know of a far shore).
pub const PRESTIGE_FADE: f32 = 0.99;
pub const PRESTIGE_FINE: f32 = 0.01;
pub const PRESTIGE_VOYAGE: f32 = 1.0;
pub const PRESTIGE_LEARN: f32 = 0.5;
pub const PRESTIGE_MARRY: f32 = 0.3;
pub const PRESTIGE_LURE: f32 = 0.05;

/// (v9) How much more readily `dst` learns from `src` given their prestige (0.5..2).
pub fn prestige_bias(src: &Band, dst: &Band) -> f32 {
    (1.0 + PRESTIGE_LEARN * (src.prestige - dst.prestige)).clamp(0.5, 2.0)
}

/// Straight-line distance between two cells, in cells.
pub fn dist_cells(a: usize, b: usize, w: usize) -> f32 {
    let (dx, dy) = ((a % w) as f32 - (b % w) as f32, (a / w) as f32 - (b / w) as f32);
    (dx * dx + dy * dy).sqrt()
}

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
    /// first people ever to reach a landmass (other = landmass id, 1 = largest)
    Landfall,
    /// an item nobody can make any more, but one only ever known to a few bands (minor)
    LostLocal,
    /// first time a good passed from one band to another in trade
    FirstTraded,
    /// people lost on the water (other = how many died; a whole band if it ended)
    LostAtSea,
    /// a fishing party blown out to sea came ashore somewhere new (other = new band id)
    Drifted,
    /// v4 conflict: band (winner) fought other (loser); recipe = people killed, item = winner's weapon
    Fight,
    /// the world's first fight between bands (major)
    FirstFight,
    /// a fight deadlier than any before it (major; 10+ dead)
    BloodiestFight,
    /// band was driven off by other (recipe = 1 if it fled by sea)
    Fled,
    /// band bought peace from other with food or goods
    Tribute,
    /// what was left of band after a lost fight joined other (recipe = how many)
    Absorbed,
    /// first band to flee a stronger neighbour across the sea (major; recipe = cell left)
    FirstSeaRefuge,
    // ---- farming stage (v5)
    /// the world's first band to sow and tend fields (major)
    FirstCultivation,
    /// farming begun again, independently, far from any earlier start (major; recipe = origin no.)
    FarmingOrigin,
    /// band took up farming (learned from other, or found it near an earlier start)
    TookUpFarming,
    /// band gave up its fields and went back to foraging
    AbandonedFarming,
    /// first fully domesticated crop / fibre / fleece (major; item = raw grain / flax / wool)
    FirstDomestic,
    /// the world's first band to keep and breed animals (major)
    FirstHerding,
    /// first outbreak of a crowd disease (major)
    FirstEpidemic,
    /// an outbreak of crowd disease in band (other = band it caught it from, 0 = arose there)
    Epidemic,
    /// first band to till with an animal-drawn ard (major)
    FirstPlough,
    /// (v8) first band to ride its herd animals (major)
    FirstRiding,
    /// (v10, history store only) band and other exchanged spouses (cell = band's); kept in
    /// `Archive::marriages`, never in the event log
    Married,
    /// (inventions step 0) a crossing to another landmass over more open water than any before
    /// it (major; other = landmass reached, recipe = open-water cells, item = the boat)
    LongestCrossing,
    /// (inventions step 5) the world's first crossing to another landmass by ship (major;
    /// other = landmass reached, recipe = open-water cells, item = the ship)
    FirstShip,
    /// (inventions step 8) the world's first band to keep records (records >= `RECORDS_FIRST`;
    /// major; item = its counting mark)
    FirstRecords,
    /// (inventions step 9) the world's first band to keep a chart (records and pilotage both
    /// >= `CHART_MIN`, with a boat; major; other = landmass of the far shore on it, or u32::MAX)
    FirstChart,
    /// (inventions step 10) the world's first feast (major; band = host, other = guest, item =
    /// ale if poured, recipe = food given, person-seasons rounded)
    FirstFeast,
    /// (inventions step 10, history store only) a feast: band hosted other (same fields as
    /// `FirstFeast`); tallied in `Archive`, never in the event log
    Feast,
}
impl EventKind {
    pub fn from_u8(x: u8) -> EventKind {
        use EventKind::*;
        [
            FirstInvention, Invention, Learned, Forgot, LostWorldwide, FireCaptured, FireLost, Split, Merged, Died, Founded,
            FirstFire, Landfall, LostLocal, FirstTraded, LostAtSea, Drifted, Fight, FirstFight, BloodiestFight, Fled, Tribute,
            Absorbed, FirstSeaRefuge, FirstCultivation, FarmingOrigin, TookUpFarming, AbandonedFarming, FirstDomestic,
            FirstHerding, FirstEpidemic, Epidemic, FirstPlough, FirstRiding, Married, LongestCrossing, FirstShip, FirstRecords,
            FirstChart, FirstFeast, Feast,
        ][(x as usize).min(Self::COUNT - 1)]
    }
    /// number of kinds (the last one is `COUNT - 1`)
    pub const COUNT: usize = 41;
    pub fn major(self) -> bool {
        matches!(
            self,
            EventKind::FirstInvention
                | EventKind::LostWorldwide
                | EventKind::FirstFire
                | EventKind::Landfall
                | EventKind::FirstTraded
                | EventKind::FirstFight
                | EventKind::BloodiestFight
                | EventKind::FirstSeaRefuge
                | EventKind::FirstCultivation
                | EventKind::FarmingOrigin
                | EventKind::FirstDomestic
                | EventKind::FirstHerding
                | EventKind::FirstEpidemic
                | EventKind::FirstPlough
                | EventKind::FirstRiding
                | EventKind::LongestCrossing
                | EventKind::FirstShip
                | EventKind::FirstRecords
                | EventKind::FirstChart
                | EventKind::FirstFeast
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// exchanges this year (one good passed one way counts once)
    pub trades: u32,
    /// stock units of goods passed between bands
    pub trade_volume: f32,
    /// food given to hungry neighbours (person-seasons)
    pub food_shared: f32,
    /// summed band-to-band distance of trades (cells); mean = trade_dist / trades
    pub trade_dist: f32,
    /// standing partnerships (pairs) at year end
    pub partnerships: u32,
    /// carried copper/tin/bronze/obsidian: mean and max distance (cells) from where it was gathered
    pub goods_dist_mean: f32,
    pub goods_dist_max: f32,
    /// conflict (v4): tense meetings that became confrontations, and how they ended
    pub confrontations: u32,
    pub avoided: u32,
    pub tributes: u32,
    pub fights: u32,
    pub conflict_deaths: u32,
    /// bands driven off (fled, or lost a fight and left)
    pub displaced: u32,
    pub absorbed: u32,
    /// farming (v5): bands that sow fields, their people, bands keeping herds
    pub farmers: u32,
    pub farmer_pop: u32,
    pub herders: u32,
    /// cultivated cells (tend > 0.4) and those with a domesticated crop (dom > 0.6)
    pub fields: u32,
    pub dom_fields: u32,
    /// mean soil fertility of cultivated cells (1 = fresh)
    pub field_fert: f32,
    /// share of all food eaten this year that came from fields and herds
    pub farm_food_share: f32,
    /// outbreaks of crowd disease, and deaths from crowd disease
    pub epidemics: u32,
    pub disease_deaths: u32,
    /// people per occupied cell: farmers vs foragers
    pub farmer_density: f32,
    pub forager_density: f32,
    /// lineages (descent from a founding band) still alive, and the largest one's share of people (%)
    pub lineages: u32,
    pub top_lineage_pct: f32,
    /// (v9) effective number of lineages, 1/sum(share^2): 6 even founders read 6, one
    /// lineage holding everyone reads 1 (0 in older saves)
    pub eff_lineages: f32,
    /// (inventions step 0) seafaring at year end: bands with boats, of them those that can use
    /// a sail, their mean and 90th-percentile seacraft, the share of all bands holding word of
    /// a far shore, and the longest crossing to another landmass so far (open-water cells)
    pub boat_bands: u32,
    pub sail_bands: u32,
    pub seacraft_mean: f32,
    pub seacraft_p90: f32,
    pub rumour_share: f32,
    pub leg_record: f32,
    /// (inventions step 1) bands that can make pitch, and bands with sealed jars
    pub pitch_bands: u32,
    pub jar_bands: u32,
    /// (inventions step 2) bands drying meat and fish on racks, bands with cold cellars, and
    /// the median and 90th-percentile endurance (open-water cells) of bands with boats
    pub dried_bands: u32,
    pub cellar_bands: u32,
    pub endurance_p50: f32,
    pub endurance_p90: f32,
    /// (inventions step 3) bands with pilotage above 0.1, and the mean and 90th-percentile
    /// pilotage of bands with boats
    pub pilot_bands: u32,
    pub pilot_mean: f32,
    pub pilot_p90: f32,
    /// (inventions step 4) bands with sky lore above 0.1, mean sky lore of farming bands
    /// (sow > 0) and of bands with boats, and mean sky lore as it counts at sea (boat bands)
    pub sky_bands: u32,
    pub sky_farm_mean: f32,
    pub sky_sea_mean: f32,
    pub sky_at_sea_mean: f32,
    /// (inventions step 5) bands with a ship, and the mean recipe quality of their ships
    pub ship_bands: u32,
    pub ship_q_mean: f32,
    /// (inventions step 5b) bands whose sail is woven matting (usable only on a plank hull or ship)
    pub mat_sail_bands: u32,
    /// (inventions step 6) farming bands with iron tools; cultivated cells more than half opened
    /// with iron; cultivated cells in forest (temperate, tropical, taiga) and on heavy wet
    /// ground (rain >= 0.75)
    pub iron_farm_bands: u32,
    pub opened_fields: u32,
    pub forest_fields: u32,
    pub wet_fields: u32,
    /// (inventions step 7) bands whose kit has a steel tool or weapon; people-weighted mean
    /// weapon quality (`band::weapon_q`, what fight losses scale with)
    pub steel_bands: u32,
    pub weapon_q_mean: f32,
    /// (inventions step 8) bands with a counting mark in their kit; bands with records above
    /// 0.1; mean records over all bands; mean and most trade partners per band
    pub mark_bands: u32,
    pub records_bands: u32,
    pub records_mean: f32,
    pub partners_mean: f32,
    pub partners_max: u32,
    /// (inventions step 8b) close ties (trade strength `CLOSE_TIE` or more, after the yearly
    /// fade) per band: mean and most
    pub close_ties_mean: f32,
    pub close_ties_max: u32,
    /// (inventions step 9) bands keeping a chart (`World::charts`), and those of them with a
    /// far shore on it
    pub chart_bands: u32,
    pub charted_routes: u32,
    /// (inventions step 10) feasts held this year and the food given at them (person-seasons)
    pub feasts: u32,
    pub feast_food: f32,
    /// (inventions step 10) bands that know ale, and those brewing it (grain farmers in warm
    /// enough country) at year end
    pub ale_bands: u32,
    pub brew_bands: u32,
    /// (inventions step 10) at year end: Pearson correlation of band prestige with band size,
    /// the share of all prestige held by the single most prestigious band (%), and the Gini of
    /// prestige across bands
    pub prestige_size_r: f32,
    pub prestige_top_pct: f32,
    pub prestige_gini: f32,
    /// (inventions step 11) bands wearing gold ornaments, bands holding any gold, and the mean
    /// prestige of the gold-wearing bands over the mean of all bands (0 = none wear gold)
    pub gold_bands: u32,
    pub gold_held: u32,
    pub gold_prestige_x: f32,
}

/// Running totals since the start, declared once through `counters!`: the struct, the summing
/// of the per-band effect buffers (`Counters::add`, used by `fx::add_counters`), the names and
/// values used by the metrics output and the per-year deltas of the history store all come from
/// this one list, so a new counter cannot be left out of any of them (it used to stay 0 when it
/// was missed in a hand-kept list). `sum` counters are added up; `max` ones keep the largest.
macro_rules! counters {
    (sum { $( $(#[$m:meta])* $f:ident, )* } max { $( $(#[$mm:meta])* $g:ident, )* }) => {
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Counters {
            $( $(#[$m])* pub $f: u64, )*
            $( $(#[$mm])* pub $g: u64, )*
        }
        impl Counters {
            /// field names, in declaration order (the metrics columns)
            pub const NAMES: &'static [&'static str] = &[$(stringify!($f),)* $(stringify!($g),)*];
            /// values in the order of `NAMES`
            pub fn values(&self) -> Vec<u64> {
                vec![$(self.$f,)* $(self.$g,)*]
            }
            /// the inverse of `values` (missing values read 0)
            pub fn from_values(v: &[u64]) -> Counters {
                let mut it = v.iter().copied();
                Counters { $( $f: it.next().unwrap_or(0), )* $( $g: it.next().unwrap_or(0), )* }
            }
            /// add another buffer's counts (`max` counters keep the larger)
            pub fn add(&mut self, b: &Counters) {
                $( self.$f += b.$f; )*
                $( self.$g = self.$g.max(b.$g); )*
            }
            /// what happened since `earlier` (`max` counters: the value now)
            pub fn since(&self, earlier: &Counters) -> Counters {
                Counters { $( $f: self.$f.saturating_sub(earlier.$f), )* $( $g: self.$g, )* }
            }
        }
    };
}
counters! {
    sum {
        experiments,
        inventions,
        transfers,
        forgets,
        splits,
        /// (v9) splits brought on by dissent rather than size
        dissent_splits,
        /// (v9) weddings between bands (each an exchange of spouses), and fights between in-laws
        marriages,
        fights_in_law,
        /// (v9) recipes made better by play and practice (`World::refine`)
        refinements,
        /// (v9) recipes taught by the more prestigious of the two bands
        prestige_taught,
        deaths,
        /// band moves and daughter settlements made across open water
        voyages,
        /// voyages (planned or drift) that ended in deaths at sea, and the people lost
        sea_disasters,
        lost_at_sea,
        /// fishing parties blown off course: total, came ashore and founded a band, got home
        drifts,
        drift_landings,
        drift_returns,
        /// bands that sighted a far shore from their coast
        sightings,
        /// (v5) scouting trips by experienced seafarers: sent, came home with news of a far shore,
        /// lost at sea
        scouting,
        scout_finds,
        scouts_lost,
        /// (v5) bands (or drifted parties) that came ashore alive on a landmass other than the one
        /// they set out from
        crossings,
        /// (inventions step 0) open-water cells crossed by those crossings (mean leg =
        /// crossing_cells / crossings), and planned voyages made with food put by for the trip
        crossing_cells,
        voyages_provisioned,
        /// (inventions step 2) planned voyages longer than the food and water lasted
        past_endurance,
        /// (inventions step 3) planned voyages that turned toward signs of land on the way
        sign_voyages,
        /// (inventions step 3) planned voyages steered for a far shore the band had word of
        aimed_voyages,
        /// (inventions step 4) planned voyages whose crew steered by the stars (sky lore at sea
        /// >= 0.2: the ledger's `sky` bit)
        sky_voyages,
        /// (inventions step 5) planned voyages made in a ship (the ledger's `ship` bit)
        ship_voyages,
        /// (inventions step 5b) planned voyages by a well-provisioned ship crew past the boat's
        /// usual range (the ledger's `expedition` bit)
        expeditions,
        /// (inventions step 5b) ship legs out of sight of land (>= `PILOT_OUT` cells) made alive,
        /// each a lesson in deep-water seamanship
        deep_water_legs,
        /// (inventions step 6) band-seasons spent opening forest or heavy ground with iron tools
        iron_clearing,
        /// (inventions step 7) band-seasons a band forged a fresh batch of steel (its steel
        /// stock topped up from its own smithy)
        steel_made,
        /// (inventions step 8) band-seasons a band kept count with a tally or token (it had one
        /// and something worth counting)
        marks_used,
        /// (inventions step 9) far-shore routes a chart kept past their fade (renewed), routes
        /// forgotten for age, and routes taught from a chart
        routes_kept,
        routes_faded,
        routes_taught,
        /// (inventions step 10) feasts held, those of them with ale, and band-seasons a band
        /// brewed ale
        feasts,
        ale_feasts,
        ale_brewed,
        /// (v5, diagnostic) band pairs within meeting range, summed over seasons
        pairs_seen,
        meetings,
        /// conflict (v4)
        confrontations,
        avoided,
        tributes,
        fled,
        fights,
        conflict_deaths,
        displaced,
        fled_by_sea,
        absorbed,
        captives,
        /// farming (v5)
        farm_starts,
        farm_origins,
        took_up_farming,
        abandoned_farming,
        herds_started,
        herds_lost,
        epidemics,
        disease_deaths,
    }
    max {
        max_fight_deaths,
        /// (inventions step 0) the longest crossing to another landmass so far (open-water cells)
        longest_leg,
    }
}

/// (v9) How people first reached a landmass (see `World::track_landfall`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LandfallRec {
    pub landmass: u16,
    pub tick: u32,
    /// the band that arrived first
    pub band: u32,
    /// came over open water (voyage, sea flight, a daughter's crossing, drift) rather than on foot
    pub by_sea: bool,
    /// landmass they put to sea from (0 = walked, or unknown)
    pub from: u16,
    /// fewest open-water cells between this landmass and any land already settled when they
    /// arrived (u32::MAX = no sea path)
    pub gap: u32,
    /// (inventions step 0) open-water cells crossed on the trip that brought them (0 on foot,
    /// or a save from before it)
    pub leg: f32,
    /// (inventions step 0) what the party had with it (`TECH_*` bits; 0 on foot)
    pub tech: u16,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct World {
    pub cfg: Config,
    pub params: Params,
    /// (v10) binary saves store it as the difference from the regenerated map (`codec::map`)
    #[serde(with = "crate::codec::map")]
    pub map: Map,
    pub tick: u32,
    pub rng_state: u64,
    pub next_id: u32,
    pub plants: Vec<f32>,
    pub game: Vec<f32>,
    pub fish: Vec<f32>,
    /// small game (snared / trapped); empty in stage-1 saves, filled on load
    pub small: Vec<f32>,
    /// (v10) binary saves store rows plus columns (`codec::bands`)
    #[serde(with = "crate::codec::bands")]
    pub bands: Vec<Band>,
    pub reg: Registry,
    /// recent events, a ring per class (`archive::EventLog`; one list when serialized)
    pub events: EventLog,
    #[serde(with = "crate::codec::events")]
    pub chronicle: Vec<Event>,
    pub firsts: Vec<First>,
    /// item -> per cell: first tick a band in that cell could make it (sparse since format 6)
    #[serde(rename = "cell_first_sparse", with = "crate::save::pairs")]
    pub cell_first: BTreeMap<Item, CellFirst>,
    /// the dense table of formats 1-5 (`u32::MAX` = never): read from old saves only and moved
    /// into `cell_first` by `after_load`
    #[serde(rename = "cell_first", with = "crate::save::pairs", skip_serializing)]
    cell_first_dense: BTreeMap<Item, Vec<u32>>,
    pub history: Vec<YearStats>,
    pub settings_log: Vec<ParamChange>,
    pub counters: Counters,
    /// item -> number of bands able to make it (last year)
    #[serde(with = "crate::save::pairs")]
    pub item_bands: BTreeMap<Item, u32>,
    /// landmass id -> tick people first arrived (u32::MAX = never); empty in stage-1 saves
    pub landfall: Vec<u32>,
    /// (v9) per cell: 1 + the last year any band camped there (0 = nobody ever has). Cells left
    /// empty for `NOVELTY_YEARS` count as new country again. Saves before it rebuild it in
    /// `after_load` (settled landmasses as known now, the rest as never visited).
    pub camped: Vec<u16>,
    /// (v9) how each landmass after the first was reached, in order of arrival; empty in older
    /// saves (their landfalls show as "how unknown" in the analysis)
    #[serde(default)]
    pub landfalls: Vec<LandfallRec>,
    /// item -> most bands that could make it at once since it last appeared (log thinning)
    #[serde(with = "crate::save::pairs")]
    pub item_peak: BTreeMap<Item, u32>,
    /// item -> how many times it has been lost to the whole world
    #[serde(with = "crate::save::pairs")]
    pub item_lost: BTreeMap<Item, u32>,
    /// materials (bitmask) that have changed hands at least once
    pub traded_first: u32,
    /// goods moved through each cell recently (decays; for the trade overlay). Saved from v3 on
    /// so the overlay survives a reload; older saves start it empty.
    pub trade_flow: Vec<f32>,
    /// people killed in fights around each cell recently (decays; for the conflict overlay, v4)
    pub conflict_flow: Vec<f32>,
    // ---- farming stage, per cell (v5; older saves start them fresh on load)
    /// memory of camp use (person-seasons, decaying)
    pub camp: Vec<f32>,
    /// how worked the ground is, 0..1: up to 0.4 tended wild stands, above that cleared, sown fields
    pub tend: Vec<f32>,
    /// domestication of the crop growing there, 0..1
    pub dom: Vec<f32>,
    /// soil fertility, 0..1 (1 = never cropped)
    pub fert: Vec<f32>,
    /// standing crop
    pub crop: Vec<f32>,
    /// (inventions step 6) per cell, 0..1: share of its forest and heavy ground opened with iron
    /// tools (raises its crop potential toward `open_pot`; fades slowly when not under fields;
    /// absent in older saves and built empty on load)
    pub opened: Vec<f32>,
    /// where and when farming began independently: (tick, cell, band)
    pub farm_starts: Vec<(u32, u32, u32)>,
    /// world firsts with no item of their own (bit 0: ard, bit 1: herding, bit 2: riding;
    /// bit 8: first crossing by ship, `FLAG_SHIP_CROSSED`)
    pub farm_flags: u32,
    /// (v10) the history store: tiered per-year frames, band genealogy, recent weddings
    pub archive: Archive,
    /// (inventions step 0) the longest crossing to another landmass so far (open-water cells;
    /// 0 in older saves, so the first crossing after loading one is reported as the record)
    pub leg_record: f32,
    /// crop potential per cell (derived from the map)
    #[serde(skip)]
    pub crop_pot: Vec<f32>,
    /// (inventions step 6) crop potential once fully opened with iron tools (derived, not saved)
    #[serde(skip)]
    pub open_pot: Vec<f32>,
    #[serde(skip)]
    pub graze_pot: Vec<f32>,
    /// where wild crop / herd ancestors live (`farming::WILD_*` bits; derived, not saved)
    #[serde(skip)]
    pub wild: Vec<u8>,
    /// river system of each cell (`u32::MAX` = no river; derived, not saved)
    #[serde(skip)]
    pub river_sys: Vec<u32>,
    #[serde(skip)]
    pub occ: Vec<Vec<u32>>,
    /// people per cell (snapshot used for crowding)
    #[serde(skip)]
    pub occ_pop: Vec<u32>,
    /// bands founded during the band loop (drift landings), added at the end of the season
    #[serde(skip)]
    pending: Vec<Band>,
    /// this season's harvest log while phase 3 of the band seasons runs (see `seen`)
    #[serde(skip)]
    hlog: Vec<HarvestMark>,
    #[serde(skip)]
    hlog_on: bool,
    /// per cell: first entry of its chain in `hlog` (u32::MAX = no harvest there this season)
    /// and the last one; reset only on the cells touched, so it costs nothing on big maps
    #[serde(skip)]
    hhead: Vec<(u32, u32)>,
    /// `hlog` regrouped by cell (band order kept), what `seen` reads: (band's place, stocks)
    #[serde(skip)]
    hcell: Vec<(u32, [f32; 4])>,
    #[serde(skip)]
    rng: Option<Rng>,
    /// optional per-phase wall-clock timing (seconds), for perf work; native only, never saved
    #[serde(skip)]
    pub prof: Option<Box<Prof>>,
    #[serde(skip)]
    year_acc: YearStats,
    /// (inventions step 3) per sea cell: steps to the nearest land and that land cell
    /// (derived from the map on first use, not saved)
    #[serde(skip)]
    signs: std::sync::OnceLock<SeaSigns>,
}

/// attention every material gets regardless of place and need (see `attention`)
const ATT_FLOOR: f32 = 1.0;
/// how strongly a band's way of life steers which parts it puts together (see `experiment`)
const NEED_FORM: f32 = 3.0;
fn raw(m: Mat) -> Item {
    item(Form::Raw, m)
}

/// Goods bands carry and can pass on (food is shared separately; wood is everywhere).
pub const TRADE_MATS: [Mat; 19] = [
    Mat::Stone,
    Mat::Flint,
    Mat::Obsidian,
    Mat::Clay,
    Mat::CopperOre,
    Mat::TinOre,
    Mat::Fibre,
    Mat::Hide,
    Mat::Bone,
    Mat::Copper,
    Mat::Tin,
    Mat::Bronze,
    Mat::Grain,
    Mat::Flax,
    Mat::Wool,
    Mat::IronOre,
    Mat::Iron,
    Mat::Salt,
    // (inventions step 7)
    Mat::Steel,
];

/// What material `mi` is worth to band `b` right now, as (reason to want it, marginal worth).
/// No price is set anywhere: the reason is the band's own tools (1.0), recipes it knows (0.5),
/// what its partners have said others want, or plain curiosity about something it has never
/// held; the worth falls as its stock rises.
pub fn want(b: &Band, mi: usize) -> (f32, f32) {
    let bit = 1u32 << mi;
    let need: f32 = if b.d.needed & bit != 0 {
        1.0
    } else if b.d.uses & bit != 0 {
        0.5
    } else {
        0.0
    };
    let st = b.stock.get(mi).copied().unwrap_or(0.0);
    let nov = if st < 0.05 { 0.12 * b.curiosity } else { 0.0 };
    // (v8) a small, lasting want for what the band makes ornaments of, more for rare stuff
    let need = if b.d.orn & bit != 0 {
        let rare = matches!(ALL_MATS[mi], Mat::Copper | Mat::Tin | Mat::Bronze | Mat::Obsidian | Mat::Gold);
        need.max(if rare { 0.4 } else { 0.15 })
    } else {
        need
    };
    // (v8) people who put meat and fish by in pots or over smoke want salt, once they know it
    let need = if mi == Mat::Salt as usize && st > 0.0 && (b.d.kit.vessel != NONE || b.d.kit.smoke) { need.max(0.6) } else { need };
    let base = need.max(b.d.demand[mi]).max(nov);
    (base, base * (1.3 - st).clamp(0.0, 1.3))
}

impl World {
    pub fn new(cfg: Config, params: Params) -> World {
        let map = Map::generate(cfg.map_w, cfg.map_h, cfg.seed);
        let n = map.w * map.h;
        let mut w = World {
            plants: map.k_plants.clone(),
            game: map.k_game.clone(),
            fish: map.k_fish.clone(),
            small: map.k_small.clone(),
            map,
            cfg,
            params,
            rng_state: 0,
            next_id: 1,
            ..Default::default()
        };
        w.occ = vec![Vec::new(); n];
        w.trade_flow = vec![0.0; n];
        w.conflict_flow = vec![0.0; n];
        w.ensure_farm_layers();
        w.ensure_terrain_layers();
        let mut rng = Rng::new(w.cfg.seed);
        // founders: one region anywhere people can live off the land - forest, grassland, coast,
        // cool or hot - more likely the richer it is. Where people start (what grows, what
        // lies about, how cold the winters are) shapes what they come to work with first.
        // (v9) on a continent, and not beside a strait: no other landmass within
        // `Map::START_CLEAR` cells, so nobody walks to a "new continent" in the first years.
        // Fall back to any good land if no such place exists.
        let food = |i: usize| w.map.k_plants[i] + w.map.k_game[i] + 0.5 * w.map.k_fish[i];
        let good = |i: usize| w.map.is_land(i) && w.map.temp[i] > 3.0 && w.map.temp[i] < 30.0 && food(i) > 38.0;
        let mut cands: Vec<usize> = (0..n)
            .filter(|&i| good(i) && w.map.is_continent(w.map.landmass_of(i) as usize) && w.map.clear_of_other_land(i, Map::START_CLEAR))
            .collect();
        if cands.is_empty() {
            cands = (0..n).filter(|&i| good(i)).collect();
        }
        if cands.is_empty() {
            cands = (0..n).filter(|&i| w.map.is_land(i) && w.map.k_plants[i] > 5.0).collect();
        }
        let cw: Vec<f32> = cands.iter().map(|&i| (food(i) - 35.0).max(1.0).sqrt()).collect();
        let origin = cands[rng.weighted(&cw).unwrap_or(0)];
        // chosen starts (the start picker / --start): keep only those on habitable land
        let starts: Vec<usize> = w
            .cfg
            .starts
            .iter()
            .filter(|&&(x, y)| w.map.start_check(x as usize, y as usize) != StartCheck::Uninhabitable)
            .map(|&(x, y)| y as usize * w.map.w + x as usize)
            .collect();
        let chosen = !starts.is_empty();
        if chosen && starts.len() > w.cfg.founders {
            w.cfg.founders = starts.len();
        }
        // the founders are one people: they share habits of hand (what they are used to working,
        // which ways of working) with small differences between groups
        // (drawn in the v4 order - 18 materials, 3 ways of working - then the v5 materials, so a
        // seed's founders lean the same way they did before farming existed)
        let mut culture: Vec<f32> = (0..21).map(|_| (0.5 * rng.gauss()).exp()).collect();
        // (the v8 materials, brine and salt, start neutral: nobody has a habit with them yet, and
        // drawing nothing for them keeps every seed's founders as they were)
        let extra: Vec<f32> = (21..26).map(|_| (0.5 * rng.gauss()).exp()).collect();
        let acts = culture.split_off(18);
        culture.extend(extra);
        culture.resize(NMAT, 1.0);
        culture.extend(acts);
        debug_assert_eq!(culture.len(), CRAFT_LEN);
        for k in 0..w.cfg.founders {
            // the start: the random homeland, the shared chosen area, or this lineage's own cell
            let origin = if chosen { starts[k % starts.len()] } else { origin };
            let (ox, oy) = (origin % w.map.w, origin / w.map.w);
            let alone = chosen && starts.len() >= w.cfg.founders;
            let mut pos = origin;
            for _ in 0..if alone { 0 } else { 20 } {
                let x = (ox as i32 + rng.below(7) as i32 - 3).clamp(0, w.map.w as i32 - 1) as usize;
                let y = (oy as i32 + rng.below(7) as i32 - 3).clamp(0, w.map.h as i32 - 1) as usize;
                let i = y * w.map.w + x;
                // (v9) founders stay on their homeland's landmass
                if w.map.is_land(i) && w.map.landmass_of(i) == w.map.landmass_of(origin) {
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
                ambition: (AMBITION_FOUNDER_SD * rng.gauss()).clamp(-AMBITION_MAX, AMBITION_MAX),
                flame: false,
                stock: vec![0.0; NMAT],
                origin: vec![u32::MAX; NMAT],
                craft: culture.iter().map(|c| (c * (0.15 * rng.gauss()).exp()).clamp(0.2, 4.0)).collect(),
                ..Default::default()
            };
            w.push_event(EventKind::Founded, id, chosen as u32, pos as u32, NONE, u32::MAX);
            w.bands.push(b);
        }
        w.rng_state = rng.0;
        w.archive.begun = Some(0);
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
        self.archive.note(&e);
        self.events.push(e);
    }

    /// (v10) A wedding, for the history store only (see `Archive`).
    pub(crate) fn note_wedding(&mut self, a: u32, b: u32, la: u32, lb: u32, cell: u32) {
        let e = Event { tick: self.tick, kind: EventKind::Married as u8, band: a, other: b, cell, item: NONE, recipe: u32::MAX };
        self.archive.note(&e);
        self.archive.note_wedding_lineages(la, lb);
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
        // (v10) saves before the history store: archive from here on, counting from now
        if self.archive.begun.is_none() {
            self.archive.begun = Some(self.tick);
            self.archive.last_counters = self.counters.clone();
        }
        let n = self.map.w * self.map.h;
        // formats 1-5 kept the first-reach table dense
        for (it, v) in std::mem::take(&mut self.cell_first_dense) {
            self.cell_first.insert(it, CellFirst::from_dense(self.map.w, self.map.h, &v));
        }
        // stage-1 maps gain the newer derived layers (small game, offshore fish, landmasses)
        let legacy = self.map.gen_version < 2;
        self.map.upgrade_legacy();
        if self.map.landmass.len() != n {
            self.map.label_landmasses();
        }
        if self.small.len() != n {
            self.small = self.map.k_small.clone();
        }
        if self.fish.len() != n {
            self.fish.resize(n, 0.0);
        }
        for i in 0..n {
            // newly added shelf fish start at capacity
            if legacy && !self.map.is_land(i) {
                self.fish[i] = self.map.k_fish[i];
            }
        }
        self.occ = vec![Vec::new(); n];
        for b in &mut self.bands {
            b.fit_vectors();
        }
        if self.conflict_flow.len() != n {
            self.conflict_flow = vec![0.0; n];
        }
        if self.trade_flow.len() != n {
            self.trade_flow = vec![0.0; n];
        }
        self.ensure_farm_layers();
        self.ensure_terrain_layers();
        // (v9) saves before the camp layer: settled landmasses count as known now
        if self.camped.len() != n {
            let now = self.year_u16().saturating_add(1);
            let lf = &self.landfall;
            let m = &self.map;
            self.camped = (0..n)
                .map(|i| {
                    let l = m.landmass_of(i) as usize;
                    if m.is_land(i) && lf.get(l).is_some_and(|&t| t != u32::MAX) { now } else { 0 }
                })
                .collect();
        }
        // saves from before v3 did not keep the overlay: sketch it from the standing trade
        // partnerships (who trades with whom is saved), so it is not blank after loading
        if self.trade_flow.iter().all(|&f| f == 0.0) {
            let w = self.map.w;
            let pos: std::collections::HashMap<u32, (f32, f32)> = self.bands.iter().map(|b| (b.id, (b.x as f32, b.y as f32))).collect();
            for b in &self.bands {
                for p in &b.partners {
                    let Some(&(x1, y1)) = pos.get(&p.id) else { continue };
                    let (x0, y0) = (b.x as f32, b.y as f32);
                    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1.0) as usize;
                    let q = 0.1 * p.strength;
                    for k in 0..=steps {
                        let t = k as f32 / steps as f32;
                        let (x, y) = ((x0 + (x1 - x0) * t).round() as usize, (y0 + (y1 - y0) * t).round() as usize);
                        if let Some(f) = self.trade_flow.get_mut(y * w + x) {
                            *f += q;
                        }
                    }
                }
            }
        }
        // (inventions step 12) recipes the rules no longer allow go, before toolkits and kits
        // are derived again from what is left
        self.clean_recipes();
        // derived toolkits so a freshly loaded (paused) world can be inspected
        // and the season's figures (food, stress, cold...), which are not saved: recompute them
        // from the loaded world so the inspector and stats are right before the first step
        let mut bands = std::mem::take(&mut self.bands);
        let s = self.season();
        for b in &mut bands {
            self.refresh_toolkit(b);
            let ec = self.economy(b, s);
            ec.apply(&mut b.d);
        }
        self.bands = bands;
    }

    /// (inventions step 12) Save cleanup on load. Recipes learned under older, laxer rules
    /// that physics no longer allows (a wool or hide shaft, a tin knife or point, cold-hammered
    /// bronze, iron or steel, a sail of clothing on a canoe, clothing sewn from raw fibre, bronze
    /// fired from the bare metals, a hammered obsidian pendant...) are dropped from every band
    /// that knows them; the registry keeps them (recipe ids are stable) marked retired, so they
    /// are never taught, inherited or captured. Pitch boiled from wood is the one recipe carried
    /// over: it becomes the same recipe with resinous wood (the know-how is the same; the band
    /// needs pine or birch at hand to use it). The cold-gold ring recipe of step 11 stays with
    /// the bands that know it (step 11b). Steel soaked alone in the blown furnace is dropped too
    /// (step 13: steel is now worked under the hammer at the open charcoal hearth); the steel
    /// a band holds stays in its stores. Kits are then re-derived from the toolkit that is left
    /// (`refresh_toolkit`, below in `after_load`). Idempotent: a second pass changes nothing.
    /// Returns (recipes repointed, known entries dropped).
    pub fn clean_recipes(&mut self) -> (usize, usize) {
        let wood = item(Form::Raw, Mat::Wood);
        let resin = item(Form::Raw, Mat::ResinWood);
        let pitch = item(Form::Raw, Mat::Pitch);
        let mut repointed = 0;
        for k in 0..self.reg.recipes.len() {
            let r = &self.reg.recipes[k];
            if r.action != Action::Heat as u8 || r.output != pitch || !r.inputs[..2].contains(&wood) || !is_retired(r) {
                continue;
            }
            let mut inp = r.inputs;
            for x in inp[..2].iter_mut() {
                if *x == wood {
                    *x = resin;
                }
            }
            let inp = physics::canonical(Action::Heat, inp);
            let r2 = Recipe { inputs: inp, ..r.clone() };
            if self.reg.find(Action::Heat, &inp).is_none() && !is_retired(&r2) {
                self.reg.recipes[k] = r2;
                repointed += 1;
            }
        }
        self.reg.rebuild_index();
        let reg = &self.reg;
        let mut dropped = 0;
        for b in &mut self.bands {
            let before = b.known.len();
            b.known.retain(|kn| !reg.retired(kn.recipe) || is_legacy_gold(&reg.recipes[kn.recipe as usize]));
            dropped += before - b.known.len();
        }
        (repointed, dropped)
    }

    // ------------------------------------------------------------------ step
    pub fn step(&mut self) {
        let mut rng = self.rng.take().unwrap_or(Rng(self.rng_state));
        if self.occ.len() != self.map.w * self.map.h {
            self.after_load();
        }
        let mut pr = [0f64; 5];
        let mut t = Tick::now();
        self.regrow();
        self.regrow_fields();
        fx::par_scale(&mut self.trade_flow, 0.85);
        fx::par_scale(&mut self.conflict_flow, 0.93);
        self.build_occupancy();
        pr[0] += t.lap();
        let mut bands = std::mem::take(&mut self.bands);
        self.bands_season(&mut bands, &mut rng);
        pr[1] += t.lap();
        self.contacts(&mut bands, &mut rng);
        pr[2] += t.lap();
        self.demography(&mut bands, &mut rng);
        pr[3] += t.lap();
        self.bands = bands;
        self.track_landfall();
        self.track_leg_record();
        self.track_knowledge();
        self.tick += 1;
        if self.tick % 4 == 0 {
            self.year_stats();
        }
        pr[4] += t.lap();
        self.rng_state = rng.0;
        self.rng = Some(rng);
        if let Some(p) = self.prof.as_mut() {
            for k in 0..5 {
                p.t[k] += pr[k];
            }
        }
    }

    /// Every band's season, in three phases: (1) in parallel, each band refits its knowledge and
    /// works out its toolkit; (2) band by band, each plans its economy against the stocks the
    /// earlier bands left and takes its harvest (exactly as in v5); (3) in parallel, the rest of each band's season (farming,
    /// materials, births and deaths, disease, fire, experiments, the sea, forgetting, moving),
    /// with its effects on the shared world buffered and applied afterwards in band order.
    fn bands_season(&mut self, bands: &mut [Band], rng: &mut Rng) {
        let s = self.season();
        let dts: Vec<f64> = {
            let me = &*self;
            fx::par_map(bands, |b| me.band_prep(b))
        };
        if let Some(p) = self.prof.as_mut() {
            p.t[13] += dts.iter().sum::<f64>();
        }
        let mut tk = if self.prof.is_some() { Some(Tick::now()) } else { None };
        let ecs = self.take_harvest(bands, s);
        if let (Some(t), Some(p)) = (tk.as_mut(), self.prof.as_mut()) {
            p.t[5] += t.lap();
        }
        for (b, ec) in bands.iter_mut().zip(&ecs) {
            b.store = ec.store_after;
        }
        // phase 3 reads the wild stocks as each band would have found them in v5, right after
        // its own harvest (before the later bands took theirs): see `seen`
        let mut tk = if self.prof.is_some() { Some(Tick::now()) } else { None };
        // regroup the log by cell so each lookup reads one short contiguous run;
        // hhead becomes (start, len) into hcell
        const DONE: u32 = u32::MAX - 1;
        self.hcell.clear();
        for i in 0..self.hlog.len() {
            if self.hlog[i].next == DONE {
                continue;
            }
            // the first entry of a cell not yet seen is the head of its chain
            let c = self.hlog[i].cell as usize;
            let start = self.hcell.len() as u32;
            let mut j = i;
            loop {
                let m = self.hlog[j];
                self.hcell.push((m.k, m.v));
                self.hlog[j].next = DONE;
                if m.next == u32::MAX {
                    break;
                }
                j = m.next as usize;
            }
            self.hhead[c] = (start, self.hcell.len() as u32 - start);
        }
        self.hlog_on = true;
        if let (Some(t), Some(p)) = (tk.as_mut(), self.prof.as_mut()) {
            p.t[14] += t.lap();
        }
        let key = rng.next_u64();
        let tick = self.tick as u64;
        let fxs: Vec<Fx> = {
            let me = &*self;
            let mut work: Vec<(&mut Band, Econ)> = bands.iter_mut().zip(ecs).collect();
            fx::par_map(&mut work, |(b, ec)| {
                let mut fx = Fx::default();
                let mut r = Rng::new(key ^ (b.id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (tick << 40));
                for &it in &ec.used {
                    mark_used(&me.reg, &mut b.known, &b.d.tk, it);
                }
                me.band_season(b, ec, &mut r, &mut fx);
                fx
            })
        };
        self.hlog_on = false;
        let mut tk = if self.prof.is_some() { Some(Tick::now()) } else { None };
        for (b, f) in bands.iter_mut().zip(fxs) {
            self.apply_fx(f, Some(b));
        }
        if let (Some(t), Some(p)) = (tk.as_mut(), self.prof.as_mut()) {
            p.t[15] += t.lap();
        }
    }

    /// Wild stock `r` (0 plants, 1 game, 2 fish, 3 small game) on cell `c` as band `b` sees it.
    /// During phase 3 of the band seasons that is the stock right after `b`'s own harvest,
    /// before any later band took its share (as in v5, where each band's whole season ran
    /// before the next band harvested): the value logged by the first later band to harvest
    /// the cell, or the current value if none did. Otherwise simply the current stock.
    #[inline]
    pub(crate) fn seen(&self, r: usize, c: usize, b: &Band) -> f32 {
        if let Some(v) = self.seen_logged(c, b) {
            return v[r];
        }
        let v = match r {
            0 => &self.plants,
            1 => &self.game,
            2 => &self.fish,
            _ => &self.small,
        };
        v.get(c).copied().unwrap_or(0.0)
    }

    /// All four wild stocks of `seen` at once.
    #[inline]
    pub(crate) fn seen4(&self, c: usize, b: &Band) -> [f32; 4] {
        if let Some(v) = self.seen_logged(c, b) {
            return v;
        }
        let g = |v: &Vec<f32>| v.get(c).copied().unwrap_or(0.0);
        [g(&self.plants), g(&self.game), g(&self.fish), g(&self.small)]
    }

    #[inline]
    fn seen_logged(&self, c: usize, b: &Band) -> Option<[f32; 4]> {
        if !self.hlog_on {
            return None;
        }
        let &(start, len) = self.hhead.get(c)?;
        if start == u32::MAX {
            return None;
        }
        let run = &self.hcell[start as usize..(start + len) as usize];
        let i = run.partition_point(|e| e.0 <= b.d.hk);
        run.get(i).map(|e| e.1)
    }

    /// Phase 2, sequential and exactly as in v5: band by band, plan the economy against the
    /// stocks as earlier bands left them, then take the harvest. (Planning every band in
    /// parallel against the start-of-season stock and sharing out contested cells was tried
    /// first: it left stocks fuller, gave +23% people and +40% war deaths over 10 seeds; and
    /// with 8-cell home ranges over 90% of bands share a cell with an earlier one, so a
    /// speculative parallel plan is almost never still valid.)
    fn take_harvest(&mut self, bands: &mut [Band], s: u32) -> Vec<Econ> {
        let mut ecs = Vec::with_capacity(bands.len());
        let n = self.map.w * self.map.h;
        if self.hhead.len() != n {
            self.hhead = vec![(u32::MAX, u32::MAX); n];
        } else {
            for m in &self.hlog {
                self.hhead[m.cell as usize] = (u32::MAX, u32::MAX);
            }
        }
        self.hlog.clear();
        for b in bands.iter_mut() {
            // as in v5, the toolkit reflects the cell as the earlier bands left it (usually
            // the toolkit worked out in phase 1, or the one cached before it)
            self.refresh_toolkit(b);
            let ec = self.economy(b, s);
            let k = ecs.len() as u32;
            b.d.hk = k;
            if ec.ys.iter().any(|&y| y > 0.01) {
                for &(c, _) in &ec.cells {
                    let g = |v: &Vec<f32>| v.get(c).copied().unwrap_or(0.0);
                    let i = self.hlog.len() as u32;
                    self.hlog.push(HarvestMark { cell: c as u32, k, next: u32::MAX, v: [g(&self.plants), g(&self.game), g(&self.fish), g(&self.small)] });
                    let h = &mut self.hhead[c];
                    if h.0 == u32::MAX {
                        *h = (i, i);
                    } else {
                        let t = h.1;
                        h.1 = i;
                        self.hlog[t as usize].next = i;
                    }
                }
            }
            let stocks: [&mut Vec<f32>; 5] = [&mut self.plants, &mut self.game, &mut self.fish, &mut self.small, &mut self.crop];
            for (r, st) in stocks.into_iter().enumerate() {
                if ec.ys[r] > 0.01 {
                    let phi = (ec.h[r] / ec.ys[r]).min(1.0);
                    for &(c, wt) in &ec.cells {
                        if let Some(v) = st.get_mut(c) {
                            *v -= phi * wt * *v;
                        }
                    }
                }
            }
            ecs.push(ec);
        }
        ecs
    }

    /// Phase 1 of a band's season (read-only on the world): fit its knowledge vectors and work
    /// out its toolkit.
    fn band_prep(&self, b: &mut Band) -> f64 {
        let mut tk = if self.prof.is_some() { Some(Tick::now()) } else { None };
        b.fit_vectors();
        b.d.sailed_from = None;
        b.d.sailed_sea = 0.0;
        b.d.sailed_tech = 0;
        for k in &mut b.known {
            k.used = false;
        }
        self.refresh_toolkit(b);
        tk.as_mut().map(|t| t.lap()).unwrap_or(0.0)
    }

    fn regrow(&mut self) {
        let s = self.season();
        let r = self.params.regen;
        let m = &self.map;
        let n = m.w * m.h;
        let tend = &self.tend;
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            const CH: usize = 4096;
            if n >= 65536 && self.plants.len() == n && self.game.len() == n && self.fish.len() == n && self.small.len() == n {
                self.plants
                    .par_chunks_mut(CH)
                    .zip(self.game.par_chunks_mut(CH))
                    .zip(self.fish.par_chunks_mut(CH))
                    .zip(self.small.par_chunks_mut(CH))
                    .enumerate()
                    .for_each(|(ci, (((pl, ga), fi), sm))| {
                        for j in 0..pl.len() {
                            regrow_cell(m, tend, s, r, ci * CH + j, &mut pl[j], &mut ga[j], &mut fi[j], &mut sm[j]);
                        }
                    });
                return;
            }
        }
        for i in 0..n {
            let (pl, ga, fi, sm) = (&mut self.plants[i], &mut self.game[i], &mut self.fish[i], self.small.get_mut(i));
            let mut dummy = 0.0;
            let sm = sm.unwrap_or(&mut dummy);
            regrow_cell(m, tend, s, r, i, pl, ga, fi, sm);
        }
    }

    fn build_occupancy(&mut self) {
        for v in &mut self.occ {
            v.clear();
        }
        let w = self.map.w;
        self.occ_pop.clear();
        self.occ_pop.resize(self.occ.len(), 0);
        let stamp = self.year_u16().saturating_add(1);
        self.camped.resize(self.occ.len(), 0);
        for (k, b) in self.bands.iter().enumerate() {
            self.occ[b.cell(w)].push(k as u32);
            self.occ_pop[b.cell(w)] += b.pop;
            self.camped[b.cell(w)] = stamp;
        }
    }

    /// (v9) How new cell `c` is to people: 1 if nobody has ever camped there, falling to 0 for a
    /// cell camped in this year (linear over `NOVELTY_YEARS`).
    pub fn novelty(&self, c: usize) -> f32 {
        match self.camped.get(c).copied().unwrap_or(0) {
            0 => 1.0,
            t => ((self.year_u16() + 1).saturating_sub(t) as f32 / NOVELTY_YEARS).min(1.0),
        }
    }

    /// (v9) Wanderlust: the factor by which band `b` favours cell `c` for being new country.
    /// Curious bands full of young people feel it most.
    fn novelty_pull(&self, b: &Band, c: usize, split_size: f32) -> f32 {
        let youth = (b.pop as f32 / split_size).min(1.0);
        1.0 + (NOVELTY_PULL * b.curiosity * youth).min(NOVELTY_MAX) * self.novelty(c)
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
        // (inventions step 12) pine and birch, for pitch
        if m.resin_wood(i) || st(Mat::ResinWood) {
            s.add(raw(Mat::ResinWood));
        }
        for (d, mm) in DEPOSITS {
            if m.has(i, d) || st(mm) {
                s.add(raw(mm));
            }
        }
        // hides, bones, and metal carried as blanks / ingots (made here or traded in)
        for mm in [Mat::Hide, Mat::Bone, Mat::Copper, Mat::Tin, Mat::Bronze, Mat::Iron, Mat::Steel, Mat::Grain, Mat::Flax, Mat::Wool, Mat::Salt] {
            if st(mm) {
                s.add(raw(mm));
            }
        }
        // (v8) sea water on the shore, or brine in a dry salt pan
        if self.brine_at(i) {
            s.add(raw(Mat::Brine));
        }
        if self.seen(1, i, b) > 5.0 {
            s.add(raw(Mat::Meat));
        }
        if self.seen(0, i, b) > 5.0 {
            s.add(raw(Mat::Plants));
        }
        if self.seen(2, i, b) > 3.0 {
            s.add(raw(Mat::Fish));
        }
        s
    }

    /// Recompute every band's derived toolkit (used by tools that inspect a loaded world).
    pub fn refresh_all_toolkits(&mut self) {
        let mut bands = std::mem::take(&mut self.bands);
        for b in &mut bands {
            self.refresh_toolkit(b);
        }
        self.bands = bands;
    }

    fn refresh_toolkit(&self, b: &mut Band) {
        let raws = self.raws_at(b);
        let key = (raws, b.flame, b.d.known_ver);
        if b.d.fin_dirty {
            // (v9) the cached spare kit was made with older recipe qualities
            b.d.tk_prev = None;
        }
        if b.d.tk_key == Some(key) {
            // (diffusion D3) the kit follows skill too, which moves every season
            if b.d.fin_dirty || b.d.kit_dirty {
                let mut kit = make_kit_for(&b.d.tk, &b.known);
                refine_kit(&mut kit, &b.d.tk, &b.known);
                b.d.kit = kit;
                b.d.fin_dirty = false;
                b.d.kit_dirty = false;
            }
            return;
        }
        // a two-entry cache: a band's toolkit tends to flip between two states within and
        // across seasons (game or plants on its cell above / below the threshold as bands
        // harvest and stocks regrow), so keep the previous one rather than recompute it
        if b.d.tk_prev.as_ref().map(|p| p.0) == Some(key) {
            let mut prev = b.d.tk_prev.take().unwrap();
            std::mem::swap(&mut prev.1, &mut b.d.tk);
            std::mem::swap(&mut prev.2, &mut b.d.kit);
            prev.0 = b.d.tk_key.unwrap_or(key);
            b.d.tk_key = Some(key);
            if prev.0 != key {
                b.d.tk_prev = Some(prev);
            }
            // (diffusion D3) the cached kit was chosen at older skills
            let mut kit = make_kit_for(&b.d.tk, &b.known);
            refine_kit(&mut kit, &b.d.tk, &b.known);
            b.d.kit = kit;
            b.d.kit_dirty = false;
            return;
        }
        {
            let tk = closure(&self.reg, &b.known, &raws, b.flame);
            let mut kit = make_kit_for(&tk, &b.known);
            refine_kit(&mut kit, &tk, &b.known);
            b.d.fin_dirty = false;
            b.d.kit_dirty = false;
            let old_tk = std::mem::replace(&mut b.d.tk, tk);
            let old_kit = std::mem::replace(&mut b.d.kit, kit);
            b.d.tk_prev = match b.d.tk_key {
                Some(k) if k.2 == b.d.known_ver => Some(Box::new((k, old_tk, old_kit))),
                _ => None,
            };
            b.d.tk_key = Some(key);
            let mut uses = 0u32;
            for k in &b.known {
                for &i in self.reg.recipes[k.recipe as usize].inputs.iter() {
                    if i != NONE && form(i) == Form::Raw {
                        uses |= 1 << (mat(i) as u32);
                    }
                }
            }
            b.d.uses = uses;
            let mut orn = 0u32;
            for k in &b.known {
                let out = self.reg.recipes[k.recipe as usize].output;
                if form(out) == Form::Ornament {
                    orn |= 1 << (mat(out) as u32);
                }
            }
            b.d.orn = orn;
        }
    }

    /// (v8) Brine to hand: a sea shore, or a hot, dry lowland where salt pans form.
    pub fn brine_at(&self, c: usize) -> bool {
        let m = &self.map;
        if !m.is_land(c) {
            return false;
        }
        let (x, y) = (c % m.w, c / m.w);
        let pan = m.rain[c] < 0.15 && m.temp[c] > 12.0 && m.height[c] < 500.0 && !m.river.get(c).copied().unwrap_or(false);
        pan || nbrs8(x, y, m.w, m.h).any(|(a, bb)| !m.is_land(bb * m.w + a))
    }

    /// Is there open water (sea or a river) at or next to this cell?
    pub fn near_water(&self, c: usize) -> bool {
        let m = &self.map;
        let (x, y) = (c % m.w, c / m.w);
        m.river.get(c).copied().unwrap_or(false) || nbrs8(x, y, m.w, m.h).any(|(a, bb)| !m.is_land(bb * m.w + a))
    }

    /// The cells a band forages this season with their weights: its own cell, the land around it,
    /// and, if it has a boat and lives by the sea, a stretch of coastal water it can fish.
    pub fn home_range(&self, b: &Band, kit: &Kit) -> Vec<(usize, f32)> {
        let m = &self.map;
        let w = m.w;
        let ci = b.cell(w);
        let (x, y) = (b.x as usize, b.y as usize);
        let mut cells = Vec::with_capacity(16);
        cells.push((ci, 1.0));
        let mut coastal = false;
        for (nx, ny) in nbrs8(x, y, w, m.h) {
            let j = ny * w + nx;
            if m.is_land(j) {
                cells.push((j, 0.3));
            } else {
                coastal = true;
            }
        }
        if kit.boat_q > 0.0 && coastal {
            let r = 1 + (2.0 * kit.boat_q).round() as i32;
            let mut sea: Vec<(usize, f32)> = Vec::new();
            for dy in -r..=r {
                for dx in -r..=r {
                    let (sx, sy) = (x as i32 + dx, y as i32 + dy);
                    if sx < 0 || sy < 0 || sx as usize >= w || sy as usize >= m.h {
                        continue;
                    }
                    let j = sy as usize * w + sx as usize;
                    if m.is_land(j) {
                        continue;
                    }
                    let d = dx.abs().max(dy.abs()) as f32;
                    // nearshore water was already fished from the beach; boats add the rest
                    let wt = if d <= 1.0 { 0.12 } else { 0.25 / d };
                    sea.push((j, wt));
                }
            }
            // a band can only work so much water: total weight capped
            let tot: f32 = sea.iter().map(|c| c.1).sum();
            let cap = 0.6 + 1.2 * kit.boat_q;
            let sc = if tot > cap { cap / tot } else { 1.0 };
            cells.extend(sea.into_iter().map(|(j, wt)| (j, wt * sc)));
        }
        cells
    }

    /// Whether a carried material is at hand where the band is this season (so the tools that
    /// need it do not wear its stock down extra): wood, fibre and resinous wood where they grow,
    /// metal and salt the band makes itself, ores and stone at their deposits, and (diffusion
    /// D1) hide and bone in any season the band hunted big game (`hunted` > 1).
    pub(crate) fn mat_at_hand(&self, b: &Band, ci: usize, mm: Mat, hunted: f32) -> bool {
        match mm {
            Mat::Wood => self.map.k_wood[ci] > 2.0,
            Mat::ResinWood => self.map.resin_wood(ci),
            Mat::Fibre => self.map.k_fibre[ci] > 2.0,
            Mat::Copper | Mat::Tin | Mat::Bronze | Mat::Iron | Mat::Steel | Mat::Salt => b.d.tk.producible.has(raw(mm)),
            Mat::Brine => false,
            Mat::Flax | Mat::Wool => false, // grown / shorn: topped up in `farm_season`
            // (diffusion D1) hunters dress hides and work bone from their own kills: they
            // are at hand in any season the band hunted big game, not only at a deposit
            Mat::Hide | Mat::Bone => hunted > 1.0,
            mm => DEPOSITS.iter().any(|&(d, m2)| m2 == mm && self.map.has(ci, d)),
        }
    }

    /// The season's economy for a band, computed from the current world without changing it:
    /// how hard it works, what it harvests, how full it eats, and what its tools need.
    pub fn economy(&self, b: &Band, s: u32) -> Econ {
        let p = self.params;
        let ci = b.cell(self.map.w);
        let kit = b.d.kit;
        let pop = b.pop.max(1) as f32;
        let labour = 0.55 * pop;
        let t = self.map.temp_season(ci, s);
        let cold = p.cold_scale * ((8.0 - t) / 14.0).max(0.0) * (1.0 - kit.protect);
        let need = pop * (1.0 + cold);
        let wet = self.near_water(ci);
        let cells = self.home_range(b, &kit);
        let get = |v: &Vec<f32>, c: usize| v.get(c).copied().unwrap_or(0.0);
        let ys = [
            cells.iter().map(|&(c, wt)| wt * get(&self.plants, c)).sum::<f32>(),
            cells.iter().map(|&(c, wt)| wt * get(&self.game, c)).sum::<f32>(),
            cells.iter().map(|&(c, wt)| wt * get(&self.fish, c)).sum::<f32>(),
            cells.iter().map(|&(c, wt)| wt * get(&self.small, c)).sum::<f32>(),
            if b.sow > 0.0 { cells.iter().map(|&(c, wt)| wt * get(&self.crop, c)).sum::<f32>() } else { 0.0 },
        ];
        // tool upkeep (a boat only matters by the water, a net only where there are fish)
        let mut upkeep = 0.0;
        let mut needed = 0u32;
        let mut used = Vec::with_capacity(17);
        // (inventions step 1) pitch is boiled up only for something that needs it: hulls by the
        // water to caulk, or jars to seal; while it is about, hafts are glued with it too
        let has_craft = [kit.boat, kit.outrigger, kit.sail, kit.plank, kit.ship].iter().any(|&i| i != NONE);
        let pitch = kit.pitch != NONE && ((wet && has_craft) || kit.jar != NONE);
        for &it in kit.used_items().iter() {
            if it == NONE {
                continue;
            }
            let craft = it == kit.boat || it == kit.outrigger || it == kit.sail || it == kit.plank || it == kit.ship;
            // ornaments are kept for show, not worn out at work; (diffusion D2) but worn, so
            // their making stays in mind: practised, with no wear and no stock drawn
            if it == kit.ornament {
                used.push(it);
                continue;
            }
            // (inventions step 8) a tally is only kept (and so remembered) with something to count
            let idle_mark = it == kit.mark && !Self::keeps_count(b);
            if (craft && !wet) || (it == kit.net && ys[2] < 1.0) || (it == kit.pitch && !pitch) || idle_mark {
                continue;
            }
            let (_, mask, depth) = b.d.tk.via(it).unwrap_or((0, 0, 0));
            needed |= mask;
            if !matches!(form(it), Form::Fire | Form::Cooked) {
                // a caulked hull leaks and rots less; a glued haft works loose less often
                let sealed = if !pitch {
                    1.0
                } else if craft {
                    PITCH_CAULK
                } else if it == kit.hunt || it == kit.axe {
                    PITCH_GLUE
                } else {
                    1.0
                };
                upkeep += sealed * p.upkeep * (1.15 - durability(mat(it))) * (1.0 + 0.3 * depth as f32);
            } else {
                upkeep += 0.01;
            }
            used.push(it);
        }
        // (diffusion D3) now and then a band tries the better-in-prospect tool beside the one
        // it relies on (only in a season it uses that slot): the trial wears it and practises it
        for j in 0..kit.trial.len() {
            let (inc, ch) = (kit.slot_item(j), kit.trial[j]);
            if ch == NONE || !used.contains(&inc) || used.contains(&ch) {
                continue;
            }
            let roll = hash3(self.cfg.seed ^ 0x7121_A15E_D3D3_0001, b.id as i64, (self.tick as i64) * 16 + j as i64);
            if (roll >> 40) as f32 / (1u64 << 24) as f32 >= KIT_TRIAL_P {
                continue;
            }
            let (_, mask, depth) = b.d.tk.via(ch).unwrap_or((0, 0, 0));
            needed |= mask;
            upkeep += p.upkeep * (1.15 - durability(mat(ch))) * (1.0 + 0.3 * depth as f32);
            used.push(ch);
        }
        let upkeep_l = upkeep * labour;
        // fields and herds: tending fields and minding animals take labour every season
        let fm = self.farm_terms(b, &cells, t);
        let upkeep_l = upkeep_l + fm.labour * labour;
        let ks = [
            3.0 * kit.plants_m * p.forage_eff,
            2.5 * kit.game_m * p.forage_eff,
            2.5 * kit.fish_m * p.forage_eff,
            2.5 * kit.small_m * p.forage_eff,
            3.5 * kit.farm_m * fm.plough * p.forage_eff,
        ];
        let mut dg = kit.digest;
        // (inventions step 10) brewing makes a little more of the grain
        if self.brews(b) {
            dg[4] *= 1.0 + ALE_FOOD;
        }
        // effort goes where it pays best (food value per unit of effort)
        let wsum: f32 = (0..5).map(|r| ys[r] * ks[r] * dg[r]).sum::<f32>().max(1e-6);
        let harvest = |e: f32| -> [f32; 5] {
            let mut h = [0f32; 5];
            for r in 0..5 {
                if ys[r] > 0.01 {
                    let er = e * ys[r] * ks[r] * dg[r] / wsum;
                    h[r] = ys[r] * (1.0 - (-ks[r] * er / ys[r]).exp());
                }
            }
            h
        };
        // herd food (meat, later milk) comes in whatever the effort elsewhere
        let food_of = |h: &[f32; 5]| (0..5).map(|r| h[r] * dg[r]).sum::<f32>() + fm.herd_food;
        let ls = self.map.local_season(ci, s);
        let winter_next = ls == 2 || (ls == 1 && self.map.tamp[ci] > 8.0);
        let vq = if kit.vessel == NONE { 0.0 } else { vessel_q(mat(kit.vessel)) };
        // laying food by for winter: some is always cached, containers let far more keep
        // farmers who eat from a harvest must carry it through the lean seasons
        let crop_share = if fm.fields > 0.0 { (ys[4] * ks[4] * dg[4] / wsum).min(1.0) } else { 0.0 };
        let store_target = if winter_next { need * (0.15 + 0.75 * vq) * (1.0 + 0.8 * crop_share) } else { 0.0 };
        let target = (need + store_target - b.store * 0.5).max(need * 0.5);
        let max_e = (labour * 0.85 - upkeep_l).max(labour * 0.3);
        let e_food = if food_of(&harvest(max_e)) <= target {
            max_e
        } else {
            let (mut lo, mut hi) = (0.0f32, max_e);
            for _ in 0..14 {
                let mid = 0.5 * (lo + hi);
                if food_of(&harvest(mid)) < target {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            hi
        };
        let h = harvest(e_food);
        // stored food rots faster in the warm; sealed containers and smoking slow it
        let base = 0.12 + 0.28 * (t / 25.0).clamp(0.0, 1.0);
        // dry grain keeps far better than fresh food (and better still in pits and jars)
        // (v8) salted meat and fish keep 30% longer; grain in pits or granaries (dug by settled
        // people with a digging stick, on ground dry enough not to rot it) 40% longer
        let salt = b.stock.get(Mat::Salt as usize).copied().unwrap_or(0.0) > 0.1;
        let pits = b.settled > 0.5 && kit.dig != NONE && self.map.rain[ci] < 0.6;
        let fresh = 1.0 - crop_share;
        // (inventions step 2) meat and fish dried on racks in dry or freezing air, and food kept
        // in cold cellars dug where the winters freeze hard
        let racks = self.dries_food(b, ci);
        let cellar = self.cold_cellar(b, ci);
        let keep = (1.0 - if salt { 0.3 * fresh } else { 0.0 })
            * (1.0 - if pits { 0.4 * crop_share } else { 0.0 })
            * (1.0 - if racks { RACK_KEEP * fresh } else { 0.0 })
            * (1.0 - if cellar { CELLAR_KEEP * fresh } else { 0.0 });
        // (inventions step 1) sealed jars keep out damp, air and vermin
        let jar = kit.jar != NONE;
        let spoil = (1.0 - RECORDS_SPOIL * b.records) * base * (1.0 - 0.65 * vq) * if kit.smoke { 0.7 } else { 1.0 } * (1.0 - 0.55 * crop_share) * keep * if jar { JAR_SPOIL } else { 1.0 };
        // (v8) cheese: milk from a tame herd, kept in fired pots, lasts the winter
        let cheese = b.tame > 0.8 && b.herd > 0.1 && kit.vessel != NONE && mat(kit.vessel) == Mat::Ceramic;
        let store_cap = pop
            * (0.5 + 1.5 * vq + 1.5 * crop_share * (0.3 + 0.7 * b.settled) * if pits { 1.3 } else { 1.0 }
                + if cheese { 0.3 * b.herd.min(1.0) } else { 0.0 }
                + if jar { JAR_STORE } else { 0.0 });
        let spare = ((labour - e_food - upkeep_l) / labour).clamp(0.0, 1.0);
        let mut ec = Econ {
            cells,
            ys,
            h,
            food: 0.0,
            need,
            cold,
            upkeep,
            needed,
            used,
            food_ratio: 0.0,
            store_after: 0.0,
            spare,
            stress: 0.0,
            spoil,
            farm_food: 0.0,
            herd_food: fm.herd_food,
            fin: EconFin { dg, store0: b.store, store_cap },
        };
        ec.finish();
        ec
    }

    /// Phase 3 of a band's season: reads the world, writes the band, buffers the rest in `fx`.
    fn band_season(&self, b: &mut Band, ec: &Econ, rng: &mut Rng, fx: &mut Fx) {
        let p = self.params;
        let w = self.map.w;
        let ci = b.cell(w);
        let s = self.season();
        let pop = b.pop.max(1) as f32;
        let mut tk = if self.prof.is_some() { Some(Tick::now()) } else { None };
        self.farm_season(b, ec, rng, fx);
        fx.lap(&mut tk, 6);
        // hides and bones from hunting (small game gives a little)
        let hunted = ec.h[1] + 0.3 * ec.h[3];
        b.stock[Mat::Hide as usize] = (b.stock[Mat::Hide as usize] + hunted * 0.01).min(1.5);
        b.stock[Mat::Bone as usize] = (b.stock[Mat::Bone as usize] + hunted * 0.01).min(1.5);
        let (food_ratio, spare, stress, needed) = (ec.food_ratio, ec.spare, ec.stress, ec.needed);
        ec.apply(&mut b.d);
        // ---------------- materials: gather what is here, use up what tools consume
        for (d, mm) in DEPOSITS {
            let mi = mm as usize;
            let want = if needed & (1 << mi) != 0 { 1.2 } else { 0.4 };
            if self.map.has(ci, d) {
                b.stock[mi] = b.stock[mi].max(want);
                b.origin[mi] = ci as u32;
            }
        }
        if self.map.k_wood[ci] > 2.0 {
            b.stock[Mat::Wood as usize] = b.stock[Mat::Wood as usize].max(if needed & 1 != 0 { 1.0 } else { 0.3 });
            b.origin[Mat::Wood as usize] = ci as u32;
        }
        if self.map.k_fibre[ci] > 2.0 {
            b.stock[Mat::Fibre as usize] = b.stock[Mat::Fibre as usize].max(0.6);
            b.origin[Mat::Fibre as usize] = ci as u32;
        }
        // (inventions step 12) resinous wood is gathered where pine and birch grow, and a band
        // that boils pitch keeps more of it by
        if self.map.resin_wood(ci) {
            let mi = Mat::ResinWood as usize;
            b.stock[mi] = b.stock[mi].max(if needed & (1 << mi) != 0 { 1.0 } else { 0.3 });
            b.origin[mi] = ci as u32;
        }
        for mi in [Mat::Hide as usize, Mat::Bone as usize] {
            if hunted > 1.0 {
                b.origin[mi] = ci as u32;
            }
        }
        // metal the band can smelt or alloy itself is kept as blanks / ingots; it remembers
        // where its ore came from
        for (metal, ore) in [(Mat::Copper, Mat::CopperOre), (Mat::Tin, Mat::TinOre), (Mat::Bronze, Mat::CopperOre), (Mat::Iron, Mat::IronOre), (Mat::Salt, Mat::Brine), (Mat::Steel, Mat::IronOre)] {
            let mi = metal as usize;
            if b.d.tk.producible.has(raw(metal)) && b.stock[mi] < 0.8 {
                if metal == Mat::Steel {
                    fx.counters.steel_made += 1;
                }
                let o = b.origin[ore as usize];
                if b.stock[mi] < 0.4 || b.origin[mi] == u32::MAX {
                    b.origin[mi] = if o != u32::MAX { o } else { ci as u32 };
                }
                b.stock[mi] = 0.8;
            }
        }
        for mi in 0..NMAT {
            let mm = ALL_MATS[mi];
            if mm.is_food() || mm == Mat::Charcoal || mm == Mat::Ceramic || mm == Mat::Pitch {
                continue;
            }
            let present = self.mat_at_hand(b, ci, mm, hunted);
            // metal is durable and re-worked (blunt tools are re-hammered, scrap re-melted),
            // so it wears away far slower than stone, hide or fibre
            // (inventions step 11) gold never tarnishes and is kept: it wears as slowly as metal
            let metal = mm.is_metal() || mm == Mat::Gold;
            let mut use_ = if metal { 0.006 } else { 0.02 };
            if needed & (1 << mi) != 0 && !present {
                use_ += if metal { 0.02 } else { 0.06 };
            }
            b.stock[mi] = (b.stock[mi] - use_).max(0.0);
            if b.stock[mi] <= 0.0 {
                b.origin[mi] = u32::MAX;
            }
            b.d.demand[mi] *= 0.9;
        }
        // ---------------- demography (births / deaths; splits happen later)
        let fed = ((food_ratio - 0.75) / 0.25).clamp(0.0, 1.0);
        // settled people with soft weaning foods (porridge, milk) space births closer
        let farm_share = (ec.farm_food / ec.food.max(1e-3)).clamp(0.0, 1.0);
        let sed = 1.0 + 0.6 * b.settled * farm_share;
        let births = rng.poisson(pop * p.birth_rate * fed * (0.8 + 0.4 * spare) * sed);
        let deaths = rng.poisson(pop * p.death_rate * (1.0 - 0.05 * b.herbal) + pop * p.starve_rate * (1.0 - food_ratio).powf(1.5));
        b.pop = (b.pop + births).saturating_sub(deaths);
        self.disease_season(b, rng, fx);
        // ---------------- fire keeping
        let can_make_fire = b.d.tk.producible.has(item(Form::Fire, Mat::Wood));
        if can_make_fire {
            b.flame = true;
        } else if b.flame && rng.chance(p.flame_loss) {
            b.flame = false;
            fx.push_event(EventKind::FireLost, b.id, 0, ci as u32, NONE, u32::MAX);
        }
        let dry = (1.0 - self.map.rain[ci]).clamp(0.0, 1.0);
        let ls = self.map.local_season(ci, s);
        let dry_s = (dry + if ls == 1 { 0.2 } else if ls == 3 { -0.2 } else { 0.0 }).clamp(0.0, 1.0);
        if !b.flame && self.map.k_wood[ci] + self.map.k_plants[ci] > 10.0 {
            let pf = p.wildfire_rate * dry_s * if ls == 1 { 2.0 } else { 0.6 };
            if rng.chance(pf) && rng.chance((0.3 * b.curiosity).min(0.9)) {
                b.flame = true;
                let fire = item(Form::Fire, Mat::Wood);
                fx.first(fire, b.id, ci as u32, u32::MAX, EventKind::FirstFire, Some(EventKind::FireCaptured));
            }
        }
        fx.lap(&mut tk, 9);
        // ---------------- experiments
        self.refresh_toolkit(b);
        let lam = p.invent_rate * b.curiosity * (0.15 + spare + p.stress_invent * stress) * (pop / 25.0).sqrt();
        let n = rng.poisson(lam).min(6);
        b.d.experiments = n;
        if n > 0 {
            let att = self.attention(b);
            for _ in 0..n {
                self.experiment(b, rng, dry_s, &att, fx);
            }
        }
        // ---------------- (v9) play and refinement of what the band already makes
        // ---------------- (v9) standing: fine work earns it, time wears it away (inventions step
        // 10: food given away earns it too, at feasts and to the hungry)
        b.prestige = b.prestige * PRESTIGE_FADE
            + if b.d.kit.ornament != NONE {
                // (inventions step 11) a prized material (gold) counts on top of the work
                ornament_prestige(b.d.kit.fine + mat(b.d.kit.ornament).props().prized, pop)
            } else {
                0.0
            };
        Self::refine(b, REFINE_RATE * b.curiosity * (0.1 + spare) * (pop / 25.0).sqrt(), rng, fx);
        // craft habits fade back toward no leaning when not renewed (half-life ~85 years)
        for c in &mut b.craft {
            *c = 1.0 + (*c - 1.0) * 0.998;
        }
        fx.lap(&mut tk, 7);
        // ---------------- the sea: drift and far shores
        self.sea_season(b, rng, fx);
        // ---------------- practice / forgetting
        // (inventions step 8) records: kept while a band counts something with its tally or
        // token; they slow the forgetting of what is not practised
        if b.d.kit.mark != NONE && Self::keeps_count(b) {
            b.records += RECORDS_GAIN * (1.0 - b.records);
            fx.counters.marks_used += 1;
        }
        b.records *= RECORDS_FADE;
        // (inventions step 10) brewing ale (its food is counted in `economy`, its draw at feasts)
        if self.brews(b) {
            fx.counters.ale_brewed += 1;
        }
        b.owed *= 0.95;
        let floss = p.forget_rate * (1.0 + p.small_band_loss / pop.sqrt()) * (1.0 - RECORDS_FORGET * b.records);
        let mut forgot: Vec<u32> = Vec::new();
        for k in &mut b.known {
            if k.used {
                k.skill += p.practice * (1.0 - k.skill);
            } else {
                k.skill -= floss;
            }
            if k.quality > 0.0 {
                k.quality = (k.quality * (1.0 - QUALITY_FADE * floss)).min(k.skill.max(0.0));
            }
            if k.skill < 0.04 {
                forgot.push(k.recipe);
            }
        }
        b.d.kit_dirty = true;
        if !forgot.is_empty() {
            b.known.retain(|k| k.skill >= 0.04);
            b.d.known_ver += 1;
            b.forgotten += forgot.len() as u32;
            fx.counters.forgets += forgot.len() as u64;
            fx.acc.forgets += forgot.len() as u32;
            for r in forgot {
                let it = self.reg.recipes[r as usize].output;
                fx.push_event(EventKind::Forgot, b.id, 0, ci as u32, it, r);
            }
        }
        fx.lap(&mut tk, 9);
        // ---------------- movement
        self.refresh_toolkit(b);
        self.maybe_move(b, rng, fx);
        fx.lap(&mut tk, 8);
    }

    #[inline]
    fn prof_lap(&mut self, tk: &mut Option<Tick>, i: usize) {
        if let (Some(t), Some(p)) = (tk.as_mut(), self.prof.as_mut()) {
            p.t[i] += t.lap();
        }
    }

    /// One experiment: pick a primitive action and things at hand, see what physics says.
    /// What a band's attention turns to when it tinkers. Nothing here says what to invent; it
    /// says what is in people's hands and on their minds: the stuff of their way of getting food
    /// (more so when hungry), what their troubles call for (cold: hides, bone, firewood; no water
    /// at hand or a hard winter to store food for: things that hold), whether a material lies
    /// all around or is a carried rarity (curious bands play with novelties), and the band's own
    /// craft tradition. Physics still decides what works.
    pub(crate) fn attention(&self, b: &Band) -> [f32; NMAT] {
        use Mat::*;
        let m = &self.map;
        let ci = b.cell(m.w);
        let h = b.d.harvest;
        let tot = h.iter().sum::<f32>().max(1e-3);
        let hunger = 1.0 + 1.5 * b.d.stress.clamp(0.0, 1.0);
        let f = |r: usize| 2.0 * (h[r] / tot) * hunger;
        // a low floor: what a band notices is mostly what its own country, season and needs put
        // in front of it, so bands in different places tinker with different things
        let mut a = [ATT_FLOOR; NMAT];
        let mut add = |mm: Mat, v: f32| a[mm as usize] += v;
        // gathering plants: fibre, containers, digging sticks
        add(Plants, 1.5 * f(0));
        add(Fibre, 1.0 * f(0));
        add(Clay, 0.8 * f(0));
        add(Wood, 0.4 * f(0));
        // hunting: carcasses, hides, bone, edges and points
        add(Meat, 1.5 * f(1));
        add(Hide, 1.2 * f(1));
        add(Bone, 1.0 * f(1));
        add(Stone, 0.5 * f(1));
        add(Flint, 0.6 * f(1));
        add(Obsidian, 0.6 * f(1));
        add(Wood, 0.5 * f(1));
        // fishing: lines and nets, floats, hooks
        add(Fish, 1.5 * f(2));
        add(Fibre, 1.0 * f(2));
        add(Wood, 0.8 * f(2));
        add(Bone, 0.6 * f(2));
        // trapping small game: cord, sticks, skins
        add(Fibre, 1.0 * f(3));
        add(Wood, 0.8 * f(3));
        add(Hide, 0.5 * f(3));
        // field work: grain to cook and grind, flax to spin, stone for querns and sickles
        add(Grain, 1.5 * f(4));
        add(Flax, 0.8 * f(4));
        add(Stone, 0.4 * f(4));
        add(Clay, 0.5 * f(4));
        add(Wool, 0.8 * b.herd.min(1.0));
        let cold = (b.d.cold * 2.0).clamp(0.0, 1.5);
        add(Hide, 1.5 * cold);
        add(Bone, 0.6 * cold);
        add(Fibre, 0.5 * cold);
        add(Wood, 1.0 * cold);
        let store = ((5.0 - m.temp_winter(ci)) / 10.0).clamp(0.0, 1.0)
            + if self.near_water(ci) { 0.0 } else { 1.0 };
        add(Clay, 0.8 * store);
        add(Hide, 0.5 * store);
        add(Fibre, 0.5 * store);
        // (inventions step 13) resinous wood is wood: the pine and birch a band cuts, burns and
        // whittles draws the same eye as its other wood (step 12 split it off the wood but left it
        // at the floor, and never "at hand" even in a pine forest)
        a[ResinWood as usize] = a[Wood as usize];
        // at hand here vs carried in
        let here = |mm: Mat| match mm {
            Wood => m.k_wood[ci] > 2.0,
            ResinWood => m.resin_wood(ci),
            Fibre => m.k_fibre[ci] > 2.0,
            Meat | Hide | Bone => self.seen(1, ci, b) > 5.0,
            Plants => self.seen(0, ci, b) > 5.0,
            Fish => self.seen(2, ci, b) > 3.0,
            mm => DEPOSITS.iter().any(|&(d, m2)| m2 == mm && m.has(ci, d)),
        };
        // striking stones catch the eye where they crop out: green and grey ores (pigments,
        // heavy shiny lumps), glassy obsidian
        let lies = |mm: Mat| DEPOSITS.iter().any(|&(d, m2)| m2 == mm && m.has(ci, d));
        let mut add = |mm: Mat, v: f32| {
            if lies(mm) {
                a[mm as usize] += v;
            }
        };
        add(CopperOre, 0.8);
        add(TinOre, 0.5);
        // red and yellow ochre is a pigment to most; a band that already smelts metal from
        // stone looks at heavy rust-red lumps with new eyes
        let smelts = [Mat::Copper, Mat::Tin, Mat::Bronze].iter().any(|&mm| b.d.tk.producible.has(raw(mm)));
        add(IronOre, if smelts { 1.0 } else { 0.2 });
        add(Obsidian, 0.5);
        // (inventions step 11) bright yellow grains in a stream bed or a quartz seam catch anyone's eye
        add(Gold, 1.0);
        // a band that gets spongy iron out of its furnace has that lump on its mind
        let blooms = b.d.tk.producible.has(item(Form::Bloom, Mat::Iron));
        if blooms {
            a[Iron as usize] += 2.0;
        }
        let carried = 0.2 + 0.5 * b.curiosity;
        for (mi, v) in a.iter_mut().enumerate() {
            let mm = ALL_MATS[mi];
            if !here(mm) && !(blooms && mm == Iron) {
                *v *= carried;
            }
            // attention is focused: what is on people's minds crowds out the rest
            *v = (*v * b.craft.get(mi).copied().unwrap_or(1.0)).powf(1.0);
        }
        a
    }

    /// Doing (inventing or being shown) a recipe deepens a band's habit of working its materials
    /// that way. Tradition is relative: leanings are renormalised to a mean of 1.
    fn reinforce_craft(b: &mut Band, r: &Recipe, by: f32) {
        crate::band::fit_craft(&mut b.craft);
        for &i in r.inputs.iter() {
            if i != NONE && !matches!(form(i), Form::Fire | Form::Kiln) {
                let c = &mut b.craft[mat(i) as usize];
                *c = (*c * by).min(4.0);
            }
        }
        let ai = NMAT + (r.action as usize).min(2);
        b.craft[ai] = (b.craft[ai] * by.sqrt()).min(3.0);
        let mean = b.craft[..NMAT].iter().sum::<f32>() / NMAT as f32;
        if mean > 0.0 {
            for c in &mut b.craft[..NMAT] {
                *c = (*c / mean).max(0.2);
            }
        }
    }

    fn experiment(&self, b: &mut Band, rng: &mut Rng, dryness: f32, att: &[f32; NMAT], fx: &mut Fx) {
        fx.counters.experiments += 1;
        fx.acc.experiments += 1;
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
        // what gets picked up follows the band's attention (see `attention`)
        let pick = |rng: &mut Rng, v: &[Item]| {
            let ws: Vec<f32> = v.iter().map(|&i| att[mat(i) as usize]).collect();
            v[rng.weighted(&ws).unwrap_or_else(|| rng.below(v.len()))]
        };
        // salience: things that already work as tools get tried as tools; hard cobbles too
        let tool_w: Vec<f32> = pool
            .iter()
            .map(|&i| match form(i) {
                Form::Hammer | Form::Edge | Form::Point => 3.0,
                Form::Raw if mat(i).props().hard >= 5.0 => 1.5,
                _ => 0.3,
            })
            .collect();
        // which way of working: heat (only with a fire at hand), shape, combine - each band's
        // habits tilt the mix
        let cr = |k: usize| b.craft.get(NMAT + k).copied().unwrap_or(1.0);
        let wh = if fires.is_empty() { 0.0 } else { 0.15 * cr(Action::Heat as usize) };
        let ws = 0.45 * cr(Action::Shape as usize);
        let wc = 0.40 * cr(Action::Combine as usize);
        let roll = rng.f32() * (wh + ws + wc);
        // what goes into a fire: people soon learn that fibre, hide and bone just burn away, while
        // earths and stones come out changed - so the fire gets fed stones, clays, food, pots
        // (attention still tilts it, more loosely than at the work bench)
        let pick_heat = |rng: &mut Rng, v: &[Item]| {
            let ws: Vec<f32> = v
                .iter()
                .map(|&i| {
                    let m = mat(i);
                    let keeps = if matches!(m, Mat::Fibre | Mat::Hide | Mat::Bone) { 0.3 } else if m.is_food() || m.props().fuel_t > 0.0 { 1.0 } else { 1.5 };
                    // a slaggy bloom fresh from the furnace begs to go straight back in the fire
                    // (inventions step 11b) and so does a crucible charged with metal to melt
                    // (inventions step 12) and a bar or blade of iron or steel, which is only
                    // worked hot
                    let unfinished = if matches!(form(i), Form::Bloom | Form::Crucible) {
                        8.0
                    } else if matches!(form(i), Form::Raw | Form::Edge) && physics::hot_worked(m) {
                        // (inventions step 12b) a smith with a bar of iron keeps it in the fire
                        // as he does the bloom it came from: there is nothing else to do with it
                        8.0
                    } else {
                        1.0
                    };
                    att[m as usize].sqrt() * keeps * unfinished
                })
                .collect();
            v[rng.weighted(&ws).unwrap_or_else(|| rng.below(v.len()))]
        };
        // need-driven tinkering: when putting things together, people reach for the parts of
        // what their way of life calls for - hunters for points, edges and shafts, trappers for
        // sticks and cord, fishers for cord (lines, nets, floats), the cold for points to sew
        // with, the storing for containers
        let need_w = {
            let h = b.d.harvest;
            let tot = h.iter().sum::<f32>().max(1e-3);
            let hunger = 1.0 + 1.5 * b.d.stress.clamp(0.0, 1.0);
            let sh = move |r: usize| NEED_FORM * (h[r] / tot) * hunger;
            let cold = (b.d.cold * 2.0).clamp(0.0, 1.5);
            let ci = b.cell(self.map.w);
            let store = ((5.0 - self.map.temp_winter(ci)) / 10.0).clamp(0.0, 1.0) + if self.near_water(ci) { 0.0 } else { 1.0 };
            // (inventions step 5) seafarers with a plank hull on the shore work at it: they reach
            // for the hull, pitch for its seams and metal points to fasten it, the more the
            // better they know the sea
            let hull = if b.d.kit.plank != NONE && self.near_water(ci) { NEED_FORM * b.seacraft } else { 0.0 };
            move |i: Item| match form(i) {
                Form::Point => 1.0 + sh(1) + NEED_FORM * 0.5 * cold + if mat(i).is_metal() { hull } else { 0.0 },
                Form::Plank => 1.0 + hull,
                Form::Raw if mat(i) == Mat::Pitch => 1.0 + hull,
                Form::Edge => 1.0 + 0.8 * sh(1),
                Form::Shaft => 1.0 + 0.5 * sh(1) + sh(3) + 0.5 * hull,
                Form::Cord => 1.0 + 0.5 * sh(3) + sh(2) + 0.3 * NEED_FORM * cold,
                Form::Vessel => 1.0 + 0.5 * NEED_FORM * store + if mat(i) == Mat::Fibre { 0.5 * hull } else { 0.0 },
                // (inventions step 12) the cold reach for the loom and its cloth, as for needles
                // (inventions step 12b) and seafarers with a plank hull reach for a mast and
                // something to hang from it: cloth or plaited matting
                Form::Loom => 1.0 + NEED_FORM * 0.5 * cold,
                Form::Cloth => 1.0 + NEED_FORM * 0.5 * cold + 0.5 * hull,
                _ => 1.0,
            }
        };
        let pick_c = |rng: &mut Rng, v: &[Item]| {
            let ws: Vec<f32> = v.iter().map(|&i| att[mat(i) as usize] * need_w(i)).collect();
            v[rng.weighted(&ws).unwrap_or_else(|| rng.below(v.len()))]
        };
        let (action, inputs): (Action, Inputs) = if roll < wh {
            let mut t1 = pick_heat(rng, &pool);
            let t2 = if rng.chance(0.25) { pick_heat(rng, &pool) } else { NONE };
            if t1 == t2 {
                return;
            }
            // a bloom is reworked in the furnace it came out of, in its hottest fire
            // (inventions step 11b) and so is a charged crucible
            // (inventions step 12) and so is iron or steel to be forged
            let hot = |i: Item| i != NONE && (matches!(form(i), Form::Bloom | Form::Crucible) || (matches!(form(i), Form::Raw | Form::Edge) && physics::hot_worked(mat(i))));
            let smithing = hot(t1) || hot(t2);
            let k = if !kilns.is_empty() && (smithing || rng.chance(0.6)) { pick(rng, &kilns) } else { NONE };
            // the fire: a band that knows a hotter fire (charcoal) mostly banks that one for
            // firing work; otherwise any fire it keeps
            let hottest = fires.iter().copied().max_by_key(|&f| (crate::materials::fire_temp(mat(f), None) * 10.0) as i64);
            let fire = match hottest {
                Some(f) if smithing || rng.chance(0.6) => f,
                _ => fires[rng.below(fires.len())],
            };
            // a band with bellows works them into a hot charcoal furnace
            let mut t2 = t2;
            // (inventions step 11b) metal melting in a crucible wants somewhere to be poured
            // (inventions step 12) molten copper (or tin) in a crucible invites the other metal
            // stirred in: bronze
            if t2 == NONE && form(t1) == Form::Crucible && matches!(mat(t1), Mat::Copper | Mat::Tin) && rng.chance(0.5) {
                let partner = raw(if mat(t1) == Mat::Copper { Mat::Tin } else { Mat::Copper });
                if pool.contains(&partner) {
                    t2 = partner;
                }
            }
            if t2 == NONE && form(t1) == Form::Crucible && rng.chance(0.5) {
                // a fired mould before a raw clay one: a mould that has already been through the
                // fire is the one that takes the pour (fixes the 11b first run, which always
                // reached for the unfired clay mould and so never cast)
                let fired = |i: Item| form(i) == Form::Mould && mat(i).props().fires_to.is_none();
                let pick = pool.iter().copied().find(|&i| fired(i)).or_else(|| pool.iter().copied().find(|&i| form(i) == Form::Mould));
                if let Some(md) = pick {
                    t2 = md;
                }
            }
            // a hot spongy lump from the furnace invites a blow with the hammer stone
            // (inventions step 12) so does a bar or blade of iron or steel at white heat
            let forgeable = matches!(form(t1), Form::Raw | Form::Edge) && physics::hot_worked(mat(t1));
            // (inventions step 12b) a bar at white heat is only ever worked under the hammer
            if t2 == NONE && (forgeable || (form(t1) == Form::Bloom && rng.chance(0.5))) {
                if let Some(hm) = pool.iter().copied().find(|&i| form(i) == Form::Hammer && mat(i).props().hard >= 5.0) {
                    t2 = hm;
                }
            }
            // (inventions step 13) a smith who has bellows works the bar under the hammer at the
            // open charcoal hearth they blow, as often as in the closed furnace: the bellows go
            // where the chamber would be (see `physics::smith_hearth`)
            let mut k = k;
            if forgeable && t2 != NONE && form(t2) == Form::Hammer && mat(fire).props().fuel_t >= 900.0 && rng.chance(0.5) {
                if let Some(bw) = pool.iter().copied().find(|&i| form(i) == Form::Bellows) {
                    k = bw;
                }
            }
            // (inventions step 12b) resinous wood in the fire sweats sticky tar that begs to be
            // caught: it goes into a fired pot to be cooked out (the hearth-side trick behind
            // birch and pine tar), as a charged crucible invites its mould
            if t2 == NONE && form(t1) == Form::Raw && mat(t1).props().resin >= 5.0 && rng.chance(0.5) {
                if let Some(pot) = pool.iter().copied().find(|&i| form(i) == Form::Vessel && mat(i) == Mat::Ceramic) {
                    t2 = pot;
                }
            }
            if t2 == NONE && k != NONE && form(t1) != Form::Bellows && mat(fire).props().fuel_t >= 900.0 && rng.chance(0.5) {
                if let Some(bw) = pool.iter().copied().find(|&i| form(i) == Form::Bellows) {
                    t2 = bw;
                    // what a forced furnace is for: cooking stones. Its keepers feed it the
                    // heavy, coloured stones they have (attention still picks which)
                    if form(t1) == Form::Raw && mat(t1).props().reduces_to.is_none() && rng.chance(0.5) {
                        let ores: Vec<Item> = pool.iter().copied().filter(|&i| form(i) == Form::Raw && mat(i).props().reduces_to.is_some()).collect();
                        if !ores.is_empty() {
                            t1 = pick(rng, &ores);
                        }
                    }
                }
            }
            (Action::Heat, [t1, t2, fire, k])
        } else if roll < wh + ws {
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
            let a = pick_c(rng, src);
            let mut bb = pick_c(rng, src);
            let mut c = if rng.chance(0.45) { pick_c(rng, src) } else { NONE };
            // (inventions step 13) pitch is boiled out in a fired pot and comes out hot and
            // sticky: half the time it is daubed on a fired pot (the one it was cooked in), as
            // resinous wood goes into the pot and a charged crucible to its mould. Not when it
            // was picked up for a plank hull
            let plank = |i: Item| i != NONE && form(i) == Form::Plank;
            if a == raw(Mat::Pitch) && !plank(bb) && !plank(c) && rng.chance(0.5) {
                if let Some(pot) = src.iter().copied().find(|&i| form(i) == Form::Vessel && mat(i) == Mat::Ceramic) {
                    (bb, c) = (pot, NONE);
                }
            }
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
        let ctx = Ctx { dryness, water: self.near_water(b.cell(self.map.w)) };
        let Some(out) = physics::apply(action, &inputs, &ctx) else { return };
        if b.d.tk.producible.has(out.out) || !rng.chance(out.p) {
            return;
        }
        if fx.invented.contains(&out.out) {
            return;
        }
        let ci = b.cell(self.map.w) as u32;
        // (registered and learned when the season's effects are applied)
        if let Some(rid) = self.reg.find(action, &inputs) {
            if b.knows(rid) {
                return;
            }
        }
        fx.invented.push(out.out);
        fx.ops.push(Op::Invent { action, inputs, out: out.out, p: out.p, cell: ci });
    }

    fn cell_score(&self, b: &Band, c: usize) -> f32 {
        let m = &self.map;
        if !m.is_land(c) {
            return -1.0;
        }
        let kit = &b.d.kit;
        let dg = kit.digest;
        let pot = |j: usize| {
            let v = self.seen4(j, b);
            3.0 * kit.plants_m * dg[0] * v[0]
                + 2.5 * kit.game_m * dg[1] * v[1]
                + 2.5 * kit.fish_m * dg[2] * v[2]
                + 2.5 * kit.small_m * dg[3] * v[3]
                + if b.sow > 0.0 { 3.5 * kit.farm_m * dg[4] * 0.6 * self.farm_prospect(b, j) } else { 0.0 }
        };
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
        // with a boat, the sea next to a coastal camp is food too
        if kit.boat_q > 0.0 {
            let mut sea = 0.0;
            for (nx, ny) in nbrs4(x, y, m.w, m.h) {
                let j = ny * m.w + nx;
                if !m.is_land(j) {
                    sea += self.seen(2, j, b);
                }
            }
            s += 2.5 * kit.fish_m * dg[2] * 0.25 * sea * kit.boat_q;
        }
        let crowd = self.crowd(c, b) as f32;
        s /= 1.0 + crowd / (b.pop as f32 + 10.0);
        let winter = m.temp_winter(c);
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

    /// How hard life at home pushes a band to look elsewhere: hunger, cold and crowding.
    fn push_out(&self, b: &Band) -> f32 {
        let c = b.cell(self.map.w);
        let crowd = self.crowd(c, b) as f32 / (b.pop as f32 + 10.0);
        (b.d.stress + 0.5 * crowd + b.pressed).clamp(0.0, 1.0)
    }

    /// Is any of the 8 cells around `c` open sea (not just a river)?
    pub fn coastal(&self, c: usize) -> bool {
        let m = &self.map;
        nbrs8(c % m.w, c / m.w, m.w, m.h).any(|(a, bb)| !m.is_land(bb * m.w + a))
    }

    /// Chance that people in a boat of quality `q` come through `sea` cells of open water.
    /// Better boats (higher q) are both faster and safer; a raft on a long crossing often is not.
    pub fn sea_survival(q: f32, sea: f32) -> f32 {
        Self::sea_survival_x(q, 0.0, 0.0, sea)
    }
    /// ... for people with seafaring experience `xp` (0..1), and with `fed` (0..1) of the food
    /// the trip wants put by (inventions step 2: continuous; before, all or nothing)
    pub fn sea_survival_x(q: f32, xp: f32, fed: f32, sea: f32) -> f32 {
        let prep = 1.0 - 0.3 * fed.clamp(0.0, 1.0);
        (-0.02 * sea * prep * (1.4 - q - 0.25 * xp).max(0.3).powi(2)).exp()
    }
    /// How far out over open water a band can go (cells): the boat, and how well they handle it.
    pub fn sea_range(q: f32, xp: f32) -> f32 {
        if q <= 0.0 {
            0.0
        } else {
            2.0 + 10.0 * q + 12.0 * xp * q
        }
    }

    /// (v8) How safe a band's boat is: an outrigger keeps it upright (+0.15).
    pub fn boat_safety(kit: &Kit) -> f32 {
        kit.boat_q + if kit.outrigger != NONE { 0.15 } else { 0.0 }
    }
    /// (v8) How far a band can go: a sail carries it 1.6 times as far, for people who know the
    /// sea well enough to use the wind (seacraft above 0.4).
    pub fn boat_range(kit: &Kit, xp: f32) -> f32 {
        let sail = Self::sail_usable(kit, xp);
        // (inventions step 5) a ship under sail holds its course on a long leg
        let ship = if sail && kit.ship != NONE { SHIP_RANGE } else { 1.0 };
        Self::sea_range(kit.boat_q, xp) * if sail { 1.6 } else { 1.0 } * ship
    }

    /// A trip out over open water: pick a heading from a coastal camp and paddle until land shows
    /// up or the boat's range runs out. Returns the landing cell and how much open water was
    /// crossed. Without a known far shore the heading is pure chance; with one (`aim`, seen from
    /// the coast or heard of) they steer for it, roughly. Range grows with the boat.
    pub fn voyage(&self, b: &Band, rng: &mut Rng, aim: Option<u32>) -> Option<(usize, f32)> {
        self.voyage_r(b, rng, aim, self.usual_reach(b)).map(|v| (v.0, v.1))
    }
    /// How far band `b` goes on an ordinary trip: its boat's range, (v9) pushed by a bold crew
    /// past the safe range (and the sea takes its toll).
    pub fn usual_reach(&self, b: &Band) -> f32 {
        Self::boat_range(&b.d.kit, b.seacraft) * (AMBITION_REACH * b.ambition).exp()
    }
    /// How far band `b` goes on a planned voyage (a move or a daughter band setting out): the
    /// usual reach, or (inventions step 5b) for a well-provisioned ship crew, as far as its food
    /// and water last if that is farther.
    pub fn planned_reach(&self, b: &Band) -> f32 {
        let r = self.usual_reach(b);
        self.expedition_reach(b).map_or(r, |e| r.max(e))
    }
    /// ... (inventions step 3) and whether the crew turned toward signs of land on the way:
    /// out on the water a band with pilotage reads land up to `PILOT_SIGNS` x pilot cells off
    /// (another landmass than the one it left) and steers for it. Signs never add range.
    /// A planned voyage (`planned_reach`).
    pub fn voyage_x(&self, b: &Band, rng: &mut Rng, aim: Option<u32>) -> Option<(usize, f32, bool)> {
        self.voyage_r(b, rng, aim, self.planned_reach(b))
    }
    /// A voyage of at most `range` open-water cells.
    pub fn voyage_r(&self, b: &Band, rng: &mut Rng, aim: Option<u32>, range: f32) -> Option<(usize, f32, bool)> {
        let kit = &b.d.kit;
        let m = &self.map;
        if kit.boat_q <= 0.0 || !self.near_water(b.cell(m.w)) {
            return None;
        }
        // (v8) wayfinding: seasoned seafarers read stars, swells and birds, and steer half as wide;
        // (inventions step 4) star paths steady the heading further
        let err = Self::heading_error(b);
        let (x0, y0) = (b.x as f32 + 0.5, b.y as f32 + 0.5);
        let ang = match aim {
            Some(t) => {
                let (tx, ty) = ((t as usize % m.w) as f32 + 0.5, (t as usize / m.w) as f32 + 0.5);
                (ty - y0).atan2(tx - x0) + err * rng.gauss()
            }
            None => rng.f32() * std::f32::consts::TAU,
        };
        let (mut dx, mut dy) = (ang.cos(), ang.sin());
        let (mut x0, mut y0) = (x0, y0);
        let step = 0.7f32;
        let mut sea = 0.0f32;
        let mut t = step;
        let sign_r = PILOT_SIGNS * b.pilot;
        let signs = if sign_r >= 1.0 { Some(self.sea_signs()) } else { None };
        let own_lm = m.landmass_of(b.cell(m.w));
        let mut steered = false;
        while t < range + 4.0 {
            let (px, py) = (x0 + dx * t, y0 + dy * t);
            if px < 0.0 || py < 0.0 || px >= m.w as f32 || py >= m.h as f32 {
                return None;
            }
            let c = py as usize * m.w + px as usize;
            if m.is_land(c) {
                if sea > 0.0 {
                    return Some((c, sea, steered));
                }
                if t > 2.5 && !steered {
                    return None; // headed inland: not a voyage
                }
            } else {
                sea += step;
                if sea > range {
                    return None;
                }
                if let (false, Some(sg)) = (steered, signs) {
                    let (d, n) = (sg.dist[c], sg.near[c]);
                    if n != u32::MAX && (d as f32) <= sign_r && m.landmass_of(n as usize) != own_lm {
                        // birds, clouds, swell: turn toward the land they point to, roughly
                        let (tx, ty) = ((n as usize % m.w) as f32 + 0.5, (n as usize / m.w) as f32 + 0.5);
                        let a = (ty - py).atan2(tx - px) + err * rng.gauss();
                        (dx, dy) = (a.cos(), a.sin());
                        (x0, y0, t) = (px, py, 0.0);
                        steered = true;
                    }
                }
            }
            t += step;
        }
        None
    }

    /// (v9) Band `b` learns something of the sea: seacraft moves `gain` of the way to 1.
    fn learn_sea(b: &mut Band, gain: f32) {
        b.seacraft = (b.seacraft + gain * (1.0 - b.seacraft)).min(1.0);
    }

    /// Open water takes its toll: maybe some (or all) of `b` drown on a crossing of `sea` cells.
    /// Returns the number lost.
    fn sea_toll(&self, b: &mut Band, sea: f32, rng: &mut Rng, fx: &mut Fx) -> u32 {
        // (inventions step 2) how much food is put by, and how far it and the water last
        let home = b.d.sailed_from.map_or(b.cell(self.map.w), |c| c as usize);
        let (fed, endurance) = self.provisions(b, home);
        let prepared = fed >= 1.0;
        b.d.sailed_sea = sea;
        b.d.sailed_tech = Self::voyage_tech(b, prepared);
        if prepared {
            fx.counters.voyages_provisioned += 1;
        }
        if sea > endurance {
            fx.counters.past_endurance += 1;
        }
        let p = Self::sea_survival_x(Self::boat_safety(&b.d.kit), b.seacraft, fed, sea) * Self::endurance_factor(sea, endurance);
        if b.far_shore.is_some() && self.rumour_age(b) > RUMOUR_YEARS {
            b.d.sailed_tech |= TECH_ROUTE; // word kept only as remembered sailing directions
        }
        if b.d.sailed_tech & TECH_SKY != 0 {
            fx.counters.sky_voyages += 1;
        }
        if b.d.sailed_tech & TECH_SHIP != 0 {
            fx.counters.ship_voyages += 1;
        }
        if rng.chance(p) {
            if Self::sea_lessons(b, sea) {
                fx.counters.deep_water_legs += 1;
            }
            return 0;
        }
        // swamped within sight of land, many struggle ashore; far out, few do
        let near = (sea / 6.0).clamp(0.2, 1.0);
        let lost = ((b.pop as f32 * rng.range(0.3, 1.05) * near) as u32).min(b.pop);
        b.pop -= lost;
        fx.counters.sea_disasters += 1;
        fx.counters.lost_at_sea += lost as u64;
        fx.push_event(EventKind::LostAtSea, b.id, lost, b.cell(self.map.w) as u32, NONE, u32::MAX);
        lost
    }

    /// What a crew that made a crossing of `sea` open-water cells learns from it. Returns
    /// whether it was a deep-water leg (inventions step 5b: a ship out of sight of land).
    pub(crate) fn sea_lessons(b: &mut Band, sea: f32) -> bool {
        // a crossing made is a lesson learned, (v9) the more so the longer it was
        Self::learn_sea(b, VOYAGE_LESSON + VOYAGE_LESSON_PER_CELL * sea.min(10.0));
        // (inventions step 3) and days out of sight of land teach the signs of it
        if sea < PILOT_OUT {
            return false;
        }
        // (inventions step 4) a pilot who sails on through the night steers by the stars
        // (as far as its pilotage lets it read the night sky: a novice learns little)
        let read = (b.pilot / SKY_PILOT).min(1.0);
        Self::learn_sky(b, SKY_PER_CELL * read * sea.min(PILOT_CELLS_MAX));
        if b.d.kit.ship != NONE {
            // (inventions step 5b) deep-water seamanship: days and nights out of sight of land in
            // a ship teach handling and the signs of land, the whole leg long
            Self::learn_sea(b, DEEP_SEA_LESSON * (sea - PILOT_OUT));
            Self::learn_pilot(b, DEEP_PILOT * PILOT_PER_CELL * sea);
            true
        } else {
            Self::learn_pilot(b, PILOT_PER_CELL * sea.min(PILOT_CELLS_MAX));
            false
        }
    }

    /// The current year, as stored with a rumour of a far shore.
    fn year_u16(&self) -> u16 {
        (self.tick / 4).min(u16::MAX as u32) as u16
    }

    /// Years since band `b`'s far shore was last seen or confirmed.
    fn rumour_age(&self, b: &Band) -> u32 {
        (self.year_u16().saturating_sub(b.far_shore_year)) as u32
    }

    /// (v9) Band `b` sees, or hears first-hand of, land at cell `c` on another landmass. A
    /// sighting of the landmass already in mind confirms it (fresh again); a different shore
    /// replaces it unless the one held is nearer and still fresh.
    fn sight_shore(&self, b: &mut Band, c: usize) {
        let m = &self.map;
        if let Some(f) = b.far_shore {
            let f = f as usize;
            let here = b.cell(m.w);
            let same = m.landmass_of(f) == m.landmass_of(c);
            if !same && dist_cells(here, f, m.w) < dist_cells(here, c, m.w) && self.rumour_age(b) < RUMOUR_YEARS / 2 {
                return;
            }
        }
        b.far_shore = Some(c as u32);
        b.far_shore_year = self.year_u16();
    }

    /// The sea in a coastal band's year: looking out from the shore (or from the fishing grounds)
    /// people may make out land on the horizon; and fishing parties are sometimes blown out to sea
    /// - most drown or wash back, some come ashore somewhere new, a few find their way home with
    /// news of it.
    pub(crate) fn sea_season(&self, b: &mut Band, rng: &mut Rng, fx: &mut Fx) {
        let m = &self.map;
        let ci = b.cell(m.w);
        b.seacraft *= 0.998; // what is not practised is lost, generation by generation
        b.pilot *= PILOT_FADE;
        b.sky *= SKY_FADE; // (inventions step 4) every band: renewed only by sowing and night sailing
        // (v9) news nobody has confirmed for a lifetime or two is forgotten; (inventions step 3)
        // pilots keep it longer, as sailing directions
        if b.far_shore.is_some() && self.rumour_age(b) as f32 > Self::rumour_keep(b) {
            if Self::charts(b) {
                // (inventions step 9) unless it is on a chart still in use: read again, renewed
                b.far_shore_year = self.year_u16();
                fx.counters.routes_kept += 1;
            } else {
                b.far_shore = None;
                fx.counters.routes_faded += 1;
            }
        }
        if !self.coastal(ci) {
            return;
        }
        let q = b.d.kit.boat_q;
        let h = b.d.harvest;
        let fish_share = h[2] / h.iter().sum::<f32>().max(1e-3);
        if q > 0.0 && fish_share > 0.1 {
            // a life on the water: currents, weather signs, how far a boat can be pushed
            b.seacraft = (b.seacraft + 0.01 * fish_share * q * (1.0 - b.seacraft)).min(1.0);
            // (inventions step 3) fishing grounds out of sight of land: birds, swells and cloud
            // that show where the land is are the way home
            if 1.0 + 3.0 * q + 3.0 * b.seacraft * q > PILOT_OUT {
                Self::learn_pilot(b, PILOT_FISH * fish_share);
            }
        }
        let xp = b.seacraft;
        let own_lm = m.landmass_of(ci);
        // a look out to sea, about once a year, more often for the curious; (v9) people keep
        // looking even when they already know of land out there
        if rng.chance((0.25 * b.curiosity).min(0.9)) {
            let mut sight = 3.0 + (m.height.get(ci).copied().unwrap_or(0.0) / 500.0).clamp(0.0, 3.0);
            if q > 0.0 && fish_share > 0.1 {
                sight += 1.0 + 3.0 * q + 3.0 * xp * q; // seen from out on the fishing grounds
            }
            let a0 = rng.f32() * std::f32::consts::TAU;
            'rays: for k in 0..16 {
                let ang = a0 + k as f32 * std::f32::consts::TAU / 16.0;
                let (dx, dy) = (ang.cos(), ang.sin());
                let (x0, y0) = (b.x as f32 + 0.5, b.y as f32 + 0.5);
                let mut sea = 0.0f32;
                let mut t = 0.7f32;
                while t < sight + 2.0 {
                    let (px, py) = (x0 + dx * t, y0 + dy * t);
                    if px < 0.0 || py < 0.0 || px >= m.w as f32 || py >= m.h as f32 {
                        break;
                    }
                    let c = py as usize * m.w + px as usize;
                    if m.is_land(c) {
                        if sea >= 1.4 && m.landmass_of(c) != own_lm {
                            self.sight_shore(b, c);
                            fx.counters.sightings += 1;
                            break 'rays;
                        }
                        if sea > 0.0 || t > 2.0 {
                            break;
                        }
                    } else {
                        sea += 0.7;
                        if sea > sight {
                            break;
                        }
                    }
                    t += 0.7;
                }
            }
        }
        // scouting: seasoned seafarers with good boats sometimes send a party out to look for
        // land beyond the horizon; most find nothing, some do not come back
        if q > 0.5 && xp > 0.3 && b.pop >= 20 && rng.chance(0.01 * xp) {
            fx.counters.scouting += 1;
            let found = self.voyage(b, rng, None);
            let sea = found.map(|f| f.1).unwrap_or(Self::boat_range(&b.d.kit, xp));
            if !rng.chance(Self::sea_survival_x(Self::boat_safety(&b.d.kit), xp, 1.0, 2.0 * sea)) {
                let party = 5.min(b.pop / 4);
                b.pop -= party;
                fx.counters.scouts_lost += 1;
                fx.counters.sea_disasters += 1;
                fx.counters.lost_at_sea += party as u64;
                fx.push_event(EventKind::LostAtSea, b.id, party, ci as u32, NONE, u32::MAX);
            } else if let Some((c, sea)) = found {
                Self::learn_sea(b, SCOUT_LESSON); // (v9) the party comes home wiser
                Self::learn_pilot(b, PILOT_SCOUT); // (inventions step 3) and knows the signs
                if sea >= 1.4 && self.map.landmass_of(c) != own_lm {
                    self.sight_shore(b, c);
                    fx.counters.scout_finds += 1;
                }
            }
        }
        let m = &self.map;
        // drift: only people who fish from boats are out there to be blown away
        if q <= 0.0 || fish_share < 0.1 || b.pop < 15 {
            return;
        }
        let pd = 0.006 * fish_share * (1.0 - 0.5 * q) * (1.0 - 0.6 * xp);
        if !rng.chance(pd) {
            return;
        }
        fx.counters.drifts += 1;
        let party = (5 + rng.below(7) as u32).min(b.pop / 3);
        let blown = 3.0 + 15.0 * rng.f32();
        let ang = rng.f32() * std::f32::consts::TAU;
        let (dx, dy) = (ang.cos(), ang.sin());
        let (x0, y0) = (b.x as f32 + 0.5, b.y as f32 + 0.5);
        let mut sea = 0.0f32;
        let mut t = 0.7f32;
        let mut landing = None;
        while sea <= blown {
            let (px, py) = (x0 + dx * t, y0 + dy * t);
            if px < 0.0 || py < 0.0 || px >= m.w as f32 || py >= m.h as f32 {
                break;
            }
            let c = py as usize * m.w + px as usize;
            if m.is_land(c) {
                if sea > 0.0 {
                    landing = Some(c);
                    break;
                }
            } else {
                sea += 0.7;
            }
            t += 0.7;
        }
        let survive = landing.is_some() && rng.chance(Self::sea_survival_x(0.7 * q, xp, 0.0, sea));
        if !survive {
            b.pop -= party;
            fx.counters.sea_disasters += 1;
            fx.counters.lost_at_sea += party as u64;
            fx.push_event(EventKind::LostAtSea, b.id, party, ci as u32, NONE, u32::MAX);
            return;
        }
        let c = landing.unwrap();
        if sea < 2.0 || self.map.landmass_of(c) == own_lm && sea < 4.0 {
            return; // washed back onto a known shore: a fright, nothing more
        }
        let far_lm = self.map.landmass_of(c) != own_lm;
        if rng.chance(0.3 + 0.4 * q) {
            // they make it home and tell of what they saw
            fx.counters.drift_returns += 1;
            Self::learn_pilot(b, PILOT_DRIFT); // (inventions step 3) and how they found the way
            if far_lm {
                self.sight_shore(b, c);
            }
            return;
        }
        // they stay where they came ashore, with what they carry in their heads and hands
        fx.counters.drift_landings += 1;
        if far_lm {
            fx.counters.crossings += 1;
            Self::ledger_crossing(sea, fx);
        }
        b.pop -= party;
        let id = 0; // assigned when applied
        let known: Vec<Known> = b
            .known
            .iter()
            .filter(|kn| rng.chance(0.3 + 0.6 * kn.skill))
            .map(|kn| Known { skill: kn.skill * 0.8, quality: kn.quality * QUALITY_SPLIT, from: FROM_PARENT, since: self.tick, ..*kn })
            .collect();
        let nb = Band {
            id,
            lineage: b.lineage,
            parent: b.id,
            gen: b.gen + 1,
            anc: [b.parent, b.anc[0], b.anc[1]],
            ancestry: b.ancestry.clone(),
            x: (c % self.map.w) as u16,
            y: (c / self.map.w) as u16,
            pop: party,
            store: 0.5,
            curiosity: b.curiosity,
            ambition: b.ambition,
            seacraft: b.seacraft,
            pilot: b.pilot + PILOT_DRIFT * (1.0 - b.pilot),
            sky: b.sky,
            records: b.records,
            flame: b.flame && rng.chance(0.3),
            known,
            stock: b.stock.iter().map(|v| v * 0.3).collect(),
            origin: b.origin.clone(),
            craft: b.craft.clone(),
            born: self.tick,
            ..Default::default()
        };
        let mut nb = nb;
        nb.d.sailed_from = Some(b.cell(self.map.w) as u32);
        nb.d.sailed_sea = sea;
        nb.d.sailed_tech = TECH_ADRIFT;
        fx.ops.push(Op::Drift(Box::new(nb), b.id, c as u32));
    }

    fn maybe_move(&self, b: &mut Band, rng: &mut Rng, fx: &mut Fx) {
        let m = &self.map;
        let (x, y) = (b.x as usize, b.y as usize);
        let cur = b.cell(m.w);
        let mut best = cur;
        // a band recently driven off or menaced holds its ground less
        let inertia = 1.0 + (self.params.move_inertia - 1.0) * (1.0 - b.pressed);
        // people with houses, stores and fields leave them only for much better ground
        let inertia = inertia * (1.0 + 1.5 * b.settled * b.d.farm_share);
        b.pressed *= 0.8;
        let mut best_s = self.cell_score(b, cur) * inertia;
        // the effort of getting there: rough and high country costs more than flat, a boat on a
        // river makes the river a road (see `terrain`)
        let boat = b.d.kit.boat_q > 0.0;
        let effort = |c: usize| 1.0 + terrain::MOVE_EFFORT * (self.travel_cost(cur, c, boat) - 1.0).max(0.0);
        let mut cands: Vec<(usize, f32)> = nbrs8(x, y, m.w, m.h).map(|(a, c)| c * m.w + a).map(|c| (c, effort(c))).collect();
        // occasionally scout further (v8: riders on open land twice as far)
        let sr = if self.mounted(b) > 0.5 { 6 } else { 3 };
        let dx = rng.below(2 * sr + 1) as i32 - sr as i32;
        let dy = rng.below(2 * sr + 1) as i32 - sr as i32;
        let (sx, sy) = (x as i32 + dx, y as i32 + dy);
        if sx >= 0 && sy >= 0 && (sx as usize) < m.w && (sy as usize) < m.h {
            let c = sy as usize * m.w + sx as usize;
            // (v9) on foot, never across water
            if m.walkable(cur, c) {
                cands.push((c, effort(c)));
            }
        }
        // and people who paddle sometimes go a day or two up or down their river
        if boat {
            if let Some((c, n)) = self.river_scout(cur, rng).filter(|&(c, _)| m.landmass_of(c) == m.landmass_of(cur)) {
                let pen = 1.0 + terrain::MOVE_EFFORT * (n as f32 * terrain::RIVER_STEP - 1.0).max(0.0);
                cands.push((c, pen));
            }
        }
        // people with boats sometimes go and look across the water: more often when home is
        // hungry or crowded, or when they know of land out there; hard times make a poorer
        // landing acceptable
        let mut by_sea: Option<(usize, f32)> = None;
        let mut by_signs = false;
        let mut by_aim = false;
        let mut by_expedition = false;
        if b.d.kit.boat_q > 0.0 {
            let push = self.push_out(b);
            let lure = if b.far_shore.is_some() {
                0.1 + FAR_SHORE_LURE * (b.curiosity - 1.0).max(0.0) * b.seacraft
                    + PRESTIGE_LURE * b.ambition.max(0.0)
            } else {
                0.0
            };
            let pv = 0.06 + 0.35 * push + lure;
            if rng.chance(pv) {
                let aim = if rng.chance(0.7) { b.far_shore } else { None };
                if let Some((c, sea, signs)) = self.voyage_x(b, rng, aim) {
                    cands.push((c, (1.15 + 0.06 * sea) * (1.0 - 0.3 * push)));
                    by_sea = Some((c, sea));
                    by_signs = signs;
                    by_aim = aim.is_some();
                    by_expedition = sea > self.usual_reach(b);
                }
            }
        }
        let split = self.params.split_size;
        for (c, pen) in cands {
            let sc = self.cell_score(b, c) * self.novelty_pull(b, c, split) / pen;
            if sc > best_s {
                best_s = sc;
                best = c;
            }
        }
        if best != cur {
            b.x = (best % m.w) as u16;
            b.y = (best / m.w) as u16;
            b.settled *= 0.5;
            if let Some((c, sea)) = by_sea {
                if c == best {
                    b.d.sailed_from = Some(cur as u32);
                    fx.counters.voyages += 1;
                    if by_signs {
                        fx.counters.sign_voyages += 1;
                    }
                    if by_aim {
                        fx.counters.aimed_voyages += 1;
                    }
                    if by_expedition {
                        fx.counters.expeditions += 1;
                    }
                    self.sea_toll(b, sea, rng, fx);
                    if by_signs {
                        b.d.sailed_tech |= TECH_PILOT;
                    }
                    if by_expedition {
                        b.d.sailed_tech |= TECH_EXPEDITION;
                    }
                    self.note_crossing(cur, b, fx);
                    self.forget_reached_shore(b);
                }
            }
        }
    }

    /// Count a band that came ashore alive on another landmass than the one it left from.
    /// (v9) The crossing earns the band prestige.
    pub(crate) fn note_crossing(&self, from: usize, b: &mut Band, fx: &mut Fx) {
        if b.pop > 0 && self.map.landmass_of(from) != self.map.landmass_of(b.cell(self.map.w)) {
            fx.counters.crossings += 1;
            Self::ledger_crossing(b.d.sailed_sea, fx);
            b.prestige += PRESTIGE_VOYAGE;
        }
    }

    /// Once a band stands on the land it had heard of, it is no longer a far shore.
    fn forget_reached_shore(&self, b: &mut Band) {
        if let Some(f) = b.far_shore {
            if self.map.landmass_of(f as usize) == self.map.landmass_of(b.cell(self.map.w)) {
                b.far_shore = None;
            }
        }
    }

    /// Record the first arrival of people on each landmass: the tick in `landfall`, and (v9)
    /// how they came in `landfalls` - over water or on foot, from which landmass, and across how
    /// much open sea from the nearest land people had already settled. The Landfall event carries
    /// the boat (NONE on foot or adrift) and, in `recipe`, 1 = came over water, 0 = walked.
    fn track_landfall(&mut self) {
        let nl = self.map.landmass_size.len();
        if self.landfall.len() != nl {
            self.landfall.resize(nl, u32::MAX);
        }
        let w = self.map.w;
        let any = self.landfall.iter().any(|&t| t != u32::MAX);
        let mut evs = Vec::new();
        for b in &self.bands {
            let c = b.cell(w);
            let l = self.map.landmass_of(c) as usize;
            if l == 0 || l >= nl || self.landfall[l] != u32::MAX {
                continue;
            }
            if any {
                // the boat they came in, when they did not walk (a drifted party has none)
                let boat = if b.d.sailed_from.is_some() && b.d.sailed_tech & TECH_ADRIFT == 0 { b.d.kit.boat } else { NONE };
                evs.push((b.id, l, c as u32, b.d.sailed_from, boat, b.d.sailed_sea, b.d.sailed_tech));
            }
            self.landfall[l] = self.tick;
        }
        if evs.is_empty() {
            return;
        }
        // sea gaps from land already settled before this tick
        let tick = self.tick;
        let settled: Vec<bool> = self.landfall.iter().map(|&t| t < tick).collect();
        let gaps = self.map.sea_gaps(|l| settled.get(l).copied().unwrap_or(false));
        for (id, l, c, from, boat, leg, tech) in evs {
            self.landfalls.push(LandfallRec {
                landmass: l as u16,
                tick,
                band: id,
                by_sea: from.is_some(),
                from: from.map(|f| self.map.landmass_of(f as usize)).unwrap_or(0),
                gap: gaps.get(l).copied().flatten().unwrap_or(u32::MAX),
                leg,
                tech,
            });
            self.push_event(EventKind::Landfall, id, l as u32, c, boat, from.is_some() as u32);
        }
    }

    // ------------------------------------------------------------------ contacts
    fn contacts(&mut self, bands: &mut [Band], rng: &mut Rng) {
        let p = self.params;
        let w = self.map.w as i32;
        let h = self.map.h as i32;
        let r = p.teach_radius as i32;
        let mut tk = if self.prof.is_some() { Some(Tick::now()) } else { None };
        // rebuild occupancy after movement
        for v in &mut self.occ {
            v.clear();
        }
        self.occ_pop.clear();
        self.occ_pop.resize(self.occ.len(), 0);
        for (k, b) in bands.iter().enumerate() {
            self.occ[b.cell(w as usize)].push(k as u32);
            self.occ_pop[b.cell(w as usize)] += b.pop;
        }
        // who would meet this season (each band's neighbours drawn on its own random stream,
        // so the neighbourhoods can be searched in parallel), then in random order: a band only
        // has time for a few meetings a season, so in crowded country most neighbours go unvisited
        let key0 = rng.next_u64();
        let tick = self.tick;
        let per: Vec<(u64, Vec<(usize, usize, i32)>)> = {
            let world = &*self;
            let occ = &self.occ;
            let bs = &*bands;
            fx::par_collect(bands.len(), |a| {
                let b = &bs[a];
                let mut r2 = Rng::new(key0 ^ (b.id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                let (x, y) = (b.x as i32, b.y as i32);
                let mut seen = 0u64;
                let mut out = Vec::new();
                let ca = b.cell(w as usize);
                let boat_a = b.d.kit.boat_q > 0.0;
                // with a boat on a river, neighbours further along the same river are within reach
                let river_a = boat_a && world.river_sys.get(ca).copied().unwrap_or(u32::MAX) != u32::MAX;
                let rr = if river_a { r.max(terrain::RIVER_SCOUT as i32) } else { r };
                // (v8) riders on open land meet people twice as far off, across open land
                let ride_a = world.mounted(b) > 0.5;
                let rr = if ride_a { rr.max(2 * r) } else { rr };
                for dy in -rr..=rr {
                    for dx in -rr..=rr {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let d = dx.abs().max(dy.abs());
                        let cn = (ny * w + nx) as usize;
                        let ring = d > r;
                        let by_river = boat_a && world.same_river(ca, cn) && d <= r.max(terrain::RIVER_SCOUT as i32);
                        let by_horse = ride_a && d <= 2 * r && world.open_land(cn);
                        if ring && !by_river && !by_horse {
                            continue;
                        }
                        for &k in &occ[cn] {
                            let k = k as usize;
                            let boat_k = bs[k].d.kit.boat_q > 0.0;
                            // each pair is looked at once: from the lower index, or beyond the
                            // land radius from the side that can reach when only one can
                            let reach_k = (boat_k && world.same_river(cn, ca) && d <= r.max(terrain::RIVER_SCOUT as i32))
                                || (world.mounted(&bs[k]) > 0.5 && d <= 2 * r && world.open_land(ca));
                            let mine = if ring && !reach_k { k != a } else { k > a };
                            if mine {
                                seen += 1;
                                let kin = 1.0 + 0.3 * (2.0 * b.kinship(&bs[k], tick)).min(1.0);
                                let cost = if d == 0 { 0.0 } else { world.travel_cost(ca, cn, boat_a || boat_k) };
                                let pc = p.teach_rate * kin / (1.0 + cost);
                                if r2.chance(pc) {
                                    out.push((a, k, d));
                                }
                            }
                        }
                    }
                }
                (seen, out)
            })
        };
        self.counters.pairs_seen += per.iter().map(|x| x.0).sum::<u64>();
        let mut met: Vec<(usize, usize, i32)> = per.into_iter().flat_map(|x| x.1).collect();
        self.prof_lap(&mut tk, 10);
        for i in (1..met.len()).rev() {
            met.swap(i, rng.below(i + 1));
        }
        let mut busy = vec![0u8; bands.len()];
        let mut todo: Vec<(usize, usize, i32)> = Vec::with_capacity(met.len());
        for (a, c, d) in met {
            if bands[a].pop == 0 || bands[c].pop == 0 || busy[a] >= MAX_MEETINGS || busy[c] >= MAX_MEETINGS {
                continue;
            }
            busy[a] += 1;
            busy[c] += 1;
            self.counters.meetings += 1;
            todo.push((a, c, d));
        }
        let key = rng.next_u64();
        self.run_meetings(bands, &todo, key, true);
        self.prof_lap(&mut tk, 11);
        // partners keep visiting each other after they drift apart: a standing relationship
        // reaches further than chance meetings (much further by boat)
        let idx: crate::knowledge::FastMap<u32, usize> = bands.iter().enumerate().map(|(k, b)| (b.id, k)).collect();
        let mut far: Vec<(usize, usize, i32)> = Vec::new();
        for (a, b) in bands.iter().enumerate() {
            for pt in &b.partners {
                let Some(&c) = idx.get(&pt.id) else { continue };
                if b.id > pt.id && bands[c].partner(b.id).is_some() {
                    continue; // the pair is visited from the other side
                }
                let o = &bands[c];
                let d = (b.x as i32 - o.x as i32).abs().max((b.y as i32 - o.y as i32).abs());
                if d <= r {
                    continue;
                }
                let boat = b.d.kit.boat_q.max(o.d.kit.boat_q);
                let frame = b.d.kit.frame != NONE || o.d.kit.frame != NONE;
                let reach = 2.0 + 4.0 * pt.strength + 8.0 * boat + if frame { 1.0 } else { 0.0 };
                let (ca, cc) = (b.cell(w as usize), o.cell(w as usize));
                let lm_a = self.map.landmass_of(ca);
                let lm_c = self.map.landmass_of(cc);
                // the effort of the trip: overland over the hills, along a shared river by
                // boat (twice as far for the effort), or by sea between coasts
                let mut eff = self.land_cost(ca, cc);
                if boat > 0.0 {
                    if self.same_river(ca, cc) {
                        eff = eff.min(terrain::RIVER_VISIT * d as f32);
                    }
                    if lm_a != lm_c || (self.coastal(ca) && self.coastal(cc)) {
                        eff = eff.min(d as f32);
                    }
                }
                if eff > reach || (lm_a != lm_c && boat <= 0.0) {
                    continue;
                }
                if rng.chance(0.5 * pt.strength / (1.0 + 0.15 * eff)) {
                    far.push((a, c, d));
                }
            }
        }
        let key = rng.next_u64();
        self.run_meetings(bands, &far, key, false);
        self.prof_lap(&mut tk, 12);
    }

    /// Run a season's meetings. They are split into rounds in which no band meets twice; the
    /// meetings of a round run in parallel (each on its own pair of bands, with its own random
    /// stream) and their effects on the world are applied in meeting order after the round.
    fn run_meetings(&mut self, bands: &mut [Band], list: &[(usize, usize, i32)], key: u64, full: bool) {
        if list.is_empty() {
            return;
        }
        // each meeting goes in the first round where neither band is busy yet (a band in m
        // meetings needs at most 2m-1 rounds; past 64 rounds, meetings just go one after another)
        let mut used = vec![0u64; bands.len()];
        let mut rounds: Vec<Vec<(usize, usize, i32, u32)>> = Vec::new();
        let mut tail: Vec<(usize, usize, i32, u32)> = Vec::new();
        for (k, &(a, c, d)) in list.iter().enumerate() {
            let free = !(used[a] | used[c]);
            if free == 0 {
                tail.push((a, c, d, k as u32));
                continue;
            }
            let r = free.trailing_zeros() as usize;
            used[a] |= 1 << r;
            used[c] |= 1 << r;
            if rounds.len() <= r {
                rounds.resize_with(r + 1, Vec::new);
            }
            rounds[r].push((a, c, d, k as u32));
        }
        rounds.extend(tail.into_iter().map(|m| vec![m]));
        for round in rounds {
            // every band meets at most once in a round, so each meeting can hold both of its
            // bands mutably at once (no copying them out of the list)
            let mut slots: Vec<Option<&mut Band>> = bands.iter_mut().map(Some).collect();
            let mut pairs: Vec<(Pair, i32, u32)> = round
                .iter()
                .map(|&(a, c, d, k)| (Pair(slots[a].take().unwrap(), slots[c].take().unwrap()), d, k))
                .collect();
            drop(slots);
            let fxs: Vec<Fx> = {
                let me = &*self;
                fx::par_map(&mut pairs, |(two, d, k)| {
                    let mut fx = Fx::default();
                    let mut r = Rng::new(key ^ (*k as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93));
                    me.meeting(two, *d, &mut r, &mut fx, full);
                    fx
                })
            };
            drop(pairs);
            for f in fxs {
                self.apply_fx(f, None);
            }
        }
    }

    /// One meeting between two bands (index 0 and 1 of `two`), `d` cells apart. `full`: a
    /// chance meeting of neighbours (disease, conflict, teaching, farming, trade); otherwise a
    /// visit between distant partners (teaching and trade).
    fn meeting(&self, bands: &mut Pair, d: i32, rng: &mut Rng, fx: &mut Fx, full: bool) {
        let (a, c) = (0, 1);
        if bands[a].pop == 0 || bands[c].pop == 0 {
            return;
        }
        if full {
            self.spread_disease(bands, a, c, rng, fx);
            if self.confront(bands, a, c, d, rng, fx) {
                return;
            }
            self.exchange(bands, a, c, rng, fx);
            self.exchange(bands, c, a, rng, fx);
            self.share_farming(bands, a, c, rng, fx);
            self.share_farming(bands, c, a, rng, fx);
            self.marry(bands, a, c, rng, fx);
            // (inventions step 10) the one with food to spare may feast the other
            self.feast(bands, a, c, fx);
        } else {
            // (v9) a visit across open water teaches whoever paddled it
            if self.map.landmass_of(bands[a].cell(self.map.w)) != self.map.landmass_of(bands[c].cell(self.map.w)) {
                for x in [a, c] {
                    if bands[x].d.kit.boat_q > 0.0 {
                        Self::learn_sea(&mut bands[x], SEA_VISIT_LESSON);
                    }
                }
            }
            if rng.chance(0.5) {
                self.exchange(bands, a, c, rng, fx);
                self.exchange(bands, c, a, rng, fx);
            }
        }
        self.trade(bands, a, c, d, rng, fx);
    }

    /// (v9) Play and refinement: `lam` expected attempts this season, each improving one recipe
    /// the band used.
    fn refine(b: &mut Band, lam: f32, rng: &mut Rng, fx: &mut Fx) {
        let n = rng.poisson(lam).min(4);
        if n == 0 {
            return;
        }
        let used: Vec<usize> = (0..b.known.len()).filter(|&i| b.known[i].used).collect();
        if used.is_empty() {
            return;
        }
        for _ in 0..n {
            let k = &mut b.known[used[rng.below(used.len())]];
            let room = k.skill - k.quality;
            if room > 0.01 {
                k.quality += REFINE_STEP * room;
                b.d.fin_dirty = true;
                fx.counters.refinements += 1;
            }
        }
    }

    /// (v9) At a meeting, `dst` may see `src` make something better than it does.
    fn show_quality(bands: &mut Pair, src: usize, dst: usize, rng: &mut Rng) {
        let s = &bands[src];
        if s.known.is_empty() || !rng.chance((QUALITY_SHOW_P * prestige_bias(s, &bands[dst])).min(0.9)) {
            return;
        }
        let kn = s.known[rng.below(s.known.len())];
        if kn.quality < 0.05 {
            return;
        }
        let d = &mut bands[dst];
        if let Some(k) = d.known.iter_mut().find(|k| k.recipe == kn.recipe) {
            if kn.quality > k.quality {
                k.quality = (k.quality + QUALITY_SHOW * (kn.quality - k.quality)).min(k.skill);
                d.d.fin_dirty = true;
            }
        }
    }

    /// (inventions step 8) Whether a band has something worth keeping count of: a big store, a
    /// few real trade partners, a herd, or tribute owed or owing.
    pub fn keeps_count(b: &Band) -> bool {
        b.store >= RECORDS_STORE * b.d.need.max(1.0)
            || b.herd > 0.1
            || b.owed > 0.2
            || b.partners.iter().filter(|p| p.strength >= RECORDS_PARTNER_TIE).count() >= RECORDS_PARTNERS
    }

    /// (v9) Two bands at peace may exchange spouses: a few people move each way (numbers are
    /// unchanged), mixing ancestry and tying the bands together.
    fn marry(&self, bands: &mut Pair, a: usize, c: usize, rng: &mut Rng, fx: &mut Fx) {
        let small = bands[a].pop.min(bands[c].pop);
        let pull = 1.0 + PRESTIGE_MARRY * 0.5 * (bands[a].prestige + bands[c].prestige);
        if small < 10 || !rng.chance((MARRIAGE_P * pull).min(0.3)) {
            return;
        }
        let k = ((MARRIAGE_SHARE * small as f32).round() as u32).max(1);
        let (aa, ca) = (bands[a].ancestry(), bands[c].ancestry());
        let (ia, ic) = (bands[a].id, bands[c].id);
        let tick = self.tick;
        for (x, other, oid) in [(a, &ca, ic), (c, &aa, ia)] {
            let b = &mut bands[x];
            let f = k as f32 / b.pop as f32;
            b.mix_ancestry(other, f);
            b.bond(oid, 0.2, 0.0, tick);
            if let Some(p) = b.partners.iter_mut().find(|p| p.id == oid) {
                p.married = (p.married + MARRIAGE_TIE).min(1.0);
            }
            for g in b.grudges.iter_mut().filter(|g| g.id == oid) {
                g.w *= 1.0 - MARRIAGE_FORGIVE;
            }
        }
        fx.counters.marriages += 1;
        fx.ops.push(Op::Married { a: ia, b: ic, la: bands[a].lineage, lb: bands[c].lineage, cell: bands[a].cell(self.map.w) as u32 });
    }

    /// Two bands that meet swap what each has spare for what the other values more, pass food to
    /// the hungry, and hear about what each other wants. Nothing has a set price: worth is each
    /// band's own need and shortage (see `want`).
    fn trade(&self, bands: &mut Pair, a: usize, c: usize, d: i32, rng: &mut Rng, fx: &mut Fx) {
        let tick = self.tick;
        // news of wants travels with every meeting, so goods can pass down a chain of bands
        for (x, y) in [(a, c), (c, a)] {
            for &m in TRADE_MATS.iter() {
                let mi = m as usize;
                let (base, _) = want(&bands[y], mi);
                if bands[y].stock[mi] < 0.3 && base >= 0.3 {
                    let dm = &mut bands[x].d.demand[mi];
                    *dm = dm.max(0.6 * base);
                }
            }
        }
        let kin = bands[a].kinship(&bands[c], tick) >= 0.2;
        let pstr = bands[a].partner(bands[c].id).map(|p| p.strength).unwrap_or(0.0);
        let close = kin || pstr >= CLOSE_TIE;
        // best good each side could pass the other, and what it is worth to both
        let best = |from: &Band, to: &Band| -> Option<(usize, f32, f32)> {
            let mut bst: Option<(usize, f32, f32)> = None;
            for &m in TRADE_MATS.iter() {
                let mi = m as usize;
                let sf = from.stock[mi];
                if sf < 0.3 {
                    continue;
                }
                // (v8) a carrying frame takes a bigger load, more for a better basket
                let fr = if from.d.kit.frame != NONE { 1.0 + vessel_q(mat(from.d.kit.frame)) } else { 1.0 };
                let q = (sf - 0.2).min(0.3 * fr);
                let g = want(to, mi).1 - want(from, mi).1;
                if bst.map_or(true, |b| g > b.1) {
                    bst = Some((mi, g, q));
                }
            }
            bst
        };
        let ac = best(&bands[a], &bands[c]);
        let ca = best(&bands[c], &bands[a]);
        let ok = |o: Option<(usize, f32, f32)>, th: f32| o.filter(|x| x.1 > th);
        let mut moved = 0.0;
        match (ok(ac, 0.05), ok(ca, 0.05)) {
            (Some(x), Some(y)) => {
                // barter: both come out ahead by their own reckoning
                moved += self.pass_good(bands, a, c, x.0, x.2, d, fx);
                moved += self.pass_good(bands, c, a, y.0, y.2, d, fx);
            }
            (Some(x), None) | (None, Some(x)) => {
                let (from, to) = if ok(ac, 0.05).is_some() { (a, c) } else { (c, a) };
                // pay in food if the receiver has plenty, otherwise a gift between close bands
                let (fr, tr) = (&bands[from], &bands[to]);
                let can_pay = tr.d.food_ratio >= 1.0 && tr.store > 0.4 * tr.d.need && fr.store < 0.6 * fr.d.need.max(1.0);
                if x.1 > 0.15 && (can_pay || (close && rng.chance(0.5))) {
                    moved += self.pass_good(bands, from, to, x.0, x.2, d, fx);
                    if can_pay {
                        let f = 0.08 * bands[to].d.need;
                        bands[to].store -= f;
                        bands[from].store += f;
                    }
                }
            }
            _ => {}
        }
        // food for the hungry among kin and partners
        if close {
            for (x, y) in [(a, c), (c, a)] {
                let (gx, gy) = (&bands[x], &bands[y]);
                if gx.d.food_ratio >= 1.0 && gx.store > 0.3 * gx.d.need && gy.d.food_ratio < 0.9 {
                    let give = (0.3 * (gx.store - 0.3 * gx.d.need)).min((0.9 - gy.d.food_ratio) * gy.d.need);
                    if give > 0.0 {
                        bands[x].store -= give;
                        bands[y].store += give;
                        fx.acc.food_shared += give;
                        bands[x].prestige += give_prestige(give, 1.0);
                        moved += 0.01;
                    }
                }
            }
        }
        if moved > 0.0 {
            let (ia, ic) = (bands[a].id, bands[c].id);
            bands[a].bond(ic, 0.2, moved, tick);
            bands[c].bond(ia, 0.2, moved, tick);
            self.refresh_toolkit(&mut bands[a]);
            self.refresh_toolkit(&mut bands[c]);
        }
    }

    /// Move `q` of material `mi` from one band to another; returns the amount moved.
    fn pass_good(&self, bands: &mut Pair, from: usize, to: usize, mi: usize, q: f32, d: i32, fx: &mut Fx) -> f32 {
        let w = self.map.w;
        let o = bands[from].origin[mi];
        let strain = bands[from].strain;
        bands[from].stock[mi] -= q;
        let (fc, fid) = (bands[from].cell(w), bands[from].id);
        let t = &mut bands[to];
        if t.stock[mi] < q || t.origin[mi] == u32::MAX {
            t.origin[mi] = if o == u32::MAX { fc as u32 } else { o };
        }
        t.stock[mi] = (t.stock[mi] + q).min(1.5);
        if ALL_MATS[mi] == Mat::Grain && strain > t.strain {
            t.strain = 0.95 * strain; // seed grain carries its strain
        }
        let (tc, tid) = (t.cell(w), t.id);
        fx.acc.trades += 1;
        fx.acc.trade_volume += q;
        fx.acc.trade_dist += d as f32;
        if self.traded_first & (1 << mi) == 0 {
            fx.ops.push(Op::TradedFirst { mi, from: fid, to: tid, cell: tc as u32 });
        }
        // the overlay: mark the cells along the way
        fx.ops.push(Op::TradeLine { from: fc as u32, to: tc as u32, d, q });
        q
    }

    /// (v8) Open country a rider can cross at a gallop: steppe, grassland, savanna.
    pub fn open_land(&self, c: usize) -> bool {
        self.graze_pot.get(c).copied().unwrap_or(0.0) >= 0.8
    }

    /// (v8) How much riding counts for a band where it stands (0 off open land).
    pub fn mounted(&self, b: &Band) -> f32 {
        if b.ride > 0.0 && b.herd > 0.05 && self.open_land(b.cell(self.map.w)) { b.ride } else { 0.0 }
    }

    /// `src` shows `dst` one thing it knows (if `dst` can use it, it is more likely to stick).
    pub(crate) fn exchange(&self, bands: &mut Pair, src: usize, dst: usize, rng: &mut Rng, fx: &mut Fx) {
        // fire can be shared
        if bands[src].flame && !bands[dst].flame && rng.chance(0.5) {
            bands[dst].flame = true;
        }
        // (v8) and what plants ease the sick
        if bands[src].herbal > bands[dst].herbal + 0.1 && rng.chance(0.3) {
            bands[dst].herbal += 0.5 * (bands[src].herbal - bands[dst].herbal);
        }
        // (v8) herders on open land can learn to ride from riders, if they keep animals to ride
        if bands[src].ride > bands[dst].ride + 0.1 && bands[dst].herd > 0.1 && bands[dst].tame > 0.6 && self.open_land(bands[dst].cell(self.map.w)) && rng.chance(0.2) {
            bands[dst].ride += 0.5 * (bands[src].ride - bands[dst].ride);
        }
        // (v9) and seamanship, to people who have boats and live by the sea; kin teach more
        // readily. Nobody learns past their teacher.
        let gap = bands[src].seacraft - bands[dst].seacraft;
        if gap > 0.1 && bands[dst].d.kit.boat_q > 0.0 && self.coastal(bands[dst].cell(self.map.w)) {
            let kin = bands[src].kinship(&bands[dst], self.tick) >= 0.2;
            if rng.chance(if kin { 2.0 * SEA_TEACH_P } else { SEA_TEACH_P }) {
                bands[dst].seacraft += SEA_TEACH_SHARE * gap;
            }
        }
        // (inventions step 3) and the signs of land, the same way
        let gap = bands[src].pilot - bands[dst].pilot;
        if gap > 0.1 && bands[dst].d.kit.boat_q > 0.0 && self.coastal(bands[dst].cell(self.map.w)) {
            let kin = bands[src].kinship(&bands[dst], self.tick) >= 0.2;
            if rng.chance(if kin { 2.0 * SEA_TEACH_P } else { SEA_TEACH_P }) {
                bands[dst].pilot += SEA_TEACH_SHARE * gap;
            }
        }
        // (inventions step 4) and sky lore, to people with a use for it: farmers to time their
        // sowing, pilots to steer by at night (so farmers' calendars reach coastal seafarers)
        let gap = bands[src].sky - bands[dst].sky;
        if gap > 0.1 && (bands[dst].sow > 0.1 || bands[dst].pilot > 0.02) {
            let kin = bands[src].kinship(&bands[dst], self.tick) >= 0.2;
            if rng.chance(if kin { 2.0 * SEA_TEACH_P } else { SEA_TEACH_P }) {
                bands[dst].sky += SEA_TEACH_SHARE * gap;
            }
        }
        // (inventions step 8) and the keeping of records, to people with something to count
        let gap = bands[src].records - bands[dst].records;
        if gap > 0.1 && Self::keeps_count(&bands[dst]) {
            let kin = bands[src].kinship(&bands[dst], self.tick) >= 0.2;
            if rng.chance(if kin { 2.0 * SEA_TEACH_P } else { SEA_TEACH_P }) {
                bands[dst].records += SEA_TEACH_SHARE * gap;
            }
        }
        // so can word of a land across the water - (v9) told with its age, and taken to heart
        // only by people who could hope to get there (within twice their boats' range);
        // (inventions step 3) a pilot tells the stepping stones, so word carries a little further
        // (inventions step 9) a chart is taught: further, more readily, and as fresh directions
        if let (None, Some(f)) = (bands[dst].far_shore, bands[src].far_shore) {
            let chart = Self::charts(&bands[src]);
            let reach = 2.0 * Self::boat_range(&bands[dst].d.kit, bands[dst].seacraft) * (1.0 + PILOT_TELL * bands[src].pilot) * if chart { CHART_TELL } else { 1.0 };
            if dist_cells(bands[dst].cell(self.map.w), f as usize, self.map.w) <= reach && rng.chance(if chart { CHART_TEACH_P } else { 0.3 }) {
                bands[dst].far_shore = Some(f);
                bands[dst].far_shore_year = if chart { self.year_u16() } else { bands[src].far_shore_year };
                if chart {
                    fx.counters.routes_taught += 1;
                }
            }
        }
        // (v9) watching a better craftsman: some of a finer way to make what dst already makes
        Self::show_quality(bands, src, dst, rng);
        // a 1024-bit filter of the recipes dst knows: a clear bit means "not known" without the
        // linear `knows` scan (which was ~5% of a run); a set bit falls back to the exact scan
        let mut dk = [0u64; 16];
        for k in &bands[dst].known {
            let h = (k.recipe & 1023) as usize;
            dk[h >> 6] |= 1 << (h & 63);
        }
        let maybe_known = |r: u32| {
            let h = (r & 1023) as usize;
            dk[h >> 6] & (1 << (h & 63)) != 0
        };
        let cands: Vec<(u32, f32, f32)> = bands[src]
            .known
            .iter()
            .filter(|k| k.skill >= 0.25 && !self.reg.retired(k.recipe))
            .filter(|k| {
                let out = self.reg.recipes[k.recipe as usize].output;
                !bands[dst].d.tk.producible.has(out) && !(maybe_known(k.recipe) && bands[dst].knows(k.recipe))
            })
            .map(|k| (k.recipe, k.skill, k.quality))
            .collect();
        if cands.is_empty() {
            return;
        }
        let (rid, skill, tq) = cands[rng.below(cands.len())];
        let r = &self.reg.recipes[rid as usize];
        let relevant = r.inputs.iter().all(|&i| bands[dst].d.tk.avail.has(i));
        let bias = prestige_bias(&bands[src], &bands[dst]);
        let pl = (skill * if relevant { 0.8 } else { 0.25 } * bias).min(0.95);
        if !rng.chance(pl) {
            return;
        }
        let out = r.output;
        let srcid = bands[src].id;
        let d = &mut bands[dst];
        Self::reinforce_craft(d, &self.reg.recipes[rid as usize], 1.06);
        d.known.push(Known { recipe: rid, skill: 0.3, quality: (QUALITY_TEACH * tq).min(0.3), since: self.tick, from: srcid, kin: false, used: true });
        d.d.known_ver += 1;
        d.learned += 1;
        let (did, dc) = (d.id, d.cell(self.map.w) as u32);
        fx.ops.push(Op::RecipeTransfer(rid));
        fx.counters.transfers += 1;
        fx.acc.transfers += 1;
        if bias > 1.0 {
            fx.counters.prestige_taught += 1;
        }
        fx.push_event(EventKind::Learned, did, srcid, dc, out, rid);
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
            // (v9) friction builds with hunger, crowding and size, the more for the ambitious
            let friction = {
                let b = &bands[k];
                self.push_out(b) * (b.pop as f32 / p.split_size).min(1.5) * b.ambition.exp()
            };
            let b = &mut bands[k];
            b.dissent += DISSENT_RATE * (friction - b.dissent);
            let pop = b.pop as f32;
            let quarrel = pop <= thresh
                && b.dissent > DISSENT_SPLIT
                && pop >= DISSENT_MIN_SIZE * p.split_size
                && rng.chance(DISSENT_SPLIT_P * (b.dissent - DISSENT_SPLIT));
            if pop <= thresh && !quarrel {
                continue;
            }
            let b = &bands[k];
            // daughter settles in the best nearby cell; (v9) those who leave in anger go farther
            let (x, y) = (b.x as i32, b.y as i32);
            let rad = if quarrel { DISSENT_RADIUS } else { 2 };
            let mut best = (b.cell(w), f32::MIN);
            for _ in 0..8 {
                let nx = x + rng.below(2 * rad as usize + 1) as i32 - rad;
                let ny = y + rng.below(2 * rad as usize + 1) as i32 - rad;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let c = ny as usize * w + nx as usize;
                // (v9) a daughter that walks stays on this side of the water
                if !self.map.walkable(b.cell(w), c) {
                    continue;
                }
                let s = self.cell_score(b, c) * self.novelty_pull(b, c, p.split_size);
                if s > best.1 {
                    best = (c, s);
                }
            }
            // a daughter band with boats may strike out across the water instead
            let mut sailed = None;
            if b.d.kit.boat_q > 0.0 && rng.chance(if quarrel { 0.8 } else { 0.5 }) {
                let push = self.push_out(b);
                let aim = if rng.chance(if quarrel { 0.9 } else { 0.7 }) { b.far_shore } else { None };
                if let Some((c, sea, signs)) = self.voyage_x(b, rng, aim) {
                    let s = self.cell_score(b, c) * self.novelty_pull(b, c, p.split_size) / ((1.1 + 0.05 * sea) * (1.0 - 0.3 * push));
                    if s > best.1 {
                        best = (c, s);
                        sailed = Some((sea, signs, aim.is_some(), sea > self.usual_reach(b)));
                    }
                }
            }
            // (v8) a plank boat carries a bigger party
            let dpop = if sailed.is_some() && b.d.kit.plank != NONE {
                (b.pop as f32 * rng.range(0.45, 0.6)) as u32
            } else {
                (b.pop as f32 * rng.range(0.35, 0.5)) as u32
            };
            // (v9) after a quarrel the bolder, more curious people leave, and the rest stay:
            // the daughter's traits shift up by `tail`, the parent's down in proportion
            let tail = if quarrel { EMIGRANT_TAIL * rng.gauss().abs() } else { 0.0 };
            let stay_shift = tail * dpop as f32 / (b.pop - dpop).max(1) as f32;
            let id = self.next_id;
            self.next_id += 1;
            let known: Vec<Known> = b
                .known
                .iter()
                .filter_map(|kn| {
                    if rng.chance(0.55 + 0.45 * kn.skill) {
                        Some(Known { skill: kn.skill * 0.9, quality: kn.quality * QUALITY_SPLIT, from: FROM_PARENT, since: self.tick, ..*kn })
                    } else {
                        None
                    }
                })
                .collect();
            let mut d = Band {
                id,
                lineage: b.lineage,
                parent: b.id,
                gen: b.gen + 1,
                anc: [b.parent, b.anc[0], b.anc[1]],
                ancestry: b.ancestry.clone(),
                x: (best.0 % w) as u16,
                y: (best.0 / w) as u16,
                pop: dpop,
                store: b.store * 0.4,
                curiosity: (b.curiosity * (0.12 * rng.gauss() + tail).exp()).clamp(0.3, 2.5),
                ambition: (b.ambition + AMBITION_DRIFT * rng.gauss() + tail).clamp(-AMBITION_MAX, AMBITION_MAX),
                flame: b.flame,
                known,
                stock: b.stock.clone(),
                origin: b.origin.clone(),
                partners: vec![Partner { id: b.id, strength: 0.5, since: self.tick, ..Default::default() }],
                // habits of hand carry over, each daughter a little different
                craft: b.craft.iter().map(|c| (c * (0.15 * rng.gauss()).exp()).clamp(0.2, 4.0)).collect(),
                far_shore: b.far_shore,
                far_shore_year: b.far_shore_year,
                // old wrongs are remembered, less sharply, by the daughters
                grudges: b.grudges.iter().map(|g| Grudge { w: 0.6 * g.w, ..*g }).filter(|g| g.w >= 0.05).collect(),
                born: self.tick,
                sow: b.sow * 0.9,
                strain: b.strain,
                herd: b.herd,
                tame: b.tame,
                immune: b.immune,
                sick: b.sick,
                seacraft: b.seacraft,
                pilot: b.pilot,
                sky: b.sky,
                records: b.records,
                herbal: 0.9 * b.herbal,
                ride: b.ride,
                ..Default::default()
            };
            if let Some((sea, signs, aimed, expedition)) = sailed {
                d.d.kit.boat = b.d.kit.boat; // remembered for the landfall record
                d.d.kit.boat_q = b.d.kit.boat_q;
                d.d.kit.outrigger = b.d.kit.outrigger;
                d.d.kit.sail = b.d.kit.sail;
                d.d.kit.plank = b.d.kit.plank;
                d.d.kit.ship = b.d.kit.ship;
                d.d.kit.ship_q = b.d.kit.ship_q;
                // (inventions step 2) and what keeps their food and water on the way
                d.d.kit.jar = b.d.kit.jar;
                d.d.kit.cut = b.d.kit.cut;
                d.d.kit.cord = b.d.kit.cord;
                d.d.kit.vessel = b.d.kit.vessel;
                d.d.kit.smoke = b.d.kit.smoke;
                d.d.farm_share = b.d.farm_share;
                d.d.sailed_from = Some(b.cell(w) as u32);
                self.counters.voyages += 1;
                let mut fx = Fx::default();
                if signs {
                    fx.counters.sign_voyages += 1;
                }
                if aimed {
                    fx.counters.aimed_voyages += 1;
                }
                if expedition {
                    fx.counters.expeditions += 1;
                }
                self.sea_toll(&mut d, sea, rng, &mut fx);
                if signs {
                    d.d.sailed_tech |= TECH_PILOT;
                }
                if expedition {
                    d.d.sailed_tech |= TECH_EXPEDITION;
                }
                if d.pop >= 4 {
                    self.note_crossing(bands[k].cell(self.map.w), &mut d, &mut fx);
                }
                self.apply_fx(fx, None);
                self.forget_reached_shore(&mut d);
            }
            let bm = &mut bands[k];
            bm.pop -= dpop;
            if quarrel {
                bm.curiosity = (bm.curiosity * (-stay_shift).exp()).clamp(0.3, 2.5);
                bm.ambition = (bm.ambition - stay_shift).clamp(-AMBITION_MAX, AMBITION_MAX);
                bm.dissent *= 0.3; // the quarrel leaves with them
            }
            if d.pop < 4 {
                continue; // the daughter band was lost at sea (or too few made it to stand alone)
            }
            bm.store *= 0.6;
            bm.bond(id, 0.5, 0.0, self.tick);
            self.counters.splits += 1;
            if quarrel {
                self.counters.dissent_splits += 1;
            }
            self.push_event(EventKind::Split, bm.id, id, best.0 as u32, NONE, u32::MAX);
            newb.push(d);
        }
        // tiny bands merge with a neighbour or vanish (the neighbour: the first band in the list
        // of 4+ people within one cell; found through the occupancy grid rather than a scan of
        // every band)
        let mut grid: crate::knowledge::FastMap<usize, Vec<usize>> = Default::default();
        let has_tiny = bands.iter().any(|b| b.pop > 0 && b.pop < 4);
        if has_tiny {
            for (j, b) in bands.iter().enumerate().take(n0) {
                if b.pop >= 4 {
                    grid.entry(b.cell(w)).or_default().push(j);
                }
            }
        }
        for k in 0..n0 {
            if bands[k].pop >= 4 || bands[k].pop == 0 {
                continue; // (0: already absorbed by a stronger band this season)
            }
            let (x, y) = (bands[k].x as i32, bands[k].y as i32);
            let mut target: Option<usize> = None;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    if let Some(v) = grid.get(&(ny as usize * w + nx as usize)) {
                        for &j in v {
                            if j != k && bands[j].pop >= 4 && target.map_or(true, |t| j < t) {
                                target = Some(j);
                            }
                        }
                    }
                }
            }
            let (id, cell) = (bands[k].id, bands[k].cell(w) as u32);
            if let Some(j) = target {
                let moved: Vec<Known> = bands[k].known.clone();
                let pop = bands[k].pop;
                let tid = bands[j].id;
                let tb = &mut bands[j];
                tb.pop += pop;
                for kn in moved {
                    // (inventions step 11b) a retired recipe (cold-hammered gold) is not passed on
                    if !tb.knows(kn.recipe) && !self.reg.retired(kn.recipe) && rng.chance(kn.skill * 0.6) {
                        tb.known.push(Known { skill: kn.skill * 0.7, quality: kn.quality * QUALITY_TEACH, from: id, since: self.tick, kin: false, ..kn });
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
        newb.extend(std::mem::take(&mut self.pending));
        for mut d in newb {
            self.refresh_toolkit(&mut d);
            bands.push(d);
        }
    }

    // ------------------------------------------------------------------ statistics
    fn track_knowledge(&mut self) {
        let w = self.map.w;
        let (mw, mh) = (self.map.w, self.map.h);
        let tick = self.tick;
        let fire = item(Form::Fire, Mat::Wood);
        for b in &mut self.bands {
            let c = b.cell(w);
            let mut av = b.d.tk.producible;
            if b.flame {
                av.add(fire);
            }
            let key = (c as u32, av);
            if b.d.kn_key == Some(key) {
                continue;
            }
            b.d.kn_key = Some(key);
            for it in av.iter() {
                self.cell_first.entry(it).or_insert_with(|| CellFirst::new(mw, mh)).set_first(c, tick);
            }
        }
    }

    pub(crate) fn year_stats(&mut self) {
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
        // anything that was known last year and now by nobody is lost to the world. Only items
        // that had spread (5+ bands at once) go in the chronicle; a local trick that one or two
        // bands had and dropped is logged quietly
        let lost: Vec<Item> = self
            .item_bands
            .iter()
            .filter(|(it, &c)| c > 0 && !counts.contains_key(it))
            .map(|(&it, _)| it)
            .collect();
        for it in lost {
            let peak = self.item_peak.remove(&it).unwrap_or(5);
            let times = self.item_lost.entry(it).or_insert(0);
            *times += 1;
            // the chronicle gets the first loss of a widespread item, and any later loss only if
            // it had become common again (25+ bands); the rest is logged quietly
            let big = peak >= 5 && (*times == 1 || peak >= 25);
            let kind = if big { EventKind::LostWorldwide } else { EventKind::LostLocal };
            self.push_event(kind, 0, 0, 0, it, u32::MAX);
        }
        for (&it, &c) in &counts {
            let pk = self.item_peak.entry(it).or_insert(0);
            *pk = (*pk).max(c);
        }
        self.item_bands = counts;
        // partnerships fade unless used; partners that no longer exist are dropped
        let ids: std::collections::HashSet<u32> = self.bands.iter().map(|b| b.id).collect();
        let mut links = 0u32;
        let (mut gd, mut gn, mut gmax) = (0.0f32, 0u32, 0.0f32);
        let w = self.map.w;
        // (inventions step 8b) kinship of every band with each of its partners, for the care
        // each tie gets (flat, in partner order)
        let kin: Vec<f32> = {
            let idx: crate::knowledge::FastMap<u32, usize> = self.bands.iter().enumerate().map(|(k, b)| (b.id, k)).collect();
            let (bands, tick, idx) = (&self.bands, self.tick, &idx);
            let of = move |b: &'_ Band| b.partners.iter().map(move |p| idx.get(&p.id).map_or(0.0, |&j| b.kinship(&bands[j], tick))).collect::<Vec<f32>>();
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                bands.par_iter().flat_map_iter(of).collect()
            }
            #[cfg(not(feature = "parallel"))]
            {
                bands.iter().flat_map(of).collect()
            }
        };
        let mut ki = 0usize;
        let (mut close_sum, mut close_max) = (0u64, 0u32);
        for b in &mut self.bands {
            // (inventions step 8b) care goes mostly to the ties that already mean most (see
            // `TIE_CARE`); records widen the budget and slow the fade of kept-up ties
            let n = b.partners.len();
            b.fade_ties(&kin[ki..ki + n]);
            ki += n;
            let nc = b.partners.iter().filter(|p| p.strength >= CLOSE_TIE).count() as u32;
            close_sum += nc as u64;
            close_max = close_max.max(nc);
            // (v9) marriage ties outlast trade
            b.partners.retain(|p| (p.strength >= 0.05 || p.married >= 0.05) && ids.contains(&p.id));
            links += b.partners.len() as u32;
            let c = b.cell(w);
            for m in [Mat::Obsidian, Mat::Copper, Mat::Tin, Mat::Bronze, Mat::CopperOre, Mat::TinOre, Mat::Iron, Mat::Steel, Mat::Grain] {
                let mi = m as usize;
                let o = b.origin.get(mi).copied().unwrap_or(u32::MAX);
                if b.stock.get(mi).copied().unwrap_or(0.0) > 0.1 && o != u32::MAX {
                    let (ox, oy) = ((o as usize % w) as i32, (o as usize / w) as i32);
                    let dd = (ox - b.x as i32).abs().max((oy - b.y as i32).abs()) as f32;
                    let _ = c;
                    gd += dd;
                    gn += 1;
                    gmax = gmax.max(dd);
                }
            }
        }
        // (inventions step 8) the world's first band to keep records
        if self.farm_flags & FLAG_RECORDS == 0 {
            if let Some(b) = self.bands.iter().find(|b| b.records >= RECORDS_FIRST) {
                let (id, c, mk) = (b.id, b.cell(w), b.d.kit.mark);
                self.farm_flags |= FLAG_RECORDS;
                self.push_event(EventKind::FirstRecords, id, 0, c as u32, mk, u32::MAX);
            }
        }
        // (inventions step 9) the world's first chart
        if self.farm_flags & FLAG_CHART == 0 {
            if let Some(b) = self.bands.iter().find(|b| Self::charts(b)) {
                let (id, c) = (b.id, b.cell(w));
                let other = b.far_shore.map_or(u32::MAX, |f| self.map.landmass_of(f as usize) as u32);
                self.farm_flags |= FLAG_CHART;
                self.push_event(EventKind::FirstChart, id, other, c as u32, NONE, u32::MAX);
            }
        }
        self.fade_grudges();
        self.build_walls();
        // farming
        let (mut farmers, mut farmer_pop, mut herders, mut iron_farm_bands) = (0u32, 0u32, 0u32, 0u32);
        let (mut steel_bands, mut wq_sum, mut wq_pop) = (0u32, 0.0f64, 0.0f64);
        let (mut mark_bands, mut records_bands, mut rec_sum, mut np_sum, mut np_max) = (0u32, 0u32, 0.0f64, 0u64, 0u32);
        let (mut ff, mut fall) = (0.0f32, 0.0f32);
        let mut fcells = std::collections::HashSet::new();
        let mut gcells = std::collections::HashSet::new();
        let mut gpop = 0u32;
        for b in &self.bands {
            if b.sow > 0.0 {
                farmers += 1;
                farmer_pop += b.pop;
                fcells.insert(b.cell(w));
            } else {
                gpop += b.pop;
                gcells.insert(b.cell(w));
            }
            if b.herd > 0.0 {
                herders += 1;
            }
            if b.sow > 0.0 && farming::iron_tools(b) {
                iron_farm_bands += 1;
            }
            let k = &b.d.kit;
            if [k.hunt, k.cut, k.axe, k.point].iter().any(|&i| i != NONE && mat(i) == Mat::Steel) {
                steel_bands += 1;
            }
            if k.mark != NONE {
                mark_bands += 1;
            }
            if b.records > 0.1 {
                records_bands += 1;
            }
            rec_sum += b.records as f64;
            np_sum += b.partners.len() as u64;
            np_max = np_max.max(b.partners.len() as u32);
            wq_sum += crate::band::weapon_q(k) as f64 * b.pop as f64;
            wq_pop += b.pop as f64;
            ff += b.d.farm_food;
            fall += b.d.food;
        }
        let (mut fields, mut dom_fields, mut fsum) = (0u32, 0u32, 0.0f32);
        let (mut opened_fields, mut forest_fields, mut wet_fields) = (0u32, 0u32, 0u32);
        for i in 0..self.tend.len() {
            if self.tend[i] > 0.4 {
                fields += 1;
                fsum += self.fert[i];
                if self.opened.get(i).is_some_and(|&o| o > 0.5) {
                    opened_fields += 1;
                }
                if matches!(self.map.biome_at(i), crate::map::Biome::TempForest | crate::map::Biome::TropForest | crate::map::Biome::Taiga) {
                    forest_fields += 1;
                }
                if self.map.rain[i] >= 0.75 {
                    wet_fields += 1;
                }
                if self.dom[i] > 0.6 {
                    dom_fields += 1;
                }
            }
        }
        let mut lin: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for b in &self.bands {
            *lin.entry(b.lineage).or_default() += b.pop;
        }
        let tp = self.total_pop();
        let top_lineage_pct = if tp > 0 { 100.0 * lin.values().copied().max().unwrap_or(0) as f32 / tp as f32 } else { 0.0 };
        let mut acc = std::mem::take(&mut self.year_acc);
        self.sea_year_stats(&mut acc);
        self.feast_year_stats(&mut acc);
        // (step 10b) the year's feasts go into the history store's tally in one go
        self.archive.note_many(EventKind::Feast as u8, acc.feasts);
        let nb = self.bands.len() as u32;
        self.history.push(YearStats {
            year: self.tick / 4,
            pop: self.total_pop(),
            bands: nb,
            items_world: self.item_bands.len() as u32,
            items_mean: if nb > 0 { sum as f32 / nb as f32 } else { 0.0 },
            items_max: mx,
            with_fire,
            partnerships: links / 2,
            goods_dist_mean: if gn > 0 { gd / gn as f32 } else { 0.0 },
            goods_dist_max: gmax,
            farmers,
            farmer_pop,
            herders,
            fields,
            dom_fields,
            field_fert: if fields > 0 { fsum / fields as f32 } else { 0.0 },
            farm_food_share: if fall > 0.0 { ff / fall } else { 0.0 },
            farmer_density: if fcells.is_empty() { 0.0 } else { farmer_pop as f32 / fcells.len() as f32 },
            forager_density: if gcells.is_empty() { 0.0 } else { gpop as f32 / gcells.len() as f32 },
            lineages: lin.len() as u32,
            top_lineage_pct,
            eff_lineages: crate::analysis::effective_number(lin.values().map(|&p| p as u64)),
            iron_farm_bands,
            steel_bands,
            weapon_q_mean: (wq_sum / wq_pop.max(1.0)) as f32,
            mark_bands,
            records_bands,
            records_mean: (rec_sum / nb.max(1) as f64) as f32,
            partners_mean: np_sum as f32 / nb.max(1) as f32,
            partners_max: np_max,
            close_ties_mean: close_sum as f32 / nb.max(1) as f32,
            close_ties_max: close_max,
            opened_fields,
            forest_fields,
            wet_fields,
            ..acc
        });
        self.archive_year();
    }

    /// (v10) Close the year in the history store: per-lineage, per-landmass and grid snapshots,
    /// the items' band counts, and this year's tallies (see `Archive`). Reads the world only.
    fn archive_year(&mut self) {
        let w = self.map.w;
        let (gw, gh) = (self.map.w.max(1), self.map.h.max(1));
        let mut lin: BTreeMap<u32, (u32, u32, std::collections::HashSet<usize>, u64)> = BTreeMap::new();
        let mut reg: BTreeMap<u32, (u32, u32, u32, u64)> = BTreeMap::new();
        let mut grid = vec![0u32; GRID * GRID];
        for b in &self.bands {
            let c = b.cell(w);
            let l = lin.entry(b.lineage).or_default();
            l.0 += 1;
            l.1 += b.pop;
            l.2.insert(c);
            l.3 += b.known.len() as u64;
            let r = reg.entry(self.map.landmass_of(c) as u32).or_default();
            r.0 += 1;
            r.1 += b.pop;
            r.2 += (b.sow > 0.0) as u32;
            r.3 += b.known.len() as u64;
            let (gx, gy) = (b.x as usize * GRID / gw, b.y as usize * GRID / gh);
            grid[gy.min(GRID - 1) * GRID + gx.min(GRID - 1)] += b.pop;
        }
        let f = Frame {
            lineages: lin.into_iter().map(|(k, (n, p, cells, kn))| LineageRow(k, n, p, cells.len() as u32, kn as f32 / n.max(1) as f32)).collect(),
            regions: reg.into_iter().map(|(k, (n, p, fa, kn))| RegionRow(k, n, p, fa, kn as f32 / n.max(1) as f32)).collect(),
            grid,
            lore: self.item_bands.iter().map(|(&it, &n)| LoreRow(it, n, 0, 0)).collect(),
            ..Default::default()
        };
        let year = self.history.last().map_or(self.tick / 4, |y| y.year);
        let counters = self.counters.clone();
        self.archive.close_year(year, self.tick, &counters, f);
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
            EventKind::Founded if e.other == 1 => format!("Y{yr}: band {} founded at ({cx},{cy}) (chosen start)", e.band),
            EventKind::Founded => format!("Y{yr}: band {} founded at ({cx},{cy})", e.band),
            EventKind::Landfall => {
                let sz = self.map.landmass_size.get(e.other as usize).copied().unwrap_or(0);
                let kind = if self.map.is_continent(e.other as usize) { "continent" } else { "island" };
                // recipe: 1 = over water, 0 = on foot, u32::MAX = before v9 (item = any boat they had)
                let how = match (e.recipe, it.is_empty()) {
                    (0, _) => " on foot".to_string(),
                    (1, true) => " adrift".to_string(),
                    (_, true) => String::new(),
                    (_, false) => format!(" by {it}"),
                };
                format!("Y{yr}: band {} lands at ({cx},{cy}), the first people on {kind} #{} ({sz} cells){how}", e.band, e.other)
            }
            EventKind::LostLocal => format!("Y{yr}: {it} is no longer made anywhere (it never spread far)"),
            EventKind::FirstTraded => format!("Y{yr}: band {} trades {it} to band {}, the first such exchange", e.band, e.other),
            EventKind::LostAtSea => format!("Y{yr}: {} of band {} lost at sea near ({cx},{cy})", e.other, e.band),
            EventKind::Drifted => format!("Y{yr}: a fishing party of band {} blown out to sea comes ashore at ({cx},{cy}) as band {}", e.band, e.other),
            EventKind::Fight => {
                let w = if it.is_empty() { String::new() } else { format!(" ({it})") };
                format!("Y{yr}: band {}{w} beat band {} in a fight at ({cx},{cy}); {} killed", e.band, e.other, e.recipe)
            }
            EventKind::FirstFight => format!("Y{yr}: the first fight between bands: band {} beat band {} at ({cx},{cy}); {} killed", e.band, e.other, e.recipe),
            EventKind::BloodiestFight => format!("Y{yr}: the deadliest fight yet: band {} beat band {} at ({cx},{cy}); {} killed{}", e.band, e.other, e.recipe, if it.is_empty() { String::new() } else { format!(", winners armed with {it}") }),
            EventKind::Fled => format!("Y{yr}: band {} was driven off by band {} and moved to ({cx},{cy}){}", e.band, e.other, if e.recipe == 1 { " by sea" } else { "" }),
            EventKind::Tribute => format!("Y{yr}: band {} bought peace from band {} with food and goods", e.band, e.other),
            EventKind::Absorbed => format!("Y{yr}: the last {} of band {} were taken in by band {}, who beat them", e.recipe, e.band, e.other),
            EventKind::FirstSeaRefuge => format!("Y{yr}: band {}, driven off by band {}, is the first to escape a stronger neighbour by sea, landing at ({cx},{cy})", e.band, e.other),
            EventKind::FirstCultivation => format!("Y{yr}: band {} at ({cx},{cy}) is the first to clear and sow fields, from wild stands it had long tended", e.band),
            EventKind::FarmingOrigin => format!("Y{yr}: band {} at ({cx},{cy}) begins farming on its own, far from any other fields (origin #{})", e.band, e.recipe),
            EventKind::TookUpFarming => {
                if e.other == 0 {
                    format!("Y{yr}: band {} took up farming near other fields", e.band)
                } else {
                    format!("Y{yr}: band {} learned to farm from band {}", e.band, e.other)
                }
            }
            EventKind::AbandonedFarming => format!("Y{yr}: band {} gave up its fields at ({cx},{cy}) and went back to foraging", e.band),
            EventKind::FirstDomestic => {
                let what = match mat(e.item) {
                    Mat::Grain => "the first fully domesticated grain (it no longer sheds its seed)",
                    Mat::Flax => "the first to grow flax as a fibre crop",
                    Mat::Wool => "the first to shear woolly sheep",
                    _ => "the first with a domesticated crop",
                };
                format!("Y{yr}: band {} at ({cx},{cy}) has {what}", e.band)
            }
            EventKind::FirstHerding => format!("Y{yr}: band {} at ({cx},{cy}) is the first to keep and breed animals", e.band),
            EventKind::FirstEpidemic => format!("Y{yr}: the first epidemic: crowd sickness breaks out in band {} at ({cx},{cy})", e.band),
            EventKind::Epidemic => {
                if e.other == 0 {
                    format!("Y{yr}: sickness breaks out in band {} at ({cx},{cy})", e.band)
                } else {
                    format!("Y{yr}: band {} caught a sickness from band {}", e.band, e.other)
                }
            }
            EventKind::FirstPlough => format!("Y{yr}: band {} at ({cx},{cy}) is the first to till with an ard drawn by oxen", e.band),
            EventKind::FirstRiding => format!("Y{yr}: band {} at ({cx},{cy}) is the first to ride its herd animals", e.band),
            EventKind::Married => format!("Y{yr}: bands {} and {} exchanged spouses", e.band, e.other),
            EventKind::LongestCrossing => {
                let by = if it.is_empty() { String::new() } else { format!(" by {it}") };
                format!("Y{yr}: band {} comes ashore at ({cx},{cy}) on landmass #{} after {} cells of open water{by}, the longest crossing yet", e.band, e.other, e.recipe)
            }
            EventKind::FirstShip => format!(
                "Y{yr}: band {} comes ashore at ({cx},{cy}) on landmass #{} in a {it} after {} cells of open water, the first crossing by ship",
                e.band, e.other, e.recipe
            ),
            EventKind::FirstRecords => format!("Y{yr}: band {} at ({cx},{cy}) is the first to keep count of its goods and dealings, with {it}s", e.band),
            EventKind::FirstFeast => {
                let ale = if e.item == NONE { String::new() } else { " and ale".to_string() };
                format!("Y{yr}: band {} at ({cx},{cy}) holds the first feast, giving band {} {} seasons' food{ale}", e.band, e.other, e.recipe)
            }
            EventKind::Feast => format!("Y{yr}: band {} feasts band {} ({} seasons' food)", e.band, e.other, e.recipe),
            EventKind::FirstChart => {
                let to = if e.other == u32::MAX { String::new() } else { format!(", the way to landmass #{}", e.other) };
                format!("Y{yr}: band {} at ({cx},{cy}) is the first to keep its sailing directions as a chart{to}", e.band)
            }
        }
    }
}

/// One season's economic outcome for a band (see `World::economy`).
#[derive(Clone, Debug, Default)]
pub struct Econ {
    pub cells: Vec<(usize, f32)>,
    /// standing stock in range [plants, game, fish, small game] and the harvest of each
    pub ys: [f32; 5],
    pub h: [f32; 5],
    pub food: f32,
    pub need: f32,
    pub cold: f32,
    pub upkeep: f32,
    pub needed: u32,
    pub used: Vec<Item>,
    pub food_ratio: f32,
    pub store_after: f32,
    pub spare: f32,
    pub stress: f32,
    pub spoil: f32,
    /// food from fields and herds, and from the herd alone
    pub farm_food: f32,
    pub herd_food: f32,
    pub fin: EconFin,
}

/// What `Econ::finish` needs to turn a harvest into food, store and stress.
#[derive(Clone, Copy, Debug, Default)]
pub struct EconFin {
    pub dg: [f32; 5],
    pub store0: f32,
    pub store_cap: f32,
}

impl Econ {
    /// Food, store and stress from the harvest `h` (called again when bands that share cells
    /// turn out to have competed for the same stock).
    pub fn finish(&mut self) {
        let f = self.fin;
        let food = (0..5).map(|r| self.h[r] * f.dg[r]).sum::<f32>() + self.herd_food;
        let need = self.need;
        let mut store = f.store0 * (1.0 - self.spoil);
        let food_ratio = if food >= need {
            store = (store + food - need).min(f.store_cap);
            1.0
        } else {
            let take = store.min(need - food);
            store -= take;
            ((food + take) / need).min(1.0)
        };
        self.food = food;
        self.food_ratio = food_ratio;
        self.store_after = store;
        self.stress = ((1.0 - food_ratio) * 2.0 + 0.15 * self.cold).clamp(0.0, 1.0);
        self.farm_food = self.h[4] * f.dg[4] + self.herd_food;
    }
    pub fn apply(&self, d: &mut crate::band::Derived) {
        d.food_ratio = self.food_ratio;
        d.spare = self.spare;
        d.stress = self.stress;
        d.cold = self.cold;
        d.need = self.need;
        d.upkeep = self.upkeep;
        d.needed = self.needed;
        d.food = self.food;
        d.harvest = self.h;
        d.farm_food = self.farm_food;
        d.farm_share = (self.farm_food / self.food.max(1e-3)).clamp(0.0, 1.0);
    }
}

/// Per-phase timing totals (seconds): 0 regrow+occupancy, 1 band seasons (all), 2 contacts,
/// 3 demography, 4 bookkeeping+stats; within band seasons: 5 economy, 6 farming+disease,
/// 7 experiments, 8 movement, 9 the rest (materials, demography, sea, forgetting); within
/// contacts: 10 finding neighbours, 11 meetings, 12 partner visits
/// A band's harvest on a cell this season: the wild stocks there just before it (phase 2 log).
#[derive(Clone, Copy, Debug)]
pub(crate) struct HarvestMark {
    cell: u32,
    k: u32,
    /// next entry for the same cell (u32::MAX = last)
    next: u32,
    v: [f32; 4],
}

#[derive(Clone, Debug, Default)]
pub struct Prof {
    pub t: [f64; 16],
}
impl Prof {
    pub const NAMES: [&'static str; 16] = [
        "regrow+occ", "band seasons", "contacts", "demography", "stats", "  economy", "  farming", "  experiments", "  movement", "  other",
        "  finding neighbours", "  meetings", "  partner visits", "  prep (toolkits)", "  harvest log", "  merge",
    ];
}

/// A stopwatch that does nothing on the web build (no clock there)
pub struct Tick {
    #[cfg(not(target_arch = "wasm32"))]
    t: std::time::Instant,
}
impl Tick {
    pub fn now() -> Tick {
        Tick {
            #[cfg(not(target_arch = "wasm32"))]
            t: std::time::Instant::now(),
        }
    }
    /// seconds since the last lap
    pub fn lap(&mut self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let n = std::time::Instant::now();
            let d = (n - self.t).as_secs_f64();
            self.t = n;
            d
        }
        #[cfg(target_arch = "wasm32")]
        0.0
    }
}

/// One cell's seasonal regrowth of wild food (plants, game, fish, small game).
#[inline]
#[allow(clippy::too_many_arguments)]
fn regrow_cell(m: &Map, tend: &[f32], s: u32, r: f32, i: usize, pl: &mut f32, ga: &mut f32, fi: &mut f32, sm: &mut f32) {
    if !m.is_land(i) {
        // sea fish
        let k = m.k_fish[i];
        if k > 0.0 {
            *fi += r * (0.3 * *fi * (1.0 - *fi / k) + 0.02 * k);
            *fi = fi.clamp(0.0, k);
        }
        return;
    }
    let t = m.temp_season(i, s);
    let g = ((t - 3.0) / 12.0).clamp(0.0, 1.0);
    let mut k = m.k_plants[i];
    if let Some(&td) = tend.get(i) {
        if td > 0.0 {
            k = farming::wild_plant_cap(k, td);
        }
    }
    if k > 0.0 {
        *pl += r * (0.5 * g * *pl * (1.0 - *pl / k) + 0.03 * k * g) - 0.12 * (1.0 - g) * *pl;
        *pl = pl.clamp(0.0, k);
    }
    let k = m.k_game[i];
    if k > 0.0 {
        *ga += r * (0.12 * *ga * (1.0 - *ga / k) + 0.01 * k);
        *ga = ga.clamp(0.0, k);
    }
    let k = m.k_fish[i];
    if k > 0.0 {
        *fi += r * (0.3 * *fi * (1.0 - *fi / k) + 0.02 * k);
        *fi = fi.clamp(0.0, k);
    }
    // small game breeds fast but suffers in hard winters
    let k = m.k_small.get(i).copied().unwrap_or(0.0);
    if k > 0.0 {
        *sm += r * (0.45 * g.max(0.2) * *sm * (1.0 - *sm / k) + 0.02 * k);
        *sm = sm.clamp(0.0, k);
    }
}
