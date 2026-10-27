//! Bands: the unit of simulation (a co-residing group of households). Knowledge is per band.
use crate::knowledge::{Known, Toolkit};
use crate::materials::*;
use serde::{Deserialize, Serialize};

/// shape, combine, heat (indexed by `Action as usize`)
pub const CRAFT_ACTS: usize = 3;
pub const CRAFT_LEN: usize = NMAT + CRAFT_ACTS;

/// Bring a craft vector ([material leanings.., action leanings x CRAFT_ACTS]) up to the current
/// length: leanings for materials added since it was made go in neutral (1.0) before the action
/// leanings, so every leaning keeps its meaning.
pub fn fit_craft(c: &mut Vec<f32>) {
    if c.len() >= CRAFT_LEN {
        return;
    }
    if c.len() < CRAFT_ACTS {
        c.resize(CRAFT_LEN, 1.0);
        return;
    }
    let at = c.len() - CRAFT_ACTS;
    let n = CRAFT_LEN - c.len();
    c.splice(at..at, std::iter::repeat(1.0).take(n));
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Band {
    pub id: u32,
    /// id of the founding band this one descends from
    pub lineage: u32,
    pub parent: u32,
    pub gen: u32,
    /// earlier ancestors: grandparent, great-grandparent, great-great-grandparent (0 = unknown)
    pub anc: [u32; 3],
    /// (v9) where the band's people come from: shares of founding lineages (summing to 1, at
    /// most `ANCESTRY_SLOTS`, largest first). Empty = all of its own `lineage` (older saves, and
    /// founders). `lineage` stays the inherited "surname"; marriage and captives mix this.
    pub ancestry: Vec<(u32, f32)>,
    pub x: u16,
    pub y: u16,
    pub pop: u32,
    /// stored food (person-seasons)
    pub store: f32,
    /// heritable inclination to tinker (drifts at each split)
    pub curiosity: f32,
    /// (v9) heritable boldness on a log scale (0 = ordinary, e^ambition is the multiplier): bold
    /// bands push a boat past the safe range, press a quarrel harder, and chafe sooner in a
    /// crowded band. Drifts at splits like curiosity; clamped to +-`AMBITION_MAX`.
    pub ambition: f32,
    /// (v9) friction within the band, 0..~1.5: follows hunger, crowding and size (more so for
    /// the ambitious), and eases in good times. High dissent splits a band before it is full,
    /// and the bolder, more curious people leave, and go farther.
    pub dissent: f32,
    /// (v9) standing in others' eyes, 0..~3: earned by surplus, fine work, gifts of food and
    /// voyages to new land, and fading (`PRESTIGE_FADE`). Others learn from, and marry into,
    /// prestigious bands more readily. New bands start with none.
    pub prestige: f32,
    /// keeps a captured fire alight
    pub flame: bool,
    #[serde(with = "crate::codec::column")]
    pub known: Vec<Known>,
    /// carried raw materials (0..~1.5 per material)
    pub stock: Vec<f32>,
    pub born: u32,
    /// where each carried material was first gathered (cell; u32::MAX = unknown)
    pub origin: Vec<u32>,
    /// exchange partners: bands this one trades with and visits
    #[serde(with = "crate::codec::column")]
    pub partners: Vec<Partner>,
    /// craft tradition: how readily this band turns to each material (NMAT entries) and to each
    /// way of working (CRAFT_ACTS entries: shape, combine, heat). 1 = no leaning. Inherited with
    /// drift at splits, reinforced by what the band finds or learns, fades back slowly. (v3)
    #[serde(with = "crate::codec::column")]
    pub craft: Vec<f32>,
    /// a far shore this band has seen, heard of, or reached (cell); None = none known (v3)
    pub far_shore: Option<u32>,
    /// (v9) the year that far shore was last seen or confirmed. Passed on with the rumour, so
    /// news ages as it travels; a shore nobody confirms for `RUMOUR_YEARS` is forgotten.
    pub far_shore_year: u16,
    /// bands this one bears a grudge against (people killed, stores taken); fades over the
    /// years and is passed on, weaker, to daughter bands (v4)
    pub grudges: Vec<Grudge>,
    /// recent pressure from stronger neighbours, 0..1: being driven off makes a band readier
    /// to move on, or to put to sea (v4)
    pub pressed: f32,
    // ---- farming stage (v5)
    /// know-how of sowing and tending fields, 0..1 (0 = forages only). Found where a band keeps
    /// coming back to ground it already tends, taught to neighbours, fades when not practised.
    pub sow: f32,
    /// how far the crop this band carries and sows has been domesticated, 0..1 (seed grain is
    /// in `stock[Grain]`; lose the seed and the strain is gone)
    pub strain: f32,
    /// tamed herd animals per person (0 = none)
    pub herd: f32,
    /// how tame / bred-for-use the herd is, 0..1
    pub tame: f32,
    /// how long the band has stayed put, 0..1 (sedentism)
    pub settled: f32,
    /// current outbreak of crowd disease, 0..1
    pub sick: f32,
    /// resistance left by past outbreaks, 0..1 (fades as new generations are born)
    pub immune: f32,
    /// seafaring experience, 0..1: built up by fishing from boats and by crossings, inherited by
    /// daughter bands, fades ashore. Longer range, fewer losses, scouting trips (v5)
    pub seacraft: f32,
    /// (v8) palisade or earthwork round the camp, 0..1: raised by a settled band that has been
    /// attacked, with an axe and a digging stick; needs work each year and rots when neglected
    pub wall: f32,
    /// the cell the wall stands in (it stays behind when the band moves)
    pub wall_cell: u32,
    /// (v8) herbal lore, 0..1: what plants ease the sick, learned under disease and taught on
    /// meeting
    pub herbal: f32,
    /// (v8) wariness of sick neighbours after an outbreak, 0..1, fading
    pub wary: f32,
    /// (v8) riding, 0..1: horse-like herd animals broken to carry people, on open land; lost
    /// with the herd
    pub ride: f32,
    /// running counters
    pub inventions: u32,
    pub learned: u32,
    pub forgotten: u32,
    // ---- derived each season; the parts the next season reads before recomputing them are
    // saved as `carry` (v5, additive: absent in older saves and beta saves, which start fresh)
    #[serde(rename = "carry", serialize_with = "ser_carry", deserialize_with = "de_carry")]
    pub d: Derived,
    // ---- inventions (additive: appended after `carry` so older band rows load with defaults)
    /// (inventions step 3) pilotage, 0..1: reading the signs of land out of sight of it (homing
    /// birds, clouds piled over islands, swells bent by land, drifting weed). Learned out of
    /// sight of land (long voyages, scouting, coming home from a drift), taught to seafaring
    /// neighbours, inherited by daughters, fades when not used.
    pub pilot: f32,
    /// (inventions step 4) sky lore, 0..1: star paths and the sun's rising points through the
    /// year. Farmers learn it timing their sowing, pilots sailing at night; taught to neighbours
    /// who farm or pilot, inherited by daughters, fades when not used. Helps sowing a little;
    /// at sea it steadies an aimed heading, but only as far as the crew has pilotage to use it.
    pub sky: f32,
    /// (inventions step 8) records, 0..1: keeping count with tallies and tokens. Grows while a
    /// band has a counting mark and something to keep track of (a big store, three or more
    /// partners, a herd, or tribute owed or owing); taught to neighbours who keep accounts,
    /// inherited by daughters, fades when not used. Stores are kept a little better, recipes
    /// and trade ties are forgotten more slowly.
    pub records: f32,
    /// (inventions step 8) tribute owed or owing, 0..1: set by a tribute paid either way and
    /// fading year by year (one of the things worth keeping count of)
    pub owed: f32,
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
    /// (v9) marriage ties, 0..1: grow with each exchange of spouses, fade slowly (see
    /// `MARRIAGE_FADE`), and cool quarrels between the two bands
    pub married: f32,
}

/// A remembered wrong done by another band (and, more weakly, by its whole lineage).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Grudge {
    pub id: u32,
    /// founding lineage of the wrongdoer (kept for old saves; no longer used)
    pub lineage: u32,
    pub w: f32,
    /// the wrongdoer's parent band: the grudge spills onto its close family
    pub par: u32,
}

pub const MAX_GRUDGES: usize = 6;
/// years over which remembered kinship between split bands fades (e-folding)
pub const KIN_FADE_YEARS: f32 = 150.0;

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
    // v8 slots (technologies for activities); their effects are wired in step by step
    pub bow: Item,
    pub shield: Item,
    pub outrigger: Item,
    pub sail: Item,
    pub plank: Item,
    pub frame: Item,
    pub ornament: Item,
    // inventions step 1
    /// pitch the band can make (caulking, glue, seals)
    pub pitch: Item,
    /// sealed jars
    pub jar: Item,
    // inventions step 5
    /// a nailed, caulked plank ship
    pub ship: Item,
    /// (v9 refinement) quality of the recipe that makes the ship (0..1): a well-built ship
    /// keeps its crew and stores longer at sea
    pub ship_q: f32,
    /// (inventions step 8) a counting mark (tally or token)
    pub mark: Item,
    /// (inventions step 10) ale: grain left to work in a sealed jar (`item(Jar, Grain)`)
    pub ale: Item,
    /// cooking known for plants / meat / fish
    pub cooked: [Item; 3],
    /// cooking known for grain (v5)
    pub cooked_grain: Item,
    /// gathering efficiency per resource
    pub plants_m: f32,
    pub game_m: f32,
    pub fish_m: f32,
    pub small_m: f32,
    /// field work efficiency (digging stick, sickle, axe for clearing)
    pub farm_m: f32,
    /// food value per unit harvested [plants, game, fish, small game, crops]: cooking makes more of the
    /// same harvest digestible (gelatinised starch, softened meat), so it adds calories, not speed
    pub digest: [f32; 5],
    /// smoking / drying over a fire slows spoilage of stored meat and fish
    pub smoke: bool,
    /// seaworthiness of the best boat (0 = none)
    pub boat_q: f32,
    pub protect: f32,
    /// (v9) quality of the band's best ornament recipe (fine work, for prestige)
    pub fine: f32,
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
            bow: NONE,
            shield: NONE,
            outrigger: NONE,
            sail: NONE,
            plank: NONE,
            frame: NONE,
            ornament: NONE,
            pitch: NONE,
            jar: NONE,
            ship: NONE,
            ship_q: 0.0,
            mark: NONE,
            ale: NONE,
            cooked: [NONE; 3],
            cooked_grain: NONE,
            plants_m: 1.0,
            game_m: 1.0,
            fish_m: 1.0,
            small_m: 0.35,
            farm_m: 0.5,
            digest: [1.0; 5],
            smoke: false,
            boat_q: 0.0,
            protect: 0.0,
            fine: 0.0,
        }
    }
}
impl Kit {
    pub fn used_items(&self) -> [Item; 30] {
        [
            self.hunt, self.cut, self.dig, self.vessel, self.axe, self.garment, self.fire, self.kiln,
            self.cord, self.hammer, self.point, self.cooked[0], self.cooked[1], self.cooked[2],
            self.boat, self.net, self.trap, self.cooked_grain, self.bow, self.shield, self.outrigger,
            self.sail, self.plank, self.frame, self.ornament, self.jar, self.pitch, self.ship, self.mark,
            self.ale,
        ]
    }
    /// mean food value per unit across the four wild food kinds (for display)
    pub fn digest_mean(&self) -> f32 {
        self.digest[..4].iter().sum::<f32>() / 4.0
    }
}

/// What of last season's `Derived` state carries into the next season (food, stress, harvest
/// mix, trade demand): saved so a reloaded world runs on exactly as the original would.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Carry {
    food_ratio: f32,
    spare: f32,
    stress: f32,
    cold: f32,
    need: f32,
    upkeep: f32,
    food: f32,
    harvest: [f32; 5],
    farm_food: f32,
    farm_share: f32,
    experiments: u32,
    needed: u32,
    uses: u32,
    orn: u32,
    demand: Vec<f32>,
}

fn ser_carry<S: serde::Serializer>(d: &Derived, s: S) -> Result<S::Ok, S::Error> {
    Carry {
        food_ratio: d.food_ratio,
        spare: d.spare,
        stress: d.stress,
        cold: d.cold,
        need: d.need,
        upkeep: d.upkeep,
        food: d.food,
        harvest: d.harvest,
        farm_food: d.farm_food,
        farm_share: d.farm_share,
        experiments: d.experiments,
        needed: d.needed,
        uses: d.uses,
        orn: d.orn,
        demand: d.demand.to_vec(),
    }
    .serialize(s)
}

fn de_carry<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Derived, D::Error> {
    let c = Carry::deserialize(de)?;
    let mut d = Derived {
        food_ratio: c.food_ratio,
        spare: c.spare,
        stress: c.stress,
        cold: c.cold,
        need: c.need,
        upkeep: c.upkeep,
        food: c.food,
        harvest: c.harvest,
        farm_food: c.farm_food,
        farm_share: c.farm_share,
        experiments: c.experiments,
        needed: c.needed,
        uses: c.uses,
        orn: c.orn,
        ..Default::default()
    };
    for (i, v) in c.demand.into_iter().take(NMAT).enumerate() {
        d.demand[i] = v;
    }
    Ok(d)
}

#[derive(Clone, Debug, Default)]
pub struct Derived {
    pub tk: Toolkit,
    pub tk_key: Option<(ItemSet, bool, u32)>,
    /// the toolkit before the current one, with its key and kit
    pub tk_prev: Option<Box<((ItemSet, bool, u32), Toolkit, Kit)>>,
    pub known_ver: u32,
    /// (v9) recipe qualities changed since the kit was made: rebuild the kit (not the toolkit)
    pub fin_dirty: bool,
    /// the band's place in this season's harvest order (valid while `World::seen` uses it)
    pub hk: u32,
    pub kit: Kit,
    pub food_ratio: f32,
    pub spare: f32,
    pub stress: f32,
    pub cold: f32,
    pub need: f32,
    pub upkeep: f32,
    /// food eaten this season (person-seasons) and the harvest [plants, game, fish, small game]
    pub food: f32,
    pub harvest: [f32; 5],
    /// food from fields and herds this season, and its share of all food
    pub farm_food: f32,
    pub farm_share: f32,
    pub experiments: u32,
    /// raw materials (bitmask) that the tools in use need
    pub needed: u32,
    /// raw materials (bitmask) that appear in any recipe the band knows
    pub uses: u32,
    /// (v8) materials (bitmask) the band knows how to make ornaments of
    pub orn: u32,
    /// demand for each material heard from trade partners (lets goods pass down a chain)
    pub demand: [f32; NMAT],
    /// (cell, available items) when the knowledge map was last updated for this band: while
    /// neither changes there is nothing new to record
    pub kn_key: Option<(u32, ItemSet)>,
    /// (v9) the cell this band put to sea from this season, if it came here over open water
    /// (a voyage, a sea flight, a daughter's crossing or a drift); cleared at the start of each
    /// season, read by `World::track_landfall` to tell boat-borne landfalls from walking ones
    pub sailed_from: Option<u32>,
    /// (inventions step 0) with `sailed_from`: the open-water cells crossed and what the party
    /// had with it (`TECH_*` bits), for the landfall record and the crossing ledger
    pub sailed_sea: f32,
    pub sailed_tech: u16,
}

/// (inventions step 8) There is no longer a fixed number of partners: ties fade unless kept up
/// by visits, trade or marriage (`World` yearly fade, slowed by records).
/// (v9) Founding lineages kept in a band's ancestry (the smallest shares are dropped).
pub const ANCESTRY_SLOTS: usize = 8;

impl Band {
    /// Older saves carry stock / origin vectors for fewer materials: pad them.
    pub fn fit_vectors(&mut self) {
        if self.stock.len() < NMAT {
            self.stock.resize(NMAT, 0.0);
        }
        if self.origin.len() < NMAT {
            self.origin.resize(NMAT, u32::MAX);
        }
        fit_craft(&mut self.craft);
    }
    pub fn partner(&self, id: u32) -> Option<&Partner> {
        self.partners.iter().find(|p| p.id == id)
    }
    /// (v9) Ancestry shares, with the implicit "all own lineage" spelled out.
    pub fn ancestry(&self) -> Vec<(u32, f32)> {
        if self.ancestry.is_empty() {
            vec![(self.lineage, 1.0)]
        } else {
            self.ancestry.clone()
        }
    }

    /// (v9) A share `f` (0..1) of the band's people now have ancestry `other`.
    pub fn mix_ancestry(&mut self, other: &[(u32, f32)], f: f32) {
        let f = f.clamp(0.0, 1.0);
        if f <= 0.0 {
            return;
        }
        let mut v: Vec<(u32, f32)> = self.ancestry().into_iter().map(|(l, x)| (l, x * (1.0 - f))).collect();
        for &(l, x) in other {
            match v.iter_mut().find(|e| e.0 == l) {
                Some(e) => e.1 += f * x,
                None => v.push((l, f * x)),
            }
        }
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(ANCESTRY_SLOTS);
        v.retain(|e| e.1 >= 1e-4);
        let t: f32 = v.iter().map(|e| e.1).sum();
        for e in &mut v {
            e.1 /= t.max(1e-6);
        }
        self.ancestry = if v.len() == 1 && v[0].0 == self.lineage { Vec::new() } else { v };
    }

    /// (v9) Marriage ties with band `id` (0 if none).
    pub fn married_to(&self, id: u32) -> f32 {
        self.partner(id).map(|p| p.married).unwrap_or(0.0)
    }

    /// Strengthen (or start) a partnership. (inventions step 8) No cap: ties that are not kept
    /// up fade away on their own.
    pub fn bond(&mut self, id: u32, by: f32, traded: f32, tick: u32) {
        if let Some(p) = self.partners.iter_mut().find(|p| p.id == id) {
            p.strength = (p.strength + by).min(1.0);
            p.traded += traded;
            return;
        }
        self.partners.push(Partner { id, strength: by.min(1.0), since: tick, traded, married: 0.0 });
    }
    /// (inventions step 8b) The yearly fade of this band's ties. `kin[i]` is its kinship with
    /// `partners[i]`. Care (`TIE_CARE`, more with records) is shared out by what each tie
    /// already means, `w` = trade strength + marriage + kinship: a tie gets `care * w / sum(w^2)`
    /// of full upkeep (at most all of it), so a band's strongest ties are kept up and its many
    /// loose ones fade quickly towards `LOOSE_KEEP` a year. Marriage ties fade on their own
    /// clock; ties under 0.05 (and not married) are left for the caller to drop.
    pub fn fade_ties(&mut self, kin: &[f32]) {
        use crate::world::{LOOSE_KEEP, MARRIAGE_FADE, RECORDS_SPAN, RECORDS_TIE, TIE_CARE};
        let care = TIE_CARE * (1.0 + RECORDS_SPAN * self.records);
        let full = 0.8 + RECORDS_TIE * self.records;
        let wt = |p: &Partner, k: f32| p.strength + p.married + k;
        let s2: f32 = self.partners.iter().zip(kin.iter().chain(std::iter::repeat(&0.0))).map(|(p, &k)| wt(p, k).powi(2)).sum();
        for (i, p) in self.partners.iter_mut().enumerate() {
            let k = kin.get(i).copied().unwrap_or(0.0);
            let c = if s2 > 0.0 { (care * wt(p, k) / s2).min(1.0) } else { 1.0 };
            p.strength *= LOOSE_KEEP + (full - LOOSE_KEEP) * c;
            p.married *= MARRIAGE_FADE;
        }
    }
    /// Grudge this band holds against `o`: in full against that band, a little against its people.
    pub fn grudge_vs(&self, o: &Band) -> f32 {
        let mut g = 0.0f32;
        for x in &self.grudges {
            if x.id == o.id {
                g = g.max(x.w);
            } else if o.parent == x.id || o.id == x.par || (x.par != 0 && o.parent == x.par) {
                // the wrongdoer's parent, daughters and sisters share some of the blame
                g = g.max(0.3 * x.w);
            }
        }
        g
    }
    pub fn add_grudge(&mut self, o: &Band, by: f32) {
        if let Some(x) = self.grudges.iter_mut().find(|x| x.id == o.id) {
            x.w = (x.w + by).min(2.0);
            return;
        }
        if self.grudges.len() >= MAX_GRUDGES {
            let (wi, ww) = self.grudges.iter().enumerate().min_by(|a, b| a.1.w.total_cmp(&b.1.w)).map(|(i, x)| (i, x.w)).unwrap();
            if ww >= by {
                return;
            }
            self.grudges.swap_remove(wi);
        }
        self.grudges.push(Grudge { id: o.id, lineage: o.lineage, w: by.min(2.0), par: o.parent });
    }
    /// Close kin: parent and daughter, or sisters split from the same band.
    pub fn close_kin(&self, o: &Band) -> bool {
        self.parent == o.id || o.parent == self.id || (self.parent != 0 && self.parent == o.parent)
    }
    /// How related two bands are, 0..1: shared descent through the last few splits (1 = parent
    /// and daughter, halving each step further apart), fading with the years since the younger
    /// band split off, as memory of the tie thins (a pure time decay: no marriage between bands
    /// is modelled, so no people or ancestry flow along the tie; v9 step 6 adds intermarriage). Bands that
    /// merely share a distant founder are strangers.
    pub fn kinship(&self, o: &Band, tick: u32) -> f32 {
        if self.lineage != o.lineage {
            return 0.0;
        }
        let ca = [self.id, self.parent, self.anc[0], self.anc[1], self.anc[2]];
        let cb = [o.id, o.parent, o.anc[0], o.anc[1], o.anc[2]];
        let mut deg = usize::MAX;
        for (i, &x) in ca.iter().enumerate() {
            if x == 0 {
                break;
            }
            for (j, &y) in cb.iter().enumerate() {
                if y == 0 {
                    break;
                }
                if x == y {
                    deg = deg.min(i + j);
                }
            }
        }
        if deg == usize::MAX || deg == 0 {
            return 0.0;
        }
        let yrs = tick.saturating_sub(self.born.max(o.born)) as f32 / 4.0;
        0.5f32.powi(deg as i32 - 1) * (-yrs / KIN_FADE_YEARS).exp()
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

/// How dangerous a band's best weapons are, 0..~1.3: the same spears, points and axes it hunts
/// and works with. Nothing is made "for war"; reach and a hard, sharp edge decide.
pub fn weapon_q(k: &Kit) -> f32 {
    // (v8) in a fight that lasts, an edge that chips or breaks is soon useless: weapon quality
    // weighs durability as well as sharpness, so bronze and iron finally beat flint
    let edge = |m: Mat| 0.55 * cut_q(m) + 0.45 * durability(m);
    let hunt = if k.hunt == NONE {
        0.0
    } else {
        let m = mat(k.hunt);
        match form(k.hunt) {
            Form::Spear => 0.35 + 0.65 * edge(m),
            Form::Point => 0.35 * edge(m),
            _ => 0.2 * edge(m),
        }
    };
    let axe = if k.axe == NONE { 0.0 } else { 0.5 * cut_q(mat(k.axe)) + 0.5 * durability(mat(k.axe)) };
    hunt + 0.3 * axe
}

/// (v8) a bow needs arrows: points to tip them
pub fn has_bow(k: &Kit) -> bool {
    k.bow != NONE && k.point != NONE
}

/// (v8) share of a side's losses that still fall when it has bows (fights at a distance, a
/// covered retreat) and shields
pub fn loss_guard(k: &Kit) -> f32 {
    (if has_bow(k) { 0.7 } else { 1.0 }) * (if k.shield != NONE { 0.75 } else { 1.0 })
}

/// Choose the best item for each task from what a band can produce; compute multipliers.
pub fn make_kit(avail: &ItemSet) -> Kit {
    let mut k = Kit::default();
    let score = |q: f32, m: Mat| q * (0.75 + 0.25 * durability(m));
    let mut best = [0f32; 24];
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
            // v8 forms: no recipe makes them yet; the slots are ready for the steps that do
            Form::Bow => upd(14, score(m.props().tough + 1.0, m), &mut k.bow),
            Form::Shield => upd(15, score(m.props().tough + 1.0, m), &mut k.shield),
            Form::Outrigger => upd(16, score(boat_q(m), m), &mut k.outrigger),
            // (inventions step 5b) a cloth sail is kept before a mat one: it serves any hull
            Form::Sail => upd(17, m.props().tough + if m == Mat::Fibre { 0.0 } else { 10.0 }, &mut k.sail),
            Form::Plank => upd(18, score(boat_q(m), m), &mut k.plank),
            Form::Frame => upd(19, vessel_q(m) + 0.1, &mut k.frame),
            // (inventions step 11) a prized material (gold) is worn before any hard one
            Form::Ornament => upd(20, m.props().hard + 1.0 + 10.0 * m.props().prized, &mut k.ornament),
            // (inventions step 10) a jar of grain left to work is ale, not a store jar
            Form::Jar if m == Mat::Grain => k.ale = it,
            Form::Jar => upd(21, vessel_q(m), &mut k.jar),
            Form::Ship => upd(22, score(boat_q(m), m), &mut k.ship),
            // (inventions step 8) the mark that lasts: fired token, bone, stick, raw clay
            Form::Mark => upd(23, durability(m) + if m == Mat::Ceramic { 1.0 } else { 0.0 }, &mut k.mark),
            Form::Raw if m == Mat::Pitch => k.pitch = it,
            Form::Cooked => match m {
                Mat::Plants => k.cooked[0] = it,
                Mat::Meat => k.cooked[1] = it,
                Mat::Fish => k.cooked[2] = it,
                Mat::Grain => k.cooked_grain = it,
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
    // (v8) an outrigger canoe or a plank boat is a boat too, and a plank hull is the soundest
    k.boat_q = k.boat_q.max(q(k.outrigger, &boat_q));
    if k.plank != NONE {
        k.boat_q = k.boat_q.max((boat_q(mat(k.plank)) + 0.25).min(1.0));
    }
    // (inventions step 5) a ship is a plank hull too: no sounder, only bigger and better found
    if k.ship != NONE {
        k.boat_q = k.boat_q.max((boat_q(mat(k.ship)) + 0.25).min(1.0));
    }
    k.plants_m = 1.0 + 0.35 * dig + 0.3 * vq + 0.1 * hammer + 0.1 * axe;
    k.game_m = (1.0 + 1.4 * hunt_q + 0.15 * cord) * (1.0 + 0.35 * cut + 0.05 * hammer);
    k.fish_m = 1.0 + 0.8 * hunt_q + 0.4 * vq + 0.3 * point + 0.2 * cord + 0.9 * netq;
    // small game (hares, birds, rodents): hard to catch by hand, easy with snares and nets
    k.small_m = 0.35 + 0.4 * hunt_q + 1.6 * trapq + 0.4 * netq;
    // (v8) a bow reaches fast game and birds a spear cannot
    if has_bow(&k) {
        k.game_m *= 1.2;
        k.small_m *= 1.15;
    }
    // cooking makes more of each food digestible (boiling in pots helps plants most)
    let ck = |j: usize, g: f32| if k.cooked[j] != NONE { g } else { 1.0 };
    // grain: boiled into porridge (pots help) or ground on a quern stone and baked
    let grain = if k.cooked_grain != NONE { 1.3 + 0.15 * ceramic + 0.1 * hammer } else { 1.0 + 0.1 * hammer };
    k.digest = [ck(0, 1.25 + 0.15 * ceramic), ck(1, 1.2), ck(2, 1.15), ck(1, 1.2), grain];
    // field work: digging stick to plant, a sharp edge as a sickle, an axe to clear ground
    k.farm_m = 0.5 + 0.3 * dig + 0.35 * cut + 0.2 * axe;
    k.smoke = fire > 0.0 && (k.cooked[1] != NONE || k.cooked[2] != NONE);
    let _ = kiln;
    let gq = q(k.garment, &garment_q);
    k.protect = (0.35 * fire + 0.5 * gq + 0.05 * kiln + 0.05 * axe * fire).min(0.92);
    k
}

/// (v9) Refined recipes make the same tools work better: a well-practised hull rides the sea
/// better (boat seaworthiness x(1 + `QUALITY_BOAT` x quality), at most 1), and finer spears,
/// nets, pots and digging sticks bring in a little more (yields x(1 + `QUALITY_YIELD` x
/// quality)). Quality is that of the recipe that makes the item the kit uses.
pub const QUALITY_BOAT: f32 = 0.3;
pub const QUALITY_YIELD: f32 = 0.1;
/// Better-made clothes and shelter: protection + `QUALITY_PROTECT` x quality (still capped).
pub const QUALITY_PROTECT: f32 = 0.06;

/// (v9) Apply recipe quality to a kit made by `make_kit` from the same toolkit.
pub fn refine_kit(k: &mut Kit, tk: &Toolkit, known: &[Known]) {
    let q = |it: Item| {
        tk.via(it).and_then(|(i, _, _)| known.get(i as usize)).map(|kn| kn.quality.clamp(0.0, 1.0)).unwrap_or(0.0)
    };
    let boat = q(k.boat).max(q(k.outrigger)).max(q(k.plank)).max(q(k.ship));
    k.ship_q = if k.ship != NONE { q(k.ship) } else { 0.0 };
    if k.boat_q > 0.0 {
        k.boat_q = (k.boat_q * (1.0 + QUALITY_BOAT * boat)).min(1.0);
    }
    let y = |x: f32| 1.0 + QUALITY_YIELD * x;
    k.game_m *= y(q(k.hunt));
    k.fish_m *= y(q(k.net).max(q(k.hunt)));
    k.small_m *= y(q(k.trap).max(q(k.net)));
    k.plants_m *= y(0.5 * (q(k.vessel) + q(k.dig)));
    k.farm_m *= y(0.5 * (q(k.dig) + q(k.cut)));
    if k.garment != NONE {
        k.protect = (k.protect + QUALITY_PROTECT * q(k.garment)).min(0.92);
    }
    k.fine = q(k.ornament);
}
