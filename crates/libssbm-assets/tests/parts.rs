//! Part animations (script command 41, ftAnim_ApplyPartAnim): the fighter's part table
//! loads, a playing part animation moves only the bones it keys, scripts start them, and
//! knockback swaps Fox head parts (skipped without the player's extracted cache).
use glam::Vec3;
use libssbm::{fox::Fox, sim::{FlatHost, PartPlay}, Input, Sim};
use libssbm_assets::{
    animation::{part_blend, Animations, PartAnimations},
    fighter::{FOX, YOSHI},
    presentation::Presentation,
    Result,
};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn fox_sim(dir: &std::path::Path) -> Result<Sim> {
    let params = libssbm_assets::params::load_common(dir, &FOX)?;
    let mut sim = Sim::with_behavior(
        params,
        Box::new(Fox::new(libssbm_assets::params::load_fox_specials(dir)?)),
        Vec3::ZERO,
    );
    let animations = Animations::load_for(dir, &FOX)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(dir, &animations, &FOX)?;
    sim.grounded = true;
    Ok(sim)
}

fn moved(a: &[glam::Mat4], b: &[glam::Mat4]) -> Vec<usize> {
    (0..a.len())
        .filter(|&i| !a[i].abs_diff_eq(b[i], 1e-4))
        .collect()
}

#[test]
fn the_part_tables_load_for_the_fighters_that_have_them() -> Result<()> {
    let dir = dir();
    for (name, f, slots) in [("fox", &FOX, [3usize, 4]), ("yoshi", &YOSHI, [3, 4])] {
        if !f.complete(&dir) {
            eprintln!("SKIP: {name} disc cache absent");
            return Ok(());
        }
        let parts = PartAnimations::load(&dir, f)?;
        assert_eq!(parts.slots.len(), 5);
        for slot in slots {
            assert!(parts.slots[slot].len() >= 4, "{name} slot {slot}: {} animations", parts.slots[slot].len());
        }
        // The hands: four animations each.
        assert!(parts.slots[0].len() >= 4 && parts.slots[1].len() >= 4, "{name} hand slots");
    }
    Ok(())
}

#[test]
fn a_playing_part_animation_moves_only_the_bones_it_keys() -> Result<()> {
    let dir = dir();
    if !FOX.complete(&dir) {
        eprintln!("SKIP: Fox disc cache absent");
        return Ok(());
    }
    let mut sim = fox_sim(&dir)?;
    let mut host = FlatHost { events: Vec::new() };
    for _ in 0..3 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    let mut presentation = Presentation::load_for(&dir, 0, &FOX)?;
    sim.parts = [None; 5];
    presentation.update(&sim, 0.);
    let plain = presentation.world.clone();
    // Fox hurt head parts (slot 3, animation 3) key bones 42 to 45.
    sim.parts[3] = Some(PartPlay { index: 3, age: 0., blend: 0 });
    presentation.update(&sim, 0.);
    let eyes = moved(&plain, &presentation.world);
    assert!(!eyes.is_empty(), "the part animation changed nothing");
    assert!(eyes.iter().all(|b| (42..=45).contains(b) || *b > 45), "unexpected bones moved: {eyes:?}");
    assert!(eyes.contains(&42), "the first eye bone did not move: {eyes:?}");
    // Easing in: with blend frames the first tick has moved less than a full tick later.
    sim.parts[3] = Some(PartPlay { index: 3, age: 0., blend: 8 });
    presentation.update(&sim, 0.);
    let eased = presentation.world[42];
    sim.parts[3] = Some(PartPlay { index: 3, age: 6., blend: 8 });
    presentation.update(&sim, 0.);
    let later = presentation.world[42];
    let full = {
        sim.parts[3] = Some(PartPlay { index: 3, age: 0., blend: 0 });
        presentation.update(&sim, 0.);
        presentation.world[42]
    };
    let distance = |a: glam::Mat4, b: glam::Mat4| a.to_cols_array().iter().zip(b.to_cols_array()).map(|(x, y)| (x - y).abs()).sum::<f32>();
    assert!(distance(eased, plain[42]) < distance(later, plain[42]) + 1e-6, "no ease-in");
    assert!(distance(later, full) <= distance(eased, full) + 1e-6, "the blend did not approach the animation");
    Ok(())
}

#[test]
fn the_blend_weight_eases_in_and_ends_at_one() {
    assert_eq!(part_blend(0., 0., 10.), 1., "no blend frames: all at once");
    let ticks: Vec<f32> = (0..8).map(|t| part_blend(t as f32, 4., 10.)).collect();
    assert!(ticks.windows(2).all(|w| w[1] >= w[0]), "not monotonic: {ticks:?}");
    assert_eq!(ticks[7], 1.);
    assert!(ticks[1] > 0. && ticks[1] < 1.);
}

#[test]
fn a_script_starts_a_head_part_animation_and_knockback_swaps_it() -> Result<()> {
    let dir = dir();
    if !FOX.complete(&dir) {
        eprintln!("SKIP: Fox disc cache absent");
        return Ok(());
    }
    let mut sim = fox_sim(&dir)?;
    let mut host = FlatHost { events: Vec::new() };
    // Wait starts one at frame 25 (slot 3, animation 2) and other frames set others.
    let mut seen = false;
    for _ in 0..600 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        if sim.parts[3].is_some() && sim.parts[4].is_some() {
            seen = true;
            break;
        }
    }
    assert!(seen, "Wait never started a head-part animation");
    // A hit puts him in knockback: animation 3 in knockback, 2 afterwards.
    sim.hurt();
    sim.advance(&mut host, Input::idle(), 1. / 60.);
    assert!(sim.reaction.is_some());
    assert_eq!(sim.parts[3].map(|p| p.index), Some(3), "no hurt head parts in knockback");
    assert_eq!(sim.parts[4].map(|p| p.index), Some(3));
    let mut recovered = false;
    for _ in 0..600 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        if sim.reaction.is_none() {
            recovered = true;
            break;
        }
    }
    assert!(recovered, "never left knockback");
    assert_eq!(sim.parts[3].map(|p| p.index).filter(|i| *i == 3), None, "still hurt after recovering");
    Ok(())
}
