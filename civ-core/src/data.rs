//! Structured run outputs (data schema v1, see docs/DATA_SCHEMA.md): the run manifest, the
//! per-year metrics table and the event records written by `kindling --headless --out DIR`.
//! Plain functions over the world, so the browser build (and the planned live stream) can
//! produce the same records.
use crate::world::{Counters, Event, EventKind, YearStats};
use crate::World;
use serde::Serialize;

/// Version of the structured outputs (manifest, metrics columns, summary, event records).
/// Adding columns or fields keeps it; renaming or removing them bumps it.
pub const DATA_SCHEMA: u32 = 1;

/// What a run was: enough to reproduce it and to tell runs apart.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Manifest {
    pub schema: u32,
    pub kind: &'static str,
    pub program: String,
    pub version: String,
    /// commit the binary was built from ("unknown" outside a git checkout; "+dirty" if the tree
    /// had changes)
    pub git_sha: String,
    pub seed: u64,
    pub map_w: usize,
    pub map_h: usize,
    /// save loaded to continue from, if any (path as given)
    pub loaded_from: Option<String>,
    /// year the run started at (0, or the loaded save's year) and ended at
    pub start_year: f32,
    pub end_year: f32,
    pub years_requested: u32,
    pub threads: usize,
    pub save_format: u32,
    pub saved_to: Option<String>,
    pub save_bytes: Option<u64>,
    /// seconds: world generation or loading, the simulation, writing the save
    pub t_gen: f32,
    pub t_run: f32,
    pub t_save: f32,
    /// wall-clock start, seconds since 1970 (UTC)
    pub started_unix: u64,
    pub command: Vec<String>,
    /// the parameter set and world config in full (as in a save)
    pub params: serde_json::Value,
    pub config: serde_json::Value,
}

/// Field names and values of a `YearStats`, in declaration order (all scalar).
pub fn year_fields(h: &YearStats) -> Vec<(String, String)> {
    let s = serde_json::to_string(h).unwrap_or_default();
    let s = s.trim_start_matches('{').trim_end_matches('}');
    s.split(',').filter_map(|kv| kv.split_once(':')).map(|(k, v)| (k.trim_matches('"').to_string(), v.to_string())).collect()
}

/// The per-year metrics table as CSV: every `YearStats` column of `history`, then the
/// counters' increase over that year (`c_` + counter name; blank for years before the run, as
/// `deltas[i]` belongs to `history[first + i]`).
pub fn metrics_csv(history: &[YearStats], first: usize, deltas: &[Counters]) -> String {
    let mut out = String::new();
    let Some(h0) = history.first() else { return out };
    let mut head: Vec<String> = year_fields(h0).into_iter().map(|(k, _)| k).collect();
    head.extend(Counters::NAMES.iter().map(|n| format!("c_{n}")));
    out.push_str(&head.join(","));
    out.push('\n');
    for (i, h) in history.iter().enumerate() {
        let mut row: Vec<String> = year_fields(h).into_iter().map(|(_, v)| if v == "null" { String::new() } else { v }).collect();
        match i.checked_sub(first).and_then(|k| deltas.get(k)) {
            Some(c) => row.extend(c.values().iter().map(|v| v.to_string())),
            None => row.extend(std::iter::repeat(String::new()).take(Counters::NAMES.len())),
        }
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

/// One event as a record: `{"t":tick,"year":..,"kind":"Split","band":..,"other":..,"x":..,"y":..,
/// "item":..,"recipe":..,"text":".."}` (x, y: the cell; `other`, `item`, `recipe` omitted when
/// unset).
pub fn event_record(w: &World, e: &Event) -> serde_json::Value {
    let mw = w.map.w.max(1) as u32;
    let mut m = serde_json::Map::new();
    m.insert("t".into(), e.tick.into());
    m.insert("year".into(), serde_json::json!(e.tick as f32 / 4.0));
    m.insert("kind".into(), format!("{:?}", EventKind::from_u8(e.kind)).into());
    m.insert("band".into(), e.band.into());
    if e.other != u32::MAX {
        m.insert("other".into(), e.other.into());
    }
    m.insert("x".into(), (e.cell % mw).into());
    m.insert("y".into(), (e.cell / mw).into());
    if e.item != crate::materials::NONE {
        m.insert("item".into(), e.item.into());
    }
    if e.recipe != u32::MAX {
        m.insert("recipe".into(), e.recipe.into());
    }
    m.insert("text".into(), w.event_text(e).into());
    serde_json::Value::Object(m)
}
