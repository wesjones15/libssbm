//! Link-specific special-move behavior: Hero's Bow, Boomerang, Spin Attack and
//! Bomb. Prototype physics/timing; disc tuning reads PlLk.dat ext_attr
//! (ftLk_DatAttrs). References: ftLink/ftlinkspecial{n,s,hi,lw}.c.
use crate::{
    action::GfxCue,
    behavior::Behavior,
    sim::{Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, SPECIAL, UP_B},
};
use glam::Vec3;

/// Link's special tuning, separate from the shared `Params`. Projectile speed
/// lives in the items' own special attributes at spawn.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Hero's Bow max charge frames (ftLk_DatAttrs.x0).
    pub bow_charge_steps: u32,
    /// Spin Attack air rise velocity (ftLk_DatAttrs.x40).
    pub spin_rise_velocity: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            bow_charge_steps: 60,
            spin_rise_velocity: 3.2,
        }
    }
}

pub struct Link {
    specials: Specials,
}
impl Link {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Link behavior for headless tests.
    pub fn fixture() -> Self {
        Self::new(Specials::fixture())
    }
}
impl Behavior for Link {
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
            // Spin Attack: a rising multi-hit recovery (ftlinkspecialhi.c::
            // ftLk_SpecialAirHi_Enter sets self_vel.y from x40).
            sim.pad_gravity = None;
            sim.grounded = false;
            sim.velocity.y = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity)
            } else {
                self.specials.spin_rise_velocity * sim.options.jump_scale
            };
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
            // Spin Attack slash (efsync.c 0x4BB ground / 0x4BC air).
            let id = if sim.move_started_grounded { 1211 } else { 1212 };
            sim.action.effects.push(GfxCue {
                id,
                bone: 0,
                common: false,
                cycle: 0,
                destroy_on_change: false,
                offset: Vec3::ZERO,
                param: 0.,
                clip: if sim.move_started_grounded { "SpecialHi" } else { "SpecialAirHi" },
                frame: 0.,
                origin: sim.position,
                facing: sim.facing,
            });
        } else if m == Move::SpecialS {
            // Boomerang: throw it forward; the item flight is the projectile
            // host's job (ftlinkspecials.c::onAccessory4).
            sim.velocity = Vec3::ZERO;
        }
        // SpecialN (Hero's Bow) charges into Phase::ChargeShot; SpecialLw (Bomb)
        // pulls the bomb and lobs it through the projectile host. Neither
        // changes the body's velocity on entry.
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS)
    }
    fn is_charge_move(&self, m: Move) -> bool {
        m == Move::SpecialN
    }
    fn charge_clip(&self) -> &'static str {
        "SpecialNLoop"
    }
    fn charge_voice_band(&self, _sim: &Sim) -> u32 {
        0
    }
    fn charge_steps(&self) -> u32 {
        self.specials.bow_charge_steps
    }
    fn charge_visual(&self) -> (f32, f32) {
        (1., 1.)
    }
    fn store_special(&self, _sim: &mut Sim, _host: &mut dyn Host, _pressed: u32) -> bool {
        false
    }
    fn resolve_startup(&self, sim: &mut Sim, input: Input) -> bool {
        // Draw and hold the bow while neutral-B stays pressed.
        if sim.current_move == Some(Move::SpecialN)
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
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32 {
        if m == Move::SpecialN {
            // Charge ratio drives the arrow's speed and damage (itLinkArrow_802A850C).
            let ratio = sim.shot_charge as f32 / self.charge_steps().max(1) as f32;
            sim.shot_charge = 0;
            0.4 + 0.6 * ratio
        } else {
            sim.move_strength
        }
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
    fn spins_whole_body(&self, m: Move) -> bool {
        m == Move::SpecialHi
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
        matches!(m, Move::SpecialN | Move::SpecialS | Move::SpecialLw)
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
            Move::SpecialN => Some(6),
            Move::SpecialS => Some(8),
            Move::SpecialHi => Some(6),
            Move::SpecialLw => Some(8),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(20),
            Move::SpecialS => Some(20),
            Move::SpecialHi => Some(30),
            Move::SpecialLw => Some(20),
            _ => None,
        }
    }
}
