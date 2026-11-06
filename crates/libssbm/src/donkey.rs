//! Donkey Kong-specific special-move behavior: Giant Punch, Headbutt, Spinning
//! Kong and Hand Slap. Giant Punch and Hand Slap are staged, driving the disc's
//! own clips; tuning reads PlDk.dat ext_attr (ftDonkeyAttributes). References:
//! ftDonkey/ftdonkeyspecial{n,s,hi,lw}.c.
//!
//! RoR2 adaptations, marked where they occur: the Giant Punch charges while the
//! button is held (the source toggles on a second press), Spinning Kong steers
//! with the move stick, and the Hand Slap's shockwaves are host jobs.
use crate::{
    behavior::Behavior,
    sim::{EventKind, Host, Input, Move, Phase, Sim, CANCEL, DOWN_B, NEUTRAL_B},
};
use glam::Vec3;

/// Donkey Kong's special tuning, separate from the shared `Params`.
#[derive(Clone, Debug)]
pub struct Specials {
    /// Giant Punch: the arm swings it can store (SpecialN.x2C), the damage each adds
    /// to the release (x30) and the horizontal momentum of the aerial punch (x34).
    pub max_swings: u32,
    pub damage_per_swing: f32,
    pub punch_velocity: f32,
    /// Headbutt: the divisor on aerial momentum (ftDonkeyAttributes.SpecialS.x3C).
    pub headbutt_divisor: f32,
    /// Spinning Kong aerial rise (ftDonkeyAttributes.SpecialHi.x4C) and
    /// horizontal speed (x54 grounded, x58 aerial).
    pub spin_rise_velocity: f32,
    pub spin_horizontal: f32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            max_swings: 8,
            damage_per_swing: 3.,
            punch_velocity: 3.,
            headbutt_divisor: 2.,
            spin_rise_velocity: 3.2,
            spin_horizontal: 6.,
        }
    }
}

// Giant Punch: SpecialNStart (the wind-up), SpecialNLoop (one arm swing per pass,
// swings stored), SpecialN (the release), SpecialNCansel (let go, keeping the swings).
const PUNCH_START: u32 = 0;
const PUNCH_LOOP: u32 = 1;
const PUNCH_RELEASE: u32 = 2;
const PUNCH_CANCEL: u32 = 3;
/// Ticks in one arm swing (SpecialNLoop is 12 frames).
const SWING_TICKS: u32 = 12;
/// Each stored swing also adds knockback (base units) to the release (host adaptation).
const SWING_KNOCKBACK: u32 = 6;
// Hand Slap: SpecialLw (the wind-up), SpecialLwLoop (two slams a pass), SpecialLwEnd.
const SLAP_START: u32 = 0;
const SLAP_LOOP: u32 = 1;
const SLAP_END: u32 = 2;
/// The slams' frames in SpecialLwLoop (their hitboxes and ThrowFlags).
const SLAP_FIRST: f32 = 5.;
const SLAP_SECOND: f32 = 16.;
/// Spinning Kong's horizontal steering per tick, as a share of the way to the stick.
const SPIN_STEER: f32 = 0.12;

pub struct Donkey {
    specials: Specials,
}
impl Donkey {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Donkey Kong behavior for headless tests.
    pub fn fixture() -> Self {
        Self::new(Specials::fixture())
    }

    fn hold_gravity(sim: &mut Sim, gravity: f32) {
        sim.pad_gravity = Some(Vec3::new(0., -gravity, 0.));
        sim.owns_gravity = true;
    }
    fn release_gravity(sim: &mut Sim) {
        if sim.owns_gravity {
            sim.pad_gravity = None;
            sim.owns_gravity = false;
        }
    }

    // --- Giant Punch ---------------------------------------------------------------
    fn begin_release(&self, sim: &mut Sim) {
        // The stored swings go into this punch and are spent.
        sim.stage_len = sim.shot_charge.min(self.specials.max_swings);
        sim.shot_charge = 0;
        sim.stage = PUNCH_RELEASE;
        sim.stage_frame = 0;
        sim.stage_value = 0.;
        sim.action.projectile_fired = false;
    }
    fn tick_punch(&self, sim: &mut Sim, input: Input) {
        sim.stage_frame += 1;
        if sim.grounded {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
        match sim.stage {
            PUNCH_LOOP => {
                if input.buttons & CANCEL != 0 {
                    sim.stage = PUNCH_CANCEL;
                    sim.stage_frame = 0;
                } else if sim.stage_frame % SWING_TICKS == 0 {
                    // ftDk_SpecialNLoop_Anim: each pass stores one more swing, up to the max,
                    // and a full arm lets go on its own.
                    sim.shot_charge = (sim.shot_charge + 1).min(self.specials.max_swings);
                    if sim.shot_charge >= self.specials.max_swings {
                        self.begin_release(sim);
                    }
                }
                // Letting go of the button releases the punch (the source: a second press).
                if sim.stage == PUNCH_LOOP && input.buttons & NEUTRAL_B == 0 {
                    self.begin_release(sim);
                }
            }
            PUNCH_RELEASE => {
                // The clip makes a weak hit on frame 17 and the real punch on frame 18
                // (cleared on 22); every stored swing adds damage to the real one
                // (ftDk_SpecialN_Anim: x10 + swings * x30).
                if sim.stage_value < 1. && sim.motion.frame >= 19. && !sim.action.active.is_empty() {
                    sim.stage_value = 1.;
                    // ftDk_SpecialN_Anim: the swing's streak (efSync 1224 ground, 1225 air).
                    let id = if sim.grounded { 1224 } else { 1225 };
                    let streak = crate::effect::part_cue(sim, id, crate::effect::part::TOP_N);
                    sim.action.effects.push(streak);
                    let swings = sim.stage_len as f32;
                    for active in sim.action.active.values_mut() {
                        active.hitbox.damage += swings * self.specials.damage_per_swing;
                        active.hitbox.base_knockback += sim.stage_len * SWING_KNOCKBACK;
                    }
                }
            }
            _ => {}
        }
    }

    // --- Hand Slap -----------------------------------------------------------------
    fn tick_slap(&self, sim: &mut Sim, host: &mut dyn Host) {
        sim.stage_frame += 1;
        // He stays put through the whole slap.
        sim.velocity.x = 0.;
        sim.velocity.z = 0.;
        if sim.grounded {
            Self::hold_gravity(sim, 0.);
        } else {
            Self::release_gravity(sim);
        }
        if sim.stage == SLAP_LOOP {
            // The slams land on frames 5 and 16 of each pass; the host sends a shockwave
            // out along the ground from each. strength 1 is the first slam, 2 the second.
            let frame = sim.motion.frame;
            let last = sim.stage_value;
            for (mark, strength) in [(SLAP_FIRST, 1.), (SLAP_SECOND, 2.)] {
                if last < mark && frame >= mark {
                    // ftDk_SpecialLw_8010E0CC / 8010E148: the slam's burst on TransN.
                    let id = if sim.grounded { 1222 } else { 1223 };
                    let burst = crate::effect::part_cue(sim, id, crate::effect::part::TRANS_N);
                    sim.action.effects.push(burst);
                    sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, strength);
                }
            }
            sim.stage_value = frame;
        }
    }
}
impl Behavior for Donkey {
    fn mobile_moves(&self) -> u64 {
        0
    }

    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) {
        if m == Move::SpecialHi {
            // Spinning Kong rises to the host's standard jump apex while drifting
            // horizontally. ftdonkeyspecialhi.c::ftDk_SpecialHi_Enter.
            sim.pad_gravity = None;
            sim.grounded = false;
            sim.velocity.y = if let Some(height) = sim.host_jump_height {
                crate::controls::jump_velocity(height, sim.params.gravity)
            } else {
                self.specials.spin_rise_velocity * sim.options.jump_scale
            };
            let aim = sim.move_aim.with_y(0.).normalize_or_zero();
            if aim.length_squared() > 0.001 {
                sim.velocity.x = aim.x * self.specials.spin_horizontal * sim.options.speed_scale;
                sim.velocity.z = aim.z * self.specials.spin_horizontal * sim.options.speed_scale;
            }
            sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
            // ftDk_SpecialHi_Enter: the spin's wind (efSync 1226, TopN).
            let wind = crate::effect::part_cue(sim, 1226, crate::effect::part::TOP_N);
            sim.action.effects.push(wind);
        } else if m == Move::SpecialS {
            // Headbutt: he stays where he is on the ground; in the air the momentum is
            // divided down and the fall waits for the strike
            // (ftDk_SpecialS_Enter / ftDk_SpecialAirS_Enter).
            if sim.grounded {
                sim.velocity.x = 0.;
                sim.velocity.z = 0.;
            } else {
                let divisor = self.specials.headbutt_divisor.abs().max(1.);
                sim.velocity.x /= divisor;
                sim.velocity.z /= divisor;
                sim.velocity.y = 0.;
            }
        } else if m == Move::SpecialN {
            // Giant Punch: an arm already at full swing winds up no further.
            if !sim.grounded {
                sim.velocity = sim.facing * self.specials.punch_velocity * sim.options.speed_scale * 0.5;
            } else {
                sim.velocity.x = 0.;
                sim.velocity.z = 0.;
            }
            if sim.shot_charge >= self.specials.max_swings {
                self.begin_release(sim);
            }
        } else if m == Move::SpecialLw {
            // Hand Slap: he plants himself (efAsync 1228 on TopN while it lasts).
            sim.velocity = Vec3::ZERO;
            let aura = crate::effect::part_cue(sim, 1228, crate::effect::part::TOP_N);
            sim.action.effects.push(aura);
        }
    }
    fn charge_steps(&self) -> u32 {
        self.specials.max_swings.max(1)
    }
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, _pressed: u32) {
        let mine = matches!(
            sim.current_move,
            Some(Move::SpecialN | Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
        );
        if !mine || sim.phase == Phase::Free {
            // Ended or interrupted: never leave a held gravity override behind.
            Self::release_gravity(sim);
            return;
        }
        match sim.current_move {
            Some(Move::SpecialN) => self.tick_punch(sim, input),
            Some(Move::SpecialS) => {
                // The aerial headbutt hangs until the strike, then falls
                // (ftDk_SpecialAirS_Phys reads cmd var 0).
                if !sim.grounded && sim.action.variables[0] != 1 {
                    Self::hold_gravity(sim, 0.);
                    sim.velocity.y = 0.;
                } else {
                    Self::release_gravity(sim);
                }
            }
            Some(Move::SpecialHi) => {
                // Strong horizontal travel, steered by the move stick, over a limited rise.
                sim.stage_frame += 1;
                let stick = input.movement.with_y(0.);
                if stick.length() > 0.2 {
                    let target = stick.normalize() * self.specials.spin_horizontal * sim.options.speed_scale;
                    sim.velocity.x += (target.x - sim.velocity.x) * SPIN_STEER;
                    sim.velocity.z += (target.z - sim.velocity.z) * SPIN_STEER;
                }
            }
            Some(Move::SpecialLw) => self.tick_slap(sim, host),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, input: Input) -> bool {
        match (m, sim.stage) {
            (Move::SpecialN, PUNCH_START) => {
                if sim.shot_charge >= self.specials.max_swings {
                    self.begin_release(sim);
                } else {
                    sim.stage = PUNCH_LOOP;
                    sim.stage_frame = 0;
                }
                true
            }
            // The loop clip loops on its own; its swings are counted in the tick.
            (Move::SpecialN, PUNCH_LOOP) => true,
            (Move::SpecialLw, SLAP_START) => {
                sim.stage = SLAP_LOOP;
                sim.stage_frame = 0;
                sim.stage_value = 0.;
                true
            }
            (Move::SpecialLw, SLAP_LOOP) => {
                if input.buttons & DOWN_B != 0 {
                    // Held: another pass, fresh hitboxes. It goes on as long as he holds B.
                    sim.motion.frame = 0.;
                    sim.stage_value = 0.;
                    sim.action.active.clear();
                } else {
                    sim.stage = SLAP_END;
                    sim.stage_frame = 0;
                }
                true
            }
            _ => false,
        }
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        let now = sim.grounded;
        match (m, sim.stage) {
            (Move::SpecialN, PUNCH_START) => {
                if now { "SpecialNStart" } else { "SpecialAirNStart" }
            }
            (Move::SpecialN, PUNCH_LOOP) => {
                if now { "SpecialNLoop" } else { "SpecialAirNLoop" }
            }
            (Move::SpecialN, PUNCH_RELEASE) => {
                if now { "SpecialN" } else { "SpecialAirN" }
            }
            (Move::SpecialN, _) => {
                if now { "SpecialNCansel" } else { "SpecialAirNCancel" }
            }
            (Move::SpecialS, _) => {
                if now { "SpecialS" } else { "SpecialAirS" }
            }
            (Move::SpecialHi, _) => {
                if sim.move_started_grounded { "SpecialHi" } else { "SpecialAirHi" }
            }
            (Move::SpecialLw, SLAP_START) => "SpecialLw",
            (Move::SpecialLw, SLAP_LOOP) => "SpecialLwLoop",
            (Move::SpecialLw, _) => "SpecialLwEnd",
            _ => "",
        }
    }
    fn facing_locked_now(&self, sim: &Sim, m: Move) -> bool {
        self.facing_locked(m) || (m == Move::SpecialN && sim.stage >= PUNCH_RELEASE)
    }
    fn projectile_move(&self, _m: Move) -> bool {
        false
    }
    fn phase_effect(&self, _sim: &Sim, _phase: Phase) -> Option<crate::action::GfxCue> {
        None
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(20), // Giant Punch wind-up
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
