//! Materials, forms and items. This file is the world's fixed "chemistry": every material has
//! physical properties, and every useful quality of an artefact is computed from the properties
//! of the material it is made of. Nothing here says what a band can do or in which order: that is
//! decided by `physics::apply` (what happens when a primitive action is applied to things) and by
//! which materials a band can reach.
use serde::{Deserialize, Serialize};
#[cfg(all(test, feature = "durability-v2"))]
use std::cell::Cell;

pub const NMAT: usize = 29;
/// Materials that can be gathered from the world (indices below this). The rest only exist as
/// products of physics (charcoal, ceramic, metals).
pub const NRAW: usize = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Mat {
    Wood = 0,
    Stone,
    Flint,
    Obsidian,
    Clay,
    CopperOre,
    TinOre,
    Fibre,
    Hide,
    Bone,
    Meat,
    Plants,
    Fish,
    Charcoal,
    Ceramic,
    Copper,
    Tin,
    Bronze,
    // v5 (farming stage): appended so every older index is unchanged
    /// threshed seed of a cultivated cereal: food that keeps; also the carried crop strain
    Grain,
    /// bast fibre from a cultivated flax-like plant
    Flax,
    /// fleece from tamed herd animals
    Wool,
    /// bog / laterite iron ore (limonite, haematite)
    IronOre,
    /// bloomery iron
    Iron,
    // v8 (technologies for activities): appended so every older index is unchanged
    /// sea water or the brine of a dry salt pan: only where it lies, never carried
    Brine,
    /// salt boiled out of brine: keeps meat and fish
    Salt,
    // inventions step 1: appended so every older index is unchanged
    /// pitch (wood tar): resinous wood heated slowly in a closed pot, short of air; caulks
    /// hulls, glues hafts, seals jars. Made where it is used, never carried
    Pitch,
    // inventions step 7: appended so every older index is unchanged
    /// steel: wrought iron soaked in a blown charcoal fire until it takes up carbon, then
    /// quenched in water. Harder and keener than iron, and as tough
    Steel,
    // inventions step 11: appended so every older index is unchanged
    /// native gold, picked from quartz veins or panned from river gravel: soft, never tarnishes.
    /// Good for nothing but show, and (step 11b) only made into a ring by melting it in a
    /// crucible and pouring it into a mould
    Gold,
    // inventions step 12: appended so every older index is unchanged
    /// resinous wood: the resin-soaked knots, stumps and bark of pine and birch, found in the
    /// conifer and birch woods of cool lands and hills. Heated in a closed pot it sweats out
    /// pitch (wood tar); ordinary wood does not
    ResinWood,
}
pub const ALL_MATS: [Mat; NMAT] = [
    Mat::Wood,
    Mat::Stone,
    Mat::Flint,
    Mat::Obsidian,
    Mat::Clay,
    Mat::CopperOre,
    Mat::TinOre,
    Mat::Fibre,
    Mat::Hide,
    Mat::Bone,
    Mat::Meat,
    Mat::Plants,
    Mat::Fish,
    Mat::Charcoal,
    Mat::Ceramic,
    Mat::Copper,
    Mat::Tin,
    Mat::Bronze,
    Mat::Grain,
    Mat::Flax,
    Mat::Wool,
    Mat::IronOre,
    Mat::Iron,
    Mat::Brine,
    Mat::Salt,
    Mat::Pitch,
    Mat::Steel,
    Mat::Gold,
    Mat::ResinWood,
];

/// Physical properties, on rough 0..10 scales unless noted. Values are loosely based on real
/// materials (Mohs hardness, fracture behaviour, melting / reduction temperatures).
#[derive(Clone, Copy, Debug)]
pub struct Props {
    pub name: &'static str,
    /// scratch hardness (Mohs-like)
    pub hard: f32,
    /// resistance to breaking / chipping
    pub tough: f32,
    /// conchoidal fracture: how well it flakes into sharp edges when struck
    pub fracture: f32,
    /// how sharp an edge of it can be (edge acuity)
    pub sharp: f32,
    /// ease of carving / scraping with an edge
    pub workable: f32,
    /// plasticity when wet (clay)
    pub plastic: f32,
    /// fibrous / flexible (can be twisted or cut into cord)
    pub fibrous: f32,
    /// can be hammered into shape without breaking (metals)
    pub malleable: f32,
    /// fire temperature it can sustain as fuel (deg C, 0 = not a fuel)
    pub fuel_t: f32,
    /// food energy (0 = inedible)
    pub food: f32,
    /// heating in a reducing (charcoal, enclosed) fire at >= this temperature yields `reduces_to`
    pub reduce_t: f32,
    pub reduces_to: Option<Mat>,
    /// heating at >= this temperature turns it into `fires_to` (clay -> ceramic)
    pub fire_t: f32,
    pub fires_to: Option<Mat>,
    /// melting point (deg C, 0 = does not melt usefully)
    pub melt: f32,
    /// resists heat / holds heat in (kiln lining)
    pub refractory: f32,
    /// buoyancy: floats and carries weight (a log, a bundle of reeds)
    pub float: f32,
    /// watertight as a sheet (can be stretched over a frame)
    pub seal: f32,
    /// (inventions step 11) worth for show beyond the work put in: rare, bright and lasting
    pub prized: f32,
    /// (inventions step 12) woody: long straight grain that can be carved into a shaft
    pub woody: f32,
    /// (inventions step 12) resinous: gives pitch when heated in a closed pot
    pub resin: f32,
}

const fn p(name: &'static str) -> Props {
    Props {
        name,
        hard: 0.0,
        tough: 0.0,
        fracture: 0.0,
        sharp: 0.0,
        workable: 0.0,
        plastic: 0.0,
        fibrous: 0.0,
        malleable: 0.0,
        fuel_t: 0.0,
        food: 0.0,
        reduce_t: 0.0,
        reduces_to: None,
        fire_t: 0.0,
        fires_to: None,
        melt: 0.0,
        refractory: 0.0,
        float: 0.0,
        seal: 0.0,
        prized: 0.0,
        woody: 0.0,
        resin: 0.0,
    }
}

pub static PROPS: [Props; NMAT] = [
    Props { hard: 2.0, tough: 6.0, sharp: 2.0, workable: 7.0, fibrous: 4.0, fuel_t: 650.0, float: 7.0, woody: 8.0, ..p("wood") },
    Props { hard: 6.0, tough: 7.0, fracture: 3.5, sharp: 3.0, workable: 1.0, refractory: 4.0, ..p("stone") },
    Props { hard: 7.0, tough: 3.5, fracture: 9.0, sharp: 8.0, workable: 0.5, refractory: 3.0, ..p("flint") },
    Props { hard: 5.5, tough: 1.5, fracture: 10.0, sharp: 10.0, workable: 0.5, ..p("obsidian") },
    Props { hard: 1.0, tough: 1.0, plastic: 9.0, workable: 8.0, fire_t: 600.0, fires_to: Some(Mat::Ceramic), refractory: 8.0, ..p("clay") },
    Props { hard: 3.8, tough: 3.0, fracture: 1.0, reduce_t: 1150.0, reduces_to: Some(Mat::Copper), refractory: 2.0, ..p("copper ore") },
    Props { hard: 6.5, tough: 4.0, fracture: 1.0, reduce_t: 1050.0, reduces_to: Some(Mat::Tin), refractory: 2.0, ..p("tin ore") },
    Props { hard: 0.5, tough: 3.0, workable: 6.0, fibrous: 9.0, fuel_t: 400.0, float: 6.0, ..p("plant fibre") },
    Props { hard: 1.0, tough: 6.0, workable: 6.0, fibrous: 6.0, food: 0.5, seal: 7.0, ..p("hide") },
    Props { hard: 4.0, tough: 5.0, fracture: 2.5, sharp: 4.5, workable: 5.0, ..p("bone") },
    Props { hard: 0.2, tough: 1.0, food: 9.0, ..p("meat") },
    Props { hard: 0.5, tough: 1.0, food: 6.0, fibrous: 2.0, ..p("wild plants") },
    Props { hard: 0.2, tough: 1.0, food: 8.0, ..p("fish") },
    Props { hard: 1.0, tough: 1.0, fuel_t: 950.0, ..p("charcoal") },
    Props { hard: 5.5, tough: 2.0, sharp: 1.0, refractory: 9.0, ..p("ceramic") },
    Props { hard: 3.0, tough: 9.0, sharp: 6.0, malleable: 9.0, melt: 1085.0, ..p("copper") },
    Props { hard: 1.5, tough: 5.0, sharp: 1.5, malleable: 8.0, melt: 232.0, ..p("tin") },
    Props { hard: 5.0, tough: 9.0, sharp: 7.5, malleable: 6.0, melt: 950.0, ..p("bronze") },
    Props { hard: 1.0, tough: 1.0, food: 7.0, ..p("grain") },
    Props { hard: 0.5, tough: 4.5, workable: 6.0, fibrous: 9.5, ..p("flax") },
    Props { hard: 0.3, tough: 4.0, workable: 6.0, fibrous: 8.0, seal: 2.0, ..p("wool") },
    Props { hard: 5.0, tough: 4.0, fracture: 1.0, reduce_t: 1400.0, reduces_to: Some(Mat::Iron), refractory: 2.0, ..p("iron ore") },
    Props { hard: 5.5, tough: 9.0, sharp: 7.0, malleable: 6.0, melt: 1538.0, ..p("iron") },
    Props { hard: 0.0, tough: 0.0, ..p("brine") },
    Props { hard: 2.5, tough: 1.0, ..p("salt") },
    Props { hard: 0.5, tough: 2.0, seal: 9.0, ..p("pitch") },
    // worked hot, so a hammerstone (hard 6) still shapes it; quench-hardened, it takes and
    // keeps a keener edge than iron (cut 0.78 against iron's 0.64 and bronze's 0.65)
    Props { hard: 6.0, tough: 9.0, sharp: 9.0, malleable: 5.5, melt: 1450.0, ..p("steel") },
    // soft and too heavy-and-blunt for any tool; hammered cold into beads, sheet and wire
    Props { hard: 2.5, tough: 5.0, sharp: 1.0, malleable: 10.0, melt: 1064.0, prized: 1.0, ..p("gold") },
    // knotty, gnarled and sticky: no good for a shaft (not `woody`), burns as wood does; its
    // only use is pitch
    Props { hard: 2.0, tough: 4.0, workable: 3.0, fuel_t: 650.0, float: 6.0, resin: 9.0, ..p("resinous wood") },
];

impl Mat {
    #[inline]
    pub fn props(self) -> &'static Props {
        &PROPS[self as usize]
    }
    pub fn from_u8(x: u8) -> Mat {
        ALL_MATS[(x as usize).min(NMAT - 1)]
    }
    pub fn name(self) -> &'static str {
        self.props().name
    }
    pub fn is_food(self) -> bool {
        self.props().food >= 5.0
    }
    pub fn is_metal(self) -> bool {
        matches!(self, Mat::Copper | Mat::Tin | Mat::Bronze | Mat::Iron | Mat::Steel)
    }
}

pub const NFORM: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Form {
    /// the material as found (or a smelted lump / ingot for metals)
    Raw = 0,
    /// a hand-sized pounding stone
    Hammer,
    /// a flake / blade / knife: something that cuts
    Edge,
    /// an awl, borer or projectile point
    Point,
    /// a straight handle / stick
    Shaft,
    /// string, thong, binding
    Cord,
    /// a container (basket, bag, pot)
    Vessel,
    /// a point or sharpened end on a long shaft
    Spear,
    /// a heavy edge hafted on a handle
    Axe,
    /// sewn clothing
    Garment,
    /// cooked food
    Cooked,
    /// a fire fuelled by the material
    Fire,
    /// an enclosed fire chamber lined with the material
    Kiln,
    /// something that floats and carries people (dugout, skin boat, reed bundle boat)
    Boat,
    /// knotted mesh for catching fish and birds
    Net,
    /// a snare: a cord noose on a springy stick
    Trap,
    /// a bag worked on a stick to force air into a fire
    Bellows,
    /// a spongy, slag-filled lump of metal straight from the furnace, useless until forged
    Bloom,
    // v8: appended so every older item id is unchanged
    /// a sprung stave with a cord that throws a small point (material = the string)
    Bow,
    /// hide stretched on a frame of sticks, held up against blows (material = the cover)
    Shield,
    /// a float on booms lashed beside a hull, so it does not capsize (material = the hull)
    Outrigger,
    /// cloth on a mast (material = the cloth)
    Sail,
    /// a large hull of split planks, sewn or pegged (material = the timber)
    Plank,
    /// a pack frame or carrying basket on a stick (material = the container)
    Frame,
    /// a bead, pendant or ring, kept for show and given away
    Ornament,
    // inventions step 1: appended so every older item id is unchanged
    /// a fired pot sealed with pitch: keeps food and water (material = the pot)
    Jar,
    // inventions step 5
    /// a plank hull caulked with pitch and fastened with metal nails or rivets: a ship that
    /// carries a big party and its stores (material = the hull's timber)
    Ship,
    // inventions step 8
    /// a counting mark: notches cut in a stick or bone (a tally), or a clay token pressed with
    /// a point, perhaps fired hard (material = what is marked)
    Mark,
    // inventions step 11b
    /// a hollow carved in a slab of clay (and fired hard), into which molten metal is poured
    /// (material = the mould's)
    Mould,
    /// a fired-clay crucible charged with metal for melting (material = the metal)
    Crucible,
    // inventions step 12
    /// a woven length of cloth (material = the yarn's fibre: linen, wool)
    Cloth,
    /// a simple warp-weighted loom: an upright frame of poles, the warp hung from it and kept
    /// taut by stone weights (material = the frame's wood)
    Loom,
}
pub const ALL_FORMS: [Form; NFORM] = [
    Form::Raw,
    Form::Hammer,
    Form::Edge,
    Form::Point,
    Form::Shaft,
    Form::Cord,
    Form::Vessel,
    Form::Spear,
    Form::Axe,
    Form::Garment,
    Form::Cooked,
    Form::Fire,
    Form::Kiln,
    Form::Boat,
    Form::Net,
    Form::Trap,
    Form::Bellows,
    Form::Bloom,
    Form::Bow,
    Form::Shield,
    Form::Outrigger,
    Form::Sail,
    Form::Plank,
    Form::Frame,
    Form::Ornament,
    Form::Jar,
    Form::Ship,
    Form::Mark,
    Form::Mould,
    Form::Crucible,
    Form::Cloth,
    Form::Loom,
];

/// An item = (form, material), packed into a u16. `NONE` = empty slot / bare hands.
pub type Item = u16;
pub const NONE: Item = u16::MAX;
pub const NITEMS: usize = NFORM * 32;

#[inline]
pub const fn item(f: Form, m: Mat) -> Item {
    (f as u16) * 32 + m as u16
}
#[inline]
pub fn form(i: Item) -> Form {
    ALL_FORMS[((i / 32) as usize).min(NFORM - 1)]
}
#[inline]
pub fn mat(i: Item) -> Mat {
    Mat::from_u8((i % 32) as u8)
}
pub fn valid_item(i: Item) -> bool {
    i != NONE && ((i / 32) as usize) < NFORM && ((i % 32) as usize) < NMAT
}

pub fn item_name(i: Item) -> String {
    if i == NONE {
        return "bare hands".into();
    }
    let m = mat(i);
    let n = m.name();
    match form(i) {
        Form::Raw => match m {
            Mat::Copper | Mat::Tin | Mat::Bronze => format!("{n} (metal)"),
            Mat::Iron => "wrought iron".into(),
            Mat::Steel => "steel bar".into(),
            _ => n.to_string(),
        },
        Form::Hammer => format!("{n} hammerstone"),
        Form::Edge => match m {
            Mat::Copper | Mat::Tin | Mat::Bronze | Mat::Iron | Mat::Steel => format!("{n} knife"),
            _ => format!("{n} blade"),
        },
        Form::Point => format!("{n} point"),
        Form::Shaft => format!("{n} shaft"),
        Form::Cord => match m {
            Mat::Hide => "hide thong".into(),
            _ => format!("{n} cord"),
        },
        Form::Vessel => match m {
            Mat::Fibre => "woven basket".into(),
            Mat::Flax => "flax basket".into(),
            Mat::Wool => "woollen bag".into(),
            Mat::Hide => "hide bag".into(),
            Mat::Clay => "unfired clay pot".into(),
            Mat::Ceramic => "fired pottery".into(),
            _ => format!("{n} vessel"),
        },
        Form::Spear => match m {
            Mat::Wood => "sharpened-stick spear".into(),
            _ => format!("{n}-tipped spear"),
        },
        Form::Axe => format!("{n} axe"),
        Form::Garment => match m {
            Mat::Flax => "linen clothing".into(),
            Mat::Wool => "woollen clothing".into(),
            _ => format!("{n} clothing"),
        },
        Form::Cooked => format!("cooked {n}"),
        Form::Fire => match m {
            Mat::Wood => "fire".into(),
            _ => format!("{n} fire"),
        },
        Form::Kiln => match m {
            Mat::Ceramic => "sherd-lined kiln".into(),
            _ => format!("{n}-lined kiln"),
        },
        Form::Boat => match m {
            Mat::Wood => "dugout canoe".into(),
            Mat::Hide => "skin boat".into(),
            Mat::Fibre => "reed bundle boat".into(),
            _ => format!("{n} raft"),
        },
        Form::Net => format!("{n} net"),
        Form::Trap => match m {
            Mat::Hide => "rawhide snare".into(),
            _ => format!("{n} snare"),
        },
        Form::Bellows => format!("{n} bellows"),
        Form::Bloom => format!("{n} bloom"),
        Form::Bow => match m {
            Mat::Hide => "rawhide-strung bow".into(),
            _ => format!("bow ({n} string)"),
        },
        Form::Shield => format!("{n} shield"),
        // every (form, material) pair has its own name, so reports keyed by name never merge two
        // items (the material of an outrigger is its hull's; of a sail, its cloth)
        Form::Outrigger => match m {
            Mat::Wood => "outrigger canoe".into(),
            Mat::Hide => "outrigger skin boat".into(),
            Mat::Fibre => "outrigger reed boat".into(),
            _ => format!("{n} outrigger"),
        },
        Form::Sail => match m {
            Mat::Flax => "linen sail".into(),
            Mat::Wool => "woollen sail".into(),
            Mat::Fibre => "woven mat sail".into(),
            _ => format!("{n} sail"),
        },
        Form::Plank => match m {
            Mat::Wood => "plank boat".into(),
            _ => format!("{n} plank boat"),
        },
        Form::Frame => format!("carrying frame ({n})"),
        Form::Ornament => match m {
            Mat::Bone => "bone beads".into(),
            Mat::Copper | Mat::Tin | Mat::Bronze | Mat::Iron | Mat::Steel | Mat::Gold => format!("{n} ring"),
            _ => format!("{n} pendant"),
        },
        Form::Jar => match m {
            Mat::Ceramic => "sealed jar".into(),
            // (inventions step 10) grain and water left to work in a sealed jar
            Mat::Grain => "ale".into(),
            _ => format!("sealed {n} jar"),
        },
        Form::Ship => match m {
            Mat::Wood => "nailed plank ship".into(),
            _ => format!("{n} ship"),
        },
        Form::Mark => match m {
            Mat::Wood => "tally stick".into(),
            Mat::Bone => "notched bone tally".into(),
            Mat::Clay => "clay token".into(),
            Mat::Ceramic => "fired clay token".into(),
            _ => format!("{n} tally"),
        },
        Form::Mould => match m {
            Mat::Clay => "clay mould".into(),
            Mat::Ceramic => "fired clay mould".into(),
            _ => format!("{n} mould"),
        },
        Form::Crucible => format!("crucible of {n}"),
        Form::Cloth => match m {
            Mat::Flax => "linen cloth".into(),
            Mat::Wool => "woollen cloth".into(),
            _ => format!("{n} cloth"),
        },
        Form::Loom => match m {
            Mat::Wood => "warp-weighted loom".into(),
            _ => format!("{n} loom"),
        },
    }
}

/// Fixed-size bitset over all item ids (room for 32 forms; never saved, so it can grow freely).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemSet(pub [u64; ITEMSET_WORDS]);
pub const ITEMSET_WORDS: usize = 16;
const _: () = assert!(NITEMS <= ITEMSET_WORDS * 64);
impl ItemSet {
    #[inline]
    pub fn has(&self, i: Item) -> bool {
        i == NONE || (self.0[(i / 64) as usize] >> (i % 64)) & 1 == 1
    }
    #[inline]
    pub fn add(&mut self, i: Item) {
        if i != NONE {
            self.0[(i / 64) as usize] |= 1 << (i % 64);
        }
    }
    pub fn count(&self) -> u32 {
        self.0.iter().map(|w| w.count_ones()).sum()
    }
    /// Members in ascending order (word by word, skipping empty words and zero bits).
    pub fn iter(&self) -> impl Iterator<Item = Item> + '_ {
        self.0.iter().enumerate().flat_map(|(w, &word)| {
            let mut x = word;
            std::iter::from_fn(move || {
                if x == 0 {
                    return None;
                }
                let t = x.trailing_zeros();
                x &= x - 1;
                Some((w as u32 * 64 + t) as Item)
            })
        })
    }
}

// ---------------------------------------------------------------- qualities from properties
/// Cutting quality of an edge / point / spear / axe head made of `m`, 0..~1.
pub fn cut_q(m: Mat) -> f32 {
    let p = m.props();
    ((0.6 * p.sharp + 0.4 * p.hard) / 10.0).clamp(0.0, 1.0)
}
/// Durability 0..1: tough or re-sharpenable (malleable) materials last longer.
///
/// Default build. The `/ 12` then `.clamp(0.05, 1.0)` puts copper, bronze and
/// iron on the same cap, and steel (less malleable) just under it, so a steel
/// edge wears faster than a copper one. `durability-v2` does not change this
/// function. The sim calls `score_durability` / `upkeep_wear` / `record_durability`,
/// which are this function when the feature is off.
pub fn durability(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + 0.5 * p.malleable) / 12.0).clamp(0.05, 1.0)
}

/// What is wearing. Proposal (`durability-v2`); the default build does not read it.
/// Malleability is not a bonus here. It is `repair_factor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WearClass {
    /// Knife, axe, adze, sickle, spear, point: dulling and chipping.
    Edge,
    /// Hammer, and a shield face: impact. Toughness dominates; a brittle face shatters.
    Impact,
    /// Pot, basket, bag, jar, frame, bellows.
    Vessel,
    /// Cord, net, snare, garment, bowstring, sail, cloth.
    Cordage,
    /// Boat, outrigger, plank hull, ship.
    Hull,
    /// Kiln lining, mould, and the clay body of a crucible.
    Thermal,
    /// A tally or token: weathering, not an edge.
    Record,
    /// Shaft, loom: it breaks, it does not dull.
    Structure,
    /// Raw stock, bloom, and forms whose upkeep is not this model (fire, cooked, ornament).
    Stock,
}

/// Which wear formula an item's form uses. Ornaments, fires and cooked meals never
/// reach it: `tool_upkeep` returns before the wear term.
pub fn wear_class(f: Form) -> WearClass {
    match f {
        Form::Edge | Form::Point | Form::Spear | Form::Axe => WearClass::Edge,
        Form::Hammer | Form::Shield => WearClass::Impact,
        Form::Vessel | Form::Jar | Form::Frame | Form::Bellows => WearClass::Vessel,
        Form::Cord | Form::Net | Form::Trap | Form::Garment | Form::Bow | Form::Sail | Form::Cloth => WearClass::Cordage,
        Form::Boat | Form::Outrigger | Form::Plank | Form::Ship => WearClass::Hull,
        Form::Kiln | Form::Mould | Form::Crucible => WearClass::Thermal,
        Form::Mark => WearClass::Record,
        Form::Shaft | Form::Loom => WearClass::Structure,
        Form::Raw | Form::Bloom | Form::Cooked | Form::Fire | Form::Ornament => WearClass::Stock,
    }
}

/// PROPOSAL (`durability-v2`). Working life of `m` used as `class`. Higher lasts
/// longer. No shared cap: a result is equal only when the properties this class
/// reads are equal. Inputs are the existing 0..10 props. Nothing here is saved.
///
/// An edge is not one of these lives. The feature-on sim prices an edge as
/// resharpen labour plus breakage (`split_edge_wear`). The `Edge` arm is not read.
pub fn class_durability(class: WearClass, m: Mat) -> f32 {
    let p = m.props();
    match class {
        WearClass::Edge => 0.0,
        WearClass::Impact => impact_life(p),
        WearClass::Vessel => vessel_life(p),
        WearClass::Cordage => cord_life(p),
        WearClass::Hull => hull_life(p),
        WearClass::Thermal => thermal_life(p),
        WearClass::Record => record_life(p),
        WearClass::Structure => structure_life(p),
        WearClass::Stock => p.tough / 12.0,
    }
}

/// Toughness left after the knap discount. Malleability is not in it. A high
/// `fracture` spends toughness; it is not a brittleness column, and it is only
/// applied here as "this edge was knapped".
fn chip_tough(p: &Props) -> f32 {
    p.tough * (1.0 - 0.50 * p.fracture / 10.0)
}

/// Which model the feature-on sim is scoring. `"v1"` when a test forces the old
/// formula. The default build always reports `"v1"`. The sim binary (not a test)
/// reports `"split"` and does not read an override.
pub fn durability_account_name() -> &'static str {
    #[cfg(not(feature = "durability-v2"))]
    {
        "v1"
    }
    #[cfg(all(feature = "durability-v2", not(test)))]
    {
        "split"
    }
    #[cfg(all(feature = "durability-v2", test))]
    {
        if dur_force_v1() { "v1" } else { "split" }
    }
}

// Test-only. The sim binary does not compile this, and the scoring functions
// do not read it unless this crate is built for tests.
#[cfg(all(test, feature = "durability-v2"))]
thread_local! {
    static DUR_V1: Cell<bool> = Cell::new(false);
}

/// Score today's formula inside a feature-on test. Not in the sim binary.
#[cfg(all(test, feature = "durability-v2"))]
pub fn with_v1_model<T>(f: impl FnOnce() -> T) -> T {
    DUR_V1.with(|v| {
        let old = v.replace(true);
        let out = f();
        v.set(old);
        out
    })
}

#[cfg(all(test, feature = "durability-v2"))]
#[inline]
pub(crate) fn dur_force_v1() -> bool {
    DUR_V1.with(|c| c.get())
}

/// Share of fresh keenness a short-lived edge still gets. Used only by the
/// point slot's fish term (`working_edge_q`), not by the split edge.
#[cfg(feature = "durability-v2")]
pub const EDGE_KEEP_FLOOR: f32 = 0.40;

/// Raw edge life behind `working_edge_q`. Not wear, and not the split cut.
#[cfg(feature = "durability-v2")]
fn edge_life(p: &Props) -> f32 {
    let keep = 0.65 * p.hard + 0.35 * p.sharp;
    let roll = 1.0 + 0.09 * p.malleable;
    let chip = chip_tough(p);
    (keep / roll) * chip / 45.0 + 0.004 * p.hard
}

#[cfg(feature = "durability-v2")]
fn raw_edge_life(m: Mat) -> f32 {
    edge_life(m.props())
}

/// Keenness discounted by raw edge life. The point slot's fish term still
/// reads this. Knife, hunt and the axe do not: they use the split cut.
#[cfg(feature = "durability-v2")]
pub fn working_edge_q(m: Mat) -> f32 {
    let life = raw_edge_life(m);
    cut_q(m) * (EDGE_KEEP_FLOOR + (1.0 - EDGE_KEEP_FLOOR) * life)
}

/// Hardness and acuity, 0..1. Malleability is not read. Neither is toughness.
fn keep_index(p: &Props) -> f32 {
    (0.65 * p.hard + 0.35 * p.sharp) / 10.0
}

/// How long a working edge stays keen, as a 0..1 index. The square is Archard-shaped:
/// the props hardness is a compressed rank, and a soft rank dulls much sooner than
/// a linear reading of it. Malleability does not extend this. It does not enter it.
pub fn edge_retention(m: Mat) -> f32 {
    let k = keep_index(m.props());
    k * k
}

/// Below this, resharpen labour would divide by zero. A material with no hardness
/// and no acuity (brine) is not a tool; the floor only keeps the arithmetic finite.
pub const SPLIT_INTERVAL_FLOOR: f32 = 0.02;

/// Seasons of keenness relative to a perfect keep, before the use rate.
/// Higher means you hone less often. Not compressed.
pub fn resharpen_interval(m: Mat) -> f32 {
    edge_retention(m).max(SPLIT_INTERVAL_FLOOR)
}

/// Share of toughness a chipped edge still contributes to the average cut.
/// 0 would zero a knapped edge's food; flint would then lose to soft copper.
pub const SPLIT_CHIP_FLOOR: f32 = 0.25;

/// Toughness after the knap discount, 0..about 1. Breakage reads `1 - this`.
/// Malleability is not in it.
pub fn edge_toughness(m: Mat) -> f32 {
    chip_tough(m.props()).max(0.0) / 10.0
}

/// Average cut between sharpenings. Fresh cut times how dull the edge gets
/// (retention squared) times how much of that edge survives chipping.
/// A soft edge spends the interval dull. A knapped edge spends it chipped.
/// Malleability does not raise this.
pub fn split_working_cut(m: Mat) -> f32 {
    let chip = edge_toughness(m);
    let body = SPLIT_CHIP_FLOOR + (1.0 - SPLIT_CHIP_FLOOR) * chip;
    cut_q(m) * edge_retention(m) * body
}

/// Wear-scale of one resharpen per season at interval 1 and cost 1.
/// `resharpen_labour = RESHARPEN_USE / interval * cost`. The existing upkeep
/// rate is the "uses". There is no per-band sharpening loop.
pub const RESHARPEN_USE: f32 = 0.07;

/// Wear-scale of replacing the tool once a season. Times the break rate.
/// Chain depth is the existing `(1 + 0.3 * depth)` outside this number, which
/// is the sim's stand-in for the cost of a new tool.
pub const BREAK_SCALE: f32 = 0.60;

/// Cost of restoring the edge, 0.5..1. A hone scales down with malleability
/// (`repair_factor`). A knapped edge is re-knapped, so the cost goes to 1
/// (a new edge) as `fracture` goes to 10. Malleability does not buy a longer life.
pub fn resharpen_cost(m: Mat) -> f32 {
    let p = m.props();
    let hone = 1.0 - (p.fracture / 10.0).clamp(0.0, 1.0);
    let cheap = repair_factor(WearClass::Edge, m);
    hone * cheap + (1.0 - hone) * 1.0
}

/// Resharpen labour in upkeep-wear units, for one season at the sim's use rate.
pub fn resharpen_labour(m: Mat) -> f32 {
    RESHARPEN_USE / resharpen_interval(m) * resharpen_cost(m)
}

/// Replacement labour in the same units. `1 - toughness`, times `BREAK_SCALE`.
pub fn break_labour(m: Mat) -> f32 {
    (1.0 - edge_toughness(m)).max(0.0) * BREAK_SCALE
}

/// Edge upkeep. Resharpen plus replacement. Not negative.
/// Non-edge forms do not use this; they use the class life times repair.
pub fn split_edge_wear(m: Mat) -> f32 {
    resharpen_labour(m) + break_labour(m)
}

/// Axe food term under the split account. Half the season's average cut, half
/// the chance the head is still in one piece. Same 0.5/0.5 shape as the
/// combined axe, so `+0.1 * axe` and `+0.2 * axe` still multiply a number on
/// that scale. Malleability is in neither half.
pub fn split_axe_score(m: Mat) -> f32 {
    0.5 * split_working_cut(m) + 0.5 * edge_toughness(m)
}

/// Keenness a food slot pays. Feature off: `cut_q`, the same bits as before.
/// Feature on: `split_working_cut`. A test can force the old model; the sim
/// binary does not branch.
#[inline(always)]
pub fn food_edge_q(m: Mat) -> f32 {
    #[cfg(not(feature = "durability-v2"))]
    {
        cut_q(m)
    }
    #[cfg(all(feature = "durability-v2", not(test)))]
    {
        split_working_cut(m)
    }
    #[cfg(all(feature = "durability-v2", test))]
    {
        if dur_force_v1() { cut_q(m) } else { split_working_cut(m) }
    }
}

/// The number an axe contributes to food and to the fight blend.
/// Feature off: `0.5 * cut_q + 0.5 * durability`, the same operations as the
/// inlined expression this replaced. Feature on: `split_axe_score`.
#[inline(always)]
pub fn axe_food_score(m: Mat) -> f32 {
    #[cfg(not(feature = "durability-v2"))]
    {
        0.5 * cut_q(m) + 0.5 * durability(m)
    }
    #[cfg(all(feature = "durability-v2", not(test)))]
    {
        split_axe_score(m)
    }
    #[cfg(all(feature = "durability-v2", test))]
    {
        if dur_force_v1() { 0.5 * cut_q(m) + 0.5 * durability(m) } else { split_axe_score(m) }
    }
}

/// A striking face. Toughness is the life; flaking shatters it. Hardness keeps
/// the face from dishing, malleability mushrooms it.
fn impact_life(p: &Props) -> f32 {
    let face = p.tough * (1.0 - 0.65 * p.fracture / 10.0);
    let dish = p.hard / (p.hard + 0.30 * p.malleable + 0.5);
    (face / 10.0) * (0.45 + 0.55 * dish)
}

/// A wall. Fibre is a basket (rot and fray, from toughness and fibrousness).
/// Anything else is a pot: it cracks (low toughness, flaking) and abrades.
fn vessel_life(p: &Props) -> f32 {
    let organic = p.fibrous / 10.0;
    let crack = (p.tough / 10.0) * (1.0 - 0.55 * p.fracture / 10.0);
    let abrasion = p.hard / (p.hard + 2.0);
    let pot = crack * (0.35 + 0.65 * abrasion);
    let fibre = (p.tough / 12.0) * (0.35 + 0.65 * organic);
    fibre * organic + pot * (1.0 - organic)
}

/// Cord, cloth, net, clothing, a bowstring. Fibre toughness. There is no rot
/// property; nothing here is immortal because tough and fibrous are not 10.
fn cord_life(p: &Props) -> f32 {
    (0.55 * p.tough + 0.45 * p.fibrous) / 12.0
}

/// A hull breaking up. Float and seal are `boat_q`, not life. Hardness separates
/// a dugout from a skin boat that the old formula tied (both tough 6).
fn hull_life(p: &Props) -> f32 {
    (p.tough / 10.0) * (0.55 + 0.45 * p.hard / (p.hard + 3.0))
}

/// Heat. A kiln or a mould uses its own refractory and toughness. A crucible's
/// material is the metal charge, whose refractory is 0; the body is fired clay
/// (refractory 9, tough 2), stressed by how hot that charge has to be.
fn thermal_life(p: &Props) -> f32 {
    let (refr, tough, shock) = if p.refractory > 0.0 {
        (p.refractory, p.tough, 1.0)
    } else if p.melt > 0.0 {
        (9.0, 2.0, 1.0 - 0.35 * (p.melt / 2000.0))
    } else {
        (0.0, p.tough, 1.0)
    };
    ((0.65 * refr + 0.35 * tough) / 12.0) * shock
}

/// Weathering of a mark. Not an edge. The ceramic +1 stays in the kit choice.
fn record_life(p: &Props) -> f32 {
    (0.55 * p.hard + 0.45 * p.tough) / 10.0
}

/// A shaft or a loom. Toughness, helped a little by hardness, hurt by flaking.
fn structure_life(p: &Props) -> f32 {
    let chip = p.tough * (1.0 - 0.40 * p.fracture / 10.0);
    (chip / 10.0) * (0.60 + 0.40 * p.hard / (p.hard + 2.0))
}

/// Labour per unit of wear. 1 when `malleable` is 0 (knap another edge, or
/// throw it out). Copper (9) is about 0.60; steel (5.5) about 0.75. A crucible's
/// charge is not the body that cracks, so it does not get the metal's discount.
/// Malleable is at most 10 on these props, so this stays above 0.5 with no clamp.
pub fn repair_factor(class: WearClass, m: Mat) -> f32 {
    let p = m.props();
    if class == WearClass::Thermal && p.refractory <= 0.0 {
        return 1.0;
    }
    1.0 - 0.045 * p.malleable
}

/// The `(1.15 - durability)` piece of upkeep. Feature off: that expression.
/// Feature on, and the form is an edge: `split_edge_wear` (resharpen plus
/// replacement). Other forms: `(1.15 - class life) * repair_factor`. No per-use loop.
#[inline]
pub fn upkeep_wear(it: Item) -> f32 {
    #[cfg(not(feature = "durability-v2"))]
    {
        1.15 - durability(mat(it))
    }
    #[cfg(feature = "durability-v2")]
    {
        #[cfg(test)]
        if dur_force_v1() {
            return 1.15 - durability(mat(it));
        }
        let class = wear_class(form(it));
        let m = mat(it);
        if class == WearClass::Edge {
            split_edge_wear(m)
        } else {
            (1.15 - class_durability(class, m)) * repair_factor(class, m)
        }
    }
}

/// Durability mixed into an edge's fight score (`edge_q`, and the old axe half).
/// Feature off: `durability`. Feature on: toughness after the knap discount.
/// Malleability is not added.
#[inline]
pub fn score_durability(m: Mat) -> f32 {
    #[cfg(not(feature = "durability-v2"))]
    {
        durability(m)
    }
    #[cfg(all(feature = "durability-v2", not(test)))]
    {
        edge_toughness(m)
    }
    #[cfg(all(feature = "durability-v2", test))]
    {
        if dur_force_v1() { durability(m) } else { edge_toughness(m) }
    }
}

/// Durability the mark slot ranks by. Feature off: `durability`.
#[inline]
pub fn record_durability(m: Mat) -> f32 {
    #[cfg(not(feature = "durability-v2"))]
    {
        durability(m)
    }
    #[cfg(feature = "durability-v2")]
    {
        class_durability(WearClass::Record, m)
    }
}
/// Container quality (storage / carrying) by material.
pub fn vessel_q(m: Mat) -> f32 {
    match m {
        Mat::Ceramic => 1.0,
        Mat::Hide => 0.6,
        Mat::Fibre | Mat::Flax => 0.5,
        Mat::Wool => 0.4,
        Mat::Clay => 0.25,
        _ => 0.3,
    }
}
/// Seaworthiness of a boat: a sturdy hull that floats (or a watertight skin on a frame).
pub fn boat_q(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + p.float.max(p.seal)) / 14.0).clamp(0.0, 1.0)
}
/// Quality of a net or snare: strong, flexible cord.
pub fn mesh_q(m: Mat) -> f32 {
    let p = m.props();
    ((p.tough + p.fibrous) / 15.0).clamp(0.0, 1.0)
}
/// Warmth of clothing.
pub fn garment_q(m: Mat) -> f32 {
    if m == Mat::Flax {
        return 0.55; // linen: strong and light, but not warm
    }
    let p = m.props();
    ((p.tough + p.fibrous) / 12.0).clamp(0.0, 1.0)
}
/// Temperature a fire of `fuel` reaches in the open, plus the gain of a kiln lined with `lining`.
pub fn fire_temp(fuel: Mat, lining: Option<Mat>) -> f32 {
    let base = fuel.props().fuel_t;
    let k = lining
        .map(|l| 40.0 * l.props().refractory)
        .unwrap_or(0.0);
    base + k
}
