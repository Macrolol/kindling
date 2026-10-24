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
pub const FORMAT_VERSION: u32 = 1;
pub const SUPPORT_FLOOR: u32 = 1;

pub type Migration = fn(serde_json::Value) -> Result<serde_json::Value, String>;
/// `MIGRATIONS[k]` upgrades a payload from `SUPPORT_FLOOR+k` to `SUPPORT_FLOOR+k+1`. Kept forever.
pub const MIGRATIONS: &[Migration] = &[
    // v1 -> v2 goes here
];
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
        let state: SaveState = if h.format_version == FORMAT_VERSION {
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
