//! Compact, lossless binary encodings (save format v10; meant to be reused as stream/replay frame
//! payloads, see docs/DATA_SCHEMA.md).
//!
//! Binary saves are MessagePack, whose serde (de)serializer reports `is_human_readable() ==
//! false`. The heavy parts of a `World` then go out as length-prefixed byte blobs laid out in
//! columns (varints, zigzag deltas, byte-shuffled f32s), which shrink under deflate far better than
//! named MessagePack records. Human-readable formats (JSON saves, the JSON migration path, and the
//! old binary v5-v9 saves, which `save::load_bytes` decodes in rmp's human-readable mode) keep the
//! plain logical shape, so every older save and every frozen migration still works unchanged.
//!
//! Everything here is exact: floats are stored bit for bit and orders are kept, so a world saved
//! and loaded continues exactly as if it had never stopped.
//!
//! Each blob starts with its own layout number (`*_V`). Changing a layout means a new number and
//! keeping the old decoder: blobs are not migrated through JSON.
use crate::band::Band;
use crate::map::{Map, GEN_VERSION};
use crate::world::Event;
use serde::de::{Deserializer, Error as _, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::cell::Cell;

// ---------------------------------------------------------------- primitive writers / readers

/// Append-only byte writer: LEB128 varints, zigzag deltas, shuffled f32 columns.
#[derive(Default)]
pub struct Wr(pub Vec<u8>);
impl Wr {
    #[inline]
    pub fn u(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.0.push(v as u8 | 0x80);
            v >>= 7;
        }
        self.0.push(v as u8);
    }
    #[inline]
    pub fn i(&mut self, v: i64) {
        self.u(((v << 1) ^ (v >> 63)) as u64)
    }
    /// Length-prefixed bytes.
    pub fn bytes(&mut self, b: &[u8]) {
        self.u(b.len() as u64);
        self.0.extend_from_slice(b);
    }
    /// A column of f32s, bit-exact, byte-shuffled (all low bytes, then the next byte, ...): the
    /// exponent/sign bytes of similar values line up, which deflate compresses well.
    pub fn f32s(&mut self, v: &[f32]) {
        self.u(v.len() as u64);
        self.u32s_shuffled(v.iter().map(|x| x.to_bits()));
    }
    pub fn u32s_shuffled(&mut self, v: impl ExactSizeIterator<Item = u32>) {
        let n = v.len();
        let base = self.0.len();
        self.0.resize(base + 4 * n, 0);
        for (i, x) in v.enumerate() {
            let b = x.to_le_bytes();
            for (k, byte) in b.iter().enumerate() {
                self.0[base + k * n + i] = *byte;
            }
        }
    }
    /// Bits packed 8 to a byte.
    pub fn bits(&mut self, v: impl ExactSizeIterator<Item = bool>) {
        self.u(v.len() as u64);
        let mut cur = 0u8;
        let mut k = 0;
        for b in v {
            cur |= (b as u8) << k;
            k += 1;
            if k == 8 {
                self.0.push(cur);
                cur = 0;
                k = 0;
            }
        }
        if k > 0 {
            self.0.push(cur);
        }
    }
}

pub struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Rd<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Rd { b, p: 0 }
    }
    pub fn done(&self) -> bool {
        self.p >= self.b.len()
    }
    #[inline]
    pub fn u(&mut self) -> Result<u64, String> {
        let mut v = 0u64;
        let mut sh = 0;
        loop {
            let byte = *self.b.get(self.p).ok_or("blob truncated")?;
            self.p += 1;
            if sh >= 64 {
                return Err("bad varint".into());
            }
            v |= ((byte & 0x7f) as u64) << sh;
            if byte & 0x80 == 0 {
                return Ok(v);
            }
            sh += 7;
        }
    }
    #[inline]
    pub fn i(&mut self) -> Result<i64, String> {
        let z = self.u()?;
        Ok((z >> 1) as i64 ^ -((z & 1) as i64))
    }
    pub fn len(&mut self, max: usize) -> Result<usize, String> {
        let n = self.u()? as usize;
        if n > max {
            return Err(format!("blob length {n} out of range"));
        }
        Ok(n)
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.p..self.p + n).ok_or("blob truncated")?;
        self.p += n;
        Ok(s)
    }
    pub fn bytes(&mut self) -> Result<&'a [u8], String> {
        let n = self.len(self.b.len())?;
        self.take(n)
    }
    pub fn u32s_shuffled(&mut self, n: usize) -> Result<Vec<u32>, String> {
        let s = self.take(n.checked_mul(4).ok_or("overflow")?)?;
        Ok((0..n).map(|i| u32::from_le_bytes([s[i], s[n + i], s[2 * n + i], s[3 * n + i]])).collect())
    }
    pub fn f32s(&mut self) -> Result<Vec<f32>, String> {
        let n = self.len(self.b.len() / 4 + 1)?;
        Ok(self.u32s_shuffled(n)?.into_iter().map(f32::from_bits).collect())
    }
    pub fn bits(&mut self) -> Result<Vec<bool>, String> {
        let n = self.len(self.b.len().saturating_mul(8) + 8)?;
        let s = self.take(n.div_ceil(8))?;
        Ok((0..n).map(|i| s[i / 8] >> (i % 8) & 1 == 1).collect())
    }
}

/// Bytes through serde as a MessagePack `bin` (not an array of numbers).
struct Blob<'a>(&'a [u8]);
impl Serialize for Blob<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(self.0)
    }
}
fn de_blob<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a byte blob")
        }
        fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Vec<u8>, E> {
            Ok(v.to_vec())
        }
        fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Vec<u8>, E> {
            Ok(v)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Vec<u8>, A::Error> {
            let mut v = Vec::new();
            while let Some(b) = a.next_element::<u8>()? {
                v.push(b);
            }
            Ok(v)
        }
    }
    d.deserialize_byte_buf(V)
}

// ---------------------------------------------------------------- bands

thread_local! {
    /// Set while band rows are written to / read from a bands blob: the column-stored fields
    /// (`known`, `partners`, `craft`) are then left out of the rows.
    static IN_BLOB: Cell<bool> = const { Cell::new(false) };
}
struct BlobGuard(bool);
impl BlobGuard {
    fn new() -> Self {
        BlobGuard(IN_BLOB.with(|c| c.replace(true)))
    }
}
impl Drop for BlobGuard {
    fn drop(&mut self) {
        let prev = self.0;
        IN_BLOB.with(|c| c.set(prev));
    }
}

/// `#[serde(with)]` for the Band fields stored as columns in a bands blob: normal everywhere
/// else (JSON, old saves, a single band on its own).
pub mod column {
    use super::IN_BLOB;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<T: Serialize, S: Serializer>(v: &T, s: S) -> Result<S::Ok, S::Error> {
        if IN_BLOB.with(|c| c.get()) {
            s.serialize_unit()
        } else {
            v.serialize(s)
        }
    }
    pub fn deserialize<'de, T: Deserialize<'de> + Default, D: Deserializer<'de>>(d: D) -> Result<T, D::Error> {
        if IN_BLOB.with(|c| c.get()) {
            serde::de::IgnoredAny::deserialize(d)?;
            Ok(T::default())
        } else {
            T::deserialize(d)
        }
    }
}

const BANDS_V: u64 = 1;
const FROM_SELF: u64 = 0;
const FROM_INHERITED: u64 = 1;

/// Bands as a blob: the rows (every field but the three below, positional MessagePack: new
/// fields must be appended at the end of `Band` and default when missing), then `known`,
/// `partners` and `craft` as columns across all bands, in their original order.
pub fn encode_bands(bands: &[Band]) -> Result<Vec<u8>, String> {
    let mut w = Wr::default();
    w.u(BANDS_V);
    let rows = {
        let _g = BlobGuard::new();
        rmp_serde::to_vec(bands).map_err(|e| e.to_string())?
    };
    w.bytes(&rows);
    // known
    let mut c = Wr::default();
    let (mut skill, mut qual, mut kin) = (Vec::new(), Vec::new(), Vec::new());
    for b in bands {
        c.u(b.known.len() as u64);
        let (mut pr, mut ps) = (0i64, 0i64);
        for k in &b.known {
            c.i(k.recipe as i64 - pr);
            pr = k.recipe as i64;
            c.i(k.since as i64 - ps);
            ps = k.since as i64;
            c.u(match k.from {
                u32::MAX => FROM_SELF,
                x if x == u32::MAX - 1 => FROM_INHERITED,
                x => 2 + {
                    let d = x as i64 - b.id as i64;
                    ((d << 1) ^ (d >> 63)) as u64
                },
            });
            skill.push(k.skill);
            qual.push(k.quality);
            kin.push(k.kin);
        }
    }
    c.bits(kin.into_iter());
    c.f32s(&skill);
    c.f32s(&qual);
    w.bytes(&c.0);
    // partners
    let mut c = Wr::default();
    let (mut st, mut tr, mut ma) = (Vec::new(), Vec::new(), Vec::new());
    for b in bands {
        c.u(b.partners.len() as u64);
        let mut ps = 0i64;
        for p in &b.partners {
            c.i(p.id as i64 - b.id as i64);
            c.i(p.since as i64 - ps);
            ps = p.since as i64;
            st.push(p.strength);
            tr.push(p.traded);
            ma.push(p.married);
        }
    }
    c.f32s(&st);
    c.f32s(&tr);
    c.f32s(&ma);
    w.bytes(&c.0);
    // craft
    let mut c = Wr::default();
    let mut all = Vec::new();
    for b in bands {
        c.u(b.craft.len() as u64);
        all.extend_from_slice(&b.craft);
    }
    c.f32s(&all);
    w.bytes(&c.0);
    Ok(w.0)
}

pub fn decode_bands(blob: &[u8]) -> Result<Vec<Band>, String> {
    use crate::band::Partner;
    use crate::knowledge::Known;
    let mut r = Rd::new(blob);
    let v = r.u()?;
    if v != BANDS_V {
        return Err(format!("bands blob layout {v} is newer than this build"));
    }
    let rows = r.bytes()?;
    let mut bands: Vec<Band> = {
        let _g = BlobGuard::new();
        rmp_serde::from_slice(rows).map_err(|e| format!("band rows: {e}"))?
    };
    let max = blob.len();
    // known
    let mut c = Rd::new(r.bytes()?);
    let mut heads = Vec::with_capacity(bands.len());
    for b in &mut bands {
        let n = c.len(max)?;
        let (mut pr, mut ps) = (0i64, 0i64);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            pr += c.i()?;
            ps += c.i()?;
            let from = match c.u()? {
                FROM_SELF => u32::MAX,
                FROM_INHERITED => u32::MAX - 1,
                z => {
                    let z = z - 2;
                    (b.id as i64 + ((z >> 1) as i64 ^ -((z & 1) as i64))) as u32
                }
            };
            v.push(Known { recipe: pr as u32, since: ps as u32, from, ..Default::default() });
        }
        heads.push(n);
        b.known = v;
    }
    let kin = c.bits()?;
    let skill = c.f32s()?;
    let qual = c.f32s()?;
    let total: usize = heads.iter().sum();
    if kin.len() != total || skill.len() != total || qual.len() != total {
        return Err("known columns do not match".into());
    }
    let mut j = 0;
    for b in &mut bands {
        for k in &mut b.known {
            k.kin = kin[j];
            k.skill = skill[j];
            k.quality = qual[j];
            j += 1;
        }
    }
    // partners
    let mut c = Rd::new(r.bytes()?);
    for b in &mut bands {
        let n = c.len(max)?;
        let mut ps = 0i64;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let id = (b.id as i64 + c.i()?) as u32;
            ps += c.i()?;
            v.push(Partner { id, since: ps as u32, ..Default::default() });
        }
        b.partners = v;
    }
    let (st, tr, ma) = (c.f32s()?, c.f32s()?, c.f32s()?);
    let total: usize = bands.iter().map(|b| b.partners.len()).sum();
    if st.len() != total || tr.len() != total || ma.len() != total {
        return Err("partner columns do not match".into());
    }
    let mut j = 0;
    for b in &mut bands {
        for p in &mut b.partners {
            (p.strength, p.traded, p.married) = (st[j], tr[j], ma[j]);
            j += 1;
        }
    }
    // craft
    let mut c = Rd::new(r.bytes()?);
    let mut lens = Vec::with_capacity(bands.len());
    for _ in 0..bands.len() {
        lens.push(c.len(max)?);
    }
    let all = c.f32s()?;
    if all.len() != lens.iter().sum::<usize>() {
        return Err("craft column does not match".into());
    }
    let mut j = 0;
    for (b, n) in bands.iter_mut().zip(lens) {
        b.craft = all[j..j + n].to_vec();
        j += n;
    }
    Ok(bands)
}

/// `#[serde(with)]` for `World::bands`.
pub mod bands {
    use super::*;
    pub fn serialize<S: Serializer>(v: &Vec<Band>, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            v.serialize(s)
        } else {
            let blob = encode_bands(v).map_err(serde::ser::Error::custom)?;
            Blob(&blob).serialize(s)
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Band>, D::Error> {
        if d.is_human_readable() {
            Vec::<Band>::deserialize(d)
        } else {
            decode_bands(&de_blob(d)?).map_err(D::Error::custom)
        }
    }
}

// ---------------------------------------------------------------- map

const MAP_V: u64 = 1;

/// The terrain is a pure function of (w, h, seed) for the current generator, so binary saves keep
/// only the bitwise difference from a freshly generated map (all zeros unless something changed
/// it, e.g. an upgraded old map), which deflate reduces to almost nothing. Exact either way: if
/// generation ever came out different, the difference just stops being small.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct MapWire {
    w: usize,
    h: usize,
    seed: u64,
    lat_n: f32,
    lat_s: f32,
    gen_version: u32,
    /// 1 = layers are XOR against `Map::generate(w, h, seed)`, 0 = stored as they are
    base: u8,
    #[serde(with = "serde_bytes_compat")]
    layers: Vec<u8>,
}
mod serde_bytes_compat {
    pub fn serialize<S: serde::Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        super::de_blob(d)
    }
}

macro_rules! map_layers {
    ($m:ident, $f:ident, $u:ident) => {{
        let $f = [&$m.height, &$m.temp, &$m.tamp, &$m.rain, &$m.flow, &$m.k_plants, &$m.k_game, &$m.k_fish, &$m.k_wood, &$m.k_fibre, &$m.k_small];
        let $u = [&$m.biome, &$m.deposits];
        ($f, $u)
    }};
}

/// (inventions step 11) Generator 5 only adds gold, from its own random stream, so a generator-4
/// map is the same map without the gold: saves made before gold still decode against it.
fn base_map(w: usize, h: usize, seed: u64, gen_version: u32) -> Option<Map> {
    if !(w > 0 && h > 0 && w.saturating_mul(h) <= 1 << 26) {
        return None;
    }
    match gen_version {
        GEN_VERSION => Some(Map::generate(w, h, seed)),
        4 => Some(Map::generate_layers(w, h, seed, false)),
        _ => None,
    }
}

pub fn encode_map(m: &Map) -> Result<Vec<u8>, String> {
    let base = base_map(m.w, m.h, m.seed, m.gen_version);
    let empty = Map::default();
    let g = base.as_ref().unwrap_or(&empty);
    let mut w = Wr::default();
    w.u(MAP_V);
    let (mf, mu) = map_layers!(m, f, u);
    let (gf, gu) = map_layers!(g, f, u);
    for (a, b) in mf.iter().zip(gf.iter()) {
        w.u(a.len() as u64);
        w.u32s_shuffled(a.iter().enumerate().map(|(i, x)| x.to_bits() ^ b.get(i).map_or(0, |y| y.to_bits())));
    }
    for (a, b) in mu.iter().zip(gu.iter()) {
        w.u(a.len() as u64);
        w.0.extend(a.iter().enumerate().map(|(i, x)| x ^ b.get(i).copied().unwrap_or(0)));
    }
    w.u(m.river.len() as u64);
    w.0.extend(m.river.iter().enumerate().map(|(i, x)| (*x ^ g.river.get(i).copied().unwrap_or(false)) as u8));
    let wire = MapWire { w: m.w, h: m.h, seed: m.seed, lat_n: m.lat_n, lat_s: m.lat_s, gen_version: m.gen_version, base: base.is_some() as u8, layers: w.0 };
    rmp_serde::to_vec_named(&wire).map_err(|e| e.to_string())
}

pub fn decode_map(blob: &[u8]) -> Result<Map, String> {
    let wire: MapWire = rmp_serde::from_slice(blob).map_err(|e| format!("map: {e}"))?;
    let base = if wire.base == 1 { base_map(wire.w, wire.h, wire.seed, wire.gen_version) } else { None };
    if wire.base == 1 && base.is_none() {
        return Err("map was stored against a generator this build cannot rebuild".into());
    }
    let g = base.unwrap_or_default();
    let mut m = Map { w: wire.w, h: wire.h, seed: wire.seed, lat_n: wire.lat_n, lat_s: wire.lat_s, gen_version: wire.gen_version, ..Default::default() };
    let mut r = Rd::new(&wire.layers);
    if r.u()? != MAP_V {
        return Err("map blob layout is newer than this build".into());
    }
    let max = wire.layers.len();
    {
        let (gf, _) = map_layers!(g, f, u);
        let mut out: Vec<Vec<f32>> = Vec::new();
        for b in gf.iter() {
            let n = r.len(max)?;
            let x = r.u32s_shuffled(n)?;
            out.push(x.into_iter().enumerate().map(|(i, v)| f32::from_bits(v ^ b.get(i).map_or(0, |y| y.to_bits()))).collect());
        }
        let mut it = out.into_iter();
        for dst in [&mut m.height, &mut m.temp, &mut m.tamp, &mut m.rain, &mut m.flow, &mut m.k_plants, &mut m.k_game, &mut m.k_fish, &mut m.k_wood, &mut m.k_fibre, &mut m.k_small] {
            *dst = it.next().unwrap_or_default();
        }
        for (dst, b) in [(&mut m.biome, &g.biome), (&mut m.deposits, &g.deposits)] {
            let n = r.len(max)?;
            *dst = r.take(n)?.iter().enumerate().map(|(i, x)| x ^ b.get(i).copied().unwrap_or(0)).collect();
        }
        let n = r.len(max)?;
        m.river = r.take(n)?.iter().enumerate().map(|(i, x)| (*x != 0) ^ g.river.get(i).copied().unwrap_or(false)).collect();
    }
    Ok(m)
}

/// `#[serde(with)]` for `World::map`.
pub mod map {
    use super::*;
    pub fn serialize<S: Serializer>(m: &Map, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            m.serialize(s)
        } else {
            let blob = encode_map(m).map_err(serde::ser::Error::custom)?;
            Blob(&blob).serialize(s)
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Map, D::Error> {
        if d.is_human_readable() {
            Map::deserialize(d)
        } else {
            decode_map(&de_blob(d)?).map_err(D::Error::custom)
        }
    }
}

// ---------------------------------------------------------------- generic helpers

/// Serialize bytes as a blob (used by other modules).
pub fn ser_blob<S: Serializer>(b: &[u8], s: S) -> Result<S::Ok, S::Error> {
    Blob(b).serialize(s)
}
pub fn de_blob_pub<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    de_blob(d)
}

// ---------------------------------------------------------------- event lists

const EVENTS_V: u64 = 1;

/// A list of events as columns: tick deltas, kinds, bands, `other` relative to the band (split
/// daughters, learners' teachers and in-laws are often close in id), cells, items and recipes
/// (+1, so the common "none" is 0). Also the planned wire form of the event feed.
pub fn encode_events(ev: &[Event]) -> Vec<u8> {
    let mut w = Wr::default();
    w.u(EVENTS_V);
    w.u(ev.len() as u64);
    let mut cols: [Wr; 7] = Default::default();
    let mut t = 0i64;
    for e in ev {
        cols[0].i(e.tick as i64 - t);
        t = e.tick as i64;
        cols[1].0.push(e.kind);
        cols[2].u(e.band as u64);
        cols[3].i(e.other as i64 - e.band as i64);
        cols[4].u(e.cell as u64);
        cols[5].u(e.item.wrapping_add(1) as u64);
        cols[6].u(e.recipe.wrapping_add(1) as u64);
    }
    for c in cols {
        w.bytes(&c.0);
    }
    w.0
}

pub fn decode_events(blob: &[u8]) -> Result<Vec<Event>, String> {
    let mut r = Rd::new(blob);
    if r.u()? != EVENTS_V {
        return Err("event list layout is newer than this build".into());
    }
    let n = r.len(blob.len())?;
    let mut cols = Vec::with_capacity(7);
    for _ in 0..7 {
        cols.push(Rd::new(r.bytes()?));
    }
    let mut out = Vec::with_capacity(n);
    let mut t = 0i64;
    for _ in 0..n {
        t += cols[0].i()?;
        let kind = cols[1].take(1)?[0];
        let band = cols[2].u()? as u32;
        let other = (band as i64 + cols[3].i()?) as u32;
        let cell = cols[4].u()? as u32;
        let item = (cols[5].u()? as u16).wrapping_sub(1);
        let recipe = (cols[6].u()? as u32).wrapping_sub(1);
        out.push(Event { tick: t as u32, kind, band, other, cell, item, recipe });
    }
    Ok(out)
}

/// `#[serde(with)]` for a `Vec<Event>` (the chronicle, the genealogy, ...).
pub mod events {
    use super::*;
    pub fn serialize<S: Serializer>(v: &[Event], s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            v.serialize(s)
        } else {
            Blob(&encode_events(v)).serialize(s)
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Event>, D::Error> {
        if d.is_human_readable() {
            Vec::<Event>::deserialize(d)
        } else {
            // a blob, or (v10 development saves from before the event blob) a plain list
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Vec<Event>;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("an event blob or list")
                }
                fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Vec<Event>, E> {
                    decode_events(v).map_err(E::custom)
                }
                fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Vec<Event>, E> {
                    decode_events(&v).map_err(E::custom)
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Vec<Event>, A::Error> {
                    let mut out = Vec::with_capacity(a.size_hint().unwrap_or(0).min(1 << 20));
                    while let Some(e) = a.next_element::<Event>()? {
                        out.push(e);
                    }
                    Ok(out)
                }
            }
            d.deserialize_any(V)
        }
    }
}
