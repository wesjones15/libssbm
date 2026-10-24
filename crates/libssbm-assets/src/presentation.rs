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
    /// Draw the alternate DObj choice (the morph ball / egg) instead of the body.
    pub ball: bool,
    /// The move that swaps to `ball` (fighter data); None for non-fighters.
    pub morph_move: Option<Move>,
    /// Joint the source rotates by the distance rolled (Yoshi's whole-egg spin,
    /// ftYs_SpecialS_WrapAndSetRotX -> ftPartSetRotX(fp, 3, ...)).
    pub spin_joint: Option<usize>,
    pub hidden: bool,
    pub material_tracks: Vec<crate::animation::Track>,
    pub duration: f32,
    pub commands: Vec<libssbm::action::TimedCommand>,
    pub object_scale: f32,
    pub upper_body_root: Option<usize>,
    /// Common effect models (dust, bursts): see [`Presentation::color`].
    pub effect_fade: bool,
    /// The face's texture animations: (batch, frame-to-image track) per costume TObj, and the
    /// frames the sim last set (`Sim::face`).
    pub face_tracks: Vec<(usize, crate::animation::Track)>,
    pub face: [f32; 8],
    /// The fighter's part animations (hand poses, ears, mouth) and the ones the sim has playing.
    pub part_clips: crate::animation::PartAnimations,
    pub parts: [Option<libssbm::sim::PartPlay>; 5],
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
            morph_move: None,
            spin_joint: None,
            hidden: false,
            material_tracks: Vec::new(),
            duration: 0.,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: None,
            effect_fade: false,
            face_tracks: Vec::new(),
            face: [0.; 8],
            part_clips: Default::default(),
            parts: [None; 5],
        })
    }
    /// The results-screen GAME! burst: `ScInfCnt_scene_models[5]` in the shared
    /// IfAll.usd archive. if_2F6E.c::ifStatus_802F7134 loads the descriptor and
    /// if_2F72.c::fn_802F7288 advances its object animation; the burst voice
    /// fires once its frame passes 8. The model is a DynamicModelDesc (joint +
    /// NULL-terminated anim/matanim arrays), reusing the object effect readers.
    pub fn game(dir: &Path) -> Result<Self> {
        let bytes = std::fs::read(dir.join(crate::fighter::RESULTS_SCENE.file))?;
        let d = crate::dat::Dat::parse(&bytes)?;
        let root = d.root(crate::fighter::RESULTS_SCENE.root)?;
        let desc = d.required_ptr(root + 5 * 4)?;
        let mut model = Model::from_joint(&d, d.required_ptr(desc)?)?;
        let first = |array: Option<usize>| -> Result<Option<usize>> {
            let Some(array) = array else { return Ok(None) };
            let pointer = d.u32(array)? as usize;
            Ok((pointer != 0).then_some(pointer))
        };
        let animations = Animations::object(&d, first(d.ptr(desc + 4)?)?, &model)?;
        let material_tracks = Animations::materials(&d, first(d.ptr(desc + 8)?)?, &mut model)?;
        let duration = animations.clips.get("Object").map(|c| c.frames).unwrap_or(0.);
        let world = model.bind_world();
        Ok(Self {
            model,
            animations,
            clip: "Object".into(),
            frame: 0.,
            world,
            ball: false,
            morph_move: None,
            spin_joint: None,
            hidden: false,
            material_tracks,
            duration,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: None,
            effect_fade: false,
            face_tracks: Vec::new(),
            face: [0.; 8],
            part_clips: Default::default(),
            parts: [None; 5],
        })
    }
    pub fn effect(dir: &Path, id: u32) -> Result<Self> {
        Self::effect_for(dir, id, &crate::fighter::SAMUS)
    }
    pub fn effect_for(
        dir: &Path,
        id: u32,
        fighter: &crate::fighter::FighterAssets<'_>,
    ) -> Result<Self> {
        let archive = if id >= 2000 {
            fighter.effects
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
            morph_move: None,
            spin_joint: None,
            hidden: false,
            material_tracks: model.material_tracks,
            duration: model.lifetime,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: None,
            effect_fade: true,
            face_tracks: Vec::new(),
            face: [0.; 8],
            part_clips: Default::default(),
            parts: [None; 5],
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
            morph_move: None,
            spin_joint: None,
            hidden: false,
            material_tracks: a.material_tracks,
            duration,
            commands: a.commands,
            object_scale,
            upper_body_root: None,
            effect_fade: false,
            face_tracks: Vec::new(),
            face: [0.; 8],
            part_clips: Default::default(),
            parts: [None; 5],
        })
    }
    /// An item of the common item archive (the Super Scope held, or one level of its beam), posed like an article.
    pub fn item_for(dir: &Path, item: crate::article::Item, state: u32) -> Result<Self> {
        let a = crate::article::Article::load_item(dir, item, state)?;
        let world = a.model.bind_world();
        let duration = a.animations.clips.get("Object").map(|c| c.frames).unwrap_or(0.);
        Ok(Self {
            model: a.model,
            animations: a.animations,
            clip: "Object".into(),
            frame: 0.,
            world,
            ball: false,
            morph_move: None,
            spin_joint: None,
            hidden: false,
            material_tracks: a.material_tracks,
            duration,
            commands: a.commands,
            object_scale: 1.,
            upper_body_root: None,
            effect_fade: false,
            face_tracks: Vec::new(),
            face: [0.; 8],
            part_clips: Default::default(),
            parts: [None; 5],
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
        let mut model = Model::load_for(dir, costume, fighter)?;
        let face_tracks = face_animation(dir, costume, fighter, &mut model)?;
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
            morph_move: fighter.morph_move(),
            // Yoshi's egg is skinned to joint 3; the source spins that joint as he rolls.
            spin_joint: (fighter.common_index == 14).then_some(3),
            hidden: false,
            material_tracks: Vec::new(),
            duration: 0.,
            commands: Vec::new(),
            object_scale: 1.,
            upper_body_root: fighter.upper_body_root,
            effect_fade: false,
            face_tracks,
            face: [0.; 8],
            part_clips: crate::animation::PartAnimations::load(dir, fighter)?,
            parts: [None; 5],
        })
    }
    /// A fighter's alternate DObj alone (`ball`), for a view that does not follow a
    /// sim: Yoshi's laid egg and his rolling shell. The body is hidden by the
    /// fighter's visibility table.
    pub fn load_for_variant(
        dir: &Path,
        costume: usize,
        fighter: &crate::fighter::FighterAssets<'_>,
        ball: bool,
    ) -> Result<Self> {
        let mut view = Self::load_for(dir, costume, fighter)?;
        view.ball = ball;
        Ok(view)
    }
    pub fn update(&mut self, sim: &Sim, _dt: f32) {
        let clip = sim.motion.clip;
        self.clip = clip.into();
        self.face = sim.face;
        self.parts = sim.parts;
        self.frame = sim.motion.render_frame(sim.alpha());
        // ftParts_80074B0C: the fighter's alternate DObj (Samus's morph ball,
        // Yoshi's egg) replaces the body for its move.
        self.ball = (self.morph_move.is_some() && sim.current_move == self.morph_move)
            || (sim.morph_roll_hold && self.morph_move == Some(Move::SpecialLw));
        self.hidden = sim
            .rebirth
            .as_ref()
            .is_some_and(|s| s.phase == libssbm::rebirth::Phase::Ko);
        let overlay = self.part_overlay(sim.alpha());
        self.world = mobile_pose_over(
            &self.model,
            &self.animations,
            sim,
            self.frame,
            self.upper_body_root,
            &overlay,
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
        if self.ball && self.morph_move == Some(Move::SpecialS) {
            // Yoshi's roll passes animate TransN (joint 1) 25 units forward and restart at
            // zero on the next pass. The sim owns the travel, so pin the rig to the sim
            // position; otherwise the model lurches forward and snaps back every loop.
            pin_joint_to_bind(&self.model, &mut self.world, 1);
        }
        if self.ball
            && let Some(joint) = self.spin_joint
        {
            // Distance rolled over the egg's radius (5.77 model units), matching the
            // source's per-frame x14 accumulation and the host's movement scale.
            let angle = sim.roll_angle(5.77);
            spin_subtree(&self.model, &mut self.world, joint, angle);
        }
        if let Some(entry) = sim.entry {
            let transform = entry.transform();
            for world in &mut self.world {
                *world = transform * *world;
            }
        }
    }
    /// The part animations to lay over this tick's pose (ftAnim_800707B0 and the animation
    /// the joints play): each slot's chosen clip at its age, with its blend-in weight.
    pub fn part_overlay(&self, alpha: f32) -> Vec<crate::animation::Overlay<'_>> {
        self.parts
            .iter()
            .enumerate()
            .filter_map(|(slot, play)| {
                let play = play.as_ref()?;
                let clip = self.part_clips.get(slot, play.index as usize)?;
                let age = play.age + alpha.clamp(0., 1.);
                Some(crate::animation::Overlay {
                    clip,
                    frame: age,
                    weight: crate::animation::part_blend(age, play.blend as f32, clip.frames),
                })
            })
            .collect()
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
    /// Animated TObj SRT relative to the matrix already applied to bind UVs.
    pub fn texture_matrix(&self, batch: usize) -> Mat4 {
        let b = &self.model.batches[batch];
        let Some(base) = b.texture_transform else { return Mat4::IDENTITY };
        let mut animated = base;
        let mut changed = false;
        for track in &self.material_tracks {
            if track.joint != b.material_dobj { continue; }
            let Some(value) = track.sample(self.frame) else { continue };
            match track.channel {
                102 => animated.translation.x = value,
                103 => animated.translation.y = value,
                104 => animated.scale.x = value,
                105 => animated.scale.y = value,
                106..=108 => animated.rotation[(track.channel - 106) as usize] = value,
                _ => continue,
            }
            changed = true;
        }
        let matrix = base.matrix();
        if !changed || matrix.determinant().abs() < 1e-8 { return Mat4::IDENTITY; }
        animated.matrix() * matrix.inverse()
    }
    pub fn texture_frame(&self, batch: usize) -> Option<usize> {
        let b = self.model.batches.get(batch)?;
        if b.texture_frames.is_empty() { return None; }
        // A face TObj shows the image its track gives at the frame the script set.
        if let Some((k, (_, track))) = self.face_tracks.iter().enumerate().find(|(_, (bi, _))| *bi == batch) {
            let image = track.sample(self.face[k.min(self.face.len() - 1)]).unwrap_or(0.).max(0.) as usize;
            return Some(image.min(b.texture_frames.len() - 1));
        }
        let frame = self.material_tracks.iter().filter(|t| t.joint == b.material_dobj && t.channel == 101)
            .filter_map(|t| t.sample(self.frame)).last().unwrap_or(0.).max(0.) as usize;
        Some(frame.min(b.texture_frames.len() - 1))
    }
    pub fn texture(&self, batch: usize) -> Option<&crate::model::Texture> {
        let b = self.model.batches.get(batch)?;
        self.texture_frame(batch).map(|i| &b.texture_frames[i]).or(b.texture.as_ref())
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
            // Adaptation, not decomp behaviour: a common effect authored with MODULATE alpha and
            // an animated blend value (the Screw Attack's ground dust ring) would otherwise hold
            // its full size at full opacity until it is removed, which reads as a static ring.
            // Dim it with the authored blend value and fade it out over its last 10 frames.
            if self.effect_fade
                && b.texture_alpha == 3
                && self.duration > 0.
                && self
                    .material_tracks
                    .iter()
                    .any(|t| t.joint == b.material_dobj && t.channel == 109)
            {
                let tail = 10_f32.min(self.duration);
                color[3] *= blending * ((self.duration - self.frame) / tail).clamp(0., 1.);
            }
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
    mobile_pose_over(model, animations, sim, action_frame, upper_root, &[])
}
/// [`mobile_pose`] with part animations laid over the action's pose.
pub fn mobile_pose_over(
    model: &Model,
    animations: &Animations,
    sim: &Sim,
    action_frame: f32,
    upper_root: Option<usize>,
    overlay: &[crate::animation::Overlay<'_>],
) -> Vec<Mat4> {
    let mut pose = mobile_pose_layers_over(
        model,
        animations,
        sim.motion.clip,
        action_frame,
        sim.motion.looping(),
        sim.locomotion.clip,
        sim.locomotion.render_frame(sim.alpha()),
        sim.locomotion.looping(),
        sim.gait_layer_weight(),
        upper_root,
        overlay,
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
    mobile_pose_layers_over(
        model, animations, action_clip, action_frame, action_loop, gait_clip, gait_frame,
        gait_loop, weight, upper_root, &[],
    )
}
/// [`mobile_pose_layers`] with part animations laid over the action's pose (the gait only
/// drives the legs, so the hands and face parts always come from the action).
pub fn mobile_pose_layers_over(
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
    overlay: &[crate::animation::Overlay<'_>],
) -> Vec<Mat4> {
    let action = animations.pose_over(model, action_clip, action_frame, action_loop, overlay);
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
/// Undo a clip's animated translation on `joint` and its descendants, keeping the bind
/// translation. A looping clip that travels forward and restarts at zero otherwise makes
/// the model jump back once per pass when the host already owns movement.
fn pin_joint_to_bind(model: &Model, world: &mut [Mat4], joint: usize) {
    let Some(info) = model.joints.get(joint) else { return };
    if joint >= world.len() {
        return;
    }
    let parent = info.parent;
    let parent_world = parent.map_or(Mat4::IDENTITY, |p| world[p]);
    let local = parent_world.inverse() * world[joint];
    let shift = Mat4::from_translation(info.translation - local.w_axis.truncate());
    let world_shift = parent_world * shift * parent_world.inverse();
    for i in 0..world.len() {
        let mut current = Some(i);
        while let Some(j) = current {
            if j == joint {
                world[i] = world_shift * world[i];
                break;
            }
            current = model.joints.get(j).and_then(|j| j.parent);
        }
    }
}
/// Rotate a joint and its descendants about the joint's own origin (the source's
/// `ftPartSetRotX(fighter, joint, angle)`): the egg is skinned to that joint, so the
/// roll spins it about the egg's centre instead of the feet.
fn spin_subtree(model: &Model, world: &mut [Mat4], root: usize, angle: f32) {
    if root >= world.len() {
        return;
    }
    let pivot = world[root].w_axis.truncate();
    let turn = Mat4::from_translation(pivot)
        * Mat4::from_quat(glam::Quat::from_rotation_x(angle))
        * Mat4::from_translation(-pivot);
    for i in 0..world.len() {
        let mut current = Some(i);
        while let Some(j) = current {
            if j == root {
                world[i] = turn * world[i];
                break;
            }
            current = model.joints.get(j).and_then(|joint| joint.parent);
        }
    }
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

/// The costume's material animation: the texture animations of the face TObjs (command 40
/// drives them). The two eye/mouth TObjs are the model's textured DObjs that carry a frame
/// track, in model order; fighters without a face (Captain, Samus) have none.
fn face_animation(
    dir: &Path,
    costume: usize,
    fighter: &crate::fighter::FighterAssets<'_>,
    model: &mut Model,
) -> Result<Vec<(usize, crate::animation::Track)>> {
    let Some(file) = fighter.costumes.get(costume) else { return Ok(Vec::new()) };
    let bytes = std::fs::read(dir.join(file))?;
    let d = crate::dat::Dat::parse(&bytes)?;
    let Some((_, at)) = d.symbols.iter().find(|(s, _)| s.ends_with("_matanim_joint")) else {
        return Ok(Vec::new());
    };
    let tracks = Animations::materials(&d, Some(*at), model)?;
    let mut faces = Vec::new();
    for (index, batch) in model.batches.iter().enumerate() {
        if batch.texture_frames.is_empty() {
            continue;
        }
        if let Some(track) = tracks
            .iter()
            .find(|t| t.joint == batch.material_dobj && t.channel == 101)
        {
            faces.push((index, track.clone()));
        }
    }
    Ok(faces)
}
