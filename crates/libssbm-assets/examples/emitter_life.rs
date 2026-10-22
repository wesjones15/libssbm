//! How long a generator keeps running when nothing stops it: `emitter_life samus 2011 2014`.
//! Prints the tick it ended (or "still running") and the particle count at a few ticks.
use libssbm_assets::{effects::Effects, fighter::*, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let fighter = match args[0].as_str() {
        "samus" => &SAMUS, "fox" => &FOX, "luigi" => &LUIGI, "pikachu" => &PIKACHU,
        "captain" => &CAPTAIN, "yoshi" => &YOSHI, other => panic!("fighter {other}"),
    };
    let bank = Effects::load_archive(&dir, fighter.effects)?;
    for id in args[1..].iter().map(|a| a.parse::<u32>().unwrap()) {
        let mut system = libssbm::particle::System::default();
        system.definitions = bank.emitters.clone();
        system.spawn(id, glam::Mat4::IDENTITY, 1)?;
        let mut end = None;
        let mut counts = Vec::new();
        for t in 0..1800 {
            system.tick();
            if [10, 60, 120, 240, 480, 900, 1500].contains(&t) { counts.push((t, system.particles.len())); }
            if !system.active() { end = Some(t); break; }
        }
        println!("{id}: ended {end:?}, particle counts {counts:?}");
    }
    Ok(())
}
