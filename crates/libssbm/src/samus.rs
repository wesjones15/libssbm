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

/// Host adaptation: down-B drops one bomb, then holds a steerable Morph Ball.
pub const HOST_MORPH_ROLL: u32 = 4;
const MORPH_ROLL: u32 = 20;
const MORPH_BOX: u32 = 0x8001;
const MORPH_HIT_TICKS: u32 = 8;
const MORPH_DUST_TICKS: u32 = 6;
const MORPH_BOUNCE_COOLDOWN: u32 = 8;
/// The down-B clip is 54 frames: it morphs in by 13, holds the ball, and unfolds from
/// 43. Freeze on 13 so the ball stays out instead of playing the unfold.
const MORPH_BALL_FRAME: f32 = 13.;
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
    /// Authored no-assets tuning (rounded approximations of the asset-backed fighter's): seven
    /// charge steps 17 ticks apart, the Screw Attack's rise and the bomb hop.
    pub fn authored() -> Self {
        Self {
            charge_steps: 7,
            charge_interval: 17,
            missile_spawn_offset: 1.,
            bomb_spawn_offset: [0., 2., 0.],
            screw_velocity: 2.5,
            bomb_velocity: 0.9,
            ..Self::fixture()
        }
    }
    /// Synthetic baseline for tests and built-in play; disc-backed play loads tuning.
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
    /// The no-assets profile's Samus: authored tuning approximating the asset-backed fighter
    /// (see `authored::samus` and docs/samus-reference.md), not extracted values.
    pub fn authored() -> Self {
        Self::new(Specials::authored(), None)
    }
    fn morph_roll(sim: &Sim) -> bool {
        sim.current_move == Some(Move::SpecialLw)
            && (sim.morph_roll_hold || sim.host_flags & HOST_MORPH_ROLL != 0)
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
        sim.morph_roll_hold = true;
        sim.morph_roll_distance = 0.;
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
        let broke = sim.roll_break;
        sim.action.reset();
        sim.roll_break = false;
        sim.morph_roll_hold = false;
        // The same shell-bit burst Yoshi's egg roll ends with (efSync 0x4CF).
        for _ in 0..3 {
            let shell = crate::effect::part_cue(sim, 1231, crate::effect::part::HIP_N);
            sim.action.effects.push(shell);
        }
        Self::release_gravity(sim);
        sim.current_move = None;
        sim.set_phase(Phase::Free);
        sim.velocity.x = 0.;
        sim.velocity.z = 0.;
        if broke {
            // Knocked out of the ball (the host's armor ran out): fall into the knockdown
            // (DownBoundU), then get up, instead of standing straight back up.
            sim.reaction = Some(crate::reaction::Stage::Bound);
            sim.motion.select("DownBoundU", 1.);
        } else {
            // Released: back to normal Samus at once, not the down-B's folded end pose.
            sim.motion.select("Wait", 1.);
        }
        sim.motion.frame = 0.;
    }
    fn tick_morph_roll(&self, sim: &mut Sim, input: Input, pressed: u32) {
        let scale = sim.options.speed_scale;
        match sim.stage {
            MORPH_ROLL => {
                sim.stage_frame += 1;
                // The ball hit an enemy last tick: bounce back off it, like the wall bonk.
                if sim.roll_bonk {
                    sim.roll_bonk = false;
                    sim.stage_dir = -sim.stage_dir;
                    sim.stage_value *= 0.7;
                    sim.stage_len = MORPH_BOUNCE_COOLDOWN;
                    let bonk = crate::effect::part_cue(sim, 1030, crate::effect::part::TOP_N);
                    sim.action.effects.push(bonk);
                }
                // Keep the down-B action open for as long as the ball is held. Without
                // this the generic startup/recovery clock ends the move after a few
                // seconds, dropping the ball pose while LT is still down.
                sim.phase = Phase::Startup;
                sim.phase_frame = 0;
                // Freeze on the ball part of the morph clip. Its last frames are the
                // unfold, which is why the old end-of-clip hold became folded Samus.
                if matches!(sim.motion.clip, "SpecialLw" | "SpecialAirLw")
                    && sim.motion.frame >= MORPH_BALL_FRAME
                {
                    sim.motion.frame = MORPH_BALL_FRAME;
                    sim.motion.rate = 0.;
                }
                sim.stage_len = sim.stage_len.saturating_sub(1);
                let movement = input.movement.with_y(0.);
                if movement.length() > 0.3 {
                    sim.stage_dir = movement.normalize();
                }
                let moved = (sim.position - sim.stage_vec).with_y(0.).length();
                sim.morph_roll_distance += moved;
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
                // ftYs_SpecialS_8012F0DC -> efSync 0x3FF -> efasync.c
                // case 0x3FF -> EfCoData model 5. Host adaptation: retain
                // Yoshi's roll trail in air while the ball keeps spinning.
                if sim.stage_frame % MORPH_DUST_TICKS == 0 {
                    let dust = crate::effect::part_cue(sim, 1023, crate::effect::part::TOP_N);
                    sim.action.effects.push(dust);
                }
                if sim.stage_frame == 1 || sim.stage_frame % MORPH_HIT_TICKS == 0 {
                    Self::morph_hitbox(sim);
                }
                // Hold until release or armor depletion; the animation never loops.
                if sim.roll_break || input.buttons & DOWN_B == 0 {
                    Self::end_morph_roll(sim);
                }
            }
            _ => {}
        }
    }
}
impl Behavior for Samus {
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
    fn enter_special(&self, sim: &mut Sim, host: &mut dyn Host, m: Move, _input: Input) {
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
                // Combined RoR2 attack: one bomb at entry, on ground or in air.
                // The source article callback is ftSs_SpecialLw_8012ADF0;
                // its later throw flag is muted so it cannot drop a second bomb.
                sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 1.);
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
        if Self::morph_roll(sim) { return true; }
        if sim.current_move == Some(Move::SpecialN)
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
        if pressed & crate::sim::JUMP != 0
            && (sim.grounded || sim.jumps_used < sim.params.jumps)
        {
            // Jump while keeping the charge: the split anim layers the charge upper
            // body over the jump/fall legs instead of ending the hold.
            sim.short_hop = false;
            sim.jump_holding(host, input);
        } else if pressed & (crate::sim::CANCEL | crate::sim::GRAB) != 0
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
        } else if (sim.options.hold_charge && input.buttons & crate::sim::SPECIAL == 0)
            || (!sim.options.hold_charge && pressed & crate::sim::SPECIAL != 0)
        {
            sim.fire(host);
        }
    }

    fn tick_special(&self, sim: &mut Sim, _host: &mut dyn Host, input: Input, pressed: u32) {
        if Self::morph_roll(sim) {
            Self::tick_morph_roll(self, sim, input, pressed);
        }
    }
    fn next_stage(&self, sim: &mut Sim, _host: &mut dyn Host, _m: Move, _input: Input) -> bool {
        if Self::morph_roll(sim) {
            // If the freeze in tick_special is missed (a different clip rate), hold the
            // ball frame here instead of ending on the clip's folded last frame.
            sim.motion.frame = MORPH_BALL_FRAME;
            sim.motion.rate = 0.;
            return true;
        }
        false
    }
    fn script_muted(&self, sim: &Sim) -> bool {
        // Entry already emitted the combined move's bomb; suppress the clip's
        // projectile flag and hitboxes for the remainder of the held roll.
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
    fn animation_rate(&self, sim: &Sim) -> f32 {
        if Self::morph_roll(sim) && sim.motion.frame >= MORPH_BALL_FRAME { 0. } else { 1. }
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
                // A roll jump/landing keeps the same frozen ball clip instead of
                // restarting the morph animation at every ground-air transition.
                let grounded = if Self::morph_roll(sim) { sim.move_started_grounded } else { sim.grounded };
                if grounded { "SpecialLw" } else { "SpecialAirLw" }
            }
            _ => "",
        }
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
