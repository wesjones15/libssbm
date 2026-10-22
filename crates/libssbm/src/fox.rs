//! Fox-specific special-move behavior: Blaster, Fox Illusion, Fire Fox and
//! Reflector. Each is the source's multi-state motion machine reimplemented as
//! stages of one staged special (`Sim::stage`), driving the disc's own clips;
//! tuning comes from ftFox_DatAttrs on the player's disc. References:
//! ftFox/ftfoxspecial{n,s,hi,lw}.c, it/kinds/itfoxillusion.c.
//!
//! RoR2 adaptations, marked where they occur: the camera aim stands in for the
//! 2D control stick (Fire Fox's launch direction, the Blaster's aim), Fire Fox
//! and the Illusion dash travel further than the 60 Hz source for open worlds,
//! and the Reflector's frame-1 burst is a host area hit.
use crate::{
    action::{ActiveHitbox, GfxCue, Hitbox, SoundCue},
    behavior::Behavior,
    sim::{EventKind, Host, Input, Move, Phase, Sim, DOWN_B, JUMP, NEUTRAL_B, SIDE_B, UP_B},
};
use glam::Vec3;

/// Fox's special tuning, separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    pub blaster_angle: f32,
    pub blaster_speed: f32,
    pub illusion_velocity: f32,
    pub fire_fox_velocity: f32,
    pub reflector_frames: u32,
    /// Illusion's velocity after the dash ends (x34 ground, x3C air) and the
    /// ground friction that bleeds it off (x38).
    pub illusion_ground_end: f32,
    pub illusion_friction: f32,
    pub illusion_air_end: f32,
    /// Fire Fox: frames of travel (x68), the frame the travel starts to brake
    /// (x70) and the braking acceleration (x78).
    pub fire_fox_duration: f32,
    pub fire_fox_duration_end: f32,
    pub fire_fox_reverse_accel: f32,
    /// Frames before the Reflector lets gravity act in the air (xA4).
    pub reflector_gravity_delay: u32,
    /// The Reflector's ReflectDesc (xB0): bubble size (xC4), the damage a
    /// reflected shot is multiplied by (xC8) and its speed multiplier (xCC).
    pub reflect_size: f32,
    pub reflect_damage_mul: f32,
    pub reflect_speed_mul: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            blaster_angle: 0.,
            blaster_speed: 20.,
            illusion_velocity: 12.,
            fire_fox_velocity: 3.,
            reflector_frames: 24,
            illusion_ground_end: 1.,
            illusion_friction: 0.1,
            illusion_air_end: 0.5,
            fire_fox_duration: 33.,
            fire_fox_duration_end: 30.,
            fire_fox_reverse_accel: 0.3,
            reflector_gravity_delay: 5,
            reflect_size: 12.,
            reflect_damage_mul: 1.5,
            reflect_speed_mul: 1.,
        }
    }
}

// Blaster: SpecialNStart, SpecialNLoop (a shot at frame 5, repeating while the
// button is held), SpecialNEnd.
//
// Repeat rule (host adaptation of ftfoxspecialn.c). In the source a shot repeats only
// when B is pressed while the Start or Loop clip's input window is open
// (ftFox_SpecialN_CheckLoopInput sets isBlasterLoop); the Loop clip, 10 frames, then plays
// again (ftFx_SpecialNLoop_Anim) and releases the next shot at its frame 5. Here:
//  * Held: the button keeps isBlasterLoop set, so the Loop clip replays back to back, one
//    shot per clip length (10 ticks, 1/6 s). This is the source's own cadence.
//  * Tapped: every press owes one shot. It leaves as soon as the minimum gap allows,
//    by replaying the Loop clip from its frame 0 early (or from the End clip), so fast
//    clicking fires faster than holding. Shots are never closer than BLASTER_MIN_GAP.
const N_START: u32 = 0;
const N_LOOP: u32 = 1;
const N_END: u32 = 2;
// Illusion: SpecialSStart, the SpecialS dash, SpecialSEnd.
const S_START: u32 = 0;
const S_DASH: u32 = 1;
const S_END: u32 = 2;
// Fire Fox: SpecialHiHold (the charge in place), SpecialHi (the launch),
// SpecialHiFall / SpecialHiLanding.
const HI_HOLD: u32 = 0;
const HI_GO: u32 = 1;
const HI_END: u32 = 2;
// Reflector: SpecialLwStart, SpecialLwLoop (held), SpecialLwEnd.
const LW_START: u32 = 0;
const LW_LOOP: u32 = 1;
const LW_END: u32 = 2;

/// FtPart indices into the common bone table (ft/forward.h).
const PART_TOP_N: usize = 0;
const PART_TRANS_N: usize = 1;
const PART_HIP_N: usize = 4;
/// FtPart_RThumbNb, flagged so the host resolves it through the common bone table.
const ATTACH_RIGHT_THUMB: usize = 0x1_0000 + 49;

/// Illusion's dash covers the TransN travel of its 4-frame clip (78 units) in
/// the source; here a little further, over a few more ticks so the sweep never
/// tunnels (host adaptation).
const ILLUSION_TRAVEL: f32 = 96.;
const ILLUSION_FRAMES: u32 = 6;
/// The Illusion article's hitbox (itFoxIllusion, article 2): 7% slash, angle 80,
/// bkb 68, growth 40 grounded / 60 aerial. Widened to the RoR2 body size (host
/// adaptation); it follows Fox for the dash so everything in the path is hit once.
const ILLUSION_BOX: u32 = 90;
const ILLUSION_GROUP: u32 = 3;
/// Fire Fox travels further than the source (host adaptation) and hits harder.
const FIREFOX_HOST_SCALE: f32 = 2.0;
const FIREFOX_DAMAGE_SCALE: f32 = 1.4;
const FIREFOX_RADIUS: f32 = 9.;
/// Ticks from replaying the Loop clip to the shot leaving: the clip releases it at frame 5
/// (its cmd_vars[2] script command), and the restart tick already plays frame 1.
const BLASTER_FIRE_DELAY: f32 = 4.;
/// The least time between two Blaster shots when tapping, in ticks: 0.12 s is 7.2 ticks and
/// shots land on whole ticks, so 7 (0.117 s) keeps a 0.12 s tap rhythm firing every press
/// (host design, not a source value).
const BLASTER_MIN_GAP: f32 = 7.;
/// Blaster fire sound (ftfoxspecialn.c::foxSFX[0], facing-forward id).
const BLASTER_SOUND: u32 = 110103;

pub struct Fox {
    specials: Specials,
}
impl Fox {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Fox behavior for headless tests.
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
    fn cue(sim: &Sim, id: u32, bone: usize, destroy_on_change: bool) -> GfxCue {
        GfxCue {
            id,
            bone,
            common: true, // FtPart index into the common bone table
            cycle: 0,
            destroy_on_change,
            offset: Vec3::ZERO,
            param: 0.,
            clip: sim.motion.clip,
            frame: sim.motion.frame,
            origin: sim.position,
            facing: sim.facing,
        }
    }

    // --- Illusion (ftfoxspecials.c, itfoxillusion.c) --------------------------
    fn begin_dash(&self, sim: &mut Sim) {
        let heading = sim.facing.with_y(0.).normalize_or_zero();
        let travel = ILLUSION_TRAVEL * sim.options.speed_scale;
        sim.stage = S_DASH;
        sim.stage_frame = 0;
        sim.stage_len = ILLUSION_FRAMES;
        sim.stage_vec = heading * (travel / ILLUSION_FRAMES as f32);
        sim.velocity = sim.stage_vec;
        Self::hold_gravity(sim, 0.);
        // ftFx_SpecialS_CreateGFX (efSync 0x48D on TopN): the blue streak.
        let streak = Self::cue(sim, 1165, PART_TOP_N, false);
        sim.action.effects.push(streak);
        let grounded = sim.grounded;
        let group = ILLUSION_GROUP as usize;
        sim.action.generations[group] = sim.action.generations[group].wrapping_add(1);
        let generation = sim.action.generations[group];
        sim.action.active.insert(
            ILLUSION_BOX,
            ActiveHitbox {
                hitbox: Hitbox {
                    id: ILLUSION_BOX,
                    group: ILLUSION_GROUP,
                    bone: 0,
                    common_bone: false,
                    damage: 7.,
                    radius: 10.,
                    offset: Vec3::new(0., 6., 0.),
                    angle: 80,
                    growth: if grounded { 40 } else { 60 },
                    fixed_knockback: 0,
                    base_knockback: 68,
                    element: 3,
                    sfx_kind: 3,
                    sfx_severity: 2,
                    ground: true,
                    air: true,
                },
                generation,
            },
        );
    }
    fn end_dash(&self, sim: &mut Sim) {
        sim.action.active.remove(&ILLUSION_BOX);
        let heading = sim.stage_vec.with_y(0.).normalize_or_zero();
        let end = if sim.grounded {
            self.specials.illusion_ground_end
        } else {
            self.specials.illusion_air_end
        };
        sim.velocity = heading * end * sim.options.speed_scale;
        sim.stage = S_END;
        sim.stage_frame = 0;
        Self::release_gravity(sim);
    }

    // --- Fire Fox (ftfoxspecialhi.c) ------------------------------------------
    fn begin_launch(&self, sim: &mut Sim) {
        // RoR2 adaptation: the launch follows the camera aim (the source reads a
        // 360-degree stick angle), pitch included.
        let mut dir = if sim.move_aim.length_squared() > 0.001 {
            sim.move_aim.normalize()
        } else {
            Vec3::Y
        };
        if sim.grounded && dir.y < 0. {
            dir.y = 0.;
            dir = dir.normalize_or_zero();
            if dir == Vec3::ZERO {
                dir = sim.facing;
            }
        }
        let heading = dir.with_y(0.);
        if heading.length_squared() > 0.01 {
            sim.facing = heading.normalize();
        }
        let speed = self.specials.fire_fox_velocity * FIREFOX_HOST_SCALE * sim.options.speed_scale;
        sim.stage = HI_GO;
        sim.stage_frame = 0;
        sim.stage_len = self.specials.fire_fox_duration.round().max(1.) as u32;
        sim.stage_dir = dir;
        sim.stage_value = speed;
        sim.velocity = dir * speed;
        if dir.y > 0.01 {
            sim.grounded = false;
        }
        Self::hold_gravity(sim, 0.);
        // The source spends the double jump on the launch (x1968_jumpsUsed).
        sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
        // ftFx_SpecialHi_CreateLaunchGFX: efSync 1164 on the hip.
        let flame = Self::cue(sim, 1164, PART_HIP_N, false);
        sim.action.effects.push(flame);
    }
    fn end_launch(&self, sim: &mut Sim) {
        if sim.stage == HI_END {
            return;
        }
        sim.stage = HI_END;
        sim.stage_frame = 0;
        sim.velocity = sim.stage_dir * sim.stage_value * 0.25;
        Self::release_gravity(sim);
    }

    // --- Per-move ticks ---------------------------------------------------------
    /// `stage_len` counts ticks since the last shot (zeroed in `fire_strength`);
    /// `stage_frame` is 1 while a tap's shot is still owed; `stage_value` is the source's
    /// isBlasterLoop (replay the Loop clip when this pass ends).
    fn tick_blaster(&self, sim: &mut Sim, input: Input, pressed: u32) {
        if sim.grounded {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
        sim.stage_len = sim.stage_len.saturating_add(1);
        // isBlasterLoop: a held button keeps the Loop clip replaying at its own length.
        if sim.stage == N_LOOP && input.buttons & NEUTRAL_B != 0 {
            sim.stage_value = 1.;
        }
        // A pass whose shot has not left yet (Start, or Loop before its fire frame) cannot
        // restart; once the shot is out, an owed tap replays the Loop clip as soon as the new
        // shot (BLASTER_FIRE_DELAY ticks later) keeps the minimum gap. This runs before this
        // tick's press is recorded, so a press never merges into the tap it follows.
        let shot_out = sim.stage != N_START && sim.action.projectile_fired;
        if sim.stage_frame == 1
            && shot_out
            && sim.stage_len as f32 + BLASTER_FIRE_DELAY >= BLASTER_MIN_GAP
        {
            sim.stage = N_LOOP;
            sim.stage_frame = 0;
            sim.stage_value = 0.;
            sim.motion.frame = 0.;
            sim.action.projectile_fired = false;
        }
        if pressed & NEUTRAL_B != 0 {
            sim.stage_frame = 1;
        }
    }
    fn tick_illusion(&self, sim: &mut Sim) {
        match sim.stage {
            S_START => {
                // Gravity waits through the start; momentum is divided by x28.
                Self::hold_gravity(sim, 0.);
                if sim.stage_frame == 0 {
                    let divisor = self.specials.illusion_velocity.max(1.);
                    sim.velocity.x /= divisor;
                    sim.velocity.z /= divisor;
                    sim.velocity.y = 0.;
                }
                sim.stage_frame += 1;
            }
            S_DASH => {
                sim.stage_frame += 1;
                sim.velocity = sim.stage_vec;
                Self::hold_gravity(sim, 0.);
                if sim.stage_frame >= sim.stage_len {
                    self.end_dash(sim);
                }
            }
            _ => {
                sim.stage_frame += 1;
            }
        }
    }
    fn tick_firefox(&self, sim: &mut Sim) {
        match sim.stage {
            HI_HOLD => {
                // The fireball charges in place: the fighter holds position in
                // space (the source zeroes vertical speed and delays gravity).
                sim.velocity = Vec3::ZERO;
                Self::hold_gravity(sim, 0.);
                sim.stage_frame += 1;
            }
            HI_GO => {
                sim.stage_frame += 1;
                if sim.stage_frame >= self.specials.fire_fox_duration_end.round() as u32 {
                    // ftFx_SpecialHi_Phys: after DURATION_END ticks the reverse
                    // acceleration brakes the travel.
                    let accel = self.specials.fire_fox_reverse_accel * FIREFOX_HOST_SCALE;
                    sim.stage_value = (sim.stage_value - accel).max(0.);
                }
                sim.velocity = sim.stage_dir * sim.stage_value;
                Self::hold_gravity(sim, 0.);
                // The disc's launch hitbox (14%), enlarged and strengthened for
                // RoR2 once the clip's frame-0 script has created it.
                let first = sim.stage_frame == 2;
                for active in sim.action.active.values_mut() {
                    active.hitbox.radius = active.hitbox.radius.max(FIREFOX_RADIUS);
                    if first {
                        active.hitbox.damage *= FIREFOX_DAMAGE_SCALE;
                    }
                }
                let landed = sim.stage_frame > 2 && sim.grounded && sim.stage_dir.y < -0.05;
                if sim.stage_frame >= sim.stage_len || landed {
                    self.end_launch(sim);
                }
            }
            _ => {
                sim.stage_frame += 1;
            }
        }
    }
    fn tick_reflector(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        // A jump cancels the shine at any point it is up (the combo tool of
        // the source's ftFx_SpecialLw_*_IASA): the move ends and the Free
        // phase that runs next in the same tick takes the jump.
        if matches!(sim.stage, LW_START | LW_LOOP) && pressed & JUMP != 0 {
            sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 2.);
            Self::release_gravity(sim);
            sim.interrupt();
            return;
        }
        if matches!(sim.stage, LW_START | LW_LOOP) {
            sim.stage_frame += 1;
            if input.buttons & DOWN_B == 0 {
                sim.stage_value = 1.; // isRelease
            }
            if sim.grounded {
                sim.velocity.x = 0.;
                sim.velocity.z = 0.;
            } else if sim.stage_frame <= self.specials.reflector_gravity_delay {
                sim.velocity.y = 0.;
                Self::hold_gravity(sim, 0.);
            } else {
                Self::release_gravity(sim);
            }
            // Released with the minimum lag spent (x98): the End clip.
            if sim.stage == LW_LOOP
                && sim.stage_value >= 1.
                && sim.stage_frame >= self.specials.reflector_frames
            {
                sim.stage = LW_END;
                sim.stage_frame = 0;
                Self::release_gravity(sim);
                // The bubble is down: the host stops reflecting.
                sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 2.);
            }
        }
    }
}
impl Behavior for Fox {
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
    fn enter_special(&self, sim: &mut Sim, host: &mut dyn Host, m: Move, _input: Input) {
        match m {
            Move::SpecialN => {
                // ftFx_SpecialN_Enter: a grounded Fox stops to shoot.
                if sim.grounded {
                    sim.velocity.x = 0.;
                    sim.velocity.z = 0.;
                }
            }
            Move::SpecialHi => {
                // ftFx_SpecialHi_Enter: the charge GFX (efSync 1163 on TransN).
                sim.velocity = Vec3::ZERO;
                Self::hold_gravity(sim, 0.);
                let charge = Self::cue(sim, 1163, PART_TRANS_N, true);
                sim.action.effects.push(charge);
            }
            Move::SpecialLw => {
                // Reflector: a frame-1 shine centred on the hip
                // (ftFx_SpecialLw_CreateStartGFX, efSync 1161), and the host's area
                // burst around Fox.
                if sim.grounded {
                    sim.velocity.x = 0.;
                    sim.velocity.z = 0.;
                } else {
                    sim.velocity.y = 0.;
                }
                let shine = Self::cue(sim, 1161, PART_HIP_N, false);
                sim.action.effects.push(shine);
                // The event's direction carries the ReflectDesc (bubble size,
                // damage multiplier, speed multiplier) for the host's reflection.
                let aim = sim.move_aim;
                sim.move_aim = Vec3::new(
                    self.specials.reflect_size,
                    self.specials.reflect_damage_mul,
                    self.specials.reflect_speed_mul,
                );
                sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 1.);
                sim.move_aim = aim;
            }
            _ => {}
        }
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS | Move::SpecialHi)
    }
    fn aim_frozen(&self, sim: &Sim) -> bool {
        match sim.current_move {
            Some(Move::SpecialS) => sim.stage >= S_DASH,
            Some(Move::SpecialHi) => sim.stage >= HI_GO,
            _ => false,
        }
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
    fn resolve_startup(&self, _sim: &mut Sim, _input: Input) -> bool {
        false
    }
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32 {
        if m == Move::SpecialN {
            sim.stage_len = 0; // ticks since the last shot (see tick_blaster)
            sim.action.sounds.push(SoundCue {
                sound: BLASTER_SOUND,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
        }
        sim.move_strength
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

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        let mine = matches!(
            sim.current_move,
            Some(Move::SpecialN | Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
        );
        if !mine || sim.phase == Phase::Free {
            // Ended or interrupted: never leave a held gravity override or the
            // Illusion hitbox behind.
            Self::release_gravity(sim);
            sim.action.active.remove(&ILLUSION_BOX);
            return;
        }
        match sim.current_move {
            Some(Move::SpecialN) => self.tick_blaster(sim, input, pressed),
            Some(Move::SpecialS) => self.tick_illusion(sim),
            Some(Move::SpecialHi) => self.tick_firefox(sim),
            Some(Move::SpecialLw) => self.tick_reflector(sim, host, input, pressed),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) -> bool {
        match (m, sim.stage) {
            (Move::SpecialN, N_START) => {
                // stage_frame (a tap owed during the Start clip) carries into the Loop.
                sim.stage = N_LOOP;
                sim.action.projectile_fired = false;
                true
            }
            (Move::SpecialN, N_LOOP) => {
                if sim.stage_value >= 1. {
                    // Another shot: the Loop clip restarts and fires at its frame 5.
                    sim.stage_value = 0.;
                    sim.motion.frame = 0.;
                    sim.action.projectile_fired = false;
                } else {
                    sim.stage = N_END;
                }
                true
            }
            (Move::SpecialS, S_START) => {
                self.begin_dash(sim);
                true
            }
            (Move::SpecialS, S_DASH) => {
                self.end_dash(sim);
                true
            }
            (Move::SpecialHi, HI_HOLD) => {
                self.begin_launch(sim);
                true
            }
            // The launch is timed by ticks, not by its 30-frame clip.
            (Move::SpecialHi, HI_GO) => true,
            (Move::SpecialLw, LW_START) => {
                sim.stage = LW_LOOP;
                // ftFx_SpecialLw_CreateLoopGFX: efSync 1160 on the hip while held.
                let hold = Self::cue(sim, 1160, PART_HIP_N, true);
                sim.action.effects.push(hold);
                true
            }
            (Move::SpecialLw, LW_LOOP) => true,
            _ => false,
        }
    }
    fn projectile_variable(&self) -> Option<u32> {
        // The Loop clip raises cmd_vars[2] on the frame a shot leaves the gun.
        Some(2)
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        let now = sim.grounded;
        match (m, sim.stage) {
            (Move::SpecialN, N_START) => {
                if now { "SpecialN" } else { "SpecialAirN" }
            }
            (Move::SpecialN, N_LOOP) => {
                if now { "SpecialNLoop" } else { "SpecialAirNLoop" }
            }
            (Move::SpecialN, _) => {
                if now { "SpecialNEnd" } else { "SpecialAirNEnd" }
            }
            (Move::SpecialS, S_START) => {
                if now { "SpecialSStart" } else { "SpecialAirSStart" }
            }
            (Move::SpecialS, S_DASH) => {
                if now { "SpecialS" } else { "SpecialAirS" }
            }
            (Move::SpecialS, _) => {
                if now { "SpecialSEnd" } else { "SpecialAirSEnd" }
            }
            (Move::SpecialHi, HI_HOLD) => {
                if now { "SpecialHiHold" } else { "SpecialHiHoldAir" }
            }
            (Move::SpecialHi, HI_GO) => "SpecialHi",
            (Move::SpecialHi, _) => {
                if now { "SpecialHiLanding" } else { "SpecialHiFall" }
            }
            (Move::SpecialLw, LW_START) => {
                if now { "SpecialLw" } else { "SpecialAirLw" }
            }
            (Move::SpecialLw, LW_LOOP) => {
                if now { "SpecialLwLoop" } else { "SpecialAirLwLoop" }
            }
            (Move::SpecialLw, _) => {
                if now { "SpecialLwEnd" } else { "SpecialAirLwEnd" }
            }
            _ => "",
        }
    }
    fn landing(&self, _sim: &Sim, m: Move) -> &'static str {
        if m == Move::SpecialHi { "LandingFallSpecial" } else { "Landing" }
    }
    fn facing_locked(&self, m: Move) -> bool {
        m == Move::SpecialLw
    }
    fn facing_locked_now(&self, sim: &Sim, m: Move) -> bool {
        self.facing_locked(m)
            || (m == Move::SpecialS && sim.stage >= S_DASH)
            || (m == Move::SpecialHi && sim.stage >= HI_GO)
    }
    fn spins_whole_body(&self, m: Move) -> bool {
        matches!(m, Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
    }
    fn side_special_layered(&self, _m: Move) -> bool {
        false
    }
    fn side_special_upper_aim(&self, _sim: &Sim) -> Option<(f32, f32)> {
        None
    }
    fn hit_attachment(&self, _sim: &Sim, m: Move) -> (Option<usize>, Vec3) {
        // The Blaster's shots leave from the right hand holding the gun.
        if m == Move::SpecialN {
            (Some(ATTACH_RIGHT_THUMB), Vec3::ZERO)
        } else {
            (None, Vec3::ZERO)
        }
    }
    fn projectile_move(&self, m: Move) -> bool {
        m == Move::SpecialN
    }
    fn projectile_cue(&self, _m: Move) -> Option<(u32, usize, bool)> {
        None
    }
    fn phase_effect(&self, _sim: &Sim, _phase: Phase) -> Option<GfxCue> {
        None
    }
    fn phase_stop(&self, _phase: Phase) -> Option<SoundCue> {
        None
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(8),
            Move::SpecialS => Some(12),
            Move::SpecialHi => Some(10),
            Move::SpecialLw => Some(1),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(20),
            Move::SpecialS => Some(30),
            Move::SpecialHi => Some(30),
            Move::SpecialLw => Some(self.specials.reflector_frames),
            _ => None,
        }
    }
}
