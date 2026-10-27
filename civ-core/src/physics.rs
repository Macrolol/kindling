//! What happens when a primitive action is applied to things. These rules are written in terms of
//! material PROPERTIES (hardness, fracture, plasticity, fuel temperature...), never in terms of a
//! technology list. A "recipe" is simply an (action, inputs) pair that a band has observed to give
//! a useful output; recipes are discovered by trying actions on whatever is at hand.
use crate::materials::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Action {
    /// work a target with a tool (or bare hands): knap, carve, twist, mould, hammer
    Shape = 0,
    /// put 2-3 things together: haft, bind, sew, build around, feed, rub
    Combine = 1,
    /// put 1-2 things in a fire (optionally an enclosed kiln)
    Heat = 2,
}
impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::Shape => "shape",
            Action::Combine => "combine",
            Action::Heat => "heat",
        }
    }
    pub fn from_u8(x: u8) -> Action {
        match x {
            0 => Action::Shape,
            1 => Action::Combine,
            _ => Action::Heat,
        }
    }
}

/// Canonical input layout:
/// - Shape:   [target, tool or NONE (hands), NONE, NONE]
/// - Combine: up to 3 items sorted ascending, NONE-padded
/// - Heat:    [target1, target2 or NONE, fire, kiln or NONE] (targets sorted)
pub type Inputs = [Item; 4];

pub fn canonical(action: Action, mut inp: Inputs) -> Inputs {
    match action {
        Action::Shape => [inp[0], inp[1], NONE, NONE],
        Action::Combine => {
            inp[..3].sort();
            [inp[0], inp[1], inp[2], NONE]
        }
        Action::Heat => {
            if inp[1] < inp[0] {
                inp.swap(0, 1);
            }
            inp
        }
    }
}

/// Environmental context of an attempt.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ctx {
    /// 0 (wet) .. 1 (arid): friction fire-making is easier in dry places
    pub dryness: f32,
    /// at the water's edge (sea or river): only here does anyone find out what floats
    pub water: bool,
}

/// Result of applying an action: the output item and the chance a single attempt succeeds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    pub out: Item,
    pub p: f32,
}

fn o(out: Item, p: f32) -> Option<Outcome> {
    Some(Outcome { out, p: p.clamp(0.0, 1.0) })
}

pub fn apply(action: Action, inp: &Inputs, ctx: &Ctx) -> Option<Outcome> {
    match action {
        Action::Shape => shape(inp[0], inp[1], ctx),
        Action::Combine => combine(inp, ctx),
        Action::Heat => heat(inp, ctx),
    }
}

fn is_metal(m: Mat) -> bool {
    m.props().malleable >= 5.0
}

/// (inventions step 12) the hardness of copper: the softest metal that takes and holds a working
/// edge or point (tin and gold are too soft for a blade, an awl, a needle or a nail)
pub const EDGE_HARD: f32 = 3.0;
/// (inventions step 12) a metal soft and ductile enough to be beaten into a blade cold, with a
/// hammerstone (copper). Bronze is cast, iron and steel are forged hot
pub fn cold_workable(m: Mat) -> bool {
    let p = m.props();
    p.malleable >= 8.0 && p.hard >= EDGE_HARD && p.prized <= 0.0
}
/// (inventions step 12) a metal that melts in a charcoal furnace (below forging heat), so it can
/// be melted in a crucible and cast: copper, tin, bronze, gold. Iron and steel never melt there
pub fn castable(m: Mat) -> bool {
    is_metal(m) && m.props().melt > 0.0 && m.props().melt < FORGE_T
}
/// (inventions step 12) a metal worked only hot, at the forge (iron, steel)
pub fn hot_worked(m: Mat) -> bool {
    is_metal(m) && m.props().melt >= FORGE_T
}
/// (inventions step 12) only woody stuff (wood) has the long straight grain of a shaft
pub fn woody(m: Mat) -> bool {
    m.props().woody >= 5.0
}
/// (inventions step 12) a point that can pierce hide, carry a spear or serve as a nail: hard
/// enough (no tin or gold points; there are none anyway, this is a second guard)
fn point_ok(i: Item) -> bool {
    form(i) == Form::Point && mat(i).props().hard >= EDGE_HARD
}
/// (inventions step 12) a loose hard, tough stone used as a grindstone (a raw cobble, not a tool)
fn grindstone(i: Item) -> bool {
    form(i) == Form::Raw && mat(i).props().hard >= 5.0 && mat(i).props().tough >= 5.0
}
/// (inventions step 12) a fibre that spins into a yarn strong and fine enough to weave into
/// cloth (flax, wool; not brittle wild plant fibre, not hide)
fn cloth_fibre(m: Mat) -> bool {
    let p = m.props();
    m != Mat::Hide && p.fibrous >= 8.0 && p.tough >= 4.0 && p.food < 1.0
}

fn shape(t: Item, tool: Item, ctx: &Ctx) -> Option<Outcome> {
    if t == NONE {
        return None;
    }
    let (tf, tm) = (form(t), mat(t));
    let tp = tm.props();
    let hands = tool == NONE;
    let (of, om) = if hands { (Form::Raw, Mat::Wood) } else { (form(tool), mat(tool)) };
    let op = om.props();
    match tf {
        Form::Raw => {
            // (inventions step 11b) native gold is too soft for any edge, and it is not hammered
            // into a ring either: it has to be melted in a crucible and cast (see `heat`)
            if tm == Mat::Gold {
                return None;
            }
            // metals: hammer into a blade (cold working) with a harder hammerstone. (inventions
            // step 12) only a soft, ductile metal as hard as copper: tin is too soft to hold an
            // edge, bronze is cast, iron and steel are forged hot (see `heat`)
            if is_metal(tm) {
                if cold_workable(tm) && of == Form::Hammer && op.hard >= tp.hard {
                    return o(item(Form::Edge, tm), 0.5);
                }
                return None;
            }
            // a hard, tough cobble picked and used as a pounder
            if hands && tp.hard >= 5.0 && tp.tough >= 5.0 {
                return o(item(Form::Hammer, tm), 0.9);
            }
            // knapping: strike a stone that fractures conchoidally with a hard-enough hammer
            if of == Form::Hammer && tp.fracture >= 3.0 && op.hard >= tp.hard - 1.5 && tool != t {
                let p = 0.08 * tp.fracture;
                return o(item(Form::Edge, tm), p);
            }
            // twist fibres into cord with bare hands; cut hide into thongs with an edge
            if tp.fibrous >= 8.0 && hands {
                return o(item(Form::Cord, tm), 0.6);
            }
            if tp.fibrous >= 5.0 && of == Form::Edge && cut_q(om) > 0.3 && tp.food < 2.0 && tp.workable >= 5.0 && tm != Mat::Wood {
                return o(item(Form::Cord, tm), 0.5 * cut_q(om) + 0.2);
            }
            // mould a plastic material into a container
            if tp.plastic >= 6.0 && hands {
                return o(item(Form::Vessel, tm), 0.5);
            }
            // break off / strip a branch: a crude digging stick
            if tm == Mat::Wood && hands {
                return o(item(Form::Shaft, tm), 0.4);
            }
            // (v8) a metal axe splits a log into planks, sewn or pegged into a large hull
            if ctx.water && tm == Mat::Wood && of == Form::Axe && is_metal(om) {
                return o(item(Form::Plank, tm), 0.05);
            }
            // fell and hollow out a log with an axe: a dugout (floats because wood floats)
            if ctx.water && tp.float >= 5.0 && tp.tough >= 5.0 && of == Form::Axe && cut_q(om) > 0.4 {
                return o(item(Form::Boat, tm), 0.04 + 0.2 * cut_q(om));
            }
            // carve with an edge harder than the material
            if of == Form::Edge && tp.workable >= 4.0 && op.hard > tp.hard + 1.0 {
                // (inventions step 12) a shaft only from wood: wool, flax, fibre or hide carved
                // with a blade give no handle
                if woody(tm) && tp.food < 1.0 {
                    return o(item(Form::Shaft, tm), 0.3 + 0.6 * cut_q(om));
                }
                if tp.hard >= 3.0 {
                    // bone: an awl / point
                    return o(item(Form::Point, tm), 0.2 + 0.5 * cut_q(om));
                }
            }
            // (inventions step 11b) scoop a hollow out of a slab of soft clay with an edge: a mould
            // (useless until fired hard, and until there is molten metal to pour into it)
            if of == Form::Edge && tp.plastic >= 6.0 && cut_q(om) > 0.3 {
                return o(item(Form::Mould, tm), 0.3);
            }
            // (inventions step 8) press a lump of soft clay with a point: a token that stands for
            // a thing counted
            if point_ok(tool) && tp.plastic >= 6.0 {
                return o(item(Form::Mark, tm), 0.3);
            }
            None
        }
        Form::Edge => {
            // metals: draw an edge out into a point with a hammer (cold: copper only; iron and
            // steel are drawn out hot, see `heat`)
            if cold_workable(tm) && of == Form::Hammer && op.hard >= tp.hard {
                return o(item(Form::Point, tm), 0.5);
            }
            // (inventions step 12) a cast blade of bronze (or copper) ground to a point on a
            // grindstone: an awl, a pin, a nail
            if castable(tm) && tp.hard >= EDGE_HARD && grindstone(tool) {
                return o(item(Form::Point, tm), 0.3);
            }
            // pressure flaking: press a fine flaking stone with a softer, tough tool
            if tp.fracture >= 5.0 && !hands && op.hard < tp.hard && op.tough >= 4.0 && op.hard >= 3.0 {
                return o(item(Form::Point, tm), 0.05 * tp.fracture);
            }
            None
        }
        Form::Point => {
            // (v8) a bone or soft metal point hammered and ground into a bead or pin, kept for show
            // (inventions step 12: no tin, which makes no point; obsidian is not hammered)
            if of == Form::Hammer && matches!(tm, Mat::Bone | Mat::Copper | Mat::Bronze) {
                return o(item(Form::Ornament, tm), 0.2);
            }
            // (inventions step 12) an obsidian point ground and polished on a grindstone: a
            // pendant (a blow with a hammer only shatters it)
            if tm == Mat::Obsidian && grindstone(tool) {
                return o(item(Form::Ornament, tm), 0.15);
            }
            None
        }
        Form::Cord => {
            // knot cord into an open mesh over a gauge stick / with a netting needle
            if !hands && ((of == Form::Shaft && woody(om)) || point_ok(tool)) && tp.fibrous >= 5.0 {
                let p = if of == Form::Point { 0.25 } else { 0.15 };
                return o(item(Form::Net, tm), p);
            }
            // weave / knot cord into a container
            if hands && tp.fibrous >= 5.0 {
                return o(item(Form::Vessel, tm), 0.4);
            }
            None
        }
        Form::Shaft => {
            // (v8) bend a springy stave and string it with a strong cord: a bow
            if of == Form::Cord && tm == Mat::Wood && tp.tough >= 5.0 && op.tough >= 4.0 && op.fibrous >= 5.0 {
                return o(item(Form::Bow, om), 0.03 + 0.01 * op.tough);
            }
            // sharpen the end of a shaft with an edge: a simple spear (a wooden shaft only)
            if of == Form::Edge && woody(tm) && op.hard > tp.hard + 1.0 && tp.workable >= 4.0 {
                return o(item(Form::Spear, tm), 0.3 + 0.5 * cut_q(om));
            }
            None
        }
        _ => None,
    }
}

fn combine(inp: &Inputs, ctx: &Ctx) -> Option<Outcome> {
    let items: Vec<Item> = inp.iter().copied().filter(|&i| i != NONE).collect();
    let n = items.len();
    if n < 2 {
        return None;
    }
    let find = |f: Form| items.iter().copied().find(|&i| form(i) == f);
    let count = |f: Form| items.iter().filter(|&&i| form(i) == f).count();
    let raw = |m: Mat| items.contains(&item(Form::Raw, m));
    // (inventions step 12) every frame, haft, mast and stick below is a wooden shaft; a "shaft"
    // of anything else (none can be made now) holds nothing together
    if items.iter().any(|&i| form(i) == Form::Shaft && !woody(mat(i))) {
        return None;
    }
    // (inventions step 12) a point that is to pierce, tip or fasten must be hard enough
    if items.iter().any(|&i| form(i) == Form::Point && !point_ok(i)) {
        return None;
    }
    // hafting: a shaft + a head + a binding cord
    if n == 3 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        if let Some(pt) = find(Form::Point) {
            let pm = mat(pt);
            return o(item(Form::Spear, pm), 0.3 + 0.3 * cut_q(pm));
        }
        if let Some(e) = find(Form::Edge) {
            let em = mat(e);
            if em.props().hard >= 3.0 && em.props().tough + em.props().malleable >= 3.0 {
                return o(item(Form::Axe, em), 0.25 + 0.3 * cut_q(em));
            }
        }
    }
    // a watertight skin lashed over a frame of shafts: a skin boat
    if ctx.water && n == 3 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        if let Some(sk) = items.iter().copied().find(|&i| form(i) == Form::Raw && mat(i).props().seal >= 5.0 && mat(i).props().tough >= 5.0) {
            return o(item(Form::Boat, mat(sk)), 0.06);
        }
    }
    // (v8) a float lashed out to the side of a boat on poles: an outrigger, much harder to capsize
    if ctx.water && n == 3 && count(Form::Boat) == 1 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        let bt = find(Form::Boat)?;
        return o(item(Form::Outrigger, mat(bt)), 0.04);
    }
    // (v8) woven cloth hung from a mast: a sail. (inventions step 12) made of cloth, not of
    // clothing, and stepped only in a plank hull or a ship: a dugout or canoe carries no mast
    // (like the mat sail below; see `sail_usable`)
    if ctx.water && n == 3 && count(Form::Plank) + count(Form::Ship) == 1 && count(Form::Shaft) == 1 {
        if let Some(cl) = items.iter().copied().find(|&i| form(i) == Form::Cloth && cloth_fibre(mat(i))) {
            return o(item(Form::Sail, mat(cl)), 0.05);
        }
    }
    // (inventions step 5b) plaited plant-fibre matting (the basket-maker's weave, opened out)
    // hung from a mast stepped in a plank hull or a ship: a mat sail. Only a big hull carries a
    // mast and a heavy mat; a dugout or canoe cannot (no recipe, and no use: `sail_usable`).
    if ctx.water && n == 3 && count(Form::Plank) + count(Form::Ship) == 1 && count(Form::Shaft) == 1 && items.contains(&item(Form::Vessel, Mat::Fibre)) {
        return o(item(Form::Sail, Mat::Fibre), 0.05);
    }
    // (v8) away from the water, the same hide, frame and lashing make a shield, not a boat
    if n == 3 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        if let Some(sk) = items.iter().copied().find(|&i| form(i) == Form::Raw && mat(i).props().seal >= 5.0 && mat(i).props().tough >= 5.0) {
            return o(item(Form::Shield, mat(sk)), 0.06);
        }
    }
    // (inventions step 12) an upright frame of poles with the warp hung from it, held taut by
    // stone weights tied on with cord: a warp-weighted loom
    if n == 3 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 && raw(Mat::Stone) {
        return o(item(Form::Loom, mat(find(Form::Shaft)?)), 0.08);
    }
    // (inventions step 12) yarn spun from a long, strong fibre and woven on a loom: cloth of
    // that fibre
    if n == 2 && count(Form::Loom) == 1 {
        if let Some(y) = items.iter().copied().find(|&i| form(i) == Form::Cord && cloth_fibre(mat(i))) {
            return o(item(Form::Cloth, mat(y)), 0.2);
        }
    }
    // a springy stick with a cord noose: a snare
    if n == 2 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        let c = find(Form::Cord)?;
        return o(item(Form::Trap, mat(c)), 0.2);
    }
    // bundles of buoyant reeds bound tightly with cord: a reed boat
    if ctx.water && n == 2 && count(Form::Cord) == 1 {
        if let Some(r) = items.iter().copied().find(|&i| form(i) == Form::Raw && mat(i).props().float >= 5.0 && mat(i).props().fibrous >= 8.0) {
            return o(item(Form::Boat, mat(r)), 0.03);
        }
    }
    // (v8) a basket or bag lashed to a pole frame, carried on the back: a carrying frame
    if n == 3 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        if let Some(bag) = items.iter().copied().find(|&i| form(i) == Form::Vessel && mat(i).props().seal < 5.0 && mat(i).props().fibrous >= 5.0) {
            return o(item(Form::Frame, mat(bag)), 0.08);
        }
    }
    // bellows: a tough, airtight bag lashed to a stick, pumped to blow air into a fire
    if n == 3 && count(Form::Shaft) == 1 && count(Form::Cord) == 1 {
        if let Some(bag) = items.iter().copied().find(|&i| form(i) == Form::Vessel && mat(i).props().seal >= 5.0 && mat(i).props().tough >= 5.0) {
            return o(item(Form::Bellows, mat(bag)), 0.04);
        }
    }
    // (inventions step 5) at the water, a plank hull fastened with metal nails or rivets and
    // its seams paid with pitch: a ship (copper, bronze or iron points serve as the nails)
    if ctx.water && n == 3 && count(Form::Plank) == 1 && raw(Mat::Pitch) {
        if items.iter().any(|&i| point_ok(i) && is_metal(mat(i))) {
            return o(item(Form::Ship, mat(find(Form::Plank)?)), 0.2);
        }
    }
    // (inventions step 11b) a prized soft metal (gold) put into a fired pot that stands great
    // heat (a crucible) for melting down; (inventions step 12) any metal that melts in the
    // charcoal furnace: copper, tin, bronze, gold
    if n == 2 {
        if let Some(m) = items.iter().copied().find(|&i| form(i) == Form::Raw && castable(mat(i))) {
            if let Some(pot) = items.iter().copied().find(|&i| i != m && form(i) == Form::Vessel) {
                let pp = mat(pot).props();
                if pp.refractory >= 8.0 && pp.fires_to.is_none() {
                    return o(item(Form::Crucible, mat(m)), 0.3);
                }
            }
        }
    }
    // (inventions) a fired pot whose mouth and seams are sealed with pitch: a sealed jar
    if n == 2 && raw(Mat::Pitch) {
        if let Some(pot) = items.iter().copied().find(|&i| form(i) == Form::Vessel && mat(i) == Mat::Ceramic) {
            return o(item(Form::Jar, mat(pot)), 0.25);
        }
    }
    // (inventions step 10) grain steeped in water and left to work in a sealed jar: ale. Only
    // found where there is water at hand to steep it in
    if ctx.water && n == 2 && raw(Mat::Grain) {
        if items.iter().any(|&i| form(i) == Form::Jar && mat(i) != Mat::Grain) {
            return o(item(Form::Jar, Mat::Grain), 0.2);
        }
    }
    // sewing: hide + cord + a point to pierce it
    if n == 3 && raw(Mat::Hide) && count(Form::Cord) == 1 && count(Form::Point) == 1 {
        return o(item(Form::Garment, Mat::Hide), 0.35);
    }
    // sewing cloth: woven cloth (linen, wool) + thread + a needle. (inventions step 12) cloth
    // first has to be spun and woven (see the loom above); raw fibre is not clothing
    if n == 3 && count(Form::Cord) == 1 && count(Form::Point) == 1 {
        if let Some(fb) = items.iter().copied().find(|&i| form(i) == Form::Cloth && cloth_fibre(mat(i))) {
            return o(item(Form::Garment, mat(fb)), 0.25);
        }
    }
    // (inventions step 8) notches cut with an edge into a stick or a bone: a tally
    if n == 2 && count(Form::Edge) == 1 {
        let e = find(Form::Edge)?;
        if cut_q(mat(e)) > 0.3 {
            if let Some(t) = items.iter().copied().find(|&i| (form(i) == Form::Shaft && mat(i) == Mat::Wood) || i == item(Form::Raw, Mat::Bone)) {
                return o(item(Form::Mark, mat(t)), 0.3);
            }
        }
    }
    // friction fire: rub a shaft against dry wood (tinder helps)
    if items.contains(&item(Form::Shaft, Mat::Wood)) && raw(Mat::Wood) && (n == 2 || raw(Mat::Fibre)) {
        let tinder = if raw(Mat::Fibre) { 0.15 } else { 0.0 };
        return o(item(Form::Fire, Mat::Wood), 0.03 + 0.25 * ctx.dryness + tinder);
    }
    if n == 2 {
        if let Some(f) = find(Form::Fire) {
            let other = items.iter().copied().find(|&i| i != f)?;
            let (ofm, om) = (form(other), mat(other));
            // feed a better fuel to an existing fire
            if ofm == Form::Raw && om.props().fuel_t > mat(f).props().fuel_t {
                return o(item(Form::Fire, om), 0.8);
            }
            // build a heat-holding enclosure around a fire
            // (a closed chamber: easy to pile stones, harder to seal; clay seals best)
            if ofm == Form::Raw && om.props().refractory >= 4.0 && om.props().fuel_t == 0.0 {
                return o(item(Form::Kiln, om), 0.015 * om.props().refractory);
            }
            // a chamber built of already-fired pieces (sherds, bricks) holds far more heat
            if ofm == Form::Vessel && om.props().refractory >= 8.0 && om.props().fires_to.is_none() {
                return o(item(Form::Kiln, om), 0.004 * om.props().refractory);
            }
        }
    }
    None
}

/// (inventions step 13) The smith's hearth: hot iron worked with a hard hammer on an open bed
/// of glowing charcoal that bellows blow to white heat (charcoal + `BELLOWS_HEAT`, past
/// `FORGE_T`). Heat after heat in the charcoal the bar takes up carbon at its skin
/// (carburising), and folded and beaten it spreads through; plunged hot into water it hardens:
/// steel. Needs the charcoal fire (the carbon), the bellows (the heat), a hard hammer and water
/// at hand for the quench; most bars are spoiled (`STEEL_P`). Away from water there is no
/// quench and nothing comes of it. A bar or blade of steel, and a blade of iron, are forged here
/// as in the closed forge (to a blade, to a point). Nothing else is done at the hearth.
fn smith_hearth(t1: Item, t2: Item, fire: Item, ctx: &Ctx) -> Option<Outcome> {
    if t1 == NONE || t2 == NONE || mat(fire).props().fuel_t < 900.0 {
        return None;
    }
    if fire_temp(mat(fire), None) + BELLOWS_HEAT < FORGE_T {
        return None;
    }
    let (mt, hm) = if form(t2) == Form::Hammer { (t1, t2) } else { (t2, t1) };
    if form(hm) != Form::Hammer || mat(hm).props().hard < 5.0 || !hot_worked(mat(mt)) {
        return None;
    }
    match (form(mt), mat(mt)) {
        (Form::Raw, Mat::Iron) => {
            if ctx.water {
                o(item(Form::Raw, Mat::Steel), STEEL_P)
            } else {
                None
            }
        }
        (Form::Raw, m) => o(item(Form::Edge, m), 0.10),
        (Form::Edge, m) => o(item(Form::Point, m), 0.10),
        _ => None,
    }
}

/// extra heat from bellows blowing into a closed charcoal furnace (deg C)
pub const BELLOWS_HEAT: f32 = 250.0;
/// heat at which an iron bloom can be hammered into wrought iron (deg C)
pub const FORGE_T: f32 = 1100.0;
/// (inventions) heat at which resinous wood in a closed pot gives off pitch (deg C)
pub const PITCH_T: f32 = 350.0;

/// (inventions step 7) Chance per try that wrought iron comes out as true steel: the carbon is
/// taken up unevenly and most bars are spoiled or merely iron again, so it takes many heats.
/// (inventions step 13) The try is a smith's: the bar is worked under the hammer, heat after
/// heat, in the open charcoal hearth blown by bellows, and quenched (see `heat`).
pub const STEEL_P: f32 = 0.03;

fn heat(inp: &Inputs, ctx: &Ctx) -> Option<Outcome> {
    let (t1, t2, fire, kiln) = (inp[0], inp[1], inp[2], inp[3]);
    if fire == NONE || form(fire) != Form::Fire || t1 == NONE {
        return None;
    }
    // (inventions step 13) the smith's hearth: an open bed of charcoal with bellows blowing
    // into it, in the slot of the closed chamber. Only the hammer is worked there (below)
    if kiln != NONE && form(kiln) == Form::Bellows {
        return smith_hearth(t1, t2, fire, ctx);
    }
    if kiln != NONE && form(kiln) != Form::Kiln {
        return None;
    }
    let fuel = mat(fire);
    let lining = if kiln == NONE { None } else { Some(mat(kiln)) };
    let mut temp = fire_temp(fuel, lining);
    // reducing atmosphere: carbon-rich fuel burning in a closed chamber
    let reducing = lining.is_some() && fuel.props().fuel_t >= 900.0;
    // bellows worked into a closed charcoal furnace force the draught: a few hundred degrees
    // more. In an open or wood fire the blast only fans the flames
    let (t1, t2) = match (t1 != NONE && form(t1) == Form::Bellows, t2 != NONE && form(t2) == Form::Bellows) {
        (false, false) => (t1, t2),
        (true, true) => return None,
        (b1, _) => {
            if !reducing {
                return None;
            }
            temp += BELLOWS_HEAT;
            (if b1 { t2 } else { t1 }, NONE)
        }
    };
    if t1 == NONE {
        return None;
    }
    // forging: a hot iron bloom hammered on a stone to squeeze out the slag and weld the sponge
    // into solid, workable wrought iron (needs a hearth that holds the heat, and many tries)
    if t2 != NONE {
        // (v8) brine boiled down in a fired pot until the salt is left
        let (br, pot) = if t1 == item(Form::Raw, Mat::Brine) { (t1, t2) } else { (t2, t1) };
        if br == item(Form::Raw, Mat::Brine) {
            if form(pot) == Form::Vessel && mat(pot) == Mat::Ceramic {
                return o(item(Form::Raw, Mat::Salt), 0.3);
            }
            return None;
        }
        // (inventions) resinous wood heated slowly in a closed pot, starved of air, sweats out
        // its tar (about 350-450 C: any wood fire is hot enough). (inventions step 12) only
        // resinous wood (pine, birch): ordinary wood gives no pitch
        let resinous = |i: Item| form(i) == Form::Raw && mat(i).props().resin >= 5.0;
        let (wd, pot) = if resinous(t1) { (t1, t2) } else { (t2, t1) };
        if resinous(wd) && form(pot) == Form::Vessel && mat(pot) == Mat::Ceramic {
            return if temp >= PITCH_T { o(item(Form::Raw, Mat::Pitch), 0.2) } else { None };
        }
        // (inventions step 11b) casting: a charged crucible in a closed furnace hot enough to
        // melt its metal (gold 1064 C: a charcoal fire in a kiln, the furnace that smelts
        // copper), poured into a fired mould: a ring. Never in an open fire
        // (inventions step 12) a blade cast of bronze or copper; molten tin makes nothing that
        // holds an edge
        let (cr, md) = if form(t1) == Form::Crucible { (t1, t2) } else { (t2, t1) };
        if form(cr) == Form::Crucible {
            let (cm, mp) = (mat(cr), mat(md).props());
            if form(md) == Form::Mould && mp.refractory >= 8.0 && mp.fires_to.is_none() && reducing && temp >= cm.props().melt {
                if cm.props().prized > 0.0 {
                    return o(item(Form::Ornament, cm), 0.25);
                }
                if cm.props().hard >= EDGE_HARD {
                    return o(item(Form::Edge, cm), 0.25);
                }
                return None;
            }
            // (inventions step 12) bronze from the metals: tin stirred into copper molten in a
            // crucible (or copper into tin), in a closed charcoal furnace hot enough to melt
            // copper
            let other = if cr == t1 { t2 } else { t1 };
            let pair = (cm, mat(other));
            if form(other) == Form::Raw && matches!(pair, (Mat::Copper, Mat::Tin) | (Mat::Tin, Mat::Copper)) {
                if reducing && temp >= Mat::Copper.props().melt {
                    return o(item(Form::Raw, Mat::Bronze), 0.3);
                }
            }
            return None;
        }
        // (inventions step 12) iron and steel are forged hot: a bar heated in the forge to
        // white heat and beaten out on a stone into a blade, or a blade drawn out into a point.
        // Never cold, and never melted (they do not melt in any furnace here)
        let (mt, hm) = if form(t2) == Form::Hammer { (t1, t2) } else { (t2, t1) };
        if form(hm) == Form::Hammer && matches!(form(mt), Form::Raw | Form::Edge) && hot_worked(mat(mt)) {
            if mat(hm).props().hard >= 5.0 && temp >= FORGE_T {
                let to = if form(mt) == Form::Raw { Form::Edge } else { Form::Point };
                return o(item(to, mat(mt)), 0.10);
            }
            return None;
        }
        let (bl, hm) = if form(t1) == Form::Bloom { (t1, t2) } else { (t2, t1) };
        if form(bl) == Form::Bloom && form(hm) == Form::Hammer {
            if mat(hm).props().hard >= 5.0 && temp >= FORGE_T {
                return o(item(Form::Raw, mat(bl)), 0.10);
            }
            return None;
        }
    }
    if t2 == NONE {
        let (f, m) = (form(t1), mat(t1));
        let p = m.props();
        if f == Form::Raw && m.is_food() {
            return o(item(Form::Cooked, m), 0.9);
        }
        // firing a moulded clay vessel (or, inventions step 8, a clay token; step 11b, a mould)
        if f == Form::Vessel || f == Form::Mark || f == Form::Mould {
            if let Some(to) = p.fires_to {
                if temp >= p.fire_t {
                    let pr = if lining.is_some() { 0.8 } else { 0.35 };
                    return o(item(f, to), pr);
                }
            }
        }
        if f == Form::Raw {
            // (inventions step 7) wrought iron left long in a blown charcoal furnace took up
            // carbon and was quenched into steel. (inventions step 13) no longer: a bar heated
            // on its own is just hot iron. Steel is a smith's work under the hammer at the
            // charcoal hearth (`smith_hearth`); the old recipe is retired on load
            // wood heated without enough air chars instead of burning
            if m == Mat::Wood && lining.is_some() {
                return o(item(Form::Raw, Mat::Charcoal), 0.15);
            }
            // smelting
            if let Some(to) = p.reduces_to {
                if reducing && temp >= p.reduce_t {
                    // iron only reduces under a forced blast, and never melts: it comes out as a
                    // spongy bloom that still has to be forged; most charges are mere slag
                    if to == Mat::Iron {
                        return o(item(Form::Bloom, to), 0.05);
                    }
                    return o(item(Form::Raw, to), 0.12);
                }
            }
        }
        return None;
    }
    // two targets: alloying (or co-smelting) in a closed chamber
    let (a, b) = (mat(t1), mat(t2));
    if form(t1) != Form::Raw || form(t2) != Form::Raw || lining.is_none() {
        return None;
    }
    let metal_or_ore = |m: Mat, metal: Mat, ore: Mat| m == metal || m == ore;
    let cu = metal_or_ore(a, Mat::Copper, Mat::CopperOre) && metal_or_ore(b, Mat::Tin, Mat::TinOre)
        || metal_or_ore(b, Mat::Copper, Mat::CopperOre) && metal_or_ore(a, Mat::Tin, Mat::TinOre);
    if cu {
        let ores = [a, b].iter().any(|m| m.props().reduces_to.is_some());
        // (inventions step 12) the two metals are no longer just fired together in a kiln: they
        // have to be melted together in a crucible (above). Co-smelting ores is unchanged
        if !ores {
            return None;
        }
        if reducing && temp >= Mat::CopperOre.props().reduce_t {
            return o(item(Form::Raw, Mat::Bronze), 0.08);
        }
    }
    None
}
