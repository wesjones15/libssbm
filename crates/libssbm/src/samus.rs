//! Samus-specific special-move behavior: Charge Shot, missiles, Screw Attack,
//! Bomb, plus the grapple-based throws the shared executor already understands.
//! References: ftSamus/ftsamusspecial{n,s,hi,lw1}.c and its article attributes.
use crate::{
    action::{GfxCue, SoundCue},
    behavior::Behavior,
    sim::{side_special_upper_angles, Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
    EventKind,
};
use glam::Vec3;

/// Samus's `ext_attr` tuning (ftSs_DatAttrs and charge-shot article attributes),
/// separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    pub charge_steps: u32,
    pub charge_visual_min: f32,
    pub charge_visual_max: f32,
    pub charge_interval: u32,
    pub missile_smash_window: u32,
    pub missile_spawn_offset: f32,
    pub bomb_spawn_offset: [f32; 3],
    pub screw_velocity: f32,
    pub bomb_velocity: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            charge_steps: 10,
            charge_visual_min: 0.5,
            charge_visual_max: 2.,
            charge_interval: 3,
            missile_smash_window: 3,
            missile_spawn_offset: 0.,
            bomb_spawn_offset: [0.; 3],
            screw_velocity: 3.,
            bomb_velocity: 0.5,
        }
    }
}

pub struct Samus {
    specials: Specials,
    upper_body_root: Option<usize>,
}
impl Samus {
    pub fn new(specials: Specials, upper_body_root: Option<usize>) -> Self {
        Self { specials, upper_body_root }
    }
    /// Synthetic Samus behavior for headless tests.
    pub fn fixture() -> Self {
        Self::new(Specials::fixture(), None)
    }
}
impl Behavior for Samus {
    fn slot(&self, m: Move) -> u32 {
        match m {
            Move::ChargeShot => 0,
            Move::Missile | Move::SuperMissile => 2,
            Move::ScrewAttack => 3,
            Move::Bomb => 4,
            _ => 1,
        }
    }
    fn mobile_moves(&self) -> u64 {
        (1u64 << Move::ChargeShot as u32)
            | (1u64 << Move::Missile as u32)
            | (1u64 << Move::SuperMissile as u32)
            | (1u64 << Move::ScrewAttack as u32)
            | (1u64 << Move::Bomb as u32)
    }
    fn upper_body_root(&self) -> Option<usize> {
        self.upper_body_root
    }
    fn is_up_special(&self, m: Move) -> bool {
        m == Move::ScrewAttack
    }

    fn select_special(&self, sim: &Sim, input: Input, movement: Vec3) -> Option<Move> {
        let smash = sim.flick_age < self.specials.missile_smash_window;
        Some(if input.buttons & NEUTRAL_B != 0 {
            Move::ChargeShot
        } else if input.buttons & UP_B != 0 {
            Move::ScrewAttack
        } else if input.buttons & DOWN_B != 0 {
            Move::Bomb
        } else if input.buttons & SIDE_B != 0 {
            if smash { Move::SuperMissile } else { Move::Missile }
        } else if input.attack_vertical() > 0 {
            Move::ScrewAttack
        } else if input.attack_vertical() < 0 {
            Move::Bomb
        } else if movement.length() > 0.3 {
            if smash { Move::SuperMissile } else { Move::Missile }
        } else {
            Move::ChargeShot
        })
    }
    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) {
        if m == Move::ScrewAttack {
            sim.action.effects.push(GfxCue {
                id: 1154,
                bone: 3,
                common: false,
                cycle: 0,
                destroy_on_change: true,
                offset: Vec3::ZERO,
                param: 0.,
                clip: if sim.move_started_grounded {
                    "SpecialHi"
                } else {
                    "SpecialAirHi"
                },
                frame: 0.,
                origin: sim.position,
                facing: sim.facing,
            });
            sim.pad_gravity = None;
            sim.grounded = false;
            // The up-B (Screw Attack) rises to the host's standard jump apex, like
            // every other jump, instead of Melee's smaller recovery rise. Without a
            // host height (tests/viewer) the source screw velocity remains.
            sim.velocity.y = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity)
            } else {
                self.specials.screw_velocity * sim.options.jump_scale
            };
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
        } else if m == Move::Bomb && !sim.grounded {
            sim.pad_gravity = None;
            sim.velocity.y = self.specials.bomb_velocity;
        }
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::ChargeShot | Move::Missile | Move::SuperMissile)
    }
    fn is_charge_move(&self, m: Move) -> bool {
        m == Move::ChargeShot
    }
    fn charge_voice_band(&self, sim: &Sim) -> u32 {
        (sim.shot_charge * 5 / self.specials.charge_steps.max(1)).min(4)
    }
    fn charge_steps(&self) -> u32 {
        self.specials.charge_steps
    }
    fn charge_visual(&self) -> (f32, f32) {
        (self.specials.charge_visual_min, self.specials.charge_visual_max)
    }
    fn store_special(&self, sim: &mut Sim, host: &mut dyn Host, pressed: u32) -> bool {
        if sim.current_move == Some(Move::ChargeShot)
            && matches!(sim.phase, Phase::Startup | Phase::ChargeShot)
            && pressed & crate::sim::SPRINT != 0
        {
            sim.emit(
                host,
                EventKind::ChargeStored,
                Move::ChargeShot as u32,
                sim.shot_charge as f32,
            );
            sim.action.reset();
            sim.current_move = None;
            sim.set_phase(Phase::Free);
            true
        } else {
            false
        }
    }
    fn resolve_startup(&self, sim: &mut Sim, input: Input) -> bool {
        if sim.current_move == Some(Move::ChargeShot)
            && sim.grounded
            && sim.shot_charge < self.specials.charge_steps
            && !(sim.options.hold_charge && input.buttons & crate::sim::SPECIAL == 0)
        {
            sim.charge_clock = 0;
            sim.set_phase(Phase::ChargeShot);
            true
        } else {
            false
        }
    }
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32 {
        if m == Move::ChargeShot {
            let ratio = sim.shot_charge as f32 / self.specials.charge_steps as f32;
            sim.shot_charge = 0;
            ratio
        } else {
            sim.move_strength
        }
    }

    fn tick_charge(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        sim.charge_clock += 1;
        if sim.charge_clock >= self.specials.charge_interval {
            sim.charge_clock = 0;
            sim.shot_charge = (sim.shot_charge + 1).min(self.specials.charge_steps);
        }
        if pressed & (crate::sim::CANCEL | crate::sim::GRAB | crate::sim::JUMP) != 0
            || sim.shot_charge >= self.specials.charge_steps
        {
            // SpecialNHold_Anim starts colour animation 53 at maximum, not the
            // firing voice. The stored-charge presenter owns it.
            sim.emit(
                host,
                EventKind::ChargeStored,
                Move::ChargeShot as u32,
                sim.shot_charge as f32,
            );
            sim.set_phase(if sim.action.definitions.is_empty() {
                Phase::Free
            } else {
                Phase::ChargeCancel
            });
            sim.current_move = None;
            if pressed & crate::sim::JUMP != 0 && sim.grounded {
                sim.short_hop = false;
                sim.set_phase(Phase::JumpSquat);
            }
        } else if (sim.options.hold_charge && input.buttons & crate::sim::SPECIAL == 0)
            || (!sim.options.hold_charge && pressed & crate::sim::SPECIAL != 0)
        {
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
            Move::ChargeShot => {
                if sim.phase == Phase::Startup {
                    if sim.move_started_grounded {
                        "SpecialNStart"
                    } else {
                        "SpecialAirNStart"
                    }
                } else if sim.grounded {
                    "SpecialN"
                } else {
                    "SpecialAirN"
                }
            }
            Move::Missile => {
                if sim.grounded { "SpecialS" } else { "SpecialAirS" }
            }
            Move::SuperMissile => {
                if sim.grounded { "Special" } else { "SpecialAir" }
            }
            Move::ScrewAttack => {
                if sim.move_started_grounded {
                    "SpecialHi"
                } else {
                    "SpecialAirHi"
                }
            }
            Move::Bomb => {
                if sim.grounded { "SpecialLw" } else { "SpecialAirLw" }
            }
            _ => "",
        }
    }
    fn landing(&self, _sim: &Sim, m: Move) -> &'static str {
        if m == Move::ScrewAttack { "LandingFallSpecial" } else { "Landing" }
    }
    fn facing_locked(&self, m: Move) -> bool {
        matches!(m, Move::ScrewAttack | Move::Bomb)
    }
    fn spins_whole_body(&self, m: Move) -> bool {
        matches!(m, Move::Bomb | Move::ScrewAttack)
    }
    fn side_special_layered(&self, m: Move) -> bool {
        matches!(m, Move::Missile | Move::SuperMissile)
    }
    fn side_special_upper_aim(&self, sim: &Sim) -> Option<(f32, f32)> {
        matches!(sim.current_move, Some(Move::Missile | Move::SuperMissile))
            .then(|| side_special_upper_angles(sim.move_aim, sim.facing))
    }
    fn hit_attachment(&self, sim: &Sim, m: Move) -> (Option<usize>, Vec3) {
        match m {
            Move::ChargeShot => (Some(51), Vec3::ZERO),
            Move::Missile | Move::SuperMissile => {
                (Some(56), sim.facing * self.specials.missile_spawn_offset)
            }
            Move::Bomb => (
                Some(0),
                sim.facing * self.specials.bomb_spawn_offset[0]
                    + Vec3::Y * self.specials.bomb_spawn_offset[1]
                    + Vec3::new(-sim.facing.z, 0., sim.facing.x)
                        * self.specials.bomb_spawn_offset[2],
            ),
            _ => (None, Vec3::ZERO),
        }
    }
    fn projectile_move(&self, m: Move) -> bool {
        matches!(
            m,
            Move::ChargeShot | Move::Missile | Move::SuperMissile | Move::Bomb
        )
    }
    fn projectile_cue(&self, m: Move) -> Option<(u32, usize, bool)> {
        match m {
            Move::ChargeShot => Some((1158, 51, false)),
            Move::Missile | Move::SuperMissile => Some((1155, 56, false)),
            _ => None,
        }
    }
    fn phase_effect(&self, sim: &Sim, phase: Phase) -> Option<GfxCue> {
        (phase == Phase::ChargeShot).then(|| GfxCue {
            id: 1151,
            bone: 50,
            common: false,
            cycle: 0,
            destroy_on_change: true,
            offset: Vec3::new(0., 0., 4.),
            param: 0.,
            clip: "SpecialNHold",
            frame: 0.,
            origin: sim.position,
            facing: sim.facing,
        })
    }
    fn phase_stop(&self, phase: Phase) -> Option<SoundCue> {
        (phase == Phase::ChargeShot).then(|| SoundCue {
            sound: 0,
            volume: 127,
            pan: 64,
            channel: 6,
            stop: true,
        })
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::ChargeShot => Some(12),
            Move::Missile | Move::SuperMissile => Some(12),
            Move::Bomb => Some(10),
            Move::ScrewAttack => Some(3),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::ChargeShot => Some(24),
            Move::Missile | Move::SuperMissile => Some(24),
            Move::Bomb => Some(25),
            Move::ScrewAttack => Some(36),
            _ => None,
        }
    }
}
