//! Captain Falcon's staged specials against his real disc clips (skipped without
//! the player's extracted cache): the punch steps forward, the Raptor Boost
//! lunges and (on contact) uppercuts or spikes, the Dive grabs, explodes and
//! rebounds, and the Kick travels, in the air diagonally down.
use glam::Vec3;
use libssbm::{
    captain::Captain,
    sim::{EventKind, FlatHost, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
    Input, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::CAPTAIN};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn falcon_at(position: Vec3, grounded: bool) -> Result<Option<Sim>> {
    let dir = dir();
    if !CAPTAIN.complete(&dir) {
        eprintln!("SKIP: Captain Falcon disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &CAPTAIN)?;
    let specials = libssbm_assets::params::load_captain_specials(&dir)?;
    eprintln!("falcon specials {specials:?}");
    let mut sim = Sim::with_behavior(params, Box::new(Captain::new(specials)), position);
    let animations = Animations::load_for(&dir, &CAPTAIN)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &CAPTAIN)?;
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
fn input(buttons: u32) -> Input {
    Input { buttons, aim: Vec3::X, ..Input::idle() }
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
/// Runs a move to its end, calling `connect` once on tick `connect_at`, and returns
/// the clips seen, the strongest hitbox damage and the ticks taken.
fn run(sim: &mut Sim, host: &mut FlatHost, connect_at: Option<u32>) -> (Vec<&'static str>, f32, u32) {
    let mut clips: Vec<&'static str> = Vec::new();
    let mut strongest = 0f32;
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < 400 {
        if Some(ticks) == connect_at {
            sim.attack_connected();
        }
        step(sim, host, Input::idle());
        if clips.last() != Some(&sim.motion.clip) {
            clips.push(sim.motion.clip);
        }
        for s in &sim.action.samples {
            strongest = strongest.max(s.hitbox.damage);
        }
        ticks += 1;
    }
    (clips, strongest, ticks)
}

#[test]
fn falcon_punch_steps_forward_and_hits_once() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(NEUTRAL_B));
    let (clips, strongest, ticks) = run(&mut sim, &mut host, None);
    assert!(sim.current_move.is_none(), "the punch never ended");
    assert_eq!(clips.first(), Some(&"SpecialN"));
    assert!(strongest >= 27., "the punch hit for only {strongest}%");
    assert!(ticks < 130, "the punch took {ticks} ticks");
    assert!(sim.position.x > 5., "the ground punch did not step forward ({})", sim.position.x);
    Ok(())
}

#[test]
fn raptor_boost_lunges_and_a_miss_just_ends() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B));
    let (clips, _, _) = run(&mut sim, &mut host, None);
    assert!(sim.current_move.is_none(), "the lunge never ended");
    assert_eq!(clips.first(), Some(&"SpecialSStart"));
    assert!(!clips.contains(&"SpecialS"), "a miss played the uppercut");
    assert!(sim.position.x > 60., "the lunge covered only {}", sim.position.x);
    Ok(())
}

#[test]
fn raptor_boost_uppercuts_on_contact() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B));
    let mut upward = false;
    let mut connected = false;
    for tick in 0..200 {
        if tick == 25 {
            sim.attack_connected();
            connected = true;
        }
        step(&mut sim, &mut host, Input::idle());
        upward |= sim.action.samples.iter().any(|s| s.hitbox.damage >= 7. && s.hitbox.angle == 90);
        if sim.current_move.is_none() {
            break;
        }
    }
    assert!(connected && upward, "contact never produced the 7% uppercut");
    Ok(())
}

#[test]
fn aerial_raptor_boost_spikes_on_contact() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::new(0., 300., 0.), false)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B));
    let mut spike = false;
    for tick in 0..200 {
        if tick == 25 {
            sim.attack_connected();
        }
        step(&mut sim, &mut host, Input::idle());
        spike |= sim.action.samples.iter().any(|s| s.hitbox.damage >= 7. && s.hitbox.angle == 270);
        if sim.current_move.is_none() {
            break;
        }
    }
    assert!(spike, "the aerial uppercut did not spike");
    Ok(())
}

#[test]
fn falcon_kick_travels_and_the_aerial_one_dives() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(DOWN_B));
    let (clips, strongest, _) = run(&mut sim, &mut host, None);
    assert!(sim.current_move.is_none(), "the kick never ended");
    assert!(clips.contains(&"SpecialLwEnd"), "no recovery clip: {clips:?}");
    assert!(strongest >= 15., "the kick hit for only {strongest}%");
    assert!(sim.position.x > 80., "the kick covered only {}", sim.position.x);

    let Some(mut air) = falcon_at(Vec3::new(0., 300., 0.), false)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut air, &mut host);
    let start = air.position;
    step(&mut air, &mut host, input(DOWN_B));
    for _ in 0..30 {
        step(&mut air, &mut host, Input::idle());
    }
    assert!(air.position.x - start.x > 10., "the aerial kick did not travel ({})", air.position.x - start.x);
    assert!(start.y - air.position.y > 10., "the aerial kick did not dive ({})", start.y - air.position.y);
    Ok(())
}

#[test]
fn falcon_dive_catches_explodes_throws_and_rebounds() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(UP_B));
    let mut peak_x = 0f32;
    let mut exploded = false;
    let mut clips: Vec<&'static str> = Vec::new();
    for tick in 0..300 {
        if tick == 16 {
            sim.attack_connected();
        }
        step(&mut sim, &mut host, Input::idle());
        peak_x = peak_x.max(sim.position.x);
        exploded |= sim.action.samples.iter().any(|s| s.hitbox.damage >= 5. && s.hitbox.element == 1);
        if clips.last() != Some(&sim.motion.clip) {
            clips.push(sim.motion.clip);
        }
        if sim.current_move.is_none() {
            break;
        }
    }
    assert!(clips.contains(&"SpecialHiCatch") && clips.contains(&"SpecialHiThrow"), "stages: {clips:?}");
    assert_eq!(hits(&host, 23), vec![1., 2.], "catch then throw events");
    assert!(exploded, "the explosion had no hitbox");
    assert!(sim.position.x < peak_x - 10., "no rebound: ended at {} after {peak_x}", sim.position.x);
    Ok(())
}

#[test]
fn falcon_dive_without_a_grab_is_a_plain_leap() -> Result<()> {
    let Some(mut sim) = falcon_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(UP_B));
    let (clips, _, _) = run(&mut sim, &mut host, None);
    assert!(!clips.contains(&"SpecialHiCatch"), "a miss caught something: {clips:?}");
    assert!(hits(&host, 23).is_empty());
    Ok(())
}
