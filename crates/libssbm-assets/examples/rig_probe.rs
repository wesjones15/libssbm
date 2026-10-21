//! How alike are the fighters' rigs? Prints each fighter's joint count, parent-chain depth and the
//! number of tracks in its Wait and jab clips, and how many joints its jab hitboxes address.
use libssbm::action::Command;
use libssbm_assets::{animation::Animations, fighter::*, model::Model, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (name, f) in [
        ("samus", &SAMUS),
        ("fox", &FOX),
        ("luigi", &LUIGI),
        ("pikachu", &PIKACHU),
        ("captain", &CAPTAIN),
        ("link", &LINK),
        ("yoshi", &YOSHI),
        ("donkey", &DONKEY),
    ] {
        if !f.complete(&dir) {
            println!("{name}: absent");
            continue;
        }
        let model = Model::load_for(&dir, 0, f)?;
        let animations = Animations::load_for(&dir, f)?;
        let actions = libssbm_assets::action::load_for(&dir, &animations, f)?;
        let tracks = |c: &str| animations.clips.get(c).map_or(0, |c| c.tracks.len());
        let mut bones = std::collections::BTreeSet::new();
        for clip in ["Attack11", "AttackS3S", "AttackHi4", "AttackAirN"] {
            if let Some(a) = actions.get(clip) {
                for e in &a.commands {
                    if let Command::Hitbox(h) = &e.command {
                        bones.insert((h.bone, h.common_bone));
                    }
                }
            }
        }
        println!(
            "{name}: {} joints, Wait {} tracks, Attack11 {} tracks, upper-body root {:?}, jab/tilt/smash/air hitbox bones {:?}",
            model.joints.len(),
            tracks("Wait"),
            tracks("Attack11"),
            f.upper_body_root,
            bones
        );
    }
    Ok(())
}
