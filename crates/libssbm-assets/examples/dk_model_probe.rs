//! Identifies Donkey Kong's model DObjs (is the tie a separate model?) and prints each one's
//! bind and Wait-pose bounding box, so an orientation bug can be pinned to an exact DObj/joint.
//! cargo run --release -p libssbm-assets --example dk_model_probe
use glam::Vec3;
use libssbm_assets::{animation::Animations, fighter::DONKEY, model::Model, Result};

fn bounds(model: &Model, world: &[glam::Mat4], batch: usize) -> (Vec3, Vec3) {
    let pts = model.skin(batch, world);
    let lo = pts.iter().fold(Vec3::splat(f32::INFINITY), |a, (p, _)| a.min(*p));
    let hi = pts.iter().fold(Vec3::splat(f32::NEG_INFINITY), |a, (p, _)| a.max(*p));
    (lo, hi)
}

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !DONKEY.complete(&dir) {
        println!("SKIP: Donkey disc cache absent");
        return Ok(());
    }
    let model = Model::load_for(&dir, 0, &DONKEY)?;
    let animations = Animations::load_for(&dir, &DONKEY)?;
    println!("DK: {} joints, {} batches", model.joints.len(), model.batches.len());
    let bind = model.bind_world();
    let wait = animations.pose(&model, "Wait", 0., false);
    // Compare bind rot.x vs Wait clip rot.x for head (43-45) and tie (66-70) joints.
    let wait_clip = animations.clips.get("Wait").unwrap();
    for j in [43usize, 44, 45, 66, 67, 68, 69, 70] {
        let br = model.joints[j].rotation.x;
        let w = wait_clip.tracks.iter().find(|t| t.joint == j && t.channel == 1).and_then(|t| t.sample(0.));
        println!("joint {j:2} bind rot.x {:>8.4}  Wait rot.x {:?}", br, w);
    }
    // The tie chain joints 66..70: what the Wait clip keys them to.
    if let Some(clip) = animations.clips.get("Wait") {
        println!("Wait clip: {} tracks, {} frames", clip.tracks.len(), clip.frames);
        // Sample joint 66's X rotation across the whole clip to see the sway range.
        for track in &clip.tracks {
            if track.joint == 66 && track.channel == 1 {
                let mut min = f32::INFINITY;
                let mut max = f32::NEG_INFINITY;
                let mut count = 0;
                for f in 0..(clip.frames as u32) {
                    if let Some(v) = track.sample(f as f32) {
                        min = min.min(v);
                        max = max.max(v);
                        count += 1;
                    }
                }
                println!("  Wait joint66 rot.x range over {count} frames: {min:.3} .. {max:.3}");
            }
        }
    }
    if let Some(clip) = animations.clips.get("Walk1") {
        for track in &clip.tracks {
            if track.joint == 66 && track.channel == 1 {
                let mut min = f32::INFINITY;
                let mut max = f32::NEG_INFINITY;
                for f in 0..(clip.frames as u32) {
                    if let Some(v) = track.sample(f as f32) {
                        min = min.min(v);
                        max = max.max(v);
                    }
                }
                println!("Walk1 clip {} frames, joint66 rot.x range: {min:.3} .. {max:.3}", clip.frames);
            }
        }
    }
    for track in &animations.clips.get("Wait").unwrap().tracks {
        if (66..=70).contains(&track.joint) {
            println!("  Wait track joint {} channel {} @0 = {:?}", track.joint, track.channel, track.sample(0.));
        }
    }
    // The tie chain is joints 66..70. Find every batch skinned to it (low- and high-detail).
    for (i, b) in model.batches.iter().enumerate() {
        let tie_joints: std::collections::BTreeSet<usize> = b
            .vertices
            .iter()
            .flat_map(|v| v.weights.iter().map(|&(j, _)| j))
            .filter(|j| (66..=70).contains(j))
            .collect();
        if !tie_joints.is_empty() {
            let (blo, bhi) = bounds(&model, &bind, i);
            let (wlo, whi) = bounds(&model, &wait, i);
            println!(
                "TIE batch {i} dobj {:2} verts {} visible {} owner {} joints {:?} | bind size ({:?}) Wait size ({:?})",
                b.dobj, b.vertices.len(), model.normal_visible[b.dobj], b.owner, tie_joints,
                bhi - blo, whi - wlo,
            );
        }
    }
    // The tie (dobj 33): which bones its 7 vertices are weighted to.
    if let Some((i, _)) = model.batches.iter().enumerate().find(|(_, b)| b.dobj == 33) {
        let mut joints: std::collections::BTreeMap<usize, (usize, f32)> = Default::default();
        for v in &model.batches[i].vertices {
            for &(j, w) in &v.weights {
                joints.entry(j).and_modify(|e| e.1 += w).or_insert((j, w));
            }
        }
        println!("tie (dobj 33) owner {} weighted joints: {:?}", model.batches[i].owner, joints);
    }
    // Every joint's bind translation, to locate the tie bone near the neck.
    for i in [26usize, 66, 67, 68, 69, 70] {
        let j = &model.joints[i];
        println!("joint {i} flags {:#010x} parent {:?} rot ({:?})", j.flags, j.parent, j.rotation);
    }
    for (i, j) in model.joints.iter().enumerate() {
        if j.bind.w_axis.y > 12. && j.bind.w_axis.y < 30. {
            println!("joint {i:2} parent {:?} bind ({:>6.1?}) rot ({:>6.2?})", j.parent, j.bind.w_axis.truncate(), j.rotation);
        }
    }
    for (i, b) in model.batches.iter().enumerate() {
        let (blo, bhi) = bounds(&model, &bind, i);
        let (wlo, whi) = bounds(&model, &wait, i);
        let bsize = bhi - blo;
        let wsize = whi - wlo;
        println!(
            "batch {i:2} dobj {:2} owner {:2} verts {:5} visible {} | bind centre ({:>6.1?}) size ({:>6.1?}) | Wait centre ({:>6.1?}) size ({:>6.1?})",
            b.dobj, b.owner, b.vertices.len(), model.normal_visible[b.dobj],
            (blo + bhi) * 0.5, bsize,
            (wlo + whi) * 0.5, wsize,
        );
    }
    Ok(())
}
