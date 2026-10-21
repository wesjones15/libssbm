//! Link's staged specials against his real disc clips (skipped without the
//! player's extracted cache): the bow draws, charges and releases one arrow per
//! use, the boomerang leaves his hand once and the empty-handed clip plays while
//! it is out, and the bomb is pulled once.
use glam::Vec3;
use libssbm::{
    link::{Link, HOST_BOOMERANG_OUT},
    sim::{EventKind, FlatHost, DOWN_B, NEUTRAL_B, SIDE_B},
    Input, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::LINK};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn link() -> Result<Option<Sim>> {
    let dir = dir();
    if !LINK.complete(&dir) {
        eprintln!("SKIP: Link disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &LINK)?;
    let specials = libssbm_assets::params::load_link_specials(&dir)?;
    let mut sim = Sim::with_behavior(params, Box::new(Link::new(specials)), Vec3::ZERO);
    let animations = Animations::load_for(&dir, &LINK)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &LINK)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = true;
    sim.facing = Vec3::X;
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
fn shots(host: &FlatHost, move_id: u32) -> Vec<f32> {
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
fn bow_tap_fires_one_weak_arrow() -> Result<()> {
    let Some(mut sim) = link()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(NEUTRAL_B));
    let ticks = run_out(&mut sim, &mut host, 300);
    assert!(sim.current_move.is_none(), "bow never ended");
    let arrows = shots(&host, 20);
    assert_eq!(arrows.len(), 1, "one tap, one arrow ({arrows:?})");
    assert!(arrows[0] < 0.4, "a tap released a charged arrow ({})", arrows[0]);
    assert!(ticks < 120, "a tap took {ticks} ticks (draw 60 + release 25)");
    Ok(())
}

#[test]
fn bow_held_fires_one_full_charge_arrow() -> Result<()> {
    let Some(mut sim) = link()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    let mut loop_seen = false;
    for _ in 0..150 {
        step(&mut sim, &mut host, input(NEUTRAL_B));
        loop_seen |= sim.motion.clip.contains("NLoop");
    }
    assert!(loop_seen, "holding never reached the Loop clip");
    assert!(shots(&host, 20).is_empty(), "an arrow left while the button was held");
    run_out(&mut sim, &mut host, 200);
    let arrows = shots(&host, 20);
    assert_eq!(arrows.len(), 1, "one arrow on release ({arrows:?})");
    assert!(arrows[0] > 0.99, "a full charge fired at {}", arrows[0]);
    Ok(())
}

#[test]
fn boomerang_leaves_the_hand_once_and_the_empty_clip_plays_while_it_is_out() -> Result<()> {
    let Some(mut sim) = link()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(SIDE_B));
    run_out(&mut sim, &mut host, 200);
    assert_eq!(shots(&host, 21).len(), 1, "the boomerang was not thrown exactly once");
    assert!(sim.host_flags & HOST_BOOMERANG_OUT != 0, "the throw did not mark the boomerang out");

    // Still out: the next press plays the empty-handed throw and releases nothing.
    host.events.clear();
    step(&mut sim, &mut host, input(SIDE_B));
    let mut empty = false;
    for _ in 0..80 {
        empty |= sim.motion.clip.contains("S1Empty");
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(empty, "no empty-handed clip while the boomerang was out");
    assert!(shots(&host, 21).is_empty(), "a second boomerang was thrown");

    // Caught: the host clears the flag and he can throw again.
    sim.host_flags = 0;
    step(&mut sim, &mut host, input(SIDE_B));
    run_out(&mut sim, &mut host, 200);
    assert_eq!(shots(&host, 21).len(), 1, "he could not throw again after the catch");
    Ok(())
}

#[test]
fn bomb_is_pulled_once() -> Result<()> {
    let Some(mut sim) = link()? else { return Ok(()) };
    let mut host = FlatHost::default();
    settle(&mut sim, &mut host);
    step(&mut sim, &mut host, input(DOWN_B));
    let ticks = run_out(&mut sim, &mut host, 200);
    assert_eq!(shots(&host, 24).len(), 1, "the bomb was not pulled exactly once");
    assert!(ticks < 60, "pulling the bomb took {ticks} ticks (clip is 40)");
    Ok(())
}

#[test]
fn a_sword_swing_turns_the_trail_on_and_off_again() -> Result<()> {
    let Some(mut sim) = link()? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    settle(&mut sim, &mut host);
    assert_eq!(sim.afterimage, -1);
    // Neutral A from standing: the jab's script switches the trail on during the swing.
    step(&mut sim, &mut host, Input { buttons: libssbm::sim::ATTACK, aim: Vec3::X, ..Input::idle() });
    let mut on = false;
    for _ in 0..80 {
        step(&mut sim, &mut host, Input::idle());
        on |= sim.afterimage >= 0;
    }
    assert!(on, "the swing never turned the trail on");
    assert_eq!(sim.afterimage, -1, "the trail stayed on after the move");
    Ok(())
}

#[test]
fn the_trail_strip_fades_from_the_newest_edge_to_the_oldest() -> Result<()> {
    let dir = dir();
    if !LINK.complete(&dir) {
        eprintln!("SKIP: Link disc cache absent");
        return Ok(());
    }
    let (params, bone) = libssbm_assets::params::load_link_trail(&dir)?;
    assert!(bone > 0, "the trail follows a sword bone, not the root");
    // A blade sweeping a quarter turn over three ticks.
    let sample = |angle: f32| libssbm::trail::Sample {
        position: Vec3::new(angle.sin() * 5., 0., 0.),
        axis: Vec3::new(angle.cos(), angle.sin(), 0.),
    };
    let strip = libssbm::trail::strip(&[sample(1.2), sample(0.6), sample(0.)], &params);
    assert!(strip.len() >= 6 && strip.len() % 2 == 0, "strip of {}", strip.len());
    assert!(strip[0].color[3] >= strip[strip.len() - 1].color[3], "newest edge is the most opaque");
    assert_eq!(strip[0].color[..3], params.inner_color);
    assert_eq!(strip[1].color[..3], params.outer_color);
    assert!(libssbm::trail::strip(&[sample(0.)], &params).is_empty(), "one sample is not a strip");
    Ok(())
}
