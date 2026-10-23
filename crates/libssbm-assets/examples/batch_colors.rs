//! Development-only: each fighter's batch material colours, lit flags and textures (why a model looks dark in a host).
//! `cargo run --release -p libssbm-assets --example batch_colors -- <assets>`
use libssbm_assets::{fighter, model::Model};
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("assets"));
    let all = [("Samus", &fighter::SAMUS), ("Fox", &fighter::FOX), ("Luigi", &fighter::LUIGI), ("Pikachu", &fighter::PIKACHU), ("Falcon", &fighter::CAPTAIN), ("Link", &fighter::LINK), ("Yoshi", &fighter::YOSHI), ("DK", &fighter::DONKEY)];
    for (name, f) in all {
        let Ok(model) = Model::load_for(&dir, 0, f) else { continue };
        let (mut lum, mut n, mut lit) = (0f32, 0, 0);
        let mut rows = Vec::new();
        for b in &model.batches {
            let l = (b.color[0] + b.color[1] + b.color[2]) / 3.;
            lum += l; n += 1; lit += (b.lit != 0) as u32;
            rows.push(format!("({:.2},{:.2},{:.2} a{:.2} lit{} tex{})", b.color[0], b.color[1], b.color[2], b.color[3], b.lit, b.texture.is_some() as u8));
        }
        println!("{name}: {n} batches, {lit} lit, mean diffuse {:.2}\n   {}", lum / n.max(1) as f32, rows.iter().take(12).cloned().collect::<Vec<_>>().join(" "));
    }
}
