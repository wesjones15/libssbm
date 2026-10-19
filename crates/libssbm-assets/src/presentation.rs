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
    pub hidden: bool,
    pub material_tracks: Vec<crate::animation::Track>,
    pub duration: f32,
    pub commands: Vec<libssbm::action::TimedCommand>,
    pub object_scale: f32,
    pub upper_body_root: Option<usize>,
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
            hidden: false,
            material_tracks: Vec::new(),
            duration: 0.,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: None,
        })
    }
    pub fn effect(dir: &Path, id: u32) -> Result<Self> {
        let archive = if id >= 2000 {
            crate::fighter::SAMUS.effects
        } else {
            crate::fighter::COMMON_EFFECTS
        };
        let mut effect = Self::effect_from(dir, archive, if id >= 2000 { id - 2000 } else { id })?;
        if id == 25 {
            crate::effects::ko_palette(dir, &mut effect.model, 0)?;
        } else if (11..=14).contains(&id) {
            crate::effects::shield_palette(dir, &mut effect.model, 0, usize::from(id == 11))?;
        }
        Ok(effect)
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
            hidden: false,
            material_tracks: model.material_tracks,
            duration: model.lifetime,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: None,
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
        let object_scale = if kind == 1 && state > 0 {
            a.special[6] + (a.special[7] - a.special[6]) * (state - 1) as f32 / 7.
        } else {
            1.
        };
        Ok(Self {
            model: a.model,
            animations: a.animations,
            clip: "Object".into(),
            frame: 0.,
            world,
            ball: false,
            hidden: false,
            material_tracks: a.material_tracks,
            duration,
            commands: a.commands,
            object_scale,
            upper_body_root: None,
        })
    }
    pub fn object_pose(&mut self, frame: f32) {
        self.frame = frame;
        self.world = self.animations.pose_object(&self.model, frame);
        if self.object_scale != 1. {
            let scale = Mat4::from_scale(glam::Vec3::splat(self.object_scale));
            for world in &mut self.world {
                *world = scale * *world;
            }
        }
    }
    pub fn face_camera(&mut self, camera: glam::Vec3) {
        self.model.face_camera(&mut self.world, camera);
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
        if fighter
            .upper_body_root
            .is_some_and(|root| root >= model.joints.len())
        {
            return Err("upper-body joint outside fighter rig".into());
        }
        let world = model.bind_world();
        Ok(Self {
            model,
            animations: Animations::load_for(dir, fighter)?,
            clip: String::new(),
            frame: 0.,
            world,
            ball: false,
            hidden: false,
            material_tracks: Vec::new(),
            duration: 0.,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: fighter.upper_body_root,
        })
    }
    pub fn update(&mut self, sim: &Sim, _dt: f32) {
        let clip = sim.motion.clip;
        self.clip = clip.into();
        self.frame = sim.motion.render_frame(sim.alpha());
        self.ball = sim.current_move == Some(Move::Bomb);
        self.hidden = sim
            .rebirth
            .as_ref()
            .is_some_and(|s| s.phase == libssbm::rebirth::Phase::Ko);
        self.world = mobile_pose(
            &self.model,
            &self.animations,
            sim,
            self.frame,
            self.upper_body_root,
        );
        let neutral = match clip {
            "FallF" | "FallB" => Some("Fall"),
            "FallAerialF" | "FallAerialB" => Some("FallAerial"),
            "FallSpecialF" | "FallSpecialB" => Some("FallSpecial"),
            _ => None,
        };
        if let Some(neutral) = neutral {
            let base = self.animations.pose(&self.model, neutral, self.frame, true);
            self.world = blend_pose(&self.model, &base, &self.world, sim.fall_blend);
        }
        if let Some(entry) = sim.entry {
            let transform = entry.transform();
            for world in &mut self.world {
                *world = transform * *world;
            }
        }
    }
    pub fn visible(&self, batch: usize) -> bool {
        if self.hidden {
            return false;
        }
        let b = &self.model.batches[batch];
        if self.ball {
            self.model.ball_visible[b.dobj]
        } else {
            self.model.normal_visible[b.dobj]
        }
    }
    fn material_color(&self, batch: usize) -> ([f32; 4], f32) {
        let b = &self.model.batches[batch];
        let mut color = b.color;
        let mut blending = b.texture_blend;
        for track in &self.material_tracks {
            if track.joint == b.material_dobj
                && let Some(value) = track.sample(self.frame)
            {
                match track.channel {
                    4..=6 => color[(track.channel - 4) as usize] = value.clamp(0., 1.),
                    10 => color[3] = value.clamp(0., 1.),
                    109 => blending = value.clamp(0., 1.),
                    _ => (),
                }
            }
        }
        (color, blending)
    }
    /// Normalized source metadata for the common host alpha adapter. The
    /// viewer keeps the same resolution below; no HSD flags cross into Unity.
    pub fn alpha_source(&self, batch: usize) -> [f32; 3] {
        let b = &self.model.batches[batch];
        let (color, blending) = self.material_color(batch);
        let mode = if b.texture.is_some() {
            match b.texture_alpha {
                2 => 1.,
                4 => 2.,
                _ => 0.,
            }
        } else {
            0.
        };
        [color[3], blending, mode]
    }
    pub fn color(&self, batch: usize) -> [f32; 4] {
        let b = &self.model.batches[batch];
        let (mut color, blending) = self.material_color(batch);
        if b.texture.is_some() {
            // baselib/tobj.c::MakeTExp: TEX_ALPHAMAP_BLEND interpolates
            // material and sampled alpha. Multiplying a zero material alpha
            // by the texture incorrectly erases the entire Screw Attack.
            color[3] = match b.texture_alpha {
                2 => color[3] + (1. - color[3]) * blending,
                4 => 1.,
                _ => color[3],
            };
        }
        color
    }
}
pub fn clip_name(s: &Sim) -> &'static str {
    s.motion.clip
}
/// Render the action on its original clock while a separately timed gait drives
/// the root, pelvis and legs. `upper_root` is fighter data, not a vertex cut.
pub fn mobile_pose(
    model: &Model,
    animations: &Animations,
    sim: &Sim,
    action_frame: f32,
    upper_root: Option<usize>,
) -> Vec<Mat4> {
    let mut pose = mobile_pose_layers(
        model,
        animations,
        sim.motion.clip,
        action_frame,
        sim.motion.looping(),
        sim.locomotion.clip,
        sim.locomotion.render_frame(sim.alpha()),
        sim.locomotion.looping(),
        if sim.mobile_action() {
            sim.locomotion_weight
        } else {
            0.
        },
        upper_root,
    );
    if let (Some(root), Some((_, yaw))) = (upper_root, sim.side_special_upper_aim()) {
        aim_upper_body(model, &mut pose, root, yaw);
    }
    pose
}
/// Turn the torso toward camera aim without changing the source cannon motion.
pub fn aim_upper_body(model: &Model, pose: &mut [Mat4], root: usize, yaw: f32) {
    if root >= model.joints.len()
        || pose.len() != model.joints.len()
        || !yaw.is_finite()
    {
        return;
    }
    let pivot = pose[root].transform_point3(glam::Vec3::ZERO);
    let turn = Mat4::from_translation(pivot)
        * Mat4::from_rotation_y(yaw)
        * Mat4::from_translation(-pivot);
    let mut branch = vec![false; model.joints.len()];
    for (i, joint) in model.joints.iter().enumerate() {
        branch[i] = i == root || joint.parent.is_some_and(|parent| branch[parent]);
        if branch[i] {
            pose[i] = turn * pose[i];
        }
    }
}
pub fn mobile_pose_layers(
    model: &Model,
    animations: &Animations,
    action_clip: &str,
    action_frame: f32,
    action_loop: bool,
    gait_clip: &str,
    gait_frame: f32,
    gait_loop: bool,
    weight: f32,
    upper_root: Option<usize>,
) -> Vec<Mat4> {
    let action = animations.pose(model, action_clip, action_frame, action_loop);
    let Some(root) = upper_root else {
        return action;
    };
    if weight <= 0. || root >= model.joints.len() {
        return action;
    }
    let gait = animations.pose(model, gait_clip, gait_frame, gait_loop);
    let combined = splice_pose(model, &gait, &action, root);
    blend_pose(model, &action, &combined, weight)
}
/// Combine local transforms, then rebuild the hierarchy. Matrix cuts in world
/// space would shear the armor and move the cannon away from its attachment.
pub fn splice_pose(model: &Model, lower: &[Mat4], upper: &[Mat4], upper_root: usize) -> Vec<Mat4> {
    let mut world = Vec::with_capacity(model.joints.len());
    let mut upper_branch = vec![false; model.joints.len()];
    for (i, joint) in model.joints.iter().enumerate() {
        upper_branch[i] = i == upper_root || joint.parent.is_some_and(|p| upper_branch[p]);
        let source = if upper_branch[i] { upper } else { lower };
        let local = joint
            .parent
            .map_or(source[i], |p| source[p].inverse() * source[i]);
        world.push(joint.parent.map_or(local, |p| world[p] * local));
    }
    world
}
/// ftCo_Fall_Anim_Inner blends local joint transforms, then concatenates the
/// hierarchy. Blending world matrices would shorten limbs during rotations.
pub fn blend_pose(model: &Model, a: &[Mat4], b: &[Mat4], amount: f32) -> Vec<Mat4> {
    let amount = amount.clamp(0., 1.);
    let mut world = Vec::with_capacity(model.joints.len());
    for (i, joint) in model.joints.iter().enumerate() {
        let local = |pose: &[Mat4]| {
            joint
                .parent
                .map_or(pose[i], |parent| pose[parent].inverse() * pose[i])
        };
        let (sa, ra, ta) = local(a).to_scale_rotation_translation();
        let (sb, rb, tb) = local(b).to_scale_rotation_translation();
        let matrix = Mat4::from_scale_rotation_translation(
            sa.lerp(sb, amount),
            ra.slerp(rb, amount),
            ta.lerp(tb, amount),
        );
        world.push(joint.parent.map_or(matrix, |parent| world[parent] * matrix));
    }
    world
}
