//! Captain Falcon-specific special-move behavior: Falcon Punch, Raptor Boost,
//! Falcon Dive and Falcon Kick. Prototype physics/timing; disc tuning reads
//! PlCa.dat ext_attr (ftCaptain_DatAttrs). References:
//! ftCaptain/ftcaptainspecial{n,s,hi,lw}.c.
use crate::{
    action::GfxCue,
    behavior::Behavior,
    sim::{Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
};
use glam::Vec3;

/// Captain Falcon's special tuning, separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Falcon Punch momentum (ftCaptain_DatAttrs.xC specialn_vel_x).
    pub punch_momentum: f32,
    /// Raptor Boost dash speed (ftCaptain_DatAttrs.x14 specials_gr_vel_x).
    pub raptor_velocity: f32,
    /// Falcon Dive launch speed (ftCaptain_DatAttrs.x44 specialhi_horz_vel).
    pub dive_velocity: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            punch_momentum: 2.4,
            raptor_velocity: 10.,
            dive_velocity: 3.2,
        }
    }
}

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
}
impl Behavior for Captain {
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
        (1u64 << Move::SpecialN as u32) | (1u64 << Move::SpecialS as u32)
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
            // Raptor Boost: a grounded horizontal lunge. ftcaptainspecials.c::
            // ftCa_SpecialS_Enter zeroes velocity, then the lunge carries Falcon.
            let dir = sim.facing.with_y(0.).normalize_or_zero();
            sim.velocity = dir * self.specials.raptor_velocity * sim.options.speed_scale;
            sim.action.effects.push(GfxCue {
                id: 1169, // ftCa_SpecialS_Enter head spark (efSync_Spawn)
                bone: 0,
                common: false,
                cycle: 0,
                destroy_on_change: false,
                offset: Vec3::ZERO,
                param: 0.,
                clip: "SpecialS",
                frame: 0.,
                origin: sim.position,
                facing: sim.facing,
            });
        } else if m == Move::SpecialLw {
            // Falcon Kick: a grounded dash kick with a trailing flame.
            // ftcaptainspeciallw.c::ftCa_SpecialHi_800E3EAC (efAsync 0x490).
            let dir = sim.facing.with_y(0.).normalize_or_zero();
            sim.velocity = dir * self.specials.raptor_velocity * sim.options.speed_scale;
            sim.action.effects.push(GfxCue {
                id: 1168,
                bone: 0,
                common: false,
                cycle: 0,
                destroy_on_change: false,
                offset: Vec3::ZERO,
                param: 0.,
                clip: "SpecialLw",
                frame: 0.,
                origin: sim.position,
                facing: sim.facing,
            });
        } else if m == Move::SpecialN {
            // Falcon Punch: a single devastating punch. The source gives the
            // aerial version forward momentum (ftcaptainspecialn.c IASA); the
            // flame at the fist is authored by the disc script (efasync 0x446
            // and 0x3FF), so no manual effect cue belongs here.
            if !sim.grounded {
                sim.velocity = sim.facing * self.specials.punch_momentum * sim.options.speed_scale;
            }
        }
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS)
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
    fn fire_strength(&self, sim: &mut Sim, _m: Move) -> f32 {
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

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        match m {
            Move::SpecialN => {
                if sim.grounded { "SpecialN" } else { "SpecialAirN" }
            }
            Move::SpecialS => {
                if sim.grounded { "SpecialS" } else { "SpecialAirS" }
            }
            Move::SpecialHi => {
                if sim.move_started_grounded { "SpecialHi" } else { "SpecialAirHi" }
            }
            Move::SpecialLw => {
                if sim.grounded { "SpecialLw" } else { "SpecialAirLw" }
            }
            _ => "",
        }
    }
    fn landing(&self, _sim: &Sim, m: Move) -> &'static str {
        if m == Move::SpecialHi { "LandingFallSpecial" } else { "Landing" }
    }
    fn facing_locked(&self, m: Move) -> bool {
        matches!(m, Move::SpecialHi | Move::SpecialLw)
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
    fn projectile_move(&self, _m: Move) -> bool {
        false
    }
    fn projectile_cue(&self, _m: Move) -> Option<(u32, usize, bool)> {
        None
    }
    fn phase_effect(&self, _sim: &Sim, _phase: Phase) -> Option<GfxCue> {
        None
    }
    fn phase_stop(&self, _phase: Phase) -> Option<crate::action::SoundCue> {
        None
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
