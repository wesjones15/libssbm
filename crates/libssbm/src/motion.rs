//! Animation state advances on fighter ticks, independently of renderer calls.
//! ftwalkcommon::{800DFDDC,800DFEC8}, ftCo_Run_Anim, ftCo_Jump_Anim and
//! ftCo_JumpAerial_Anim specify speed scaling, gait phase and clip completion.
use crate::{DOWN, Input, Move, Phase, SPRINT, Sim};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct MotionDef {
    pub frames: f32,
    pub looping: bool,
    /// Source TransN forward positions at 60 Hz, when motion extraction is enabled.
    pub root_forward: Option<Vec<f32>>,
    /// Source TransN vertical positions at 60 Hz (the lift of Luigi's Super Jump Punch).
    pub root_up: Option<Vec<f32>>,
}
impl MotionDef {
    pub fn forward_delta(&self, frame: f32, rate: f32) -> Option<f32> {
        self.delta(self.root_forward.as_ref()?, frame, rate)
    }
    /// How far the clip's root motion lifts the fighter between `frame` and `frame + rate`.
    pub fn up_delta(&self, frame: f32, rate: f32) -> Option<f32> {
        self.delta(self.root_up.as_ref()?, frame, rate)
    }
    fn delta(&self, samples: &[f32], frame: f32, rate: f32) -> Option<f32> {
        if samples.is_empty() {
            return Some(0.);
        }
        let sample = |frame: f32| {
            let frame = frame.clamp(0., self.frames);
            let index = (frame.floor() as usize).min(samples.len() - 1);
            let next = (index + 1).min(samples.len() - 1);
            samples[index] + (samples[next] - samples[index]) * frame.fract()
        };
        Some(sample(frame + rate) - sample(frame))
    }
}
#[derive(Clone, Debug)]
pub struct MotionState {
    pub clip: &'static str,
    pub frame: f32,
    pub rate: f32,
    pub serial: u64,
    pub definitions: BTreeMap<String, MotionDef>,
}
impl Default for MotionState {
    fn default() -> Self {
        Self {
            clip: "Wait",
            frame: 0.,
            rate: 1.,
            serial: 0,
            definitions: BTreeMap::new(),
        }
    }
}
impl MotionState {
    pub fn length(&self, clip: &str) -> f32 {
        self.definitions.get(clip).map_or(1., |d| d.frames.max(1.))
    }
    pub fn looping(&self) -> bool {
        self.definitions.get(self.clip).is_some_and(|d| d.looping)
    }
    pub fn finished(&self) -> bool {
        self.frame >= self.length(self.clip)
    }
    pub fn select(&mut self, clip: &'static str, rate: f32) {
        if clip != self.clip {
            let gait = |s: &str| s.starts_with("Walk") || s == "Run";
            self.frame = if gait(self.clip) && gait(clip) {
                (self.frame / self.length(self.clip)).fract() * self.length(clip)
            } else if self.clip.starts_with("Fall") && clip.starts_with("Fall") {
                self.frame.min(self.length(clip))
            } else {
                0.
            };
            self.clip = clip;
            self.serial = self.serial.wrapping_add(1);
        }
        self.rate = rate.max(0.);
    }
    pub fn advance(&mut self) {
        self.frame += self.rate;
        if self.looping() {
            self.frame = self.frame.rem_euclid(self.length(self.clip));
        } else {
            self.frame = self.frame.min(self.length(self.clip));
        }
    }
    pub fn render_frame(&self, alpha: f32) -> f32 {
        let value = self.frame + alpha * self.rate;
        if self.looping() {
            value.rem_euclid(self.length(self.clip))
        } else {
            value.min(self.length(self.clip))
        }
    }
}

pub fn select(s: &Sim, input: Input) -> (&'static str, f32) {
    if let Some(stage) = s.reaction {
        return (stage.clip(), 1.);
    }
    if s.phase == Phase::JumpSquat {
        return ("KneeBend", 1.);
    }
    if s.phase == Phase::ChargeShot {
        return (s.behavior.charge_clip(), 1.);
    }
    if s.phase == Phase::Helpless {
        return (fall_clip(s, true), 1.);
    }
    if s.phase == Phase::ChargeCancel {
        return ("SpecialNCancel", 1.);
    }
    if let Some(m) = s.current_move {
        let clip = match m {
            Move::Jab => "Attack11",
            Move::Grab => "Catch",
            Move::DashGrab => "CatchDash",
            Move::AirGrab => "AirCatch",
            Move::GrabHold => "CatchWait",
            Move::Pummel => "CatchAttack",
            Move::ThrowForward => "ThrowF",
            Move::ThrowBack => "ThrowB",
            Move::ThrowUp => "ThrowHi",
            Move::ThrowDown => "ThrowLw",
            Move::Jab2 => "Attack12",
            Move::Jab3 => "Attack13",
            // ftCo_Attack100Start -> Loop -> End; `stage` is the clip.
            Move::RapidJab => match s.stage {
                0 => "Attack100Start",
                1 => "Attack100Loop",
                _ => "Attack100End",
            },
            Move::BackAir => "AttackAirB",
            Move::SideTilt => "AttackS3S",
            Move::UpTilt => "AttackHi3",
            Move::DownTilt => "AttackLw3",
            Move::DashAttack => "AttackDash",
            Move::NeutralAir => "AttackAirN",
            Move::ForwardAir => "AttackAirF",
            Move::UpAir => "AttackAirHi",
            Move::DownAir => "AttackAirLw",
            Move::SideSmash => "AttackS4S",
            Move::UpSmash => "AttackHi4",
            Move::DownSmash => "AttackLw4",
            m => s.behavior.clip(s, m),
        };
        return (
            clip,
            if s.phase == Phase::SmashCharge {
                0.
            } else {
                1.
            },
        );
    }
    if !s.grounded {
        // Jump animation completion, not the sign of vertical velocity, enters Fall.
        if s.motion.clip.starts_with("Jump") && !s.motion.finished() {
            return (s.motion.clip, 1.);
        }
        return (fall_clip(s, false), 1.);
    }
    if s.motion.clip.starts_with("Landing") && !s.motion.finished() {
        return (s.motion.clip, s.landing_rate);
    }
    let gait = locomotion(s, input);
    if gait.0 != "Wait" {
        return gait;
    }
    if input.buttons & DOWN != 0 {
        return (
            if s.motion.clip == "Squat" && s.motion.finished() || s.motion.clip == "SquatWait" {
                "SquatWait"
            } else {
                "Squat"
            },
            1.,
        );
    }
    ("Wait", 1.)
}

/// Generic grounded gait independent of the action clip. Mobile upper-body
/// attacks keep this clock moving through startup, hold and recovery.
pub fn locomotion(s: &Sim, input: Input) -> (&'static str, f32) {
    if !s.grounded {
        return ("Wait", 1.);
    }
    let speed = s.velocity.with_y(0.).length();
    if speed <= 0.05 {
        return ("Wait", 1.);
    }
    let scaled = speed / s.options.speed_scale.max(0.01);
    if input.buttons & SPRINT != 0 && !s.mobile_action() {
        return ("Run", speed / s.params.run_animation_rate);
    }
    let gait = if scaled >= s.params.walk_speed * s.params.walk_animation_thresholds[1] {
        2
    } else if scaled >= s.params.walk_speed * s.params.walk_animation_thresholds[0] {
        1
    } else {
        0
    };
    (
        ["WalkSlow", "WalkMiddle", "WalkFast"][gait],
        speed / s.params.walk_animation_rates[gait],
    )
}
fn fall_clip(s: &Sim, special: bool) -> &'static str {
    let drift = s.velocity.with_y(0.).dot(s.facing)
        / (s.params.air_speed * s.options.speed_scale).max(0.001);
    let (neutral, front, back) = if special {
        ("FallSpecial", "FallSpecialF", "FallSpecialB")
    } else if s.jumps_used > 1 {
        ("FallAerial", "FallAerialF", "FallAerialB")
    } else {
        ("Fall", "FallF", "FallB")
    };
    if drift > s.params.fall_pose_threshold {
        front
    } else if drift < -s.params.fall_pose_threshold {
        back
    } else if s.fall_blend > 0.001 && (s.motion.clip == front || s.motion.clip == back) {
        s.motion.clip
    } else {
        neutral
    }
}
