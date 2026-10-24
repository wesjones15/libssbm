//! Yoshi's staged specials and flutter jump against his real disc clips (skipped
//! without the player's extracted cache).
use glam::Vec3;
use libssbm::{
    sim::{EventKind, FlatHost, DOWN_B, JUMP, NEUTRAL_B, SIDE_B},
    yoshi::Yoshi,
    Input, Move, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::YOSHI, presentation::Presentation};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn yoshi_at(position: Vec3, grounded: bool) -> Result<Option<Sim>> {
    let dir = dir();
    if !YOSHI.complete(&dir) {
        eprintln!("SKIP: Yoshi disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &YOSHI)?;
    let specials = libssbm_assets::params::load_yoshi_specials(&dir)?;
    eprintln!("yoshi specials {specials:?}, air jump sound {}", params.air_jump_sound);
    let mut sim = Sim::with_behavior(params, Box::new(Yoshi::new(specials)), position);
    let animations = Animations::load_for(&dir, &YOSHI)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &YOSHI)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = grounded;
    sim.facing = Vec3::X;
    sim.host_jump_height = Some(40.);
    Ok(Some(sim))
}
fn input(buttons: u32, movement: Vec3) -> Input {
    Input { buttons, aim: Vec3::X, movement, ..Input::idle() }
}
fn step(sim: &mut Sim, host: &mut FlatHost, i: Input) {
    sim.advance(host, i, 1. / 60.);
}
fn settle(sim: &mut Sim, host: &mut FlatHost) {
    for _ in 0..4 {
        step(sim, host, Input::idle());
    }
}
fn hits(host: &FlatHost, move_id: u32) -> Vec<f32> {
    host.events
        .iter()
        .filter(|e| e.kind == EventKind::Hit && e.move_id == move_id)
        .map(|e| e.strength)
        .collect()
}
fn run_out(sim: &mut Sim, host: &mut FlatHost, limit: u32) -> u32 {
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < limit {
        step(sim, host, Input::idle());
        ticks += 1;
    }
    ticks
}

#[test]
fn egg_lay_sticks_its_tongue_out_once_and_swallows_on_contact() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    // A miss: the tongue goes out and back, nothing is laid.
    step(&mut sim, &mut host, input(NEUTRAL_B, Vec3::ZERO));
    let ticks = run_out(&mut sim, &mut host, 200);
    assert!(sim.current_move.is_none(), "the tongue never came back");
    assert_eq!(hits(&host, 20), vec![1.], "a miss should only show the tongue");
    assert!(ticks < 60, "a miss took {ticks} ticks (clip is 40)");

    // A hit: the host reports contact and the swallow clip plays, laying the egg.
    host.events.clear();
    sim.host_flags = 0;
    step(&mut sim, &mut host, input(NEUTRAL_B, Vec3::ZERO));
    let mut swallowed = false;
    for tick in 0..120 {
        if sim.motion.clip.contains("N2") {
            swallowed = true;
        }
        if tick == 24 {
            sim.attack_connected();
        }
        step(&mut sim, &mut host, Input::idle());
        if sim.current_move.is_none() {
            break;
        }
    }
    assert!(swallowed, "contact never played the swallow clip");
    assert_eq!(hits(&host, 20), vec![1., 2.], "tongue then egg expected");
    Ok(())
}

#[test]
fn egg_roll_curls_up_rolls_steers_and_ends() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut rolling = 0;
    let mut boxed = false;
    let mut heading_changed = false;
    for tick in 0..400 {
        // Hold the button to roll; after ten ticks of rolling steer the other way, then
        // release to break out.
        let stick = if rolling > 10 { Vec3::Z } else { Vec3::X };
        let button = if tick < 120 { SIDE_B } else { 0 };
        step(&mut sim, &mut host, input(button, stick));
        if sim.motion.clip == "SpecialSLoop" {
            rolling += 1;
            boxed |= sim.action.samples.iter().any(|s| s.hitbox.damage >= 3.);
            heading_changed |= sim.facing.z > 0.3;
        }
        if sim.current_move.is_none() {
            assert!(tick > 100, "the roll was over in {tick} ticks");
            break;
        }
    }
    assert!(sim.current_move.is_none(), "the roll never ended");
    assert!(rolling > 30, "rolled for only {rolling} ticks");
    assert!(boxed, "the egg never had a hitbox");
    assert!(heading_changed, "the stick did not steer the egg");
    assert!(sim.position.length() > 10., "the roll covered only {}", sim.position.length());
    Ok(())
}

#[test]
fn egg_roll_lasts_while_held_and_ends_on_release() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    // Releasing during the curl-up ends it before the roll starts.
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    step(&mut sim, &mut host, Input::idle());
    assert_eq!(sim.motion.clip, "SpecialSEnd", "releasing did not end the start");
    run_out(&mut sim, &mut host, 120);

    // Holding keeps it rolling; releasing ends it immediately.
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut looped = false;
    for _ in 0..40 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
        if sim.motion.clip == "SpecialSLoop" { looped = true; break; }
    }
    assert!(looped, "the roll loop never started");
    for _ in 0..30 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    }
    assert_eq!(sim.motion.clip, "SpecialSLoop", "the roll ended while still held");
    step(&mut sim, &mut host, input(0, Vec3::X));
    assert_eq!(sim.motion.clip, "SpecialSEnd", "releasing did not end the roll");
    Ok(())
}

#[test]
fn held_egg_roll_caps_at_ten_seconds() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut loop_ticks = 0;
    for _ in 0..750 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
        if sim.motion.clip == "SpecialSLoop" { loop_ticks += 1; }
        if sim.current_move.is_none() { break; }
    }
    assert!(sim.current_move.is_none(), "a held roll never capped");
    assert!(loop_ticks <= 601, "the roll ran {loop_ticks} loop ticks");
    assert!(loop_ticks >= 590, "the roll ended early at {loop_ticks} loop ticks");
    Ok(())
}

#[test]
fn egg_roll_armor_break_ends_the_roll() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut looped = false;
    for _ in 0..40 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
        if sim.motion.clip == "SpecialSLoop" { looped = true; break; }
    }
    assert!(looped, "the roll loop never started");
    sim.roll_break = true;
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    assert_eq!(sim.motion.clip, "SpecialSEnd", "an armor break did not end the roll");
    Ok(())
}

#[test]
fn egg_roll_strikes_repeatedly() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut generations = std::collections::BTreeSet::new();
    let mut rolling = 0;
    for _ in 0..90 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
        if sim.motion.clip != "SpecialSLoop" { continue; }
        rolling += 1;
        for sample in &sim.action.samples {
            if sample.clip == "SpecialSLoop" {
                generations.insert((sample.hitbox.group, sample.generation));
            }
        }
    }
    assert!(rolling > 30, "only rolled for {rolling} ticks");
    assert!(generations.len() >= 6, "the egg struck {} times in {rolling} ticks", generations.len());
    Ok(())
}

#[test]
fn egg_roll_draws_the_egg_model_not_the_body() -> Result<()> {
    let Some(_) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let dir = dir();
    assert_eq!(YOSHI.morph_move(), Some(Move::SpecialS));
    let body = Presentation::load_for(&dir, 0, &YOSHI)?;
    let egg = Presentation::load_for_variant(&dir, 0, &YOSHI, true)?;
    let body_visible = (0..body.model.batches.len()).filter(|&i| body.visible(i)).count();
    let egg_visible: Vec<usize> = (0..egg.model.batches.len()).filter(|&i| egg.visible(i)).collect();
    assert!(body_visible > 20, "the normal body is missing its parts");
    assert_eq!(egg_visible.len(), 1, "the egg must hide every body part");
    assert_eq!(egg.model.batches[egg_visible[0]].dobj, 35, "choice 1 is not the egg DObj");
    Ok(())
}

#[test]
fn egg_roll_pose_holds_its_place_when_the_loop_wraps() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut presentation = Presentation::load_for(&dir(), 0, &YOSHI)?;
    for _ in 0..60 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
        if sim.motion.clip == "SpecialSLoop" { break; }
    }
    assert_eq!(sim.motion.clip, "SpecialSLoop", "the roll loop never started");
    while sim.motion.frame < 58. {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    }
    presentation.update(&sim, 0.);
    let late = presentation.world[1].w_axis.truncate().length();
    // Cross the pass boundary: the disc clip travels 25 units and restarts at zero.
    let mut guard = 0;
    while sim.motion.frame > 2. && guard < 120 {
        step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
        guard += 1;
    }
    presentation.update(&sim, 0.);
    let wrapped = presentation.world[1].w_axis.truncate().length();
    assert!(late < 0.1 && wrapped < 0.1, "roll root moved {late} -> {wrapped}");
    Ok(())
}

#[test]
fn yoshi_bomb_hops_dives_and_announces_the_landing_once() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(DOWN_B, Vec3::ZERO));
    let mut peak = 0f32;
    let mut dived = false;
    let mut boxed = false;
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < 300 {
        step(&mut sim, &mut host, Input::idle());
        peak = peak.max(sim.position.y);
        dived |= sim.motion.clip == "SpecialAirLw";
        boxed |= sim.action.samples.iter().any(|s| s.hitbox.damage >= 14.);
        ticks += 1;
    }
    assert!(sim.current_move.is_none(), "the bomb never ended");
    assert!(peak > 3., "the grounded bomb never hopped (peak {peak})");
    assert!(dived, "no dive clip");
    assert!(boxed, "the dive had no hitbox");
    assert_eq!(hits(&host, 24).len(), 1, "the landing should be announced exactly once");
    Ok(())
}

#[test]
fn airborne_yoshi_bomb_dives_at_once() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::new(0., 300., 0.), false)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    let start = sim.position.y;
    step(&mut sim, &mut host, input(DOWN_B, Vec3::ZERO));
    for _ in 0..6 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(start - sim.position.y > 4., "the airborne bomb fell only {}", start - sim.position.y);
    assert_eq!(sim.motion.clip, "SpecialAirLw", "the dive clip should play straight away");
    run_out(&mut sim, &mut host, 400);
    assert_eq!(hits(&host, 24).len(), 1, "landing announced once");
    Ok(())
}

#[test]
fn every_aerial_jump_flutters_with_armor_until_he_lands() -> Result<()> {
    let Some(mut sim) = yoshi_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    // Off the ground, then the flutter.
    step(&mut sim, &mut host, input(JUMP, Vec3::ZERO));
    for _ in 0..12 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert_eq!(sim.defense_flags() & 256, 0, "the ground jump is not armored");
    step(&mut sim, &mut host, input(JUMP, Vec3::ZERO));
    step(&mut sim, &mut host, Input::idle());
    assert_ne!(sim.defense_flags() & 256, 0, "the aerial jump has no armor");
    let flutter_peak = {
        let mut peak = 0f32;
        for _ in 0..90 {
            step(&mut sim, &mut host, Input::idle());
            peak = peak.max(sim.position.y);
        }
        peak
    };
    assert_eq!(sim.defense_flags() & 256, 0, "the armor outlived the flutter");
    // The flutter rises further than a plain double jump from the same spot.
    let mut plain = Sim::with_behavior(
        libssbm_assets::params::load_common(&dir(), &YOSHI)?,
        Box::new(libssbm::samus::Samus::fixture()),
        Vec3::ZERO,
    );
    plain.host_jump_height = Some(40.);
    plain.grounded = true;
    let mut plain_host = FlatHost::default();
    for _ in 0..4 {
        step(&mut plain, &mut plain_host, Input::idle());
    }
    step(&mut plain, &mut plain_host, input(JUMP, Vec3::ZERO));
    for _ in 0..12 {
        step(&mut plain, &mut plain_host, Input::idle());
    }
    step(&mut plain, &mut plain_host, input(JUMP, Vec3::ZERO));
    let mut plain_peak = 0f32;
    for _ in 0..90 {
        step(&mut plain, &mut plain_host, Input::idle());
        plain_peak = plain_peak.max(plain.position.y);
    }
    assert!(flutter_peak > plain_peak, "flutter peak {flutter_peak} vs plain {plain_peak}");
    Ok(())
}
