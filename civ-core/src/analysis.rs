//! Per-world summaries used by the headless runner and the stats window.
use crate::materials::*;
use crate::world::World;
use serde::Serialize;

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
    /// continents (landmasses >= Map::CONTINENT_CELLS cells) in this world, largest first
    pub continents: Vec<LandmassSummary>,
    /// smaller islands: how many exist, how many were ever reached
    pub islands: u32,
    pub islands_reached: u32,
    /// share of land cells per biome (percent), in Biome order
    pub biome_pct: Vec<(String, f32)>,
    pub trade: TradeSummary,
    pub farming: FarmSummary,
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
    /// bands with boats at the end
    pub bands_with_boats: u32,
    /// year the first boat was made (None = never)
    pub first_boat: Option<f32>,
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
}

pub const STAGES: [&str; 8] = ["none", "stone tools", "fire-making", "hafted tools", "pottery", "copper", "bronze", "iron"];

fn stage_of(items: &[Item]) -> usize {
    let has = |f: &dyn Fn(Item) -> bool| items.iter().any(|&i| f(i));
    if has(&|i| mat(i) == Mat::Iron) {
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
            for (c, &t) in cf.iter().enumerate() {
                if t != u32::MAX {
                    let (x, y) = ((c % mw) as f32, (c / mw) as f32);
                    let d = ((x - fx).powi(2) + (y - fy).powi(2)).sqrt();
                    pts.push((d, (t.saturating_sub(f.tick)) as f32 / 4.0));
                }
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
    let lost = w
        .chronicle
        .iter()
        .filter(|e| e.kind == crate::world::EventKind::LostWorldwide as u8)
        .map(|e| format!("{} (Y{})", item_name(e.item), e.tick / 4))
        .collect();
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
    let mut gap = vec![None::<u32>; nl];
    {
        let n = mw * m.h;
        let mut dist = vec![u32::MAX; n];
        let mut q = std::collections::VecDeque::new();
        for i in 0..n {
            if m.landmass_of(i) as usize == home {
                dist[i] = 0;
                q.push_back(i);
            }
        }
        if home < nl {
            gap[home] = Some(0);
        }
        while let Some(i) = q.pop_front() {
            let (x, y) = ((i % mw) as i32, (i / mw) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= mw as i32 || ny >= m.h as i32 {
                        continue;
                    }
                    let j = ny as usize * mw + nx as usize;
                    if dist[j] != u32::MAX {
                        continue;
                    }
                    let l = m.landmass_of(j) as usize;
                    if l == 0 {
                        dist[j] = dist[i] + 1;
                        q.push_back(j);
                    } else if l < nl && gap[l].is_none() {
                        gap[l] = Some(dist[i]);
                    }
                }
            }
        }
    }
    let mut continents = Vec::new();
    let (mut islands, mut islands_reached) = (0u32, 0u32);
    for l in 1..nl {
        let sz = m.landmass_size[l];
        if sz >= crate::map::Map::CONTINENT_CELLS {
            continents.push(LandmassSummary { id: l as u32, cells: sz, landfall_year: lf(l), pop_now: lpop[l], gap: gap[l] });
        } else {
            islands += 1;
            if lf(l).is_some() {
                islands_reached += 1;
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
            bands_with_boats: w.bands.iter().filter(|b| b.d.kit.boat_q > 0.0).count() as u32,
            first_boat: w.firsts.iter().filter(|f| form(f.item) == Form::Boat).map(|f| f.tick as f32 / 4.0).reduce(f32::min),
        },
        range_cells: range,
        continents,
        islands,
        islands_reached,
        biome_pct,
        trade,
        farming,
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
