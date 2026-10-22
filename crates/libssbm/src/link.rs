//! Link-specific special-move behavior: Hero's Bow, Boomerang, Spin Attack and
//! Bomb. The bow and the boomerang are staged specials driving the disc's own
//! clips; tuning reads PlLk.dat ext_attr (ftLk_DatAttrs). References:
//! ftLink/ftlinkspecial{n,s,hi,lw}.c, it/kinds/itlink{arrow,boomerang,bomb}.c.
//!
//! RoR2 adaptations, marked where they occur: the camera aim stands in for the
//! 2D control stick (the arrow's and the boomerang's direction), and the flight
//! of the arrow, boomerang and bomb is the host's projectile job.
use crate::{
    action::GfxCue,
    behavior::Behavior,
    sim::{EventKind, Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
};
use glam::Vec3;

/// `Sim::host_flags`: the host reports that Link's boomerang is out. Link cannot
/// throw another until it comes back (the source plays `SpecialS1Empty` instead).
pub const HOST_BOOMERANG_OUT: u32 = 1;
/// `Sim::host_flags`: the host reports a bomb in Link's hand.
pub const HOST_BOMB_HELD: u32 = 2;

/// Link's special tuning, separate from the shared `Params`. Projectile speed
/// lives in the items' own special attributes at spawn.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Hero's Bow max charge frames (ftLk_DatAttrs.x0).
    pub bow_charge_steps: u32,
    /// Spin Attack air rise velocity (ftLk_DatAttrs.x40).
    pub spin_rise_velocity: f32,
    /// Boomerang throw: stick deadzone (x14), steepest angle (x18 * x1C) and the
    /// two throw speeds (x20, x24).
    pub boomerang_stick_threshold: f32,
    pub boomerang_max_angle: f32,
    pub boomerang_speed_a: f32,
    pub boomerang_speed_b: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            bow_charge_steps: 60,
            spin_rise_velocity: 3.2,
            boomerang_stick_threshold: 0.3,
            boomerang_max_angle: 0.6,
            boomerang_speed_a: 2.,
            boomerang_speed_b: 2.,
        }
    }
}

// Hero's Bow: SpecialNStart (the draw, the generic SpecialN), SpecialNLoop (held
// at full charge), SpecialNEnd (the release).
const N_START: u32 = 0;
const N_LOOP: u32 = 1;
const N_END: u32 = 2;
/// The boomerang leaves Link's hand on this frame of `SpecialS1` (the clip clears
/// its throw pose and plays the whoosh there).
const BOOMERANG_RELEASE_FRAME: f32 = 23.;
/// FtPart_LThumbNb, flagged so the host resolves it through the common bone table.
const ATTACH_LEFT_THUMB: usize = 0x1_0000 + 31;

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

    fn steps(&self) -> f32 {
        self.specials.bow_charge_steps.max(1) as f32
    }

    /// ftLk_SpecialNStart_IASA / ftLk_SpecialNLoop_IASA: once the draw is ready
    /// (cmd var 2, frame 18 of the draw) the charge climbs one step a tick up to
    /// x0; the Loop clip is the full charge; letting go of B releases the arrow.
    fn tick_bow(&self, sim: &mut Sim, input: Input) {
        if sim.grounded {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
        let held = input.buttons & NEUTRAL_B != 0;
        let release = match sim.stage {
            N_START => {
                sim.stage_frame += 1;
                let ready = sim.action.variables[2] == 1;
                if ready {
                    sim.stage_value = (sim.stage_value + 1.).min(self.steps());
                }
                ready && !held
            }
            N_LOOP => {
                sim.stage_value = self.steps();
                !held
            }
            _ => false,
        };
        if release {
            sim.stage = N_END;
            sim.stage_frame = 0;
            // The End clip's frame-0 script (cmd var 1) fires the arrow.
            sim.action.projectile_fired = false;
        }
    }

    /// ftlinkspecials.c::onAccessory4: the boomerang leaves the hand during the
    /// throw clip; with one already out the empty-handed clip plays instead.
    fn tick_boomerang(&self, sim: &mut Sim, host: &mut dyn Host) {
        if sim.stage_value < 1. && !sim.action.projectile_fired && sim.motion.frame >= BOOMERANG_RELEASE_FRAME {
            sim.action.projectile_fired = true;
            sim.host_flags |= HOST_BOOMERANG_OUT;
            sim.emit(host, EventKind::Hit, Move::SpecialS as u32, 1.);
        }
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
            let slash = crate::effect::part_cue_bound(sim, id, crate::effect::part::TRANS_N); // x2219_b0 set (ftlinkspecialhi.c:42)
            sim.action.effects.push(slash);
        } else if m == Move::SpecialS {
            // Boomerang: with one already out the empty-handed clip plays
            // (ftLk_SpecialS_Enter reads fp->u.lk.used_boomerang).
            sim.velocity = Vec3::ZERO;
            sim.stage_value = if sim.host_flags & HOST_BOOMERANG_OUT != 0 { 1. } else { 0. };
        }
        // SpecialN (Hero's Bow) is staged; SpecialLw (Bomb) pulls the bomb out and
        // the host carries it. Neither changes the body's velocity on entry.
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
        self.specials.bow_charge_steps
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
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32 {
        if m == Move::SpecialN {
            // The charge ratio drives the arrow's speed and damage
            // (itLinkArrow_802A850C: charge * (max - min) / x0 + min).
            (sim.stage_value / self.steps()).clamp(0., 1.)
        } else {
            sim.move_strength
        }
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

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, _pressed: u32) {
        if sim.phase == Phase::Free {
            return;
        }
        match sim.current_move {
            Some(Move::SpecialN) => self.tick_bow(sim, input),
            Some(Move::SpecialS) => self.tick_boomerang(sim, host),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) -> bool {
        match (m, sim.stage) {
            (Move::SpecialN, N_START) => {
                // Reaching the Loop clip is the full charge (doLoopIASA).
                sim.stage = N_LOOP;
                sim.stage_frame = 0;
                true
            }
            (Move::SpecialN, N_LOOP) => true,
            _ => false,
        }
    }
    fn projectile_variable(&self) -> Option<u32> {
        // The End clip raises cmd var 1 on the frame the arrow leaves the bow.
        Some(1)
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        let now = sim.grounded;
        match m {
            Move::SpecialN => match sim.stage {
                N_START => {
                    if now { "SpecialN" } else { "SpecialAirN" }
                }
                N_LOOP => {
                    if now { "SpecialNLoop" } else { "SpecialAirNLoop" }
                }
                _ => {
                    if now { "SpecialNEnd" } else { "SpecialAirNEnd" }
                }
            },
            Move::SpecialS => match (sim.stage_value >= 1., now) {
                (false, true) => "SpecialS",
                (false, false) => "SpecialAirS",
                (true, true) => "SpecialS1Empty",
                (true, false) => "SpecialAirS1Empty",
            },
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
    fn hit_attachment(&self, _sim: &Sim, m: Move) -> (Option<usize>, Vec3) {
        // The arrow leaves the nocked bow and the boomerang and bomb leave the left
        // hand (FtPart_LThumbNb, resolved through the common bone table).
        if matches!(m, Move::SpecialN | Move::SpecialS | Move::SpecialLw) {
            (Some(ATTACH_LEFT_THUMB), Vec3::ZERO)
        } else {
            (None, Vec3::ZERO)
        }
    }
    fn projectile_move(&self, m: Move) -> bool {
        // The boomerang is emitted by `tick_boomerang`, not by a clip command.
        matches!(m, Move::SpecialN | Move::SpecialLw)
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
