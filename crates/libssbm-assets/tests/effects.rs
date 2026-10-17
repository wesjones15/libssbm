use libssbm_assets::{Result, effects::Effects};
use std::path::PathBuf;
#[test]
fn real_effect_models_emitters_and_all_texture_frames_decode() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() {
        eprintln!("SKIP: effect archives absent");
        return Ok(());
    }
    for samus in [false, true] {
        let bank = Effects::load(&dir, samus).map_err(|e| format!("effects samus={samus}: {e}"))?;
        assert!(!bank.models.is_empty());
        assert!(!bank.emitters.is_empty());
        for model in &bank.models {
            for frame in [0., model.lifetime * 0.5, model.lifetime] {
                assert!(
                    model
                        .animations
                        .pose_object(&model.model, frame)
                        .iter()
                        .all(|m| m.is_finite())
                );
            }
        }
        for group in bank.textures.iter().flatten() {
            for frame in group.palettes.iter().flatten() {
                assert_eq!(
                    frame.rgba.len(),
                    frame.width as usize * frame.height as usize * 4
                );
            }
        }
        eprintln!(
            "effects samus={samus}: {} models, {} emitters, {} groups",
            bank.models.len(),
            bank.emitters.len(),
            bank.textures.len()
        );
    }
    Ok(())
}
#[test]
fn impact_particles_keep_transparent_texture_borders() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() {
        return Ok(());
    }
    let bank = Effects::load(&dir, true)?;
    let mut system = libssbm::particle::System::default();
    system.definitions = bank.emitters.clone();
    system.spawn(2007, glam::Mat4::IDENTITY, 1)?;
    for _ in 0..8 {
        system.tick();
    }
    for p in &system.particles {
        let texture = &bank.textures[p.texture as usize].as_ref().unwrap().frames[p.frame as usize];
        assert_eq!(texture.rgba[3], 0, "impact texture border became opaque");
    }
    Ok(())
}
#[test]
fn audit_active_motion_effect_dependencies() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() {
        return Ok(());
    }
    let animations = libssbm_assets::animation::Animations::load(&dir)?;
    let actions = libssbm_assets::action::load(&dir, &animations)?;
    let mut ids = std::collections::BTreeSet::new();
    for name in [
        "Wait",
        "WalkSlow",
        "WalkMiddle",
        "WalkFast",
        "Run",
        "Dash",
        "JumpF",
        "JumpAerialF",
        "Landing",
        "Attack11",
        "Attack12",
        "AttackDash",
        "AttackS3S",
        "AttackHi3",
        "AttackLw3",
        "AttackS4S",
        "AttackHi4",
        "AttackLw4",
        "AttackAirN",
        "AttackAirF",
        "AttackAirB",
        "AttackAirHi",
        "AttackAirLw",
        "SpecialNStart",
        "SpecialNHold",
        "SpecialN",
        "SpecialAirN",
        "SpecialS",
        "SpecialSSmash",
        "SpecialHi",
        "SpecialAirHi",
        "SpecialLw",
        "SpecialAirLw",
    ] {
        if let Some(action) = actions.get(name) {
            for e in &action.commands {
                if let libssbm::action::Command::Raw { opcode: 10, words } = &e.command {
                    ids.insert(words[1] >> 16);
                }
            }
        }
    }
    eprintln!("active motion effect IDs: {ids:?}");
    assert!(
        ids.iter()
            .all(|id| libssbm::effect::resolve(*id) != libssbm::effect::Target::Unknown),
        "unresolved movement/attack GFX dependency"
    );
    Ok(())
}
#[test]
fn every_samus_emitter_executes_its_source_commands_without_placeholders() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() {
        eprintln!("SKIP: effect archives absent");
        return Ok(());
    }
    let bank = Effects::load(&dir, true)?;
    for &id in bank.emitters.keys() {
        let mut system = libssbm::particle::System::default();
        system.definitions = bank.emitters.clone();
        system.spawn(id, glam::Mat4::IDENTITY, 1)?;
        let mut drawn = 0;
        for _ in 0..240 {
            system.tick();
            for p in &system.particles {
                assert!(
                    p.position.is_finite()
                        && p.velocity.is_finite()
                        && p.size.is_finite()
                        && p.color.is_finite()
                );
                assert!(
                    (p.frame as usize)
                        < bank.textures[p.texture as usize]
                            .as_ref()
                            .unwrap()
                            .frames
                            .len()
                );
            }
            drawn += system.particles.len();
        }
        assert!(
            system.errors.is_empty(),
            "emitter {id}: {:?}",
            system.errors
        );
        assert!(drawn > 0, "emitter {id} never produced particles");
    }
    Ok(())
}
#[test]
fn shared_attack_emitters_execute_their_source_dependencies() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfCoData.dat").exists() {
        return Ok(());
    }
    let bank = Effects::load(&dir, false)?;
    for id in [11, 263, 89, 94, 25, 75] {
        let mut system = libssbm::particle::System::default();
        system.definitions = bank.emitters.clone();
        system.spawn(id, glam::Mat4::IDENTITY, 1)?;
        let mut drawn = 0;
        for _ in 0..240 {
            system.tick();
            drawn += system.particles.len();
        }
        assert!(
            system.errors.is_empty(),
            "emitter {id}: {:?}",
            system.errors
        );
        assert!(drawn > 0, "emitter {id} never drew");
    }
    Ok(())
}
