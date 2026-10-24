//! Behavioral references: ftCommon/{ftCo_KneeBend,ftCo_Jump,ftCo_JumpAerial,
//! ftCo_AttackS4}.c and ftSamus/ftsamusspecial{n,s,hi,lw1}.c.
//! Move startup/active/recovery schedules here are prototype options, pending the
//! animation command reader. They are deliberately separate from disc parameters.
use crate::{behavior::Behavior, Params};
use glam::Vec3;
mod actions;
mod scope_move;

pub const TICK_HZ: f64 = 60.;
pub const JUMP: u32 = 1;
pub const ATTACK: u32 = 2;
pub const SPECIAL: u32 = 4;
pub const UP: u32 = 8;
pub const DOWN: u32 = 16;
pub const SPRINT: u32 = 32;
pub const CANCEL: u32 = 64;
pub const GRAB: u32 = 128;
pub const NEUTRAL_B: u32 = 256;
pub const SIDE_B: u32 = 512;
pub const UP_B: u32 = 1024;
pub const DOWN_B: u32 = 2048;
pub const DEFEND: u32 = 4096;
/// The sound channel of the fighter's voice (script sounds use channels 0 to 6).
pub const VOICE_CHANNEL: u32 = 8;

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// World-space analog direction. Length must retain the stick's magnitude.
    pub movement: Vec3,
    pub aim: Vec3,
    pub buttons: u32,
    /// Bit 0 attack, 1 charge shot, 2 missile/bomb, 3 screw attack. Host gates.
    pub allowed: u32,
}
impl Input {
    /// RoR2 3D adaptation, not original Melee stick thresholds: a directional
    /// modifier enables camera pitch. Up starts at 20 degrees; down requires 50
    /// degrees to avoid turning ordinary ground aiming into downward attacks.
    /// Physical shoulders remain unconditional up/down and take priority over LS.
    pub fn attack_vertical(self) -> i8 {
        if self.buttons & UP != 0 {
            return 1;
        }
        if self.buttons & DOWN != 0 {
            return -1;
        }
        if self.movement.with_y(0.).length_squared() <= 0.09 {
            return 0;
        }
        let y = self.aim.normalize_or_zero().y;
        if y >= 0.34202015 {
            1
        } else if y <= -0.76604444 {
            -1
        } else {
            0
        }
    }
    pub fn idle() -> Self {
        Self {
            aim: Vec3::X,
            allowed: 31,
            ..Self::default()
        }
    }
}

/// The jab chain's counters (ftCo_Attack1.c, ftCo_Attack100.c), kept across the jabs of one chain.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct JabChain {
    /// The jab last started (fp->unk_msid) and the ticks left to continue it (fp->hitlag_mul).
    pub last: Option<Move>,
    pub window: u32,
    /// A press landed inside the window (fp->mv.co.attack1.x0).
    pub queued: bool,
    /// Presses and releases of A since the chain's first jab (fp->x1A54).
    pub presses: u32,
    /// Command 30 opened the rapid jab (fp->x2218_b2).
    pub rapid_ok: bool,
    /// The rapid jab's loop has begun (attack100.x0) and A moved since its last beat (x4).
    pub looping: bool,
    pub fed: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Jab = 1,
    SideTilt = 2,
    UpTilt = 3,
    DownTilt = 4,
    DashAttack = 5,
    NeutralAir = 6,
    ForwardAir = 7,
    UpAir = 8,
    DownAir = 9,
    SideSmash = 10,
    UpSmash = 11,
    DownSmash = 12,
    Jab2 = 13,
    BackAir = 14,
    /// The third jab (Attack13) and the rapid jab (Attack100Start, Loop, End).
    Jab3 = 15,
    RapidJab = 16,
    /// The Super Scope held and fired (a host-selected alternate of a fighter's neutral special; see `scope`).
    SuperScope = 17,
    // Special-move slots 20..24 are fighter-neutral: each fighter's `Behavior`
    // maps its own specials here (neutral, side, side-strong, up, down).
    SpecialN = 20,
    SpecialS = 21,
    SpecialS2 = 22,
    SpecialHi = 23,
    SpecialLw = 24,
    Grab = 25,
    DashGrab = 26,
    AirGrab = 27,
    Pummel = 28,
    ThrowForward = 29,
    ThrowBack = 30,
    ThrowUp = 31,
    ThrowDown = 32,
    GrabHold = 33,
}
impl Move {
    pub fn is_smash(self) -> bool {
        matches!(self, Self::SideSmash | Self::UpSmash | Self::DownSmash)
    }
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Started = 1,
    Hit = 2,
    Jump = 3,
    Land = 4,
    ChargeStored = 5,
    Throw = 6,
}
#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub kind: EventKind,
    pub move_id: u32,
    pub strength: f32,
    pub position: Vec3,
    pub direction: Vec3,
    pub facing: Vec3,
    pub clip: &'static str,
    pub pose_frame: f32,
    pub gait_clip: &'static str,
    pub gait_frame: f32,
    pub gait_weight: f32,
    pub attachment: Option<usize>,
    pub attachment_offset: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub position: Vec3,
    pub grounded: bool,
    pub ceiling: bool,
}
pub trait Host {
    /// Sweep the capsule from `from` to `desired`, resolving ground, roof, walls.
    fn collide(&mut self, from: Vec3, desired: Vec3) -> Contact;
    fn event(&mut self, event: Event);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Free,
    JumpSquat,
    Startup,
    SmashCharge,
    ChargeShot,
    Recovery,
    Helpless,
    ChargeCancel,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub speed_scale: f32,
    pub jump_scale: f32,
    pub extra_jumps: u32,
    /// Host adaptation: hold LT to charge; release fires. False is press to start,
    /// press again to fire, cancel/jump to store, as SpecialNHold_IASA.
    pub hold_charge: bool,
    /// Prototype max charge; true move command values will replace this schedule.
    pub smash_max_frames: u32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            speed_scale: 1.,
            jump_scale: 1.,
            extra_jumps: 0,
            hold_charge: false,
            smash_max_frames: 60,
        }
    }
}

/// A part animation in progress: which of the slot's animations, how many ticks it has
/// played, and the blend frames its command gave.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartPlay {
    pub index: u8,
    pub age: f32,
    pub blend: u16,
}

pub struct Sim {
    pub grabbed: bool,
    pub defense: crate::defense::Defense,
    pub reaction: Option<crate::reaction::Stage>,
    pub entry: Option<crate::entry::State>,
    pub rebirth: Option<crate::rebirth::State>,
    pub action: crate::action::ActionState,
    pub motion: crate::motion::MotionState,
    /// Independent 60 Hz gait clock for upper-body action overlays.
    pub locomotion: crate::motion::MotionState,
    pub locomotion_weight: f32,
    pub move_started_grounded: bool,
    pub params: Params,
    pub options: Options,
    /// Per-fighter special-move behavior (specials, clips, physics).
    pub behavior: Box<dyn Behavior>,
    pub position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub phase: Phase,
    pub current_move: Option<Move>,
    pub frame: u64,
    pub phase_frame: u32,
    pub jumps_used: u32,
    pub shot_charge: u32,
    pub smash_charge: u32,
    pub facing: Vec3,
    host_facing: Option<Vec3>,
    pub host_jump_height: Option<f32>,
    accumulator: f64,
    pub fall_blend: f32,
    suppress_initial_landing: bool,
    sampled_buttons: u32,
    pending_pressed: u32,
    pending_released: u32,
    released: u32,
    /// Sounds queued between steps (a hurt voice), played at the start of the next one.
    queued_sounds: Vec<crate::action::SoundCue>,
    voice_rng: u32,
    pub(crate) jab: JabChain,
    /// The Super Scope in hand, if the host gave the fighter one (`Sim::set_scope`).
    pub scope: Option<crate::scope::Scope>,
    buffered_attack: u32,
    pub(crate) knockdown_grace: u32,
    previous: Input,
    pub(crate) flick_age: u32,
    modifier_age: u32,
    pub(crate) charge_clock: u32,
    pub(crate) short_hop: bool,
    /// Host adaptation: the up special (Screw Attack) keeps the fighter
    /// invulnerable from launch until landing, or until another attack starts.
    pub(crate) up_special_invulnerable: bool,
    pub(crate) pad_gravity: Option<Vec3>,
    pub(crate) move_aim: Vec3,
    pub(crate) move_strength: f32,
    /// Per-fighter special-move scratch; its meaning belongs to the `Behavior`
    /// (Pikachu's multi-stage Skull Bash / Quick Attack / Thunder). Reset on
    /// every move start.
    pub(crate) stage: u32,
    pub(crate) stage_frame: u32,
    pub(crate) stage_len: u32,
    pub(crate) stage_vec: Vec3,
    pub(crate) stage_dir: Vec3,
    pub(crate) stage_value: f32,
    /// Host adaptation: Yoshi's egg-roll armor ran out, so the next tick ends the roll.
    pub roll_break: bool,
    /// Consecutive rolling ticks with no horizontal movement (a stuck egg).
    pub(crate) roll_still: u32,
    /// Host lock-on: world position of the enemy the player is locked onto or
    /// aiming nearest, refreshed every tick. Staged specials (Quick Attack)
    /// steer to it; `None` falls back to stick and camera aim.
    pub lock_target: Option<Vec3>,
    /// World facts the host reports each tick (see `HOST_*` in `link.rs`): Link's
    /// boomerang being out, a carried bomb. A behavior reads them to choose a clip.
    pub host_flags: u32,
    /// Ticks left of a fluttering aerial jump (`Behavior::flutter`) and the gravity
    /// scale it applies.
    pub(crate) flutter: u32,
    pub(crate) flutter_gravity: f32,
    /// A behavior holds `pad_gravity` for a scripted move; cleared when the
    /// move ends so a special can never leave the fighter floating.
    pub(crate) owns_gravity: bool,
    /// The helpless fall an up special ended in (`Behavior::special_fall`) and the speed the
    /// landing clip plays at to last its landing lag.
    pub(crate) special_fall: Option<crate::behavior::SpecialFall>,
    pub(crate) landing_rate: f32,
    /// Set once Luigi's Cyclone has used its air lift; cleared on landing.
    pub(crate) air_charge: bool,
    /// The sword trail's state (fp->x2100): -1 off, otherwise the script turned it on and the
    /// host samples the blade each tick (see `trail`).
    pub afterimage: i32,
    /// The face's texture animation frames, one per costume TObj (fp->tobj_list): what command
    /// 40 (ftAnim_800704F0) last set; zero again whenever the motion changes (ftAnim_80070654).
    pub face: [f32; 8],
    /// The part animations scripts started (command 41, ftAnim_ApplyPartAnim), one per slot of
    /// the fighter's part table: hands, ears, tail, mouth. They end when the motion changes.
    pub parts: [Option<PartPlay>; 5],
    /// The glows and flashes scripts start (command 46), and the move and clip they began in.
    pub colanim: crate::colanim::ColorAnim,
    colanim_move: Option<Move>,
    colanim_clip: &'static str,
    /// Whether the knockback eyes (`Behavior::hurt_eyes`) are showing.
    knockback_eyes: bool,
    rng: u32,
}
impl Sim {
    /// A pseudo-random number in `0..bound` (the source's HSD_Randi), deterministic from the
    /// sim's own history so headless runs repeat.
    pub fn random(&mut self, bound: u32) -> u32 {
        if self.rng == 0 {
            self.rng = 0x2545_F491 ^ (self.frame as u32).wrapping_mul(0x9E37_79B9);
        }
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        if bound == 0 { 0 } else { x % bound }
    }
    /// Synthetic Samus for headless tests. Runtime hosts use [`Sim::with_behavior`].
    pub fn new(params: Params, position: Vec3) -> Self {
        Self::with_behavior(
            params,
            Box::new(crate::samus::Samus::fixture()),
            position,
        )
    }
    pub fn with_behavior(params: Params, behavior: Box<dyn Behavior>, position: Vec3) -> Self {
        Self {
            grabbed: false,
            defense: Default::default(),
            reaction: None,
            entry: None,
            rebirth: None,
            action: crate::action::ActionState::default(),
            motion: crate::motion::MotionState::default(),
            locomotion: crate::motion::MotionState::default(),
            locomotion_weight: 0.,
            move_started_grounded: false,
            params,
            options: Options::default(),
            behavior,
            position,
            velocity: Vec3::ZERO,
            grounded: false,
            phase: Phase::Free,
            current_move: None,
            frame: 0,
            phase_frame: 0,
            jumps_used: 1,
            shot_charge: 0,
            smash_charge: 0,
            facing: Vec3::X,
            host_facing: None,
            host_jump_height: None,
            accumulator: 0.,
            fall_blend: 0.,
            suppress_initial_landing: true,
            sampled_buttons: 0,
            pending_pressed: 0,
            pending_released: 0,
            released: 0,
            queued_sounds: Vec::new(),
            voice_rng: 0x9e37_79b9,
            jab: JabChain::default(),
            scope: None,
            buffered_attack: 0,
            knockdown_grace: 0,
            previous: Input::idle(),
            flick_age: 100,
            modifier_age: 100,
            charge_clock: 0,
            short_hop: false,
            up_special_invulnerable: false,
            pad_gravity: None,
            move_aim: Vec3::X,
            move_strength: 1.,
            stage: 0,
            stage_frame: 0,
            stage_len: 0,
            stage_vec: Vec3::ZERO,
            stage_dir: Vec3::ZERO,
            stage_value: 0.,
            roll_break: false,
            roll_still: 0,
            lock_target: None,
            host_flags: 0,
            flutter: 0,
            flutter_gravity: 1.,
            owns_gravity: false,
            special_fall: None,
            landing_rate: 1.,
            air_charge: false,
            afterimage: -1,
            face: [0.; 8],
            parts: [None; 5],
            colanim: Default::default(),
            colanim_move: None,
            colanim_clip: "",
            knockback_eyes: false,
            rng: 0,
        }
    }
    /// Split-borrow bridge: temporarily moves the behavior out so `f` can mutate
    /// the sim while holding `&dyn Behavior`, then restores it. The behavior
    /// callbacks never unwind; the FFI catches panics and poisons the handle.
    fn using_behavior<T>(&mut self, f: impl FnOnce(&dyn Behavior, &mut Sim) -> T) -> T {
        let behavior = std::mem::replace(
            &mut self.behavior,
            Box::new(crate::samus::Samus::fixture()),
        );
        let result = f(&*behavior, self);
        self.behavior = behavior;
        result
    }
    pub fn alpha(&self) -> f32 {
        (self.accumulator * TICK_HZ) as f32
    }
    /// How far a staged roll has turned, in radians: the rolling stage's ticks times its
    /// speed over the body radius. Yoshi's egg uses it for ftPartSetRotX(fp, 3, ...).
    pub fn roll_angle(&self, radius: f32) -> f32 {
        self.stage_frame as f32 * self.stage_value / radius.max(0.001)
    }
    pub fn mobile_action(&self) -> bool {
        self.grounded
            && self.reaction.is_none()
            && self.entry.is_none()
            && self.rebirth.is_none()
            && self
                .current_move
                .is_some_and(|m| self.behavior.mobile_moves() & (1u64 << m as u32) != 0)
    }
    /// Side-B (and its Super Missile) is always drawn as an upper-body layer over the
    /// gait/idle legs, grounded or airborne, moving or still. The disc clips fire toward
    /// the fighter's local +X and bake that turn into the pelvis and feet (about -68 degrees
    /// toward it, with the cannon at 180 in the raw clip), so the raw pose faces sideways.
    pub fn side_special_layered(&self) -> bool {
        self.reaction.is_none()
            && self.entry.is_none()
            && self.rebirth.is_none()
            && self.current_move.is_some_and(|m| self.behavior.side_special_layered(m))
    }
    /// Down-B and Screw Attack spin the whole body: the disc clips flip the joints below the
    /// spine (joints 2-4, 180 degrees at a time), which a gait layer would replace, leaving only
    /// the upper body bobbing and restarting. They keep any steering but draw the raw clip.
    pub fn spins_whole_body(&self) -> bool {
        self.current_move.is_some_and(|m| self.behavior.spins_whole_body(m))
    }
    /// How much of the gait/idle pose replaces the action's own legs: all of it for side-B,
    /// the walking blend for other mobile moves, none otherwise.
    pub fn gait_layer_weight(&self) -> f32 {
        if self.side_special_layered() {
            1.
        } else if self.mobile_action() && !self.spins_whole_body() {
            self.locomotion_weight
        } else {
            0.
        }
    }
    /// Melee's side-B cannon fires toward local +X in the extracted pose. Turn only its
    /// upper rig toward the live camera ray; locomotion and body yaw stay put.
    pub fn side_special_upper_aim(&self) -> Option<(f32, f32)> {
        self.behavior.side_special_upper_aim(self)
    }
    /// The trophy entry's colour flash (ft_0C31.c::ftCo_800BFFD0 col_anim 0x75):
    /// alternates the fighter's colour during the entry for the source duration.
    pub fn entry_flash(&self) -> Option<[f32; 4]> {
        let entry = self.entry.as_ref()?;
        if entry.frame < entry.params.grow + entry.params.color_frames
            && (entry.frame / 3) % 2 == 0
        {
            Some(entry.params.color)
        } else {
            None
        }
    }
    pub fn advance(&mut self, host: &mut impl Host, mut input: Input, dt: f64) -> u32 {
        // Explicit neutral-B keeps the existing charge hold/fire protocol while
        // remaining neutral even when LS is held or the camera points upward.
        if input.buttons & NEUTRAL_B != 0 {
            input.buttons |= SPECIAL;
        }
        self.action.samples.clear();
        self.action.sounds.clear();
        self.action.sounds.append(&mut self.queued_sounds);
        self.action.effects.clear();
        if !dt.is_finite() || dt < 0. || !input.movement.is_finite() || !input.aim.is_finite() {
            return 0;
        }
        self.pending_pressed |= input.buttons & !self.sampled_buttons;
        self.pending_released |= self.sampled_buttons & !input.buttons;
        self.sampled_buttons = input.buttons;
        self.accumulator += dt.min(0.25);
        let mut ticks = 0;
        while self.accumulator + 1e-10 >= 1. / TICK_HZ {
            self.accumulator = (self.accumulator - 1. / TICK_HZ).max(0.);
            let pressed = std::mem::take(&mut self.pending_pressed);
            self.released = std::mem::take(&mut self.pending_released);
            self.tick(host, input, pressed);
            ticks += 1;
        }
        ticks
    }
    /// Optional host-owned yaw (e.g. RoR2 CharacterDirection). It changes heading,
    /// never velocity, and cannot twist an authored dash/throw/spin action.
    pub fn set_host_facing(&mut self, facing: Vec3) {
        if facing.is_finite() && facing.with_y(0.).length_squared() > 0.001 {
            self.host_facing = Some(facing.with_y(0.).normalize());
        }
    }
    pub fn facing_locked(&self) -> bool {
        self.entry.is_some()
            || self.reaction.is_some()
            || self
                .rebirth
                .as_ref()
                .is_some_and(|state| state.flags() & 1 != 0)
            || matches!(
                self.current_move,
                Some(
                    Move::DashAttack
                        | Move::BackAir
                        | Move::Grab
                        | Move::DashGrab
                        | Move::AirGrab
                        | Move::Pummel
                        | Move::ThrowForward
                        | Move::ThrowBack
                        | Move::ThrowUp
                        | Move::ThrowDown
                        | Move::GrabHold
                )
            )
            || self
                .current_move
                .is_some_and(|m| self.behavior.facing_locked_now(self, m))
    }
    /// The host reports that the current move's hitbox connected (a staged
    /// special may react, like Skull Bash stopping on its victim).
    pub fn attack_connected(&mut self) {
        self.using_behavior(|b, s| b.on_connect(s));
    }
    pub fn teleport(&mut self, position: Vec3) {
        self.defense.mode = None;
        self.grabbed = false;
        self.suppress_initial_landing = true;
        self.action.reset();
        self.motion.select("Wait", 1.);
        self.motion.frame = 0.;
        self.locomotion.select("Wait", 1.);
        self.locomotion.frame = 0.;
        self.locomotion_weight = 0.;
        self.reaction = None;
        self.buffered_attack = 0;
        self.knockdown_grace = 0;
        self.entry = None;
        self.rebirth = None;
        self.pad_gravity = None;
        self.position = position;
        self.velocity = Vec3::ZERO;
        self.grounded = false;
        self.phase = Phase::Free;
        self.current_move = None;
        self.jumps_used = 1;
        self.up_special_invulnerable = false;
    }
    pub fn launch(&mut self, velocity: Vec3) {
        self.pad_gravity = None;
        self.velocity = velocity;
        self.grounded = false;
    }
    /// Host-only ballistic jump pad: preserve all momentum and host gravity until
    /// contact, rather than applying Melee's steering/terminal velocity.
    pub fn launch_pad(&mut self, velocity: Vec3, gravity: Vec3) {
        self.reaction = None;
        self.launch(velocity);
        self.pad_gravity = Some(gravity);
    }
    /// Host crowd-control interruption. Stored neutral charge survives.
    pub fn interrupt(&mut self) {
        self.defense.mode = None;
        self.grabbed = false;
        self.current_move = None;
        self.smash_charge = 0;
        self.up_special_invulnerable = false;
        self.special_fall = None;
        self.afterimage = -1;
        self.colanim.clear();
        self.jab = JabChain::default();
        self.set_phase(Phase::Free);
    }
    /// Host damage notification, deliberately independent of engine health types.
    pub fn hurt(&mut self) {
        self.hurt_scaled(0.);
    }
    /// A hit that cost this fraction of the fighter's health. The source plays a hurt voice when
    /// the knockback is large (ftCo_Damage.c compares it with two PlCo thresholds); the host has
    /// no knockback, so the damage fraction stands in (see `hurt_voice`).
    pub fn hurt_scaled(&mut self, severity: f32) {
        if self.defense_flags() & (2 | 64 | 128) != 0 {
            return;
        }
        if self.entry.is_some() || self.rebirth.is_some() {
            return;
        }
        self.interrupt();
        let stage = if self.grounded {
            crate::reaction::Stage::GroundHit
        } else {
            crate::reaction::Stage::AirHit
        };
        self.reaction = Some(stage);
        self.motion.select(stage.clip(), 1.);
        self.motion.frame = 0.;
        self.action.active.clear();
        self.hurt_voice(severity);
    }
    /// Keep the post-get-up window open while a host stun still owns control.
    /// It cannot grant protection to an unrelated crowd-control state.
    pub fn hold_knockdown_protection(&mut self) {
        if self.reaction.is_some() || self.knockdown_grace > 0 {
            self.knockdown_grace = 60;
        }
    }
    pub(crate) fn emit(&self, host: &mut dyn Host, kind: EventKind, move_id: u32, strength: f32) {
        let (attachment, attachment_offset) = if kind == EventKind::Hit {
            self.behavior
                .hit_attachment(self, self.current_move.unwrap_or(Move::Jab))
        } else {
            (None, Vec3::ZERO)
        };
        host.event(Event {
            kind,
            move_id,
            strength,
            position: self.position,
            direction: self.move_aim,
            facing: self.facing,
            clip: self.motion.clip,
            pose_frame: self.motion.frame,
            gait_clip: self.locomotion.clip,
            gait_frame: self.locomotion.frame,
            gait_weight: self.gait_layer_weight(),
            attachment,
            attachment_offset,
        });
    }
    pub(crate) fn set_phase(&mut self, phase: Phase) {
        if let Some(stop) = self.behavior.phase_stop(self.phase) {
            if self.phase != phase {
                self.action.sounds.push(stop);
            }
        }
        self.phase = phase;
        self.phase_frame = 0;
        if let Some(effect) = self.behavior.phase_effect(self, phase) {
            self.action.effects.push(effect);
        }
    }
    fn start(&mut self, host: &mut impl Host, m: Move, input: Input) {
        if input.allowed & (1 << self.behavior.slot(m)) == 0 {
            return;
        }
        // ftCo_Attack1.c checkAttack11 / doAttack12Normal: a first jab starts a fresh chain; the
        // later jabs keep the press count; the rapid jab keeps it too; anything else ends it.
        match m {
            Move::Jab => {
                self.jab = JabChain {
                    last: Some(m),
                    window: self.params.jab_2_window,
                    ..JabChain::default()
                }
            }
            Move::Jab2 | Move::Jab3 => {
                self.jab.last = Some(m);
                self.jab.window = if m == Move::Jab2 { self.params.jab_3_window } else { 0 };
                self.jab.queued = false;
            }
            Move::RapidJab => {
                self.jab.last = None;
                self.jab.window = 0;
                self.jab.looping = false;
                self.jab.fed = false;
            }
            _ => self.jab = JabChain::default(),
        }
        self.current_move = Some(m);
        self.stage = 0;
        self.flutter = 0;
        self.stage_frame = 0;
        self.stage_len = 0;
        self.stage_vec = Vec3::ZERO;
        self.stage_dir = Vec3::ZERO;
        self.stage_value = 0.;
        self.roll_break = false;
        self.roll_still = 0;
        if self.behavior.aimable(m)
            && self.host_facing.is_none()
            && input.aim.with_y(0.).length_squared() > 0.001
        {
            self.facing = input.aim.with_y(0.).normalize();
        }
        self.action.reset();
        self.motion.frame = 0.;
        self.move_started_grounded = self.grounded;
        self.up_special_invulnerable = self.behavior.is_up_special(m);
        if m == Move::BackAir
            && self.host_facing.is_none()
            && input.aim.with_y(0.).length_squared() > 0.01
        {
            self.facing = input.aim.with_y(0.).normalize();
        }
        self.smash_charge = 0;
        self.move_strength = 1.;
        self.move_aim = if input.aim.length_squared() > 0.01 {
            input.aim.normalize()
        } else {
            self.facing
        };
        self.emit(host, EventKind::Started, m as u32, 1.);
        self.set_phase(Phase::Startup);
        self.using_behavior(|b, s| b.enter_special(s, host, m, input));
    }
    /// The host owns enemy identity/collision; fighter capture/throw transitions
    /// and disc animation command timing remain engine-independent.
    pub fn set_grabbed(&mut self, grabbed: bool) {
        self.grabbed = grabbed;
        self.jab = JabChain::default();
        self.action.reset();
        self.velocity = Vec3::ZERO;
        self.current_move = if grabbed { Some(Move::GrabHold) } else { None };
        self.set_phase(if grabbed {
            Phase::Recovery
        } else {
            Phase::Free
        });
        self.motion
            .select(if grabbed { "CatchWait" } else { "Wait" }, 1.);
        self.motion.frame = 0.;
    }
    /// A host/test can request a named fighter move; the same slot gate and entry
    /// logic used by physical input apply. No engine-specific objects are needed.
    pub fn start_move(&mut self, host: &mut impl Host, m: Move, input: Input) -> bool {
        let serial = self.action.serial;
        self.start(host, m, input);
        self.action.serial != serial
    }
    fn choose_attack(&self, input: Input) -> Move {
        let vertical = input.attack_vertical();
        let up = vertical > 0;
        let down = vertical < 0;
        let magnitude = input.movement.with_y(0.).length();
        if !self.grounded {
            return if up {
                Move::UpAir
            } else if down {
                Move::DownAir
            } else if magnitude > 0.3 {
                if input.movement.with_y(0.).dot(input.aim.with_y(0.)) < -0.1 {
                    Move::BackAir
                } else {
                    Move::ForwardAir
                }
            } else {
                Move::NeutralAir
            };
        }
        let flick = if input.buttons & (UP | DOWN) != 0 {
            self.modifier_age < self.params.smash_window
        } else {
            magnitude >= self.params.smash_threshold && self.flick_age < self.params.smash_window
        };
        if flick {
            if up {
                Move::UpSmash
            } else if down {
                Move::DownSmash
            } else {
                Move::SideSmash
            }
        } else if up {
            Move::UpTilt
        } else if down {
            Move::DownTilt
        } else if input.buttons & SPRINT != 0 && magnitude > 0.3 {
            Move::DashAttack
        } else if magnitude > 0.3 {
            Move::SideTilt
        } else {
            Move::Jab
        }
    }
    fn jump(&mut self, host: &mut impl Host, aerial: bool, input: Input) {
        let stick = input.movement.with_y(0.).clamp_length_max(1.);
        let backwards = stick.dot(self.facing) < -0.3;
        self.motion.select(
            match (aerial, backwards) {
                (true, true) => "JumpAerialB",
                (true, false) => "JumpAerialF",
                (false, true) => "JumpB",
                _ => "JumpF",
            },
            1.,
        );
        // ftCo_Jump_Enter / ftCo_JumpAerial_Enter_Basic: ground momentum and
        // stick impulse are distinct; the aerial jump replaces horizontal speed.
        let horizontal = if aerial {
            stick * self.params.air_jump_horizontal * self.options.speed_scale
        } else {
            (self.velocity.with_y(0.) * self.params.jump_momentum
                + stick * self.params.jump_horizontal * self.options.speed_scale)
                .clamp_length_max(self.params.jump_horizontal_max * self.options.speed_scale)
        };
        self.velocity.x = horizontal.x;
        self.velocity.z = horizontal.z;
        let sound = if aerial {
            self.params.air_jump_sound
        } else {
            self.params.jump_sound
        };
        if sound != u32::MAX && sound != 0x83d60 {
            self.action.sounds.push(crate::action::SoundCue {
                sound,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
        }
        self.pad_gravity = None;
        self.grounded = false;
        self.velocity.y = if let Some(height) = self.host_jump_height {
            crate::controls::jump_velocity(height, self.params.gravity)
        } else {
            (if aerial {
                self.params.jump_speed * self.params.air_jump_scale
            } else if self.short_hop {
                self.params.short_hop_speed
            } else {
                self.params.jump_speed
            }) * self.options.jump_scale
        };
        self.jumps_used += 1;
        self.flutter = 0;
        if aerial {
            if let Some(flutter) = self.behavior.flutter() {
                self.flutter = flutter.frames;
                self.flutter_gravity = flutter.gravity;
                self.velocity.y *= flutter.boost;
            }
        }
        self.set_phase(Phase::Free);
        self.emit(host, EventKind::Jump, 0, self.jumps_used as f32);
    }
    fn tick(&mut self, host: &mut impl Host, input: Input, mut pressed: u32) {
        self.frame += 1;
        self.scope_refill(1. / 60.);
        if self.tick_rebirth(input) {
            self.previous = input;
            return;
        }
        if let Some(mut entry) = self.entry {
            if entry.waiting {
                self.current_move = None;
                self.velocity = Vec3::ZERO;
                self.action.active.clear();
                self.motion.select("Entry", 0.);
                self.motion.frame = 0.;
                entry.frame = (entry.frame + 1).min(entry.params.grow);
                self.entry = Some(entry);
                self.previous = input;
                return;
            }
            if !entry.started {
                entry.started = true;
                self.action.sounds.push(crate::action::SoundCue {
                    sound: 0x8b,
                    volume: 127,
                    pan: 64,
                    channel: 0,
                    stop: false,
                });
                self.action.effects.push(crate::action::GfxCue {
                    id: 0x43e,
                    bone: 0,
                    common: false,
                    cycle: 0,
                    destroy_on_change: false,
                    offset: Vec3::ZERO,
                    param: 1.,
                    clip: "Entry",
                    frame: 0.,
                    origin: self.position,
                    facing: self.facing,
                });
            }
            self.current_move = None;
            self.velocity = Vec3::ZERO;
            self.action.active.clear();
            self.motion.select("Entry", 0.);
            self.motion.frame = 0.;
            entry.frame += 1;
            self.entry = if entry.finished() {
                self.motion.select("Wait", 1.);
                None
            } else {
                Some(entry)
            };
            self.previous = input;
            return;
        }
        self.phase_frame = self.phase_frame.saturating_add(1);
        if self.tick_defense(host, input, pressed) {
            self.previous = input;
            return;
        }
        if let Some(stage) = self.reaction {
            if self.motion.finished() {
                self.reaction = stage.after();
                if self.reaction.is_none() {
                    // A full second after regaining control is long enough to
                    // start a buffered attack without another hit restarting stun.
                    self.knockdown_grace = 60;
                }
            }
        } else {
            self.knockdown_grace = self.knockdown_grace.saturating_sub(1);
        }
        // A short contextual-A press during hitstun should execute once control
        // returns. Keep the request bounded and retain the live aim/stick vector.
        if self.reaction.is_some() && pressed & ATTACK != 0 {
            self.buffered_attack = 90;
        } else if self.reaction.is_none()
            && self.phase == Phase::Free
            && self.buffered_attack > 0
            && input.allowed & (1 << 1) != 0
        {
            pressed |= ATTACK;
            self.buffered_attack = 0;
        }
        self.buffered_attack = self.buffered_attack.saturating_sub(1);
        self.flick_age = self.flick_age.saturating_add(1);
        self.modifier_age = self.modifier_age.saturating_add(1);
        let movement = input.movement.with_y(0.).clamp_length_max(1.);
        let old = self.previous.movement.with_y(0.);
        if movement.length() >= self.params.smash_threshold
            && (old.length() < self.params.smash_threshold
                || movement.normalize().dot(old.normalize_or_zero()) < 0.5)
        {
            self.flick_age = 0;
        }
        // A camera band change while LS is held is also a new direction flick.
        if input.buttons & (UP | DOWN) == 0
            && input.attack_vertical() != self.previous.attack_vertical()
        {
            self.flick_age = 0;
        }
        if pressed & (UP | DOWN) != 0 {
            self.modifier_age = 0;
        }
        if !self.facing_locked() {
            if let Some(facing) = self.host_facing {
                self.facing = facing;
            } else if self.phase == Phase::Free && self.grounded && movement.length_squared() > 0.01
            {
                self.facing = movement.normalize();
            }
        }
        if self
            .current_move
            .is_some_and(|m| self.behavior.aimable(m) && !self.behavior.aim_frozen(self))
            && input.aim.length_squared() > 0.001
        {
            self.move_aim = input.aim.normalize();
            if input.aim.with_y(0.).length_squared() > 0.001 {
                self.facing = self
                    .host_facing
                    .unwrap_or_else(|| input.aim.with_y(0.).normalize());
            }
        }
        let was_mobile = self.mobile_action();
        // RoR2 mobile neutral-B adaptation. Sprint is the explicit store/cancel
        // gesture while walking is allowed. The source charge amount survives.
        // ftSamus_SpecialNHold_IASA is the reference for storing charge; sprint
        // is a host control decision, not a claim about Melee's original input.
        self.using_behavior(|b, s| {
            b.store_special(s, host, pressed);
            b.tick_special(s, host, input, pressed);
        });
        match self.phase {
            Phase::Free => {
                if self.reaction.is_some()
                    || self.motion.clip.starts_with("Landing") && !self.motion.finished()
                {
                    // Damage/down and authored landing clips finish before control.
                } else if pressed & JUMP != 0 {
                    if self.grounded {
                        self.short_hop = false;
                        self.set_phase(Phase::JumpSquat);
                    } else if self.jumps_used < self.params.jumps + self.options.extra_jumps {
                        self.jump(host, true, input);
                    }
                } else if pressed & GRAB != 0 {
                    let m = if !self.grounded {
                        Move::AirGrab
                    } else if input.buttons & SPRINT != 0 {
                        Move::DashGrab
                    } else {
                        Move::Grab
                    };
                    self.start(host, m, input);
                } else if pressed & (SPECIAL | NEUTRAL_B | SIDE_B | UP_B | DOWN_B) != 0 {
                    if let Some(m) = self.behavior.select_special(self, input, movement) {
                        self.start(host, m, input);
                    }
                } else if pressed & ATTACK != 0 {
                    // Attack1_CheckInput: a press inside the window after a jab ends, with the
                    // combo opened, goes on to the next jab (a smash or tilt input does not).
                    let attack = self.choose_attack(input);
                    let next = if attack == Move::Jab { self.jab_resume() } else { None };
                    self.start(host, next.unwrap_or(attack), input);
                }
            }
            Phase::JumpSquat => {
                if input.buttons & JUMP == 0 {
                    self.short_hop = true;
                }
                if self.phase_frame >= self.params.jump_squat {
                    self.jump(host, false, input);
                }
            }
            Phase::Startup => {
                let m = self.current_move.unwrap();
                if !self.action.definitions.is_empty() {
                    self.update_disc_action(host, input, pressed);
                } else if self.phase_frame >= self.behavior.startup(m).unwrap_or_else(|| startup(m)) {
                    if m.is_smash() && input.buttons & ATTACK != 0 {
                        self.set_phase(Phase::SmashCharge);
                    } else if self.using_behavior(|b, s| b.resolve_startup(s, input)) {
                        // The special entered its hold phase.
                    } else {
                        self.fire(host);
                    }
                }
            }
            Phase::SmashCharge => {
                self.smash_charge = self
                    .smash_charge
                    .saturating_add(1)
                    .min(self.options.smash_max_frames);
                if input.buttons & ATTACK == 0 || self.smash_charge >= self.options.smash_max_frames
                {
                    if self.action.definitions.is_empty() {
                        self.move_strength = 1.
                            + 0.4 * self.smash_charge as f32
                                / self.options.smash_max_frames.max(1) as f32;
                        self.fire(host);
                    } else {
                        self.move_strength = 1.
                            + (self.action.smash_multiplier - 1.).max(0.)
                                * self.smash_charge as f32
                                / self.action.smash_max_frames.max(1) as f32;
                        self.set_phase(Phase::Startup);
                    }
                }
            }
            Phase::ChargeShot => self.using_behavior(|b, s| b.tick_charge(s, host, input, pressed)),
            Phase::Recovery => {
                if !self.action.definitions.is_empty() {
                    self.update_disc_action(host, input, pressed);
                } else if self.phase_frame
                    >= self.behavior.recovery(self.current_move.unwrap()).unwrap_or_else(|| recovery(self.current_move.unwrap()))
                {
                    // The up special no longer enters helpless fall: the host lets
                    // the fighter act airborne, and the i-frames survive until
                    // landing or another attack.
                    self.set_phase(Phase::Free);
                    self.current_move = None;
                }
            }
            Phase::Helpless => {
                if self.grounded {
                    self.set_phase(Phase::Free);
                }
            }
            Phase::ChargeCancel => {
                self.using_behavior(|b, s| b.tick_charge_cancel(s));
            }
        }
        self.jab.window = self.jab.window.saturating_sub(1);
        let movable = self.reaction.is_none()
            && (matches!(self.phase, Phase::Free | Phase::JumpSquat | Phase::Helpless)
                || self.mobile_action())
            || !self.grounded;
        if let Some(gravity) = self.pad_gravity {
            self.velocity += gravity;
        } else {
            let horizontal = self.velocity.with_y(0.);
            // A helpless special fall drifts at a fraction of the usual air speed.
            let mobility = if self.phase == Phase::Helpless {
                self.special_fall.map_or(1., |f| f.mobility)
            } else {
                1.
            };
            let (speed, friction) = if self.grounded {
                if input.buttons & SPRINT != 0 && !self.mobile_action() {
                    (self.params.run_speed, self.params.friction)
                } else {
                    (self.params.walk_speed, self.params.friction)
                }
            } else {
                (self.params.air_speed, self.params.air_friction)
            };
            let target = if movable {
                movement * speed * self.options.speed_scale * mobility
            } else {
                Vec3::ZERO
            };
            let difference = target - horizontal;
            let step = if movable && target.length_squared() > 0.001 {
                // The source scales the stick component, then adds base accel.
                let (mul, base) = if self.grounded {
                    if input.buttons & SPRINT != 0 && !self.mobile_action() {
                        (self.params.dash_accel, self.params.dash_base)
                    } else {
                        (self.params.walk_accel, self.params.walk_base)
                    }
                } else {
                    (self.params.air_accel, self.params.air_base)
                };
                (mul * movement.length() + base) * self.options.speed_scale
            } else {
                friction
            };
            let horizontal = if self.grounded {
                horizontal + difference.clamp_length_max(step.max(0.))
            } else {
                // ftCommon_CalcSelfAccel_DriftFrom / AccelToVelClampedFrom.
                // Apply the source's signed drift independently on the host's
                // horizontal axes. Neutral input decelerates by air friction.
                let drift = |vel: f32, stick: f32| {
                    let scale = self.options.speed_scale;
                    let target = stick * self.params.air_speed * scale * mobility;
                    let friction = self.params.air_friction * scale;
                    if target.abs() < 0.00001 {
                        return vel - sign_deceleration(vel, friction);
                    }
                    let accel = (stick * self.params.air_accel
                        + stick.signum() * self.params.air_base)
                        * scale;
                    if vel * accel < 0. {
                        return vel + accel;
                    }
                    if (accel > 0. && vel + accel > target) || (accel < 0. && vel + accel < target)
                    {
                        let next = if accel > 0. {
                            (vel - friction).max(target)
                        } else {
                            (vel + friction).min(target)
                        };
                        next.clamp(
                            -self.params.air_horizontal_max * scale,
                            self.params.air_horizontal_max * scale,
                        )
                    } else {
                        vel + accel
                    }
                };
                let stick = if movable { movement } else { Vec3::ZERO };
                Vec3::new(
                    drift(horizontal.x, stick.x),
                    0.,
                    drift(horizontal.z, stick.z),
                )
            };
            self.velocity.x = horizontal.x;
            self.velocity.z = horizontal.z;
            if !self.grounded {
                let cap = if input.buttons & DOWN != 0 && self.velocity.y <= 0. {
                    self.params.fast_fall
                } else {
                    self.params.fall_speed
                };
                let gravity = if self.flutter > 0 {
                    self.flutter -= 1;
                    self.params.gravity * self.flutter_gravity
                } else {
                    self.params.gravity
                };
                self.velocity.y = (self.velocity.y - gravity).max(-cap);
            } else {
                self.flutter = 0;
                self.velocity.y = self.velocity.y.min(0.);
            }
        }
        // ftCo_AttackDash_Phys -> ft_80085030 uses the flagged TransN delta as
        // ground velocity. It replaces friction velocity; never add a second
        // visual dash or teleport at clip completion. Collision owns the result.
        // Any grounded move whose clip carries flagged TransN travel commits it
        // to the sim position (dash attack, dash grab): the model never adds a
        // second copy, so the fighter keeps the ground it covered at clip end.
        if self.grounded && matches!(self.current_move, Some(Move::DashAttack | Move::DashGrab)) {
            let (clip, rate) = crate::motion::select(self, input);
            let frame = if clip == self.motion.clip {
                self.motion.frame
            } else {
                0.
            };
            if let Some(distance) = self
                .motion
                .definitions
                .get(clip)
                .and_then(|d| d.forward_delta(frame, rate))
            {
                let velocity = self.facing * distance * self.options.speed_scale;
                self.velocity.x = velocity.x;
                self.velocity.z = velocity.z;
            }
        }
        // Initial placement/warp is already a standing host placement. Its
        // first support probe must not invent landing lag or fall damage.
        let before = self.grounded || self.suppress_initial_landing;
        self.suppress_initial_landing = false;
        let contact = host.collide(self.position, self.position + self.velocity);
        self.position = contact.position;
        self.grounded = contact.grounded && self.velocity.y <= 0.;
        if contact.ceiling {
            self.velocity.y = self.velocity.y.min(0.);
        }
        if self.grounded {
            self.pad_gravity = None;
            // The up special's i-frames end on touchdown.
            self.up_special_invulnerable = false;
            if !before {
                if matches!(
                    self.reaction,
                    Some(crate::reaction::Stage::AirHit | crate::reaction::Stage::Fall)
                ) {
                    self.reaction = Some(crate::reaction::Stage::Bound);
                    self.motion.select("DownBoundU", 1.);
                } else {
                    let helpless_landing = self.phase == Phase::Helpless;
                    let landing = match self.current_move {
                        Some(Move::NeutralAir) => "LandingAirN",
                        Some(Move::ForwardAir) => "LandingAirF",
                        Some(Move::BackAir) => "LandingAirB",
                        Some(Move::UpAir) => "LandingAirHi",
                        Some(Move::DownAir) => "LandingAirLw",
                        Some(m) => self.behavior.landing(self, m),
                        None => {
                            if self.phase == Phase::Helpless {
                                "LandingFallSpecial"
                            } else {
                                "Landing"
                            }
                        }
                    };
                    if landing != "Landing" {
                        self.current_move = None;
                        self.action.active.clear();
                        self.set_phase(Phase::Free);
                    }
                    // A special fall's landing lasts its landing lag (ftCo_LandingFallSpecial).
                    let mut rate = 1.;
                    if helpless_landing {
                        if let Some(fall) = self.special_fall.take() {
                            if fall.landing_lag > 0. {
                                rate = self.motion.length(landing) / fall.landing_lag;
                            }
                        }
                    }
                    self.landing_rate = rate;
                    self.motion.select(landing, rate);
                }
                self.emit(host, EventKind::Land, 0, -self.velocity.y);
            }
            self.velocity.y = 0.;
            self.jumps_used = 0;
            self.air_charge = false;
            if self.phase == Phase::Helpless {
                self.set_phase(Phase::Free);
            }
        }
        let (clip, rate) = crate::motion::select(self, input);
        if clip.starts_with("Fall") {
            if !self.motion.clip.starts_with("Fall") {
                self.fall_blend = 0.;
            }
            let drift = (self.velocity.with_y(0.).dot(self.facing)
                / (self.params.air_speed * self.options.speed_scale).max(0.001))
            .abs()
            .min(1.);
            let target = ((drift - self.params.fall_pose_threshold)
                / (1. - self.params.fall_pose_threshold))
                .max(0.);
            self.fall_blend += self.params.fall_blend_response * (target - self.fall_blend);
        } else {
            self.fall_blend = 0.;
        }
        let (gait, gait_rate) = crate::motion::locomotion(self, input);
        if self.locomotion.definitions.is_empty() && !self.motion.definitions.is_empty() {
            self.locomotion.definitions = self.motion.definitions.clone();
        }
        self.locomotion.select(gait, gait_rate);
        self.locomotion.advance();
        self.motion.select(clip, rate);
        self.motion.advance();
        if was_mobile && !self.mobile_action() && clip == self.locomotion.clip {
            // Handoff to ordinary locomotion without restarting the foot cycle.
            self.motion.frame = self.locomotion.frame;
        }
        let target = if self.mobile_action() && self.velocity.with_y(0.).length() > 0.05 {
            1.
        } else {
            0.
        };
        self.locomotion_weight += (target - self.locomotion_weight).clamp(-0.25, 0.25);
        if !self.action.definitions.is_empty() {
            self.execute_disc_commands(host, input);
            self.using_behavior(|b, s| b.after_script(s));
        }
        if self.behavior.hurt_eyes() {
            // Part animation 3 while in knockback (reapplied after each motion change clears
            // it), 2 again on leaving it.
            let hurt = self.reaction.is_some();
            for slot in [3, 4] {
                if hurt && self.parts[slot].is_none() {
                    self.parts[slot] = Some(PartPlay { index: 3, age: 0., blend: 0 });
                } else if !hurt && self.knockback_eyes {
                    self.parts[slot] = Some(PartPlay { index: 2, age: 0., blend: 0 });
                }
            }
            self.knockback_eyes = hurt;
        }
        self.tick_colanim();
        for part in self.parts.iter_mut().flatten() {
            part.age += 1.;
        }
        self.previous = input;
    }
    pub(crate) fn fire(&mut self, host: &mut dyn Host) {
        let m = self.current_move.unwrap();
        if !self.action.definitions.is_empty() {
            self.set_phase(Phase::Recovery);
            self.motion.frame = 0.;
            return;
        }
        let strength = self.using_behavior(|b, s| b.fire_strength(s, m));
        self.emit(host, EventKind::Hit, m as u32, strength);
        self.set_phase(Phase::Recovery);
    }
}

fn sign_deceleration(velocity: f32, friction: f32) -> f32 {
    velocity.signum() * velocity.abs().min(friction)
}

// Prototype schedules, NOT purported frame-perfect Melee command data. The
// special moves override these through the fighter's `Behavior`.
fn startup(m: Move) -> u32 {
    match m {
        Move::Jab => 3,
        Move::SideSmash => 10,
        Move::UpSmash => 10,
        Move::DownSmash => 6,
        _ => 6,
    }
}
fn recovery(m: Move) -> u32 {
    match m {
        Move::Jab => 15,
        _ => 24,
    }
}

/// Source side-B's final cannon pose points along local +X, while the fighter's
/// forward is +Z. Turn that firing pose to camera aim without pitching the spine.
/// The host mirrors model X, so model yaw opposes the world-space heading delta.
pub fn side_special_upper_angles(aim: Vec3, facing: Vec3) -> (f32, f32) {
    let horizontal = aim.with_y(0.).normalize_or_zero();
    let body = facing.with_y(0.).normalize_or_zero();
    let yaw = if horizontal.length_squared() > 0. && body.length_squared() > 0. {
        (body.z * horizontal.x - body.x * horizontal.z).atan2(body.dot(horizontal))
    } else {
        0.
    };
    let elevation = aim.y.atan2(aim.with_y(0.).length());
    (elevation, std::f32::consts::FRAC_PI_2 - yaw)
}

#[derive(Default)]
pub struct FlatHost {
    pub events: Vec<Event>,
}
impl Host for FlatHost {
    fn collide(&mut self, _from: Vec3, to: Vec3) -> Contact {
        Contact {
            position: to.with_y(to.y.max(0.)),
            grounded: to.y <= 0.,
            ceiling: false,
        }
    }
    fn event(&mut self, e: Event) {
        self.events.push(e);
    }
}
