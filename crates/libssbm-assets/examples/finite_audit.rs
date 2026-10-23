//! Poses every clip of a fighter at every frame on costume 0 and reports any skinned vertex that is not finite
//! (a NaN there is Unity's "abnormal mesh bounds"). `cargo run --release -p libssbm-assets --example finite_audit -- <assets> [kind]`
use libssbm_assets::{animation::Animations, fighter, model::Model};
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
    println!("{} clips, {checked} poses checked, {bad} clips with non-finite output", animations.clips.len());
}
