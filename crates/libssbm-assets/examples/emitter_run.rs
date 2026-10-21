//! Runs a common-effect generator and prints how many particles it has, how big and how far
//! they spread: `emitter_run 588 120`.
use libssbm_assets::{effects::Effects, fighter::COMMON_EFFECTS, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let id: u32 = args[0].parse()?;
    let ticks: usize = args.get(1).map_or(120, |a| a.parse().unwrap_or(120));
    let bank = Effects::load_archive(&dir, COMMON_EFFECTS)?;
    let mut system = libssbm::particle::System::default();
    system.definitions = bank.emitters.clone();
    system.spawn(id, glam::Mat4::IDENTITY, 1)?;
    for t in 0..ticks {
        system.tick();
        if t % 10 == 0 || !system.active() {
            let n = system.particles.len();
            let (mut min, mut max) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
            let mut size = 0f32;
            let mut alpha = 0f32;
            for p in &system.particles {
                min = min.min(p.position);
                max = max.max(p.position);
                size = size.max(p.size);
                alpha = alpha.max(p.color.w);
            }
            println!("tick {t}: {n} particles, max size {size:.1}, max alpha {alpha:.2}, span {:?}", if n > 0 { max - min } else { glam::Vec3::ZERO });
        }
        if !system.active() {
            println!("ended at tick {t}");
            break;
        }
    }
    Ok(())
}
