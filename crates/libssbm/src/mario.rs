//! Mario's fighter behavior for the contextual-A lab.
//!
//! Melee's shared A-attack selector (ftCo_Attack1.c / ftCo_AttackS4.c and the
//! aerials) is fighter-neutral; it picks the move, plays the disc clip and runs
//! that clip's hitbox/effect script. Mario's four specials are not implemented
//! here yet, so every special press falls back to the same contextual A rather
//! than pretending to be a Fireball/Cape/Super Jump Punch/Tornado. The reason is
//! the animation port this exists for: Sunshine Mario's FLUDDless contextual A
//! only needs Mario's A clips, and adding a half-guessed special would be worse
//! than leaving it out. Clip names and frame counts come from `PlMrAJ.dat`.
use crate::{
    action::{GfxCue, SoundCue},
    behavior::Behavior,
    sim::{Host, Input, Move, Phase, Sim},
};
use glam::Vec3;

/// Mario's A-attack behavior. Construct with [`Mario::new`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Mario;

impl Mario {
    pub fn new() -> Self {
        Self
    }
}

impl Behavior for Mario {
    fn slot(&self, _m: Move) -> u32 {
        // Every implemented move is a common A attack (host skill slot 1).
        1
    }
    fn mobile_moves(&self) -> u64 {
        0
    }
    fn upper_body_root(&self) -> Option<usize> {
        // Torso root above HipN, the same seam the other humanoid fighters use.
        Some(5)
    }
    fn is_up_special(&self, _m: Move) -> bool {
        false
    }

    fn select_special(&self, _sim: &Sim, _input: Input, _movement: Vec3) -> Option<Move> {
        // No specials yet: fall through to the contextual A attack.
        None
    }
    fn enter_special(&self, _sim: &mut Sim, _host: &mut dyn Host, _m: Move, _input: Input) {}
    fn aimable(&self, _m: Move) -> bool {
        false
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
    fn tick_charge_cancel(&self, _sim: &mut Sim) -> bool {
        false
    }

    fn clip(&self, _sim: &Sim, _m: Move) -> &'static str {
        "Wait"
    }
    fn landing(&self, _sim: &Sim, _m: Move) -> &'static str {
        "Landing"
    }
    fn facing_locked(&self, _m: Move) -> bool {
        false
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
    fn phase_stop(&self, _phase: Phase) -> Option<SoundCue> {
        None
    }
    fn startup(&self, _m: Move) -> Option<u32> {
        None
    }
    fn recovery(&self, _m: Move) -> Option<u32> {
        None
    }
}
