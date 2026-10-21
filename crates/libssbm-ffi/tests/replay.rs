//! Multiplayer replay (skipped without the player's extracted cache): a shadow sim fed the owner's
//! recorded calls ends each batch where the owner's sim is, in the same action and frame, for every
//! fighter, over a scripted run of walking, jumping, jabs, a smash, specials and a hurt.
use ssbm::*;
use std::ffi::{c_void, CString};

unsafe extern "C" fn collide(_: *mut c_void, from: *const V3, to: *const V3, out: *mut Collision) {
    // A flat floor at y = 0.
    let (from, to) = unsafe { (*from, *to) };
    let _ = from;
    let landed = to.y <= 0.0;
    unsafe {
        *out = Collision {
            position: V3 { x: to.x, y: if landed { 0.0 } else { to.y }, z: to.z },
            grounded: landed as u32,
            ceiling: 0,
        };
    }
}
unsafe extern "C" fn event(_: *mut c_void, _: *const NativeEvent) {}

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

const ATTACK: u32 = 2;
const SPECIAL: u32 = 4;
const JUMP: u32 = 1;

fn pad(x: f32, buttons: u32) -> Pad {
    Pad {
        movement: V3 { x, y: 0.0, z: 0.0 },
        aim: V3 { x: 1.0, y: 0.0, z: 0.0 },
        buttons,
        allowed: 31,
    }
}

/// What the owner pressed on tick `t`: a repeatable script that exercises most of the sim.
fn script(t: u32) -> Pad {
    match t {
        0..=29 => pad(0.0, 0),
        30..=89 => pad(1.0, 0),
        90..=91 => pad(0.0, JUMP),
        92..=151 => pad(0.4, 0),
        152 => pad(0.0, ATTACK),
        158 => pad(0.0, ATTACK),
        164 => pad(0.0, ATTACK),
        200..=205 => pad(1.0, ATTACK),
        260 => pad(0.0, SPECIAL),
        400..=402 => pad(0.0, JUMP),
        _ => pad(0.0, 0),
    }
}

fn state(h: *mut Handle) -> State {
    let mut s = State::default();
    assert_eq!(unsafe { ssbm_sim_state(h, &mut s) }, 1);
    s
}

#[test]
fn a_shadow_sim_follows_the_owners_recorded_calls_for_every_fighter() {
    let dir = dir();
    if !libssbm_assets::complete(&dir) {
        eprintln!("SKIP: disc cache absent");
        return;
    }
    let cdir = CString::new(dir.to_str().unwrap()).unwrap();
    let host = HostTable { user: std::ptr::null_mut(), collide: Some(collide), event: Some(event) };
    for kind in 0..8u32 {
        let owner = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
        if owner.is_null() {
            eprintln!("SKIP: fighter {kind} absent");
            continue;
        }
        let shadow = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
        assert!(!shadow.is_null());
        assert_eq!(unsafe { ssbm_sim_record(owner, 1) }, 1);
        let tuning = Tuning { speed_scale: 1.37, jump_scale: 1.0, extra_jumps: 0, hold_charge: 0, smash_max_frames: 60 };
        let mut bytes = vec![0u8; 1 << 14];
        let mut worst = 0.0f32;
        // Effect cues the owner's host read after each of its steps, and the shadow's after each replay.
        let (mut owner_cues, mut shadow_cues) = (0u32, 0u32);
        for batch in 0..(600 / 3) {
            for i in 0..3 {
                let t = batch * 3 + i;
                let p = script(t);
                unsafe {
                    ssbm_sim_tuning(owner, tuning);
                    ssbm_sim_facing(owner, V3 { x: 1.0, y: 0.0, z: 0.0 });
                    if t == 300 {
                        ssbm_sim_hurt_scaled(owner, 0.3);
                    }
                    ssbm_sim_step(owner, &host, &p, 1.0 / 60.0);
                    owner_cues += unsafe { ssbm_sim_gfx(owner, std::ptr::null_mut(), 0) };
                }
            }
            let n = unsafe { ssbm_sim_ops_take(owner, bytes.as_mut_ptr(), bytes.len() as u32) };
            assert!(n > 0 && (n as usize) <= bytes.len(), "log of {n} bytes");
            let applied = unsafe { ssbm_sim_replay(shadow, &host, bytes.as_ptr(), n) };
            assert_ne!(applied, u32::MAX, "replay rejected the log");
            shadow_cues += unsafe { ssbm_sim_gfx(shadow, std::ptr::null_mut(), 0) };
            let (a, b) = (state(owner), state(shadow));
            let d = ((a.position.x - b.position.x).powi(2) + (a.position.y - b.position.y).powi(2) + (a.position.z - b.position.z).powi(2)).sqrt();
            worst = worst.max(d);
            assert!(d < 1.5, "fighter {kind} batch {batch}: shadow {d} units away");
            assert_eq!(a.move_id, b.move_id, "fighter {kind} batch {batch}: different moves");
            assert_eq!(a.grounded, b.grounded, "fighter {kind} batch {batch}");
            assert_eq!(a.frame, b.frame, "fighter {kind} batch {batch}: different tick counts");
        }
        assert_eq!(owner_cues, shadow_cues, "fighter {kind}: the shadow lost effect cues");
        let end = state(owner);
        assert!(end.position.x.abs() > 5.0, "fighter {kind} never walked ({})", end.position.x);
        assert!(end.frame >= 600, "fighter {kind} ran {} ticks", end.frame);
        eprintln!("fighter {kind}: shadow stayed within {worst} units (owner ended at x = {})", end.position.x);
        unsafe {
            ssbm_sim_free(owner);
            ssbm_sim_free(shadow);
        }
    }
}

#[test]
fn a_log_that_is_not_one_is_refused_and_a_big_one_is_too() {
    let dir = dir();
    if !libssbm_assets::complete(&dir) {
        eprintln!("SKIP: disc cache absent");
        return;
    }
    let cdir = CString::new(dir.to_str().unwrap()).unwrap();
    let host = HostTable { user: std::ptr::null_mut(), collide: Some(collide), event: Some(event) };
    let h = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), 1) };
    assert!(!h.is_null());
    let junk = [200u8, 1, 2, 3];
    assert_eq!(unsafe { ssbm_sim_replay(h, &host, junk.as_ptr(), junk.len() as u32) }, u32::MAX);
    let truncated = [1u8, 0, 0];
    assert_eq!(unsafe { ssbm_sim_replay(h, &host, truncated.as_ptr(), truncated.len() as u32) }, u32::MAX);
    assert_eq!(unsafe { ssbm_sim_replay(h, &host, junk.as_ptr(), 1 << 20) }, u32::MAX);
    // Not recording: nothing to take.
    assert_eq!(unsafe { ssbm_sim_ops_take(h, std::ptr::null_mut(), 0) }, 0);
    unsafe { ssbm_sim_free(h) };
}
