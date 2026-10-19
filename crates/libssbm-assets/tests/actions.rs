use glam::Vec3;
use libssbm::{EventKind, FlatHost, Input, Move, Sim};
use libssbm_assets::{Result, animation::Animations};
use std::path::PathBuf;

#[test]
fn real_disc_attacks_keep_all_hit_windows_at_different_host_rates() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    let definitions = libssbm_assets::action::load(&dir, &animations)?;
    let params = libssbm_assets::params::load(&dir)?;
    for (movement, airborne, expected) in [
        (Move::Jab, false, "Attack11"),
        (Move::Jab2, false, "Attack12"),
        (Move::SideTilt, false, "AttackS3S"),
        (Move::UpTilt, false, "AttackHi3"),
        (Move::DownTilt, false, "AttackLw3"),
        (Move::DashAttack, false, "AttackDash"),
        (Move::SideSmash, false, "AttackS4S"),
        (Move::UpSmash, false, "AttackHi4"),
        (Move::DownSmash, false, "AttackLw4"),
        (Move::NeutralAir, true, "AttackAirN"),
        (Move::ForwardAir, true, "AttackAirF"),
        (Move::BackAir, true, "AttackAirB"),
        (Move::UpAir, true, "AttackAirHi"),
        (Move::DownAir, true, "AttackAirLw"),
        (Move::ScrewAttack, false, "SpecialHi"),
        (Move::ScrewAttack, true, "SpecialAirHi"),
    ] {
        let mut reference = None;
        for fps in [30, 60, 120] {
            let mut sim = Sim::new(params.clone(), Vec3::Y * if airborne { 1000. } else { 0. });
            sim.grounded = !airborne;
            sim.motion.definitions = animations.motion_definitions();
            sim.action.definitions = definitions.clone();
            let mut host = FlatHost::default();
            assert!(sim.start_move(&mut host, movement, Input::idle()));
            let mut samples = Vec::new();
            for _ in 0..fps * 4 {
                sim.advance(&mut host, Input::idle(), 1. / fps as f64);
                for sample in &sim.action.samples {
                    assert_eq!(sample.clip, expected);
                    samples.push((
                        sample.frame.to_bits(),
                        sample.generation,
                        sample.hitbox.id,
                        sample.hitbox.damage.to_bits(),
                    ));
                }
            }
            assert!(
                !samples.is_empty(),
                "No runtime hitboxes for {movement:?} at {fps} Hz"
            );
            assert!(
                !host.events.iter().any(|e| e.kind == EventKind::Hit),
                "Placeholder melee event for {movement:?}"
            );
            if let Some(reference) = &reference {
                assert_eq!(
                    &samples, reference,
                    "Hit windows differ: {movement:?}, {fps} Hz"
                );
            } else {
                reference = Some(samples);
            }
        }
    }
    Ok(())
}

#[test]
fn real_disc_special_scripts_fire_one_projectile() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    for movement in [
        Move::ChargeShot,
        Move::Missile,
        Move::SuperMissile,
        Move::Bomb,
    ] {
        for airborne in [false, true] {
            let mut sim = Sim::new(
                libssbm_assets::params::load(&dir)?,
                Vec3::Y * if airborne { 1000. } else { 0. },
            );
            sim.grounded = !airborne;
            sim.shot_charge = sim.behavior.charge_steps();
            sim.motion.definitions = animations.motion_definitions();
            sim.action.definitions = libssbm_assets::action::load(&dir, &animations)?;
            let mut host = FlatHost::default();
            sim.start_move(&mut host, movement, Input::idle());
            for _ in 0..240 {
                sim.advance(&mut host, Input::idle(), 1. / 60.);
            }
            assert_eq!(
                host.events
                    .iter()
                    .filter(|e| e.kind == EventKind::Hit)
                    .count(),
                1,
                "{movement:?}, airborne={airborne}"
            );
        }
    }
    Ok(())
}
