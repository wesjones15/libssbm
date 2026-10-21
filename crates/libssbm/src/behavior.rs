//! Per-fighter special-move behavior. The shared sim owns locomotion, jump,
//! shield, reactions, the A-attack/smash selection and the disc command executor;
//! a `Behavior` supplies a fighter's specials: which moves they are, their
//! clips, physics, phase logic and presentation. Adding a fighter means adding a
//! `Behavior` implementation, not editing the sim.
use crate::{
    action::{GfxCue, SoundCue},
    sim::{Host, Input, Move, Phase, Sim},
};
use glam::Vec3;

/// A fluttering mid-air jump; see [`Behavior::flutter`].
#[derive(Clone, Copy, Debug)]
pub struct Flutter {
    pub frames: u32,
    pub gravity: f32,
    pub boost: f32,
}

pub trait Behavior: Send {
    // --- Move classification ------------------------------------------------
    /// Skill slot (0..4) a move maps to. Common A-attacks use slot 1.
    fn slot(&self, m: Move) -> u32;
    /// Moves that may steer while grounded with the upper/lower split.
    fn mobile_moves(&self) -> u64;
    /// Root joint of the upper-body subtree for the mobile split. `None` means
    /// the fighter has no split and mobile moves stay whole-body. This is the
    /// per-character "top half of B, bottom half of walk" seam.
    fn upper_body_root(&self) -> Option<usize>;
    /// The recovery special (up-B). The shared sim grants it i-frames from launch
    /// until landing, and clears them if another attack starts midair.
    fn is_up_special(&self, m: Move) -> bool;

    // --- Special selection and lifecycle ------------------------------------
    /// Pick the special move for a special-button press, or `None` to fall back
    /// to the contextual A attack. Reads the sim for flick age, aim and movement.
    fn select_special(&self, sim: &Sim, input: Input, movement: Vec3) -> Option<Move>;
    /// Per-special entry work after the common `start` bookkeeping (velocity,
    /// effects, jump usage). Grounded/airborne is already recorded.
    fn enter_special(&self, sim: &mut Sim, host: &mut dyn Host, m: Move, input: Input);
    /// Whether a move keeps aiming at the live camera ray during its action.
    fn aimable(&self, m: Move) -> bool;
    /// The charge-shot move, if this fighter has one (Samus: ChargeShot).
    fn is_charge_move(&self, m: Move) -> bool;
    /// Clip for the hold-to-charge phase (Samus: SpecialNHold).
    fn charge_clip(&self) -> &'static str {
        "SpecialNHold"
    }
    /// Source charge-voice band for the held charge (0..4).
    fn charge_voice_band(&self, sim: &Sim) -> u32;
    /// Charge levels and visual scale range for a charge-capable fighter.
    fn charge_steps(&self) -> u32;
    fn charge_visual(&self) -> (f32, f32);
    /// Optional pre-phase handling (e.g. sprint stores a held charge). True when
    /// the whole tick was consumed.
    fn store_special(&self, sim: &mut Sim, host: &mut dyn Host, pressed: u32) -> bool;
    /// After startup, decide whether to enter a hold phase; true when handled
    /// (the caller must not fire).
    fn resolve_startup(&self, sim: &mut Sim, input: Input) -> bool;
    /// Strength for firing a move (charge ratio for a charge shot, else move
    /// strength). May consume stored charge.
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32;

    // --- Special phases ------------------------------------------------------
    /// Handle `Phase::ChargeShot` (a hold-to-charge special).
    fn tick_charge(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32);
    /// Handle `Phase::ChargeCancel`; true when the phase finished.
    fn tick_charge_cancel(&self, sim: &mut Sim) -> bool;

    // --- Presentation --------------------------------------------------------
    fn clip(&self, sim: &Sim, m: Move) -> &'static str;
    fn landing(&self, sim: &Sim, m: Move) -> &'static str;
    fn facing_locked(&self, m: Move) -> bool;
    fn spins_whole_body(&self, m: Move) -> bool;
    fn side_special_layered(&self, m: Move) -> bool;
    fn side_special_upper_aim(&self, sim: &Sim) -> Option<(f32, f32)>;
    fn hit_attachment(&self, sim: &Sim, m: Move) -> (Option<usize>, Vec3);
    fn projectile_move(&self, m: Move) -> bool;
    fn projectile_cue(&self, m: Move) -> Option<(u32, usize, bool)>;
    fn phase_effect(&self, sim: &Sim, phase: Phase) -> Option<GfxCue>;
    fn phase_stop(&self, phase: Phase) -> Option<SoundCue>;

    // --- Multi-stage specials (all optional; defaults leave a move single-clip)
    /// Runs every sim tick before the phase logic. A behavior with staged
    /// specials drives velocity, gravity ownership and stage changes here.
    fn tick_special(&self, _sim: &mut Sim, _host: &mut dyn Host, _input: Input, _pressed: u32) {}
    /// A special's disc clip finished. Return true after moving to the next
    /// stage (the behavior's `clip` now names the next clip); false ends the move.
    fn next_stage(&self, _sim: &mut Sim, _host: &mut dyn Host, _m: Move, _input: Input) -> bool {
        false
    }
    /// The disc cmd-var index whose rising edge (value 1) fires the move's
    /// projectile, for fighters whose article spawns from a variable rather than
    /// the throw flag (Pikachu's Thunder Jolt uses variable 0).
    fn projectile_variable(&self) -> Option<u32> {
        None
    }
    /// A mid-move stage that stops following the camera aim (a launched bash).
    fn aim_frozen(&self, _sim: &Sim) -> bool {
        false
    }
    /// A stage whose clip script must not apply hitboxes, effects or sounds.
    fn script_muted(&self, _sim: &Sim) -> bool {
        false
    }
    /// Facing lock that may depend on the move's current stage (Skull Bash
    /// follows the camera while charging, then commits to its heading).
    fn facing_locked_now(&self, _sim: &Sim, m: Move) -> bool {
        self.facing_locked(m)
    }
    /// The host reports that the current move's hitbox connected.
    fn on_connect(&self, _sim: &mut Sim) {}
    /// A neutral special whose single disc clip has finished: `true` replays the
    /// clip in the Recovery phase (Samus's charge shot, whose second pass is the
    /// release); `false` ends the move there. The default is `false` because the
    /// common disc neutral special is a single pass of the clip: replaying it
    /// would play the whole animation (and its hitboxes) twice, the "double anim"
    /// every new fighter inherited. Only Samus's charge shot opts back in.
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }
    /// A fighter whose mid-air jump flutters (Yoshi, ftCo_JumpAerial.c::
    /// ftYs_JumpAerial_Enter): for `frames` ticks after each aerial jump gravity is
    /// scaled by `gravity`, the takeoff speed by `boost`, and hits do not move him
    /// (the jump's armor). `None` is an ordinary double jump.
    fn flutter(&self) -> Option<Flutter> {
        None
    }

    // --- Prototype timing (not purported frame-perfect source data) ----------
    /// Special-only override for the shared prototype startup; `None` uses the
    /// common move schedule.
    fn startup(&self, m: Move) -> Option<u32>;
    fn recovery(&self, m: Move) -> Option<u32>;
}
