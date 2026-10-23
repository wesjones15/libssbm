//! Poses every clip of a fighter at every frame on costume 0 and reports any skinned vertex that is not finite
//! (a NaN there is Unity's "abnormal mesh bounds"). `cargo run --release -p libssbm-assets --example finite_audit -- <assets> [kind]`
use libssbm_assets::{animation::Animations, fighter, model::Model, presentation::{blend_pose, splice_pose}};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().expect("assets dir"));
    let kind: usize = args.next().map_or(0, |k| k.parse().unwrap());
    let fighters = [
        &fighter::SAMUS, &fighter::FOX, &fighter::LUIGI, &fighter::PIKACHU,
        &fighter::CAPTAIN, &fighter::LINK, &fighter::YOSHI, &fighter::DONKEY,
    ];
    let f = fighters[kind];
    let animations = Animations::load_for(&dir, f).expect("animations");
    let model = Model::load_for(&dir, 0, f).expect("model");
    let (mut bad, mut checked) = (0, 0);
    for (name, clip) in &animations.clips {
        let frames = clip.frames.ceil() as usize;
        let mut first = None;
        for frame in 0..=frames {
            let pose = animations.pose(&model, name, frame as f32, false);
            checked += 1;
            let finite_pose = pose.iter().all(|m| m.to_cols_array().iter().all(|v| v.is_finite()));
            let finite_skin = (0..model.batches.len())
                .all(|i| model.skin(i, &pose).iter().all(|(p, n)| p.is_finite() && n.is_finite()));
            if !finite_pose || !finite_skin {
                first = Some((frame, finite_pose));
                break;
            }
        }
        if let Some((frame, pose_ok)) = first {
            bad += 1;
            println!("NON-FINITE clip {name} at frame {frame} (pose finite: {pose_ok})");
        }
    }
    // The mobile-attack layering: each clip as the action over a walking gait, spliced and blended.
    let mut layered_bad = 0;
    if let Some(root) = f.upper_body_root {
        let gaits: Vec<&String> = animations.clips.keys().filter(|k| k.starts_with("Walk") || *k == "Run").collect();
        for (name, clip) in &animations.clips {
            let frames = clip.frames.ceil() as usize;
            'clip: for frame in (0..=frames).step_by(2) {
                let action = animations.pose(&model, name, frame as f32, false);
                for gait in &gaits {
                    let lower = animations.pose(&model, gait, 5., true);
                    let combined = splice_pose(&model, &lower, &action, root);
                    let mixed = blend_pose(&model, &action, &combined, 0.7);
                    let finite = |pose: &[glam::Mat4]| (0..model.batches.len())
                        .all(|i| model.skin(i, pose).iter().all(|(p, n)| p.is_finite() && n.is_finite()));
                    if !finite(&combined) || !finite(&mixed) {
                        layered_bad += 1;
                        println!("NON-FINITE layered: action {name} frame {frame} over {gait} (splice finite: {})", finite(&combined));
                        break 'clip;
                    }
                }
            }
        }
    }
    println!("{} clips, {checked} poses checked, {bad} clips with non-finite output, {layered_bad} clips non-finite when layered over a gait", animations.clips.len());
}
