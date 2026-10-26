//! Where and when each item first became makeable, per cell, stored sparsely.
//!
//! Until v6.0 this was a dense `Vec<u32>` per item (4 bytes x cells x items, ~760 MB on a
//! 1650x1650 map). Most cells never see most items, so the table is now kept in 64x64 tiles that
//! are allocated only once a band in them can make the item. Saves store just the reached cells
//! as `(cell, tick)` pairs (format 6; see docs/SAVE_FORMAT.md).
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const T: usize = 64;
const NEVER: u32 = u32::MAX;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellFirst {
    w: usize,
    h: usize,
    tw: usize,
    tiles: Vec<Option<Box<[u32]>>>,
}

impl CellFirst {
    pub fn new(w: usize, h: usize) -> Self {
        let (tw, th) = (w.div_ceil(T), h.div_ceil(T));
        CellFirst { w, h, tw, tiles: vec![None; tw * th] }
    }
    /// From the old dense per-cell layout (`u32::MAX` = never).
    pub fn from_dense(w: usize, h: usize, v: &[u32]) -> Self {
        let mut cf = Self::new(w, h);
        for (c, &t) in v.iter().enumerate().take(w * h) {
            if t != NEVER {
                cf.set_first(c, t);
            }
        }
        cf
    }
    #[inline]
    fn locate(&self, c: usize) -> (usize, usize) {
        let (x, y) = (c % self.w, c / self.w);
        ((y / T) * self.tw + x / T, (y % T) * T + x % T)
    }
    /// First tick cell `c` could make the item (`u32::MAX` = never).
    #[inline]
    pub fn get(&self, c: usize) -> u32 {
        if self.w == 0 || c >= self.w * self.h {
            return NEVER;
        }
        let (ti, k) = self.locate(c);
        self.tiles[ti].as_ref().map_or(NEVER, |t| t[k])
    }
    /// Record `tick` for cell `c` unless it already has an earlier entry.
    #[inline]
    pub fn set_first(&mut self, c: usize, tick: u32) {
        if self.w == 0 || c >= self.w * self.h {
            return;
        }
        let (ti, k) = self.locate(c);
        let t = self.tiles[ti].get_or_insert_with(|| vec![NEVER; T * T].into_boxed_slice());
        if t[k] == NEVER {
            t[k] = tick;
        }
    }
    /// Reached cells in ascending cell order, with their first tick.
    pub fn iter(&self) -> impl Iterator<Item = (usize, u32)> + '_ {
        (0..self.h).flat_map(move |y| {
            let ty = y / T;
            (0..self.tw).filter_map(move |tx| self.tiles[ty * self.tw + tx].as_ref().map(|t| (tx, t))).flat_map(move |(tx, t)| {
                let x0 = tx * T;
                let x1 = (x0 + T).min(self.w);
                let row = (y % T) * T;
                (x0..x1).filter_map(move |x| {
                    let v = t[row + x - x0];
                    (v != NEVER).then_some((y * self.w + x, v))
                })
            })
        })
    }
    /// Heap bytes held by the table.
    pub fn bytes(&self) -> usize {
        self.tiles.len() * std::mem::size_of::<Option<Box<[u32]>>>() + self.tiles.iter().flatten().count() * T * T * 4
    }
}

/// On disk: `{w, h, cells: [(cell, tick), ...]}` with cells ascending.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Wire {
    w: usize,
    h: usize,
    cells: Vec<(u32, u32)>,
}

impl Serialize for CellFirst {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if !s.is_human_readable() {
            // format 10 binary: [layout 1, w, h, n, n cell gaps, n zigzag tick deltas] as varints
            let mut b = crate::codec::Wr::default();
            let cells: Vec<(usize, u32)> = self.iter().collect();
            for v in [1, self.w as u64, self.h as u64, cells.len() as u64] {
                b.u(v);
            }
            let mut pc = 0;
            for &(c, _) in &cells {
                b.u((c - pc) as u64);
                pc = c;
            }
            let mut pt = 0i64;
            for &(_, t) in &cells {
                b.i(t as i64 - pt);
                pt = t as i64;
            }
            return crate::codec::ser_blob(&b.0, s);
        }
        Wire { w: self.w, h: self.h, cells: self.iter().map(|(c, t)| (c as u32, t)).collect() }.serialize(s)
    }
}
impl<'de> Deserialize<'de> for CellFirst {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if !d.is_human_readable() {
            use serde::de::Error;
            let blob = crate::codec::de_blob_pub(d)?;
            let mut r = crate::codec::Rd::new(&blob);
            let mut go = || -> Result<CellFirst, String> {
                if r.u()? != 1 {
                    return Err("first-reach blob layout is newer than this build".into());
                }
                let (w, h) = (r.u()? as usize, r.u()? as usize);
                let n = r.len(blob.len())?;
                let mut cells = Vec::with_capacity(n);
                let mut c = 0usize;
                for _ in 0..n {
                    c += r.u()? as usize;
                    cells.push(c);
                }
                let mut cf = CellFirst::new(w, h);
                let mut t = 0i64;
                for c in cells {
                    t += r.i()?;
                    cf.set_first(c, t as u32);
                }
                Ok(cf)
            };
            return go().map_err(D::Error::custom);
        }
        let wire = Wire::deserialize(d)?;
        let mut cf = CellFirst::new(wire.w, wire.h);
        for (c, t) in wire.cells {
            cf.set_first(c as usize, t);
        }
        Ok(cf)
    }
}
