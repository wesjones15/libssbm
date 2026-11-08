//! Authored stand-ins. No disc data; model IDs/cues intentionally do not apply.
use crate::{
    animation::Animations,
    model::{Batch, Joint, Model, Vertex},
    presentation::Presentation,
};
use glam::{Mat4, Vec2, Vec3};

pub fn model(color: [f32; 4], object: bool) -> Model {
    let mut joints = Vec::new();
    // Fixed body-relative anchors support native attachment APIs without a disc skeleton.
    for i in 0..256 {
        let translation = if i == 0 {
            Vec3::ZERO
        } else {
            Vec3::new(0., 8., 3.)
        };
        let bind = Mat4::from_translation(translation);
        joints.push(Joint {
            flags: 0,
            address: 0,
            parent: None,
            translation,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
            bind,
            inverse: bind.inverse(),
            envelope: None,
        });
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let (radius, height) = if object { (1.5, 3.) } else { (3., 16.) };
    for i in 0..12 {
        let a = i as f32 * std::f32::consts::TAU / 12.;
        let normal = Vec3::new(a.cos(), 0., a.sin());
        for y in [0., height] {
            vertices.push(Vertex {
                position: normal * radius + Vec3::Y * y,
                normal,
                uv: Vec2::ZERO,
                color: [1.; 4],
                weights: vec![(0, 1.)],
                envelope: false,
            });
        }
        let n = i * 2;
        let next = (n + 2) % 24;
        indices.extend([n, next, n + 1, n + 1, next, next + 1]);
    }
    for i in 1..11 {
        indices.extend([0, i * 2 + 2, i * 2, 1, i * 2 + 1, i * 2 + 3]);
    }
    let batch = Batch {
        texture_transform: None,
        texture_frames: Vec::new(),
        instance: None,
        material_dobj: 0,
        vertices,
        indices,
        color,
        texture: None,
        dobj: 0,
        owner: 0,
        blend: 0,
        lit: 1,
        texture_alpha: 0,
        texture_blend: 0.,
    };
    let mut ball = batch.clone();
    ball.dobj = 1;
    for vertex in &mut ball.vertices {
        vertex.position.x *= 1.5;
        vertex.position.z *= 1.5;
        vertex.position.y *= 0.55;
    }
    Model {
        joints,
        batches: vec![batch, ball],
        normal_visible: vec![true, false],
        ball_visible: vec![false, true],
    }
}
pub fn presentation(kind: u32, object: bool) -> Presentation {
    let palette = [
        [1., 0.55, 0.05, 1.],
        [0.8, 0.8, 0.8, 1.],
        [0.15, 0.8, 0.2, 1.],
        [1., 0.85, 0.05, 1.],
        [0.15, 0.3, 1., 1.],
        [0.2, 0.65, 0.15, 1.],
        [0.25, 0.9, 0.35, 1.],
        [0.55, 0.3, 0.1, 1.],
    ];
    let model = model(
        if object {
            [0.1, 0.65, 1., 1.]
        } else {
            palette[kind as usize % palette.len()]
        },
        object,
    );
    let world = model.bind_world();
    Presentation {
        attach_joint: object.then_some(0),
        model,
        animations: Animations {
            clips: Default::default(),
            rigid: Vec::new(),
        },
        clip: String::new(),
        frame: 0.,
        world,
        ball: false,
        morph_move: match kind {
            0 => Some(libssbm::Move::SpecialLw),
            6 => Some(libssbm::Move::SpecialS),
            _ => None,
        },
        spin_joint: None,
        hidden: false,
        material_tracks: Vec::new(),
        duration: 30.,
        commands: Vec::new(),
        object_scale: 1.,
        upper_body_root: None,
        effect_fade: false,
        face_tracks: Vec::new(),
        face: [0.; 8],
        part_clips: Default::default(),
        parts: [None; 5],
    }
}
