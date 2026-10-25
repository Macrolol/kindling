//! Terrain and rivers as routes (v8, step 2 of "technologies for activities").
//!
//! Nothing here is a wall or a cap. Moving, meeting and visiting simply cost effort: a step into
//! rough or high country costs more than one across rolling land, so ranges thin out contact and
//! passes and valleys carry it. People with any boat who live on a river use it as a road: along
//! the same river system a cell costs a third of a land step, so moves and scouting reach up to
//! six cells for the effort of two, and partners on the same river visit twice as far.
use super::*;

/// Height change between neighbouring cells that costs nothing extra (rolling country), metres.
const GENTLE: f32 = 150.0;
/// Metres of climb or descent (beyond `GENTLE`) per extra unit of effort.
const PER_STEEP: f32 = 400.0;
/// Above this height thin air, snow and broken ground add effort ...
const HIGH: f32 = 1500.0;
/// ... one unit per this many metres higher.
const PER_HIGH: f32 = 1000.0;
/// Share of a land step's effort a boat spends per river cell.
pub const RIVER_STEP: f32 = 1.0 / 3.0;
/// Partners on the same river system visit as if they lived this much closer.
pub const RIVER_VISIT: f32 = 0.5;
/// Longest scouting trip along a river, cells.
pub const RIVER_SCOUT: usize = 6;
/// Move penalty per unit of effort beyond one step (the same rate scouting always paid per cell).
pub const MOVE_EFFORT: f32 = 0.08;

impl World {
    /// Label the river systems (8-connected river cells, so tributaries join their main stream).
    /// Derived from the map, never saved.
    pub(crate) fn ensure_terrain_layers(&mut self) {
        let m = &self.map;
        let n = m.w * m.h;
        if self.river_sys.len() == n {
            return;
        }
        let mut lab = vec![u32::MAX; n];
        let mut next = 0u32;
        let mut stack = Vec::new();
        for s in 0..n {
            if !m.river.get(s).copied().unwrap_or(false) || lab[s] != u32::MAX {
                continue;
            }
            lab[s] = next;
            stack.push(s);
            while let Some(c) = stack.pop() {
                for (a, b) in nbrs8(c % m.w, c / m.w, m.w, m.h) {
                    let j = b * m.w + a;
                    if m.river[j] && lab[j] == u32::MAX {
                        lab[j] = next;
                        stack.push(j);
                    }
                }
            }
            next += 1;
        }
        self.river_sys = lab;
    }

    /// Effort of one step from cell `a` into the neighbouring cell `b`, in flat-step units (>= 1).
    pub fn step_cost(&self, a: usize, b: usize) -> f32 {
        let m = &self.map;
        let (ha, hb) = (m.height[a].max(0.0), m.height[b].max(0.0));
        1.0 + ((ha - hb).abs() - GENTLE).max(0.0) / PER_STEEP + (hb - HIGH).max(0.0) / PER_HIGH
    }

    /// Effort over land along the straight line from `a` to `b` (one step per cell of the longer
    /// axis, so on flat ground it equals the cell distance).
    pub fn land_cost(&self, a: usize, b: usize) -> f32 {
        let w = self.map.w as i32;
        let (ax, ay, bx, by) = ((a as i32) % w, (a as i32) / w, (b as i32) % w, (b as i32) / w);
        let (dx, dy) = (bx - ax, by - ay);
        let n = dx.abs().max(dy.abs());
        let mut cost = 0.0;
        let mut prev = a;
        for k in 1..=n {
            // nearest cell on the line, rounded half away from zero
            let x = ax + (2 * dx * k + n * dx.signum()) / (2 * n);
            let y = ay + (2 * dy * k + n * dy.signum()) / (2 * n);
            let c = (y * w + x) as usize;
            cost += self.step_cost(prev, c);
            prev = c;
        }
        cost
    }

    /// Are both cells on the same river system?
    pub fn same_river(&self, a: usize, b: usize) -> bool {
        let r = self.river_sys.get(a).copied().unwrap_or(u32::MAX);
        r != u32::MAX && self.river_sys.get(b).copied() == Some(r)
    }

    /// Effort of going from `a` to `b` for people who may paddle along a shared river: never less
    /// than one step, so a river makes far places nearer but not closer than next door.
    pub fn travel_cost(&self, a: usize, b: usize, boat: bool) -> f32 {
        let land = self.land_cost(a, b);
        if boat && self.same_river(a, b) {
            let d = cheb(a, b, self.map.w) as f32;
            land.min((d * RIVER_STEP).max(1.0))
        } else {
            land
        }
    }

    /// A scouting trip downstream or upstream: a random walk along the river from `c`, up to
    /// `RIVER_SCOUT` cells; returns the cell reached and the river cells paddled.
    pub(crate) fn river_scout(&self, c: usize, rng: &mut Rng) -> Option<(usize, usize)> {
        let m = &self.map;
        if self.river_sys.get(c).copied().unwrap_or(u32::MAX) == u32::MAX {
            return None;
        }
        let steps = 2 + rng.below(RIVER_SCOUT - 1);
        let mut path = vec![c];
        let mut cur = c;
        for _ in 0..steps {
            let opts: Vec<usize> = nbrs8(cur % m.w, cur / m.w, m.w, m.h)
                .map(|(a, b)| b * m.w + a)
                .filter(|&j| m.river[j] && !path.contains(&j))
                .collect();
            if opts.is_empty() {
                break;
            }
            cur = opts[rng.below(opts.len())];
            path.push(cur);
        }
        (cur != c).then(|| (cur, path.len() - 1))
    }
}

/// Chebyshev distance between two cells.
pub fn cheb(a: usize, b: usize, w: usize) -> i32 {
    let (ax, ay, bx, by) = ((a % w) as i32, (a / w) as i32, (b % w) as i32, (b / w) as i32);
    (ax - bx).abs().max((ay - by).abs())
}
