//! Luigi-specific special-move behavior: Fireball, Green Missile, Super Jump
//! Punch and Luigi Cyclone. Prototype physics/timing; disc tuning reads
//! PlLg.dat ext_attr (ftLuigiAttributes). References: ftLuigi/ftluigispecial{n,s,hi,lw}.c.
use crate::{
    action::GfxCue,
    behavior::Behavior,
    sim::{Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, SPECIAL, UP_B},
};
use glam::Vec3;

/// Luigi's special tuning, separate from the shared `Params`. Fireball speed is
/// read from the item's own special attributes at projectile spawn.
#[derive(Clone, Debug)]
pub struct Specials {
    pub missile_velocity: f32,
    pub super_jump_velocity: f32,
    pub cyclone_momentum: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            missile_velocity: 14.,
            super_jump_velocity: 3.2,
            cyclone_momentum: 4.,
        }
    }
}

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
    /// Fire the charged Green Missile: forward along the camera aim, with the
    /// charge scaled from 250% (uncharged) to 1600% (full).
    fn launch_missile(&self, sim: &mut Sim) {
        let ratio = sim.shot_charge as f32 / self.charge_steps().max(1) as f32;
        sim.move_strength = 2.5 + 13.5 * ratio;
        let aim = sim.move_aim.with_y(0.);
        let aim = if aim.length_squared() > 0.001 {
            aim.normalize()
        } else {
            sim.facing
        };
        sim.velocity = aim * self.specials.missile_velocity * sim.options.speed_scale;
        sim.grounded = false;
        sim.pad_gravity = None;
        sim.action.effects.push(GfxCue {
            id: 1290,
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
    }
}
impl Behavior for Luigi {
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
            // Super Jump Punch rises to the host's standard jump apex.
            sim.velocity.y = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity)
            } else {
                self.specials.super_jump_velocity * sim.options.jump_scale
            };
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
        } else if m == Move::SpecialS {
            // Green Missile charge: hold in place; the launch happens on release.
            sim.velocity = Vec3::ZERO;
        } else if m == Move::SpecialLw {
            // Luigi Cyclone: briefly halt the fall and spin with its cyclone FX.
            sim.velocity = Vec3::ZERO;
            sim.pad_gravity = None;
            sim.action.effects.push(GfxCue {
                id: 1289,
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
        }
        // Fireball (SpecialN) spawns through the projectile path; no body sound.
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS)
    }
    fn is_charge_move(&self, m: Move) -> bool {
        m == Move::SpecialS
    }
    fn charge_clip(&self) -> &'static str {
        "SpecialSHold"
    }
    fn charge_voice_band(&self, _sim: &Sim) -> u32 {
        0
    }
    fn charge_steps(&self) -> u32 {
        100
    }
    fn charge_visual(&self) -> (f32, f32) {
        (1., 1.)
    }
    fn store_special(&self, _sim: &mut Sim, _host: &mut dyn Host, _pressed: u32) -> bool {
        false
    }
    fn resolve_startup(&self, sim: &mut Sim, input: Input) -> bool {
        // Enter the charge hold while RB stays pressed.
        if sim.current_move == Some(Move::SpecialS)
            && sim.shot_charge < self.charge_steps()
            && input.buttons & SPECIAL != 0
        {
            sim.charge_clock = 0;
            sim.set_phase(Phase::ChargeShot);
            true
        } else {
            false
        }
    }
    fn fire_strength(&self, sim: &mut Sim, _m: Move) -> f32 {
        sim.move_strength
    }
    fn tick_charge(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        sim.charge_clock += 1;
        if sim.charge_clock >= 1 {
            sim.charge_clock = 0;
            sim.shot_charge = (sim.shot_charge + 1).min(self.charge_steps());
        }
        let fire = pressed & (crate::sim::CANCEL | crate::sim::GRAB | crate::sim::JUMP) != 0
            || sim.shot_charge >= self.charge_steps()
            || input.buttons & SPECIAL == 0;
        if fire {
            self.launch_missile(sim);
            sim.fire(host);
        }
    }
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
