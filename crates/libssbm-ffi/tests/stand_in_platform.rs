//! The no-assets entry platform (a flat cylinder) grows and shrinks with the entry animation and
//! flashes with its colour program, through the native calls the host makes. No disc needed.
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

/// How far below the feet the platform's lowest vertex is, and its widest reach.
fn extent(view: *mut View) -> (f32, f32) {
    let mut count = 0u32;
    let positions = unsafe { ssbm_view_positions(view, &mut count) };
    let all = unsafe { std::slice::from_raw_parts(positions, count as usize) };
    (-all.iter().map(|v| v.y).fold(f32::MAX, f32::min), all.iter().map(|v| v.x.abs()).fold(0.0, f32::max))
}
fn first_color(view: *mut View) -> [f32; 4] {
    let mut count = 0u32;
    let colors = unsafe { ssbm_view_colors(view, &mut count) };
    unsafe { *colors }
}

#[test]
fn the_cylinder_platform_follows_the_entry_and_flashes() {
    let empty = CString::new("").unwrap();
    let host = HostTable { user: std::ptr::null_mut(), collide: Some(collide), event: Some(event) };
    let sim = unsafe { ssbm_sim_new_fighter(empty.as_ptr(), V3::default(), 0) };
    assert!(!sim.is_null());
    assert_eq!(unsafe { ssbm_sim_begin_arrival(sim, empty.as_ptr()) }, 1);
    let platform = unsafe { ssbm_entry_platform_new(empty.as_ptr()) };
    assert!(!platform.is_null(), "no assets must still give a platform");
    let pad = Pad { movement: V3::default(), aim: V3 { x: 1.0, y: 0.0, z: 0.0 }, buttons: 0, allowed: 31 };
    let (mut thickest, mut flashes, mut calm, mut updates) = (0.0f32, 0, 0, 0);
    for _ in 0..400 {
        unsafe { ssbm_sim_step(sim, &host, &pad, 1.0 / 60.0) };
        if unsafe { ssbm_entry_platform_update(platform, sim) } != 1 {
            break;
        }
        updates += 1;
        thickest = thickest.max(extent(platform).0);
        if first_color(platform) == [1., 1., 1., 1.] { flashes += 1 } else { calm += 1 }
    }
    assert!(updates > 20, "the platform should be updated while the entry runs ({updates})");
    assert!(thickest > 1.0, "the cylinder should thicken ({thickest})");
    let (_, reach) = extent(platform);
    assert!(reach > 8.0, "a wide platform, not a body ({reach})");
    assert!(flashes > 5 && calm > 5, "flashing: {flashes} white ticks, {calm} resting");
    unsafe {
        ssbm_view_free(platform);
        ssbm_sim_free(sim);
    }
}
