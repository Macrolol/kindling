//! Recipes (discovered action + inputs -> output), per-band knowledge, and the "toolkit": which
//! items a band can actually produce right now from what it knows and the materials it can reach.
use crate::effects::{CHAIN_EPS, CHAIN_W_MIN, TECH_CARRY};
use crate::materials::*;
use crate::physics::{cold_workable, Action, Inputs};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A small, fast, deterministic hasher (FxHash) for the hot item and recipe lookups. The
/// default SipHash showed up at ~6% of a single-threaded run. Nothing iterates these maps, so
/// the hash order never reaches the simulation.
#[derive(Default, Clone, Copy)]
pub struct FastHasher(u64);
impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.add(b as u64);
        }
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
}
impl FastHasher {
    #[inline]
    fn add(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}
pub type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Recipe {
    pub action: u8,
    pub inputs: Inputs,
    pub output: Item,
    /// success chance of one attempt (from physics, at discovery)
    pub p: f32,
    pub first_tick: u32,
    pub first_band: u32,
    pub first_cell: u32,
    /// number of times a band found it by its own experiment
    pub inventions: u32,
    /// number of times it was passed between bands (teaching / copying)
    pub transfers: u32,
}
impl Default for Recipe {
    fn default() -> Self {
        Recipe {
            action: 0,
            inputs: [NONE; 4],
            output: NONE,
            p: 0.0,
            first_tick: 0,
            first_band: 0,
            first_cell: 0,
            inventions: 0,
            transfers: 0,
        }
    }
}
impl Recipe {
    pub fn action(&self) -> Action {
        Action::from_u8(self.action)
    }
    pub fn describe(&self) -> String {
        let ins: Vec<String> = self
            .inputs
            .iter()
            .filter(|&&i| i != NONE)
            .map(|&i| item_name(i))
            .collect();
        match self.action() {
            Action::Shape => {
                let tool = if self.inputs[1] == NONE {
                    "bare hands".to_string()
                } else {
                    item_name(self.inputs[1])
                };
                format!("shape {} with {}", item_name(self.inputs[0]), tool)
            }
            Action::Combine => format!("combine {}", ins.join(" + ")),
            Action::Heat => {
                let mut t = vec![item_name(self.inputs[0])];
                if self.inputs[1] != NONE {
                    t.push(item_name(self.inputs[1]));
                }
                let mut s = format!("heat {} over {}", t.join(" + "), item_name(self.inputs[2]));
                if self.inputs[3] != NONE {
                    s += &format!(" in a {}", item_name(self.inputs[3]));
                }
                s
            }
        }
    }
}

/// (skills step 3) How a recipe is made, derived from its action, inputs and output.
/// Stored on the registry, never in the save. `Other` carries no maker skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Tech {
    Knapping = 0,
    GrindCarve,
    CordageNetting,
    Hafting,
    Sewing,
    SpinWeave,
    PottingFiring,
    FireMaking,
    Cooking,
    ColdHammer,
    Smelting,
    Casting,
    HotForging,
    SteelMaking,
    BoatBuilding,
    Other,
}

/// Global registry of every recipe any band has ever found.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Registry {
    pub recipes: Vec<Recipe>,
    #[serde(skip)]
    index: FastMap<(u8, Inputs), u32>,
    /// (inventions step 11b) recipes found under older rules that physics no longer allows
    /// (cold-hammered gold): bands that know them from an old save keep them, but they are not
    /// taught on. Derived on load, never saved
    #[serde(skip)]
    retired: Vec<bool>,
    /// (skills step 3) technique of each recipe. Derived, never saved
    #[serde(skip)]
    tech: Vec<Tech>,
}
/// A recipe from an older save that today's physics would not allow: (inventions step 11b) a
/// gold nugget beaten into a ring with a hammerstone (gold is now cast; see `physics::heat`);
/// (inventions step 12) anything the recipe-realism rules forbid (a wool shaft, a tin knife,
/// cold-hammered bronze or iron, a sail of clothing...). Judged by physics itself: the recipe is
/// retired when its inputs no longer give its output, at the water's edge or away from it.
pub fn is_retired(r: &Recipe) -> bool {
    let a = Action::from_u8(r.action);
    ![false, true].iter().any(|&water| {
        crate::physics::apply(a, &r.inputs, &crate::physics::Ctx { dryness: 0.5, water }).is_some_and(|o| o.out == r.output)
    })
}
/// (inventions step 11b) the one retired recipe bands keep from an old save: cold-hammered gold
/// rings (their rings stay; the recipe is never taught). Every other retired recipe is dropped
/// on load (`World::clean_recipes`)
pub fn is_legacy_gold(r: &Recipe) -> bool {
    r.action == Action::Shape as u8 && r.inputs[0] == item(Form::Raw, Mat::Gold)
}

/// Technique of `r`, from its action, inputs and output (the physics comments).
/// Anything that matches no named branch is `Other`.
pub fn technique(r: &Recipe) -> Tech {
    match r.action() {
        Action::Shape => technique_shape(r),
        Action::Combine => technique_combine(r),
        Action::Heat => technique_heat(r),
    }
}

fn technique_shape(r: &Recipe) -> Tech {
    let target = r.inputs[0];
    let tool = r.inputs[1];
    let (tf, tm) = (form(target), mat(target));
    match form(r.output) {
        // A hard cobble picked up as a pounder. Not a named technique.
        Form::Hammer => Tech::Other,
        Form::Edge => {
            if tf == Form::Raw && cold_workable(tm) {
                Tech::ColdHammer
            } else {
                Tech::Knapping
            }
        }
        Form::Cord | Form::Net => Tech::CordageNetting,
        Form::Vessel => {
            if tf == Form::Cord {
                Tech::CordageNetting
            } else {
                Tech::PottingFiring
            }
        }
        // Digging stick, carved shaft, sharpened spear, bow, bead, pendant.
        Form::Shaft | Form::Spear | Form::Bow | Form::Ornament => Tech::GrindCarve,
        Form::Plank | Form::Boat => Tech::BoatBuilding,
        Form::Mould => Tech::PottingFiring,
        // A clay token pressed with a point. A tally is a combine, not a shape.
        Form::Mark => {
            if tm.props().plastic >= 6.0 {
                Tech::PottingFiring
            } else {
                Tech::Other
            }
        }
        Form::Point => {
            if tf == Form::Edge && cold_workable(tm) && form(tool) == Form::Hammer {
                Tech::ColdHammer
            } else if tf == Form::Edge && tm.props().fracture >= 5.0 {
                Tech::Knapping
            } else {
                // Carved bone, or a cast blade ground on a stone.
                Tech::GrindCarve
            }
        }
        _ => Tech::Other,
    }
}

fn technique_combine(r: &Recipe) -> Tech {
    match form(r.output) {
        Form::Spear | Form::Axe | Form::Trap | Form::Frame | Form::Bellows | Form::Loom => Tech::Hafting,
        // A shield is the inland hide-on-a-frame lashing (the same inputs make a
        // skin boat at the water): the work of a bellows, a frame or a loom.
        Form::Shield => Tech::Hafting,
        Form::Boat | Form::Outrigger | Form::Sail | Form::Ship | Form::Plank => Tech::BoatBuilding,
        Form::Cloth => Tech::SpinWeave,
        Form::Crucible => Tech::Casting,
        Form::Jar => {
            if mat(r.output) == Mat::Grain {
                Tech::Cooking
            } else {
                Tech::PottingFiring
            }
        }
        Form::Garment => Tech::Sewing,
        Form::Mark => Tech::GrindCarve,
        Form::Fire | Form::Kiln => Tech::FireMaking,
        _ => Tech::Other,
    }
}

fn technique_heat(r: &Recipe) -> Tech {
    let (out_f, out_m) = (form(r.output), mat(r.output));
    // The smith's hearth (and the old furnace recipe) whose output is a steel bar.
    if out_f == Form::Raw && out_m == Mat::Steel {
        return Tech::SteelMaking;
    }
    // Open charcoal hearth: bellows sit in the kiln slot. Blades and points
    // forged there are hot-forging; the steel bar already returned above.
    if r.inputs[3] != NONE && form(r.inputs[3]) == Form::Bellows {
        return if matches!(out_f, Form::Edge | Form::Point) { Tech::HotForging } else { Tech::Other };
    }
    let crucible = r.inputs.iter().any(|&i| i != NONE && form(i) == Form::Crucible);
    match out_f {
        Form::Cooked => Tech::Cooking,
        Form::Bloom => Tech::Smelting,
        Form::Ornament => Tech::Casting,
        // Fire a moulded pot, a clay token or a mould.
        Form::Vessel | Form::Mark | Form::Mould => Tech::PottingFiring,
        Form::Edge | Form::Point => {
            if crucible {
                Tech::Casting
            } else {
                Tech::HotForging
            }
        }
        Form::Raw => match out_m {
            Mat::Salt | Mat::Pitch => Tech::Cooking,
            Mat::Charcoal => Tech::FireMaking,
            // Tin stirred into molten copper. Ores fired together are smelting.
            Mat::Bronze if crucible => Tech::Casting,
            Mat::Bronze => Tech::Smelting,
            // Bloom hammered to a wrought bar.
            Mat::Iron => Tech::HotForging,
            Mat::Copper | Mat::Tin | Mat::Gold => Tech::Smelting,
            _ => Tech::Other,
        },
        _ => Tech::Other,
    }
}

impl Registry {
    pub fn rebuild_index(&mut self) {
        self.index = self
            .recipes
            .iter()
            .enumerate()
            .map(|(i, r)| ((r.action, r.inputs), i as u32))
            .collect();
        self.retired = self.recipes.iter().map(is_retired).collect();
        self.tech = self.recipes.iter().map(technique).collect();
    }
    /// (inventions step 11b) see `retired`
    pub fn retired(&self, id: u32) -> bool {
        self.retired.get(id as usize).copied().unwrap_or(false)
    }
    /// Technique label of recipe `id`. `Other` when the id is unknown.
    pub fn tech(&self, id: u32) -> Tech {
        self.tech.get(id as usize).copied().unwrap_or(Tech::Other)
    }
    pub fn find(&self, action: Action, inputs: &Inputs) -> Option<u32> {
        self.index.get(&(action as u8, *inputs)).copied()
    }
    pub fn insert(&mut self, r: Recipe) -> u32 {
        let id = self.recipes.len() as u32;
        self.index.insert((r.action, r.inputs), id);
        self.retired.resize(id as usize, false);
        self.retired.push(is_retired(&r));
        self.tech.resize(id as usize, Tech::Other);
        self.tech.push(technique(&r));
        self.recipes.push(r);
        id
    }
}

/// One recipe a band knows, with how practised it is.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Known {
    pub recipe: u32,
    /// 0..1; decays when unused, grows with practice; forgotten below a floor
    pub skill: f32,
    pub since: u32,
    /// band id it was learned from (u32::MAX = own invention, u32::MAX-1 = inherited at a split)
    pub from: u32,
    /// found by this band or an ancestor (kept through splits, false when taught by others);
    /// added in save format v3
    pub kin: bool,
    /// (v9) how refined the band's way of doing it is, 0..`skill`: grows with play and practice
    /// (`World::refine`), passes partly to learners and daughters, and fades in small bands.
    /// 0 (older saves, new inventions) = the plain recipe.
    pub quality: f32,
    /// (diffusion D5, save format v15) the last tick the recipe was practised or all its inputs
    /// were at hand (traded stock too): while that is recent, the skill fades more slowly.
    /// 0 (older saves) = long ago.
    pub last_ok: u32,
    #[serde(skip)]
    pub used: bool,
}
pub const FROM_SELF: u32 = u32::MAX;
pub const FROM_PARENT: u32 = u32::MAX - 1;

/// Steps `fill_chain` adds for one closure, in walk order. Not saved.
/// A later refill with the same graph replays them; only the maker skills change.
#[derive(Clone, Debug, Default)]
struct ChainReplay {
    ki: Vec<u16>,
    w: Vec<f32>,
    /// one entry per `via` rank; `nstep == 0` is a raw item or a chain that added nothing
    start: Vec<u32>,
    nstep: Vec<u32>,
    ready: bool,
}

/// What a band can produce right now.
#[derive(Clone, Debug, Default)]
pub struct Toolkit {
    pub avail: ItemSet,
    /// items producible through known recipes alone (ignoring a kept flame)
    pub producible: ItemSet,
    /// for each produced item: the known-list index of the recipe used, and the raw materials
    /// (bitmask over Mat) its whole production chain needs, and chain length
    /// (one entry per member of `avail`, in item order: dense and hash-free)
    via: Vec<(u16, u32, u8)>,
    /// (skills step 3) chain skill of each available item, same order as `via`.
    /// Raw items and a kept flame are 1. Filled by `fill_chain`, never saved.
    /// The economy reads `chain_m`. The live kit choice does too; nominal callers force 1.
    chain: Vec<f32>,
    /// `Derived::skill_ver` when `chain` was filled. The toolkit is not saved.
    chain_ver: u32,
    /// Item id -> rank in `avail`, or `u16::MAX` when absent. Empty while `closure`
    /// is still inserting. Not saved.
    rank_of: Vec<u16>,
    replay: ChainReplay,
}
impl Toolkit {
    /// Position of `i` among the members of `avail`.
    #[inline]
    fn rank(&self, i: Item) -> usize {
        let iu = i as usize;
        if iu < self.rank_of.len() {
            let r = self.rank_of[iu] as usize;
            if r != u16::MAX as usize || !self.avail.has(i) {
                return r;
            }
        }
        self.rank_slow(i)
    }
    #[inline]
    fn rank_slow(&self, i: Item) -> usize {
        let (w, b) = (i as usize >> 6, i as u32 & 63);
        let below: u32 = self.avail.0[..w].iter().map(|x| x.count_ones()).sum();
        (below + (self.avail.0[w] & ((1u64 << b) - 1)).count_ones()) as usize
    }
    fn rebuild_rank(&mut self) {
        self.rank_of.clear();
        self.rank_of.resize(NITEMS, u16::MAX);
        for (rank, it) in self.avail.iter().enumerate() {
            let iu = it as usize;
            if iu < NITEMS {
                self.rank_of[iu] = rank as u16;
            }
        }
    }
    /// Chain skill of an available item. 1 when `i` is absent or `fill_chain` has not
    /// run (a fully practised maker), so a caller that does not have a band stays at 1.
    #[inline]
    pub fn chain_m(&self, i: Item) -> f32 {
        if i == NONE || !self.avail.has(i) {
            return 1.0;
        }
        self.chain.get(self.rank(i)).copied().unwrap_or(1.0)
    }
    pub(crate) fn chain_ver(&self) -> u32 {
        self.chain_ver
    }
    pub(crate) fn set_chain_ver(&mut self, v: u32) {
        self.chain_ver = v;
    }
    /// The recipe (known-list index, `u16::MAX` for a raw or kept fire), raw-material mask and
    /// chain length behind an available item.
    #[inline]
    pub fn via(&self, i: Item) -> Option<(u16, u32, u8)> {
        if i == NONE || !self.avail.has(i) {
            return None;
        }
        self.via.get(self.rank(i)).copied()
    }
    /// Make `i` available through `v` (replacing how it was available before).
    fn set_via(&mut self, i: Item, v: (u16, u32, u8)) {
        // The rank index and the recorded walk describe the graph before this edit.
        self.rank_of.clear();
        self.replay.ready = false;
        if self.avail.has(i) {
            let r = self.rank(i);
            self.via[r] = v;
        } else {
            self.avail.add(i);
            let r = self.rank(i);
            self.via.insert(r, v);
        }
    }
}

/// Compute the closure of producible items. `raws` = raw items directly at hand; `flame` = the band
/// keeps a captured fire (gives `fire` while wood is at hand).
pub fn closure(reg: &Registry, known: &[Known], raws: &ItemSet, flame: bool) -> Toolkit {
    let mut tk = Toolkit { avail: *raws, ..Default::default() };
    tk.via.reserve(raws.count() as usize + known.len() + 1);
    for i in raws.iter() {
        tk.via.push((u16::MAX, 1u32 << (i % 32), 0));
    }
    let fire = item(Form::Fire, Mat::Wood);
    let wood = item(Form::Raw, Mat::Wood);
    if flame && raws.has(wood) {
        tk.set_via(fire, (u16::MAX, 1 << Mat::Wood as u32, 0));
    }
    let mut changed = true;
    let mut guard = 0;
    while changed && guard < 16 {
        changed = false;
        guard += 1;
        for (k, kn) in known.iter().enumerate() {
            let r = &reg.recipes[kn.recipe as usize];
            if !r.inputs.iter().all(|&i| tk.avail.has(i)) {
                continue;
            }
            // a flame-kept fire does not count as "producible": a fire-making recipe replaces it
            let already = tk.producible.has(r.output);
            if already {
                continue;
            }
            let mut mask = 0u32;
            let mut depth = 0u8;
            for &i in r.inputs.iter().filter(|&&i| i != NONE) {
                if let Some((_, m, d)) = tk.via(i) {
                    mask |= m;
                    depth = depth.max(d);
                }
            }
            tk.set_via(r.output, (k as u16, mask, depth + 1));
            tk.producible.add(r.output);
            changed = true;
        }
    }
    tk.rebuild_rank();
    tk
}

/// Mark every recipe in the production chain of `it` as used this season.
pub fn mark_used(reg: &Registry, known: &mut [Known], tk: &Toolkit, it: Item) {
    // fixed stack (no allocation): each popped recipe pushes at most 4 inputs and at most 64
    // entries are popped, as before
    let mut stack = [NONE; 4 * 64 + 1];
    let mut sp = 1usize;
    stack[0] = it;
    let mut guard = 0;
    while sp > 0 {
        sp -= 1;
        let i = stack[sp];
        guard += 1;
        if guard > 64 {
            break;
        }
        if let Some((k, _, _)) = tk.via(i) {
            if k == u16::MAX {
                continue;
            }
            let kn = &mut known[k as usize];
            if kn.used {
                continue;
            }
            kn.used = true;
            for &j in reg.recipes[kn.recipe as usize].inputs.iter() {
                if j != NONE && sp < stack.len() {
                    stack[sp] = j;
                    sp += 1;
                }
            }
        }
    }
}

const TECH_N: usize = Tech::Other as usize + 1;

/// Effective maker skill of one known recipe.
/// `max(own skill, TECH_CARRY * best skill among the band's other known recipes
/// of the same technique)`. Skill is clamped to 0..1. `Other` is just own skill.
pub fn maker_skill(reg: &Registry, known: &[Known], idx: usize) -> f32 {
    CHAIN_SCRATCH.with(|cell| {
        let mut sc = cell.borrow_mut();
        maker_skills_into(reg, known, &mut sc.skills);
        sc.skills.get(idx).copied().unwrap_or(0.0)
    })
}

/// One copy of every known recipe's effective maker skill, for a caller that
/// indexes it more than once (`refine`). The scratch buffer is reused.
pub(crate) fn maker_skills_copy(reg: &Registry, known: &[Known]) -> Vec<f32> {
    CHAIN_SCRATCH.with(|cell| {
        let mut sc = cell.borrow_mut();
        maker_skills_into(reg, known, &mut sc.skills);
        sc.skills.clone()
    })
}

/// Best effective maker skill of hot-forging, steel-making, casting and smelting,
/// in that order. `None` when the band knows no recipe of that technique.
/// One pass, no allocation. The values are raw, not floored.
///
/// The best carried skill of a technique is `max(best, TECH_CARRY * second)` on
/// the strongest recipe and `max(second, TECH_CARRY * best)` on the next. With
/// `TECH_CARRY` ≤ 1 that is the best own skill; both are taken so a larger carry
/// still matches `maker_skill`.
pub fn maker_tech_best(reg: &Registry, known: &[Known]) -> [Option<f32>; 4] {
    let mut best_s = [0.0f32; 4];
    let mut second_s = [0.0f32; 4];
    let mut n = [0u32; 4];
    for kn in known {
        let slot = match reg.tech(kn.recipe) {
            Tech::HotForging => 0,
            Tech::SteelMaking => 1,
            Tech::Casting => 2,
            Tech::Smelting => 3,
            _ => continue,
        };
        let s = kn.skill.clamp(0.0, 1.0);
        n[slot] += 1;
        if s > best_s[slot] {
            second_s[slot] = best_s[slot];
            best_s[slot] = s;
        } else if s > second_s[slot] {
            second_s[slot] = s;
        }
    }
    let mut best = [None; 4];
    for slot in 0..4 {
        if n[slot] == 0 {
            continue;
        }
        let top = best_s[slot].max(TECH_CARRY * second_s[slot]).clamp(0.0, 1.0);
        let other = if n[slot] >= 2 { second_s[slot].max(TECH_CARRY * best_s[slot]).clamp(0.0, 1.0) } else { 0.0 };
        best[slot] = Some(top.max(other));
    }
    best
}

fn maker_skills_into(reg: &Registry, known: &[Known], out: &mut Vec<f32>) {
    let mut best = [0.0f32; TECH_N];
    let mut second = [0.0f32; TECH_N];
    let mut best_at = [usize::MAX; TECH_N];
    let other = Tech::Other as usize;
    for (i, kn) in known.iter().enumerate() {
        let t = reg.tech(kn.recipe) as usize;
        if t == other || t >= TECH_N {
            continue;
        }
        let s = kn.skill.clamp(0.0, 1.0);
        if s > best[t] {
            second[t] = best[t];
            best[t] = s;
            best_at[t] = i;
        } else if s > second[t] {
            second[t] = s;
        }
    }
    out.clear();
    out.reserve(known.len());
    for (i, kn) in known.iter().enumerate() {
        let t = reg.tech(kn.recipe) as usize;
        let s = kn.skill.clamp(0.0, 1.0);
        if t == other || t >= TECH_N {
            out.push(s);
            continue;
        }
        let other_best = if best_at[t] == i { second[t] } else { best[t] };
        out.push(s.max(TECH_CARRY * other_best).clamp(0.0, 1.0));
    }
}

fn chain_weight(p: f32) -> f32 {
    // Physics success chances sit in (0, 1]. A non-positive p has no finite -ln;
    // treat it as a sure step so the weight is the floor.
    let p = if p > 0.0 { p.min(1.0) } else { 1.0 };
    (-p.ln()).max(CHAIN_W_MIN)
}

/// Reused by `fill_chain` and `maker_skill`. Not saved. One per thread: the
/// season and meetings fill chains in parallel.
struct ChainScratch {
    skills: Vec<f32>,
    seen: Vec<u32>,
    rank_of: Vec<u16>,
    touched: Vec<u16>,
}

thread_local! {
    static CHAIN_SCRATCH: RefCell<ChainScratch> = RefCell::new(ChainScratch {
        skills: Vec::new(),
        seen: Vec::new(),
        rank_of: Vec::new(),
        touched: Vec::new(),
    });
}

/// Chain skill of every available item, parallel to `via`.
/// One walk per item, the same stack bound as `mark_used`, each known recipe at
/// most once. `m_item` is the weighted geometric mean of those steps' maker
/// skills; raw items and a kept flame are 1. The economy and the live kit
/// choice read it. Nominal callers force 1.
pub fn fill_chain(reg: &Registry, known: &[Known], tk: &mut Toolkit) {
    let n = tk.via.len();
    tk.chain.clear();
    tk.chain.resize(n, 1.0);
    if n == 0 || known.is_empty() {
        tk.replay.ready = false;
        return;
    }
    // Same closure as the fill that recorded the steps: only maker skills moved.
    if tk.replay.ready && tk.replay.start.len() == n && tk.replay.nstep.len() == n {
        replay_chain(reg, known, tk);
        return;
    }
    tk.replay.ki.clear();
    tk.replay.w.clear();
    tk.replay.start.clear();
    tk.replay.nstep.clear();
    tk.replay.ready = false;
    CHAIN_SCRATCH.with(|cell| {
        let mut sc = cell.borrow_mut();
        maker_skills_into(reg, known, &mut sc.skills);
        if sc.rank_of.len() != NITEMS {
            sc.rank_of.clear();
            sc.rank_of.resize(NITEMS, u16::MAX);
            sc.touched.clear();
        }
        let ntouch = sc.touched.len();
        for ti in 0..ntouch {
            let iu = sc.touched[ti] as usize;
            if iu < sc.rank_of.len() {
                sc.rank_of[iu] = u16::MAX;
            }
        }
        sc.touched.clear();
        for (rank, it) in tk.avail.iter().enumerate() {
            if rank >= n {
                break;
            }
            let iu = it as usize;
            if iu < NITEMS {
                sc.rank_of[iu] = rank as u16;
                sc.touched.push(iu as u16);
            }
        }
        if sc.seen.len() != known.len() {
            sc.seen.resize(known.len(), 0);
        }
        sc.seen.fill(0);
        // Same starting stamp as a fresh `seen`: the first item uses 2.
        let mut stamp = 1u32;
        let mut stack = [NONE; 4 * 64 + 1];
        for (rank, it) in tk.avail.iter().enumerate() {
            if rank >= n {
                break;
            }
            if tk.via[rank].0 == u16::MAX {
                tk.replay.start.push(0);
                tk.replay.nstep.push(0);
                continue;
            }
            stamp = stamp.wrapping_add(1);
            if stamp == 0 {
                sc.seen.fill(0);
                stamp = 1;
            }
            let mut sum_w = 0.0f32;
            let mut sum_wl = 0.0f32;
            let step0 = tk.replay.ki.len() as u32;
            stack[0] = it;
            let mut sp = 1usize;
            let mut guard = 0u32;
            while sp > 0 {
                sp -= 1;
                let i = stack[sp];
                guard += 1;
                if guard > 64 {
                    break;
                }
                let iu = i as usize;
                if iu >= NITEMS {
                    continue;
                }
                let rr = sc.rank_of[iu];
                if rr == u16::MAX || rr as usize >= n {
                    continue;
                }
                let k = tk.via[rr as usize].0;
                if k == u16::MAX {
                    continue;
                }
                let ki = k as usize;
                if ki >= known.len() || sc.seen[ki] == stamp {
                    continue;
                }
                sc.seen[ki] = stamp;
                let m = sc.skills.get(ki).copied().unwrap_or(1.0).max(CHAIN_EPS);
                let rec = known[ki].recipe as usize;
                let Some(recipe) = reg.recipes.get(rec) else { continue };
                let w = chain_weight(recipe.p);
                sum_w += w;
                sum_wl += w * m.ln();
                tk.replay.ki.push(ki as u16);
                tk.replay.w.push(w);
                for &j in &recipe.inputs {
                    if j != NONE && sp < stack.len() {
                        stack[sp] = j;
                        sp += 1;
                    }
                }
            }
            if sum_w > 0.0 {
                tk.chain[rank] = (sum_wl / sum_w).exp();
            }
            let nstep = tk.replay.ki.len() as u32 - step0;
            tk.replay.start.push(step0);
            tk.replay.nstep.push(nstep);
        }
        tk.replay.ready = tk.replay.start.len() == n && tk.replay.ki.len() == tk.replay.w.len();
    });
}

/// Same adds as the recording walk, in the same order. `sum_w` is rebuilt from
/// the stored weights so the division matches the first fill bit for bit.
fn replay_chain(reg: &Registry, known: &[Known], tk: &mut Toolkit) {
    CHAIN_SCRATCH.with(|cell| {
        let mut sc = cell.borrow_mut();
        maker_skills_into(reg, known, &mut sc.skills);
        let n = tk.chain.len();
        for rank in 0..n {
            let nstep = tk.replay.nstep[rank] as usize;
            if nstep == 0 {
                continue;
            }
            let s = tk.replay.start[rank] as usize;
            let mut sum_w = 0.0f32;
            let mut sum_wl = 0.0f32;
            for i in s..s + nstep {
                let ki = tk.replay.ki[i] as usize;
                let w = tk.replay.w[i];
                let m = sc.skills.get(ki).copied().unwrap_or(1.0).max(CHAIN_EPS);
                sum_w += w;
                sum_wl += w * m.ln();
            }
            if sum_w > 0.0 {
                tk.chain[rank] = (sum_wl / sum_w).exp();
            }
        }
    });
}
