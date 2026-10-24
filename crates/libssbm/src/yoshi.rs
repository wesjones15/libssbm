//! Yoshi-specific behavior: the flutter jump and the Egg Lay, Egg Roll, Egg Throw
//! and Yoshi Bomb specials. The specials are staged, driving the disc's own clips;
//! tuning reads PlYs.dat ext_attr (ftYoshiAttributes / ftYs_DatAttrs). References:
//! ftYoshi/ftyoshispecial{n,s,hi,lw}.c, ftCommon/ftCo_JumpAerial.c::ftYs_JumpAerial_Enter,
//! it/kinds/ityoshi{tongue,egglay,eggthrow,star}.c.
//!
//! RoR2 adaptations, marked where they occur: the egg roll steers by the move
//! stick relative to the camera (the source reads one stick axis), the flutter's
//! lift is a gravity scale (the source takes it from the clip), and the tongue
//! swallow, the egg and the stars are host jobs.
use crate::{
    action::{ActiveHitbox, Hitbox},
    behavior::{Behavior, Flutter},
    sim::{EventKind, Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
};
use glam::Vec3;

/// Yoshi's special tuning, separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Egg Roll: start hop (x5C), gravity (x6C), acceleration (x74), braking (x78),
    /// target speed (x7C), top speed (x80), turn-around stick deadzone (x9C), wall
    /// bounce (xAC) and duration in frames (x48).
    pub egg_roll_hop: f32,
    pub egg_roll_gravity: f32,
    pub egg_roll_accel: f32,
    pub egg_roll_brake: f32,
    pub egg_roll_speed: f32,
    pub egg_roll_max: f32,
    pub egg_roll_turn_stick: f32,
    pub egg_roll_wall_bounce: f32,
    pub egg_roll_frames: u32,
    /// Egg Throw's base throw speed (ftYs_DatAttrs.xFC).
    pub egg_throw_speed: f32,
    /// Yoshi Bomb's downward terminal speed (ftYoshiAttributes.x114).
    pub bomb_fall_speed: f32,
    /// How many frames the aerial jump's armor and flutter last (ftYoshiAttributes.x0).
    pub flutter_frames: u32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            egg_roll_hop: 1.5,
            egg_roll_gravity: 0.1,
            egg_roll_accel: 0.2,
            egg_roll_brake: 0.1,
            egg_roll_speed: 2.,
            egg_roll_max: 3.,
            egg_roll_turn_stick: 0.3,
            egg_roll_wall_bounce: 0.5,
            egg_roll_frames: 120,
            egg_throw_speed: 8.,
            bomb_fall_speed: 6.,
            flutter_frames: 20,
        }
    }
}

// Egg Lay: SpecialN1 (the tongue), SpecialN2 (the swallow).
const LAY_TONGUE: u32 = 0;
const LAY_SWALLOW: u32 = 1;
// Egg Roll: SpecialSStart (into the egg), SpecialSLoop (rolling), SpecialSEnd.
const ROLL_START: u32 = 0;
const ROLL_LOOP: u32 = 1;
const ROLL_END: u32 = 2;
// Yoshi Bomb: SpecialLw (the hop and stall), SpecialAirLw (the dive), SpecialLwLanding.
const BOMB_STALL: u32 = 0;
const BOMB_DIVE: u32 = 1;
const BOMB_LAND: u32 = 2;

/// Steering of the rolling egg toward the stick, radians per tick (host adaptation).
const ROLL_TURN: f32 = 0.14;
/// Ticks before another wall bounce can register.
const ROLL_BOUNCE_COOLDOWN: u32 = 12;
/// Ticks between the rolling egg's dust puffs (host choice; the source's interval is a
/// disc attribute not read here).
const ROLL_DUST_TICKS: u32 = 6;
/// How often the rolling hitbox may strike again. The source toggles its one hit capsule
/// every xDC (32) ticks; RoR2 enemies need more contact than that to read the roll, so a
/// host adaptation refreshes the authored hitboxes much sooner. The authored 4% damage and
/// knockback are unchanged.
const ROLL_HIT_TICKS: u32 = 8;
/// The slam's hitbox follows Yoshi down (the AirLw clip's 14% at its last frame,
/// carried the whole dive so a falling Yoshi hits what he falls on).
const DIVE_BOX: u32 = 91;
const DIVE_GROUP: u32 = 3;
/// A grounded Yoshi Bomb hops this fraction of the host's jump height before it
/// dives (the source's hop is the clip's own rise).
const BOMB_HOP: f32 = 0.35;

pub struct Yoshi {
    specials: Specials,
}
impl Yoshi {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Yoshi behavior for headless tests.
    pub fn fixture() -> Self {
        Self::new(Specials::fixture())
    }

    fn hold_gravity(sim: &mut Sim, gravity: f32) {
        sim.pad_gravity = Some(Vec3::new(0., -gravity, 0.));
        sim.owns_gravity = true;
    }
    fn release_gravity(sim: &mut Sim) {
        if sim.owns_gravity {
            sim.pad_gravity = None;
            sim.owns_gravity = false;
        }
    }

    // --- Egg Lay -----------------------------------------------------------------
    fn tick_lay(&self, sim: &mut Sim) {
        if sim.grounded {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
        sim.stage_frame += 1;
    }

    // --- Egg Roll ----------------------------------------------------------------
    fn begin_roll(&self, sim: &mut Sim) {
        sim.stage = ROLL_LOOP;
        sim.stage_frame = 0;
        sim.stage_len = 0;
        sim.stage_dir = sim.facing.with_y(0.).normalize_or_zero();
        sim.stage_vec = sim.position;
        sim.stage_value = self.specials.egg_roll_speed * 0.5;
        sim.action.active.clear();
    }
    /// End the roll: back to `SpecialSEnd`, no more roll hitboxes, and the shell bits the
    /// source spawns from the hip (efSync 0x4CF).
    fn end_roll(&self, sim: &mut Sim) {
        sim.stage = ROLL_END;
        sim.stage_frame = 0;
        sim.action.active.clear();
        sim.roll_break = false;
        for _ in 0..3 {
            let shell = crate::effect::part_cue(sim, 1231, crate::effect::part::HIP_N);
            sim.action.effects.push(shell);
        }
    }
    fn tick_roll(&self, sim: &mut Sim, input: Input) {
        let scale = sim.options.speed_scale;
        match sim.stage {
            ROLL_START => {
                sim.stage_frame += 1;
                Self::hold_gravity(sim, sim.params.gravity);
                if sim.grounded {
                    sim.velocity.x = 0.;
                    sim.velocity.z = 0.;
                }
                // Hold the button to keep rolling; releasing (or running out of armor)
                // breaks him out at any time.
                if sim.roll_break || input.buttons & SIDE_B == 0 {
                    self.end_roll(sim);
                }
            }
            ROLL_LOOP => {
                sim.stage_frame += 1;
                sim.stage_len = sim.stage_len.saturating_sub(1);
                // Steer toward the stick (ftYs_SpecialAirSLoop_0_IASA turns him around on
                // a stick past x9C; here the whole heading follows it).
                let want = input.movement.with_y(0.);
                if want.length() > self.specials.egg_roll_turn_stick {
                    let want = want.normalize();
                    let angle = sim.stage_dir.dot(want).clamp(-1., 1.).acos();
                    if angle > 1e-4 {
                        sim.stage_dir = sim
                            .stage_dir
                            .slerp(want, (ROLL_TURN / angle).min(1.))
                            .normalize_or_zero();
                    }
                }
                // ftYs_SpecialAirSLoop_0_Phys: accelerate to the target speed, brake above it.
                let target = self.specials.egg_roll_speed;
                let mut speed = sim.stage_value;
                if speed < target {
                    speed = (speed + self.specials.egg_roll_accel).min(target);
                } else {
                    speed = (speed - self.specials.egg_roll_brake).max(target);
                }
                speed = speed.min(self.specials.egg_roll_max);
                // A wall or an enemy stops the egg dead: bounce back off it.
                if sim.stage_frame > 8 && sim.stage_len == 0 {
                    let moved = (sim.position - sim.stage_vec).with_y(0.).length();
                    if moved < speed * scale * 0.3 {
                        sim.stage_dir = -sim.stage_dir;
                        speed *= self.specials.egg_roll_wall_bounce.abs().max(0.3);
                        sim.velocity.y = self.specials.egg_roll_hop * 0.35;
                        sim.stage_len = ROLL_BOUNCE_COOLDOWN;
                        // The bonk dust against the wall (efSync 0x406).
                        let bonk = crate::effect::part_cue(sim, 1030, crate::effect::part::TOP_N);
                        sim.action.effects.push(bonk);
                    }
                }
                sim.stage_vec = sim.position;
                sim.stage_value = speed;
                sim.facing = sim.stage_dir;
                // ftYs_SpecialS_8012F0DC: the rolling dust (efSync 0x3FF) at intervals on the ground.
                if sim.grounded && sim.stage_frame % ROLL_DUST_TICKS == 0 {
                    let dust = crate::effect::part_cue(sim, 1023, crate::effect::part::TOP_N);
                    sim.action.effects.push(dust);
                }
                // Refresh the authored roll hitboxes so contact registers repeatedly.
                // ftYs_SpecialAirSLoop_*_Anim toggles the one hit capsule every xDC ticks;
                // a new generation lets MeleeHitTracker hit a victim it already struck
                // while the egg is still overlapping it.
                if sim.stage_frame % ROLL_HIT_TICKS == 0 {
                    let slot = &mut sim.action.generations[0];
                    *slot = slot.wrapping_add(1);
                    let generation = *slot;
                    for active in sim.action.active.values_mut() {
                        if active.hitbox.group == 0 {
                            active.generation = generation;
                        }
                    }
                }
                sim.velocity.x = sim.stage_dir.x * speed * scale;
                sim.velocity.z = sim.stage_dir.z * speed * scale;
                let gravity = if sim.grounded { 0. } else { sim.params.gravity };
                Self::hold_gravity(sim, gravity);
                // Hold the button to keep rolling. Release, or an armor break reported by
                // the host through `roll_break`, ends it immediately.
                if sim.roll_break || input.buttons & SIDE_B == 0 {
                    self.end_roll(sim);
                }
            }
            _ => {
                sim.stage_frame += 1;
                sim.velocity.x *= 0.85;
                sim.velocity.z *= 0.85;
                let gravity = if sim.grounded { 0. } else { sim.params.gravity };
                Self::hold_gravity(sim, gravity);
            }
        }
    }

    // --- Yoshi Bomb ----------------------------------------------------------------
    fn begin_dive(&self, sim: &mut Sim) {
        sim.stage = BOMB_DIVE;
        sim.stage_frame = 0;
        sim.velocity.x = 0.;
        sim.velocity.z = 0.;
        Self::hold_gravity(sim, 0.);
        let group = DIVE_GROUP as usize;
        sim.action.generations[group] = sim.action.generations[group].wrapping_add(1);
        let generation = sim.action.generations[group];
        sim.action.active.insert(
            DIVE_BOX,
            ActiveHitbox {
                hitbox: Hitbox {
                    id: DIVE_BOX,
                    group: DIVE_GROUP,
                    bone: 0,
                    common_bone: false,
                    damage: 14.,
                    radius: 8.,
                    offset: Vec3::new(0., 4., 0.),
                    angle: 80,
                    growth: 96,
                    fixed_knockback: 0,
                    base_knockback: 46,
                    element: 0,
                    sfx_kind: 1,
                    sfx_severity: 2,
                    ground: true,
                    air: true,
                },
                generation,
            },
        );
    }
    fn tick_bomb(&self, sim: &mut Sim, host: &mut dyn Host) {
        match sim.stage {
            BOMB_STALL => {
                // The grounded version hops and hangs (the clip's rise); gravity waits.
                sim.stage_frame += 1;
                sim.velocity.x = 0.;
                sim.velocity.z = 0.;
                if sim.stage_frame == 1 {
                    sim.grounded = false;
                    sim.velocity.y = if let Some(height) = sim.host_jump_height {
                        crate::controls::jump_velocity(height * BOMB_HOP, sim.params.gravity)
                    } else {
                        1.2
                    };
                }
                Self::hold_gravity(sim, sim.params.gravity * 0.5);
            }
            BOMB_DIVE => {
                sim.stage_frame += 1;
                // ftYs_SpecialAirLw_Phys: straight down at x114.
                sim.velocity.x = 0.;
                sim.velocity.z = 0.;
                sim.velocity.y = -self.specials.bomb_fall_speed.abs() * sim.options.speed_scale.max(0.5);
                Self::hold_gravity(sim, 0.);
                if sim.grounded && sim.stage_frame > 1 {
                    sim.action.active.remove(&DIVE_BOX);
                    sim.stage = BOMB_LAND;
                    sim.stage_frame = 0;
                    sim.velocity = Vec3::ZERO;
                    Self::release_gravity(sim);
                    // ftYs_SpecialS_8012EAD8 then fn_8012E644: the landing spawns two stars
                    // (the host spreads them and deals the shockwave).
                    sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 1.);
                }
            }
            _ => {
                sim.stage_frame += 1;
                sim.velocity.x = 0.;
                sim.velocity.z = 0.;
            }
        }
    }
}
impl Behavior for Yoshi {
    fn hurt_eyes(&self) -> bool {
        true
    }
    fn slot(&self, m: Move) -> u32 {
        match m {
            Move::SpecialN => 0,
            Move::SpecialS | Move::SpecialS2 => 2,
            Move::SpecialHi => 3,
            Move::SpecialLw => 4,
            _ => 1,
        }
    }
    fn mobile_moves(&self) -> u64 {
        1u64 << Move::SpecialN as u32
    }
    fn upper_body_root(&self) -> Option<usize> {
        None
    }
    fn is_up_special(&self, m: Move) -> bool {
        m == Move::SpecialHi
    }
    fn flutter(&self) -> Option<Flutter> {
        // ftYs_JumpAerial_Enter: armor for x0 frames and a fluttering rise (the clip's
        // own lift in the source; a scaled gravity here).
        Some(Flutter {
            frames: self.specials.flutter_frames.clamp(8, 60),
            gravity: 0.3,
            boost: 0.9,
        })
    }

    fn select_special(&self, _sim: &Sim, input: Input, movement: Vec3) -> Option<Move> {
        Some(if input.buttons & NEUTRAL_B != 0 {
            Move::SpecialN
        } else if input.buttons & UP_B != 0 {
            Move::SpecialHi
        } else if input.buttons & DOWN_B != 0 {
            Move::SpecialLw
        } else if input.buttons & SIDE_B != 0 || movement.length() > 0.3 {
            Move::SpecialS
        } else {
            Move::SpecialN
        })
    }
    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) {
        if m == Move::SpecialHi {
            // Egg Throw hops to the host's standard jump apex, like every other
            // recovery; the egg itself spawns through the projectile path.
            // ftyoshispecialhi.c::ftYs_SpecialHi_Enter.
            sim.pad_gravity = None;
            sim.grounded = false;
            sim.velocity.y = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity)
            } else {
                3.2 * sim.options.jump_scale
            };
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
        } else if m == Move::SpecialS {
            // Egg Roll: he curls into the egg where he stands (ftYs_SpecialS_Enter hops
            // by x5C; the roll's own start carries it).
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
            if !sim.grounded {
                sim.velocity.y = sim.velocity.y.max(0.);
            }
        } else if m == Move::SpecialLw {
            // Yoshi Bomb: airborne, the dive starts at once; grounded, he hops first.
            sim.pad_gravity = None;
            if !sim.grounded {
                self.begin_dive(sim);
            }
        } else if m == Move::SpecialN && sim.grounded {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialHi)
    }
    fn is_charge_move(&self, _m: Move) -> bool {
        false
    }
    fn charge_voice_band(&self, _sim: &Sim) -> u32 {
        0
    }
    fn charge_steps(&self) -> u32 {
        1
    }
    fn charge_visual(&self) -> (f32, f32) {
        (1., 1.)
    }
    fn store_special(&self, _sim: &mut Sim, _host: &mut dyn Host, _pressed: u32) -> bool {
        false
    }
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }
    fn resolve_startup(&self, _sim: &mut Sim, _input: Input) -> bool {
        false
    }
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32 {
        if m == Move::SpecialN {
            // 1: the tongue is out; 2: the egg is laid.
            if sim.stage == LAY_SWALLOW { 2. } else { 1. }
        } else {
            sim.move_strength
        }
    }
    fn tick_charge(&self, _sim: &mut Sim, _host: &mut dyn Host, _input: Input, _pressed: u32) {}
    fn tick_charge_cancel(&self, sim: &mut Sim) -> bool {
        if sim.motion.finished() {
            sim.set_phase(Phase::Free);
            sim.current_move = None;
            true
        } else {
            false
        }
    }

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, _pressed: u32) {
        let mine = matches!(sim.current_move, Some(Move::SpecialN | Move::SpecialS | Move::SpecialLw));
        if !mine || sim.phase == Phase::Free {
            // Ended or interrupted: never leave a held gravity override or the dive
            // hitbox behind.
            Self::release_gravity(sim);
            sim.action.active.remove(&DIVE_BOX);
            return;
        }
        match sim.current_move {
            Some(Move::SpecialN) => self.tick_lay(sim),
            Some(Move::SpecialS) => self.tick_roll(sim, input),
            Some(Move::SpecialLw) => self.tick_bomb(sim, host),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) -> bool {
        match (m, sim.stage) {
            (Move::SpecialS, ROLL_START) => {
                self.begin_roll(sim);
                true
            }
            (Move::SpecialS, ROLL_LOOP) => {
                // The loop clip is a second long; the roll lasts longer. A new pass
                // renews the hitboxes so the egg can strike again.
                sim.motion.frame = 0.;
                sim.action.active.clear();
                true
            }
            (Move::SpecialLw, BOMB_STALL) => {
                self.begin_dive(sim);
                true
            }
            (Move::SpecialLw, BOMB_DIVE) => {
                // Held in the dive pose until he lands.
                sim.motion.frame = 18.;
                true
            }
            _ => false,
        }
    }
    fn on_connect(&self, sim: &mut Sim) {
        // The tongue touched something: swallow it.
        if sim.current_move == Some(Move::SpecialN) && sim.stage == LAY_TONGUE {
            sim.stage = LAY_SWALLOW;
            sim.stage_frame = 0;
            sim.action.projectile_fired = false;
        }
    }
    fn projectile_variable(&self) -> Option<u32> {
        // SpecialN1 raises cmd var 0 when the tongue is out and SpecialN2 when the
        // egg is laid.
        Some(0)
    }
    fn script_muted(&self, sim: &Sim) -> bool {
        // The dive's own hitbox stands in for the clip's, which would double it.
        sim.current_move == Some(Move::SpecialLw) && sim.stage == BOMB_DIVE
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        let now = sim.grounded;
        match (m, sim.stage) {
            (Move::SpecialN, LAY_TONGUE) => {
                if now { "SpecialN" } else { "SpecialAirN" }
            }
            (Move::SpecialN, _) => {
                if now { "SpecialN2" } else { "SpecialAirN2" }
            }
            (Move::SpecialS, ROLL_START) => {
                if now { "SpecialS" } else { "SpecialAirS" }
            }
            (Move::SpecialS, ROLL_LOOP) => "SpecialSLoop",
            (Move::SpecialS, _) => {
                if now { "SpecialSEnd" } else { "SpecialAirSEnd" }
            }
            (Move::SpecialHi, _) => {
                if sim.move_started_grounded { "SpecialHi" } else { "SpecialAirHi" }
            }
            (Move::SpecialLw, BOMB_STALL) => "SpecialLw",
            (Move::SpecialLw, BOMB_DIVE) => "SpecialAirLw",
            (Move::SpecialLw, _) => "SpecialLwLanding",
            _ => "",
        }
    }
    fn landing(&self, _sim: &Sim, m: Move) -> &'static str {
        if m == Move::SpecialHi { "LandingFallSpecial" } else { "Landing" }
    }
    fn facing_locked(&self, m: Move) -> bool {
        matches!(m, Move::SpecialHi | Move::SpecialLw)
    }
    fn facing_locked_now(&self, sim: &Sim, m: Move) -> bool {
        self.facing_locked(m) || (m == Move::SpecialS && sim.stage >= ROLL_LOOP)
    }
    fn spins_whole_body(&self, _m: Move) -> bool {
        false
    }
    fn side_special_layered(&self, _m: Move) -> bool {
        false
    }
    fn side_special_upper_aim(&self, _sim: &Sim) -> Option<(f32, f32)> {
        None
    }
    fn hit_attachment(&self, _sim: &Sim, _m: Move) -> (Option<usize>, Vec3) {
        (None, Vec3::ZERO)
    }
    fn projectile_move(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialHi)
    }
    fn projectile_cue(&self, _m: Move) -> Option<(u32, usize, bool)> {
        None
    }
    fn phase_effect(&self, _sim: &Sim, _phase: Phase) -> Option<crate::action::GfxCue> {
        None
    }
    fn phase_stop(&self, _phase: Phase) -> Option<crate::action::SoundCue> {
        None
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(8),
            Move::SpecialS => Some(10),
            Move::SpecialHi => Some(10),
            Move::SpecialLw => Some(6),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(20),
            Move::SpecialS => Some(30),
            Move::SpecialHi => Some(30),
            Move::SpecialLw => Some(30),
            _ => None,
        }
    }
}
