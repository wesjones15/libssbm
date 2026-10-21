//! ftCo_Guard / ftCo_EscapeAir and Fighter_procCollResolve behavioral references.
//! Shield damage is supplied by the host; animation, meter and timing stay native.
use crate::{DEFEND, Host, Input, JUMP, Phase, Sim, action::SoundCue};
use glam::Vec3;
#[derive(Clone, Debug)]
pub struct Params {
    pub maximum: f32,
    pub drain: f32,
    pub regen: f32,
    pub hit_mul: f32,
    pub hit_add: f32,
    pub restore: f32,
    pub air_force: f32,
    pub air_decay: f32,
    pub broken_frames: u32,
    pub shield_size: f32,
    pub minimum_scale: f32,
    pub hard_scale: f32,
}
impl Default for Params {
    fn default() -> Self {
        Self {
            maximum: 60.,
            drain: 0.28,
            regen: 0.07,
            hit_mul: 1.,
            hit_add: 0.,
            restore: 30.,
            air_force: 3.1,
            air_decay: 0.9,
            broken_frames: 240,
            shield_size: 10.,
            minimum_scale: 0.4,
            hard_scale: 1.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    On,
    Hold,
    Off,
    Air,
    Broken,
}
#[derive(Clone, Debug)]
pub struct Defense {
    pub params: Params,
    pub health: f32,
    pub mode: Option<Mode>,
    pub air_used: bool,
    pub frame: u32,
}
impl Default for Defense {
    fn default() -> Self {
        let params = Params::default();
        Self {
            health: params.maximum,
            params,
            mode: None,
            air_used: false,
            frame: 0,
        }
    }
}
impl Sim {
    pub fn shield_scale(&self) -> f32 {
        let p = &self.defense.params;
        p.shield_size
            * (p.minimum_scale
                + (1. - p.minimum_scale)
                    * (self.defense.health / p.maximum).clamp(0., 1.)
                    * p.hard_scale)
    }
    pub fn defense_flags(&self) -> u32 {
        let defense = match self.defense.mode {
            Some(Mode::On) => 1 | 16 | 32,
            Some(Mode::Hold) => 1 | 16,
            Some(Mode::Air) => 8 | if self.action.hurt_state != 0 { 2 } else { 0 },
            Some(Mode::Broken) => 4,
            Some(Mode::Off) => 16,
            None => 0,
        };
        defense
            | if self.reaction.is_some() || self.knockdown_grace > 0 {
                64
            } else {
                0
            }
            | if self.up_special_invulnerable {
                128
            } else {
                0
            }
            // A fluttering jump's armor (Yoshi): hits do not move him.
            | if self.flutter > 0 { 256 } else { 0 }
    }
    pub fn shield_hit(&mut self, damage: f32) -> bool {
        if self.defense_flags() & 1 == 0 || !damage.is_finite() || damage < 0. {
            return false;
        }
        self.defense.health = (self.defense.health
            - self.defense.params.hit_mul * damage
            - self.defense.params.hit_add)
            .max(0.);
        self.action.sounds.push(SoundCue {
            sound: 127,
            volume: 127,
            pan: 64,
            channel: 0,
            stop: false,
        });
        if self.defense.health <= 0. {
            self.defense_mode(Mode::Broken);
        }
        true
    }
    fn defense_mode(&mut self, mode: Mode) {
        self.defense.mode = Some(mode);
        self.defense.frame = 0;
        self.current_move = None;
        self.action.reset();
        self.phase = Phase::Free;
        self.motion.select(
            match mode {
                Mode::On => "GuardOn",
                Mode::Hold => "Guard",
                Mode::Off => "GuardOff",
                Mode::Air => "EscapeAir",
                Mode::Broken => "Furafura",
            },
            1.,
        );
        self.motion.frame = 0.;
        if mode == Mode::On {
            self.action.sounds.push(SoundCue {
                sound: 110,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
        }
        if mode == Mode::Off {
            self.action.sounds.push(SoundCue {
                sound: 128,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
        }
        if mode == Mode::Broken {
            self.action.sounds.push(SoundCue {
                sound: 129,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
        }
    }
    pub(crate) fn tick_defense(
        &mut self,
        host: &mut impl Host,
        input: Input,
        pressed: u32,
    ) -> bool {
        if self.grounded {
            self.defense.air_used = false;
        }
        if self.defense.mode.is_none() {
            self.defense.health =
                (self.defense.health + self.defense.params.regen).min(self.defense.params.maximum);
            if self.phase != Phase::Free
                || self.current_move.is_some()
                || self.reaction.is_some()
                || pressed & DEFEND == 0
                || input.allowed & (1 << 4) == 0
            {
                return false;
            }
            if self.grounded {
                self.defense_mode(Mode::On);
            } else if !self.defense.air_used {
                self.defense.air_used = true;
                self.pad_gravity = None;
                self.velocity = input.movement.normalize_or_zero() * self.defense.params.air_force;
                self.defense_mode(Mode::Air);
            } else {
                return false;
            }
        }
        let mode = self.defense.mode.unwrap();
        if matches!(mode, Mode::On | Mode::Hold) {
            self.defense.health = (self.defense.health - self.defense.params.drain).max(0.);
            if self.defense.health <= 0. {
                self.defense_mode(Mode::Broken);
            } else if pressed & JUMP != 0 {
                self.defense.mode = None;
                self.action.reset();
                self.short_hop = false;
                self.set_phase(Phase::JumpSquat);
                return false;
            } else if input.buttons & DEFEND == 0 {
                self.defense_mode(Mode::Off);
            } else if mode == Mode::On && self.motion.finished() {
                self.defense_mode(Mode::Hold);
            }
        } else if (mode != Mode::Broken && self.motion.finished())
            || (mode == Mode::Broken && self.defense.frame >= self.defense.params.broken_frames)
        {
            self.defense.mode = None;
            self.action.reset();
            if mode == Mode::Air {
                self.phase = Phase::Helpless;
            } else {
                self.phase = Phase::Free;
            }
            if mode == Mode::Broken {
                self.defense.health = self.defense.params.restore;
            }
            return false;
        }
        if mode == Mode::Air {
            if self.action.variables[0] == 0 {
                self.velocity *= self.defense.params.air_decay;
            } else {
                self.velocity.y =
                    (self.velocity.y - self.params.gravity).max(-self.params.fall_speed);
            }
        } else if self.grounded {
            self.velocity = Vec3::ZERO;
        } else {
            self.velocity.x = 0.;
            self.velocity.z = 0.;
            self.velocity.y = (self.velocity.y - self.params.gravity).max(-self.params.fall_speed);
        }
        let contact = host.collide(self.position, self.position + self.velocity);
        self.position = contact.position;
        self.grounded = contact.grounded && self.velocity.y <= 0.;
        if self.grounded && mode == Mode::Air {
            self.defense.mode = None;
            self.action.reset();
            self.velocity = Vec3::ZERO;
            self.motion.select("LandingFallSpecial", 1.);
            self.motion.frame = 0.;
            return true;
        }
        if !self.grounded && matches!(mode, Mode::On | Mode::Hold) {
            self.defense.mode = None;
            self.action.reset();
            return true;
        }
        self.defense.frame += 1;
        self.motion.advance();
        self.execute_disc_commands(host, input);
        true
    }
}
