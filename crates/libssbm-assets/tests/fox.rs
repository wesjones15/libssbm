//! Fox's staged specials against his real disc clips (skipped without the
//! player's extracted cache): the Blaster fires and repeats, the Illusion dash
//! carries its hitbox, Fire Fox holds in place then launches along the aim, and
//! the Reflector announces its burst. Each move plays its clips once.
use glam::Vec3;
use libssbm::{
    effect::{attached, resolve, Target},
    fox::Fox,
    sim::{EventKind, FlatHost, DOWN_B, JUMP, NEUTRAL_B, SIDE_B, UP_B},
    Input, Sim,
};
use libssbm_assets::{Result, animation::Animations, fighter::FOX, presentation::Presentation};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn fox_at(grounded: bool) -> Result<Option<Sim>> {
    let dir = dir();
    if !FOX.complete(&dir) {
        eprintln!("SKIP: Fox disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, &FOX)?;
    let specials = libssbm_assets::params::load_fox_specials(&dir)?;
    let position = if grounded { Vec3::ZERO } else { Vec3::new(0., 200., 0.) };
    let mut sim = Sim::with_behavior(params, Box::new(Fox::new(specials)), position);
    let animations = Animations::load_for(&dir, &FOX)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &FOX)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.colanim.table = Some(libssbm_assets::params::load_colanim(&dir)?);
    sim.grounded = grounded;
    sim.facing = Vec3::X;
    Ok(Some(sim))
}

fn input(buttons: u32, aim: Vec3) -> Input {
    Input { buttons, aim, ..Input::idle() }
}
fn step(sim: &mut Sim, host: &mut FlatHost, i: Input) {
    sim.advance(host, i, 1. / 60.);
}
fn shots(host: &FlatHost) -> usize {
    host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 20).count()
}

#[test]
fn blaster_fires_once_per_tap_and_plays_its_clips_once() -> Result<()> {
    let Some(mut sim) = fox_at(true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(NEUTRAL_B, Vec3::X));
    let mut ticks = 0;
    while sim.current_move.is_some() && ticks < 200 {
        step(&mut sim, &mut host, Input::idle());
        ticks += 1;
    }
    assert!(sim.current_move.is_none(), "blaster never ended");
    assert_eq!(shots(&host), 1, "one tap, one shot");
    // Start 7 + Loop 10 + End 24 frames, not doubled.
    assert!(ticks < 60, "blaster took {ticks} ticks (clip chain is 41)");
    Ok(())
}

#[test]
fn blaster_keeps_firing_while_the_button_is_held() -> Result<()> {
    let Some(mut sim) = fox_at(true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    for _ in 0..90 {
        step(&mut sim, &mut host, input(NEUTRAL_B, Vec3::X));
    }
    assert!(shots(&host) >= 5, "held blaster fired {} shots", shots(&host));
    Ok(())
}

#[test]
fn illusion_dashes_through_a_hitbox_and_ends() -> Result<()> {
    let Some(mut sim) = fox_at(true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(SIDE_B, Vec3::X));
    let mut boxed = false;
    let mut streak = false;
    for _ in 0..200 {
        step(&mut sim, &mut host, Input::idle());
        boxed |= sim.action.samples.iter().any(|s| s.hitbox.damage >= 7. && s.hitbox.element == 3);
        streak |= sim.action.effects.iter().any(|cue| cue.id == 1165);
        if sim.current_move.is_none() {
            break;
        }
    }
    assert!(sim.current_move.is_none(), "illusion never ended");
    assert!(boxed, "no dash hitbox was ever active");
    assert!(streak, "the Illusion's 0x48D streak was never spawned");
    // ftFx_SpecialS_CreateGFX attaches efSync 0x48D to FtPart_TopN; a one-shot
    // world effect would leave the blue streak behind when Fox dashes.
    assert!(attached(1165), "the Illusion streak is not joint-bound");
    assert_eq!(resolve(1165), Target::Particle(3008));
    assert!(sim.position.x > 80., "illusion covered only {}", sim.position.x);
    Ok(())
}

#[test]
fn illusion_ghost_is_a_translucent_blue_fox_trail() -> Result<()> {
    let dir = dir();
    // The ghost is the It_Kind_Fox_Illusion article in the item archive.
    if !dir.join("IfAll.usd").exists() {
        eprintln!("SKIP: item archive absent");
        return Ok(());
    }
    let ghost = Presentation::article_for(&dir, 2, 0, &FOX)?;
    let batch = ghost.model.batches.first().ok_or("empty ghost model")?;
    assert_eq!(batch.blend, 2, "the ghost is not translucent");
    assert!(batch.color[3] < 1., "the ghost is not faded: {:?}", batch.color);
    assert!(batch.color[2] > batch.color[0], "the ghost is not blue: {:?}", batch.color);
    assert!(
        ghost.model.batches.iter().map(|b| b.vertices.len()).sum::<usize>() > 6000,
        "the ghost model is not the full trailing body"
    );
    Ok(())
}

#[test]
fn blaster_bolt_keeps_its_source_red_vertex_colour() -> Result<()> {
    let dir = dir();
    if !dir.join("IfAll.usd").exists() {
        eprintln!("SKIP: item archive absent");
        return Ok(());
    }
    let bolt = Presentation::article_for(&dir, 0, 0, &FOX)?;
    let core = bolt.model.batches.first().ok_or("empty Blaster bolt")?;
    assert!(core.color[0] > 0.9 && core.color[1] < 0.1 && core.color[2] < 0.1, "bolt is not red: {:?}", core.color);
    // The red is a CLR0 vertex attribute on the untextured bolt.
    assert!(core.vertices.iter().all(|v| v.color[0] > 0.9 && v.color[1] < 0.1), "bolt vertex colour lost");
    Ok(())
}

#[test]
fn fire_fox_holds_in_place_then_launches_along_the_aim() -> Result<()> {
    let Some(mut sim) = fox_at(false)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    let start = sim.position;
    let aim = Vec3::new(1., 0.4, 0.).normalize();
    step(&mut sim, &mut host, input(UP_B, aim));
    // Stage one: he hangs in space for the charge.
    for _ in 0..30 {
        step(&mut sim, &mut host, input(0, aim));
    }
    assert!((sim.position - start).length() < 1., "moved during the charge: {:?}", sim.position - start);
    let mut strongest = 0f32;
    let mut launch_flame = false;
    for _ in 0..120 {
        step(&mut sim, &mut host, input(0, aim));
        for s in &sim.action.samples {
            strongest = strongest.max(s.hitbox.damage);
        }
        for cue in &sim.action.effects {
            if cue.id == 1164 {
                launch_flame = true;
                assert!(cue.destroy_on_change, "Fire Fox's launch flame is not move-bound");
                assert_eq!(cue.clip, "SpecialHi", "launch flame spawned on the hold clip");
            }
        }
        if sim.current_move.is_none() {
            break;
        }
    }
    let travelled = sim.position - start;
    assert!(travelled.x > 100. && travelled.y > 30., "fire fox travelled {travelled:?}");
    assert!(travelled.y < travelled.x, "launch did not follow the aim {travelled:?}");
    assert!(strongest >= 19., "launch hit only {strongest}%");
    assert!(launch_flame, "no Fire Fox launch flame was spawned");
    let impacts = host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 23 && e.strength >= 2.5).count();
    assert_eq!(impacts, 1, "Fire Fox did not emit exactly one impact burst");
    Ok(())
}

#[test]
fn reflector_announces_its_burst_and_releases() -> Result<()> {
    let Some(mut sim) = fox_at(true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
    for _ in 0..60 {
        step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
    }
    assert!(sim.current_move.is_some(), "reflector ended while held");
    let bursts = host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 24).count();
    assert_eq!(bursts, 1, "burst events");
    for _ in 0..80 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(sim.current_move.is_none(), "reflector never released");
    Ok(())
}

#[test]
fn reflector_event_carries_the_reflect_desc_and_ends_with_a_release() -> Result<()> {
    let Some(mut sim) = fox_at(true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
    for _ in 0..40 {
        step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
    }
    for _ in 0..80 {
        step(&mut sim, &mut host, Input::idle());
    }
    let events: Vec<_> = host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 24).collect();
    assert_eq!(events.len(), 2, "a start and a release event");
    assert_eq!(events[0].strength, 1.);
    assert!(events[0].direction.x > 0. && events[0].direction.y >= 1., "reflect desc {:?}", events[0].direction);
    assert_eq!(events[1].strength, 2.);
    Ok(())
}

#[test]
fn jump_cancels_the_reflector_on_the_ground_and_in_the_air() -> Result<()> {
    for grounded in [true, false] {
        let Some(mut sim) = fox_at(grounded)? else { return Ok(()) };
        let mut host = FlatHost { events: Vec::new() };
        for _ in 0..4 {
            step(&mut sim, &mut host, Input::idle());
        }
        step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
        for _ in 0..12 {
            step(&mut sim, &mut host, input(DOWN_B, Vec3::X));
        }
        assert!(sim.current_move.is_some(), "shine ended before the jump");
        let before = sim.velocity.y;
        // A fresh press of jump while the button is still held.
        step(&mut sim, &mut host, input(DOWN_B | JUMP, Vec3::X));
        assert!(sim.current_move.is_none(), "jump did not cancel the shine (grounded {grounded})");
        for _ in 0..12 {
            step(&mut sim, &mut host, Input::idle());
        }
        if grounded {
            assert!(sim.velocity.y > 0. || !sim.grounded, "no jump out of the shine");
        } else {
            assert!(sim.velocity.y > before, "no air jump out of the shine");
        }
        let releases = host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 24 && e.strength == 2.).count();
        assert_eq!(releases, 1, "the reflect should be announced down once");
    }
    Ok(())
}

#[test]
fn fire_fox_glows_while_it_charges_and_the_glow_ends_with_the_move() -> Result<()> {
    let Some(mut sim) = fox_at(true)? else { return Ok(()) };
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..4 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(sim.colanim.overlay().is_none());
    step(&mut sim, &mut host, input(UP_B, Vec3::Y));
    let mut glowed = false;
    for _ in 0..120 {
        step(&mut sim, &mut host, input(UP_B, Vec3::Y));
        glowed |= sim.colanim.overlay().is_some();
    }
    assert!(glowed, "Fire Fox never glowed");
    for _ in 0..200 {
        step(&mut sim, &mut host, Input::idle());
    }
    assert!(sim.current_move.is_none());
    assert!(sim.colanim.overlay().is_none(), "the glow outlived the move");
    Ok(())
}
