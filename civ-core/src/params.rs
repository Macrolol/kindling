//! World configuration ([`Config`], fixed at creation) and live-editable parameters ([`Params`],
//! editable mid-run in the settings window, saved with the world, every change logged).
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub seed: u64,
    pub map_w: usize,
    pub map_h: usize,
    /// founding bands, placed together in one temperate/tropical region
    pub founders: usize,
    pub founder_pop: u32,
}
impl Default for Config {
    fn default() -> Self {
        Config { seed: 1, map_w: 256, map_h: 160, founders: 6, founder_pop: 25 }
    }
}

/// Live parameters. Rates are per season (1 tick = 1 season, 4 per year).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Params {
    // invention
    /// expected experiments per band per season at full spare time (scaled by curiosity, size)
    pub invent_rate: f32,
    /// how much stress (hunger, cold) adds to experimenting
    pub stress_invent: f32,
    // transmission
    /// chance per season that two bands one cell apart are in contact
    pub teach_rate: f32,
    /// contact radius in cells
    pub teach_radius: u32,
    /// skill lost per season by an unpractised recipe
    pub forget_rate: f32,
    /// extra forgetting for small groups: x (1 + k / sqrt(pop))
    pub small_band_loss: f32,
    /// skill gained per season of practice
    pub practice: f32,
    // demography
    pub birth_rate: f32,
    pub death_rate: f32,
    pub starve_rate: f32,
    /// band size above which a band tends to split
    pub split_size: f32,
    // ecology / economy
    /// food per unit of labour (multiplies all foraging)
    pub forage_eff: f32,
    /// regrowth speed of plants, game and fish
    pub regen: f32,
    /// food needed per person extra in the cold
    pub cold_scale: f32,
    /// labour fraction spent maintaining each tool (scaled by its fragility)
    pub upkeep: f32,
    /// natural fires (lightning) per dry season per band neighbourhood
    pub wildfire_rate: f32,
    /// chance per season that a kept (not re-makeable) fire goes out
    pub flame_loss: f32,
    // movement
    /// a band moves only if a cell looks this much better than its own
    pub move_inertia: f32,
    // conflict (v4)
    /// how readily a tense meeting (hunger, crowding, a grudge, a neighbour's stores) turns into
    /// a confrontation; 0 = bands never confront each other
    pub aggression: f32,
    /// how deadly a fight is (scales the share of each side killed)
    pub lethality: f32,
    /// share of a grudge still held a year later (0.92: half gone in about 8 years)
    pub grudge_memory: f32,
    // farming (v5)
    /// how quickly repeated camp use turns into tended ground, and tended ground into sown
    /// fields (0 = nobody ever takes up farming)
    pub farm_rate: f32,
    /// food a fully cleared, domesticated, fertile field yields (multiplies crop capacity)
    pub farm_yield: f32,
    /// how fast cropping wears out the soil (0 = soil never tires)
    pub soil_drain: f32,
    /// how readily hunters begin keeping and breeding animals (0 = never)
    pub herd_rate: f32,
    /// strength of crowd diseases (density x sedentism x herd contact; 0 = none)
    pub disease: f32,
}
impl Default for Params {
    fn default() -> Self {
        Params {
            invent_rate: 0.8,
            stress_invent: 0.8,
            teach_rate: 0.3,
            teach_radius: 2,
            forget_rate: 0.003,
            small_band_loss: 3.0,
            practice: 0.08,
            birth_rate: 0.014,
            death_rate: 0.006,
            starve_rate: 0.35,
            split_size: 45.0,
            forage_eff: 1.0,
            regen: 1.0,
            cold_scale: 1.0,
            upkeep: 0.02,
            wildfire_rate: 0.012,
            flame_loss: 0.04,
            move_inertia: 1.12,
            aggression: 1.0,
            lethality: 1.0,
            grudge_memory: 0.92,
            farm_rate: 1.0,
            farm_yield: 1.0,
            soil_drain: 1.0,
            herd_rate: 1.0,
            disease: 1.0,
        }
    }
}

/// One live settings change (kept in the save so a run's history is reproducible / explainable).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ParamChange {
    pub tick: u32,
    pub name: String,
    pub old: f64,
    pub new: f64,
}

impl Params {
    /// Flatten to (field name, numeric value).
    pub fn fields(&self) -> Vec<(String, f64)> {
        match serde_json::to_value(self) {
            Ok(serde_json::Value::Object(m)) => m
                .into_iter()
                .filter_map(|(k, v)| v.as_f64().map(|x| (k, x)))
                .collect(),
            _ => vec![],
        }
    }
    pub fn from_json_compat(s: &str) -> Result<Params, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}
