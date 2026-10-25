//! Versioned, forward-compatible save format. See docs/SAVE_FORMAT.md before changing ANYTHING
//! here or any type reachable from `World`.
//!
//! File = MAGIC_BYTES (8 bytes) + raw-deflate( MessagePack-with-named-fields( Envelope ) )
//!   or  = UTF-8 JSON of the same Envelope (export / debugging; detected by leading '{').
//!
//! Rules: every saved struct is `#[serde(default)]` (missing fields load as defaults, unknown
//! fields are ignored); maps are stored as pair lists (so any save converts losslessly to JSON for
//! migrations); any change that is not purely additive bumps FORMAT_VERSION and appends a frozen
//! migration to MIGRATIONS.
use crate::world::World;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const MAGIC_BYTES: &[u8; 8] = b"CIVSAVE\x01";
pub const MAGIC: &str = "civ-sim/world";
pub const FORMAT_VERSION: u32 = 9;
pub const SUPPORT_FLOOR: u32 = 1;

pub type Migration = fn(serde_json::Value) -> Result<serde_json::Value, String>;
/// `MIGRATIONS[k]` upgrades a payload from `SUPPORT_FLOOR+k` to `SUPPORT_FLOOR+k+1`. Kept forever.
pub const MIGRATIONS: &[Migration] = &[v1_to_v2, v2_to_v3, v3_to_v4, v4_to_v5, v5_to_v6, v6_to_v7, v7_to_v8, v8_to_v9];

/// v1 -> v2 (food tech, boats, trade). Frozen: do not edit.
/// - `bands[].stock` held only the 13 gatherable materials; v2 holds all 18 (metals can be
///   carried and traded), so pad with 0.0.
/// - `bands[].origin` (where each carried material came from) is new: unknown (u32::MAX).
/// - `map.gen_version` did not exist: v1 maps came from generator 1; record that explicitly so
///   `Map::upgrade_legacy` fills the new map layers (shelf fish, small game, landmasses).
/// Everything else new in v2 (partners, item_peak, trade stats, landfall...) is additive and
/// loads as its default.
fn v1_to_v2(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    const NMAT_V2: usize = 18;
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    if let Some(m) = world.get_mut("map").and_then(|m| m.as_object_mut()) {
        m.entry("gen_version").or_insert(serde_json::json!(1));
    }
    if let Some(bands) = world.get_mut("bands").and_then(|b| b.as_array_mut()) {
        for b in bands {
            let Some(b) = b.as_object_mut() else { continue };
            let stock = b.entry("stock").or_insert(serde_json::json!([]));
            if let Some(a) = stock.as_array_mut() {
                while a.len() < NMAT_V2 {
                    a.push(serde_json::json!(0.0));
                }
            }
            b.entry("origin").or_insert(serde_json::json!(vec![u32::MAX; NMAT_V2]));
        }
    }
    Ok(v)
}
/// v2 -> v3 (lineage knowledge, craft habits, far shores). Frozen: do not edit.
/// - `bands[].known[].kin` is new: a v2 band's own finds are exactly `from == u32::MAX`
///   (v2 could not tell an ancestor's find from a teacher's, so inherited ones load as false).
/// - `bands[].craft` (21 attention / way-of-working weights) is new: 1.0 = no leaning.
/// - `bands[].far_shore` is new: none known.
fn v2_to_v3(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    const CRAFT_LEN_V3: usize = 21;
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    if let Some(bands) = world.get_mut("bands").and_then(|b| b.as_array_mut()) {
        for b in bands {
            let Some(b) = b.as_object_mut() else { continue };
            if let Some(known) = b.get_mut("known").and_then(|k| k.as_array_mut()) {
                for k in known {
                    let Some(k) = k.as_object_mut() else { continue };
                    let own = k.get("from").and_then(|f| f.as_u64()) == Some(u32::MAX as u64);
                    k.insert("kin".into(), serde_json::json!(own));
                }
            }
            let craft = b.entry("craft").or_insert(serde_json::json!([]));
            if let Some(a) = craft.as_array_mut() {
                while a.len() < CRAFT_LEN_V3 {
                    a.push(serde_json::json!(1.0));
                }
            }
            b.entry("far_shore").or_insert(serde_json::Value::Null);
        }
    }
    Ok(v)
}
/// v3 -> v4 (competition and conflict). Frozen: do not edit.
/// - `bands[].grudges` is new: none held (`[]`).
/// - `bands[].pressed` is new: 0 (no recent pressure from neighbours).
/// - `world.conflict_flow` is new: left out, so it is rebuilt empty on load.
/// Everything else in v4 is additive (conflict counters, YearStats fields, EventKind 17..23,
/// params aggression / lethality / grudge_memory, which take their defaults).
fn v3_to_v4(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    if let Some(bands) = world.get_mut("bands").and_then(|b| b.as_array_mut()) {
        for b in bands {
            let Some(b) = b.as_object_mut() else { continue };
            b.entry("grudges").or_insert(serde_json::json!([]));
            b.entry("pressed").or_insert(serde_json::json!(0.0));
        }
    }
    Ok(v)
}
/// v4 -> v5 (farming stage). Frozen: do not edit.
/// - five materials were appended after bronze (grain, flax, wool, iron ore, iron): per-band
///   `stock` and `origin` grow from 18 to 23 entries (0.0 / u32::MAX);
/// - `craft` is [18 material leanings, 3 action leanings]; the 5 new material leanings (1.0 =
///   neutral) go in at index 18, before the action leanings;
/// - new band fields (`sow`, `strain`, `herd`, `tame`, `settled`, `sick`, `immune`) start at 0;
/// - the per-cell farming layers (`camp`, `tend`, `dom`, `fert`, `crop`) are absent and are built
///   fresh on load (fertility 1, nothing tended); maps below gen 4 gain iron ore on load.
fn v4_to_v5(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    if let Some(bands) = world.get_mut("bands").and_then(|b| b.as_array_mut()) {
        for b in bands {
            let Some(b) = b.as_object_mut() else { continue };
            for (key, fill) in [("stock", json!(0.0)), ("origin", json!(u32::MAX))] {
                if let Some(a) = b.get_mut(key).and_then(|x| x.as_array_mut()) {
                    while a.len() < 23 {
                        a.push(fill.clone());
                    }
                }
            }
            if let Some(a) = b.get_mut("craft").and_then(|x| x.as_array_mut()) {
                if a.len() == 21 {
                    for _ in 0..5 {
                        a.insert(18, json!(1.0));
                    }
                }
            }
            for k in ["sow", "strain", "herd", "tame", "settled", "sick", "immune"] {
                b.entry(k).or_insert(json!(0.0));
            }
        }
    }
    Ok(v)
}
/// v5 -> v6 (sparse first-reach table). Frozen: do not edit.
/// - `world.cell_first` (pairs of item and a dense per-cell list of first ticks, `u32::MAX` =
///   never) becomes `world.cell_first_sparse`: pairs of item and `{w, h, cells: [[cell, tick]..]}`
///   holding only the reached cells, ascending. Nothing else changes.
/// (Binary v5 saves skip this step: the v6 schema still reads the dense field and `after_load`
/// converts it, which avoids building a JSON tree of every cell; both routes give the same world.)
fn v5_to_v6(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    let dim = |k: &str| world.get("map").and_then(|m| m.get(k)).and_then(|x| x.as_u64()).unwrap_or(0);
    let (w, h) = (dim("w"), dim("h"));
    let mut sparse = Vec::new();
    if let Some(serde_json::Value::Array(pairs)) = world.remove("cell_first") {
        for p in pairs {
            let (Some(it), Some(cells)) = (p.get(0).cloned(), p.get(1).and_then(|c| c.as_array())) else { continue };
            let reached: Vec<serde_json::Value> = cells
                .iter()
                .enumerate()
                .filter_map(|(c, t)| t.as_u64().filter(|&t| t != u32::MAX as u64).map(|t| json!([c, t])))
                .collect();
            sparse.push(json!([it, {"w": w, "h": h, "cells": reached}]));
        }
    }
    world.insert("cell_first_sparse".into(), json!(sparse));
    Ok(v)
}
/// v6 -> v7 (iron needs bellows and forging). Frozen: do not edit.
/// Smelting iron ore used to give usable iron (item 22, raw iron) directly; it now gives an iron
/// bloom (item 566 = form 17 x 32 + iron 22) that must be forged into wrought iron. Every heat
/// recipe (action 2) with raw iron ore (item 21) among its inputs and raw iron as its output is
/// re-pointed at the bloom, with the bloom's success chance (0.05). Iron already made, carried,
/// or worked stays as it is.
fn v6_to_v7(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    if let Some(rs) = world.get_mut("reg").and_then(|r| r.get_mut("recipes")).and_then(|r| r.as_array_mut()) {
        for r in rs {
            let heat = r.get("action").and_then(|a| a.as_u64()) == Some(2);
            let to_iron = r.get("output").and_then(|a| a.as_u64()) == Some(22);
            let ore = r.get("inputs").and_then(|a| a.as_array()).map(|a| a.iter().any(|x| x.as_u64() == Some(21))).unwrap_or(false);
            if heat && to_iron && ore {
                r["output"] = serde_json::json!(566);
                r["p"] = serde_json::json!(0.05);
            }
        }
    }
    Ok(v)
}
/// v7 -> v8 (technologies for activities: brine and salt, seven new forms). Frozen: do not edit.
/// - `bands[].stock` and `bands[].origin` grow from 23 to 25 entries (0.0 / u32::MAX);
/// - `bands[].craft` is [23 material leanings, 3 action leanings]; the 2 new material leanings
///   (1.0 = neutral) go in at index 23, before the action leanings.
/// Item ids are unchanged (new forms are appended); everything else new is additive.
fn v7_to_v8(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    if let Some(bands) = world.get_mut("bands").and_then(|b| b.as_array_mut()) {
        for b in bands {
            let Some(b) = b.as_object_mut() else { continue };
            for (key, fill) in [("stock", json!(0.0)), ("origin", json!(u32::MAX))] {
                if let Some(a) = b.get_mut(key).and_then(|x| x.as_array_mut()) {
                    while a.len() < 25 {
                        a.push(fill.clone());
                    }
                }
            }
            if let Some(a) = b.get_mut("craft").and_then(|x| x.as_array_mut()) {
                if a.len() == 26 {
                    for _ in 0..2 {
                        a.insert(23, json!(1.0));
                    }
                }
            }
        }
    }
    Ok(v)
}
/// v8 -> v9 (rumours age). Frozen: do not edit.
/// - `bands[].far_shore_year` is new. A rumour already held is treated as old news, heard 40
///   years before the save (`RUMOUR_YEARS` was 50 when this was written), so it fades within
///   about a decade unless someone sees that land again.
/// Everything else new in v9 (landfall records, lineage stats) is additive.
fn v8_to_v9(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    let world = v.get_mut("world").and_then(|w| w.as_object_mut()).ok_or("payload has no world")?;
    let year = world.get("tick").and_then(|t| t.as_u64()).unwrap_or(0) / 4;
    let heard = year.saturating_sub(40).min(u16::MAX as u64);
    if let Some(bands) = world.get_mut("bands").and_then(|b| b.as_array_mut()) {
        for b in bands {
            let Some(b) = b.as_object_mut() else { continue };
            if b.get("far_shore").is_some_and(|f| !f.is_null()) {
                b.insert("far_shore_year".into(), serde_json::json!(heard));
            }
        }
    }
    Ok(v)
}
/// The same as `v8_to_v9`, for saves decoded straight into the current schema.
fn age_old_rumours(w: &mut World) {
    let heard = (w.tick / 4).saturating_sub(40).min(u16::MAX as u32) as u16;
    for b in &mut w.bands {
        if b.far_shore.is_some() {
            b.far_shore_year = heard;
        }
    }
}
/// The same re-pointing as `v6_to_v7`, for saves decoded straight into the current schema.
fn iron_to_bloom(w: &mut World) {
    use crate::materials::{item, Form, Mat};
    for r in &mut w.reg.recipes {
        if r.action == 2 && r.output == item(Form::Raw, Mat::Iron) && r.inputs.contains(&item(Form::Raw, Mat::IronOre)) {
            r.output = item(Form::Bloom, Mat::Iron);
            r.p = 0.05;
        }
    }
}
const _: () = assert!(
    MIGRATIONS.len() as u32 + SUPPORT_FLOOR == FORMAT_VERSION,
    "FORMAT_VERSION must equal SUPPORT_FLOOR + MIGRATIONS.len()"
);

pub fn migrate(mut v: serde_json::Value, from: u32) -> Result<serde_json::Value, String> {
    if from < SUPPORT_FLOOR {
        return Err(format!("save format v{from} is older than the support floor v{SUPPORT_FLOOR}"));
    }
    for ver in from..FORMAT_VERSION {
        v = MIGRATIONS[(ver - SUPPORT_FLOOR) as usize](v).map_err(|e| format!("migration v{ver}->v{}: {e}", ver + 1))?;
    }
    Ok(v)
}

/// UI state stored with a save.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct ViewState {
    pub cx: f32,
    pub cy: f32,
    pub zoom: f32,
    pub selected: Option<u32>,
    pub overlay: u8,
    pub item: Option<u16>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SaveState {
    pub world: World,
    pub view: Option<ViewState>,
}
#[derive(Serialize)]
struct SaveStateRef<'a> {
    world: &'a World,
    view: Option<&'a ViewState>,
}

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    magic: String,
    format_version: u32,
    app_version: String,
    created_at: String,
    payload: T,
}
#[derive(Deserialize)]
struct Header {
    magic: String,
    format_version: u32,
    #[serde(default)]
    app_version: String,
    #[serde(default)]
    created_at: String,
}

#[derive(Clone, Debug)]
pub struct SaveInfo {
    pub format_version: u32,
    pub app_version: String,
    pub created_at: String,
    pub migrated_from: Option<u32>,
    pub warning: Option<String>,
}

pub fn format_unix_time(secs: f64) -> String {
    let s = secs.max(0.0) as i64;
    let (days, rem) = (s.div_euclid(86400), s.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

enum Enc {
    Mp(Vec<u8>),
    Json,
}

impl World {
    pub fn save_bytes(&self, view: Option<&ViewState>, app_version: &str, created_at: &str) -> Result<Vec<u8>, String> {
        let env = Envelope {
            magic: MAGIC.into(),
            format_version: FORMAT_VERSION,
            app_version: app_version.into(),
            created_at: created_at.into(),
            payload: SaveStateRef { world: self, view },
        };
        let mp = rmp_serde::to_vec_named(&env).map_err(|e| e.to_string())?;
        let mut out = MAGIC_BYTES.to_vec();
        let mut enc = flate2::write::DeflateEncoder::new(out.split_off(8), flate2::Compression::new(6));
        enc.write_all(&mp).map_err(|e| e.to_string())?;
        let body = enc.finish().map_err(|e| e.to_string())?;
        out.extend_from_slice(&body);
        Ok(out)
    }
    pub fn save_json(&self, view: Option<&ViewState>, app_version: &str, created_at: &str) -> Result<String, String> {
        let env = Envelope {
            magic: MAGIC.into(),
            format_version: FORMAT_VERSION,
            app_version: app_version.into(),
            created_at: created_at.into(),
            payload: SaveStateRef { world: self, view },
        };
        serde_json::to_string(&env).map_err(|e| e.to_string())
    }

    pub fn load_bytes(b: &[u8]) -> Result<(World, Option<ViewState>, SaveInfo), String> {
        let enc = if b.len() >= 8 && &b[..8] == MAGIC_BYTES {
            let mut d = flate2::read::DeflateDecoder::new(&b[8..]);
            let mut raw = Vec::new();
            d.read_to_end(&mut raw).map_err(|e| format!("decompress: {e}"))?;
            Enc::Mp(raw)
        } else if b.iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'{') {
            Enc::Json
        } else {
            return Err("not a civ-sim save file".into());
        };
        fn dec<T: serde::de::DeserializeOwned>(e: &Enc, b: &[u8]) -> Result<T, String> {
            match e {
                Enc::Mp(raw) => rmp_serde::from_slice(raw).map_err(|x| x.to_string()),
                Enc::Json => serde_json::from_slice(b).map_err(|x| x.to_string()),
            }
        }
        let h: Header = dec(&enc, b).map_err(|e| format!("header: {e}"))?;
        if h.magic != MAGIC {
            return Err(format!("bad magic {:?}", h.magic));
        }
        if h.format_version == 0 {
            return Err("format_version 0 is invalid".into());
        }
        let mut info = SaveInfo {
            format_version: h.format_version,
            app_version: h.app_version,
            created_at: h.created_at,
            migrated_from: None,
            warning: None,
        };
        // v5 -> v6 only moved the first-reach table, which the v6 schema still reads in its old
        // form; decoding v5 binaries directly keeps big-map saves out of a JSON value tree
        // v6 -> v7 only re-points a few recipes, done in place below for binaries, too
        // v7 -> v8 only lengthens per-material vectors, which `Band::fit_vectors` does on load
        // v8 -> v9 only dates the rumours already held, done in place below
        let direct = h.format_version == FORMAT_VERSION || (matches!(h.format_version, 5 | 6 | 7 | 8) && matches!(enc, Enc::Mp(_)));
        if matches!(h.format_version, 5 | 6 | 7 | 8) {
            info.migrated_from = Some(h.format_version);
        }
        let state: SaveState = if direct {
            dec::<Envelope<SaveState>>(&enc, b)?.payload
        } else if h.format_version < FORMAT_VERSION {
            let env: Envelope<serde_json::Value> = dec(&enc, b)?;
            info.migrated_from = Some(h.format_version);
            serde_json::from_value(migrate(env.payload, h.format_version)?).map_err(|e| e.to_string())?
        } else {
            info.warning = Some(format!(
                "save is format v{} (newer than this build's v{FORMAT_VERSION}); loaded best-effort",
                h.format_version
            ));
            dec::<Envelope<SaveState>>(&enc, b)?.payload
        };
        let mut w = state.world;
        if direct && h.format_version < 7 {
            iron_to_bloom(&mut w);
        }
        if direct && h.format_version < 9 {
            age_old_rumours(&mut w);
        }
        w.after_load();
        Ok((w, state.view, info))
    }
}

/// Serialize a map as a list of pairs (keeps every save convertible to JSON).
pub mod pairs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;
    pub fn serialize<K: Serialize + Ord, V: Serialize, S: Serializer>(m: &BTreeMap<K, V>, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(m.iter())
    }
    pub fn deserialize<'de, K: Deserialize<'de> + Ord, V: Deserialize<'de>, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<K, V>, D::Error> {
        let v: Vec<(K, V)> = Vec::deserialize(d)?;
        Ok(v.into_iter().collect())
    }
}
