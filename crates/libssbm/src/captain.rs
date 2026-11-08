//! Captain Falcon-specific special-move behavior: Falcon Punch, Raptor Boost,
//! Falcon Dive and Falcon Kick. Raptor Boost, the Dive and the Kick are staged,
//! driving the disc's own clips, and take their forward travel from the clips'
//! root motion (the source moves him with the animation). Tuning reads PlCa.dat
//! ext_attr (ftCaptain_DatAttrs). References:
//! ftCaptain/ftcaptainspecial{n,s,hi,lw}.c.
//!
//! RoR2 adaptations, marked where they occur: the lunge, dive rebound and kick
//! travel further than the source for open worlds, the camera aim stands in for
//! the stick, and the host grabs, burns and launches enemies.
use crate::{
    behavior::Behavior,
    sim::{EventKind, Host, Input, Move, Phase, Sim},
};
use glam::Vec3;

/// Captain Falcon's special tuning, separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Falcon Punch momentum (ftCaptain_DatAttrs.xC specialn_vel_x).
    pub punch_momentum: f32,
    /// What the Raptor Boost's speed is multiplied by when it connects (x14
    /// specials_gr_vel_x; it is a factor, not a speed).
    pub raptor_hit_mult: f32,
    /// Aerial Raptor Boost gravity (x18) and terminal speed (x1C).
    pub raptor_grav: f32,
    pub raptor_terminal: f32,
    /// Falcon Dive launch speed (ftCaptain_DatAttrs.x44 specialhi_horz_vel).
    pub dive_velocity: f32,
}
impl Specials {
    /// Synthetic baseline for tests and built-in play; disc-backed play loads tuning.
    pub fn fixture() -> Self {
        Self {
            punch_momentum: 2.4,
            raptor_hit_mult: 0.2,
            raptor_grav: 0.1,
            raptor_terminal: 2.,
            dive_velocity: 3.2,
        }
    }
}

// Raptor Boost: SpecialSStart (the lunge, trigger boxes), SpecialS (the hit).
const RB_START: u32 = 0;
const RB_HIT: u32 = 1;
// Falcon Dive: SpecialHi (the leap and grab), SpecialHiCatch (the explosion),
// SpecialHiThrow (the launch and the rebound).
const HI_RISE: u32 = 0;
const HI_CATCH: u32 = 1;
const HI_THROW: u32 = 2;
// Falcon Kick: SpecialLw (the kick), SpecialLwEnd (the recovery).
const LW_KICK: u32 = 0;
const LW_END: u32 = 1;

/// The Raptor Boost lunge, the kick and the dive's rebound cover this much more
/// ground than the source's clips (host adaptation).
const TRAVEL_SCALE: f32 = 1.5;
/// Facing stops following the camera at the Raptor Boost lunge's trigger frame and
/// at the Falcon Punch's hit frame (the reversal window is the wind-up).
const RAPTOR_LOCK_FRAME: f32 = 14.;
const PUNCH_LOCK_FRAME: f32 = 48.;
/// The aerial punch's launch angle follows the camera pitch within this many degrees.
const PUNCH_ANGLE: f32 = 30.;

pub struct Captain {
    specials: Specials,
}
impl Captain {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Captain behavior for headless tests.
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
    /// How far the clip's root motion carries him forward this tick (the source
    /// moves Falcon with the animation: ft_80084FA8 and the dash attack alike).
    fn root_step(&self, sim: &Sim, m: Move) -> f32 {
        let clip = self.clip(sim, m);
        let frame = if clip == sim.motion.clip { sim.motion.frame } else { 0. };
        sim.motion
            .definitions
            .get(clip)
            .and_then(|d| d.forward_delta(frame, sim.motion.rate.max(0.)))
            .unwrap_or(0.)
    }
    fn heading(sim: &Sim) -> Vec3 {
        sim.facing.with_y(0.).normalize_or_zero()
    }

    // --- Falcon Punch --------------------------------------------------------------
    fn tick_punch(&self, sim: &mut Sim) {
        // ftCaptain_SpecialN: the punch's wind (efSync 1167, two models) as the fist's
        // hitbox comes up.
        if sim.stage_vec.x == 0. && !sim.action.active.is_empty() {
            sim.stage_vec.x = 1.;
            let wind = crate::effect::part_cue(sim, 1167, crate::effect::part::TOP_N);
            sim.action.effects.push(wind);
        }
        if sim.grounded {
            // The ground punch steps forward with the clip.
            let step = self.root_step(sim, Move::SpecialN) * sim.options.speed_scale;
            let heading = Self::heading(sim);
            sim.velocity.x = heading.x * step;
            sim.velocity.z = heading.z * step;
            Self::hold_gravity(sim, 0.);
        } else if sim.stage_value < 1. && sim.action.variables[0] == 1 {
            // ftCa_SpecialAirN_IASA: at the punch the air version launches him along the
            // stick angle; here, the camera pitch.
            sim.stage_value = 1.;
            let pitch = sim.move_aim.y.clamp(-1., 1.).asin().to_degrees().clamp(-PUNCH_ANGLE, PUNCH_ANGLE).to_radians();
            let heading = Self::heading(sim);
            let speed = self.specials.punch_momentum * sim.options.speed_scale;
            sim.velocity = heading * pitch.cos() * speed + Vec3::Y * pitch.sin() * speed;
        }
    }

    // --- Raptor Boost --------------------------------------------------------------
    fn tick_raptor(&self, sim: &mut Sim) {
        sim.stage_frame += 1;
        let scale = sim.options.speed_scale * TRAVEL_SCALE;
        let heading = Self::heading(sim);
        let step = self.root_step(sim, Move::SpecialS);
        match sim.stage {
            RB_START => {
                sim.velocity.x = heading.x * step * scale;
                sim.velocity.z = heading.z * step * scale;
                if sim.move_started_grounded {
                    sim.velocity.y = 0.;
                    Self::hold_gravity(sim, 0.);
                } else if sim.action.variables[1] == 1 {
                    // ftCa_SpecialAirSStart_Phys: once the lunge is spent he falls.
                    sim.stage_value = (sim.stage_value - self.specials.raptor_grav).max(-self.specials.raptor_terminal);
                    sim.velocity.y = sim.stage_value;
                    Self::hold_gravity(sim, 0.);
                } else {
                    sim.velocity.y = 0.;
                    Self::hold_gravity(sim, 0.);
                }
                // The air lunge ends if he touches down.
                if !sim.move_started_grounded && sim.grounded && sim.stage_frame > 4 {
                    Self::release_gravity(sim);
                    sim.interrupt();
                }
            }
            _ => {
                if sim.move_started_grounded {
                    sim.velocity.x = heading.x * step * scale * 0.5;
                    sim.velocity.z = heading.z * step * scale * 0.5;
                    Self::hold_gravity(sim, 0.);
                } else {
                    // The spike: he drops with what he hit.
                    sim.velocity.x *= 0.8;
                    sim.velocity.z *= 0.8;
                    Self::release_gravity(sim);
                }
            }
        }
    }

    // --- Falcon Dive ---------------------------------------------------------------
    fn tick_dive(&self, sim: &mut Sim, host: &mut dyn Host) {
        sim.stage_frame += 1;
        match sim.stage {
            HI_RISE => {}
            HI_CATCH => {
                // Held in place while the explosion goes off.
                sim.velocity = Vec3::ZERO;
                Self::hold_gravity(sim, 0.);
                if sim.stage_frame == 1 {
                    sim.emit(host, EventKind::Hit, Move::SpecialHi as u32, 1.);
                }
            }
            _ => {
                // The rebound: the clip's backward travel and a hop away, then he falls.
                let step = self.root_step(sim, Move::SpecialHi) * sim.options.speed_scale * TRAVEL_SCALE;
                let heading = Self::heading(sim);
                sim.velocity.x = heading.x * step;
                sim.velocity.z = heading.z * step;
                if sim.stage_frame < 12 {
                    sim.velocity.y = 0.5 * sim.options.speed_scale;
                    Self::hold_gravity(sim, 0.);
                } else {
                    Self::release_gravity(sim);
                }
            }
        }
    }

    // --- Falcon Kick ---------------------------------------------------------------
    fn tick_kick(&self, sim: &mut Sim) {
        sim.stage_frame += 1;
        let scale = sim.options.speed_scale * TRAVEL_SCALE;
        let heading = Self::heading(sim);
        match sim.stage {
            LW_KICK => {
                let step = self.root_step(sim, Move::SpecialLw);
                sim.velocity.x = heading.x * step * scale;
                sim.velocity.z = heading.z * step * scale;
                if sim.grounded {
                    Self::hold_gravity(sim, 0.);
                } else {
                    // The aerial kick dives diagonally: as far down as forward.
                    sim.velocity.y = -step * scale;
                    Self::hold_gravity(sim, 0.);
                }
                // Landing from the dive goes straight into the recovery.
                if !sim.move_started_grounded && sim.grounded && sim.stage_frame > 3 {
                    sim.stage = LW_END;
                    sim.stage_frame = 0;
                }
            }
            _ => {
                sim.velocity.x *= 0.8;
                sim.velocity.z *= 0.8;
                if sim.grounded {
                    Self::hold_gravity(sim, 0.);
                } else {
                    Self::release_gravity(sim);
                }
            }
        }
    }
}
impl Behavior for Captain {
    fn mobile_moves(&self) -> u64 {
        0
    }

    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) {
        if m == Move::SpecialHi {
            // Falcon Dive rises to the host's standard jump apex, like every
            // other recovery (ftcaptainspecialhi.c::ftCa_SpecialHi_Enter).
            sim.pad_gravity = None;
            sim.grounded = false;
            sim.velocity.y = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity)
            } else {
                self.specials.dive_velocity * sim.options.jump_scale
            };
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
        } else if m == Move::SpecialS {
            // Raptor Boost: ftcaptainspecials.c::ftCa_SpecialS_Enter zeroes velocity,
            // then the lunge's root motion carries Falcon.
            sim.velocity = Vec3::ZERO;
            // ftCa_SpecialS_Enter: the spark on his head (efSync 1169, HeadN).
            let spark = crate::effect::part_cue(sim, 1169, crate::effect::part::HEAD_N);
            sim.action.effects.push(spark);
        } else if m == Move::SpecialLw {
            // Falcon Kick: a dash kick with a trailing flame.
            // ftcaptainspeciallw.c::ftCa_SpecialHi_800E3EAC (efAsync 0x490).
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
            let flame = crate::effect::part_cue(sim, 1168, crate::effect::part::R_FOOT_JA);
            sim.action.effects.push(flame);
        } else if m == Move::SpecialN {
            // Falcon Punch: a single devastating punch. The flame at the fist is
            // authored by the disc script (efasync 0x446 and 0x3FF).
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
    }
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, _input: Input, _pressed: u32) {
        let mine = matches!(
            sim.current_move,
            Some(Move::SpecialN | Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
        );
        if !mine || sim.phase == Phase::Free {
            // Ended or interrupted: never leave a held gravity override behind.
            Self::release_gravity(sim);
            return;
        }
        match sim.current_move {
            Some(Move::SpecialN) => self.tick_punch(sim),
            Some(Move::SpecialS) => self.tick_raptor(sim),
            Some(Move::SpecialHi) => self.tick_dive(sim, host),
            Some(Move::SpecialLw) => self.tick_kick(sim),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, host: &mut dyn Host, m: Move, _input: Input) -> bool {
        match (m, sim.stage) {
            (Move::SpecialHi, HI_CATCH) => {
                // The explosion launch (the throw): the host sends the victim flying.
                sim.stage = HI_THROW;
                sim.stage_frame = 0;
                sim.emit(host, EventKind::Hit, Move::SpecialHi as u32, 2.);
                true
            }
            (Move::SpecialLw, LW_KICK) => {
                sim.stage = LW_END;
                sim.stage_frame = 0;
                true
            }
            _ => false,
        }
    }
    fn on_connect(&self, sim: &mut Sim) {
        match (sim.current_move, sim.stage) {
            // The lunge's trigger boxes touched something: the uppercut (or the spike).
            (Some(Move::SpecialS), RB_START) => {
                sim.stage = RB_HIT;
                sim.stage_frame = 0;
                // ftCa_SpecialS_Anim: the boost streak on TransN once it connects.
                let streak = crate::effect::part_cue(
                    sim,
                    if sim.move_started_grounded { 1170 } else { 1171 },
                    crate::effect::part::TRANS_N,
                );
                sim.action.effects.push(streak);
                sim.velocity *= self.specials.raptor_hit_mult.max(0.05);
                if !sim.move_started_grounded {
                    sim.velocity.y = 0.;
                }
            }
            // The leap's grab connected: the catch.
            (Some(Move::SpecialHi), HI_RISE) => {
                sim.stage = HI_CATCH;
                sim.stage_frame = 0;
                sim.velocity = Vec3::ZERO;
            }
            _ => {}
        }
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        let now = sim.grounded;
        match (m, sim.stage) {
            (Move::SpecialN, _) => {
                if now { "SpecialN" } else { "SpecialAirN" }
            }
            (Move::SpecialS, RB_START) => {
                if sim.move_started_grounded { "SpecialSStart" } else { "SpecialAirSStart" }
            }
            (Move::SpecialS, _) => {
                if sim.move_started_grounded { "SpecialS" } else { "SpecialAirS" }
            }
            (Move::SpecialHi, HI_RISE) => {
                if sim.move_started_grounded { "SpecialHi" } else { "SpecialAirHi" }
            }
            (Move::SpecialHi, HI_CATCH) => "SpecialHiCatch",
            (Move::SpecialHi, _) => "SpecialHiThrow",
            (Move::SpecialLw, LW_KICK) => {
                if sim.move_started_grounded { "SpecialLw" } else { "SpecialAirLw" }
            }
            (Move::SpecialLw, _) => {
                if now { "SpecialLwEnd" } else { "SpecialAirLwEnd" }
            }
            _ => "",
        }
    }
    fn facing_locked_now(&self, sim: &Sim, m: Move) -> bool {
        match m {
            // The wind-up follows the camera, so turning around during it reverses the
            // punch; the strike commits to its heading.
            Move::SpecialN => sim.motion.frame >= PUNCH_LOCK_FRAME,
            Move::SpecialS => sim.stage >= RB_HIT || sim.motion.frame >= RAPTOR_LOCK_FRAME,
            _ => self.facing_locked(m),
        }
    }
    fn projectile_move(&self, _m: Move) -> bool {
        false
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(20), // Falcon Punch wind-up
            Move::SpecialS => Some(12),
            Move::SpecialHi => Some(10),
            Move::SpecialLw => Some(4),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(30),
            Move::SpecialS => Some(30),
            Move::SpecialHi => Some(30),
            Move::SpecialLw => Some(30),
            _ => None,
        }
    }
}
