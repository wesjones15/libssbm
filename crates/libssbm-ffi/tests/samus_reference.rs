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
}
fn scenario(name: &'static str, ticks: u32, holds: Vec<Hold>) -> Scenario {
    Scenario { name, hold_charge: 0, host_flags: 0, height: 0.0, ticks, holds, hooks: Vec::new() }
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
    match e.move_id {
        20 => Some((1, ((e.strength * 7.0).round() as i32 + 1).clamp(1, 8) as u32, e.strength)),
        21 => Some((2, 0, 0.0)),
        22 => Some((2, 1, 0.0)),
        24 if e.kind == 2 => Some((0, 0, 0.0)),
        _ => None,
    }
}

const SHOT_TICKS: u32 = 150;

/// Records one scenario to text. `assets` is the cache directory, or empty for the built-in profile.
fn record(assets: &str, s: &Scenario) -> String {
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
    for t in 0..s.ticks {
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
            for (when, hook) in &s.hooks {
                if *when != t {
                    continue;
                }
                match hook {
                    Hook::Interrupt => ssbm_sim_interrupt(h),
                    Hook::Hurt => {
                        ssbm_sim_hurt(h);
                    }
                    Hook::Catch => ssbm_sim_set_grabbed(h, 1),
                    Hook::Release => ssbm_sim_set_grabbed(h, 0),
                    Hook::Connect => ssbm_sim_connect(h),
                }
                let _ = writeln!(out, "K {t} {}", match hook {
                    Hook::Interrupt => "interrupt",
                    Hook::Hurt => "hurt",
                    Hook::Catch => "catch",
                    Hook::Release => "release",
                    Hook::Connect => "connect",
                });
            }
            world.events.clear();
            ssbm_sim_step(h, &host, &pad, 1.0 / 60.0);
        }
        let st = state(h);
        let motion = unsafe { ssbm_sim_motion_frame(h) };
        let _ = writeln!(
            out,
            "T {t} f={} ph={} mv={} pos={} vel={} gr={} ch={} sm={} jumps={} mf={:.2} btn={}",
            st.frame, st.phase, st.move_id, f3(st.position), f3(st.velocity), st.grounded,
            st.shot_charge, st.smash_charge, st.jumps_used, motion, pad.buttons
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
            unsafe { proj::ssbm_projectile_step(shot.handle, 1.0 / 60.0, v(0.0, 0.0, 0.0), 0) };
            shot.age += 1;
            let mut ps = proj::State::default();
            unsafe { proj::ssbm_projectile_state(shot.handle, &mut ps) };
            let world_pos = v(shot.origin.x + ps.position.x, shot.origin.y + ps.position.y, shot.origin.z + ps.position.z);
            let _ = writeln!(
                out,
                "P {t} index={} kind={} state={} pos={} vel={} frame={:.1} anim={:.1} dmg={:.3} r={:.3} fl={}",
                shot.index, ps.kind, ps.state, f3(world_pos), f3(ps.velocity), ps.frame, ps.animation_frame,
                ps.damage, ps.radius, ps.flags
            );
            if shot.age >= SHOT_TICKS || ps.flags & 2 != 0 {
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

/// Scenarios whose authored (built-in) profile must agree with the asset-backed fighter tick for
/// tick: moves, events, hitbox window, spawns and the tick control returns. Grows as the kit
/// migrates (step 2: the jab chain and the Charge Shot).
const MIGRATED: &[&str] = &[
    "jab_single",
    "jab_chain_taps",
    "jab_rapid_mash",
    "jab_tap_then_miss_late",
    "charge_tap_hold_control",
    "charge_half_hold_control",
    "charge_full_hold_control",
    "charge_tap_press_control",
    "charge_press_press_fire",
    "charge_aimed_up",
    "interrupt_jab_startup",
    "interrupt_jab_active",
    "interrupt_charge_holding",
    "hurt_during_charge",
    "connect_jab",
];

#[test]
fn builtin_jab_and_charge_shot_keep_the_measured_timings_without_a_disc() {
    // From the asset-backed recording: the jab strikes two ticks after it starts and hands control
    // back after 18; a tapped Charge Shot's start clip is 14 ticks and its release clip 30.
    let rows = run("", "builtin");
    let get = |key: &str, name: &str| rows[key].iter().find(|(n, _)| n == name).unwrap().1.clone();
    assert_eq!(get("moves", "jab_single"), "0:0 40:1 58:0");
    assert!(get("hitbox_window", "jab_single").starts_with("Some(42)..Some(43)"), "{}", get("hitbox_window", "jab_single"));
    assert_eq!(get("moves", "jab_chain_taps"), "0:0 40:1 52:13 82:0", "Samus has two jabs and no third");
    assert_eq!(get("spawns", "charge_tap_hold_control"), "40:a1s8 54:a1s1");
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
    let _ = writeln!(report, "\n{differing} of {total} scenarios differ.");
    std::fs::write(out_dir().join("samus-parity.md"), &report).unwrap();
    eprintln!("{differing} of {total} scenarios differ; see target/reference/samus-parity.md");
    for name in MIGRATED {
        let i = scenarios().iter().position(|s| s.name == *name).unwrap();
        for key in ["moves", "events", "hitbox_window", "spawns", "control_returns"] {
            assert_eq!(
                asset[key][i].1, builtin[key][i].1,
                "migrated scenario {name}: {key} differs from the asset-backed fighter"
            );
        }
    }
    // The recorded reference itself must have run the kit.
    let moves = &asset["moves"];
    let find = |name: &str| moves.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap();
    assert!(find("jab_single").contains(":1"), "disc jab never started");
    assert!(find("screw_attack_air").contains(":23"), "disc Screw Attack never started");
}
