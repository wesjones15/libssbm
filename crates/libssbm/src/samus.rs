//! Samus-specific special-move behavior: Charge Shot, missiles, Screw Attack,
//! Bomb, plus the grapple-based throws the shared executor already understands.
//! References: ftSamus/ftsamusspecial{n,s,hi,lw1}.c and its article attributes.
use crate::{
    action::{ActiveHitbox, GfxCue, Hitbox, SoundCue},
    behavior::Behavior,
    sim::{
        side_special_upper_angles, Host, Input, Move, Phase, Sim, DOWN_B, JUMP, NEUTRAL_B, SIDE_B,
        UP_B,
    },
    EventKind,
};
use glam::Vec3;

/// Host flag: Samus's down-B is the Morph Ball Roll alternate, not the bomb.
pub const HOST_MORPH_ROLL: u32 = 4;
const MORPH_ROLL: u32 = 20;
const MORPH_BOX: u32 = 0x8001;
const MORPH_HIT_TICKS: u32 = 8;
const MORPH_DUST_TICKS: u32 = 6;
const MORPH_BOUNCE_COOLDOWN: u32 = 8;
const MORPH_SPEED: f32 = 2.6;
const MORPH_ACCEL: f32 = 0.06;

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
    fn morph_roll(sim: &Sim) -> bool {
        sim.current_move == Some(Move::SpecialLw) && sim.host_flags & HOST_MORPH_ROLL != 0
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
    fn begin_morph_roll(sim: &mut Sim) {
        sim.stage = MORPH_ROLL;
        sim.stage_frame = 0;
        sim.stage_len = 0;
        sim.stage_dir = sim.facing.with_y(0.).normalize_or_zero();
        if sim.stage_dir.length_squared() < 0.01 {
            sim.stage_dir = Vec3::X;
        }
        sim.stage_vec = sim.position;
        sim.stage_value = MORPH_SPEED * 0.5;
        sim.roll_break = false;
        sim.roll_still = 0;
        sim.action.active.clear();
        sim.velocity.x = 0.;
        sim.velocity.z = 0.;
    }
    /// The Morph Ball Roll hit: light damage, no knockback, so contact only bounces Samus.
    fn morph_hitbox(sim: &mut Sim) {
        let slot = &mut sim.action.generations[0];
        *slot = slot.wrapping_add(1);
        let generation = *slot;
        sim.action.active.insert(
            MORPH_BOX,
            ActiveHitbox {
                hitbox: Hitbox {
                    id: MORPH_BOX,
                    group: 0,
                    bone: 0,
                    common_bone: false,
                    damage: 3.,
                    radius: 7.,
                    offset: Vec3::new(0., 4., 0.),
                    angle: 80,
                    growth: 0,
                    fixed_knockback: 0,
                    base_knockback: 0,
                    element: 0,
                    sfx_kind: 1,
                    sfx_severity: 0,
                    ground: true,
                    air: true,
                },
                generation,
            },
        );
    }
    fn end_morph_roll(sim: &mut Sim) {
        sim.action.active.clear();
        sim.roll_break = false;
        // The same shell-bit burst Yoshi's egg roll ends with (efSync 0x4CF).
        for _ in 0..3 {
            let shell = crate::effect::part_cue(sim, 1231, crate::effect::part::HIP_N);
            sim.action.effects.push(shell);
        }
        // Back to normal Samus at once, not the down-B's folded end pose.
        Self::release_gravity(sim);
        sim.current_move = None;
        sim.set_phase(Phase::Free);
        sim.motion.select("Wait", 1.);
        sim.velocity.x = 0.;
        sim.velocity.z = 0.;
    }
    fn tick_morph_roll(&self, sim: &mut Sim, input: Input, pressed: u32) {
        let scale = sim.options.speed_scale;
        match sim.stage {
            MORPH_ROLL => {
                sim.stage_frame += 1;
                // Keep the down-B action open for as long as the ball is held. Without
                // this the generic startup/recovery clock ends the move after a few
                // seconds, dropping the ball pose while LT is still down.
                sim.phase = Phase::Startup;
                sim.phase_frame = 0;
                sim.stage_len = sim.stage_len.saturating_sub(1);
                let movement = input.movement.with_y(0.);
                if movement.length() > 0.3 {
                    sim.stage_dir = movement.normalize();
                }
                let moved = (sim.position - sim.stage_vec).with_y(0.).length();
                if moved < 0.005 {
                    sim.roll_still = sim.roll_still.saturating_add(1);
                } else {
                    sim.roll_still = 0;
                }
                let mut speed = sim.stage_value;
                if sim.stage_frame > 2 {
                    speed = (speed + MORPH_ACCEL * scale).min(MORPH_SPEED * scale);
                }
                if sim.stage_frame > 6 && sim.stage_len == 0 && moved < speed * scale * 0.3 {
                    sim.stage_dir = -sim.stage_dir;
                    speed *= 0.7;
                    sim.stage_len = MORPH_BOUNCE_COOLDOWN;
                    let bonk = crate::effect::part_cue(sim, 1030, crate::effect::part::TOP_N);
                    sim.action.effects.push(bonk);
                }
                sim.stage_vec = sim.position;
                sim.stage_value = speed;
                sim.facing = sim.stage_dir;
                sim.velocity.x = sim.stage_dir.x * speed * scale;
                sim.velocity.z = sim.stage_dir.z * speed * scale;
                let gravity = if sim.grounded { 0. } else { sim.params.gravity };
                Self::hold_gravity(sim, gravity);
                // The ball can jump: a grounded hop or the fighter's remaining air jump.
                if pressed & JUMP != 0 {
                    let grounded = sim.grounded;
                    if grounded || sim.jumps_used < sim.params.jumps {
                        sim.velocity.y = match sim.host_jump_height {
                            Some(height) => crate::controls::jump_velocity(height, sim.params.gravity),
                            None => 3.,
                        };
                        sim.grounded = false;
                        sim.jumps_used = if grounded { 1 } else { sim.jumps_used + 1 };
                    }
                }
                if sim.grounded && sim.stage_frame % MORPH_DUST_TICKS == 0 {
                    let dust = crate::effect::part_cue(sim, 1023, crate::effect::part::TOP_N);
                    sim.action.effects.push(dust);
                }
                if sim.stage_frame == 1 || sim.stage_frame % MORPH_HIT_TICKS == 0 {
                    Self::morph_hitbox(sim);
                }
                // No duration or idle timeout: the ball lasts until LT is released
                // or the host's armor breaks.
                if sim.roll_break || input.buttons & DOWN_B == 0 {
                    Self::end_morph_roll(sim);
                }
            }
            _ => {}
        }
    }
}
impl Behavior for Samus {
    fn slot(&self, m: Move) -> u32 {
        match m {
            Move::SpecialN => 0,
            Move::SpecialS | Move::SpecialS2 => 2,
            Move::SpecialHi => 3,
            Move::SpecialLw => 1,
            _ => 4,
        }
    }
    fn mobile_moves(&self) -> u64 {
        (1u64 << Move::SpecialN as u32)
            | (1u64 << Move::SpecialS as u32)
            | (1u64 << Move::SpecialS2 as u32)
            | (1u64 << Move::SpecialHi as u32)
            | (1u64 << Move::SpecialLw as u32)
    }
    fn upper_body_root(&self) -> Option<usize> {
        self.upper_body_root
    }
    fn is_up_special(&self, m: Move) -> bool {
        m == Move::SpecialHi
    }

    fn select_special(&self, sim: &Sim, input: Input, movement: Vec3) -> Option<Move> {
        let smash = sim.flick_age < self.specials.missile_smash_window;
        Some(if input.buttons & NEUTRAL_B != 0 {
            Move::SpecialN
        } else if input.buttons & UP_B != 0 {
            Move::SpecialHi
        } else if input.buttons & DOWN_B != 0 {
            Move::SpecialLw
        } else if input.buttons & SIDE_B != 0 {
            if smash { Move::SpecialS2 } else { Move::SpecialS }
        } else if input.attack_vertical() > 0 {
            Move::SpecialHi
        } else if input.attack_vertical() < 0 {
            Move::SpecialLw
        } else if movement.length() > 0.3 {
            if smash { Move::SpecialS2 } else { Move::SpecialS }
        } else {
            Move::SpecialN
        })
    }
    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) {
        if m == Move::SpecialHi {
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
        } else if m == Move::SpecialLw {
            if sim.host_flags & HOST_MORPH_ROLL != 0 {
                Self::begin_morph_roll(sim);
            } else if !sim.grounded {
                sim.pad_gravity = None;
                sim.velocity.y = self.specials.bomb_velocity;
            }
        }
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS | Move::SpecialS2)
    }
    fn is_charge_move(&self, m: Move) -> bool {
        m == Move::SpecialN
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
    /// Samus's charge shot is the one neutral special that genuinely replays its
    /// clip: `SpecialNStart` finishes, then `SpecialN` plays the release. Every
    /// other fighter's neutral special is a single pass, so the shared default is
    /// `false`; Samus opts back into the replay.
    fn fire_replays(&self, _m: Move) -> bool {
        true
    }
    fn store_special(&self, sim: &mut Sim, host: &mut dyn Host, pressed: u32) -> bool {
        if sim.current_move == Some(Move::SpecialN)
            && matches!(sim.phase, Phase::Startup | Phase::ChargeShot)
            && pressed & crate::sim::SPRINT != 0
        {
            sim.emit(
                host,
                EventKind::ChargeStored,
                Move::SpecialN as u32,
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
        if sim.current_move == Some(Move::SpecialN)
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
        if m == Move::SpecialN {
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
                Move::SpecialN as u32,
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

    fn tick_special(&self, sim: &mut Sim, _host: &mut dyn Host, input: Input, pressed: u32) {
        if Self::morph_roll(sim) {
            Self::tick_morph_roll(self, sim, input, pressed);
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, _m: Move, _input: Input) -> bool {
        if Self::morph_roll(sim) {
            // The down-B morph clip is a one-shot; hold its last frame while the ball
            // rolls, so the ball does not re-morph every loop.
            sim.motion.frame = (sim.motion.length(sim.motion.clip) - 1.).max(0.);
            return true;
        }
        false
    }
    fn script_muted(&self, sim: &Sim) -> bool {
        // The roll has no bomb article/hitbox: all of that belongs to the ordinary down-B.
        Self::morph_roll(sim)
    }
    fn on_connect(&self, sim: &mut Sim) {
        if Self::morph_roll(sim) && sim.stage == MORPH_ROLL && sim.stage_len == 0 {
            sim.stage_dir = -sim.stage_dir;
            sim.stage_value *= 0.7;
            sim.stage_len = MORPH_BOUNCE_COOLDOWN;
            let bonk = crate::effect::part_cue(sim, 1030, crate::effect::part::TOP_N);
            sim.action.effects.push(bonk);
        }
    }
    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        match m {
            Move::SpecialN => {
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
            Move::SpecialS => {
                if sim.grounded { "SpecialS" } else { "SpecialAirS" }
            }
            Move::SpecialS2 => {
                if sim.grounded { "Special" } else { "SpecialAir" }
            }
            Move::SpecialHi => {
                if sim.move_started_grounded {
                    "SpecialHi"
                } else {
                    "SpecialAirHi"
                }
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
        matches!(m, Move::SpecialLw | Move::SpecialHi)
    }
    fn side_special_layered(&self, m: Move) -> bool {
        matches!(m, Move::SpecialS | Move::SpecialS2)
    }
    fn side_special_upper_aim(&self, sim: &Sim) -> Option<(f32, f32)> {
        matches!(sim.current_move, Some(Move::SpecialS | Move::SpecialS2))
            .then(|| side_special_upper_angles(sim.move_aim, sim.facing))
    }
    fn hit_attachment(&self, sim: &Sim, m: Move) -> (Option<usize>, Vec3) {
        match m {
            Move::SpecialN => (Some(51), Vec3::ZERO),
            Move::SpecialS | Move::SpecialS2 => {
                (Some(56), sim.facing * self.specials.missile_spawn_offset)
            }
            Move::SpecialLw => (
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
            Move::SpecialN | Move::SpecialS | Move::SpecialS2 | Move::SpecialLw
        )
    }
    fn projectile_cue(&self, m: Move) -> Option<(u32, usize, bool)> {
        match m {
            Move::SpecialN => Some((1158, 51, false)),
            Move::SpecialS | Move::SpecialS2 => Some((1155, 56, false)),
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
            Move::SpecialN => Some(12),
            Move::SpecialS | Move::SpecialS2 => Some(12),
            Move::SpecialLw => Some(10),
            Move::SpecialHi => Some(3),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(24),
            Move::SpecialS | Move::SpecialS2 => Some(24),
            Move::SpecialLw => Some(25),
            Move::SpecialHi => Some(36),
            _ => None,
        }
    }
}
