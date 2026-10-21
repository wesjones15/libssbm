//! Yoshi-specific special-move behavior: Egg Lay, Egg Roll, Egg Throw and
//! Yoshi Bomb. Prototype physics/timing; disc tuning reads PlYs.dat ext_attr
//! (ftYoshiAttributes / ftYs_DatAttrs). References:
//! ftYoshi/ftyoshispecial{n,s,hi,lw}.c.
use crate::{
    behavior::Behavior,
    sim::{Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
};
use glam::Vec3;

/// Yoshi's special tuning, separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Egg Roll initial horizontal speed (ftYoshiAttributes.x54 ground / x58 air).
    pub egg_roll_speed: f32,
    /// Egg Throw's base throw speed (ftYs_DatAttrs.xFC).
    pub egg_throw_speed: f32,
    /// Yoshi Bomb's downward terminal speed (ftYoshiAttributes.x114).
    pub bomb_fall_speed: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            egg_roll_speed: 8.,
            egg_throw_speed: 8.,
            bomb_fall_speed: 6.,
        }
    }
}

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
}
impl Behavior for Yoshi {
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
            // Egg Roll: a horizontal roll along the facing direction.
            // ftyoshispecials.c::ftYs_SpecialS_Enter sets x1C to x54 (ground) or
            // x58 (air), then drives x-axis velocity from it.
            let dir = sim.facing.with_y(0.).normalize_or_zero();
            sim.velocity.x = dir.x * self.specials.egg_roll_speed * sim.options.speed_scale;
            sim.velocity.z = dir.z * self.specials.egg_roll_speed * sim.options.speed_scale;
            sim.velocity.y = 0.;
        } else if m == Move::SpecialLw {
            // Yoshi Bomb: a fast ground-pound. ftyoshispeciallw.c drives
            // self_vel.y to ftYoshiAttributes.x114 once airborne.
            sim.pad_gravity = None;
            if !sim.grounded {
                sim.velocity.y = -self.specials.bomb_fall_speed.abs();
            }
        }
        // Egg Lay (SpecialN) is a short tongue grab; the disc hitbox and its
        // reach carry the move, so no body physics are authored here.
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS | Move::SpecialHi)
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
    fn projectile_move(&self, m: Move) -> bool {
        m == Move::SpecialHi
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
