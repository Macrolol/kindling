//! Per-world summaries used by the headless runner and the stats window.
use crate::knowledge::FROM_SELF;
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
    /// fraction of band-known recipes that were self-invented (vs learned / inherited)
    pub self_invented_frac: f32,
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

#[derive(Clone, Debug, Serialize)]
pub struct LandmassSummary {
    pub id: u32,
    pub cells: u32,
    /// year people first arrived (None = never)
    pub landfall_year: Option<f32>,
    pub pop_now: u32,
}

pub const STAGES: [&str; 7] = ["none", "stone tools", "fire-making", "hafted tools", "pottery", "copper", "bronze"];

fn stage_of(items: &[Item]) -> usize {
    let has = |f: &dyn Fn(Item) -> bool| items.iter().any(|&i| f(i));
    if has(&|i| mat(i) == Mat::Bronze) {
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
    for f in &w.firsts {
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
            if k.from == FROM_SELF {
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
    let mut continents = Vec::new();
    let (mut islands, mut islands_reached) = (0u32, 0u32);
    for l in 1..nl {
        let sz = m.landmass_size[l];
        if sz >= crate::map::Map::CONTINENT_CELLS {
            continents.push(LandmassSummary { id: l as u32, cells: sz, landfall_year: lf(l), pop_now: lpop[l] });
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
        range_cells: range,
        continents,
        islands,
        islands_reached,
        biome_pct,
        trade,
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
