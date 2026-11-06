//! Luigi-specific special-move behavior: Fireball, Green Missile, Super Jump
//! Punch and Luigi Cyclone. Each is the source's multi-state motion machine
//! reimplemented as stages of one staged special (`Sim::stage`), driving the
//! disc's own clips; tuning comes from ftLuigiAttributes on the player's disc.
//! References: ftLuigi/ftluigispecial{n,s,hi,lw}.c.
//!
//! RoR2 adaptations, marked where they occur: the camera aim stands in for the
//! 2D control stick (the Green Missile's heading), the missile travels further
//! than the 60 Hz source for open worlds, the Cyclone re-hits on a fixed
//! interval and pulls enemies in (the host does the pulling), and a smash
//! flick is read from the sim's flick age.
use crate::{
    action::GfxCue,
    behavior::{Behavior, SpecialFall},
    sim::{EventKind, Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
};
use glam::Vec3;

/// Luigi's special tuning, separate from the shared `Params`. Field comments give
/// the `ftLuigiAttributes` offset (ft/kinds/ftLuigi/types.h).
#[derive(Clone, Debug)]
pub struct Specials {
    /// A smash input younger than this many ticks starts the missile pre-charged (x4).
    pub smash_window: f32,
    /// Charge a smash-input missile starts with (x8) and the charge it launches at (xC).
    pub charge_start: f32,
    pub max_charge: f32,
    /// Damage of a charged missile: charge * slope + tilt (x10, x14).
    pub damage_tilt: f32,
    pub damage_slope: f32,
    /// The wind-up divides horizontal speed by this (x18) and bleeds it (x1C).
    pub start_traction: f32,
    pub start_decel: f32,
    /// Launch speed: charge * mul_x + vel_x (x28, x24); lift: vel_y (x2C).
    pub vel_x: f32,
    pub mul_x: f32,
    pub vel_y: f32,
    /// Gravity while flying (x30), the terminal speed (x34), the speed divisor when it ends
    /// (x38), the horizontal decel (x3C) and the gravity afterwards (x40).
    pub fly_gravity: f32,
    pub fly_terminal: f32,
    pub friction_end: f32,
    pub end_decel: f32,
    pub end_gravity: f32,
    /// One launch in this many misfires (x44), flying at (x48, x4C).
    pub misfire_chance: f32,
    pub misfire_vx: f32,
    pub misfire_vy: f32,
    /// Super Jump Punch: helpless drift scale (x50) and landing lag (x54), the stick range that
    /// tilts the climb (x5C) and the most it tilts (x60 degrees), the momentum kept on entry
    /// from the air (x64).
    pub jump_freefall: f32,
    pub jump_landing_lag: f32,
    pub jump_stick_min: f32,
    pub jump_angle: f32,
    pub jump_air_keep: f32,
    /// Cyclone: lift of a tap (x8C) and the dip it starts with (x70), the horizontal speed cap
    /// and its acceleration on the ground (x74, x7C) and in the air (x78, x80), and the speed the
    /// cap loses each tick once the spin winds down (x84).
    pub cyclone_tap_momentum: f32,
    pub cyclone_tap_lift: f32,
    pub cyclone_ground_cap: f32,
    pub cyclone_ground_accel: f32,
    pub cyclone_air_cap: f32,
    pub cyclone_air_accel: f32,
    pub cyclone_wind_down: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            smash_window: 3.,
            charge_start: 20.,
            max_charge: 90.,
            damage_tilt: 5.,
            damage_slope: 0.2333,
            start_traction: 1.25,
            start_decel: 0.02,
            vel_x: 1.4,
            mul_x: 0.0167,
            vel_y: 0.8,
            fly_gravity: 0.01,
            fly_terminal: 1.2,
            friction_end: 1.5,
            end_decel: 0.05,
            end_gravity: 0.1,
            misfire_chance: 8.,
            misfire_vx: 4.,
            misfire_vy: 1.2,
            jump_freefall: 0.6,
            jump_landing_lag: 40.,
            jump_stick_min: 0.625,
            jump_angle: 2.,
            jump_air_keep: 0.,
            cyclone_tap_momentum: 0.1,
            cyclone_tap_lift: 0.8,
            cyclone_ground_cap: 2.35,
            cyclone_ground_accel: 0.2,
            cyclone_air_cap: 0.8,
            cyclone_air_accel: 0.08,
            cyclone_wind_down: 0.66,
        }
    }
}

// Green Missile: SpecialSStart, SpecialSHold (the charge, looping), the launch
// clip SpecialS (which also carries the flight), SpecialSEnd.
const S_START: u32 = 0;
const S_HOLD: u32 = 1;
const S_LAUNCH: u32 = 2;
const S_FLY: u32 = 3;
const S_END: u32 = 4;
// Cyclone: before the spin (frames 0-5), spinning, wound down.
const LW_WINDUP: u32 = 0;
const LW_SPIN: u32 = 1;
const LW_DONE: u32 = 2;

/// FtPart_HipN, flagged so the host resolves it through the common bone table.
const PART_HIP_N: usize = 4;
/// Host adaptation: the missile flies further than the 60 Hz source for open worlds.
const TRAVEL_SCALE: f32 = 2.5;
/// Host adaptation: distance flown (units; 60 m) after which the missile ignites — Fox's Fire Fox
/// flame on the body — and its damage doubles for the rest of the flight.
const MISSILE_IGNITE_DIST: f32 = 600.;
const MISSILE_IGNITE_DAMAGE: f32 = 2.;
/// Host adaptation: the missile gains this many charge frames a tick (the source gains one), so
/// a full charge comes in half the time while the charged damage itself is unchanged.
const MISSILE_CHARGE_RATE: u32 = 2;
/// The Super Jump Punch climbs higher than the source's ~30 units for open-world traversal
/// (host adaptation): doubled, it reaches about a RoR2 recovery jump, on top of `JumpScale`.
const UP_B_LIFT_SCALE: f32 = 2.0;
/// Ticks between the Cyclone's hits while its first hitboxes are up (host adaptation: the
/// source's multi-hit comes from its hitbox refresh, which the shared sampler does not run).
const CYCLONE_REHIT: u32 = 4;
/// The Green Missile's gfx (efSync 0x50A on the hip) and the Cyclone's (0x509).
const MISSILE_GFX: u32 = 0x50A;
const CYCLONE_GFX: u32 = 0x509;
/// `Sim::host_flags`: the host reports that the Super Scope is the special (LB) slot's
/// alternate, so LB drives the scope instead of the Super Jump Punch.
pub const HOST_SCOPE_ON_SPECIAL: u32 = 1;

pub struct Luigi {
    specials: Specials,
}
impl Luigi {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Luigi behavior for headless tests.
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
    fn cue(sim: &Sim, id: u32, bone: usize) -> GfxCue {
        GfxCue {
            id,
            bone,
            common: true, // FtPart index into the common bone table
            cycle: 0,
            destroy_on_change: false,
            offset: Vec3::ZERO,
            param: 0.,
            clip: sim.motion.clip,
            frame: sim.motion.frame,
            origin: sim.position,
            facing: sim.facing,
        }
    }
    /// ftCommon_CalcGroundAccel_Deaccel / CalcSelfAccel_Deaccel: speed lost per tick.
    fn bleed(sim: &mut Sim, amount: f32) {
        let horizontal = sim.velocity.with_y(0.);
        let step = amount * sim.options.speed_scale;
        let slowed = horizontal.length() - step;
        let next = if slowed > 0. { horizontal.normalize() * slowed } else { Vec3::ZERO };
        sim.velocity.x = next.x;
        sim.velocity.z = next.z;
    }
    fn heading(sim: &Sim) -> Vec3 {
        let aim = sim.move_aim.with_y(0.);
        if aim.length_squared() > 0.001 {
            aim.normalize()
        } else {
            sim.facing.with_y(0.).normalize_or_zero()
        }
    }

    // --- Green Missile (ftluigispecials.c) -------------------------------------
    fn begin_launch(&self, sim: &mut Sim) {
        sim.stage = S_LAUNCH;
        sim.stage_frame = 0;
        sim.action.variables[0] = 0;
        // ftLuigi_SpecialS_Setup: efSync 0x50A on the hip.
        let gfx = Self::cue(sim, MISSILE_GFX, PART_HIP_N);
        sim.action.effects.push(gfx);
    }
    /// ftLg_SpecialSFly_Enter: the launch speed grows with the charge; a misfire flies
    /// at the fixed misfire speed.
    fn begin_fly(&self, sim: &mut Sim) {
        let s = &self.specials;
        let misfire = sim.stage_value >= 1.;
        let heading = Self::heading(sim);
        if heading != Vec3::ZERO {
            sim.facing = heading;
        }
        let (vx, vy) = if misfire {
            (s.misfire_vx, s.misfire_vy)
        } else {
            let charge = sim.shot_charge as f32;
            (
                s.mul_x * charge + s.vel_x,
                0.5 * s.vel_y + s.vel_y * (0.5 * charge / s.max_charge.max(1.)),
            )
        };
        sim.velocity = heading * vx * TRAVEL_SCALE * sim.options.speed_scale
            + Vec3::Y * vy * sim.options.jump_scale;
        sim.grounded = false;
        sim.stage = S_FLY;
        sim.stage_frame = 0;
        sim.stage_dir = sim.position;
        sim.stage_vec = sim.velocity.with_y(0.);
        sim.missile_travel = 0.;
        sim.action.variables[1] = 0;
        Self::hold_gravity(sim, 0.);
    }
    /// ftLg_SpecialS_OnGiveDamage / the wall and floor ends: the speed is cut and the End clip plays.
    fn end_missile(&self, sim: &mut Sim) {
        if sim.stage == S_END {
            return;
        }
        sim.stage = S_END;
        sim.stage_frame = 0;
        sim.action.active.clear();
        let divisor = self.specials.friction_end.max(1.);
        sim.velocity.x /= divisor;
        sim.velocity.z /= divisor;
        Self::release_gravity(sim);
    }
    fn tick_missile(&self, sim: &mut Sim, input: Input) {
        let s = &self.specials;
        match sim.stage {
            S_START => {
                // No gravity through the wind-up in the air; the speed bleeds off.
                if sim.grounded {
                    Self::release_gravity(sim);
                } else {
                    Self::hold_gravity(sim, 0.);
                    sim.velocity.y = 0.;
                }
                Self::bleed(sim, s.start_decel);
                sim.stage_frame += 1;
            }
            S_HOLD => {
                // The charge: one more frame each tick, launched on release or past the maximum.
                Self::release_gravity(sim);
                sim.stage_frame += 1;
                // Clamp to the source's maximum so a faster rate cannot overshoot the charged damage.
                sim.shot_charge = sim
                    .shot_charge
                    .saturating_add(MISSILE_CHARGE_RATE)
                    .min(s.max_charge.max(0.) as u32);
                if input.buttons & SIDE_B == 0 || sim.shot_charge as f32 > s.max_charge {
                    self.begin_launch(sim);
                }
            }
            S_LAUNCH => {
                Self::release_gravity(sim);
                sim.stage_frame += 1;
                self.set_missile_damage(sim);
                if sim.action.variables[0] == 1 {
                    self.begin_fly(sim);
                }
            }
            S_FLY => {
                sim.stage_frame += 1;
                self.set_missile_damage(sim);
                // ftLg_SpecialAirS2_Phys: a light fall while the missile flies.
                Self::hold_gravity(sim, 0.);
                sim.velocity.y = (sim.velocity.y - s.fly_gravity * sim.options.jump_scale)
                    .max(-s.fly_terminal * sim.options.jump_scale);
                // A floor or a wall ends the flight (ftLg_SpecialAirS2_Coll). The wall shows as
                // the fighter covering far less ground than his speed asked for.
                let moved = (sim.position - sim.stage_dir).with_y(0.).length();
                sim.missile_travel += moved;
                if sim.action.variables[1] == 0 && sim.missile_travel >= MISSILE_IGNITE_DIST {
                    sim.action.variables[1] = 1;
                    // Fox's Fire Fox launch flame (efalt.c 0x48C, EfCoData model 2004) on the body:
                    // the missile catches fire and hits twice as hard from here on.
                    sim.action.effects.push(crate::action::GfxCue {
                        id: 0x48c,
                        bone: 0,
                        common: false,
                        cycle: 0,
                        destroy_on_change: false,
                        offset: Vec3::ZERO,
                        param: 1.,
                        clip: "SpecialS",
                        frame: 0.,
                        origin: sim.position,
                        facing: sim.facing,
                    });
                }
                let wanted = sim.stage_vec.length();
                let wall = sim.stage_frame > 2 && wanted > 0.05 && moved < wanted * 0.5;
                let landed = sim.stage_frame > 2 && sim.grounded;
                if wall || landed {
                    self.end_missile(sim);
                    return;
                }
                sim.stage_dir = sim.position;
                sim.stage_vec = sim.velocity.with_y(0.);
            }
            _ => {
                sim.stage_frame += 1;
            }
        }
    }
    /// ftLg_SpecialS_Anim: a charged missile hits for charge * slope + tilt percent; a misfire
    /// keeps the clip's own (25%) hitbox.
    fn set_missile_damage(&self, sim: &mut Sim) {
        if sim.stage_value >= 1. {
            return;
        }
        let s = &self.specials;
        let damage = sim.shot_charge as f32 * s.damage_slope + s.damage_tilt;
        let damage = damage * if sim.action.variables[1] != 0 { MISSILE_IGNITE_DAMAGE } else { 1. };
        for active in sim.action.active.values_mut() {
            active.hitbox.damage = damage;
        }
    }

    // --- Super Jump Punch (ftluigispecialhi.c) -----------------------------------
    /// How far the clip's root motion lifts him this tick: the source rises with the animation.
    fn lift(&self, sim: &Sim) -> f32 {
        let clip = self.clip(sim, Move::SpecialHi);
        let frame = if clip == sim.motion.clip { sim.motion.frame } else { 0. };
        sim.motion
            .definitions
            .get(clip)
            .and_then(|d| d.up_delta(frame, sim.motion.rate.max(0.)))
            .unwrap_or(0.)
    }
    fn tick_super_jump(&self, sim: &mut Sim, input: Input) {
        let s = &self.specials;
        sim.stage_frame += 1;
        Self::hold_gravity(sim, 0.);
        let lift = self.lift(sim) * sim.options.jump_scale * UP_B_LIFT_SCALE;
        if lift > 0.001 {
            sim.grounded = false;
        }
        sim.velocity.y = lift;
        // The climb tilts a few degrees with the stick (ftLg_SpecialHi_IASA); that is all
        // the horizontal travel it has.
        let stick = input.movement.with_y(0.).clamp_length_max(1.);
        let magnitude = stick.length();
        if magnitude > s.jump_stick_min && lift > 0. {
            let tilt = (s.jump_angle * (magnitude - s.jump_stick_min)
                / (1. - s.jump_stick_min).max(0.01))
            .to_radians();
            let speed = lift * tilt.tan();
            let dir = stick.normalize();
            sim.velocity.x = dir.x * speed;
            sim.velocity.z = dir.z * speed;
        } else if sim.grounded {
            Self::bleed(sim, 0.2);
        } else {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
    }

    // --- Luigi Cyclone (ftluigispeciallw.c) ---------------------------------------
    /// Moves the horizontal velocity toward `target` by `step`, never past `cap`.
    fn steer(sim: &mut Sim, stick: Vec3, cap: f32, accel: f32) {
        let scale = sim.options.speed_scale;
        let horizontal = sim.velocity.with_y(0.);
        let target = stick * cap * scale;
        let step = if stick.length_squared() > 0.0025 {
            accel * stick.length() * scale
        } else {
            sim.params.friction * scale
        };
        let mut next = horizontal + (target - horizontal).clamp_length_max(step.max(0.));
        let limit = cap * scale;
        if next.length() > limit {
            next = next.normalize() * limit;
        }
        sim.velocity.x = next.x;
        sim.velocity.z = next.z;
    }
    fn tick_cyclone(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        let s = &self.specials;
        sim.stage_frame += 1;
        let tap_window = sim.action.variables[2] == 1;
        let wound = sim.action.variables[0] == 1;
        let spinning = sim.action.variables[3] == 1 && !wound;
        if spinning && sim.stage == LW_WINDUP {
            sim.stage = LW_SPIN;
            sim.stage_len = 0;
            // The host pulls enemies toward him while the spin is up.
            sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 1.);
        }
        if !spinning && sim.stage == LW_SPIN {
            sim.stage = LW_DONE;
            sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 2.);
        }
        if sim.stage == LW_SPIN && !sim.action.active.is_empty() {
            // The first spin hits again and again until its hitboxes are cleared.
            sim.stage_len += 1;
            if sim.stage_len >= CYCLONE_REHIT {
                sim.stage_len = 0;
                for active in sim.action.active.values_mut() {
                    active.generation = active.generation.wrapping_add(1000);
                }
            }
        }
        // Once the spin winds down the speed cap loses x84 each tick.
        if wound {
            sim.stage_vec.x -= s.cyclone_wind_down;
        }
        let stick = input.movement.with_y(0.).clamp_length_max(1.);
        // Taps lift him again and again while the window is open; the air lift is spent for the
        // whole airtime once the spin ends (x222C_cycloneCharge).
        if tap_window && !sim.air_charge && pressed & DOWN_B != 0 {
            sim.velocity.y = sim.velocity.y.max(s.cyclone_tap_lift * sim.options.jump_scale);
            sim.grounded = false;
        }
        if wound && !sim.grounded {
            sim.air_charge = true;
        }
        Self::hold_gravity(sim, 0.);
        if sim.grounded {
            let cap = (s.cyclone_ground_cap + sim.stage_vec.x.min(0.)).max(0.);
            Self::steer(sim, stick, cap, s.cyclone_ground_accel);
            sim.velocity.y = sim.velocity.y.min(0.);
        } else {
            let cap = (s.cyclone_air_cap + sim.stage_vec.x.min(0.)).max(0.);
            Self::steer(sim, stick, cap, s.cyclone_air_accel);
            sim.velocity.y = (sim.velocity.y - sim.params.gravity).max(-sim.params.fall_speed);
        }
    }
}
impl Behavior for Luigi {
    fn slot(&self, m: Move) -> u32 {
        match m {
            Move::SpecialN | Move::SuperScope => 0,
            Move::SpecialS | Move::SpecialS2 => 2,
            Move::SpecialHi => 3,
            Move::SpecialLw => 1,
            _ => 4,
        }
    }
    fn mobile_moves(&self) -> u64 {
        // The scope is held and fired with the upper body, like Samus's Charge Shot:
        // the legs keep the gait layer so Luigi can walk while charging.
        (1u64 << Move::SpecialN as u32) | (1u64 << Move::SuperScope as u32)
    }
    fn upper_body_root(&self) -> Option<usize> {
        Some(5) // LUIGI.upper_body_root: torso root above HipN (arms + head)
    }

    fn select_special(&self, sim: &Sim, input: Input, movement: Vec3) -> Option<Move> {
        // The Super Scope stands in for the Fireball (RT) or the Super Jump Punch (LB),
        // whichever slot the loadout selected it in; the host reports the slot via `host_flags`.
        if sim.scope.is_some() {
            let scope_on_special = sim.host_flags & HOST_SCOPE_ON_SPECIAL != 0;
            if scope_on_special {
                if input.buttons & UP_B != 0 {
                    return Some(Move::SuperScope);
                }
            } else if input.buttons & NEUTRAL_B != 0 {
                return Some(Move::SuperScope);
            }
        }
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
    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, input: Input) {
        let s = &self.specials;
        match m {
            Move::SuperScope => {
                sim.scope_begin();
                // The Scope may be the primary (RT) or the special (LB) alternate; record which
                // button drove it so the hold/charge and release read that button, not RT.
                if let Some(scope) = sim.scope.as_mut() {
                    scope.state.trigger = if input.buttons & UP_B != 0 { UP_B } else { NEUTRAL_B };
                }
            }
            Move::SpecialS => {
                // ftLg_SpecialS_SetVars / _Enter: a smash input starts pre-charged, one launch in
                // x44 misfires, and the wind-up slows him by x18.
                sim.shot_charge = if (sim.flick_age as f32) < s.smash_window {
                    s.charge_start as u32
                } else {
                    0
                };
                let chance = s.misfire_chance.round().max(1.) as u32;
                sim.stage_value = if sim.random(chance) == 0 { 1. } else { 0. };
                sim.velocity.x /= s.start_traction.max(0.01);
                sim.velocity.z /= s.start_traction.max(0.01);
                if !sim.grounded {
                    sim.velocity.y = 0.;
                }
            }
            Move::SpecialHi => {
                // ftLg_SpecialAirHi_Enter: nothing of the air momentum is kept (x64), and he
                // rises with the clip.
                if !sim.grounded {
                    sim.velocity.y = 0.;
                    sim.velocity.x *= s.jump_air_keep;
                    sim.velocity.z *= s.jump_air_keep;
                }
                sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
                Self::hold_gravity(sim, 0.);
            }
            Move::SpecialLw => {
                // ftLg_SpecialLw_Enter: the dip of the spin's first tap, unless the air lift was
                // spent; the horizontal speed is capped by the air cap.
                if !sim.grounded {
                    let dip = if sim.air_charge { 0. } else { s.cyclone_tap_lift };
                    sim.velocity.y = (s.cyclone_tap_momentum - dip) * sim.options.jump_scale;
                    let cap = s.cyclone_air_cap * sim.options.speed_scale;
                    let horizontal = sim.velocity.with_y(0.).clamp_length_max(cap);
                    sim.velocity.x = horizontal.x;
                    sim.velocity.z = horizontal.z;
                }
                sim.stage_vec = Vec3::ZERO;
                let gfx = Self::cue(sim, CYCLONE_GFX, 0);
                sim.action.effects.push(gfx);
                Self::hold_gravity(sim, 0.);
            }
            _ => {}
        }
        // Fireball (SpecialN) spawns through the projectile path; no body sound.
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS | Move::SuperScope)
    }
    fn aim_frozen(&self, sim: &Sim) -> bool {
        sim.current_move == Some(Move::SpecialS) && sim.stage >= S_LAUNCH
    }
    /// The missile's charge frames, for the HUD fraction.
    fn charge_steps(&self) -> u32 {
        self.specials.max_charge.round().max(1.) as u32
    }
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        let mine = matches!(
            sim.current_move,
            Some(Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
        );
        if !mine || sim.phase == Phase::Free {
            // Ended or interrupted: never leave a held gravity override behind.
            Self::release_gravity(sim);
            return;
        }
        match sim.current_move {
            Some(Move::SpecialS) => self.tick_missile(sim, input),
            Some(Move::SpecialHi) => self.tick_super_jump(sim, input),
            Some(Move::SpecialLw) => self.tick_cyclone(sim, host, input, pressed),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) -> bool {
        match (m, sim.stage) {
            (Move::SpecialS, S_START) => {
                sim.stage = S_HOLD;
                sim.stage_frame = 0;
                true
            }
            // The charge loop plays on until the button is released.
            (Move::SpecialS, S_HOLD) => true,
            (Move::SpecialS, S_LAUNCH) => {
                // The clip ran out before its fly flag: fly now.
                self.begin_fly(sim);
                true
            }
            (Move::SpecialS, S_FLY) => {
                // ftLg_SpecialAirS2_Anim: the clip ran out.
                self.end_missile(sim);
                true
            }
            _ => false,
        }
    }
    fn after_script(&self, sim: &mut Sim) {
        if sim.current_move == Some(Move::SpecialS) && matches!(sim.stage, S_LAUNCH | S_FLY) {
            self.set_missile_damage(sim);
        }
    }
    fn on_connect(&self, sim: &mut Sim) {
        // ftLg_SpecialS_OnGiveDamage: the missile stops on what it hits.
        if sim.current_move == Some(Move::SpecialS) && sim.stage == S_FLY {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
            if sim.velocity.y >= 0. {
                sim.velocity.y = 0.;
            }
            self.end_missile(sim);
        }
    }
    fn facing_locked_now(&self, sim: &Sim, m: Move) -> bool {
        match m {
            Move::SpecialS => sim.stage >= S_LAUNCH,
            Move::SpecialHi | Move::SpecialLw => true,
            _ => false,
        }
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        match m {
            Move::SpecialN => {
                if sim.grounded { "SpecialN" } else { "SpecialAirN" }
            }
            Move::SpecialS => match sim.stage {
                S_START => {
                    if sim.grounded { "SpecialSStart" } else { "SpecialAirSStart" }
                }
                S_HOLD => {
                    if sim.grounded { "SpecialSHold" } else { "SpecialAirSHold" }
                }
                // The launch and the flight are one clip, with no air variant.
                S_LAUNCH | S_FLY => "SpecialS",
                _ => {
                    if sim.grounded { "SpecialSEnd" } else { "SpecialAirSEnd" }
                }
            },
            // The source carries the clip's frame across a ground/air change; here the clip
            // stays the one the move started with so it never restarts mid-spin.
            Move::SpecialHi => {
                if sim.move_started_grounded { "SpecialHi" } else { "SpecialAirHi" }
            }
            Move::SpecialLw => {
                if sim.move_started_grounded { "SpecialLw" } else { "SpecialAirLw" }
            }
            _ => "",
        }
    }
    fn spins_whole_body(&self, m: Move) -> bool {
        matches!(m, Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
    }
    fn hit_attachment(&self, _sim: &Sim, m: Move) -> (Option<usize>, Vec3) {
        // The scope's beam leaves from the right hand holding it (FtPart_RThumbNb, through the common bone table).
        if m == Move::SuperScope {
            (Some(0x1_0000 + 49), Vec3::ZERO)
        } else {
            (None, Vec3::ZERO)
        }
    }
    fn projectile_move(&self, m: Move) -> bool {
        m == Move::SpecialN
    }
    fn special_fall(&self, m: Move) -> Option<SpecialFall> {
        // ftLg_SpecialHi_Anim -> ftCo_80096900: helpless at x50 drift, then x54 landing lag.
        (m == Move::SpecialHi).then_some(SpecialFall {
            mobility: self.specials.jump_freefall,
            landing_lag: self.specials.jump_landing_lag,
        })
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(8),
            Move::SpecialS => Some(12),
            Move::SpecialHi => Some(10),
            Move::SpecialLw => Some(2),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(20),
            Move::SpecialS => Some(30),
            Move::SpecialHi => Some(30),
            Move::SpecialLw => Some(40),
            _ => None,
        }
    }
}
