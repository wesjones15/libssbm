//! Donkey Kong's staged specials against his real disc clips (skipped without the
//! player's extracted cache): the Giant Punch stores swings and spends them, can
//! be cancelled without losing them, the Hand Slap goes on while held with him
//! planted, and Spinning Kong steers.
use glam::Vec3;
use libssbm::{
    donkey::Donkey,
    sim::{EventKind, FlatHost, CANCEL, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
    Input, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::DONKEY};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn dk() -> Result<Option<Sim>> {
    let dir = dir();
    if !DONKEY.complete(&dir) {
        eprintln!("SKIP: Donkey Kong disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &DONKEY)?;
    let specials = libssbm_assets::params::load_donkey_specials(&dir)?;
    eprintln!("dk specials {specials:?}");
    let mut sim = Sim::with_behavior(params, Box::new(Donkey::new(specials)), Vec3::ZERO);
    let animations = Animations::load_for(&dir, &DONKEY)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &DONKEY)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = true;
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
fn strongest(sim: &Sim) -> f32 {
    sim.action.samples.iter().map(|s| s.hitbox.damage).fold(0., f32::max)
}
/// Holds the neutral special for `hold` ticks, lets go, and runs the move out.
fn punch(sim: &mut Sim, host: &mut FlatHost, hold: u32) -> (f32, Vec<&'static str>) {
    let mut clips: Vec<&'static str> = Vec::new();
    let mut best = 0f32;
    for _ in 0..hold {
        step(sim, host, input(NEUTRAL_B, Vec3::ZERO));
        if clips.last() != Some(&sim.motion.clip) {
            clips.push(sim.motion.clip);
        }
        best = best.max(strongest(sim));
    }
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < 300 {
        step(sim, host, Input::idle());
        if clips.last() != Some(&sim.motion.clip) {
            clips.push(sim.motion.clip);
        }
        best = best.max(strongest(sim));
        ticks += 1;
    }
    (best, clips)
}

#[test]
fn giant_punch_spends_the_swings_it_stored() -> Result<()> {
    let Some(mut sim) = dk()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    let (tap, tap_clips) = punch(&mut sim, &mut host, 1);
    assert!(tap_clips.contains(&"SpecialN"), "no release clip: {tap_clips:?}");
    let (half, half_clips) = punch(&mut sim, &mut host, 60);
    assert!(half_clips.contains(&"SpecialNLoop"), "holding never reached the loop: {half_clips:?}");
    let (full, _) = punch(&mut sim, &mut host, 400);
    assert!(half > tap + 3., "a half charge ({half}%) was no stronger than a tap ({tap}%)");
    assert!(full > half + 3., "a full charge ({full}%) was no stronger than a half ({half}%)");
    assert_eq!(sim.shot_charge, 0, "the swings were not spent");
    Ok(())
}

#[test]
fn cancelling_the_wind_up_keeps_the_swings() -> Result<()> {
    let Some(mut sim) = dk()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    for _ in 0..70 {
        step(&mut sim, &mut host, input(NEUTRAL_B, Vec3::ZERO));
    }
    let stored = sim.shot_charge;
    assert!(stored >= 2, "only {stored} swings stored");
    step(&mut sim, &mut host, input(CANCEL, Vec3::ZERO));
    let mut cancelled = false;
    for _ in 0..40 {
        cancelled |= sim.motion.clip.contains("Cans") || sim.motion.clip.contains("Cance");
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(cancelled, "the cancel clip never played");
    assert!(sim.current_move.is_none(), "the cancel never ended");
    assert_eq!(sim.shot_charge, stored, "cancelling lost the swings");
    // A tap now punches with what was kept.
    let (kept, _) = punch(&mut sim, &mut host, 1);
    let mut fresh = dk()?.unwrap();
    let mut fresh_host = FlatHost::default();
    settle(&mut fresh, &mut fresh_host);
    let (plain, _) = punch(&mut fresh, &mut fresh_host, 1);
    assert!(kept > plain + 2., "the kept charge ({kept}%) did not strengthen the tap ({plain}%)");
    Ok(())
}

#[test]
fn headbutt_stays_put_and_buries_on_the_ground() -> Result<()> {
    let Some(mut sim) = dk()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B, Vec3::ZERO));
    let mut buries = false;
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < 200 {
        step(&mut sim, &mut host, Input::idle());
        buries |= sim.action.samples.iter().any(|s| s.hitbox.element == 9);
        ticks += 1;
    }
    assert!(sim.position.x.abs() < 2., "the headbutt moved him {}", sim.position.x);
    assert!(buries, "no burying hitbox (element 9)");
    Ok(())
}

#[test]
fn spinning_kong_follows_the_stick_over_a_limited_rise() -> Result<()> {
    let Some(mut sim) = dk()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(UP_B, Vec3::Z));
    let mut peak = 0f32;
    for _ in 0..70 {
        step(&mut sim, &mut host, input(0, Vec3::Z));
        peak = peak.max(sim.position.y);
    }
    assert!(sim.position.z > 40., "steering south moved him only {}", sim.position.z);
    assert!(peak < 60., "the rise was {peak}");
    assert!(peak > 10., "no rise at all ({peak})");
    Ok(())
}

#[test]
fn hand_slap_goes_on_while_held_with_him_planted() -> Result<()> {
    let Some(mut sim) = dk()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    for _ in 0..300 {
        step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
    }
    assert!(sim.current_move.is_some(), "the slap ended while B was held");
    assert!(sim.position.x.abs() < 1., "he moved while slapping: {}", sim.position.x);
    let slams = host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 24).count();
    assert!(slams >= 16, "only {slams} slams in five seconds");
    let first = host.events.iter().find(|e| e.kind == EventKind::Hit && e.move_id == 24).map(|e| e.strength);
    assert_eq!(first, Some(1.), "the first slam is strength 1");
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < 120 {
        step(&mut sim, &mut host, Input::idle());
        ticks += 1;
    }
    assert!(sim.current_move.is_none(), "the slap never ended after letting go");
    Ok(())
}
