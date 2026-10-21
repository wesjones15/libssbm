//! The face's texture animation (script command 40): a swinging fighter's sim sets face frames
//! and the presentation shows a different image for them (skipped without the disc cache).
use glam::Vec3;
use libssbm::{fox::Fox, sim::FlatHost, Input, Sim};
use libssbm_assets::{Result, animation::Animations, fighter::FOX, presentation::Presentation};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

#[test]
fn a_swing_changes_the_face() -> Result<()> {
    let dir = dir();
    if !FOX.complete(&dir) {
        eprintln!("SKIP: Fox disc cache absent");
        return Ok(());
    }
    let params = libssbm_assets::params::load_common(&dir, &FOX)?;
    let mut sim = Sim::with_behavior(
        params,
        Box::new(Fox::new(libssbm_assets::params::load_fox_specials(&dir)?)),
        Vec3::ZERO,
    );
    let animations = Animations::load_for(&dir, &FOX)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &FOX)?;
    sim.grounded = true;
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    let mut presentation = Presentation::load_for(&dir, 0, &FOX)?;
    assert_eq!(presentation.face_tracks.len(), 2, "Fox has two face TObjs");
    presentation.update(&sim, 0.);
    let calm: Vec<_> = presentation.face_tracks.iter().map(|(b, _)| presentation.texture_frame(*b)).collect();
    // A swing's script narrows his eyes (command 40, frame 4): an aerial forward attack.
    sim.grounded = false;
    sim.position = Vec3::new(0., 200., 0.);
    let swing = Input { movement: Vec3::X, buttons: libssbm::sim::ATTACK, ..Input::idle() };
    let mut narrowed = None;
    for _ in 0..8 {
        sim.advance(&mut host, swing, 1. / 60.);
        if sim.face.iter().any(|f| *f == 4.) {
            presentation.update(&sim, 0.);
            narrowed = Some(presentation.face_tracks.iter().map(|(b, _)| presentation.texture_frame(*b)).collect::<Vec<_>>());
            break;
        }
    }
    let narrowed = narrowed.expect("the swing's script never set a face frame");
    assert_ne!(calm, narrowed, "the face looked the same calm and mid-swing");
    Ok(())
}
