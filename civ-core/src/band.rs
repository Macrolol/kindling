//! Bands: the unit of simulation (a co-residing group of households). Knowledge is per band.
use crate::knowledge::{Known, Toolkit};
use crate::materials::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Band {
    pub id: u32,
    /// id of the founding band this one descends from
    pub lineage: u32,
    pub parent: u32,
    pub gen: u32,
    pub x: u16,
    pub y: u16,
    pub pop: u32,
    /// stored food (person-seasons)
    pub store: f32,
    /// heritable inclination to tinker (drifts at each split)
    pub curiosity: f32,
    /// keeps a captured fire alight
    pub flame: bool,
    pub known: Vec<Known>,
    /// carried raw materials (0..~1.5 per material)
    pub stock: Vec<f32>,
    pub born: u32,
    /// where each carried material was first gathered (cell; u32::MAX = unknown)
    pub origin: Vec<u32>,
    /// exchange partners: bands this one trades with and visits
    pub partners: Vec<Partner>,
    /// running counters
    pub inventions: u32,
    pub learned: u32,
    pub forgotten: u32,
    // ---- derived each season (not saved)
    #[serde(skip)]
    pub d: Derived,
}

/// A standing exchange relationship with another band (grows with use, fades without).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Partner {
    pub id: u32,
    pub strength: f32,
    pub since: u32,
    /// goods passed either way so far
    pub traded: f32,
}

/// The best item a band uses for each task, and the resulting multipliers.
#[derive(Clone, Copy, Debug)]
pub struct Kit {
    pub hunt: Item,
    pub cut: Item,
    pub dig: Item,
    pub vessel: Item,
    pub axe: Item,
    pub garment: Item,
    pub fire: Item,
    pub kiln: Item,
    pub cord: Item,
    pub hammer: Item,
    pub point: Item,
    pub boat: Item,
    pub net: Item,
    pub trap: Item,
    /// cooking known for plants / meat / fish
    pub cooked: [Item; 3],
    /// gathering efficiency per resource
    pub plants_m: f32,
    pub game_m: f32,
    pub fish_m: f32,
    pub small_m: f32,
    /// food value per unit harvested [plants, game, fish, small game]: cooking makes more of the
    /// same harvest digestible (gelatinised starch, softened meat), so it adds calories, not speed
    pub digest: [f32; 4],
    /// smoking / drying over a fire slows spoilage of stored meat and fish
    pub smoke: bool,
    /// seaworthiness of the best boat (0 = none)
    pub boat_q: f32,
    pub protect: f32,
}
impl Default for Kit {
    fn default() -> Self {
        Kit {
            hunt: NONE,
            cut: NONE,
            dig: NONE,
            vessel: NONE,
            axe: NONE,
            garment: NONE,
            fire: NONE,
            kiln: NONE,
            cord: NONE,
            hammer: NONE,
            point: NONE,
            boat: NONE,
            net: NONE,
            trap: NONE,
            cooked: [NONE; 3],
            plants_m: 1.0,
            game_m: 1.0,
            fish_m: 1.0,
            small_m: 0.35,
            digest: [1.0; 4],
            smoke: false,
            boat_q: 0.0,
            protect: 0.0,
        }
    }
}
impl Kit {
    pub fn used_items(&self) -> [Item; 17] {
        [
            self.hunt, self.cut, self.dig, self.vessel, self.axe, self.garment, self.fire, self.kiln,
            self.cord, self.hammer, self.point, self.cooked[0], self.cooked[1], self.cooked[2],
            self.boat, self.net, self.trap,
        ]
    }
    /// mean food value per unit across the four food kinds (for display)
    pub fn digest_mean(&self) -> f32 {
        self.digest.iter().sum::<f32>() / 4.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct Derived {
    pub tk: Toolkit,
    pub tk_key: Option<(ItemSet, bool, u32)>,
    pub known_ver: u32,
    pub kit: Kit,
    pub food_ratio: f32,
    pub spare: f32,
    pub stress: f32,
    pub cold: f32,
    pub need: f32,
    pub upkeep: f32,
    /// food eaten this season (person-seasons) and the harvest [plants, game, fish, small game]
    pub food: f32,
    pub harvest: [f32; 4],
    pub experiments: u32,
    /// raw materials (bitmask) that the tools in use need
    pub needed: u32,
    /// raw materials (bitmask) that appear in any recipe the band knows
    pub uses: u32,
    /// demand for each material heard from trade partners (lets goods pass down a chain)
    pub demand: [f32; NMAT],
}

pub const MAX_PARTNERS: usize = 8;

impl Band {
    /// Older saves carry stock / origin vectors for fewer materials: pad them.
    pub fn fit_vectors(&mut self) {
        if self.stock.len() < NMAT {
            self.stock.resize(NMAT, 0.0);
        }
        if self.origin.len() < NMAT {
            self.origin.resize(NMAT, u32::MAX);
        }
    }
    pub fn partner(&self, id: u32) -> Option<&Partner> {
        self.partners.iter().find(|p| p.id == id)
    }
    /// Strengthen (or start) a partnership; the weakest link is dropped when there are too many.
    pub fn bond(&mut self, id: u32, by: f32, traded: f32, tick: u32) {
        if let Some(p) = self.partners.iter_mut().find(|p| p.id == id) {
            p.strength = (p.strength + by).min(1.0);
            p.traded += traded;
            return;
        }
        if self.partners.len() >= MAX_PARTNERS {
            let (wi, wp) = self
                .partners
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.strength.total_cmp(&b.1.strength))
                .map(|(i, p)| (i, p.strength))
                .unwrap();
            if wp >= by {
                return;
            }
            self.partners.swap_remove(wi);
        }
        self.partners.push(Partner { id, strength: by.min(1.0), since: tick, traded });
    }
    pub fn cell(&self, w: usize) -> usize {
        self.y as usize * w + self.x as usize
    }
    pub fn knows(&self, recipe: u32) -> bool {
        self.known.iter().any(|k| k.recipe == recipe)
    }
    pub fn n_items(&self) -> u32 {
        self.d.tk.producible.count()
    }
}

/// Choose the best item for each task from what a band can produce; compute multipliers.
pub fn make_kit(avail: &ItemSet) -> Kit {
    let mut k = Kit::default();
    let score = |q: f32, m: Mat| q * (0.75 + 0.25 * durability(m));
    let mut best = [0f32; 14];
    for it in avail.iter() {
        let (f, m) = (form(it), mat(it));
        let mut upd = |slot: usize, s: f32, tgt: &mut Item| {
            if s > best[slot] {
                best[slot] = s;
                *tgt = it;
            }
        };
        match f {
            Form::Spear => upd(0, score(0.35 + 0.65 * cut_q(m), m), &mut k.hunt),
            Form::Point => {
                upd(0, score(0.35 * cut_q(m), m), &mut k.hunt);
                upd(10, score(cut_q(m), m), &mut k.point);
            }
            Form::Edge => {
                upd(0, score(0.2 * cut_q(m), m), &mut k.hunt);
                upd(1, score(cut_q(m), m), &mut k.cut);
            }
            Form::Shaft => upd(2, 1.0, &mut k.dig),
            Form::Vessel => upd(3, vessel_q(m), &mut k.vessel),
            Form::Axe => upd(4, score(0.5 * cut_q(m) + 0.5 * durability(m), m), &mut k.axe),
            Form::Garment => upd(5, garment_q(m), &mut k.garment),
            Form::Fire => upd(6, m.props().fuel_t, &mut k.fire),
            Form::Kiln => upd(7, m.props().refractory, &mut k.kiln),
            Form::Cord => upd(8, m.props().tough + 1.0, &mut k.cord),
            Form::Hammer => upd(9, m.props().hard, &mut k.hammer),
            Form::Boat => upd(11, score(boat_q(m), m), &mut k.boat),
            Form::Net => upd(12, score(mesh_q(m), m), &mut k.net),
            Form::Trap => upd(13, score(mesh_q(m), m), &mut k.trap),
            Form::Cooked => match m {
                Mat::Plants => k.cooked[0] = it,
                Mat::Meat => k.cooked[1] = it,
                Mat::Fish => k.cooked[2] = it,
                _ => {}
            },
            _ => {}
        }
    }
    let q = |it: Item, f: &dyn Fn(Mat) -> f32| if it == NONE { 0.0 } else { f(mat(it)) };
    let hunt_q = if k.hunt == NONE {
        0.0
    } else {
        let m = mat(k.hunt);
        match form(k.hunt) {
            Form::Spear => 0.35 + 0.65 * cut_q(m),
            Form::Point => 0.35 * cut_q(m),
            _ => 0.2 * cut_q(m),
        }
    };
    let cut = q(k.cut, &cut_q);
    let vq = q(k.vessel, &vessel_q);
    let dig = if k.dig == NONE { 0.0 } else { 1.0 };
    let axe = q(k.axe, &|m| 0.5 * cut_q(m) + 0.5 * durability(m));
    let cord = if k.cord == NONE { 0.0 } else { 1.0 };
    let hammer = if k.hammer == NONE { 0.0 } else { 1.0 };
    let point = if k.point == NONE { 0.0 } else { 1.0 };
    let fire = if k.fire == NONE { 0.0 } else { 1.0 };
    let kiln = if k.kiln == NONE { 0.0 } else { 1.0 };
    let ceramic = if k.vessel != NONE && mat(k.vessel) == Mat::Ceramic { 1.0 } else { 0.0 };
    let netq = q(k.net, &mesh_q);
    let trapq = q(k.trap, &mesh_q);
    k.boat_q = q(k.boat, &boat_q);
    k.plants_m = 1.0 + 0.35 * dig + 0.3 * vq + 0.1 * hammer + 0.1 * axe;
    k.game_m = (1.0 + 1.4 * hunt_q + 0.15 * cord) * (1.0 + 0.35 * cut + 0.05 * hammer);
    k.fish_m = 1.0 + 0.8 * hunt_q + 0.4 * vq + 0.3 * point + 0.2 * cord + 0.9 * netq;
    // small game (hares, birds, rodents): hard to catch by hand, easy with snares and nets
    k.small_m = 0.35 + 0.4 * hunt_q + 1.6 * trapq + 0.4 * netq;
    // cooking makes more of each food digestible (boiling in pots helps plants most)
    let ck = |j: usize, g: f32| if k.cooked[j] != NONE { g } else { 1.0 };
    k.digest = [ck(0, 1.25 + 0.15 * ceramic), ck(1, 1.2), ck(2, 1.15), ck(1, 1.2)];
    k.smoke = fire > 0.0 && (k.cooked[1] != NONE || k.cooked[2] != NONE);
    let _ = kiln;
    let gq = q(k.garment, &garment_q);
    k.protect = (0.35 * fire + 0.5 * gq + 0.05 * kiln + 0.05 * axe * fire).min(0.92);
    k
}
