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
    /// running counters
    pub inventions: u32,
    pub learned: u32,
    pub forgotten: u32,
    // ---- derived each season (not saved)
    #[serde(skip)]
    pub d: Derived,
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
    /// cooking known for plants / meat / fish
    pub cooked: [Item; 3],
    pub plants_m: f32,
    pub game_m: f32,
    pub fish_m: f32,
    pub cook_m: f32,
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
            cooked: [NONE; 3],
            plants_m: 1.0,
            game_m: 1.0,
            fish_m: 1.0,
            cook_m: 1.0,
            protect: 0.0,
        }
    }
}
impl Kit {
    pub fn used_items(&self) -> [Item; 14] {
        [
            self.hunt, self.cut, self.dig, self.vessel, self.axe, self.garment, self.fire, self.kiln,
            self.cord, self.hammer, self.point, self.cooked[0], self.cooked[1], self.cooked[2],
        ]
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
    pub experiments: u32,
    /// raw materials (bitmask) that the tools in use need
    pub needed: u32,
}

impl Band {
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
    let mut best = [0f32; 11];
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
    // cooking makes more of each food digestible (boiling in pots helps plants most)
    let ck = |j: usize, g: f32| if k.cooked[j] != NONE { g } else { 1.0 };
    k.plants_m = (1.0 + 0.35 * dig + 0.3 * vq + 0.1 * hammer + 0.1 * axe) * ck(0, 1.3 + 0.2 * ceramic);
    k.game_m = (1.0 + 1.4 * hunt_q + 0.15 * cord) * (1.0 + 0.35 * cut + 0.05 * hammer) * ck(1, 1.25);
    k.fish_m = (1.0 + 0.8 * hunt_q + 0.5 * vq + 0.3 * point + 0.2 * cord) * ck(2, 1.2);
    k.cook_m = 1.0 + 0.03 * kiln;
    let gq = q(k.garment, &garment_q);
    k.protect = (0.35 * fire + 0.5 * gq + 0.05 * kiln + 0.05 * axe * fire).min(0.92);
    k
}
