//! Luigi's staged specials against his real disc clips (skipped without the player's
//! extracted cache): the Green Missile charges, launches faster the longer it was held,
//! misfires at the fixed misfire speed with its 25% hitbox and stops on what it hits; Super
//! Jump Punch rises with the clip and ends in a helpless fall with landing lag; the Cyclone
//! announces its spin, lifts again on every tap and ends.
use glam::Vec3;
use libssbm::{
    luigi::Luigi,
    sim::{EventKind, FlatHost, Phase, DOWN_B, SIDE_B, UP_B},
    Input, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::LUIGI};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn luigi_at(grounded: bool, misfire_chance: Option<f32>) -> Result<Option<Sim>> {
    let dir = dir();
    if !LUIGI.complete(&dir) {
        eprintln!("SKIP: Luigi disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &LUIGI)?;
    let mut specials = libssbm_assets::params::load_luigi_specials(&dir)?;
    if let Some(chance) = misfire_chance {
        specials.misfire_chance = chance;
    }
    let position = if grounded { Vec3::ZERO } else { Vec3::new(0., 200., 0.) };
    let mut sim = Sim::with_behavior(params, Box::new(Luigi::new(specials)), position);
    let animations = Animations::load_for(&dir, &LUIGI)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &LUIGI)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = grounded;
    sim.facing = Vec3::X;
    Ok(Some(sim))
}

fn input(buttons: u32) -> Input {
    Input { buttons, aim: Vec3::X, ..Input::idle() }
}
fn step(sim: &mut Sim, host: &mut FlatHost, i: Input) {
    sim.advance(host, i, 1. / 60.);
}

/// Holds the missile button for `hold` ticks, releases, and reports the fastest horizontal speed
/// the launch reached and the highest hitbox damage seen.
fn missile(misfire_chance: Option<f32>, hold: u32) -> Result<Option<(f32, f32)>> {
    let Some(mut sim) = luigi_at(false, misfire_chance)? else { return Ok(None) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    for _ in 0..(20 + hold) {
        step(&mut sim, &mut host, input(SIDE_B));
    }
    let (mut speed, mut damage) = (0f32, 0f32);
    for _ in 0..90 {
        step(&mut sim, &mut host, Input::idle());
        speed = speed.max(sim.velocity.with_y(0.).length());
        for active in sim.action.active.values() {
            damage = damage.max(active.hitbox.damage);
        }
    }
    Ok(Some((speed, damage)))
}

#[test]
fn green_missile_launches_faster_and_harder_the_longer_it_is_held() -> Result<()> {
    // A chance this large never misfires.
    let Some(short) = missile(Some(1.0e9), 2)? else { return Ok(()) };
    let long = missile(Some(1.0e9), 60)?.unwrap();
    assert!(long.0 > short.0 * 1.2, "speed {:?} vs {:?}", short, long);
    assert!(long.1 > short.1 + 3., "damage {:?} vs {:?}", short, long);
    assert!(short.0 > 1.0, "the missile never flew: {short:?}");
    Ok(())
}

#[test]
fn a_misfire_flies_at_the_misfire_speed_with_the_clips_own_hitbox() -> Result<()> {
    let Some(misfire) = missile(Some(1.), 2)? else { return Ok(()) };
    let charged = missile(Some(1.0e9), 80)?.unwrap();
    assert!(misfire.0 > charged.0, "a misfire is faster than the best charge: {misfire:?} vs {charged:?}");
    assert!(misfire.1 >= 25. - 0.01, "a misfire keeps the clip's 25% hitbox: {misfire:?}");
    assert!((25. .. 27.).contains(&charged.1), "a full charge hits for {} (charge * slope + tilt)", charged.1);
    Ok(())
}

#[test]
fn the_missile_stops_when_it_hits_something() -> Result<()> {
    let Some(mut sim) = luigi_at(false, Some(1.0e9))? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    for _ in 0..30 {
        step(&mut sim, &mut host, input(SIDE_B));
    }
    let mut flying = false;
    for _ in 0..80 {
        step(&mut sim, &mut host, Input::idle());
        if sim.velocity.with_y(0.).length() > 1.5 {
            flying = true;
            break;
        }
    }
    assert!(flying, "never launched");
    sim.attack_connected();
    step(&mut sim, &mut host, Input::idle());
    assert!(sim.velocity.with_y(0.).length() < 1.5, "kept flying after a hit: {:?}", sim.velocity);
    Ok(())
}

#[test]
fn super_jump_punch_climbs_with_the_clip_and_ends_in_a_special_fall() -> Result<()> {
    let Some(mut sim) = luigi_at(true, None)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(UP_B));
    let mut peak = 0f32;
    for _ in 0..60 {
        step(&mut sim, &mut host, Input::idle());
        peak = peak.max(sim.position.y);
    }
    assert!(peak > 15., "the punch lifted him only {peak}");
    assert!(sim.current_move.is_none(), "the punch never ended");
    assert_eq!(sim.phase, Phase::Helpless, "no special fall after the punch");
    // Falling out of it he drifts at x50 of the usual air speed, and the landing lasts x54 = 40 frames.
    let mut landed = false;
    for _ in 0..600 {
        step(&mut sim, &mut host, Input::idle());
        if sim.grounded {
            landed = true;
            break;
        }
    }
    assert!(landed, "never landed");
    assert_eq!(sim.motion.clip, "Landing");
    let mut lag = 0;
    while sim.motion.clip.starts_with("Landing") && !sim.motion.finished() && lag < 200 {
        step(&mut sim, &mut host, Input::idle());
        lag += 1;
    }
    assert!((38..=44).contains(&lag), "landing lag was {lag} frames");
    Ok(())
}

#[test]
fn the_cyclone_announces_its_spin_and_lifts_again_on_every_tap() -> Result<()> {
    let Some(mut sim) = luigi_at(false, None)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(DOWN_B));
    let mut lifts = 0;
    let mut last_y = sim.velocity.y;
    for t in 0..70 {
        // A fresh press every twelve ticks.
        let pressed = t % 12 == 6;
        step(&mut sim, &mut host, input(if pressed { DOWN_B } else { 0 }));
        if sim.velocity.y > last_y + 0.3 {
            lifts += 1;
        }
        last_y = sim.velocity.y;
    }
    assert!(lifts >= 2, "taps lifted him {lifts} times");
    let spins: Vec<_> = host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 24).collect();
    assert_eq!(spins.len(), 2, "a start and an end event: {spins:?}");
    assert_eq!(spins[0].strength, 1.);
    assert_eq!(spins[1].strength, 2.);
    for _ in 0..80 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(sim.current_move.is_none(), "the cyclone never ended");
    Ok(())
}

#[test]
fn the_fireball_launch_lights_him_with_a_fading_green_rim_light() -> Result<()> {
    use libssbm::sim::NEUTRAL_B;
    let Some(mut sim) = luigi_at(true, None)? else { return Ok(()) };
    sim.colanim.table = Some(libssbm_assets::params::load_colanim(&dir())?);
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(NEUTRAL_B));
    let mut lights = Vec::new();
    for _ in 0..40 {
        step(&mut sim, &mut host, Input::idle());
        if let Some(light) = sim.colanim.light() {
            lights.push(light);
        }
    }
    assert!(!lights.is_empty(), "the fireball never lit him");
    let first = lights[0];
    // Colour program 88: a pale green additive light from the front that fades out.
    assert!(first.additive && first.color[1] > first.color[0], "not a green rim light: {first:?}");
    assert!(lights.last().unwrap().color[3] < first.color[3], "the light did not fade: {lights:?}");
    for _ in 0..80 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(sim.colanim.light().is_none(), "the light outlived the move");
    Ok(())
}

/// A floor that behaves like the game's collision: while the fighter is not rising it
/// snaps his feet back to the floor and reports him grounded, exactly as `MeleeDriver`'s
/// capsule probe does. The plain [`FlatHost`] leaves him hovering a hair above it instead.
struct SnapHost;
impl libssbm::sim::Host for SnapHost {
    fn collide(&mut self, from: Vec3, to: Vec3) -> libssbm::sim::Contact {
        let delta = to - from;
        let mut position = to;
        let mut grounded = false;
        if delta.y <= 0. && to.y <= 0.05 {
            position.y = 0.;
            grounded = true;
        }
        libssbm::sim::Contact { position, grounded, ceiling: false }
    }
    fn event(&mut self, _: libssbm::sim::Event) {}
}

/// Regression: Luigi's grounded Super Jump Punch has zero root motion for its first
/// seven frames, so the floor probe re-grounds him mid-move. That used to be mistaken for
/// a landing and cancelled the move before the climb (frames 8-22). Samus's up special
/// lifts from frame 0 and never re-grounds, which is why only Luigi broke.
#[test]
fn grounded_up_b_survives_the_floor_snap_during_its_wind_up() -> Result<()> {
    let Some(mut sim) = luigi_at(true, None)? else { return Ok(()) };
    let mut host = SnapHost;
    for _ in 0..4 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    sim.advance(&mut host, input(UP_B), 1. / 60.);
    let mut peak = 0f32;
    for _ in 0..60 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        peak = peak.max(sim.position.y);
    }
    assert!(peak > 15., "the floor snap cancelled the grounded punch: peak {peak}");
    Ok(())
}

/// Diagnostic: how far the Green Missile actually travels, charged and misfired (Melee units and
/// metres at the host's 0.1 m/unit).
#[test]
fn report_missile_travel() -> Result<()> {
    for (label, chance) in [("charged", 1.0e9f32), ("misfire", 1.0f32)] {
        let Some(mut sim) = luigi_at(true, Some(chance))? else { return Ok(()) };
        let mut host = FlatHost { events: Vec::new() };
        for _ in 0..4 {
            step(&mut sim, &mut host, Input::idle());
        }
        for _ in 0..150 {
            step(&mut sim, &mut host, input(SIDE_B));
        }
        let start = sim.position;
        let mut far = 0f32;
        for _ in 0..300 {
            step(&mut sim, &mut host, Input::idle());
            far = far.max((sim.position - start).with_y(0.).length());
        }
        eprintln!("{label}: max horizontal travel {far:.1} units = {:.2} m", far * 0.1);
    }
    Ok(())
}
