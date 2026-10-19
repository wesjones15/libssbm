//! Samus article behavior, driven by disc attributes. Reimplemented from
//! itsamus{bomb,missile,chargeshot}.c; host owns world collision and unit conversion.
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Params {
    pub kind: u32,
    pub state: u32,
    pub life: f32,
    pub speed: f32,
    pub acceleration: f32,
    pub max_speed: f32,
    pub delay: f32,
    pub homing_frames: f32,
    pub decay: f32,
    pub min_speed: f32,
    pub turn_accel: f32,
    pub max_turn: f32,
    pub min_angle: f32,
    pub gravity: f32,
    pub max_fall: f32,
    pub initial_y: f32,
    pub damage: f32,
    pub radius: f32,
    pub explosion_radius: f32,
    pub explosion_frames: f32,
}
pub struct Projectile {
    pub params: Params,
    pub position: Vec3,
    pub velocity: Vec3,
    pub direction: Vec3,
    pub frame: u32,
    pub animation_frame: u32,
    pub impacted: bool,
    pub finished: bool,
    pub grounded: bool,
    launch_direction: Vec3,
    clock: f64,
}
impl Projectile {
    pub fn new(params: Params, direction: Vec3) -> Self {
        let direction = direction.try_normalize().unwrap_or(Vec3::Z);
        let velocity = direction * params.speed + Vec3::Y * params.initial_y;
        Self {
            params,
            position: Vec3::ZERO,
            velocity,
            direction,
            frame: 0,
            animation_frame: 0,
            impacted: false,
            finished: false,
            grounded: false,
            launch_direction: direction,
            clock: 0.,
        }
    }
    pub fn impact(&mut self) {
        if self.impacted || self.finished {
            return;
        }
        self.impacted = true;
        self.animation_frame = 0;
        self.velocity = Vec3::ZERO;
        if self.params.kind == 1 {
            self.finished = true;
        }
    }
    pub fn state(&self) -> u32 {
        if !self.impacted {
            self.params.state
        } else if self.params.kind == 0 {
            1
        } else {
            self.params.state + 2
        }
    }
    pub fn advance(&mut self, dt: f64, target: Option<Vec3>) -> u32 {
        if !dt.is_finite() || dt <= 0. {
            return 0;
        }
        self.clock += dt.min(0.25);
        let mut ticks = 0;
        while self.clock + 1e-9 >= 1. / 60. {
            self.clock -= 1. / 60.;
            ticks += 1;
            if self.finished {
                continue;
            }
            self.frame += 1;
            self.animation_frame += 1;
            if self.impacted {
                if self.animation_frame as f32 >= self.params.explosion_frames {
                    self.finished = true;
                }
                continue;
            }
            if self.frame as f32 > self.params.life {
                if self.params.kind == 1 {
                    self.finished = true;
                } else {
                    self.impact();
                }
                continue;
            }
            match self.params.kind {
                0 => {
                    if !self.grounded {
                        self.velocity.y =
                            (self.velocity.y - self.params.gravity).max(-self.params.max_fall);
                    }
                }
                2 if self.params.state == 0 => {
                    if self.frame as f32 <= self.params.homing_frames {
                        if let Some(target) =
                            target.and_then(|v| (v - self.position).try_normalize())
                        {
                            let current = self.velocity.normalize_or_zero();
                            let angle = current.dot(target).clamp(-1., 1.).acos();
                            if angle >= self.params.min_angle {
                                let next = current
                                    .slerp(
                                        target,
                                        (self.params.turn_accel / angle.max(0.0001)).min(1.),
                                    )
                                    .normalize_or_zero();
                                let turn = self.launch_direction.dot(next).clamp(-1., 1.).acos();
                                self.direction = if turn > self.params.max_turn {
                                    self.launch_direction
                                        .slerp(next, self.params.max_turn / turn)
                                } else {
                                    next
                                };
                            }
                        }
                        self.velocity = self.direction * self.params.speed;
                    } else if self.params.life.is_finite() && self.velocity.length() > self.params.min_speed {
                        // A host that lifts the lifetime cap (unlimited range) keeps the cruise speed.
                        self.velocity *= self.params.decay;
                    }
                }
                2 => {
                    if self.frame as f32 > self.params.delay {
                        let speed = (self.velocity.length() + self.params.acceleration)
                            .min(self.params.max_speed);
                        self.velocity = self.direction * speed;
                    }
                }
                _ => (),
            }
            self.position += self.velocity;
        }
        ticks
    }
}
