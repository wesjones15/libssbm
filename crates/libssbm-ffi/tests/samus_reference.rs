//! Reference recording of Samus's whole kit through the native boundary, for the no-assets
//! migration (DOCS/melee-no-assets-migration-scratch.md, step 1).
//!
//! Each scenario replays fixed pad input, aim, tuning, host flags and host responses (a flat floor,
//! no enemies) against two constructions of the same fighter: the asset-backed one, from the
//! player's extracted cache, and the built-in one (an empty directory, which is how the host
//! falls back with no assets). Every tick is written to a trace: the sim state, native events,
//! hitbox samples, and the projectiles the host would have spawned from those events (mirroring
//! ror2-samus/Samus/Combat.cs), stepped one tick at a time.
//!
//! The traces and the parity report are derived from the disc, so they are written under the
//! ignored `target/reference/` and never committed. The asset-backed half prints SKIP when no
//! cache is found (`SSBM_ASSETS`, this checkout's `assets/`, or `target/melee-compare/samus`).
//! Divergence between the two modes is the thing being measured, so it never fails a test; what is
//! asserted is that each mode is deterministic (two recordings are byte-identical) and that the
//! kit's key moves actually ran.
use ssbm::projectile as proj;
use ssbm::*;
use std::collections::BTreeMap;
use std::ffi::{c_void, CString};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const JUMP: u32 = 1;
const ATTACK: u32 = 2;
const SPECIAL: u32 = 4;
const UP: u32 = 8;
const DOWN: u32 = 16;
const SPRINT: u32 = 32;
const CANCEL: u32 = 64;
const GRAB: u32 = 128;
const NEUTRAL_B: u32 = 256;
const SIDE_B: u32 = 512;
const UP_B: u32 = 1024;
const DOWN_B: u32 = 2048;
const DEFEND: u32 = 4096;
/// libssbm::samus::HOST_MORPH_ROLL: the host's combined bomb + held Morph Ball down-B.
const HOST_MORPH_ROLL: u32 = 4;

#[derive(Default)]
struct World {
    events: Vec<NativeEvent>,
}
unsafe extern "C" fn collide(_: *mut c_void, _: *const V3, to: *const V3, out: *mut Collision) {
    let to = unsafe { *to };
    let landed = to.y <= 0.0;
    unsafe {
        *out = Collision {
            position: V3 { x: to.x, y: if landed { 0.0 } else { to.y }, z: to.z },
            grounded: landed as u32,
            ceiling: 0,
        };
    }
}
unsafe extern "C" fn event(user: *mut c_void, e: *const NativeEvent) {
    unsafe { (*(user as *mut World)).events.push(*e) };
}

fn cache() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::var_os("SSBM_ASSETS") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(root.join("../../assets"));
    candidates.push(root.join("../../../target/melee-compare/samus"));
    candidates.into_iter().find(|dir| libssbm_assets::complete(dir))
}
fn out_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/reference");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn v(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

/// One scripted stretch of pad state: ticks `[from, from + len)`.
#[derive(Clone, Copy)]
struct Hold {
    from: u32,
    len: u32,
    stick: (f32, f32),
    aim: (f32, f32, f32),
    buttons: u32,
}
fn hold(from: u32, len: u32, buttons: u32) -> Hold {
    Hold { from, len, stick: (0.0, 0.0), aim: (1.0, 0.0, 0.0), buttons }
}
impl Hold {
    fn stick(mut self, x: f32, y: f32) -> Self {
        self.stick = (x, y);
        self
    }
    fn aim(mut self, x: f32, y: f32, z: f32) -> Self {
        self.aim = (x, y, z);
        self
    }
}
/// A host response applied before the step on a tick.
#[derive(Clone, Copy)]
enum Hook {
    Interrupt,
    Hurt,
    /// The host caught an enemy: the fighter holds it (`ssbm_sim_set_grabbed`).
    Catch,
    Release,
    /// The current attack connected (`ssbm_sim_connect`).
    Connect,
    /// The shield took a hit of this much damage (`ssbm_sim_shield_hit`).
    ShieldHit(f32),
    /// A harder hit of this severity (`ssbm_sim_hurt_scaled`).
    HurtScaled(f32),
    /// The host arrival sequence (`ssbm_sim_begin_arrival`), as at a stage's start.
    Arrival,
    /// A fall-off rebirth at a safe place (`ssbm_sim_begin_rebirth`).
    Rebirth,
    /// The trophy-to-fighter entry (`ssbm_sim_begin_entry`).
    Entry,
}
#[derive(Clone)]
struct Scenario {
    name: &'static str,
    /// `Tuning::hold_charge`: hold to charge and release to fire, or press, press again.
    hold_charge: u32,
    host_flags: u32,
    /// Start this high above the floor (0 = grounded).
    height: f32,
    ticks: u32,
    holds: Vec<Hold>,
    hooks: Vec<(u32, Hook)>,
    /// Where the host's lock-on is, relative to a spawned shot (homing articles steer to it).
    target: Option<V3>,
}
fn scenario(name: &'static str, ticks: u32, holds: Vec<Hold>) -> Scenario {
    Scenario { name, hold_charge: 0, host_flags: 0, height: 0.0, ticks, holds, hooks: Vec::new(), target: None }
}
impl Scenario {
    fn hold_charge(mut self) -> Self {
        self.hold_charge = 1;
        self
    }
    fn flags(mut self, f: u32) -> Self {
        self.host_flags = f;
        self
    }
    fn airborne(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    fn target(mut self, x: f32, y: f32, z: f32) -> Self {
        self.target = Some(v(x, y, z));
        self
    }
    fn hook(mut self, t: u32, h: Hook) -> Self {
        self.hooks.push((t, h));
        self
    }
}

/// The kit, against idle ground or air. Taps are one tick down; every scenario starts after a
/// 40-tick settle so the clock of the first press is the same in both modes.
fn scenarios() -> Vec<Scenario> {
    const S: u32 = 40;
    let tap = |t: u32, b: u32| hold(S + t, 1, b);
    let mut all = vec![
        scenario("idle", 60, vec![]),
        scenario("jab_single", 90, vec![tap(0, ATTACK)]),
        scenario("jab_chain_taps", 160, vec![tap(0, ATTACK), tap(10, ATTACK), tap(20, ATTACK)]),
        scenario("jab_rapid_mash", 200, (0..18).map(|i| tap(i * 4, ATTACK)).collect()),
        scenario("jab_tap_then_miss_late", 120, vec![tap(0, ATTACK), tap(40, ATTACK)]),
        scenario("side_tilt", 100, vec![hold(S - 12, 12, 0).stick(1.0, 0.0), hold(S, 1, ATTACK).stick(1.0, 0.0)]),
        // A tilt needs an old stick direction (a recent one is a smash); the camera pitch picks up or down.
        scenario("up_tilt", 100, vec![hold(S - 30, 30, 0).stick(0.4, 0.0).aim(1.0, 0.6, 0.0), hold(S, 1, ATTACK).stick(0.4, 0.0).aim(1.0, 0.6, 0.0)]),
        scenario("down_tilt", 100, vec![hold(S - 30, 30, 0).stick(0.4, 0.0).aim(1.0, -2.0, 0.0), hold(S, 1, ATTACK).stick(0.4, 0.0).aim(1.0, -2.0, 0.0)]),
        // The shoulder modifiers are the smashes (scenarios.rs shoulder_modifiers_select_vertical_smashes).
        scenario("up_smash_tap", 120, vec![hold(S, 1, ATTACK | UP)]),
        scenario("down_smash_tap", 120, vec![hold(S, 1, ATTACK | DOWN)]),
        scenario("side_smash_tap", 130, vec![hold(S, 1, ATTACK).stick(1.0, 0.0)]),
        scenario("side_smash_charged", 200, vec![hold(S, 50, ATTACK).stick(1.0, 0.0)]),
        scenario("up_smash_charged", 200, vec![hold(S, 50, ATTACK | UP)]),
        scenario("down_smash_charged", 200, vec![hold(S, 50, ATTACK | DOWN)]),
        scenario("dash_attack", 140, vec![hold(S - 30, 30, SPRINT).stick(1.0, 0.0), hold(S, 1, ATTACK | SPRINT).stick(1.0, 0.0)]),
        scenario("nair", 140, vec![tap(0, ATTACK)]).airborne(400.0),
        // Landing during the aerial (the short fall from 60 units lands mid-move in both modes).
        scenario("nair_landing", 140, vec![tap(0, ATTACK)]).airborne(60.0),
        scenario("fair", 140, vec![hold(S, 1, ATTACK).stick(1.0, 0.0)]).airborne(400.0),
        scenario("bair", 140, vec![hold(S, 1, ATTACK).stick(-1.0, 0.0)]).airborne(400.0),
        scenario("uair", 140, vec![hold(S, 1, ATTACK | UP)]).airborne(400.0),
        scenario("dair", 140, vec![hold(S, 1, ATTACK | DOWN)]).airborne(400.0),
        // Charge Shot: tap and hold on the hold-to-charge control, and on press-press.
        scenario("charge_tap_hold_control", 140, vec![hold(S, 1, NEUTRAL_B)]).hold_charge(),
        scenario("charge_half_hold_control", 160, vec![hold(S, 40, NEUTRAL_B)]).hold_charge(),
        scenario("charge_full_hold_control", 260, vec![hold(S, 150, NEUTRAL_B)]).hold_charge(),
        scenario("charge_tap_press_control", 140, vec![tap(0, NEUTRAL_B), tap(6, NEUTRAL_B)]),
        scenario("charge_press_press_fire", 200, vec![tap(0, NEUTRAL_B), tap(60, NEUTRAL_B)]),
        scenario("charge_store_then_fire", 300, vec![tap(0, NEUTRAL_B), tap(50, CANCEL), tap(90, NEUTRAL_B), tap(104, NEUTRAL_B)]),
        scenario("charge_store_by_jump", 260, vec![tap(0, NEUTRAL_B), tap(50, JUMP), tap(130, NEUTRAL_B), tap(145, NEUTRAL_B)]),
        scenario("charge_air_fire", 220, vec![tap(0, NEUTRAL_B), tap(40, NEUTRAL_B)]).airborne(400.0),
        scenario("charge_aimed_up", 200, vec![hold(S, 1, NEUTRAL_B).aim(1.0, 0.5, 0.0), hold(S + 40, 1, NEUTRAL_B).aim(1.0, 0.5, 0.0)]),
        // Missiles, Screw Attack, bomb and Morph Ball.
        scenario("missile_homing", 160, vec![tap(0, SIDE_B)]),
        scenario("missile_homing_target_up_right", 160, vec![tap(0, SIDE_B)]).target(60.0, 30.0, 25.0),
        scenario("missile_homing_target_behind", 200, vec![tap(0, SIDE_B)]).target(-40.0, 5.0, 30.0),
        scenario("missile_super_target", 160, vec![hold(S, 1, SIDE_B).stick(1.0, 0.0)]).target(80.0, 0.0, 10.0),
        scenario("missile_super", 160, vec![hold(S, 1, SIDE_B).stick(1.0, 0.0)]),
        scenario("missile_air", 160, vec![tap(0, SIDE_B)]).airborne(400.0),
        scenario("screw_attack_ground", 200, vec![tap(0, UP_B)]),
        scenario("screw_attack_air", 200, vec![tap(0, UP_B)]).airborne(400.0),
        scenario("bomb_plain", 160, vec![tap(0, DOWN_B)]),
        scenario("bomb_air", 160, vec![tap(0, DOWN_B)]).airborne(400.0),
        scenario("morph_ball_hold", 200, vec![hold(S, 70, DOWN_B)]).flags(HOST_MORPH_ROLL),
        scenario("morph_ball_steer", 200, vec![hold(S, 70, DOWN_B).stick(1.0, 0.0)]).flags(HOST_MORPH_ROLL),
        // Grapple, pummel, throws; the host's catch is a hook at the tick it would land.
        scenario("grab_whiff", 140, vec![tap(0, GRAB)]),
        scenario("grab_air_whiff", 140, vec![tap(0, GRAB)]).airborne(400.0),
        scenario("grab_pummel_throw_forward", 260, vec![
            tap(0, GRAB), tap(40, ATTACK), tap(70, ATTACK), hold(S + 100, 1, ATTACK).stick(1.0, 0.0),
        ]).hook(S + 30, Hook::Catch),
        scenario("grab_throw_back", 220, vec![tap(0, GRAB), hold(S + 50, 1, ATTACK).stick(-1.0, 0.0)]).hook(S + 30, Hook::Catch),
        scenario("grab_throw_up", 220, vec![tap(0, GRAB), hold(S + 50, 1, ATTACK | UP)]).hook(S + 30, Hook::Catch),
        scenario("grab_throw_down", 220, vec![tap(0, GRAB), hold(S + 50, 1, ATTACK | DOWN)]).hook(S + 30, Hook::Catch),
        // Defense: shield, release, shield hits, dodges and rolls.
        scenario("shield_hold", 200, vec![hold(S, 120, DEFEND)]),
        scenario("shield_tap", 120, vec![hold(S, 6, DEFEND)]),
        scenario("shield_hit", 200, vec![hold(S, 100, DEFEND)]).hook(S + 30, Hook::ShieldHit(12.0)),
        scenario("shield_break", 700, vec![hold(S, 400, DEFEND)]).hook(S + 10, Hook::ShieldHit(80.0)),
        scenario("roll_forward", 160, vec![hold(S, 2, DEFEND).stick(1.0, 0.0)]),
        scenario("roll_back", 160, vec![hold(S, 2, DEFEND).stick(-1.0, 0.0)]),
        scenario("spot_dodge", 160, vec![hold(S, 2, DEFEND | DOWN)]),
        scenario("air_dodge", 200, vec![hold(S, 2, DEFEND).stick(1.0, 0.0)]).airborne(400.0),
        scenario("hurt_scaled_ground", 200, vec![]).hook(S + 5, Hook::HurtScaled(0.5)),
        scenario("hurt_scaled_air", 260, vec![]).airborne(120.0).hook(S + 5, Hook::HurtScaled(1.0)),
        scenario("hurt_scaled_launch_heavy", 300, vec![]).hook(S + 5, Hook::HurtScaled(3.0)),
        // Arrival and fall-off rebirth.
        scenario("arrival", 500, vec![hold(300, 5, ATTACK)]).hook(5, Hook::Arrival),
        scenario("entry", 300, vec![hold(150, 5, ATTACK)]).hook(5, Hook::Entry),
        scenario("rebirth", 700, vec![hold(500, 5, ATTACK)]).hook(S, Hook::Rebirth),
        // Movement and recovery.
        scenario("walk_run", 120, vec![hold(S, 80, 0).stick(1.0, 0.0)]),
        scenario("short_and_full_hop", 220, vec![tap(0, JUMP), hold(S + 80, 8, JUMP)]),
        scenario("double_jump", 200, vec![tap(0, JUMP), tap(30, JUMP)]),
        scenario("hurt_idle", 100, vec![]).hook(S + 5, Hook::Hurt),
        // Interrupts: a hit or host cancel during startup, active frames and recovery.
        scenario("interrupt_jab_startup", 100, vec![tap(0, ATTACK)]).hook(S + 1, Hook::Interrupt),
        scenario("interrupt_jab_active", 100, vec![tap(0, ATTACK)]).hook(S + 3, Hook::Interrupt),
        scenario("interrupt_fsmash_charging", 140, vec![hold(S, 40, ATTACK).stick(1.0, 0.0)]).hook(S + 25, Hook::Interrupt),
        scenario("interrupt_charge_holding", 160, vec![hold(S, 60, NEUTRAL_B)]).hold_charge().hook(S + 40, Hook::Interrupt),
        scenario("interrupt_missile_recovery", 160, vec![tap(0, SIDE_B)]).hook(S + 30, Hook::Interrupt),
        scenario("interrupt_screw_attack", 200, vec![tap(0, UP_B)]).hook(S + 10, Hook::Interrupt),
        scenario("hurt_during_charge", 200, vec![hold(S, 80, NEUTRAL_B)]).hold_charge().hook(S + 30, Hook::Hurt),
        scenario("interrupt_morph_ball", 160, vec![hold(S, 70, DOWN_B)]).flags(HOST_MORPH_ROLL).hook(S + 30, Hook::Interrupt),
        scenario("interrupt_grab_hold_release", 200, vec![tap(0, GRAB)]).hook(S + 30, Hook::Catch).hook(S + 70, Hook::Release),
        scenario("connect_jab", 100, vec![tap(0, ATTACK)]).hook(S + 4, Hook::Connect),
    ];
    // The same kit at the other control mode for the specials that read it.
    all.push(scenario("special_unlabelled_neutral", 140, vec![hold(S, 1, SPECIAL)]));
    all
}

fn state(h: *mut Handle) -> State {
    let mut s = State::default();
    assert_eq!(unsafe { ssbm_sim_state(h, &mut s) }, 1);
    s
}
fn f3(a: V3) -> String {
    format!("({:.4},{:.4},{:.4})", a.x, a.y, a.z)
}

/// A host-spawned article (Combat.cs `Shot.Spawn`): its origin and the handle that flies it.
struct Shot {
    handle: *mut proj::Handle,
    origin: V3,
    age: u32,
    done: bool,
    index: usize,
}

/// Mirror of Combat.cs's event switch: which article, state and charge a native event spawns.
fn article_for(e: &NativeEvent) -> Option<(u32, u32, f32)> {
    // Only a Hit event (kind 2) is the shot leaving; Started (1) is the move beginning.
    if e.kind != 2 {
        return None;
    }
    match e.move_id {
        20 => Some((1, ((e.strength * 7.0).round() as i32 + 1).clamp(1, 8) as u32, e.strength)),
        21 => Some((2, 0, 0.0)),
        22 => Some((2, 1, 0.0)),
        24 => Some((0, 0, 0.0)),
        _ => None,
    }
}

const SHOT_TICKS: u32 = 150;

/// Records one scenario to text. `assets` is the cache directory, or empty for the built-in profile.
fn record(assets: &str, s: &Scenario) -> String {
    record_at(assets, s, 60)
}

/// As [`record`], but the host steps at `hz` frames a second: each call hands the sim `1 / hz`
/// seconds (zero, one or two ticks), the pad is whatever the script holds at that moment, and a
/// hook fires on the first call at or after its tick. Tick labels are the 60 Hz tick the call began in.
fn record_at(assets: &str, s: &Scenario, hz: u32) -> String {
    record_full(assets, s, hz, None)
}

/// As record_at; with `churn` (a cache directory whose presentation assets are broken or absent)
/// every call also tries to build and free a body view, an article view and an effect view from
/// it and to read audio, as a host does each frame. None of that may touch gameplay.
fn record_full(assets: &str, s: &Scenario, hz: u32, churn: Option<&CString>) -> String {
    let cdir = CString::new(assets).unwrap();
    let mut world = World::default();
    let host = HostTable { user: &mut world as *mut World as *mut c_void, collide: Some(collide), event: Some(event) };
    let h = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), v(0.0, s.height, 0.0), 0) };
    assert!(!h.is_null(), "fighter construction failed for {assets:?}");
    let tuning = Tuning {
        speed_scale: 1.0,
        jump_scale: 1.0,
        extra_jumps: 0,
        hold_charge: s.hold_charge,
        smash_max_frames: 60,
    };
    let mut out = String::new();
    let mut shots: Vec<Shot> = Vec::new();
    let mut spawned = 0usize;
    let _ = writeln!(out, "# scenario {} assets={}", s.name, if assets.is_empty() { "builtin" } else { "disc" });
    unsafe {
        ssbm_sim_tuning(h, tuning);
        ssbm_sim_facing(h, v(1.0, 0.0, 0.0));
        if s.height > 0.0 {
            ssbm_sim_anchor(h, v(0.0, s.height, 0.0), 0);
        }
    }
    let dt = 1.0 / hz as f64;
    let mut fired = vec![false; s.hooks.len()];
    for call in 0..s.ticks * hz / 60 {
        let t = call * 60 / hz;
        let mut pad = Pad { movement: v(0.0, 0.0, 0.0), aim: v(1.0, 0.0, 0.0), buttons: 0, allowed: 31 };
        for hd in &s.holds {
            if t >= hd.from && t < hd.from + hd.len {
                pad.movement = v(hd.stick.0, hd.stick.1, 0.0);
                pad.aim = v(hd.aim.0, hd.aim.1, hd.aim.2);
                pad.buttons |= hd.buttons;
            }
        }
        unsafe {
            ssbm_sim_set_host_flags(h, s.host_flags);
            ssbm_sim_facing(h, v(1.0, 0.0, 0.0));
            for (n, (when, hook)) in s.hooks.iter().enumerate() {
                if t < *when || fired[n] {
                    continue;
                }
                fired[n] = true;
                match hook {
                    Hook::Interrupt => ssbm_sim_interrupt(h),
                    Hook::Hurt => {
                        ssbm_sim_hurt(h);
                    }
                    Hook::Catch => ssbm_sim_set_grabbed(h, 1),
                    Hook::Release => ssbm_sim_set_grabbed(h, 0),
                    Hook::Connect => ssbm_sim_connect(h),
                    Hook::ShieldHit(damage) => {
                        ssbm_sim_shield_hit(h, *damage);
                    }
                    Hook::HurtScaled(severity) => {
                        ssbm_sim_hurt_scaled(h, *severity);
                    }
                    Hook::Arrival => {
                        ssbm_sim_begin_arrival(h, cdir.as_ptr());
                    }
                    Hook::Rebirth => {
                        ssbm_sim_begin_rebirth(h, cdir.as_ptr(), v(40.0, 0.0, 0.0));
                    }
                    Hook::Entry => {
                        ssbm_sim_begin_entry(h, cdir.as_ptr());
                    }
                }
                let _ = writeln!(out, "K {t} {}", match hook {
                    Hook::Interrupt => "interrupt",
                    Hook::Hurt => "hurt",
                    Hook::Catch => "catch",
                    Hook::Release => "release",
                    Hook::Connect => "connect",
                    Hook::ShieldHit(_) => "shield_hit",
                    Hook::HurtScaled(_) => "hurt_scaled",
                    Hook::Arrival => "arrival",
                    Hook::Rebirth => "rebirth",
                    Hook::Entry => "entry",
                });
            }
            world.events.clear();
            ssbm_sim_step(h, &host, &pad, dt);
        }
        if let Some(bad) = churn {
            unsafe {
                let view = ssbm_view_new(bad.as_ptr(), 0, 0);
                assert!(!view.is_null(), "a broken cache must fall back to a procedural body");
                assert_eq!(ssbm_view_update(view, h, dt as f32), 1);
                ssbm_view_free(view);
                let article = ssbm_article_new(bad.as_ptr(), 0, 1, 1);
                assert!(!article.is_null(), "a broken article must fall back to a procedural one");
                ssbm_view_free(article);
                let effect = ssbm_effect_new(bad.as_ptr(), 0, 5);
                if !effect.is_null() {
                    ssbm_view_free(effect);
                }
                let mut audio = vec![0f32; 512];
                ssbm_sim_audio(h, audio.as_mut_ptr(), 256);
            }
        }
        let st = state(h);
        let motion = unsafe { ssbm_sim_motion_frame(h) };
        let _ = writeln!(
            out,
            "T {t} f={} ph={} mv={} pos={} vel={} gr={} ch={} sm={} jumps={} mf={:.2} btn={} sh={:.3} sc={:.3} df={} rb={}",
            st.frame, st.phase, st.move_id, f3(st.position), f3(st.velocity), st.grounded,
            st.shot_charge, st.smash_charge, st.jumps_used, motion, pad.buttons,
            unsafe { ssbm_sim_shield_fraction(h) }, unsafe { ssbm_sim_shield_scale(h) },
            unsafe { ssbm_sim_defense_flags(h) }, unsafe { ssbm_sim_rebirth_flags(h) }
        );
        for e in &world.events {
            let _ = writeln!(
                out,
                "E {t} kind={} mv={} str={:.4} pos={} dir={}",
                e.kind, e.move_id, e.strength, f3(e.position), f3(e.direction)
            );
            if let Some((article, state, charge)) = article_for(e) {
                let handle = unsafe {
                    proj::ssbm_projectile_new(cdir.as_ptr(), 0, article, state, charge, e.direction)
                };
                if handle.is_null() {
                    let _ = writeln!(out, "X {t} projectile article={article} state={state} refused");
                } else {
                    unsafe {
                        // Combat.cs: no offscreen lifetime cutoff for the charge shot or missile.
                        if article == 1 || article == 2 {
                            proj::ssbm_projectile_lifetime(handle, 0.0);
                        }
                    }
                    let _ = writeln!(out, "S {t} index={spawned} article={article} state={state} charge={charge:.4} origin={} dir={}", f3(e.position), f3(e.direction));
                    shots.push(Shot { handle, origin: e.position, age: 0, done: false, index: spawned });
                    spawned += 1;
                }
            }
        }
        let mut boxes = [HitboxSample::default(); 32];
        let n = unsafe { ssbm_sim_hitboxes(h, boxes.as_mut_ptr(), 32) };
        for b in boxes.iter().take((n as usize).min(32)) {
            let _ = writeln!(
                out,
                "H {t} gen={} grp={} id={} dmg={:.3} r={:.3} ang={} gro={} fkb={} bkb={} el={} fl={} local={}",
                b.generation, b.group, b.id, b.damage, b.radius, b.angle, b.growth, b.fixed_knockback,
                b.base_knockback, b.element, b.flags, f3(b.local_position)
            );
        }
        for shot in shots.iter_mut().filter(|s| !s.done) {
            unsafe { proj::ssbm_projectile_step(shot.handle, dt, s.target.unwrap_or(v(0.0, 0.0, 0.0)), s.target.is_some() as u32) };
            shot.age += 1;
            let mut ps = proj::State::default();
            unsafe { proj::ssbm_projectile_state(shot.handle, &mut ps) };
            let world_pos = v(shot.origin.x + ps.position.x, shot.origin.y + ps.position.y, shot.origin.z + ps.position.z);
            let _ = writeln!(
                out,
                "P {t} index={} kind={} state={} pos={} vel={} frame={:.1} anim={:.1} dmg={:.3} r={:.3} fl={} world={}",
                shot.index, ps.kind, ps.state, f3(ps.position), f3(ps.velocity), ps.frame, ps.animation_frame,
                ps.damage, ps.radius, ps.flags, f3(world_pos)
            );
            if shot.age >= SHOT_TICKS * hz / 60 || ps.flags & 2 != 0 {
                shot.done = true;
            }
        }
    }
    for shot in &shots {
        unsafe { proj::ssbm_projectile_free(shot.handle) };
    }
    unsafe { ssbm_sim_free(h) };
    out
}

/// What a trace says about the gameplay, as a one-line-per-fact summary, so two modes can be
/// compared by what happened and when instead of by float noise.
fn summary(trace: &str) -> BTreeMap<&'static str, String> {
    let mut moves: Vec<(u32, u32)> = Vec::new();
    let (mut events, mut hits, mut spawns) = (Vec::new(), Vec::new(), Vec::new());
    let (mut first_hit, mut last_hit, mut boxes) = (None, None, 0u32);
    let mut ids = std::collections::BTreeSet::new();
    let (mut last_tick, mut free_after) = (0u32, None);
    let mut moved = false;
    for line in trace.lines() {
        let mut parts = line.split(' ');
        let kind = parts.next().unwrap_or("");
        let t: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let field = |name: &str| line.split(' ').find_map(|p| p.strip_prefix(name)).unwrap_or("").to_string();
        match kind {
            "T" => {
                last_tick = t;
                let mv: u32 = field("mv=").parse().unwrap_or(0);
                if moves.last().map(|m| m.1) != Some(mv) {
                    moves.push((t, mv));
                    if mv == 0 && moved && free_after.is_none() {
                        free_after = Some(t);
                    }
                }
                if mv != 0 {
                    moved = true;
                }
            }
            "E" => events.push(format!("{t}:k{}m{}", field("kind="), field("mv="))),
            "H" => {
                first_hit.get_or_insert(t);
                last_hit = Some(t);
                boxes += 1;
                ids.insert(format!("{}/{}", field("id="), field("grp=")));
                hits.push(t);
            }
            "S" => spawns.push(format!("{t}:a{}s{}", field("article="), field("state="))),
            _ => {}
        }
    }
    let _ = (last_tick, &hits);
    let mut out = BTreeMap::new();
    out.insert("moves", moves.iter().map(|(t, m)| format!("{t}:{m}")).collect::<Vec<_>>().join(" "));
    out.insert("events", events.join(" "));
    out.insert("hitbox_window", format!("{:?}..{:?} samples={boxes} ids={ids:?}", first_hit, last_hit));
    out.insert("spawns", spawns.join(" "));
    out.insert("control_returns", format!("{free_after:?}"));
    out
}

/// The gameplay fields of a trace, canonicalised for a tick-for-tick comparison: the sim state,
/// events (kind, move, strength, direction), hitbox presence (id, generation, group) and the
/// projectiles' flight. Rig-dependent geometry (bone-local hitbox positions, attachment points)
/// and authored approximations (hitbox damage and radius) are left out.
fn gameplay_lines(trace: &str, with_motion: bool) -> Vec<String> {
    let keep: &[(&str, &[&str])] = &[
        ("T", if with_motion { &["ph=", "mv=", "pos=", "vel=", "gr=", "ch=", "sm=", "jumps=", "sh=", "sc=", "df=", "rb="] } else { &["ph=", "mv=", "gr=", "ch=", "sm=", "jumps=", "sh=", "sc=", "df=", "rb="] }),
        ("E", &["kind=", "mv=", "str=", "dir="]),
        ("K", &[]),
        ("S", &["article=", "state=", "charge=", "dir="]),
        ("H", &["gen=", "grp=", "id=", "ang=", "el="]),
        ("P", &["index=", "kind=", "state=", "pos=", "vel=", "fl="]),
    ];
    trace
        .lines()
        .filter_map(|line| {
            let mut parts = line.split(' ');
            let kind = parts.next()?;
            let (_, fields) = keep.iter().find(|(k, _)| *k == kind)?;
            let tick = parts.next()?;
            let mut out = format!("{kind} {tick}");
            for part in line.split(' ').skip(2) {
                if fields.iter().any(|f| part.starts_with(f)) {
                    out.push(' ');
                    out.push_str(part);
                }
            }
            Some(out)
        })
        .collect()
}

/// Scenarios whose root travel is an authored curve, not the disc's per-tick samples: every other
/// line must match, and the body's path must stay within `APPROX_PATH_UNITS` of the disc's.
const APPROX_MOTION: &[&str] = &["dash_attack"];
const APPROX_PATH_UNITS: f32 = 2.5;

/// The largest gap between the two traces' body positions on any tick (sim units).
fn path_gap(a: &str, b: &str) -> f32 {
    let positions = |trace: &str| -> Vec<(f32, f32, f32)> {
        trace
            .lines()
            .filter(|l| l.starts_with("T "))
            .filter_map(|l| {
                let p = l.split(' ').find_map(|f| f.strip_prefix("pos=("))?.trim_end_matches(')');
                let mut c = p.split(',').map(|v| v.parse::<f32>().unwrap());
                Some((c.next()?, c.next()?, c.next()?))
            })
            .collect()
    };
    positions(a)
        .iter()
        .zip(positions(b).iter())
        .map(|(p, q)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2) + (p.2 - q.2).powi(2)).sqrt())
        .fold(0.0, f32::max)
}

/// The first line where two gameplay traces differ, with both sides, or None when they agree.
fn first_divergence(a: &[String], b: &[String]) -> Option<String> {
    (0..a.len().max(b.len())).find_map(|i| {
        let (x, y) = (a.get(i).map(String::as_str).unwrap_or("<end>"), b.get(i).map(String::as_str).unwrap_or("<end>"));
        (x != y).then(|| format!("line {i}: disc `{x}` vs builtin `{y}`"))
    })
}

fn run(assets: &str, tag: &str) -> BTreeMap<&'static str, Vec<(String, String)>> {
    let dir = out_dir().join(tag);
    std::fs::create_dir_all(&dir).unwrap();
    let mut rows: BTreeMap<&'static str, Vec<(String, String)>> = BTreeMap::new();
    for s in scenarios() {
        let a = record(assets, &s);
        let b = record(assets, &s);
        assert_eq!(a, b, "{tag}: scenario {} is not deterministic", s.name);
        std::fs::write(dir.join(format!("{}.trace", s.name)), &a).unwrap();
        for (key, value) in summary(&a) {
            rows.entry(key).or_default().push((s.name.to_string(), value));
        }
    }
    rows
}

/// The built-in Samus's timings as measured against the asset-backed fighter (the recorded
/// reference, tick for tick), pinned so they hold on a clean checkout with no disc: (scenario, the
/// ticks each move starts on, the native events, the articles spawned). Input begins at tick 40.
#[test]
fn builtin_samus_keeps_the_measured_timings_without_a_disc() {
    const PINNED: &[(&str, &str, &str, &str)] = &[
        ("jab_single", "0:0 40:1 58:0", "40:k1m1", ""),
        ("jab_chain_taps", "0:0 40:1 52:13 82:0", "40:k1m1 52:k1m13", ""),
        ("side_tilt", "0:0 40:2 72:0", "40:k1m2", ""),
        ("fair", "0:0 40:7 96:0", "40:k1m7", ""),
        ("missile_homing", "0:0 40:21 100:0", "40:k1m21 61:k2m21", "61:a2s0"),
        ("missile_super", "0:0 40:22 90:0", "40:k1m22 61:k2m22", "61:a2s1"),
        ("screw_attack_air", "0:0 40:23 88:0", "40:k1m23", ""),
        ("bomb_plain", "0:0 40:24 94:0", "40:k1m24 49:k2m24", "49:a0s0"),
        (
            "grab_pummel_throw_forward",
            "0:0 40:25 70:33 80:28 104:33 110:28 134:33 140:29 182:0",
            "40:k1m25 80:k1m28 110:k1m28 140:k1m29 154:k6m29",
            "",
        ),
        ("side_smash_charged", "0:0 40:10 130:0", "40:k1m10", ""),
        ("dash_attack", "0:0 40:5 78:0", "40:k1m5", ""),
        ("charge_tap_hold_control", "0:0 40:20 84:0", "40:k1m20 54:k2m20", "54:a1s1"),
        ("charge_full_hold_control", "0:0 40:20 173:0", "40:k1m20 173:k5m20", ""),
    ];
    let rows = run("", "builtin");
    let get = |key: &str, name: &str| rows[key].iter().find(|(n, _)| n == name).unwrap().1.clone();
    for (name, moves, events, spawns) in PINNED {
        assert_eq!(get("moves", name), *moves, "{name}: moves");
        assert_eq!(get("events", name), *events, "{name}: events");
        assert_eq!(get("spawns", name), *spawns, "{name}: spawns");
    }
}

#[test]
fn builtin_samus_kit_is_deterministic_and_recorded() {
    let rows = run("", "builtin");
    let moves: &Vec<(String, String)> = &rows["moves"];
    let pick = |name: &str| moves.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap();
    assert!(pick("jab_single").contains(":1"), "the jab never started: {}", pick("jab_single"));
    assert!(pick("screw_attack_air").contains(":23"), "no Screw Attack: {}", pick("screw_attack_air"));
}

#[test]
fn asset_backed_samus_kit_is_recorded_and_diffed_against_builtin() {
    let Some(dir) = cache() else {
        eprintln!("SKIP: disc cache absent");
        return;
    };
    let asset = run(dir.to_str().unwrap(), "disc");
    let builtin = run("", "builtin");
    let mut report = String::from(
        "# Samus parity: asset-backed (reference) vs built-in\n\nGenerated by `cargo test -p libssbm-ffi --test samus_reference`. Derived from the player's disc: do not commit.\nPer scenario, the facts that differ. Ticks count from scenario start (input begins at tick 40).\n\n",
    );
    let mut differing = 0;
    for (i, s) in scenarios().iter().enumerate() {
        let mut lines = Vec::new();
        for key in ["moves", "events", "hitbox_window", "spawns", "control_returns"] {
            let (a, b) = (&asset[key][i].1, &builtin[key][i].1);
            if a != b {
                lines.push(format!("- **{key}**\n  - disc:    `{a}`\n  - builtin: `{b}`"));
            }
        }
        if !lines.is_empty() {
            differing += 1;
            let _ = writeln!(report, "## {}\n{}\n", s.name, lines.join("\n"));
        }
    }
    let total = scenarios().len();
    let _ = writeln!(report, "\n{differing} of {total} scenarios differ in the coarse summary.");
    // The strict criterion: every gameplay line, tick for tick.
    let mut strict = Vec::new();
    for s in scenarios() {
        let read = |tag: &str| {
            gameplay_lines(
                &std::fs::read_to_string(out_dir().join(tag).join(format!("{}.trace", s.name))).unwrap(),
                !APPROX_MOTION.contains(&s.name),
            )
        };
        if let Some(d) = first_divergence(&read("disc"), &read("builtin")) {
            strict.push((s.name, d));
        }
    }
    for name in APPROX_MOTION {
        let read = |tag: &str| std::fs::read_to_string(out_dir().join(tag).join(format!("{name}.trace"))).unwrap();
        let gap = path_gap(&read("disc"), &read("builtin"));
        let _ = writeln!(report, "\nAuthored root travel `{name}`: largest path gap {gap:.2} units (limit {APPROX_PATH_UNITS}).");
        assert!(gap <= APPROX_PATH_UNITS, "{name}: authored travel strays {gap} units from the disc's");
    }
    let _ = writeln!(report, "\n## Strict gameplay divergence ({} of {total} scenarios)\n", strict.len());
    for (name, d) in &strict {
        let _ = writeln!(report, "- **{name}**: {d}");
    }
    std::fs::write(out_dir().join("samus-parity.md"), &report).unwrap();
    eprintln!("{differing} of {total} differ in summary, {} strictly; see target/reference/samus-parity.md", strict.len());
    assert!(strict.is_empty(), "the authored Samus diverges from the asset-backed one tick for tick: {strict:#?}");
    // The recorded reference itself must have run the kit.
    let moves = &asset["moves"];
    let find = |name: &str| moves.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap();
    assert!(find("jab_single").contains(":1"), "disc jab never started");
    assert!(find("screw_attack_air").contains(":23"), "disc Screw Attack never started");
}


// ---------------------------------------------------------------------------------------------
// Step 4: the authored profile against every state a cache can be in, other host step rates,
// mixed-asset replay, and presentation that fails to load.
// ---------------------------------------------------------------------------------------------

/// Every scenario's trace recorded with `assets` at `hz`, by name.
fn gameplay_at(assets: &str, hz: u32, churn: Option<&CString>) -> Vec<(&'static str, String)> {
    scenarios().iter().map(|s| (s.name, record_full(assets, s, hz, churn))).collect()
}

/// Asserts two recordings agree on every scenario's gameplay (the dash attack's authored travel
/// aside when `approx_motion`), naming each first divergence.
fn assert_same_gameplay(what: &str, a: &[(&'static str, String)], b: &[(&'static str, String)], approx_motion: bool) {
    let mut bad = Vec::new();
    for ((name, x), (_, y)) in a.iter().zip(b) {
        let motion = !(approx_motion && APPROX_MOTION.contains(name));
        if let Some(d) = first_divergence(&gameplay_lines(x, motion), &gameplay_lines(y, motion)) {
            bad.push(format!("{name}: {d}"));
        }
    }
    assert!(bad.is_empty(), "{what}: {} scenarios diverge:\n{}", bad.len(), bad.join("\n"));
}

/// A cache directory under target/reference/cache-variants holding a copy of `src` without the
/// entries named in `skip`, then `mutate`d.
fn cache_variant(name: &str, src: &Path, skip: &[&str], mutate: impl FnOnce(&Path)) -> PathBuf {
    let dir = out_dir().join("cache-variants").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    fn copy(from: &Path, to: &Path, skip: &[&str]) {
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if skip.contains(&name.as_str()) {
                continue;
            }
            let target = to.join(&name);
            if entry.file_type().unwrap().is_dir() {
                std::fs::create_dir_all(&target).unwrap();
                copy(&entry.path(), &target, skip);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    copy(src, &dir, skip);
    mutate(&dir);
    dir
}

/// A directory with every file Samus's assets name, each a few bytes of junk.
fn junk_cache(name: &str) -> PathBuf {
    let dir = out_dir().join("cache-variants").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    for file in libssbm_assets::fighter::SAMUS.required().unwrap() {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"NOT A MELEE FILE").unwrap();
    }
    dir
}

#[test]
fn empty_missing_and_junk_caches_play_exactly_like_the_authored_profile() {
    let authored = gameplay_at("", 60, None);
    let junk = junk_cache("junk");
    let missing = out_dir().join("cache-variants").join("does-not-exist");
    for (what, dir) in [("junk files", &junk), ("missing directory", &missing)] {
        let got = gameplay_at(dir.to_str().unwrap(), 60, None);
        assert_same_gameplay(what, &authored, &got, false);
    }
}

#[test]
fn partial_and_broken_disc_caches_fall_back_whole_or_keep_the_disc_gameplay() {
    let Some(src) = cache() else {
        eprintln!("SKIP: disc cache absent");
        return;
    };
    let authored = gameplay_at("", 60, None);
    let disc = gameplay_at(src.to_str().unwrap(), 60, None);
    // Gameplay data missing or unreadable: the whole fighter falls back to the authored profile.
    let truncated = cache_variant("truncated-fighter", &src, &[], |d| {
        let bytes = std::fs::read(d.join("PlSs.dat")).unwrap();
        std::fs::write(d.join("PlSs.dat"), &bytes[..bytes.len() / 2]).unwrap();
    });
    let no_animations = cache_variant("no-animations", &src, &["PlSsAJ.dat"], |_| {});
    let broken_common = cache_variant("broken-common", &src, &[], |d| {
        std::fs::write(d.join("PlCo.dat"), vec![0xFFu8; 4096]).unwrap();
    });
    for (what, dir) in [("truncated PlSs.dat", &truncated), ("missing PlSsAJ.dat", &no_animations), ("broken PlCo.dat", &broken_common)] {
        let got = gameplay_at(dir.to_str().unwrap(), 60, None);
        assert_same_gameplay(what, &authored, &got, false);
    }
    // Only presentation data missing (effects, results scene, portraits, audio): the sim still
    // loads and plays the disc gameplay, line for line.
    let no_presentation = cache_variant(
        "no-presentation",
        &src,
        &["EfSsData.dat", "EfCoData.dat", "IfAll.usd", "MnSlChr.usd", "audio"],
        |_| {},
    );
    let got = gameplay_at(no_presentation.to_str().unwrap(), 60, None);
    assert_same_gameplay("no effects, portraits or audio", &disc, &got, false);
    // And the disc gameplay is the authored profile, which is the point of the migration.
    assert_same_gameplay("disc vs authored", &disc, &authored, true);
}

#[test]
fn authored_samus_matches_the_disc_at_30_and_144_hz_host_stepping() {
    let Some(src) = cache() else {
        eprintln!("SKIP: disc cache absent");
        return;
    };
    for hz in [30, 144] {
        let disc = gameplay_at(src.to_str().unwrap(), hz, None);
        let authored = gameplay_at("", hz, None);
        assert_same_gameplay(&format!("{hz} Hz"), &disc, &authored, true);
    }
}

#[test]
fn a_held_charge_takes_the_same_ticks_at_any_host_step_rate() {
    // No disc: the authored fighter's own timeline must not depend on how the host slices time.
    let s = scenarios().into_iter().find(|s| s.name == "charge_full_hold_control").unwrap();
    let end_of_move = |hz: u32| {
        let trace = record_at("", &s, hz);
        let field = |l: &str, k: &str| l.split(' ').find_map(|f| f.strip_prefix(k)).unwrap().to_string();
        let mut last = None;
        for line in trace.lines().filter(|l| l.starts_with("T ")) {
            if field(line, "mv=") != "0" {
                last = Some(line.split(' ').nth(1).unwrap().parse::<i64>().unwrap());
            }
        }
        last.unwrap()
    };
    let at60 = end_of_move(60);
    for hz in [30, 144] {
        let got = end_of_move(hz);
        assert!((got - at60).abs() <= 2, "{hz} Hz: the charge move ended on tick {got}, not {at60}");
    }
}

#[test]
fn a_disc_owner_and_an_authored_shadow_follow_each_other_both_ways() {
    let Some(src) = cache() else {
        eprintln!("SKIP: disc cache absent");
        return;
    };
    let disc = CString::new(src.to_str().unwrap()).unwrap();
    let empty = CString::new("").unwrap();
    let mut world = World::default();
    let host = HostTable { user: &mut world as *mut World as *mut c_void, collide: Some(collide), event: Some(event) };
    let pad = |x: f32, buttons: u32| Pad { movement: v(x, 0.0, 0.0), aim: v(1.0, 0.0, 0.0), buttons, allowed: 31 };
    let script = |t: u32| match t {
        0..=29 => pad(0.0, 0),
        30..=89 => pad(1.0, 0),
        90..=91 => pad(0.0, JUMP),
        92..=151 => pad(0.4, 0),
        152 | 158 => pad(0.0, ATTACK),
        200..=205 => pad(1.0, ATTACK),
        260 => pad(0.0, SPECIAL),
        330 => pad(0.0, SIDE_B),
        400..=402 => pad(0.0, JUMP),
        _ => pad(0.0, 0),
    };
    for (owner_dir, shadow_dir, label) in [
        (&disc, &empty, "disc owner, authored shadow"),
        (&empty, &disc, "authored owner, disc shadow"),
    ] {
        let owner = unsafe { ssbm_sim_new_fighter(owner_dir.as_ptr(), V3::default(), 0) };
        let shadow = unsafe { ssbm_sim_new_fighter(shadow_dir.as_ptr(), V3::default(), 0) };
        assert!(!owner.is_null() && !shadow.is_null());
        assert_eq!(unsafe { ssbm_sim_record(owner, 1) }, 1);
        let mut bytes = vec![0u8; 1 << 14];
        let mut worst = 0.0f32;
        for batch in 0..(600 / 3) {
            for i in 0..3 {
                let t = batch * 3 + i;
                let p = script(t);
                unsafe {
                    ssbm_sim_facing(owner, v(1.0, 0.0, 0.0));
                    if t == 300 {
                        ssbm_sim_hurt_scaled(owner, 0.3);
                    }
                    ssbm_sim_step(owner, &host, &p, 1.0 / 60.0);
                }
            }
            let n = unsafe { ssbm_sim_ops_take(owner, bytes.as_mut_ptr(), bytes.len() as u32) };
            assert!(n > 0, "{label}: no log");
            assert_ne!(unsafe { ssbm_sim_replay(shadow, &host, bytes.as_ptr(), n) }, u32::MAX, "{label}: replay rejected");
            let (a, b) = (state(owner), state(shadow));
            let d = ((a.position.x - b.position.x).powi(2)
                + (a.position.y - b.position.y).powi(2)
                + (a.position.z - b.position.z).powi(2))
            .sqrt();
            worst = worst.max(d);
            assert!(d < 1.5, "{label}: batch {batch}: {d} units apart");
            assert_eq!((a.move_id, a.phase, a.grounded, a.frame), (b.move_id, b.phase, b.grounded, b.frame), "{label}: batch {batch}");
        }
        eprintln!("{label}: worst gap {worst}");
        unsafe {
            ssbm_sim_free(owner);
            ssbm_sim_free(shadow);
        }
    }
}

#[test]
fn failed_presentation_loads_change_nothing_and_do_not_kill_live_projectiles() {
    let junk = CString::new(junk_cache("junk-presentation").to_str().unwrap()).unwrap();
    // The authored fighter, with a view, an article and an effect failing to load every frame.
    let plain = gameplay_at("", 60, None);
    let churned = gameplay_at("", 60, Some(&junk));
    assert_same_gameplay("authored with failing presentation", &plain, &churned, false);
    // Every projectile the plain run flew was still flying, line for line, in the churned run.
    let flew = |runs: &[(&str, String)]| runs.iter().map(|(_, t)| t.lines().filter(|l| l.starts_with("P ")).count()).sum::<usize>();
    assert!(flew(&plain) > 1000, "the kit should fly shots: {}", flew(&plain));
    assert_eq!(flew(&plain), flew(&churned));
    // The same against the disc gameplay, with its presentation failing too.
    if let Some(src) = cache() {
        let dir = src.to_str().unwrap();
        let disc = gameplay_at(dir, 60, None);
        let churned_disc = gameplay_at(dir, 60, Some(&junk));
        assert_same_gameplay("disc with failing presentation", &disc, &churned_disc, false);
    } else {
        eprintln!("SKIP: disc cache absent (the authored half ran)");
    }
}
