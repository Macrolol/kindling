//! Per-world summaries used by the headless runner and the stats window.
use crate::materials::*;
use crate::world::World;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct ItemSummary {
    pub item: Item,
    pub name: String,
    pub year_first: f32,
    /// number of times bands found a recipe for it themselves
    pub inventions: u32,
    pub transfers: u32,
    /// bands able to make it at the end
    pub bands_now: u32,
    /// cells where a band could ever make it
    pub cells_reached: u32,
    /// correlation between distance from the first-discovery cell and the time it arrived there
    pub diffusion_r: Option<f32>,
    /// median spread speed, cells / year
    pub spread_speed: Option<f32>,
    pub how: String,
    /// everything that had to exist first in this world (inputs of the first recipe, and theirs)
    pub needs: Vec<Item>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    pub seed: u64,
    pub years: f32,
    pub pop: u32,
    pub bands: u32,
    pub items_now: u32,
    pub items_mean: f32,
    pub stage_now: &'static str,
    pub stage_ever: &'static str,
    pub experiments: u64,
    pub inventions: u64,
    pub transfers: u64,
    pub forgets: u64,
    pub splits: u64,
    pub deaths: u64,
    pub lost_worldwide: Vec<String>,
    /// every item that was made at some point but that no band can make now, with how often it
    /// was lost. `lost_worldwide` lists only items that had spread to 5+ bands; this also covers
    /// the local tricks lost quietly, so it accounts for any gap between `stage_ever` and
    /// `stage_now`
    #[serde(default)]
    pub gone_now: Vec<GoneItem>,
    pub firsts: Vec<ItemSummary>,
    /// fraction of band-known recipes found by the band itself or an ancestor (kept through
    /// splits) rather than taught by another band. v2 counted only the band's own finds, which
    /// a split immediately relabels "from parent", so it read ~0%.
    pub self_invented_frac: f32,
    /// of all recipe acquisitions by experiment or teaching: share found by experiment
    pub own_find_share: f32,
    pub sea: SeaSummary,
    /// biome and mean temperature where people started (cell of the first discovery)
    pub origin: String,
    /// span of occupied land (bounding box cells)
    pub range_cells: u32,
    /// continents (landmasses of at least `Map::continent_cells` cells) in this world, largest first
    pub continents: Vec<LandmassSummary>,
    /// smaller islands: how many exist, how many were ever reached
    pub islands: u32,
    pub islands_reached: u32,
    /// islands first reached over open water, and the longest such hop (sea cells from land
    /// already settled at the time, not from the homeland)
    #[serde(default)]
    pub islands_by_sea: u32,
    #[serde(default)]
    pub island_hop_max: Option<u32>,
    /// landmass where people started (the one with landfall at year 0) and the one with the most
    /// people now; they differ once a colony outgrows its homeland
    #[serde(default)]
    pub home_landmass: u32,
    #[serde(default)]
    pub most_people_landmass: u32,
    /// share of land cells per biome (percent), in Biome order
    pub biome_pct: Vec<(String, f32)>,
    pub trade: TradeSummary,
    pub farming: FarmSummary,
    /// (v9) founding lineages: how many survive, how concentrated, and per landmass
    pub lineages: LineageSummary,
    /// (v9) splits by cause, and the heritable traits at home and overseas
    pub motives: MotiveSummary,
}

/// (v9) Why bands split, and whether the bold and curious went farther: mean curiosity and
/// ambition of all bands and of bands living off the home landmass.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MotiveSummary {
    pub splits: u64,
    pub dissent_splits: u64,
    pub dissent_mean: f32,
    pub curiosity_mean: f32,
    pub ambition_mean: f32,
    /// bands off the home landmass, and their mean curiosity and ambition (0 when none)
    pub overseas_bands: u32,
    pub curiosity_overseas: f32,
    pub ambition_overseas: f32,
    /// share of land cells any band has ever camped in (%)
    pub explored_pct: f32,
    /// weddings between bands so far, and fights between in-laws
    pub marriages: u64,
    pub fights_in_law: u64,
    /// (v9) recipe refinements so far; people-weighted mean recipe quality (mean over each
    /// band's known recipes); the same per landmass holding 5%+ of people (landmass index,
    /// quality), most people first; mean boat seaworthiness of bands with boats
    pub refinements: u64,
    pub quality_mean: f32,
    pub quality_by_landmass: Vec<(u16, f32)>,
    pub boat_q_mean: f32,
    /// (v9) mean band prestige, its Gini across bands, the share of all prestige held by the
    /// top 10% of bands, and the share of recipes taught by the more prestigious band (%)
    pub prestige_mean: f32,
    pub prestige_gini: f32,
    pub prestige_top10_pct: f32,
    pub prestige_taught_pct: f32,
}

/// Gini coefficient of non-negative values (0 = equal, ->1 = one holds all).
pub fn gini(v: &[f32]) -> f32 {
    let mut s: Vec<f64> = v.iter().map(|&x| x.max(0.0) as f64).collect();
    let tot: f64 = s.iter().sum();
    if s.len() < 2 || tot <= 0.0 {
        return 0.0;
    }
    s.sort_by(|a, b| a.total_cmp(b));
    let n = s.len() as f64;
    let wsum: f64 = s.iter().enumerate().map(|(i, x)| (i as f64 + 1.0) * x).sum();
    ((2.0 * wsum) / (n * tot) - (n + 1.0) / n) as f32
}

fn motive_summary(w: &World) -> MotiveSummary {
    let mw = w.map.w;
    let home = w.landfall.iter().position(|&t| t == 0).unwrap_or(1);
    let mean = |v: &[f32]| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
    let all: Vec<&crate::band::Band> = w.bands.iter().collect();
    let away: Vec<&crate::band::Band> = w.bands.iter().filter(|b| w.map.landmass_of(b.cell(mw)) as usize != home).collect();
    let f = |v: &[&crate::band::Band], g: fn(&crate::band::Band) -> f32| mean(&v.iter().map(|b| g(b)).collect::<Vec<_>>());
    MotiveSummary {
        splits: w.counters.splits,
        dissent_splits: w.counters.dissent_splits,
        dissent_mean: f(&all, |b| b.dissent),
        curiosity_mean: f(&all, |b| b.curiosity),
        ambition_mean: f(&all, |b| b.ambition),
        overseas_bands: away.len() as u32,
        curiosity_overseas: f(&away, |b| b.curiosity),
        ambition_overseas: f(&away, |b| b.ambition),
        marriages: w.counters.marriages,
        fights_in_law: w.counters.fights_in_law,
        refinements: w.counters.refinements,
        quality_mean: {
            let (s, n) = w.bands.iter().fold((0.0f64, 0.0f64), |(s, n), b| (s + band_quality(b) as f64 * b.pop as f64, n + b.pop as f64));
            (s / n.max(1.0)) as f32
        },
        quality_by_landmass: {
            let mut by: std::collections::BTreeMap<u16, (f64, f64)> = Default::default();
            for b in &w.bands {
                let e = by.entry(w.map.landmass_of(b.cell(mw))).or_default();
                e.0 += band_quality(b) as f64 * b.pop as f64;
                e.1 += b.pop as f64;
            }
            let tot: f64 = by.values().map(|e| e.1).sum();
            let mut v: Vec<(u16, f64, f32)> = by.into_iter().filter(|(_, e)| e.1 >= 0.05 * tot).map(|(l, e)| (l, e.1, (e.0 / e.1.max(1.0)) as f32)).collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            v.into_iter().map(|(l, _, q)| (l, q)).collect()
        },
        prestige_mean: f(&all, |b| b.prestige),
        prestige_gini: gini(&all.iter().map(|b| b.prestige).collect::<Vec<_>>()),
        prestige_top10_pct: {
            let mut p: Vec<f32> = all.iter().map(|b| b.prestige.max(0.0)).collect();
            p.sort_by(|a, b| b.total_cmp(a));
            let tot: f32 = p.iter().sum();
            let k = (p.len() + 9) / 10;
            if tot > 0.0 { 100.0 * p[..k].iter().sum::<f32>() / tot } else { 0.0 }
        },
        prestige_taught_pct: 100.0 * w.counters.prestige_taught as f32 / w.counters.transfers.max(1) as f32,
        boat_q_mean: mean(&w.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).map(|b| b.d.kit.boat_q).collect::<Vec<_>>()),
        explored_pct: {
            let land = (0..w.map.w * w.map.h).filter(|&i| w.map.is_land(i));
            let (n, e) = land.fold((0u32, 0u32), |(n, e), i| (n + 1, e + (w.camped.get(i).copied().unwrap_or(0) > 0) as u32));
            100.0 * e as f32 / n.max(1) as f32
        },
    }
}

/// (v9) Mean quality of a band's known recipes (0 when it knows none).
fn band_quality(b: &crate::band::Band) -> f32 {
    if b.known.is_empty() {
        0.0
    } else {
        b.known.iter().map(|k| k.quality).sum::<f32>() / b.known.len() as f32
    }
}

/// (v9) Lineage concentration. `lineage` is a founder label passed down unchanged (like a
/// surname), so the top share says which founder's line spread, not who anyone descends from.
#[derive(Clone, Debug, Default, Serialize)]
pub struct LineageSummary {
    pub alive: u32,
    /// largest lineage's share of all people (%)
    pub top_pct: f32,
    /// effective number of lineages, 1/sum(share^2)
    pub effective: f32,
    /// landmasses holding at least 2% of people, most people first
    pub by_landmass: Vec<LandmassLineages>,
    /// (v9) by ancestry (who people descend from through marriage and captives, not the name
    /// their band carries): the largest founder's share of all descent (%), and the effective
    /// number of founders
    pub ancestry_top_pct: f32,
    pub ancestry_effective: f32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LandmassLineages {
    pub landmass: u32,
    pub pop: u32,
    pub top_lineage: u32,
    pub top_pct: f32,
    pub effective: f32,
    /// (v9) effective number of founders by ancestry
    pub ancestry_effective: f32,
}

/// Effective number of kinds, 1/sum(share^2), from the count of each kind (0 when empty).
/// Exact integer sums, so the result does not depend on iteration order.
pub fn effective_number(counts: impl Iterator<Item = u64>) -> f32 {
    let (mut t, mut s2) = (0u128, 0u128);
    for c in counts {
        t += c as u128;
        s2 += c as u128 * c as u128;
    }
    if s2 == 0 {
        0.0
    } else {
        (t as f64 * t as f64 / s2 as f64) as f32
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FarmSummary {
    /// year of the first sown fields anywhere
    pub first_cultivation: Option<f32>,
    /// independent starts: (year, biome, temp) - far from any other fields, no seed carried in
    pub origins: Vec<(f32, String)>,
    pub first_domestic_grain: Option<f32>,
    pub first_herding: Option<f32>,
    pub first_plough: Option<f32>,
    pub first_epidemic: Option<f32>,
    pub took_up: u64,
    pub abandoned: u64,
    pub epidemics: u64,
    pub disease_deaths: u64,
    /// at the end
    pub farmers: u32,
    pub farmer_pop: u32,
    pub forager_bands: u32,
    pub forager_pop: u32,
    pub herders: u32,
    pub fields: u32,
    pub dom_fields: u32,
    pub field_fert: f32,
    pub farm_food_share: f32,
    pub peak_farm_food_share: f32,
    pub farmer_density: f32,
    pub forager_density: f32,
    /// landmasses with farmers / with foragers at the end
    pub landmasses_farming: u32,
    pub landmasses_foraging: u32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TradeSummary {
    pub trades: u64,
    pub volume: f32,
    pub food_shared: f32,
    /// mean band-to-band distance of a trade (cells)
    pub mean_dist: f32,
    /// last year: trades, standing partnerships, distance of carried metal/obsidian from source
    pub trades_last: u32,
    pub partnerships_last: u32,
    pub goods_dist_mean_last: f32,
    /// furthest any carried copper/tin/bronze/obsidian ever was from where it was gathered
    pub goods_dist_max_ever: f32,
    /// (year, material) of the first time each good changed hands
    pub first_traded: Vec<(f32, String)>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SeaSummary {
    pub voyages: u64,
    pub sea_disasters: u64,
    pub lost_at_sea: u64,
    pub drifts: u64,
    pub drift_landings: u64,
    pub drift_returns: u64,
    pub sightings: u64,
    /// (v5) scouting trips, ones that brought back news of a far shore, ones lost
    pub scouting: u64,
    pub scout_finds: u64,
    pub scouts_lost: u64,
    /// (v5) bands / parties that came ashore alive on another landmass
    pub crossings: u64,
    /// mean seafaring experience of bands with boats at the end
    pub seacraft_mean: f32,
    /// bands with boats at the end
    pub bands_with_boats: u32,
    /// year the first boat was made (None = never)
    pub first_boat: Option<f32>,
    /// (v9) seafaring experience of bands with boats at the end: median, 90th percentile, max
    pub seacraft_p50: f32,
    pub seacraft_p90: f32,
    pub seacraft_max: f32,
    /// (v9) bands that have a sail and know the sea well enough to use it
    pub sail_bands: u32,
    /// (v9) share of bands holding word of a far shore, and of those within their boats' range
    pub rumour_share: f32,
    pub rumour_in_reach_share: f32,
    /// (v9) 90th percentile of boat range (cells) among coastal bands with boats
    pub range_p90: f32,
}

/// The `q` quantile (0..1) of `v` (sorted in place); 0 when empty.
fn quantile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f32 * q).round() as usize]
}

#[derive(Clone, Debug, Serialize)]
pub struct GoneItem {
    pub item: Item,
    pub name: String,
    pub year_first: f32,
    /// times it fell out of use everywhere (0 if no band could make it at the last yearly count)
    pub times_lost: u32,
    /// it had once spread to 5+ bands, so its loss is also in `lost_worldwide`
    pub widespread: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LandmassSummary {
    pub id: u32,
    pub cells: u32,
    /// year people first arrived (None = never)
    pub landfall_year: Option<f32>,
    pub pop_now: u32,
    /// fewest sea cells to cross from the landmass people started on (0 = that landmass)
    #[serde(default)]
    pub gap: Option<u32>,
    /// the landmass people started on
    #[serde(default)]
    pub home: bool,
    /// fewest sea cells to it from any landmass people have reached by the end (0 = reached).
    /// For an unreached continent this is the crossing still ahead, measured from the nearest
    /// settled shore rather than from the homeland
    #[serde(default)]
    pub gap_now: Option<u32>,
    /// (v9) how people first came: Some(true) over water, Some(false) on foot, None = never
    /// reached, a homeland, or a save from before v9
    pub by_sea: Option<bool>,
    /// (v9) fewest sea cells from land already settled when people first arrived
    pub gap_settled: Option<u32>,
    /// (v9) landmass the first arrivals put to sea from (0 = on foot or unknown)
    pub from: u32,
}

impl LandmassSummary {
    /// Reached over open water after the start (a real crossing, not a walk over a strait).
    pub fn crossed(&self) -> bool {
        self.landfall_year.is_some() && self.by_sea == Some(true)
    }
}

pub const STAGES: [&str; 8] = ["none", "stone tools", "fire-making", "hafted tools", "pottery", "copper", "bronze", "iron"];

fn stage_of(items: &[Item]) -> usize {
    let has = |f: &dyn Fn(Item) -> bool| items.iter().any(|&i| f(i));
    // a raw bloom is not yet usable iron; forged (wrought) iron is
    if has(&|i| mat(i) == Mat::Iron && form(i) != Form::Bloom) {
        7
    } else if has(&|i| mat(i) == Mat::Bronze) {
        6
    } else if has(&|i| mat(i) == Mat::Copper) {
        5
    } else if has(&|i| mat(i) == Mat::Ceramic) {
        4
    } else if has(&|i| matches!(form(i), Form::Spear | Form::Axe) && mat(i) != Mat::Wood || form(i) == Form::Axe) {
        3
    } else if has(&|i| form(i) == Form::Fire) {
        2
    } else if has(&|i| matches!(form(i), Form::Edge | Form::Hammer)) {
        1
    } else {
        0
    }
}

pub fn summarize(w: &World) -> Summary {
    let mw = w.map.w;
    let now: Vec<Item> = w.item_bands.keys().copied().collect();
    // only items made through recipes count for "fire-making": a kept wildfire is not a technology
    let recipe_now: Vec<Item> = w
        .bands
        .iter()
        .flat_map(|b| b.d.tk.producible.iter().collect::<Vec<_>>())
        .collect();
    let ever: Vec<Item> = w.firsts.iter().filter(|f| f.recipe != u32::MAX).map(|f| f.item).collect();
    let mut firsts = Vec::new();
    let first_inputs = |it: Item| -> Vec<Item> {
        w.firsts
            .iter()
            .find(|f| f.item == it && f.recipe != u32::MAX)
            .and_then(|f| w.reg.recipes.get(f.recipe as usize))
            .map(|r| r.inputs.iter().copied().filter(|&i| i != NONE && form(i) != Form::Raw).collect())
            .unwrap_or_default()
    };
    for f in &w.firsts {
        let mut needs: Vec<Item> = Vec::new();
        let mut todo = first_inputs(f.item);
        while let Some(i) = todo.pop() {
            if !needs.contains(&i) && i != f.item {
                needs.push(i);
                todo.extend(first_inputs(i));
            }
        }
        let inv: u32 = w.reg.recipes.iter().filter(|r| r.output == f.item).map(|r| r.inventions).sum();
        let tr: u32 = w.reg.recipes.iter().filter(|r| r.output == f.item).map(|r| r.transfers).sum();
        let (mut reached, mut r, mut speed) = (0u32, None, None);
        if let Some(cf) = w.cell_first.get(&f.item) {
            let (fx, fy) = ((f.cell as usize % mw) as f32, (f.cell as usize / mw) as f32);
            let mut pts: Vec<(f32, f32)> = Vec::new();
            for (c, t) in cf.iter() {
                let (x, y) = ((c % mw) as f32, (c / mw) as f32);
                let d = ((x - fx).powi(2) + (y - fy).powi(2)).sqrt();
                pts.push((d, (t.saturating_sub(f.tick)) as f32 / 4.0));
            }
            reached = pts.len() as u32;
            if pts.len() >= 20 {
                let n = pts.len() as f32;
                let (mx, my) = (pts.iter().map(|p| p.0).sum::<f32>() / n, pts.iter().map(|p| p.1).sum::<f32>() / n);
                let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
                for &(a, b) in &pts {
                    sxy += (a - mx) * (b - my);
                    sxx += (a - mx) * (a - mx);
                    syy += (b - my) * (b - my);
                }
                if sxx > 0.0 && syy > 0.0 {
                    r = Some(sxy / (sxx * syy).sqrt());
                }
                let mut sp: Vec<f32> = pts.iter().filter(|p| p.0 > 3.0 && p.1 > 0.0).map(|p| p.0 / p.1).collect();
                sp.sort_by(|a, b| a.partial_cmp(b).unwrap());
                if !sp.is_empty() {
                    speed = Some(sp[sp.len() / 2]);
                }
            }
        }
        let how = if f.recipe == u32::MAX {
            "taken from a wildfire".to_string()
        } else {
            w.reg.recipes[f.recipe as usize].describe()
        };
        firsts.push(ItemSummary {
            item: f.item,
            name: item_name(f.item),
            year_first: f.tick as f32 / 4.0,
            inventions: inv,
            transfers: tr,
            bands_now: w.item_bands.get(&f.item).copied().unwrap_or(0),
            cells_reached: reached,
            diffusion_r: r,
            spread_speed: speed,
            how,
            needs,
        });
    }
    let lost_big: Vec<(Item, u32)> = w
        .chronicle
        .iter()
        .filter(|e| e.kind == crate::world::EventKind::LostWorldwide as u8)
        .map(|e| (e.item, e.tick))
        .collect();
    let gone_now: Vec<GoneItem> = firsts
        .iter()
        .filter(|f| f.bands_now == 0)
        .map(|f| GoneItem {
            item: f.item,
            name: f.name.clone(),
            year_first: f.year_first,
            times_lost: w.item_lost.get(&f.item).copied().unwrap_or(0),
            widespread: lost_big.iter().any(|&(i, _)| i == f.item),
        })
        .collect();
    let lost = lost_big.iter().map(|&(i, t)| format!("{} (Y{})", item_name(i), t / 4)).collect();
    let (mut selfk, mut allk) = (0usize, 0usize);
    for b in &w.bands {
        for k in &b.known {
            allk += 1;
            if k.kin {
                selfk += 1;
            }
        }
    }
    let (mut x0, mut x1, mut y0, mut y1) = (u16::MAX, 0u16, u16::MAX, 0u16);
    for b in &w.bands {
        x0 = x0.min(b.x);
        x1 = x1.max(b.x);
        y0 = y0.min(b.y);
        y1 = y1.max(b.y);
    }
    let range = if w.bands.is_empty() { 0 } else { (x1 - x0 + 1) as u32 * (y1 - y0 + 1) as u32 };
    let last = w.history.last();
    let _ = now;
    let m = &w.map;
    let nl = m.landmass_size.len();
    let mut lpop = vec![0u32; nl];
    for b in &w.bands {
        let l = m.landmass_of(b.cell(mw)) as usize;
        if l < nl {
            lpop[l] += b.pop;
        }
    }
    let lf = |l: usize| w.landfall.get(l).copied().filter(|&t| t != u32::MAX).map(|t| t as f32 / 4.0);
    // sea gaps from the home landmass (breadth-first over sea cells, 8-connected)
    let home = w.landfall.iter().position(|&t| t == 0).unwrap_or(1);
    let gap = m.sea_gaps(|l| l == home);
    let reached = |l: usize| w.landfall.get(l).is_some_and(|&t| t != u32::MAX);
    let gap_now = m.sea_gaps(reached);
    let most_people = (1..nl).max_by_key(|&l| (lpop[l], std::cmp::Reverse(l))).filter(|&l| lpop[l] > 0).unwrap_or(home);
    let (mut islands_by_sea, mut island_hop_max) = (0u32, None::<u32>);
    let rec = |l: usize| w.landfalls.iter().find(|r| r.landmass as usize == l);
    let cont_min = m.continent_cells();
    let mut continents = Vec::new();
    let (mut islands, mut islands_reached) = (0u32, 0u32);
    for l in 1..nl {
        let sz = m.landmass_size[l];
        if sz >= cont_min {
            let r = rec(l);
            continents.push(LandmassSummary {
                id: l as u32,
                cells: sz,
                landfall_year: lf(l),
                pop_now: lpop[l],
                gap: gap.get(l).copied().flatten(),
                home: l == home,
                gap_now: gap_now.get(l).copied().flatten(),
                by_sea: r.map(|r| r.by_sea),
                gap_settled: r.map(|r| r.gap).filter(|&g| g != u32::MAX),
                from: r.map(|r| r.from as u32).unwrap_or(0),
            });
        } else {
            islands += 1;
            if lf(l).is_some() {
                islands_reached += 1;
                if let Some(r) = rec(l).filter(|r| r.by_sea && lf(l).is_some_and(|y| y > 0.0)) {
                    islands_by_sea += 1;
                    if r.gap != u32::MAX {
                        island_hop_max = Some(island_hop_max.unwrap_or(0).max(r.gap));
                    }
                }
            }
        }
    }
    let mut bc = std::collections::BTreeMap::<String, u32>::new();
    let mut land = 0u32;
    for i in 0..mw * m.h {
        if m.is_land(i) {
            land += 1;
            *bc.entry(format!("{:?}", m.biome_at(i))).or_default() += 1;
        }
    }
    let mut trade = TradeSummary::default();
    let mut dsum = 0.0f32;
    for h in &w.history {
        trade.trades += h.trades as u64;
        trade.volume += h.trade_volume;
        trade.food_shared += h.food_shared;
        dsum += h.trade_dist;
        trade.goods_dist_max_ever = trade.goods_dist_max_ever.max(h.goods_dist_max);
    }
    trade.mean_dist = if trade.trades > 0 { dsum / trade.trades as f32 } else { 0.0 };
    if let Some(h) = last {
        trade.trades_last = h.trades;
        trade.partnerships_last = h.partnerships;
        trade.goods_dist_mean_last = h.goods_dist_mean;
    }
    for e in &w.chronicle {
        if e.kind == crate::world::EventKind::FirstTraded as u8 {
            trade.first_traded.push((e.tick as f32 / 4.0, crate::materials::mat(e.item).props().name.to_string()));
        }
    }
    let mut farming = FarmSummary::default();
    {
        use crate::world::EventKind as K;
        let yr = |t: u32| t as f32 / 4.0;
        for e in &w.chronicle {
            let k = e.kind;
            if k == K::FirstCultivation as u8 || k == K::FarmingOrigin as u8 {
                let c = e.cell as usize;
                if k == K::FirstCultivation as u8 {
                    farming.first_cultivation = Some(yr(e.tick));
                }
                farming.origins.push((yr(e.tick), format!("{:?} {:.0}C", m.biome_at(c), m.temp[c])));
            } else if k == K::FirstDomestic as u8 && mat(e.item) == Mat::Grain {
                farming.first_domestic_grain = Some(yr(e.tick));
            } else if k == K::FirstHerding as u8 {
                farming.first_herding = Some(yr(e.tick));
            } else if k == K::FirstPlough as u8 {
                farming.first_plough = Some(yr(e.tick));
            } else if k == K::FirstEpidemic as u8 {
                farming.first_epidemic = Some(yr(e.tick));
            }
        }
        let c = &w.counters;
        farming.took_up = c.took_up_farming;
        farming.abandoned = c.abandoned_farming;
        farming.epidemics = c.epidemics;
        farming.disease_deaths = c.disease_deaths;
        let mut lf = std::collections::HashSet::new();
        let mut lg = std::collections::HashSet::new();
        for b in &w.bands {
            let l = m.landmass_of(b.cell(mw));
            if b.sow > 0.0 {
                lf.insert(l);
            } else {
                farming.forager_bands += 1;
                farming.forager_pop += b.pop;
                lg.insert(l);
            }
        }
        farming.landmasses_farming = lf.len() as u32;
        farming.landmasses_foraging = lg.len() as u32;
        if let Some(h) = last {
            farming.farmers = h.farmers;
            farming.farmer_pop = h.farmer_pop;
            farming.herders = h.herders;
            farming.fields = h.fields;
            farming.dom_fields = h.dom_fields;
            farming.field_fert = h.field_fert;
            farming.farm_food_share = h.farm_food_share;
            farming.farmer_density = h.farmer_density;
            farming.forager_density = h.forager_density;
        }
        farming.peak_farm_food_share = w.history.iter().map(|h| h.farm_food_share).fold(0.0, f32::max);
    }
    let lineages = {
        let mut all: BTreeMap<u32, u64> = BTreeMap::new();
        let mut per: BTreeMap<u16, BTreeMap<u32, u64>> = BTreeMap::new();
        // (v9) ancestry in thousandths of a person, so the sums stay exact integers
        let mut anc: BTreeMap<u32, u64> = BTreeMap::new();
        let mut anc_per: BTreeMap<u16, BTreeMap<u32, u64>> = BTreeMap::new();
        for b in &w.bands {
            *all.entry(b.lineage).or_default() += b.pop as u64;
            let l = m.landmass_of(b.cell(mw));
            *per.entry(l).or_default().entry(b.lineage).or_default() += b.pop as u64;
            for (k, f) in b.ancestry() {
                let q = (1000.0 * b.pop as f64 * f as f64).round() as u64;
                *anc.entry(k).or_default() += q;
                *anc_per.entry(l).or_default().entry(k).or_default() += q;
            }
        }
        let tot: u64 = all.values().sum();
        let top_of = |v: &BTreeMap<u32, u64>| v.iter().max_by_key(|(&k, &p)| (p, std::cmp::Reverse(k))).map(|(&k, &p)| (k, p)).unwrap_or((0, 0));
        let mut by_landmass: Vec<LandmassLineages> = per
            .iter()
            .filter_map(|(&l, v)| {
                let lp: u64 = v.values().sum();
                if lp == 0 || lp * 50 < tot {
                    return None;
                }
                let (k, p) = top_of(v);
                Some(LandmassLineages { landmass: l as u32, pop: lp as u32, top_lineage: k, top_pct: 100.0 * p as f32 / lp as f32, effective: effective_number(v.values().copied()), ancestry_effective: anc_per.get(&l).map(|a| effective_number(a.values().copied())).unwrap_or(0.0) })
            })
            .collect();
        by_landmass.sort_by_key(|x| std::cmp::Reverse(x.pop));
        LineageSummary {
            alive: all.len() as u32,
            top_pct: if tot > 0 { 100.0 * top_of(&all).1 as f32 / tot as f32 } else { 0.0 },
            effective: effective_number(all.values().copied()),
            by_landmass,
            ancestry_top_pct: {
                let at: u64 = anc.values().sum();
                if at > 0 { 100.0 * anc.values().copied().max().unwrap_or(0) as f32 / at as f32 } else { 0.0 }
            },
            ancestry_effective: effective_number(anc.values().copied()),
        }
    };
    let biome_pct = bc.into_iter().map(|(k, v)| (k, 100.0 * v as f32 / land.max(1) as f32)).collect();
    Summary {
        seed: w.cfg.seed,
        years: w.year(),
        pop: w.total_pop(),
        bands: w.bands.len() as u32,
        items_now: w.item_bands.len() as u32,
        items_mean: last.map(|h| h.items_mean).unwrap_or(0.0),
        stage_now: STAGES[stage_of(&recipe_now)],
        stage_ever: STAGES[stage_of(&ever)],
        experiments: w.counters.experiments,
        inventions: w.counters.inventions,
        transfers: w.counters.transfers,
        forgets: w.counters.forgets,
        splits: w.counters.splits,
        deaths: w.counters.deaths,
        lost_worldwide: lost,
        gone_now,
        firsts,
        self_invented_frac: if allk > 0 { selfk as f32 / allk as f32 } else { 0.0 },
        own_find_share: {
            let t = w.counters.inventions + w.counters.transfers;
            if t > 0 { w.counters.inventions as f32 / t as f32 } else { 0.0 }
        },
        origin: w
            .firsts
            .first()
            .map(|f| {
                let c = f.cell as usize;
                let coast = if w.coastal(c) { ", coast" } else { "" };
                format!("{:?} {:.0}C{coast}", w.map.biome_at(c), w.map.temp[c])
            })
            .unwrap_or_default(),
        sea: SeaSummary {
            voyages: w.counters.voyages,
            sea_disasters: w.counters.sea_disasters,
            lost_at_sea: w.counters.lost_at_sea,
            drifts: w.counters.drifts,
            drift_landings: w.counters.drift_landings,
            drift_returns: w.counters.drift_returns,
            sightings: w.counters.sightings,
            scouting: w.counters.scouting,
            scout_finds: w.counters.scout_finds,
            scouts_lost: w.counters.scouts_lost,
            crossings: w.counters.crossings,
            seacraft_mean: {
                let v: Vec<f32> = w.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).map(|b| b.seacraft).collect();
                if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 }
            },
            bands_with_boats: w.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).count() as u32,
            first_boat: w.firsts.iter().filter(|f| form(f.item) == Form::Boat).map(|f| f.tick as f32 / 4.0).reduce(f32::min),
            ..sea_extras(w)
        },
        range_cells: range,
        continents,
        islands,
        islands_reached,
        islands_by_sea,
        island_hop_max,
        home_landmass: home as u32,
        most_people_landmass: most_people as u32,
        biome_pct,
        trade,
        farming,
        lineages,
        motives: motive_summary(w),
    }
}

/// Normalised Kendall-tau distance between two discovery orders over their common items
/// (0 = same order, 1 = reversed). None if fewer than 3 common items.
pub fn order_distance(a: &[Item], b: &[Item]) -> Option<f32> {
    let common: Vec<Item> = a.iter().copied().filter(|i| b.contains(i)).collect();
    if common.len() < 3 {
        return None;
    }
    let pos = |v: &[Item], i: Item| v.iter().position(|&x| x == i).unwrap();
    let (mut disc, mut tot) = (0u32, 0u32);
    for i in 0..common.len() {
        for j in i + 1..common.len() {
            tot += 1;
            let (ai, aj) = (pos(a, common[i]), pos(a, common[j]));
            let (bi, bj) = (pos(b, common[i]), pos(b, common[j]));
            if (ai < aj) != (bi < bj) {
                disc += 1;
            }
        }
    }
    Some(disc as f32 / tot as f32)
}

/// Like `order_distance`, but only over pairs of items where neither had to exist before the
/// other in either world (`needs_a/b(x, y)`: x needed y). Prerequisite chains force the same
/// order everywhere; this measures how much the free choices differ. Diagnostic only.
pub fn order_distance_free(
    a: &[Item],
    b: &[Item],
    needs_a: &dyn Fn(Item, Item) -> bool,
    needs_b: &dyn Fn(Item, Item) -> bool,
) -> Option<f32> {
    let common: Vec<Item> = a.iter().copied().filter(|i| b.contains(i)).collect();
    let pos = |v: &[Item], i: Item| v.iter().position(|&x| x == i).unwrap();
    let (mut disc, mut tot) = (0u32, 0u32);
    for i in 0..common.len() {
        for j in i + 1..common.len() {
            let (x, y) = (common[i], common[j]);
            if needs_a(x, y) || needs_a(y, x) || needs_b(x, y) || needs_b(y, x) {
                continue;
            }
            tot += 1;
            if (pos(a, x) < pos(a, y)) != (pos(b, x) < pos(b, y)) {
                disc += 1;
            }
        }
    }
    if tot < 3 {
        None
    } else {
        Some(disc as f32 / tot as f32)
    }
}

/// (v9) The seafaring distribution and rumour figures of `SeaSummary`.
fn sea_extras(w: &World) -> SeaSummary {
    let boats: Vec<&crate::band::Band> = w.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).collect();
    let mut xp: Vec<f32> = boats.iter().map(|b| b.seacraft).collect();
    let mut range: Vec<f32> = boats
        .iter()
        .filter(|b| w.coastal(b.cell(w.map.w)))
        .map(|b| World::boat_range(&b.d.kit, b.seacraft))
        .collect();
    let n = w.bands.len().max(1) as f32;
    let held: Vec<&crate::band::Band> = w.bands.iter().filter(|b| b.far_shore.is_some()).collect();
    let reach = held
        .iter()
        .filter(|b| crate::world::dist_cells(b.cell(w.map.w), b.far_shore.unwrap() as usize, w.map.w) <= World::boat_range(&b.d.kit, b.seacraft))
        .count();
    SeaSummary {
        seacraft_p50: quantile(&mut xp, 0.5),
        seacraft_p90: quantile(&mut xp, 0.9),
        seacraft_max: xp.last().copied().unwrap_or(0.0),
        sail_bands: boats.iter().filter(|b| b.d.kit.sail != NONE && b.seacraft > 0.4).count() as u32,
        rumour_share: held.len() as f32 / n,
        rumour_in_reach_share: reach as f32 / n,
        range_p90: quantile(&mut range, 0.9),
        ..Default::default()
    }
}
