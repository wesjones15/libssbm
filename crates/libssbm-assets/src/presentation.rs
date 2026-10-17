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
    pub material_tracks: Vec<crate::animation::Track>,
    pub duration: f32,
    pub commands: Vec<libssbm::action::TimedCommand>,
}
impl Presentation {
    pub fn platform(dir: &Path) -> Result<Self> {
        let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
        let d = crate::dat::Dat::parse(&bytes)?;
        let model = Model::from_joint(
            &d,
            d.required_ptr(d.root(crate::fighter::COMMON_DATA.root)? + 0x40)?,
        )?;
        let world = model.bind_world();
        Ok(Self {
            model,
            world,
            animations: Animations {
                clips: Default::default(),
            },
            clip: "Object".into(),
            frame: 0.,
            ball: false,
            material_tracks: Vec::new(),
            duration: 0.,
            commands: Vec::new(),
        })
    }
    pub fn effect(dir: &Path, id: u32) -> Result<Self> {
        let archive = if id >= 2000 {
            crate::fighter::SAMUS.effects
        } else {
            crate::fighter::COMMON_EFFECTS
        };
        Self::effect_from(dir, archive, if id >= 2000 { id - 2000 } else { id })
    }
    /// A bank-local model index, independent of the fighter's source effect IDs.
    pub fn effect_from(
        dir: &Path,
        archive: crate::fighter::Archive<'_>,
        model: u32,
    ) -> Result<Self> {
        let bank = crate::effects::Effects::shared_archive(dir, archive)?;
        let model = bank
            .models
            .get(model as usize)
            .ok_or("effect model missing")?
            .clone();
        let world = model.model.bind_world();
        Ok(Self {
            model: model.model,
            animations: model.animations,
            clip: "Object".into(),
            frame: 0.,
            world,
            ball: false,
            material_tracks: model.material_tracks,
            duration: model.lifetime,
            commands: Vec::new(),
        })
    }
    pub fn article(dir: &Path, kind: u32, state: u32) -> Result<Self> {
        Self::article_for(dir, kind, state, &crate::fighter::SAMUS)
    }
    pub fn article_for(
        dir: &Path,
        kind: u32,
        state: u32,
        fighter: &crate::fighter::FighterAssets<'_>,
    ) -> Result<Self> {
        let a = crate::article::Article::load_for(dir, kind, state, fighter)?;
        let world = a.model.bind_world();
        let duration = a
            .animations
            .clips
            .get("Object")
            .map(|c| c.frames)
            .unwrap_or(0.);
        Ok(Self {
            model: a.model,
            animations: a.animations,
            clip: "Object".into(),
            frame: 0.,
            world,
            ball: false,
            material_tracks: a.material_tracks,
            duration,
            commands: a.commands,
        })
    }
    pub fn object_pose(&mut self, frame: f32) {
        self.frame = frame;
        self.world = self.animations.pose_object(&self.model, frame);
    }
    pub fn load(dir: &Path, costume: usize) -> Result<Self> {
        Self::load_for(dir, costume, &crate::fighter::SAMUS)
    }
    pub fn load_for(
        dir: &Path,
        costume: usize,
        fighter: &crate::fighter::FighterAssets<'_>,
    ) -> Result<Self> {
        let model = Model::load_for(dir, costume, fighter)?;
        let world = model.bind_world();
        Ok(Self {
            model,
            animations: Animations::load_for(dir, fighter)?,
            clip: String::new(),
            frame: 0.,
            world,
            ball: false,
            material_tracks: Vec::new(),
            duration: 0.,
            commands: Vec::new(),
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
        if let Some(entry) = sim.entry {
            let transform = entry.transform();
            for world in &mut self.world {
                *world = transform * *world;
            }
        }
    }
    pub fn visible(&self, batch: usize) -> bool {
        let b = &self.model.batches[batch];
        if self.ball {
            self.model.ball_visible[b.dobj]
        } else {
            self.model.normal_visible[b.dobj]
        }
    }
    pub fn color(&self, batch: usize) -> [f32; 4] {
        let b = &self.model.batches[batch];
        let mut color = b.color;
        for track in &self.material_tracks {
            if track.joint == b.dobj
                && let Some(value) = track.sample(self.frame)
            {
                match track.channel {
                    4..=6 => color[(track.channel - 4) as usize] = value.clamp(0., 1.),
                    10 => color[3] = value.clamp(0., 1.),
                    _ => (),
                }
            }
        }
        color
    }
}
pub fn clip_name(s: &Sim) -> &'static str {
    s.motion.clip
}
