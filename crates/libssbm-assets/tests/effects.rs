use libssbm_assets::{Result, effects::Effects};
use std::path::PathBuf;
#[test]
fn screw_attack_ground_texture_moves_on_its_own_clock() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() { eprintln!("SKIP: effect archives absent"); return Ok(()); }
    // SpecialHi's initial ground cue 1021 resolves to common effect model 3.
    let mut actor = libssbm_assets::presentation::Presentation::effect(&dir, 3)?;
    actor.object_pose(0.);
    let before: Vec<_> = (0..actor.model.batches.len()).map(|i| actor.texture(i).map(|t| t.rgba.clone())).collect();
    actor.object_pose(6.);
    assert!((0..actor.model.batches.len()).any(|i| before[i].as_ref() != actor.texture(i).map(|t| &t.rgba)), "Screw Attack ground texture remains static");
    for batch in &actor.model.batches {
        if let Some(base) = &batch.texture {
            assert!(batch.texture_frames.iter().all(|t| (t.width, t.height) == (base.width, base.height)), "animated texture dimensions must fit the persistent host texture");
        }
    }
    // SRT tracks on other common effects share the same renderer path.
    let bank = Effects::load(&dir, false)?;
    let moving = |t: &&libssbm_assets::animation::Track| (102..=108).contains(&t.channel) && t.keys.windows(2).any(|k| (k[0].value - k[1].value).abs() > 0.0001);
    let id = bank.models.iter().position(|m| m.material_tracks.iter().any(|t| moving(&t))).unwrap();
    let track = bank.models[id].material_tracks.iter().find(moving).unwrap();
    let mut actor = libssbm_assets::presentation::Presentation::effect(&dir, id as u32)?;
    actor.object_pose(track.keys.first().unwrap().time);
    let before: Vec<_> = (0..actor.model.batches.len()).map(|i| actor.texture_matrix(i)).collect();
    actor.object_pose(track.keys.last().unwrap().time);
    assert!((0..actor.model.batches.len()).any(|i| !before[i].abs_diff_eq(actor.texture_matrix(i), 0.00001)), "common effect SRT tracks remain frozen");
    Ok(())
}
// The Screw Attack's ground dust ring (cue 1030, common model 4) grows for about 12 frames. It is
// authored with MODULATE alpha, so it used to hold full size at full opacity until removed:
// a static ring. It now dims by its blend track and fades out over its last frames.
#[test]
fn screw_attack_ground_dust_ring_fades_instead_of_holding_still() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfCoData.dat").exists() { eprintln!("SKIP: effect archives absent"); return Ok(()); }
    let mut ring = libssbm_assets::presentation::Presentation::effect(&dir, 4)?;
    let alpha = |p: &mut libssbm_assets::presentation::Presentation, f: f32| { p.object_pose(f); p.color(0)[3] };
    let (start, grown, late, end) = (alpha(&mut ring, 0.), alpha(&mut ring, 12.), alpha(&mut ring, 24.), alpha(&mut ring, 29.));
    assert!(start > 0.95, "ring should start opaque: {start}");
    assert!(grown < start && late < grown && end < late && end < 0.1, "ring does not fade: {start} {grown} {late} {end}");
    // Other common effects keep their authored alpha.
    let mut burst = libssbm_assets::presentation::Presentation::effect(&dir, 3)?;
    burst.object_pose(0.);
    assert!((burst.color(0)[3] - 0.6).abs() < 0.01);
    let mut screw = libssbm_assets::presentation::Presentation::effect(&dir, 2000)?;
    screw.object_pose(12.);
    assert!((screw.color(0)[3] - 0.7).abs() < 0.05);
    Ok(())
}
#[test]
fn charged_shot_keeps_its_spinning_core_facing_cameras_from_all_sides() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: article archive absent");
        return Ok(());
    }
    let mut p = libssbm_assets::presentation::Presentation::article(&dir, 1, 8)?;
    for batch in &p.model.batches {
        let texture = batch.texture.as_ref().unwrap();
        assert_eq!((texture.wrap_s, texture.wrap_t), (2, 2));
        let lo = batch
            .vertices
            .iter()
            .fold(glam::Vec2::splat(f32::INFINITY), |a, v| a.min(v.uv));
        let hi = batch
            .vertices
            .iter()
            .fold(glam::Vec2::splat(f32::NEG_INFINITY), |a, v| a.max(v.uv));
        assert!(
            (hi.x - lo.x - 2.).abs() < 0.001 && (hi.y - lo.y - 2.).abs() < 0.001,
            "quarter texture was not mirrored into the complete shot"
        );
    }
    let billboard = p
        .model
        .joints
        .iter()
        .position(|j| j.flags & 0xe00 == 0x800)
        .expect("source rotational billboard");
    for camera in [
        glam::Vec3::new(100., 0., 0.),
        glam::Vec3::new(-80., 30., 25.),
        glam::Vec3::new(20., 75., -50.),
    ] {
        p.object_pose(12.);
        let scale = p.world[billboard].to_scale_rotation_translation().0;
        p.face_camera(camera);
        let normal = p.world[billboard].z_axis.truncate().normalize();
        let look = (camera - p.world[billboard].w_axis.truncate()).normalize();
        assert!(
            normal.dot(look) > 0.999,
            "charge core remains edge-on to camera"
        );
        assert!((p.world[billboard].to_scale_rotation_translation().0 - scale).length() < 0.0001);
    }
    Ok(())
}
#[test]
fn screw_attack_keeps_its_texture_alpha_and_animated_fade() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() {
        eprintln!("SKIP: effect archives absent");
        return Ok(());
    }
    let mut p = libssbm_assets::presentation::Presentation::effect(&dir, 2000)?;
    assert!(p.model.batches.iter().all(|b| b.color[3] == 0.));
    p.object_pose(12.);
    assert!(
        p.model
            .batches
            .iter()
            .enumerate()
            .all(|(i, _)| p.color(i)[3] > 0.1),
        "source texture alpha was erased by zero material alpha"
    );
    let middle = p.color(0)[3];
    p.object_pose(0.);
    assert!(
        p.color(0)[3] < middle,
        "texture blend animation not applied"
    );
    for state in [0, 8] {
        let p = libssbm_assets::presentation::Presentation::article(&dir, 1, state)?;
        assert!(
            p.model
                .batches
                .iter()
                .enumerate()
                .all(|(i, _)| p.color(i)[3] >= 0.99),
            "charge shot texture replacement alpha was incorrectly halved"
        );
    }
    Ok(())
}
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
        "Fall",
        "FallF",
        "FallB",
        "FallAerial",
        "FallAerialF",
        "FallAerialB",
        "FallSpecial",
        "LandingFallSpecial",
        "LandingAirN",
        "LandingAirF",
        "LandingAirB",
        "LandingAirHi",
        "LandingAirLw",
        "DamageN2",
        "DamageFlyN",
        "DamageFall",
        "DownBoundU",
        "DownStandU",
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
    for id in [11, 263, 89, 94, 25, 75, 67, 12, 20, 4, 60] {
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
