//! Mario's fighter behavior for the contextual-A lab.
//!
//! Melee's shared A-attack selector (ftCo_Attack1.c / ftCo_AttackS4.c and the
//! aerials) is fighter-neutral; it picks the move, plays the disc clip and runs
//! that clip's hitbox/effect script. This module is only the A lab: the Sunshine
//! host never feeds it a B press, so Sunshine's B dive stays libsms's. A host
//! that uses `FighterKind::Mario` directly and presses B gets the shared
//! fallback to contextual A, not a Sunshine dive; Mario's four specials are not
//! implemented here yet.
use crate::{
    action::SoundCue,
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
    fn projectile_move(&self, _m: Move) -> bool {
        false
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
