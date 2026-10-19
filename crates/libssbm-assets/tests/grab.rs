use glam::Vec3;
use libssbm::{action::Command, *};
use libssbm_assets::{Result, animation::Animations};
#[test]
fn disc_grab_pummel_and_four_throw_release_windows() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    let actions = libssbm_assets::action::load(&dir, &animations)?;
    for fps in [30, 60, 144] {
        for (direction, buttons, expected, damage) in [
            (Vec3::X, ATTACK, Move::ThrowForward, 9.),
            (-Vec3::X, SPECIAL, Move::ThrowBack, 8.),
            (Vec3::Y, ATTACK, Move::ThrowUp, 7.),
            (-Vec3::Y, SPECIAL, Move::ThrowDown, 6.),
            (Vec3::X, ATTACK | UP, Move::ThrowUp, 7.),
            (Vec3::Y, ATTACK | DOWN, Move::ThrowDown, 6.),
        ] {
            let mut sim = Sim::new(libssbm_assets::params::load(&dir)?, Vec3::ZERO);
            sim.grounded = true;
            sim.motion.definitions = animations.motion_definitions();
            sim.action.definitions = actions.clone();
            let mut host = FlatHost::default();
            sim.start_move(&mut host, Move::Grab, Input::idle());
            let mut captured = false;
            for _ in 0..fps * 2 {
                sim.advance(&mut host, Input::idle(), 1. / fps as f64);
                captured |= sim
                    .action
                    .samples
                    .iter()
                    .any(|s| s.hitbox.element == 8 && s.hitbox.damage == 0.);
            }
            assert!(captured, "source grab windows absent");
            sim.set_grabbed(true);
            for _ in 0..fps * 2 {
                sim.advance(&mut host, Input::idle(), 1. / fps as f64);
            }
            assert_eq!(sim.current_move, Some(Move::GrabHold));
            let input = Input {
                movement: direction,
                buttons,
                ..Input::idle()
            };
            sim.advance(&mut host, input, 1. / 60.);
            assert_eq!(sim.current_move, Some(expected));
            for _ in 0..fps * 2 {
                sim.advance(&mut host, Input::idle(), 1. / fps as f64);
            }
            let throws: Vec<_> = host
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Throw)
                .collect();
            assert_eq!(
                throws.len(),
                1,
                "duplicate/missing source release at {fps} Hz"
            );
            assert_eq!(throws[0].strength, damage);
            assert!(!sim.grabbed);
        }
    }
    for part in 0..8 {
        let mut p = libssbm_assets::grapple::part(&dir, part)?;
        assert!(!p.model.batches.is_empty());
        for frame in [0., 10., 20., 40.] {
            p.object_pose(frame);
            for i in 0..p.model.batches.len() {
                assert!(
                    p.model
                        .skin(i, &p.world)
                        .iter()
                        .all(|(p, n)| p.is_finite() && n.is_finite())
                );
            }
        }
    }
    assert!(
        actions["CatchAttack"]
            .commands
            .iter()
            .any(|c| matches!(&c.command,Command::Hitbox(h) if h.damage==3.))
    );
    Ok(())
}
#[test]
fn full_charge_storage_is_silent_until_a_real_shot_and_has_source_flash() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    let mut sim = Sim::new(libssbm_assets::params::load(&dir)?, Vec3::ZERO);
    sim.grounded = true;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load(&dir, &animations)?;
    let mut host = FlatHost::default();
    sim.start_move(&mut host, Move::SpecialN, Input::idle());
    for _ in 0..600 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        assert!(
            !sim.action.sounds.iter().any(|s| s.sound == 260021),
            "firing voice played while storing"
        );
    }
    assert_eq!(sim.shot_charge, sim.behavior.charge_steps());
    assert!(!host.events.iter().any(|e| e.kind == EventKind::Hit));
    let cycle = libssbm_assets::params::color_cycle(&dir, 53)?;
    assert!(cycle.iter().any(|c| c[3] > 0.) && cycle.iter().any(|c| c[3] == 0.));
    Ok(())
}
