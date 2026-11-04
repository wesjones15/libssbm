//! A fighter view's VR arms (skipped without the player's extracted cache): with them on, the
//! posed body is followed by one copy a side, and each copy's kept vertices sit about its
//! hand (arm space puts the hand's middle at the origin) and is a minority of the body, for every
//! fighter in the cache.
use ssbm::*;
use std::ffi::CString;

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

#[test]
fn every_fighter_draws_both_arms_about_the_hand() {
    let dir = dir();
    if !dir.join("PlCo.dat").is_file() {
        eprintln!("SKIP: no Melee cache in {}", dir.display());
        return;
    }
    let path = CString::new(dir.to_str().unwrap()).unwrap();
    let wait = CString::new("Wait").unwrap();
    for kind in 0..9u32 {
        let view = unsafe { ssbm_view_new(path.as_ptr(), kind, 0) };
        if view.is_null() {
            eprintln!("SKIP: fighter {kind} not extracted");
            continue;
        }
        assert_eq!(unsafe { ssbm_view_pose(view, wait.as_ptr(), 0.) }, 1);
        let mut body = 0u32;
        unsafe { ssbm_view_positions(view, &mut body) };
        let arms = unsafe { ssbm_view_set_vr_arms(view, path.as_ptr(), kind, 0, 1) };
        assert_eq!(arms, 2, "fighter {kind}");
        let mut sides = [9u8; 2];
        assert_eq!(unsafe { ssbm_view_vr_arm_sides(view, sides.as_mut_ptr(), 2) }, 2);
        sides.sort();
        assert_eq!(sides, [0, 1], "fighter {kind}");
        assert_eq!(unsafe { ssbm_view_pose(view, wait.as_ptr(), 10.) }, 1);
        let mut count = 0u32;
        let positions = unsafe { ssbm_view_positions(view, &mut count) };
        assert_eq!(count, body * 3, "fighter {kind}");
        let all = unsafe { std::slice::from_raw_parts(positions, count as usize) };
        let (lo, hi) = all[..body as usize].iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
        let height = hi - lo;
        for side in 0..2usize {
            let copy = &all[body as usize * (1 + side)..body as usize * (2 + side)];
            let kept: Vec<f32> = copy
                .iter()
                .map(|p| (p.x * p.x + p.y * p.y + p.z * p.z).sqrt())
                .filter(|&d| d > 0.)
                .collect();
            assert!(kept.len() > 10 && kept.len() < body as usize / 2, "fighter {kind} side {side}: {} of {body} kept", kept.len());
            let far = kept.iter().cloned().fold(0f32, f32::max);
            // A forearm and hand (with whatever it holds: Link's sword), not the whole body.
            assert!(far < height * 0.8, "fighter {kind} side {side}: reaches {far} of {height}");
        }
        assert_eq!(unsafe { ssbm_view_set_vr_arms(view, path.as_ptr(), kind, 0, 0) }, 0);
        let mut off = 0u32;
        unsafe { ssbm_view_positions(view, &mut off) };
        assert_eq!(off, body, "fighter {kind}");
        unsafe { ssbm_view_free(view) };
    }
}
