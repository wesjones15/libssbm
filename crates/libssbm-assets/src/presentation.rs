use crate::{Result, animation::Animations, model::Model};
use glam::Mat4;
use libssbm::{Move, Sim};
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
    pub fn update(&mut self, sim: &Sim, _dt: f32) {
        let clip = sim.motion.clip;
        self.clip = clip.into();
        self.frame = sim.motion.render_frame(sim.alpha());
        self.ball = sim.current_move == Some(Move::Bomb);
        self.world = self
            .animations
            .pose(&self.model, clip, self.frame, sim.motion.looping());
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
    s.motion.clip
}
