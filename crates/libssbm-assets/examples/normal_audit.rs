//! Skins every fighter's model in the Wait pose and reports its normals: how many are non-finite, zero length,
//! or point away from the surface (against the mean direction from the model's centre).
//! cargo run --release -p libssbm-assets --example normal_audit
use glam::Vec3;
use libssbm_assets::{animation::Animations, fighter::*, model::Model, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (name, f) in [("Samus", &SAMUS), ("Fox", &FOX), ("Luigi", &LUIGI), ("Pikachu", &PIKACHU),
        ("Falcon", &CAPTAIN), ("Link", &LINK), ("Yoshi", &YOSHI), ("Donkey", &DONKEY)] {
        if !f.complete(&dir) { println!("SKIP: {name} disc cache absent"); continue; }
        let model = Model::load_for(&dir, 0, f)?;
        let animations = Animations::load_for(&dir, f)?;
        let world = animations.pose(&model, "Wait", 0., false);
        let (mut total, mut bad, mut zero, mut inward) = (0usize, 0usize, 0usize, 0usize);
        let mut all = Vec::new();
        for b in 0..model.batches.len() { all.extend(model.skin(b, &world)); }
        let centre = all.iter().fold(Vec3::ZERO, |a, (p, _)| a + *p) / all.len().max(1) as f32;
        for (p, n) in &all {
            total += 1;
            if !n.is_finite() { bad += 1; continue; }
            if n.length() < 1e-4 { zero += 1; continue; }
            if n.normalize().dot((*p - centre).normalize_or_zero()) < -0.2 { inward += 1; }
        }
        println!("{name}: {total} vertices, {bad} non-finite normals, {zero} zero, {inward} pointing inward");
    }
    Ok(())
}
