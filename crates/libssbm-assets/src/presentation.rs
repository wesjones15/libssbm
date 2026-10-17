use crate::{Result, animation::Animations, model::Model};
use glam::Mat4;
use libssbm::{Move, Phase, Sim};
use std::path::Path;
pub struct Presentation {
    pub model: Model,
    pub animations: Animations,
    pub clip: String,
    pub frame: f32,
    pub world: Vec<Mat4>,
    pub ball: bool,
}
impl Presentation {
    pub fn load(dir: &Path, costume: usize) -> Result<Self> {
        let model = Model::load(dir, costume)?;
        let world = model.bind_world();
        Ok(Self {
            model,
            animations: Animations::load(dir)?,
            clip: String::new(),
            frame: 0.,
            world,
            ball: false,
        })
    }
    pub fn update(&mut self, sim: &Sim, dt: f32) {
        let clip = clip_name(sim);
        if self.clip != clip {
            self.clip = clip.into();
            self.frame = 0.;
        } else if sim.phase != Phase::SmashCharge {
            self.frame += dt * 60.;
        }
        self.ball = sim.current_move == Some(Move::Bomb);
        self.world = self.animations.pose(
            &self.model,
            clip,
            self.frame,
            matches!(sim.phase, Phase::Free | Phase::ChargeShot | Phase::Helpless),
        );
    }
    pub fn visible(&self, batch: usize) -> bool {
        let b = &self.model.batches[batch];
        if self.ball {
            self.model.ball_visible[b.dobj]
        } else {
            self.model.normal_visible[b.dobj]
        }
    }
}
pub fn clip_name(s: &Sim) -> &'static str {
    if s.phase == Phase::JumpSquat {
        return "KneeBend";
    }
    if s.phase == Phase::ChargeShot {
        return "SpecialNHold";
    }
    if s.phase == Phase::Helpless {
        return "FallSpecial";
    }
    if let Some(m) = s.current_move {
        return match m {
            Move::Jab => "Attack11",
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
            Move::ChargeShot => {
                if s.phase == Phase::Startup {
                    if s.grounded {
                        "SpecialNStart"
                    } else {
                        "SpecialAirNStart"
                    }
                } else {
                    if s.grounded {
                        "SpecialN"
                    } else {
                        "SpecialAirN"
                    }
                }
            }
            Move::Missile => {
                if s.grounded {
                    "SpecialS"
                } else {
                    "SpecialAirS"
                }
            }
            Move::SuperMissile => {
                if s.grounded {
                    "Special"
                } else {
                    "SpecialAir"
                }
            }
            Move::ScrewAttack => {
                if s.grounded {
                    "SpecialHi"
                } else {
                    "SpecialAirHi"
                }
            }
            Move::Bomb => {
                if s.grounded {
                    "SpecialLw"
                } else {
                    "SpecialAirLw"
                }
            }
        };
    }
    if !s.grounded {
        if s.velocity.y > 0. {
            if s.jumps_used > 1 {
                "JumpAerialF"
            } else {
                "JumpF"
            }
        } else {
            "Fall"
        }
    } else if s.velocity.with_y(0.).length() > 1. {
        "Run"
    } else if s.velocity.with_y(0.).length() > 0.1 {
        "WalkMiddle"
    } else {
        "Wait"
    }
}
