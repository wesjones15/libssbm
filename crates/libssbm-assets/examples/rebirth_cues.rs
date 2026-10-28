//! Development-only: the GFX cues a real fall-recovery rebirth emits, with their origins, so a
//! misplaced effect (the stage-high KO flash) is visible without the game.
//! `cargo run --release -p libssbm-assets --example rebirth_cues -- <assets> [samus|fox|yoshi] [frames]`
use glam::Vec3;
use libssbm::{effect, FlatHost, Input, Sim};
use libssbm_assets::{fighter, presentation::Presentation, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| "assets".into()));
    let f = match args.get(1).map(String::as_str) {
        Some("yoshi") => &fighter::YOSHI,
        Some("fox") => &fighter::FOX,
        _ => &fighter::SAMUS,
    };
    let frames: u32 = args.get(2).and_then(|n| n.parse().ok()).unwrap_or(90);
    let scale = 0.1_f32; // the host's metres per Melee unit
    let fall = Vec3::new(10., 4.5 / scale, 0.);
    let safe = Vec3::new(12., 0., 0.);
    let mut sim = Sim::new(libssbm_assets::params::load(&dir)?, fall);
    let mut host = FlatHost::default();
    if !sim.begin_rebirth(
        libssbm_assets::params::rebirth(&dir)?,
        libssbm_assets::params::entry(&dir, f)?,
        safe,
    ) {
        return Err("rebirth refused".into());
    }
    println!("fall at {fall:?} ({} m), return to {safe:?}", fall.y * scale);
    for frame in 0..frames {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        for cue in &sim.action.effects {
            let target = effect::resolve(cue.id);
            let kind = match target {
                effect::Target::Model(m) => format!("model {m}"),
                effect::Target::Particle(p) => format!("particle {p}"),
                effect::Target::NoOp => "no-op".into(),
                effect::Target::CameraShake(_) => "shake".into(),
                effect::Target::Unknown => "unknown".into(),
            };
            println!(
                "frame {frame:>2} cue {:#x}: {kind}, origin {:?} (y {:.2} m), param {:.3}, attached {}",
                cue.id,
                cue.origin,
                cue.origin.y * scale,
                cue.param,
                effect::attached(cue.id)
            );
            if let effect::Target::Model(m) = target {
                let view = Presentation::effect(&dir, m)?;
                println!("        model {m}: lifetime {} frames", view.duration);
            }
        }
    }
    Ok(())
}
