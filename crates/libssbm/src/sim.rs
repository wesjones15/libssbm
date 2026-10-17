//! Behavioral references: ftCommon/{ftCo_KneeBend,ftCo_Jump,ftCo_JumpAerial,
//! ftCo_AttackS4}.c and ftSamus/ftsamusspecial{n,s,hi,lw1}.c.
//! Move startup/active/recovery schedules here are prototype options, pending the
//! animation command reader. They are deliberately separate from disc parameters.
use crate::Params;
use glam::Vec3;
mod actions;

pub const TICK_HZ: f64 = 60.;
pub const JUMP: u32 = 1;
pub const ATTACK: u32 = 2;
pub const SPECIAL: u32 = 4;
pub const UP: u32 = 8;
pub const DOWN: u32 = 16;
pub const SPRINT: u32 = 32;
pub const CANCEL: u32 = 64;

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
    pub fn idle() -> Self {
        Self {
            aim: Vec3::X,
            allowed: 15,
            ..Self::default()
        }
    }
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
    ChargeShot = 20,
    Missile = 21,
    SuperMissile = 22,
    ScrewAttack = 23,
    Bomb = 24,
}
impl Move {
    pub fn slot(self) -> u32 {
        match self {
            Self::ChargeShot => 1,
            Self::Missile | Self::SuperMissile | Self::Bomb => 2,
            Self::ScrewAttack => 3,
            _ => 0,
        }
    }
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

pub struct Sim {
    pub reaction: Option<crate::reaction::Stage>,
    pub entry: Option<crate::entry::State>,
    pub action: crate::action::ActionState,
    pub motion: crate::motion::MotionState,
    pub move_started_grounded: bool,
    pub params: Params,
    pub options: Options,
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
    accumulator: f64,
    pub fall_blend: f32,
    suppress_initial_landing: bool,
    sampled_buttons: u32,
    pending_pressed: u32,
    previous: Input,
    flick_age: u32,
    modifier_age: u32,
    charge_clock: u32,
    short_hop: bool,
    release_phase: bool,
    pad_gravity: Option<Vec3>,
    move_aim: Vec3,
    move_strength: f32,
}
impl Sim {
    pub fn new(params: Params, position: Vec3) -> Self {
        Self {
            reaction: None,
            entry: None,
            action: crate::action::ActionState::default(),
            motion: crate::motion::MotionState::default(),
            move_started_grounded: false,
            params,
            options: Options::default(),
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
            accumulator: 0.,
            fall_blend: 0.,
            suppress_initial_landing: true,
            sampled_buttons: 0,
            pending_pressed: 0,
            previous: Input::idle(),
            flick_age: 100,
            modifier_age: 100,
            charge_clock: 0,
            short_hop: false,
            release_phase: false,
            pad_gravity: None,
            move_aim: Vec3::X,
            move_strength: 1.,
        }
    }
    pub fn alpha(&self) -> f32 {
        (self.accumulator * TICK_HZ) as f32
    }
    pub fn advance(&mut self, host: &mut impl Host, input: Input, dt: f64) -> u32 {
        self.action.samples.clear();
        self.action.sounds.clear();
        self.action.effects.clear();
        if !dt.is_finite() || dt < 0. || !input.movement.is_finite() || !input.aim.is_finite() {
            return 0;
        }
        self.pending_pressed |= input.buttons & !self.sampled_buttons;
        self.sampled_buttons = input.buttons;
        self.accumulator += dt.min(0.25);
        let mut ticks = 0;
        while self.accumulator + 1e-10 >= 1. / TICK_HZ {
            self.accumulator = (self.accumulator - 1. / TICK_HZ).max(0.);
            let pressed = std::mem::take(&mut self.pending_pressed);
            self.tick(host, input, pressed);
            ticks += 1;
        }
        ticks
    }
    pub fn teleport(&mut self, position: Vec3) {
        self.suppress_initial_landing = true;
        self.action.reset();
        self.motion.select("Wait", 1.);
        self.motion.frame = 0.;
        self.reaction = None;
        self.entry = None;
        self.pad_gravity = None;
        self.position = position;
        self.velocity = Vec3::ZERO;
        self.grounded = false;
        self.phase = Phase::Free;
        self.current_move = None;
        self.jumps_used = 1;
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
        self.current_move = None;
        self.smash_charge = 0;
        self.release_phase = false;
        self.set_phase(Phase::Free);
    }
    /// Host damage notification, deliberately independent of engine health types.
    pub fn hurt(&mut self) {
        if self.entry.is_some() {
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
    }
    fn emit(&self, host: &mut impl Host, kind: EventKind, move_id: u32, strength: f32) {
        let attachment = if kind == EventKind::Hit {
            match move_id {
                20 => Some(51),
                21 | 22 => Some(56),
                24 => Some(0),
                _ => None,
            }
        } else {
            None
        };
        let attachment_offset = if kind == EventKind::Hit {
            match move_id {
                21 | 22 => self.facing * self.params.missile_spawn_offset,
                24 => {
                    self.facing * self.params.bomb_spawn_offset[0]
                        + Vec3::Y * self.params.bomb_spawn_offset[1]
                        + Vec3::new(-self.facing.z, 0., self.facing.x)
                            * self.params.bomb_spawn_offset[2]
                }
                _ => Vec3::ZERO,
            }
        } else {
            Vec3::ZERO
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
            attachment,
            attachment_offset,
        });
    }
    fn set_phase(&mut self, phase: Phase) {
        if self.phase == Phase::ChargeShot && phase != Phase::ChargeShot {
            self.action.sounds.push(crate::action::SoundCue {
                sound: 0,
                volume: 127,
                pan: 64,
                channel: 6,
                stop: true,
            });
        }
        self.phase = phase;
        self.phase_frame = 0;
        if phase == Phase::ChargeShot {
            self.action.effects.push(crate::action::GfxCue {
                id: 1151,
                bone: 50,
                common: false,
                cycle: 0,
                destroy_on_change: true,
                offset: Vec3::new(0., 0., 4.),
                param: 0.,
                clip: "SpecialNHold",
                frame: 0.,
                origin: self.position,
                facing: self.facing,
            });
        }
    }
    fn start(&mut self, host: &mut impl Host, m: Move, input: Input) {
        if input.allowed & (1 << m.slot()) == 0 {
            return;
        }
        self.current_move = Some(m);
        self.action.reset();
        self.motion.frame = 0.;
        self.move_started_grounded = self.grounded;
        self.release_phase = m == Move::ScrewAttack;
        if m == Move::BackAir && input.aim.with_y(0.).length_squared() > 0.01 {
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
        if m == Move::ScrewAttack {
            self.action.effects.push(crate::action::GfxCue {
                id: 1154,
                bone: 3,
                common: false,
                cycle: 0,
                destroy_on_change: true,
                offset: Vec3::ZERO,
                param: 0.,
                clip: if self.move_started_grounded {
                    "SpecialHi"
                } else {
                    "SpecialAirHi"
                },
                frame: 0.,
                origin: self.position,
                facing: self.facing,
            });
            self.pad_gravity = None;
            self.grounded = false;
            self.velocity.y = self.params.screw_velocity * self.options.jump_scale;
            self.jumps_used = self.params.jumps + self.options.extra_jumps;
        } else if m == Move::Bomb && !self.grounded {
            self.pad_gravity = None;
            self.velocity.y = self.params.bomb_velocity;
        }
    }
    /// A host/test can request a named fighter move; the same slot gate and entry
    /// logic used by physical input apply. No engine-specific objects are needed.
    pub fn start_move(&mut self, host: &mut impl Host, m: Move, input: Input) -> bool {
        let serial = self.action.serial;
        self.start(host, m, input);
        self.action.serial != serial
    }
    fn choose_attack(&self, input: Input) -> Move {
        let up = input.buttons & UP != 0;
        let down = input.buttons & DOWN != 0;
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
        let flick = if up || down {
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
        self.velocity.y = if aerial {
            self.params.jump_speed * self.params.air_jump_scale
        } else if self.short_hop {
            self.params.short_hop_speed
        } else {
            self.params.jump_speed
        } * self.options.jump_scale;
        self.jumps_used += 1;
        self.set_phase(Phase::Free);
        self.emit(host, EventKind::Jump, 0, self.jumps_used as f32);
    }
    fn tick(&mut self, host: &mut impl Host, input: Input, pressed: u32) {
        self.frame += 1;
        if let Some(mut entry) = self.entry {
            if entry.frame == 0 {
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
        if let Some(stage) = self.reaction {
            if self.motion.finished() {
                self.reaction = stage.after();
            }
        }
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
        if pressed & (UP | DOWN) != 0 {
            self.modifier_age = 0;
        }
        if self.grounded
            && self.reaction.is_none()
            && self.phase == Phase::Free
            && movement.length_squared() > 0.01
        {
            self.facing = movement.normalize();
        }
        let phase = self.phase;
        match phase {
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
                } else if pressed & SPECIAL != 0 {
                    let m = if input.buttons & UP != 0 {
                        Move::ScrewAttack
                    } else if input.buttons & DOWN != 0 {
                        Move::Bomb
                    } else if movement.length() > 0.3 {
                        if self.flick_age < self.params.missile_smash_window {
                            Move::SuperMissile
                        } else {
                            Move::Missile
                        }
                    } else {
                        Move::ChargeShot
                    };
                    self.start(host, m, input);
                } else if pressed & ATTACK != 0 {
                    self.start(host, self.choose_attack(input), input);
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
                } else if self.phase_frame >= startup(m) {
                    if m.is_smash() && input.buttons & ATTACK != 0 {
                        self.set_phase(Phase::SmashCharge);
                    } else if m == Move::ChargeShot
                        && self.grounded
                        && self.shot_charge < self.params.charge_steps
                        && !(self.options.hold_charge && input.buttons & SPECIAL == 0)
                    {
                        self.charge_clock = 0;
                        self.set_phase(Phase::ChargeShot);
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
            Phase::ChargeShot => {
                self.charge_clock += 1;
                if self.charge_clock >= self.params.charge_interval {
                    self.charge_clock = 0;
                    self.shot_charge = (self.shot_charge + 1).min(self.params.charge_steps);
                }
                if pressed & (CANCEL | JUMP) != 0 || self.shot_charge >= self.params.charge_steps {
                    // SpecialNHold_Anim starts colour animation 53 at maximum,
                    // not the firing voice. The stored-charge presenter owns it.
                    self.emit(
                        host,
                        EventKind::ChargeStored,
                        Move::ChargeShot as u32,
                        self.shot_charge as f32,
                    );
                    self.set_phase(if self.action.definitions.is_empty() {
                        Phase::Free
                    } else {
                        Phase::ChargeCancel
                    });
                    self.current_move = None;
                    if pressed & JUMP != 0 && self.grounded {
                        self.short_hop = false;
                        self.set_phase(Phase::JumpSquat);
                    }
                } else if (self.options.hold_charge && input.buttons & SPECIAL == 0)
                    || (!self.options.hold_charge && pressed & SPECIAL != 0)
                {
                    self.fire(host);
                }
            }
            Phase::Recovery => {
                if !self.action.definitions.is_empty() {
                    self.update_disc_action(host, input, pressed);
                } else if self.phase_frame >= recovery(self.current_move.unwrap()) {
                    self.set_phase(if self.release_phase && !self.grounded {
                        Phase::Helpless
                    } else {
                        Phase::Free
                    });
                    self.current_move = None;
                }
            }
            Phase::Helpless => {
                if self.grounded {
                    self.set_phase(Phase::Free);
                }
            }
            Phase::ChargeCancel => {
                if self.motion.finished() {
                    self.set_phase(Phase::Free);
                    self.current_move = None;
                }
            }
        }
        let movable = self.reaction.is_none()
            && matches!(self.phase, Phase::Free | Phase::JumpSquat | Phase::Helpless)
            || !self.grounded;
        if let Some(gravity) = self.pad_gravity {
            self.velocity += gravity;
        } else {
            let horizontal = self.velocity.with_y(0.);
            let (speed, friction) = if self.grounded {
                if input.buttons & SPRINT != 0 {
                    (self.params.run_speed, self.params.friction)
                } else {
                    (self.params.walk_speed, self.params.friction)
                }
            } else {
                (self.params.air_speed, self.params.air_friction)
            };
            let target = if movable {
                movement * speed * self.options.speed_scale
            } else {
                Vec3::ZERO
            };
            let difference = target - horizontal;
            let step = if movable && target.length_squared() > 0.001 {
                // The source scales the stick component, then adds base accel.
                let (mul, base) = if self.grounded {
                    if input.buttons & SPRINT != 0 {
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
                    let target = stick * self.params.air_speed * scale;
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
                self.velocity.y = (self.velocity.y - self.params.gravity).max(-cap);
            } else {
                self.velocity.y = self.velocity.y.min(0.);
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
            if !before {
                if matches!(
                    self.reaction,
                    Some(crate::reaction::Stage::AirHit | crate::reaction::Stage::Fall)
                ) {
                    self.reaction = Some(crate::reaction::Stage::Bound);
                    self.motion.select("DownBoundU", 1.);
                } else {
                    let landing = match self.current_move {
                        Some(Move::NeutralAir) => "LandingAirN",
                        Some(Move::ForwardAir) => "LandingAirF",
                        Some(Move::BackAir) => "LandingAirB",
                        Some(Move::UpAir) => "LandingAirHi",
                        Some(Move::DownAir) => "LandingAirLw",
                        Some(Move::ScrewAttack) => "LandingFallSpecial",
                        _ => {
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
                    self.motion.select(landing, 1.);
                }
                self.emit(host, EventKind::Land, 0, -self.velocity.y);
            }
            self.velocity.y = 0.;
            self.jumps_used = 0;
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
        self.motion.select(clip, rate);
        self.motion.advance();
        if !self.action.definitions.is_empty() {
            self.execute_disc_commands(host, input);
        }
        self.previous = input;
    }
    fn fire(&mut self, host: &mut impl Host) {
        let m = self.current_move.unwrap();
        if !self.action.definitions.is_empty() {
            self.set_phase(Phase::Recovery);
            self.motion.frame = 0.;
            self.release_phase = m == Move::ScrewAttack;
            return;
        }
        let strength = if m == Move::ChargeShot {
            let ratio = self.shot_charge as f32 / self.params.charge_steps as f32;
            self.shot_charge = 0;
            ratio
        } else {
            self.move_strength
        };
        self.emit(host, EventKind::Hit, m as u32, strength);
        self.release_phase = m == Move::ScrewAttack;
        self.set_phase(Phase::Recovery);
    }
}

fn sign_deceleration(velocity: f32, friction: f32) -> f32 {
    velocity.signum() * velocity.abs().min(friction)
}

// Prototype schedules, NOT purported frame-perfect Melee command data.
fn startup(m: Move) -> u32 {
    match m {
        Move::Jab => 3,
        Move::SideSmash => 10,
        Move::UpSmash => 10,
        Move::DownSmash => 6,
        Move::ChargeShot => 12,
        Move::Missile | Move::SuperMissile => 12,
        Move::Bomb => 10,
        Move::ScrewAttack => 3,
        _ => 6,
    }
}
fn recovery(m: Move) -> u32 {
    match m {
        Move::Jab => 15,
        Move::ChargeShot => 24,
        Move::Missile | Move::SuperMissile => 24,
        Move::Bomb => 25,
        Move::ScrewAttack => 36,
        _ => 24,
    }
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
