//! Headless check: how far are each fighter's foot meshes from Y=0 in its native clips?
//! Run from `libssbm/`: cargo run -p libssbm-assets --release --example foot_audit
//!
//! A "foot batch" is a DObj owned by a foot joint or one of its descendants (toe).
//! Prints the lowest skinned foot vertex across sampled frames of each clip.
use libssbm_assets::{
    animation::Animations,
    dat::Dat,
    fighter::{CAPTAIN, DONKEY, FOX, FighterAssets, LINK, LUIGI, PIKACHU, SAMUS, YOSHI},
    model::Model,
};
use std::collections::BTreeSet;
use std::path::Path;

fn foot_joints(common: &Dat, f: &FighterAssets) -> Vec<usize> {
    let table = common.required_ptr(common.root("ftLoadCommonData").unwrap() + 0x10).unwrap();
    let parts = common.required_ptr(table + f.common_index * 4).unwrap();
    let map = common.required_ptr(parts + 4).unwrap();
    let b = common.bytes(map, 53).unwrap();
    // FtPart_LFootJA/J (9/10), RFootJA/J (14/15).
    [9usize, 10, 14, 15]
        .iter()
        .map(|&i| b[i] as usize)
        .filter(|&j| j != 255)
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dir = Path::new("assets");
    let common_bytes = std::fs::read(dir.join("PlCo.dat"))?;
    let common = Dat::parse(&common_bytes)?;
    let fighters: [(&str, &FighterAssets); 8] = [
        ("samus", &SAMUS),
        ("fox", &FOX),
        ("luigi", &LUIGI),
        ("pikachu", &PIKACHU),
        ("captain", &CAPTAIN),
        ("link", &LINK),
        ("yoshi", &YOSHI),
        ("donkey", &DONKEY),
    ];
    for (name, f) in fighters {
        if !f.complete(dir) {
            println!("{name}: absent");
            continue;
        }
        let model = Model::load_for(dir, 0, f)?;
        let animations = Animations::load_for(dir, f)?;
        let feet = foot_joints(&common, f);
        let mut under = BTreeSet::new();
        for &foot in &feet {
            for j in 0..model.joints.len() {
                let mut at = Some(j);
                while let Some(i) = at {
                    if i == foot {
                        under.insert(j);
                    }
                    at = model.joints[i].parent;
                }
            }
        }
        let batches: Vec<usize> = model
            .batches
            .iter()
            .enumerate()
            .filter(|(_, b)| under.contains(&b.owner))
            .map(|(i, _)| i)
            .collect();
        print!(
            "{name:<8} feet {feet:?} batches {} |",
            batches.len()
        );
        // Whole-model minimum in the idle: this is what a host placing the model
        // at its root would float by (the feet anchor).
        let idle = if animations.clips.contains_key("Wait") {
            "Wait"
        } else {
            "Wait1"
        };
        if let Some(c) = animations.clips.get(idle) {
            let world = animations.pose(&model, idle, 0., false);
            let mut low = f32::INFINITY;
            for bi in 0..model.batches.len() {
                for (p, _) in model.skin(bi, &world) {
                    low = low.min(p.y);
                }
            }
            print!(" idleMin {low:+.2}");
        }
        for clip in ["Wait", "WalkMiddle", "Run", "Squat", "Attack11", "SpecialHi"] {
            let Some(c) = animations.clips.get(clip) else {
                print!(" {clip}:--");
                continue;
            };
            let mut low = f32::INFINITY;
            let mut high = f32::NEG_INFINITY;
            let steps = 12;
            for k in 0..=steps {
                let frame = c.frames * k as f32 / steps as f32;
                let world = animations.pose(&model, clip, frame, false);
                for &bi in &batches {
                    for (p, _) in model.skin(bi, &world) {
                        low = low.min(p.y);
                        high = high.max(p.y);
                    }
                }
            }
            print!(" {clip}:[{low:+.2},{high:+.2}]");
        }
        println!();
    }
    Ok(())
}
