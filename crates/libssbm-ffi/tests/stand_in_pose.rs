//! The no-assets body (a cylinder) tips with the shared strike pose while the authored jab runs,
//! through the same native view update the host calls. No disc needed.
use ssbm::*;
use std::ffi::{c_void, CString};

unsafe extern "C" fn collide(_: *mut c_void, _: *const V3, to: *const V3, out: *mut Collision) {
    let to = unsafe { *to };
    let landed = to.y <= 0.0;
    unsafe {
        *out = Collision { position: V3 { x: to.x, y: if landed { 0.0 } else { to.y }, z: to.z }, grounded: landed as u32, ceiling: 0 };
    }
}
unsafe extern "C" fn event(_: *mut c_void, _: *const NativeEvent) {}

/// The top ring's mean forward (z) offset from the body's axis, in the view's model units (the
/// body's first batch holds the ring as alternating bottom and top vertices).
fn head_lean(view: *mut View) -> f32 {
    let mut count = 0u32;
    let positions = unsafe { ssbm_view_positions(view, &mut count) };
    let all = unsafe { std::slice::from_raw_parts(positions, count as usize) };
    let top: Vec<f32> = all[..24].iter().skip(1).step_by(2).map(|v| v.z).collect();
    top.iter().sum::<f32>() / top.len() as f32
}

#[test]
fn the_cylinder_body_tips_forward_through_the_jab_and_stands_up_again() {
    let empty = CString::new("").unwrap();
    let host = HostTable { user: std::ptr::null_mut(), collide: Some(collide), event: Some(event) };
    let sim = unsafe { ssbm_sim_new_fighter(empty.as_ptr(), V3::default(), 0) };
    let view = unsafe { ssbm_view_new(empty.as_ptr(), 0, 0) };
    assert!(!sim.is_null() && !view.is_null());
    let pad = |buttons| Pad { movement: V3::default(), aim: V3 { x: 1.0, y: 0.0, z: 0.0 }, buttons, allowed: 31 };
    let mut lean = Vec::new();
    for t in 0..60 {
        let p = pad(if t == 30 { 2 } else { 0 });
        unsafe {
            ssbm_sim_facing(sim, V3 { x: 1.0, y: 0.0, z: 0.0 });
            ssbm_sim_step(sim, &host, &p, 1.0 / 60.0);
            assert_eq!(ssbm_view_update(view, sim, 1.0 / 60.0), 1);
        }
        lean.push(head_lean(view));
    }
    let upright = lean[10];
    let peak = lean.iter().cloned().fold(f32::MIN, f32::max);
    // A 16-high cylinder tipped 45 degrees about its middle: the head moves 8 * sin(45) forward.
    assert!((peak - upright - 8.0 * std::f32::consts::FRAC_PI_4.sin()).abs() < 0.1, "peak {peak} upright {upright}");
    assert!((lean[59] - upright).abs() < 1e-3, "back upright: {} vs {upright}", lean[59]);
    unsafe {
        ssbm_view_free(view);
        ssbm_sim_free(sim);
    }
}
