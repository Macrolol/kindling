//! The history store and the event log (data groundwork, save format v10).
//!
//! * `EventLog`: the recent-events log with per-class retention. On a big world the old single
//!   40k ring filled with "learned" / "forgot" entries in under one season; now each class has its
//!   own ring, so fights and the life of bands reach back years even there.
//! * `Archive`: what a run did, kept for the life of the save at tiered resolution: a `Frame` a
//!   year for the last `RECENT_YEARS` years and one per decade before that (counter deltas,
//!   event counts by kind, per-lineage, per-landmass and coarse-grid series, per-item
//!   learned / forgotten tallies, weddings between lineages), the whole band genealogy (founding,
//!   splits, merges, deaths, drift landings, absorption) for ever, and every single wedding of the
//!   last `MARRIAGE_YEARS` years. The world series (`World::history`, one `YearStats` a year) and
//!   the chronicle of major events are kept in full as before.
//!
//! Nothing here is read by the simulation, so it can never change a run's outcome.
//!
//! The same types are the planned payloads of the live stream / replay (see docs/DATA_SCHEMA.md):
//! a `Frame` (plus that year's `YearStats`) is the per-year metrics frame, and `Event` (encoded
//! with `codec::encode_events`) is the event feed.
use crate::materials::Item;
use crate::world::{Counters, Event, EventKind};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, VecDeque};

/// Event classes, each with its own ring in `EventLog`.
pub const CLASS_LORE: usize = 0;
pub const CLASS_CONFLICT: usize = 1;
pub const CLASS_LIFE: usize = 2;
pub const CLASS_NAMES: [&str; 3] = ["lore", "conflict", "life"];
/// Ring sizes: learning and forgetting (per-item tallies of them are archived every year), fights
/// and their aftermath, and everything else (splits, deaths, farming, epidemics, inventions...).
pub const RING_CAPS: [usize; 3] = [20_000, 40_000, 40_000];

pub fn class_of(kind: u8) -> usize {
    match EventKind::from_u8(kind) {
        EventKind::Learned | EventKind::Forgot => CLASS_LORE,
        EventKind::Fight | EventKind::Fled | EventKind::Tribute | EventKind::Absorbed => CLASS_CONFLICT,
        _ => CLASS_LIFE,
    }
}

/// Recent events, a ring per class. Serialized as one list in time order (JSON: plain event
/// records, the same shape as the single ring of formats 1-9; binary: `codec::encode_events`).
#[derive(Clone, Debug, Default)]
pub struct EventLog {
    rings: [VecDeque<Event>; 3],
}

impl EventLog {
    pub fn push(&mut self, e: Event) {
        let c = class_of(e.kind);
        let r = &mut self.rings[c];
        if r.len() >= RING_CAPS[c] {
            r.pop_front();
        }
        r.push_back(e);
    }
    pub fn len(&self) -> usize {
        self.rings.iter().map(|r| r.len()).sum()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn ring(&self, class: usize) -> &VecDeque<Event> {
        &self.rings[class]
    }
    /// Every retained event, ring by ring (not in time order; see `all`).
    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.rings.iter().flat_map(|r| r.iter())
    }
    /// Every retained event, oldest first (same-tick events: lore, conflict, life).
    pub fn all(&self) -> Vec<Event> {
        let mut v: Vec<Event> = self.iter().copied().collect();
        v.sort_by_key(|e| e.tick); // stable: keeps ring order within a tick
        v
    }
    /// The newest `n` events that pass `f`, oldest first.
    pub fn recent(&self, n: usize, f: impl Fn(&Event) -> bool) -> Vec<Event> {
        let mut v: Vec<Event> = self.rings.iter().flat_map(|r| r.iter().rev().filter(|e| f(e)).take(n).copied()).collect();
        v.sort_by_key(|e| e.tick);
        let k = v.len().saturating_sub(n);
        v.split_off(k)
    }
    /// First tick each ring still covers (`None` = empty).
    pub fn reach(&self) -> [Option<u32>; 3] {
        [0, 1, 2].map(|c| self.rings[c].front().map(|e| e.tick))
    }
}

impl Serialize for EventLog {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        crate::codec::events::serialize(&self.all(), s)
    }
}
impl<'de> Deserialize<'de> for EventLog {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut l = EventLog::default();
        for e in crate::codec::events::deserialize(d)? {
            l.push(e);
        }
        Ok(l)
    }
}

/// Years kept at yearly resolution; older years are folded into decades.
pub const RECENT_YEARS: u32 = 100;
pub const DECADE: u32 = 10;
/// Years of individual weddings kept (older ones survive as per-lineage counts in the frames).
pub const MARRIAGE_YEARS: u32 = 10;
/// The coarse grid of `Frame::grid` (GRID x GRID blocks over the whole map, row by row).
pub const GRID: usize = 8;
/// Number of event kinds counted in `Frame::kinds` (index = `EventKind as u8`).
pub const KINDS: usize = EventKind::COUNT;

/// One lineage (descent from a founding band, `Band::lineage`): (lineage, bands, people,
/// occupied cells, mean recipes known per band).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LineageRow(pub u32, pub u32, pub u32, pub u32, pub f32);
/// One landmass (`Map::landmass_of`, 1 = largest): (landmass, bands, people, farming bands,
/// mean recipes known per band).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionRow(pub u32, pub u32, pub u32, pub u32, pub f32);
/// One item: (item, bands able to make it at the end, times learned, times forgotten).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LoreRow(pub Item, pub u32, pub u32, pub u32);
/// Weddings between two lineages (smaller id first; the same lineage twice = within it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WedRow(pub u32, pub u32, pub u32);

/// What happened in one year (or, for older history, one decade). Snapshots (`lineages`,
/// `regions`, `grid`, the band counts in `lore`) are taken at the end of the last year covered;
/// tallies (`counters`, `kinds`, learned / forgotten, `weddings`) are summed over the span.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Frame {
    /// last year covered (`YearStats::year` of that year)
    pub year: u32,
    /// years covered: 1, or up to `DECADE` once folded
    pub span: u32,
    /// counter increases over the span (`max` counters: the value at the end)
    pub counters: Counters,
    /// events by kind over the span (index = `EventKind as u8`)
    pub kinds: Vec<u32>,
    pub lineages: Vec<LineageRow>,
    pub regions: Vec<RegionRow>,
    /// people per block of a `GRID` x `GRID` grid over the map
    pub grid: Vec<u32>,
    pub lore: Vec<LoreRow>,
    pub weddings: Vec<WedRow>,
}

impl Frame {
    /// Fold `later` (the next frame in time) into this one.
    fn absorb(&mut self, later: Frame) {
        self.year = later.year;
        self.span += later.span;
        self.counters.add(&later.counters);
        self.kinds.resize(self.kinds.len().max(later.kinds.len()), 0);
        for (a, b) in self.kinds.iter_mut().zip(&later.kinds) {
            *a += b;
        }
        let mut lore: BTreeMap<Item, LoreRow> = self.lore.iter().map(|r| (r.0, LoreRow(r.0, 0, r.2, r.3))).collect();
        for r in &later.lore {
            let e = lore.entry(r.0).or_insert(LoreRow(r.0, 0, 0, 0));
            e.1 = r.1;
            e.2 += r.2;
            e.3 += r.3;
        }
        self.lore = lore.into_values().collect();
        let mut wed: BTreeMap<(u32, u32), u32> = self.weddings.iter().map(|r| ((r.0, r.1), r.2)).collect();
        for r in &later.weddings {
            *wed.entry((r.0, r.1)).or_default() += r.2;
        }
        self.weddings = wed.into_iter().map(|((a, b), n)| WedRow(a, b, n)).collect();
        self.lineages = later.lineages;
        self.regions = later.regions;
        self.grid = later.grid;
    }
}

/// This year's running tallies (saved, so a save in mid-year loses nothing).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    pub kinds: Vec<u32>,
    #[serde(with = "crate::save::pairs")]
    pub learned: BTreeMap<Item, u32>,
    #[serde(with = "crate::save::pairs")]
    pub forgot: BTreeMap<Item, u32>,
    #[serde(with = "crate::save::pairs")]
    pub wed: BTreeMap<(u32, u32), u32>,
}

/// Event kinds kept in the genealogy (for ever).
pub fn genealogical(kind: u8) -> bool {
    matches!(
        EventKind::from_u8(kind),
        EventKind::Founded | EventKind::Split | EventKind::Merged | EventKind::Died | EventKind::Drifted | EventKind::Absorbed
    )
}

/// The history store (see the module docs). Empty in saves before format 10: it starts
/// filling from the first year played after loading them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Archive {
    /// yearly for the last `RECENT_YEARS` years, decadal before that, oldest first
    pub frames: Vec<Frame>,
    /// band genealogy, for ever: Founded, Split (band = parent, other = daughter), Merged
    /// (band joined other), Died, Drifted (band = home band, other = new band), Absorbed
    /// (band's last people taken in by other; recipe = how many)
    #[serde(with = "crate::codec::events")]
    pub genealogy: Vec<Event>,
    /// single weddings (`EventKind::Married`: band, other, cell of `band`) of the last
    /// `MARRIAGE_YEARS` years
    #[serde(with = "crate::codec::events")]
    pub marriages: Vec<Event>,
    /// counters at the end of the last frame (the next frame stores the change)
    pub last_counters: Counters,
    pub tally: Tally,
    /// tick the store began at: 0 for worlds started with it, the load tick for older saves
    /// (nothing before it is in the frames or the genealogy; `None` only while loading those)
    pub begun: Option<u32>,
}

impl Archive {
    /// Note an event (called for every event the world logs, and for weddings).
    pub fn note(&mut self, e: &Event) {
        let k = e.kind as usize;
        if self.tally.kinds.len() <= k {
            self.tally.kinds.resize(KINDS.max(k + 1), 0);
        }
        self.tally.kinds[k] += 1;
        match EventKind::from_u8(e.kind) {
            EventKind::Learned => *self.tally.learned.entry(e.item).or_default() += 1,
            EventKind::Forgot => *self.tally.forgot.entry(e.item).or_default() += 1,
            EventKind::Married => self.marriages.push(*e),
            _ if genealogical(e.kind) => self.genealogy.push(*e),
            _ => {}
        }
    }
    /// Weddings between lineages `la` and `lb` (counted for the frames).
    pub fn note_wedding_lineages(&mut self, la: u32, lb: u32) {
        *self.tally.wed.entry((la.min(lb), la.max(lb))).or_default() += 1;
    }

    /// Close the year: turn the tallies into a frame, fold old years into decades and drop
    /// single weddings older than `MARRIAGE_YEARS`.
    pub fn close_year(&mut self, year: u32, tick: u32, counters: &Counters, mut f: Frame) {
        let t = std::mem::take(&mut self.tally);
        f.year = year;
        f.span = 1;
        f.counters = counters.since(&self.last_counters);
        self.last_counters = counters.clone();
        f.kinds = t.kinds;
        f.kinds.resize(KINDS, 0);
        let mut lore: BTreeMap<Item, LoreRow> = f.lore.iter().map(|r| (r.0, *r)).collect();
        for (it, n) in t.learned {
            lore.entry(it).or_insert(LoreRow(it, 0, 0, 0)).2 += n;
        }
        for (it, n) in t.forgot {
            lore.entry(it).or_insert(LoreRow(it, 0, 0, 0)).3 += n;
        }
        f.lore = lore.into_values().collect();
        f.weddings = t.wed.into_iter().map(|((a, b), n)| WedRow(a, b, n)).collect();
        self.frames.push(f);
        self.fold(year);
        let keep = tick.saturating_sub(4 * MARRIAGE_YEARS);
        let cut = self.marriages.partition_point(|e| e.tick < keep);
        self.marriages.drain(..cut);
    }

    /// Fold yearly frames of whole decades that ended before the recent window into one each.
    fn fold(&mut self, year: u32) {
        let cutoff = year.saturating_sub(RECENT_YEARS);
        let dec = |y: u32| y.saturating_sub(1) / DECADE;
        let mut out: Vec<Frame> = Vec::with_capacity(self.frames.len());
        for f in std::mem::take(&mut self.frames) {
            let old = f.span == 1 && (dec(f.year) + 1) * DECADE <= cutoff;
            match out.last_mut() {
                Some(p) if old && dec(p.year) == dec(f.year) => p.absorb(f),
                _ => out.push(f),
            }
        }
        self.frames = out;
    }
}
