//! The Super Scope's disc data (ItCo.dat and PlCo.dat): skipped when the player has not extracted the items archive.
use libssbm_assets::{
    article::{Article, Item},
    params,
};
use std::path::PathBuf;

fn dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if libssbm_assets::disc::items_present(&dir) && dir.join("PlCo.dat").exists() {
        Some(dir)
    } else {
        eprintln!("SKIP: the items archive is not extracted");
        None
    }
}

#[test]
fn the_scope_tuning_is_the_discs() {
    let Some(dir) = dir() else { return };
    let p = params::load_scope(&dir).unwrap();
    assert_eq!(p.ammo, 48);
    assert_eq!((p.charge_floor, p.charge_levels), (5, 8));
    assert_eq!(p.costs, [1, 2, 3, 4, 6, 8, 10, 12, 14, 16]);
    println!("hold_max {} rapid_shots {} muzzle {:?}", p.hold_max, p.rapid_shots, p.muzzle);
    assert!(p.hold_max > 8. && p.hold_max < 1000.);
    assert!(p.rapid_shots > 0 && p.rapid_shots < 100);
}

#[test]
fn the_beam_has_ten_levels_that_grow() {
    let Some(dir) = dir() else { return };
    let beams = params::load_scope_beams(&dir).unwrap();
    for (level, b) in beams.iter().enumerate() {
        println!("level {level}: {b:?}");
        assert!(b.velocity > 0. && b.lifetime > 0. && b.scale > 0., "level {level}: {b:?}");
    }
    assert!(beams[9].scale > beams[0].scale);
    assert!(beams[9].damage >= beams[0].damage);
}

#[test]
fn the_scope_and_its_beam_models_load() {
    let Some(dir) = dir() else { return };
    let scope = Article::load_item(&dir, Item::SuperScope, 0).unwrap();
    assert!(!scope.model.batches.is_empty(), "the scope has no model");
    for level in 0..10 {
        let beam = Article::load_item(&dir, Item::SuperScopeBeam, level).unwrap();
        assert!(!beam.model.batches.is_empty(), "beam level {level} has no model");
    }
}

// ---- the Super Scope in Luigi's hands (against his real clips) --------------------------------------

mod in_hand {
    use glam::Vec3;
    use libssbm::{
        luigi::Luigi,
        sim::{EventKind, FlatHost, NEUTRAL_B},
        Input, Move, Sim,
    };
    use libssbm_assets::{animation::Animations, fighter::LUIGI};
    use std::path::PathBuf;

    fn luigi(ammo: Option<u32>) -> Option<(Sim, FlatHost)> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !LUIGI.complete(&dir) || !libssbm_assets::disc::items_present(&dir) {
            eprintln!("SKIP: Luigi or the items archive is not extracted");
            return None;
        }
        let params = libssbm_assets::params::load_common(&dir, &LUIGI).unwrap();
        let specials = libssbm_assets::params::load_luigi_specials(&dir).unwrap();
        let mut sim = Sim::with_behavior(params, Box::new(Luigi::new(specials)), Vec3::ZERO);
        let animations = Animations::load_for(&dir, &LUIGI).unwrap();
        sim.motion.definitions = animations.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, &LUIGI).unwrap();
        for (name, action) in &sim.action.definitions {
            if let Some(motion) = sim.motion.definitions.get_mut(name) {
                motion.looping |= action.looping;
            }
        }
        sim.grounded = true;
        sim.facing = Vec3::X;
        sim.set_scope(Some(libssbm_assets::params::load_scope(&dir).unwrap()));
        if let Some(ammo) = ammo {
            sim.scope.as_mut().unwrap().state.ammo = ammo;
        }
        let mut host = FlatHost::default();
        for _ in 0..4 {
            sim.advance(&mut host, Input::idle(), 1. / 60.);
        }
        Some((sim, host))
    }

    fn step(sim: &mut Sim, host: &mut FlatHost, held: bool) {
        let input = Input { buttons: if held { NEUTRAL_B } else { 0 }, aim: Vec3::X, ..Input::idle() };
        sim.advance(host, input, 1. / 60.);
    }

    /// The levels of the beams the host was told to fire, in order.
    fn shots(host: &FlatHost) -> Vec<u32> {
        host.events
            .iter()
            .filter(|e| e.kind == EventKind::Hit && e.move_id == Move::SuperScope as u32)
            .map(|e| e.strength as u32)
            .collect()
    }

    /// Holds the button for `hold` ticks, lets go, and plays the move out.
    fn use_scope(sim: &mut Sim, host: &mut FlatHost, hold: u32) {
        for _ in 0..hold {
            step(sim, host, true);
        }
        for _ in 0..240 {
            step(sim, host, false);
        }
    }

    #[test]
    fn a_tap_is_a_burst_of_small_shots() {
        let Some((mut sim, mut host)) = luigi(None) else { return };
        step(&mut sim, &mut host, true);
        assert_eq!(sim.current_move, Some(Move::SuperScope), "the button did not raise the scope");
        use_scope(&mut sim, &mut host, 0);
        let fired = shots(&host);
        // ftCommonData x5BC taps, counted down at the start of each loop: one fewer shots.
        assert_eq!(fired, vec![0; 2], "a tap's burst: {fired:?}");
        assert_eq!(sim.current_move, None, "the move did not end");
        assert_eq!(sim.scope.as_ref().unwrap().state.ammo, 48 - 2);
    }

    #[test]
    fn a_hold_cocks_once_then_charges_and_the_shot_stops_the_charge_cue() {
        let Some((mut sim, mut host)) = luigi(None) else { return };
        let mut cues: Vec<(u32, bool)> = Vec::new();
        for _ in 0..30 {
            step(&mut sim, &mut host, true);
            cues.extend(sim.action.sounds.iter().map(|c| (c.sound, c.stop)));
        }
        // The Start clip's own command: SFX_IT_SCORPE_KAMAE (0xFD), once.
        assert_eq!(cues.iter().filter(|(sound, stop)| *sound == 0xFD && !stop).count(), 1, "{cues:?}");
        // The disc's separate charge cue: SFX_IT_SCORPE_TAME (0xFE), once, when the clip ends.
        assert_eq!(cues.iter().filter(|(sound, stop)| *sound == 0xFE && !stop).count(), 1, "{cues:?}");
        // Firing stops it on its own track.
        for _ in 0..2 {
            step(&mut sim, &mut host, false);
            cues.extend(sim.action.sounds.iter().map(|c| (c.sound, c.stop)));
        }
        assert!(cues.iter().any(|(sound, stop)| *sound == 0xFE && *stop), "the charge cue was not stopped: {cues:?}");
    }

    #[test]
    fn a_hold_is_one_charged_shot_that_costs_more() {
        let Some((mut sim, mut host)) = luigi(None) else { return };
        use_scope(&mut sim, &mut host, 40);
        let fired = shots(&host);
        assert_eq!(fired.len(), 1, "a held shot fired {fired:?}");
        assert!((3..=6).contains(&fired[0]), "level {} for a 40-tick hold", fired[0]);
        let cost = [1, 2, 3, 4, 6, 8, 10, 12, 14, 16][fired[0] as usize];
        assert_eq!(sim.scope.as_ref().unwrap().state.ammo, 48 - cost);
    }

    #[test]
    fn the_longest_hold_is_the_top_level() {
        let Some((mut sim, mut host)) = luigi(None) else { return };
        use_scope(&mut sim, &mut host, 300);
        assert_eq!(shots(&host), vec![9]);
    }

    #[test]
    fn an_empty_scope_only_clicks() {
        let Some((mut sim, mut host)) = luigi(Some(0)) else { return };
        use_scope(&mut sim, &mut host, 0);
        assert!(shots(&host).is_empty(), "an empty scope fired");
    }

    #[test]
    fn the_scope_ends_back_in_idle_and_the_fireball_is_back_without_it() {
        let Some((mut sim, mut host)) = luigi(None) else { return };
        use_scope(&mut sim, &mut host, 12);
        assert_eq!(sim.current_move, None);
        sim.set_scope(None);
        step(&mut sim, &mut host, true);
        assert_eq!(sim.current_move, Some(Move::SpecialN), "without the scope the button is the Fireball");
    }
}
