//! Effects buffer for the parallel step.
//!
//! During the parallel phases of a season each band (or each meeting) reads the world as it
//! stood at the start of the phase and writes only to its own band(s); everything it would have
//! changed in the shared world (counters, the event log, the registry, firsts, farm layers, flow
//! overlays, new bands) is recorded here and applied afterwards, one buffer at a time in band
//! (or meeting) order. "Is this the first ...?" questions are answered when the effect is applied,
//! so an earlier band in the same season still gets the world-first.
use super::*;

/// Farm layers that bands write to (additive deltas, clamped at apply time).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Layer {
    Camp,
    Tend,
    Dom,
    Fert,
}

#[derive(Clone, Debug)]
pub(crate) enum Op {
    Event(EventKind, u32, u32, u32, Item, u32),
    /// a world-first, unless an earlier band beat it: then `els` (if any) is logged instead
    First { item: Item, band: u32, cell: u32, recipe: u32, kind: EventKind, els: Option<EventKind> },
    /// a recipe found by experiment (inserted into the registry and learned at apply time)
    Invent { action: Action, inputs: Inputs, out: Item, p: f32, cell: u32 },
    RecipeTransfer(u32),
    FarmOrigin { band: u32, cell: u32 },
    Flag { flag: u32, kind: EventKind, band: u32, cell: u32 },
    Epidemic { band: u32, from: u32, cell: u32 },
    Fight { win: u32, lose: u32, cell: u32, item: Item, deaths: u32 },
    FledBySea { band: u32, by: u32, cell: u32, boat: Item, from: u32 },
    TradedFirst { mi: usize, from: u32, to: u32, cell: u32 },
    Cell(Layer, u32, f32),
    TradeLine { from: u32, to: u32, d: i32, q: f32 },
    Conflict(u32, f32),
    Drift(Box<Band>, u32, u32),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Fx {
    pub counters: Counters,
    pub acc: YearStats,
    pub ops: Vec<Op>,
    /// outputs invented this phase (not yet in the band's toolkit)
    pub invented: Vec<Item>,
    pub prof: [f64; 13],
}

impl Fx {
    #[inline]
    pub fn push_event(&mut self, kind: EventKind, band: u32, other: u32, cell: u32, it: Item, recipe: u32) {
        self.ops.push(Op::Event(kind, band, other, cell, it, recipe));
    }
    #[inline]
    pub fn first(&mut self, item: Item, band: u32, cell: u32, recipe: u32, kind: EventKind, els: Option<EventKind>) {
        self.ops.push(Op::First { item, band, cell, recipe, kind, els });
    }
    #[inline]
    pub fn cell(&mut self, l: Layer, c: usize, d: f32) {
        if d != 0.0 {
            self.ops.push(Op::Cell(l, c as u32, d));
        }
    }
    #[inline]
    pub fn lap(&mut self, tk: &mut Option<Tick>, i: usize) {
        if let Some(t) = tk.as_mut() {
            self.prof[i] += t.lap();
        }
    }
}

macro_rules! add_fields {
    ($a:expr, $b:expr; $($f:ident),*) => { $( $a.$f += $b.$f; )* };
}

pub(crate) fn add_counters(a: &mut Counters, b: &Counters) {
    add_fields!(a, b; experiments, inventions, transfers, forgets, splits, deaths, voyages, sea_disasters,
        lost_at_sea, drifts, drift_landings, drift_returns, sightings, scouting, scout_finds, scouts_lost,
        crossings, pairs_seen, meetings, confrontations, avoided, tributes, fled, fights, conflict_deaths,
        displaced, fled_by_sea, absorbed, captives, farm_starts, farm_origins, took_up_farming,
        abandoned_farming, herds_started, herds_lost, epidemics, disease_deaths);
    a.max_fight_deaths = a.max_fight_deaths.max(b.max_fight_deaths);
}

pub(crate) fn add_acc(a: &mut YearStats, b: &YearStats) {
    add_fields!(a, b; experiments, inventions, transfers, forgets, trades, trade_volume, food_shared, trade_dist,
        confrontations, avoided, tributes, fights, conflict_deaths, displaced, absorbed, epidemics, disease_deaths);
}

impl World {
    /// Apply one effects buffer. `band` is the band whose buffer it is (needed for inventions).
    pub(crate) fn apply_fx(&mut self, fx: Fx, mut band: Option<&mut Band>) {
        add_counters(&mut self.counters, &fx.counters);
        add_acc(&mut self.year_acc, &fx.acc);
        if let Some(p) = self.prof.as_mut() {
            for k in 0..13 {
                p.t[k] += fx.prof[k];
            }
        }
        let n = self.map.w * self.map.h;
        for op in fx.ops {
            match op {
                Op::Event(k, b, o, c, it, r) => self.push_event(k, b, o, c, it, r),
                Op::First { item, band: b, cell, recipe, kind, els } => {
                    if !self.firsts.iter().any(|f| f.item == item) {
                        self.firsts.push(First { item, tick: self.tick, band: b, cell, recipe });
                        self.push_event(kind, b, 0, cell, item, recipe);
                    } else if let Some(e) = els {
                        self.push_event(e, b, 0, cell, item, recipe);
                    }
                }
                Op::Invent { action, inputs, out, p, cell } => {
                    let Some(b) = band.as_deref_mut() else { continue };
                    let rid = match self.reg.find(action, &inputs) {
                        Some(r) => r,
                        None => self.reg.insert(Recipe {
                            action: action as u8,
                            inputs,
                            output: out,
                            p,
                            first_tick: self.tick,
                            first_band: b.id,
                            first_cell: cell,
                            inventions: 0,
                            transfers: 0,
                        }),
                    };
                    if b.knows(rid) || b.d.tk.producible.has(out) {
                        continue;
                    }
                    self.reg.recipes[rid as usize].inventions += 1;
                    Self::reinforce_craft(b, &self.reg.recipes[rid as usize], 1.35);
                    b.known.push(Known { recipe: rid, skill: 0.35, since: self.tick, from: FROM_SELF, kin: true, used: true });
                    b.d.known_ver += 1;
                    b.inventions += 1;
                    self.counters.inventions += 1;
                    self.year_acc.inventions += 1;
                    if !self.firsts.iter().any(|f| f.item == out) {
                        self.firsts.push(First { item: out, tick: self.tick, band: b.id, cell, recipe: rid });
                        self.push_event(EventKind::FirstInvention, b.id, 0, cell, out, rid);
                    } else {
                        self.push_event(EventKind::Invention, b.id, 0, cell, out, rid);
                    }
                    self.refresh_toolkit(b);
                }
                Op::RecipeTransfer(r) => {
                    if let Some(rc) = self.reg.recipes.get_mut(r as usize) {
                        rc.transfers += 1;
                    }
                }
                Op::FarmOrigin { band: b, cell } => {
                    let first = self.farm_starts.is_empty();
                    self.farm_starts.push((self.tick, cell, b));
                    self.counters.farm_origins += 1;
                    let k = if first { EventKind::FirstCultivation } else { EventKind::FarmingOrigin };
                    self.push_event(k, b, 0, cell, NONE, self.farm_starts.len() as u32);
                }
                Op::Flag { flag, kind, band: b, cell } => {
                    if self.farm_flags & flag == 0 {
                        self.farm_flags |= flag;
                        self.push_event(kind, b, 0, cell, NONE, u32::MAX);
                    }
                }
                Op::Epidemic { band: b, from, cell } => {
                    let first = self.counters.epidemics == 0;
                    self.counters.epidemics += 1;
                    self.year_acc.epidemics += 1;
                    let k = if first { EventKind::FirstEpidemic } else { EventKind::Epidemic };
                    self.push_event(k, b, from, cell, NONE, u32::MAX);
                }
                Op::Fight { win, lose, cell, item: it, deaths } => {
                    self.counters.fights += 1;
                    self.year_acc.fights += 1;
                    if self.counters.fights == 1 {
                        self.push_event(EventKind::FirstFight, win, lose, cell, it, deaths);
                    } else if deaths >= 10 && deaths as u64 > self.counters.max_fight_deaths {
                        self.push_event(EventKind::BloodiestFight, win, lose, cell, it, deaths);
                    } else {
                        self.push_event(EventKind::Fight, win, lose, cell, it, deaths);
                    }
                    self.counters.max_fight_deaths = self.counters.max_fight_deaths.max(deaths as u64);
                }
                Op::FledBySea { band: b, by, cell, boat, from } => {
                    self.counters.fled_by_sea += 1;
                    if self.counters.fled_by_sea == 1 {
                        self.push_event(EventKind::FirstSeaRefuge, b, by, cell, boat, from);
                    } else {
                        self.push_event(EventKind::Fled, b, by, cell, NONE, 1);
                    }
                }
                Op::TradedFirst { mi, from, to, cell } => {
                    if self.traded_first & (1 << mi) == 0 {
                        self.traded_first |= 1 << mi;
                        self.push_event(EventKind::FirstTraded, from, to, cell, raw(ALL_MATS[mi]), u32::MAX);
                    }
                }
                Op::Cell(l, c, d) => {
                    let c = c as usize;
                    let v = match l {
                        Layer::Camp => self.camp.get_mut(c),
                        Layer::Tend => self.tend.get_mut(c),
                        Layer::Dom => self.dom.get_mut(c),
                        Layer::Fert => self.fert.get_mut(c),
                    };
                    if let Some(v) = v {
                        *v += d;
                        if !matches!(l, Layer::Camp) {
                            *v = v.clamp(0.0, 1.0);
                        }
                    }
                }
                Op::TradeLine { from, to, d, q } => {
                    let w = self.map.w;
                    if self.trade_flow.len() == n {
                        let (fc, tc) = (from as usize, to as usize);
                        let (x0, y0) = ((fc % w) as f32, (fc / w) as f32);
                        let (x1, y1) = ((tc % w) as f32, (tc / w) as f32);
                        let steps = d.max(1);
                        for k in 0..=steps {
                            let t = k as f32 / steps as f32;
                            let (x, y) = ((x0 + (x1 - x0) * t).round() as usize, (y0 + (y1 - y0) * t).round() as usize);
                            self.trade_flow[y * w + x] += q;
                        }
                    }
                }
                Op::Conflict(c, v) => {
                    if let Some(f) = self.conflict_flow.get_mut(c as usize) {
                        *f += v;
                    }
                }
                Op::Drift(mut nb, parent, c) => {
                    let id = self.next_id;
                    self.next_id += 1;
                    nb.id = id;
                    self.push_event(EventKind::Drifted, parent, id, c, NONE, u32::MAX);
                    self.pending.push(*nb);
                }
            }
        }
    }
}

/// Runs `f` over every element, in parallel when the `parallel` feature is on. Results come back
/// in input order either way.
pub(crate) fn par_map<T: Send, R: Send>(v: &mut [T], f: impl Fn(&mut T) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        if v.len() >= 32 {
            return v.par_iter_mut().with_min_len(8).map(f).collect();
        }
    }
    v.iter_mut().map(f).collect()
}

/// `v *= k` for every element (in parallel for big maps).
pub(crate) fn par_scale(v: &mut [f32], k: f32) {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        if v.len() >= 65536 {
            v.par_chunks_mut(16384).for_each(|c| c.iter_mut().for_each(|x| *x *= k));
            return;
        }
    }
    v.iter_mut().for_each(|x| *x *= k);
}

/// `(0..n).map(f).collect()`, in parallel when the feature is on and `n` is big.
pub(crate) fn par_collect<R: Send>(n: usize, f: impl Fn(usize) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        if n >= 64 {
            return (0..n).into_par_iter().with_min_len(16).map(f).collect();
        }
    }
    (0..n).map(f).collect()
}
