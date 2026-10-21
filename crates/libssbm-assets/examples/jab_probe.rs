//! Prints each fighter's jab tuning (the combo windows and the rapid-jab press count) and plays a
//! tap pattern through the sim, listing the moves it went through.
//!   cargo run --release -p libssbm-assets --example jab_probe [period] [taps]
use glam::Vec3;
use libssbm::{
    sim::{FlatHost, ATTACK},
    Input, Move, Sim,
};
use libssbm_assets::{animation::Animations, fighter::*, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let args: Vec<usize> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let period = args.first().copied().unwrap_or(6);
    let taps = args.get(1).copied().unwrap_or(8);
    for (name, f) in [
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
        let params = libssbm_assets::params::load_common(&dir, f)?;
        println!(
            "{name}: jab2 window {} jab3 window {} rapid presses {}",
            params.jab_2_window, params.jab_3_window, params.rapid_jab_window
        );
        let mut sim = Sim::new(params, Vec3::ZERO);
        let animations = Animations::load_for(&dir, f)?;
        sim.motion.definitions = animations.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, f)?;
        for (n, action) in &sim.action.definitions {
            if let Some(m) = sim.motion.definitions.get_mut(n) {
                m.looping |= action.looping;
            }
        }
        sim.grounded = true;
        sim.facing = Vec3::X;
        let mut host = FlatHost::default();
        for _ in 0..4 {
            sim.advance(&mut host, Input::idle(), 1. / 60.);
        }
        let mut trace: Vec<String> = Vec::new();
        let mut last: Option<Move> = None;
        for tick in 0..(period * taps + 150) {
            // One tick down, the rest up, every `period` ticks, for `taps` taps.
            let down = tick < period * taps && tick % period == 0;
            let input = Input { buttons: if down { ATTACK } else { 0 }, aim: Vec3::X, ..Input::idle() };
            sim.advance(&mut host, input, 1. / 60.);
            if sim.current_move != last {
                trace.push(format!("{tick}:{}", sim.current_move.map_or("-".into(), |m| format!("{m:?}"))));
                last = sim.current_move;
            }
        }
        println!("  {}", trace.join(" "));
    }
    Ok(())
}
