//! Unit tests for the diffusion step (D1-D6, docs/REPORT-diffusion.md section 8).
use crate::materials::*;
use crate::{Config, Params, World};

fn world() -> World {
    World::new(Config { seed: 7, map_w: 80, map_h: 50, ..Default::default() }, Params::default())
}

/// D1: hide and bone are at hand in a season the band hunted big game, wherever it is; not
/// otherwise (they have no deposit). Other materials are unchanged.
#[test]
fn d1_hide_and_bone_at_hand_for_hunters() {
    let w = world();
    let b = &w.bands[0];
    let ci = b.cell(w.map.w);
    for mm in [Mat::Hide, Mat::Bone] {
        assert!(w.mat_at_hand(b, ci, mm, 3.0), "{mm:?} after a hunting season");
        assert!(!w.mat_at_hand(b, ci, mm, 0.5), "{mm:?} without big game");
    }
    assert_eq!(w.mat_at_hand(b, ci, Mat::Wood, 3.0), w.map.k_wood[ci] > 2.0);
    assert!(!w.mat_at_hand(b, ci, Mat::Brine, 3.0));
}

/// D1: a hunting band that needs hide keeps a stock of it: the season's gain (0.01 per unit
/// hunted) covers the plain wear (0.02) once the band hunts 2+, with no extra wear.
#[test]
fn d1_hide_stock_holds_for_hunters() {
    let mut w = world();
    let mut kept = 0;
    let mut hunters = 0;
    for _ in 0..4 * 40 {
        w.step();
    }
    for b in &w.bands {
        let h = b.d.harvest;
        if h[1] + 0.3 * h[3] >= 2.5 {
            hunters += 1;
            if b.stock[Mat::Hide as usize] > 0.05 {
                kept += 1;
            }
        }
    }
    assert!(hunters > 0, "some bands hunt");
    assert_eq!(kept, hunters, "every band hunting 2.5+ holds hide");
}
