//! Recipes (discovered action + inputs -> output), per-band knowledge, and the "toolkit": which
//! items a band can actually produce right now from what it knows and the materials it can reach.
use crate::materials::*;
use crate::physics::{Action, Inputs};
use serde::{Deserialize, Serialize};
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
impl Registry {
    pub fn rebuild_index(&mut self) {
        self.index = self
            .recipes
            .iter()
            .enumerate()
            .map(|(i, r)| ((r.action, r.inputs), i as u32))
            .collect();
        self.retired = self.recipes.iter().map(is_retired).collect();
    }
    /// (inventions step 11b) see `retired`
    pub fn retired(&self, id: u32) -> bool {
        self.retired.get(id as usize).copied().unwrap_or(false)
    }
    pub fn find(&self, action: Action, inputs: &Inputs) -> Option<u32> {
        self.index.get(&(action as u8, *inputs)).copied()
    }
    pub fn insert(&mut self, r: Recipe) -> u32 {
        let id = self.recipes.len() as u32;
        self.index.insert((r.action, r.inputs), id);
        self.retired.resize(id as usize, false);
        self.retired.push(is_retired(&r));
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
}
impl Toolkit {
    /// Position of `i` among the members of `avail`.
    #[inline]
    fn rank(&self, i: Item) -> usize {
        let (w, b) = (i as usize >> 6, i as u32 & 63);
        let below: u32 = self.avail.0[..w].iter().map(|x| x.count_ones()).sum();
        (below + (self.avail.0[w] & ((1u64 << b) - 1)).count_ones()) as usize
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
