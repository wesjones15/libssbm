//! Move presentation through the C ABI the hosts use (skipped without the player's extracted cache):
//! an effect a move binds to the fighter ends with the move, the select-screen intro of every
//! fighter plays its sounds and effects, and every special of the eight fighters is audible.
use ssbm::*;
use std::ffi::{c_void, CString};

#[allow(improper_ctypes)]
unsafe extern "C" {
    /// projectile.rs: shared fighter hit feedback.
    fn ssbm_hit_feedback(audio: *mut Handle, element: u32, kind: u32, severity: u32) -> u32;
}
const FIGHTERS: [&str; 8] = ["samus", "fox", "luigi", "pikachu", "captain", "link", "yoshi", "donkey"];
const NEUTRAL_B: u32 = 256;
const SIDE_B: u32 = 512;
const UP_B: u32 = 1024;
const DOWN_B: u32 = 2048;
/// ssbm_sim_gfx: bit 17 of `move_id` is "destroyed when the motion next changes".
const STOP: u32 = 1 << 17;
/// effect::DESTROY_ALL, efLib_DestroyAll.
const DESTROY_ALL: u32 = 0xFFFF;
const FORWARD: V3 = V3 { x: 0., y: 0., z: 1. };
const UP: V3 = V3 { x: 0., y: 1., z: 0. };

struct Events(Vec<(u32, u32, f32)>);
unsafe extern "C" fn collide(_: *mut c_void, _from: *const V3, to: *const V3, out: *mut Collision) {
    let to = unsafe { *to };
    let landed = to.y <= 0.0;
    unsafe {
        *out = Collision { position: V3 { x: to.x, y: if landed { 0.0 } else { to.y }, z: to.z }, grounded: landed as u32, ceiling: 0 };
    }
}
unsafe extern "C" fn event(user: *mut c_void, e: *const NativeEvent) {
    if !user.is_null() {
        let e = unsafe { *e };
        unsafe { &mut *(user as *mut Events) }.0.push((e.kind, e.move_id, e.strength));
    }
}
fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}
fn cdir() -> Option<CString> {
    let dir = dir();
    libssbm_assets::complete(&dir).then(|| CString::new(dir.to_str().unwrap()).unwrap())
}

/// One fighter on a flat floor, stepped at 60 Hz the way the hosts do.
struct Rig {
    h: *mut Handle,
    host: HostTable,
    events: Box<Events>,
    kind: u32,
    cdir: CString,
}
/// What a step produced.
struct Tick {
    state: State,
    serial: u64,
    /// (effect id, bone, stop bit, strength)
    gfx: Vec<(u32, u32, bool, f32)>,
    /// Sum of |sample| of the next 1/60 s of the fighter's mixer.
    audio: f32,
}
impl Rig {
    fn new(cdir: &CString, kind: u32, y: f32) -> Option<Rig> {
        let h = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
        if h.is_null() {
            eprintln!("SKIP: fighter {kind} absent");
            return None;
        }
        let mut events = Box::new(Events(Vec::new()));
        let host = HostTable { user: &mut *events as *mut Events as *mut c_void, collide: Some(collide), event: Some(event) };
        if y > 0. {
            unsafe { ssbm_sim_place(h, V3 { x: 0., y, z: 0. }, 0) };
        }
        Some(Rig { h, host, events, kind, cdir: cdir.clone() })
    }
    fn step(&mut self, buttons: u32, aim: V3) -> Tick {
        let pad = Pad { movement: V3::default(), aim, buttons, allowed: 31 };
        unsafe { ssbm_sim_step(self.h, &self.host, &pad, 1. / 60.) };
        let mut state = State::default();
        unsafe { ssbm_sim_state(self.h, &mut state) };
        let n = unsafe { ssbm_sim_gfx(self.h, std::ptr::null_mut(), 0) };
        let mut raw = vec![NativeEvent { kind: 0, move_id: 0, strength: 0., position: V3::default(), direction: V3::default() }; n as usize];
        if n > 0 {
            unsafe { ssbm_sim_gfx(self.h, raw.as_mut_ptr(), n) };
        }
        let mut pcm = vec![0f32; 2 * 534];
        unsafe { ssbm_sim_audio(self.h, pcm.as_mut_ptr(), 534) };
        Tick {
            state,
            serial: unsafe { ssbm_sim_motion_serial(self.h) },
            gfx: raw.iter().map(|e| (e.kind, e.move_id & 0xFFFF, e.move_id & STOP != 0, e.strength)).collect(),
            audio: pcm.iter().map(|v| v.abs()).sum(),
        }
    }
    fn idle(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.step(0, FORWARD);
        }
    }
    /// The duration in frames of a model effect's view, 0 when it has none.
    fn model_lifetime(&self, model: u32) -> f32 {
        let view = unsafe { ssbm_effect_new(self.cdir.as_ptr(), self.kind, model) };
        assert!(!view.is_null(), "effect model {model}");
        let life = unsafe { ssbm_object_duration(view) };
        unsafe { ssbm_view_free(view) };
        life
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        unsafe { ssbm_sim_free(self.h) };
    }
}

/// Runs one special from the ground or the air for `ticks` and returns every tick.
fn special(cdir: &CString, kind: u32, button: u32, air: bool, aim: V3, ticks: u32) -> Option<Vec<Tick>> {
    let mut rig = Rig::new(cdir, kind, if air { 150. } else { 0. })?;
    unsafe { ssbm_sim_jump_height(rig.h, 25.) };
    rig.idle(5);
    let mut out = Vec::new();
    for t in 0..ticks {
        out.push(rig.step(if t < 3 { button } else { 0 }, aim));
    }
    Some(out)
}

#[test]
fn a_model_effect_that_never_ends_by_itself_is_bound_to_its_move() {
    // efLib_Create: a descriptor lifetime of 0 means the effect lives until efLib_DestroyAll. Every
    // joint-bound model effect of that kind a special spawns must therefore carry the stop bit
    // (the source tags it x2219_b0), or it would stay on the fighter for good.
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    let mut checked = 0;
    for kind in 0..8u32 {
        let Some(rig) = Rig::new(&cdir, kind, 0.) else { continue };
        for (button, air) in [NEUTRAL_B, SIDE_B, UP_B, DOWN_B].into_iter().flat_map(|b| [(b, false), (b, true)]) {
            let Some(run) = special(&cdir, kind, button, air, FORWARD, 200) else { continue };
            for tick in &run {
                for &(id, _, stop, _) in &tick.gfx {
                    for target in [ssbm_effect_target(id), ssbm_effect_companion(id)] {
                        if !(target < 0 && target > i32::MIN + 1000) {
                            continue;
                        }
                        let model = (-target - 1) as u32;
                        if ssbm_effect_flags(id) & 1 == 0 || rig.model_lifetime(model) > 0. || ssbm_effect_lifetime(id) > 0. {
                            continue;
                        }
                        checked += 1;
                        assert!(stop, "{} effect {id} (model {model}, lifetime 0, joint-bound) lasts forever: it needs the stop bit", FIGHTERS[kind as usize]);
                    }
                }
            }
        }
    }
    assert!(checked > 0, "the Fire Fox flame is such an effect");
}

#[test]
fn fire_fox_flame_is_destroyed_when_the_launch_ends() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    let Some(run) = special(&cdir, 1, UP_B, false, UP, 220) else { return };
    let spawn = run.iter().position(|t| t.gfx.iter().any(|g| g.0 == 1164)).expect("Fire Fox launch makes its flame");
    let flame = run[spawn].gfx.iter().find(|g| g.0 == 1164).unwrap();
    assert!(flame.2, "the launch flame is destroyed with the motion (x2219_b0)");
    // The motion changes again when the launch ends, and the move itself ends after it.
    let end = run[spawn..].iter().position(|t| t.serial != run[spawn].serial).expect("the launch ends");
    assert!(end > 5, "the flame lasts through the launch, ended after {end}");
    assert!(run[spawn + end..].iter().any(|t| t.state.move_id == 0), "Fire Fox is over after its fall");
}

#[test]
fn falcon_kick_flame_sits_on_the_kicking_foot_and_ends_at_the_second_flag() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    // Ground: the right foot (RFootJA), made on the clip's first throw flag, destroyed on its second.
    let Some(run) = special(&cdir, 4, DOWN_B, false, FORWARD, 120) else { return };
    let made: Vec<usize> = run.iter().enumerate().filter(|(_, t)| t.gfx.iter().any(|g| g.0 == 1168)).map(|(i, _)| i).collect();
    assert_eq!(made.len(), 1, "one flame per kick");
    let flame = *run[made[0]].gfx.iter().find(|g| g.0 == 1168).unwrap();
    assert!(flame.2, "the flame is bound to the kick");
    let ended = run.iter().enumerate().skip(made[0]).find(|(_, t)| t.gfx.iter().any(|g| g.0 == DESTROY_ALL)).map(|(i, _)| i).expect("the kick's second flag destroys the flame");
    assert!(ended - made[0] >= 20, "the flame stays on the foot through the kick ({} ticks)", ended - made[0]);
    // The clip raises its first flag on frame 1: the flame is not made at the very start of the move.
    assert!(made[0] <= 8);
    // Air: the other foot, tilted by the disc's angle.
    let air = special(&cdir, 4, DOWN_B, true, FORWARD, 120).unwrap();
    let tick = air.iter().find(|t| t.gfx.iter().any(|g| g.0 == 1168)).expect("air kick makes its flame");
    let flame_air = *tick.gfx.iter().find(|g| g.0 == 1168).unwrap();
    assert_ne!(flame_air.1, flame.1, "the air kick's flame is on the other foot");
    assert!(flame_air.3 > 0.1 && flame_air.3 < 1.6, "tilt {} radians", flame_air.3);
    assert_eq!(flame.3, 0., "the ground kick's flame is not tilted");
}

#[test]
fn a_hit_or_a_stun_ends_the_fighters_bound_effects() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    for hurt in [true, false] {
        let Some(mut rig) = Rig::new(&cdir, 1, 0.) else { return };
        rig.idle(5);
        let mut started = false;
        for t in 0..400 {
            let tick = rig.step(if t < 3 { UP_B } else { 0 }, UP);
            if tick.gfx.iter().any(|g| g.0 == 1164) {
                started = true;
                break;
            }
        }
        assert!(started);
        if hurt {
            unsafe { ssbm_sim_hurt(rig.h) }
        } else {
            unsafe { ssbm_sim_interrupt(rig.h) }
        }
        let tick = rig.step(0, UP);
        assert!(tick.gfx.iter().any(|g| g.0 == DESTROY_ALL), "{} must destroy the fighter's effects", if hurt { "a hit" } else { "a stun" });
    }
}

/// The select screen's intro as MeleeSelectSource runs it: place, one idle step, the up special
/// pressed for a step, then released until the fighter is grounded again.
fn select_intro(cdir: &CString, kind: u32) -> Option<Vec<Tick>> {
    let mut rig = Rig::new(cdir, kind, 0.)?;
    unsafe { ssbm_sim_select_preview(rig.h) };
    unsafe { ssbm_sim_place(rig.h, V3::default(), 0) };
    let aim = if kind == 1 { UP } else { FORWARD };
    rig.step(0, aim);
    let mut out = vec![rig.step(UP_B, aim)];
    for _ in 0..600 {
        let tick = rig.step(0, aim);
        let done = tick.state.move_id != 23 && tick.state.grounded != 0;
        out.push(tick);
        if done {
            break;
        }
    }
    Some(out)
}

#[test]
fn the_select_intro_plays_the_up_special_voice_and_whooshes() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    // Falcon Dive's voice is on frame 14 of its clip, Spinning Kong's whooshes from frame 19: the
    // preview has to still be in the move then (it ended after 12 ticks with no host jump height).
    for (kind, name, first_tick) in [(4u32, "captain", 14usize), (7, "donkey", 19)] {
        let Some(run) = select_intro(&cdir, kind) else { continue };
        assert!(run.len() > first_tick + 20, "{name}: the intro lasts {} ticks", run.len());
        let late: f32 = run[first_tick..first_tick + 15].iter().map(|t| t.audio).sum();
        assert!(late > 50., "{name}: the intro is silent after its first frames ({late})");
        let peak = run.iter().map(|t| t.state.position.y).fold(0f32, f32::max);
        assert!(peak > 15., "{name}: rises only {peak} units");
    }
}

#[test]
fn the_select_intro_ends_every_effect_it_started_with_the_move() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    for kind in 0..8u32 {
        let name = FIGHTERS[kind as usize];
        let Some(run) = select_intro(&cdir, kind) else { continue };
        let last_move = run.iter().rposition(|t| t.state.move_id == 23).unwrap_or_else(|| panic!("{name}: up special never ran"));
        // An effect bound to the move (stop bit) is made while the move runs, and the motion it was
        // made under changes before the intro is over, so the host ends it.
        for (i, tick) in run.iter().enumerate() {
            for g in tick.gfx.iter().filter(|g| g.2) {
                assert!(i <= last_move, "{name}: effect {} bound to the move was made after it ended", g.0);
                assert!(run[i..].iter().any(|t| t.serial != tick.serial), "{name}: effect {} is never ended by a motion change", g.0);
            }
        }
        assert!(run.last().unwrap().state.grounded != 0, "{name}: the intro ends on the floor");
    }
}

#[test]
fn yoshi_egg_throw_stays_on_the_ground_in_the_select_preview_and_throws_an_egg() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    let Some(mut rig) = Rig::new(&cdir, 6, 0.) else { return };
    unsafe { ssbm_sim_select_preview(rig.h) };
    rig.step(0, FORWARD);
    let mut peak = 0f32;
    rig.step(UP_B, FORWARD);
    for _ in 0..120 {
        peak = peak.max(rig.step(0, FORWARD).state.position.y);
    }
    assert_eq!(peak, 0., "the disc's grounded Egg Throw does not leave the ground (ftYs_SpecialHi_Phys)");
    // A Hit event of move 23 is the egg leaving his hand: the host draws the article.
    assert!(rig.events.0.iter().any(|e| e.0 == 2 && e.1 == 23), "the throw event reaches the host: {:?}", rig.events.0);
    // In play (no preview) the same move hops to the host's jump apex.
    let mut game = Rig::new(&cdir, 6, 0.).unwrap();
    unsafe { ssbm_sim_jump_height(game.h, 25.) };
    game.idle(5);
    game.step(UP_B, FORWARD);
    let mut hop = 0f32;
    for _ in 0..60 {
        hop = hop.max(game.step(0, FORWARD).state.position.y);
    }
    assert!(hop > 10., "the host's recovery hop is unchanged in play ({hop})");
}

#[test]
fn every_fighters_specials_make_sound() {
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    // The disc's Egg Lay tongue (SpecialN, ground and air) has no sound command in its clip and its code plays none; only the swallow (SpecialN2) has a voice.
    let silent_in_the_source: [(u32, u32, bool); 2] = [(6, NEUTRAL_B, false), (6, NEUTRAL_B, true)];
    for kind in 0..8u32 {
        for (name, button) in [("neutral", NEUTRAL_B), ("side", SIDE_B), ("up", UP_B), ("down", DOWN_B)] {
            for air in [false, true] {
                let Some(run) = special(&cdir, kind, button, air, FORWARD, 240) else { continue };
                let energy: f32 = run.iter().map(|t| t.audio).sum();
                if silent_in_the_source.contains(&(kind, button, air)) {
                    continue;
                }
                assert!(energy > 100., "{} {name} {}: no sound (energy {energy})", FIGHTERS[kind as usize], if air { "air" } else { "ground" });
            }
        }
    }
}

#[test]
fn electric_hits_are_audible_for_thunder() {
    // Pikachu's Thunder has no sound command of its own beyond the start voice: the bolt's and the
    // strike's crackle is the electric hit feedback the host plays on contact (lbColl_80005BB0).
    let Some(cdir) = cdir() else { eprintln!("SKIP: disc cache absent"); return };
    let Some(mut rig) = Rig::new(&cdir, 3, 0.) else { return };
    rig.idle(5);
    unsafe { ssbm_hit_feedback(rig.h, 2, 7, 2) };
    let tick = rig.step(0, FORWARD);
    assert!(tick.audio > 50., "the electric hit sound plays on the fighter's mixer ({})", tick.audio);
}
