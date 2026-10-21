//! The jab chain against real disc clips (skipped without the player's extracted cache):
//! jab 1, 2 and 3 on taps inside the combo window, the rapid jab that presses and releases of A
//! start once a script allows it (ftCo_Attack_800D6A50) and that goes on only while A keeps
//! moving (ftCo_Attack100Loop_Anim), and the chains that stop short for fighters with no third
//! jab or no rapid jab.
use glam::Vec3;
use libssbm::{
    sim::{FlatHost, ATTACK},
    Input, Move, Sim,
};
use libssbm_assets::{
    animation::Animations,
    fighter::{FighterAssets, CAPTAIN, DONKEY, FOX, LINK, PIKACHU, YOSHI},
    Result,
};
use std::collections::BTreeSet;

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn fighter(f: &FighterAssets<'_>) -> Result<Option<(Sim, FlatHost)>> {
    let dir = dir();
    if !f.complete(&dir) {
        eprintln!("SKIP: disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, f)?;
    let mut sim = Sim::new(params, Vec3::ZERO);
    let animations = Animations::load_for(&dir, f)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, f)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = true;
    sim.facing = Vec3::X;
    let mut host = FlatHost::default();
    for _ in 0..4 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    Ok(Some((sim, host)))
}

fn tick(sim: &mut Sim, host: &mut FlatHost, down: bool) {
    let input = Input { buttons: if down { ATTACK } else { 0 }, aim: Vec3::X, ..Input::idle() };
    sim.advance(host, input, 1. / 60.);
}

/// Taps A (one tick down, `period - 1` up) `taps` times, then lets go for `after` ticks. The moves
/// the sim passed through, in order, and the hitbox generations seen while the rapid jab ran.
fn mash(sim: &mut Sim, host: &mut FlatHost, period: u32, taps: u32, after: u32) -> (Vec<Move>, usize) {
    let mut moves = Vec::new();
    let mut generations = BTreeSet::new();
    for t in 0..period * taps + after {
        tick(sim, host, t < period * taps && t % period == 0);
        if let Some(m) = sim.current_move {
            if moves.last() != Some(&m) {
                moves.push(m);
            }
        }
        if sim.current_move == Some(Move::RapidJab) {
            for s in &sim.action.samples {
                generations.insert((s.hitbox.id, s.generation));
            }
        }
    }
    (moves, generations.len())
}

#[test]
fn fox_third_tap_goes_into_the_rapid_jab_and_stopping_ends_it() -> Result<()> {
    let Some((mut sim, mut host)) = fighter(&FOX)? else { return Ok(()) };
    let (moves, hits) = mash(&mut sim, &mut host, 12, 3, 0);
    assert_eq!(moves, [Move::Jab, Move::Jab2, Move::RapidJab], "A, A, A");
    // Tapping on keeps it going well past one trip round the loop; letting go ends it.
    let (_, more) = mash(&mut sim, &mut host, 6, 12, 0);
    assert_eq!(sim.current_move, Some(Move::RapidJab), "still tapping");
    assert!(hits + more >= 3, "the loop hits again each round ({} hitboxes)", hits + more);
    for _ in 0..120 {
        tick(&mut sim, &mut host, false);
    }
    assert!(sim.current_move.is_none(), "the rapid jab never ended");
    Ok(())
}

#[test]
fn holding_a_is_one_jab_not_a_rapid_jab() -> Result<()> {
    let Some((mut sim, mut host)) = fighter(&FOX)? else { return Ok(()) };
    let mut moves = Vec::new();
    for _ in 0..90 {
        tick(&mut sim, &mut host, true);
        if let Some(m) = sim.current_move {
            if moves.last() != Some(&m) {
                moves.push(m);
            }
        }
    }
    assert_eq!(moves, [Move::Jab], "a held A is a single jab");
    Ok(())
}

#[test]
fn the_rapid_jab_loop_ends_when_the_taps_stop_between_beats() -> Result<()> {
    let Some((mut sim, mut host)) = fighter(&FOX)? else { return Ok(()) };
    mash(&mut sim, &mut host, 6, 6, 0);
    assert_eq!(sim.current_move, Some(Move::RapidJab));
    // A slow tap (every 40 ticks) misses beats that come every seven.
    let mut ended = false;
    for t in 0..200 {
        tick(&mut sim, &mut host, t % 40 == 0);
        if sim.current_move != Some(Move::RapidJab) {
            ended = true;
            break;
        }
    }
    assert!(ended, "a slow tap kept the rapid jab going");
    Ok(())
}

#[test]
fn falcon_chains_three_jabs_and_link_needs_a_quick_start_for_the_rapid_jab() -> Result<()> {
    let Some((mut sim, mut host)) = fighter(&CAPTAIN)? else { return Ok(()) };
    let (moves, _) = mash(&mut sim, &mut host, 12, 3, 80);
    assert_eq!(&moves[..3], [Move::Jab, Move::Jab2, Move::Jab3], "Falcon's three jabs");
    let Some((mut sim, mut host)) = fighter(&LINK)? else { return Ok(()) };
    let (slow, _) = mash(&mut sim, &mut host, 12, 3, 80);
    assert_eq!(slow, [Move::Jab, Move::Jab2, Move::Jab3], "slow taps: no rapid jab for Link");
    let Some((mut sim, mut host)) = fighter(&LINK)? else { return Ok(()) };
    let (quick, _) = mash(&mut sim, &mut host, 5, 6, 80);
    assert!(quick.contains(&Move::RapidJab), "quick taps start Link's rapid jab: {quick:?}");
    Ok(())
}

#[test]
fn fighters_without_a_third_jab_or_a_rapid_jab_stop_where_their_clips_do() -> Result<()> {
    for f in [&YOSHI, &DONKEY] {
        let Some((mut sim, mut host)) = fighter(f)? else { return Ok(()) };
        let (moves, _) = mash(&mut sim, &mut host, 4, 4, 100);
        assert!(
            !moves.contains(&Move::Jab3) && !moves.contains(&Move::RapidJab),
            "{moves:?}"
        );
        assert!(moves.contains(&Move::Jab2));
    }
    // Pikachu has one jab clip: a second press plays it again.
    let Some((mut sim, mut host)) = fighter(&PIKACHU)? else { return Ok(()) };
    let (moves, _) = mash(&mut sim, &mut host, 12, 4, 100);
    assert_eq!(moves, [Move::Jab]);
    let starts = host
        .events
        .iter()
        .filter(|e| e.kind == libssbm::EventKind::Started && e.move_id == Move::Jab as u32)
        .count();
    assert!(starts >= 3, "Pikachu's jab started {starts} times for 4 taps");
    Ok(())
}
