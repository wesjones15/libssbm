//! Prints the numbers of one or more common-effect emitters: `emitter_dump 588 589`.
use libssbm_assets::{effects::Effects, fighter::COMMON_EFFECTS, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let effects = Effects::load_archive(&dir, COMMON_EFFECTS)?;
    for id in std::env::args().skip(1).filter_map(|a| a.parse::<u32>().ok()) {
        match effects.emitters.get(&id) {
            Some(e) => println!(
                "{id}: kind {} shape {} texture {} generator_life {} life {} gravity {} friction {} velocity {:?} radius {} angle {} random {} size {} params {:?} commands {}",
                e.kind, e.shape, e.texture, e.generator_life, e.life, e.gravity, e.friction, e.velocity, e.radius, e.angle, e.random, e.size, e.params, e.commands.len()
            ),
            None => println!("{id}: none"),
        }
    }
    Ok(())
}
