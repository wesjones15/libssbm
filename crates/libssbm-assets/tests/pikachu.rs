//! Pikachu's staged specials against his real disc clips (skipped without the
//! player's extracted cache): camera-vector zips, no snap-back, Thunder timeline,
//! gravity release, and committed dash travel.
use glam::Vec3;
use libssbm::{
    pikachu::Pikachu,
    sim::{EventKind, FlatHost, DOWN_B, NEUTRAL_B, SIDE_B, UP_B},
    Input, Move, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::PIKACHU};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn pikachu_at(position: Vec3, grounded: bool) -> Result<Option<Sim>> {
    let dir = dir();
    if !PIKACHU.complete(&dir) {
        eprintln!("SKIP: Pikachu disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &PIKACHU)?;
    let specials = libssbm_assets::params::load_pikachu_specials(&dir)?;
    let mut sim = Sim::with_behavior(params, Box::new(Pikachu::new(specials)), position);
    let animations = Animations::load_for(&dir, &PIKACHU)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions =
        libssbm_assets::action::load_for_variants(&dir, &animations, &PIKACHU)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = grounded;
    sim.facing = Vec3::X;
    Ok(Some(sim))
}

fn idle() -> Input {
    Input::idle()
}
fn press(button: u32) -> Input {
    Input { buttons: button, ..Input::idle() }
}
fn step(sim: &mut Sim, host: &mut FlatHost, input: Input) {
    sim.advance(host, input, 1. / 60.);
}
/// Presses `button` for one tick then idles until the move ends; returns the
/// furthest +X reached and asserts the fighter never moves back along it.
fn run_move(sim: &mut Sim, host: &mut FlatHost, button: u32, max: usize) -> f32 {
    for _ in 0..4 {
        step(sim, host, idle());
    }
    let mut furthest = sim.position.x;
    step(sim, host, press(button));
    for tick in 0..max {
        step(sim, host, idle());
        assert!(
            sim.position.x >= furthest - 1e-3,
            "snap-back at tick {tick}: {} after reaching {furthest}",
            sim.position.x
        );
        furthest = furthest.max(sim.position.x);
        if sim.current_move.is_none() && tick > 3 {
            break;
        }
    }
    assert!(sim.current_move.is_none(), "move never ended");
    furthest
}

#[test]
fn quick_attack_without_a_lock_still_travels_forward() -> Result<()> {
    let Some(mut sim) = pikachu_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    let furthest = run_move(&mut sim, &mut host, UP_B, 300);
    assert!(furthest > 20., "zip travelled only {furthest}");
    Ok(())
}

#[test]
fn skull_bash_travel_scales_with_charge_and_never_snaps_back() -> Result<()> {
    let mut reach = Vec::new();
    let mut damage = Vec::new();
    for hold in [0usize, 80] {
        let Some(mut sim) = pikachu_at(Vec3::ZERO, true)? else { return Ok(()) };
        let mut host = FlatHost { events: Vec::new() };
        for _ in 0..4 {
            step(&mut sim, &mut host, idle());
        }
        let mut furthest = 0f32;
        let mut strongest = 0f32;
        step(&mut sim, &mut host, press(SIDE_B));
        for tick in 0..400 {
            // Hold B through the start clip and `hold` charge ticks, then release.
            let held = tick < 14 + hold;
            step(&mut sim, &mut host, if held { press(SIDE_B) } else { idle() });
            assert!(sim.position.x >= furthest - 1e-3, "snap-back at tick {tick}");
            furthest = furthest.max(sim.position.x);
            for sample in &sim.action.samples {
                strongest = strongest.max(sample.hitbox.damage * sample.strength);
            }
            if sim.current_move.is_none() && tick > 20 {
                break;
            }
        }
        assert!(sim.current_move.is_none(), "Skull Bash never ended");
        reach.push(furthest);
        damage.push(strongest);
    }
    assert!(reach[0] > 15., "uncharged bash travelled {}", reach[0]);
    assert!(reach[1] > reach[0] * 1.2, "charge did not extend the bash: {reach:?}");
    assert!((damage[0] - 4.).abs() < 0.5, "uncharged bash should hit for ~4%: {damage:?}");
    assert!(damage[1] > damage[0] * 3., "charge did not raise damage: {damage:?}");
    Ok(())
}

#[test]
fn thunder_rumble_plays_when_the_wait_starts_not_at_the_strike() -> Result<()> {
    let Some(mut sim) = pikachu_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, idle());
    }
    step(&mut sim, &mut host, press(DOWN_B));
    let mut spawned: Option<i32> = None;
    let mut struck: Option<i32> = None;
    let mut heard: Option<i32> = None;
    for tick in 0..300i32 {
        step(&mut sim, &mut host, idle());
        if heard.is_none()
            && sim.action.sounds.iter().any(|s| s.sound == 240082 && !s.stop)
        {
            assert_eq!(sim.motion.clip, "SpecialLwLoop", "the rumble left the wait clip");
            heard = Some(tick);
        }
        for e in host.events.drain(..) {
            if e.kind == EventKind::Hit && e.move_id == Move::SpecialLw as u32 {
                match e.strength {
                    1. => spawned = Some(tick),
                    2. => struck = Some(tick),
                    _ => {}
                }
            }
        }
        if sim.current_move.is_none() && tick > 3 {
            break;
        }
    }
    let heard = heard.expect("Thunder's wait never played its 240082 rumble");
    let spawned = spawned.expect("no cloud spawn");
    let struck = struck.expect("no strike");
    assert!(
        heard <= spawned + 1,
        "rumble at tick {heard}, cloud spawn at {spawned}: it must start with the wait"
    );
    assert!(heard < struck, "rumble at tick {heard}, strike at {struck}");
    Ok(())
}

#[test]
fn thunder_spawns_then_strikes_after_the_derived_delay() -> Result<()> {
    let Some(mut sim) = pikachu_at(Vec3::ZERO, true)? else { return Ok(()) };
    let expected = libssbm_assets::params::load_pikachu_specials(&dir())?.thunder_strike_frames;
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, idle());
    }
    step(&mut sim, &mut host, press(DOWN_B));
    let mut spawned: Option<i32> = None;
    let mut struck: Option<i32> = None;
    let mut early_hits = 0;
    let mut crackle = 0f32;
    for tick in 0..300i32 {
        step(&mut sim, &mut host, idle());
        let samples = sim.action.samples.len();
        for sample in &sim.action.samples {
            crackle = crackle.max(sample.hitbox.damage * sample.strength);
        }
        let waiting = spawned.is_some() && struck.is_none();
        for e in host.events.drain(..) {
            if e.kind == EventKind::Hit && e.move_id == Move::SpecialLw as u32 {
                if e.strength == 1. {
                    spawned = Some(tick);
                } else if e.strength == 2. {
                    struck = Some(tick);
                }
            }
        }
        // A tick that ends before the strike event must not have swung the hitbox.
        if waiting && struck.is_none() {
            early_hits += samples;
        }
        if sim.current_move.is_none() && tick > 3 {
            break;
        }
    }
    let (spawned, struck) = (spawned.expect("no cloud spawn"), struck.expect("no strike"));
    assert!(
        (struck - spawned - expected as i32).abs() <= 2,
        "strike {} ticks after spawn, expected {expected}",
        struck - spawned
    );
    assert!(sim.current_move.is_none());
    assert_eq!(early_hits, 0, "the loop's hitbox must stay muted until the bolt strikes");
    assert!((crackle - 17.).abs() < 0.5, "the strike should crackle for 17%: {crackle}");
    Ok(())
}

#[test]
fn moves_release_gravity_so_pikachu_falls_afterwards() -> Result<()> {
    for button in [UP_B, DOWN_B, SIDE_B] {
        let Some(mut sim) = pikachu_at(Vec3::new(0., 400., 0.), false)? else { return Ok(()) };
        let mut host = FlatHost { events: Vec::new() };
        for _ in 0..4 {
            step(&mut sim, &mut host, idle());
        }
        step(&mut sim, &mut host, press(button));
        for tick in 0..400 {
            step(&mut sim, &mut host, idle());
            if sim.current_move.is_none() && tick > 3 {
                break;
            }
        }
        assert!(sim.current_move.is_none(), "move {button} never ended");
        let before = sim.position.y;
        for _ in 0..30 {
            step(&mut sim, &mut host, idle());
        }
        assert!(
            sim.position.y < before - 1. || sim.grounded,
            "button {button}: still hovering at {}",
            sim.position.y
        );
    }
    Ok(())
}

#[test]
fn dash_attack_and_dash_grab_commit_their_travel() -> Result<()> {
    for m in [Move::DashAttack, Move::DashGrab] {
        let Some(mut sim) = pikachu_at(Vec3::ZERO, true)? else { return Ok(()) };
        let mut host = FlatHost { events: Vec::new() };
        let clip = if m == Move::DashAttack { "AttackDash" } else { "CatchDash" };
        let travel = sim.motion.definitions[clip].forward_delta(0., sim.motion.definitions[clip].frames);
        let Some(travel) = travel else { panic!("{clip} has no flagged travel") };
        assert!(sim.start_move(&mut host, m, idle()));
        let mut furthest = 0f32;
        for _ in 0..200 {
            step(&mut sim, &mut host, idle());
            assert!(sim.position.x >= furthest - 1e-3, "{clip} snapped back");
            furthest = furthest.max(sim.position.x);
        }
        assert!((sim.position.x - travel).abs() < 0.2, "{clip}: {} vs {travel}", sim.position.x);
    }
    Ok(())
}

// Thunder Jolt plays the grounded clip (the source's down-diagonal throw) for a level or
// downward aim and the airborne clip when it is fired from the air or aimed upward.
#[test]
fn borrowed_egg_lay_holds_the_upright_thunder_cast_pose() -> Result<()> {
    let Some(mut sim) = pikachu_at(Vec3::ZERO, true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 { step(&mut sim, &mut host, idle()); }
    sim.host_flags = libssbm::pikachu::HOST_EGG_LAY;
    let aim = Vec3::new(0., 0.5, 1.).normalize();
    let input = Input { aim, buttons: NEUTRAL_B, ..Input::idle() };
    step(&mut sim, &mut host, input);
    assert_eq!(sim.current_move, Some(Move::SpecialN), "RT did not start Egg Lay");
    assert_eq!(sim.motion.clip, "SpecialLwStart", "Egg Lay did not use the Thunder start");
    assert_eq!(sim.held_pose(), Some(15.), "Egg Lay did not hold the upright Thunder pose");
    assert!(sim.tongue_upper_pitch().is_some(), "the upper body is not pitching");
    for _ in 0..3 {
        step(&mut sim, &mut host, Input { aim, ..Input::idle() });
        assert_eq!(sim.motion.clip, "SpecialLwStart");
    }
    // The Thunder start script (cloud/bolt sounds and effects) must be muted.
    assert!(sim.action.sounds.is_empty(), "Thunder sounds leaked into Egg Lay");
    assert!(sim.action.effects.is_empty(), "Thunder effects leaked into Egg Lay");
    // Its 18-frame clip hands over to the swallow stage, which keeps the same held
    // upright Thunder pose instead of replaying the crouched wind-up.
    for _ in 0..20 { step(&mut sim, &mut host, idle()); }
    assert_eq!(sim.current_move, Some(Move::SpecialN), "Egg Lay ended before the swallow");
    assert_eq!(sim.motion.clip, "SpecialLwStart");
    assert_eq!(sim.held_pose(), Some(15.), "the swallow did not hold the Thunder pose");
    for _ in 0..24 { step(&mut sim, &mut host, idle()); }
    assert!(sim.current_move.is_none(), "Egg Lay never ended");
    Ok(())
}

// Thunder Jolt plays the grounded clip (the source's down-diagonal throw) for a level or
// downward aim and the airborne clip when it is fired from the air or aimed upward.
#[test]
fn thunder_jolt_uses_the_airborne_clip_when_aimed_up_or_fired_from_the_air() -> Result<()> {
    for (grounded, aim_y, expected) in [
        (true, 0.0, "SpecialN"),
        (true, -0.5, "SpecialN"),
        (true, 0.6, "SpecialAirN"),
        (false, 0.0, "SpecialAirN"),
        (false, 0.6, "SpecialAirN"),
    ] {
        let Some(mut sim) = pikachu_at(Vec3::new(0., if grounded { 0. } else { 10. }, 0.), grounded)? else { return Ok(()) };
        let mut host = FlatHost { events: Vec::new() };
        let input = Input { aim: Vec3::new(1., aim_y, 0.).normalize(), ..Input::idle() };
        assert!(sim.start_move(&mut host, Move::SpecialN, input));
        for _ in 0..4 {
            // The aim keeps moving after the move starts; the clip must not change.
            step(&mut sim, &mut host, Input { aim: Vec3::new(1., -aim_y, 0.).normalize(), ..Input::idle() });
            assert_eq!(sim.motion.clip, expected, "grounded {grounded}, aim.y {aim_y}");
        }
    }
    Ok(())
}
