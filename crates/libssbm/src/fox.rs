//! Fox-specific special-move behavior: Blaster, Fox Illusion, Fire Fox and
//! Reflector. Prototype physics/timing; disc tuning reads PlFx.dat ext_attr
//! (ftFx_DatAttrs) pending the article-table exercise. References:
//! ftFox/ftfoxspecial{n,s,hi,lw}.c.
use crate::{
    action::GfxCue,
    behavior::Behavior,
    sim::{Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
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
        }
    }
}

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
}
impl Behavior for Fox {
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
            sim.pad_gravity = None;
            sim.grounded = false;
            // Fire Fox launches toward the camera aim (RoR2 adaptation of the
            // source's 360-degree stick angle), with the distance scaled up for
            // open-world traversal. ftfoxspecialhi.c::ftFx_SpecialAirHi_Enter.
            let aim = if sim.move_aim.length_squared() > 0.001 {
                sim.move_aim.normalize()
            } else {
                Vec3::Y
            };
            let speed = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity) * 1.8
            } else {
                self.specials.fire_fox_velocity * sim.options.jump_scale
            };
            sim.velocity = aim * speed;
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
            // Fire Fox launch flame (ftfoxspecialhi.c::ftFx_SpecialHi_CreateLaunchGFX).
            sim.action.effects.push(crate::action::GfxCue {
                id: 1164,
                bone: 0,
                common: false,
                cycle: 0,
                destroy_on_change: false,
                offset: Vec3::ZERO,
                param: 0.,
                clip: "SpecialHi",
                frame: 0.,
                origin: sim.position,
                facing: sim.facing,
            });
        } else if m == Move::SpecialS {
            // Fox Illusion: a horizontal dash in the facing direction with the
            // blue streak. ftfoxspecials.c::ftFx_SpecialS_Enter.
            sim.velocity.x = sim.facing.x * self.specials.illusion_velocity * sim.options.speed_scale;
            sim.velocity.z = sim.facing.z * self.specials.illusion_velocity * sim.options.speed_scale;
            sim.velocity.y = 0.;
            sim.action.effects.push(crate::action::GfxCue {
                id: 1165,
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
            sim.velocity = Vec3::ZERO;
        } else if m == Move::SpecialN {
            // Blaster fire sound (ftfoxspecialn.c::foxSFX[0], facing-forward id).
            sim.action.sounds.push(crate::action::SoundCue {
                sound: 110103,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
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
        m == Move::SpecialN
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
