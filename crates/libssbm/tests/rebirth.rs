use glam::Vec3;
use libssbm::rebirth::{Params as RebirthParams, Phase};
use libssbm::{ATTACK, FlatHost, Input, Params, SPECIAL, Sim};
fn setup() -> (Sim, RebirthParams, libssbm::entry::Params) {
    (
        Sim::new(Params::fixture(), Vec3::new(10., -200., 0.)),
        RebirthParams {
            ko_frames: 6,
            arrival_frames: 6,
            wait_frames: 12,
            invulnerable_frames: 8,
            ko_scale: 0.8,
            flash: vec![[1.; 4], [0.; 4]],
        },
        libssbm::entry::Params {
            grow: 3,
            release: 3,
            flat: 0.01,
            platform: 1.,
            color: [1.; 4],
            color_frames: 20,
        },
    )
}
#[test]
fn common_rebirth_sequence_is_fixed_rate_and_has_one_ko_and_transform() {
    for fps in [30, 60, 144] {
        let (mut sim, p, e) = setup();
        let mut host = FlatHost::default();
        let destination = Vec3::new(5., 0., 3.);
        assert!(sim.begin_rebirth(p.clone(), e, destination));
        assert!(!sim.begin_rebirth(p, e, Vec3::ZERO));
        let mut ko = 0;
        let mut entry = 0;
        let mut sound = 0;
        for _ in 0..fps * 2 {
            sim.advance(
                &mut host,
                Input {
                    buttons: ATTACK | SPECIAL,
                    ..Input::idle()
                },
                1. / fps as f64,
            );
            for cue in &sim.action.effects {
                if cue.id == 0x42b {
                    ko += 1;
                    // The RoR2 adaptation plays the KO flash where the fighter returns, not at the
                    // point it left the stage.
                    assert_eq!(cue.origin, destination);
                }
                if cue.id == 0x43e {
                    entry += 1;
                    assert_eq!(cue.origin, destination);
                }
            }
            sound += sim.action.sounds.iter().filter(|s| s.sound == 0x61).count();
        }
        assert_eq!((ko, entry, sound), (1, 1, 1), "cues at {fps} FPS");
        assert_eq!(sim.frame, 120);
        assert_eq!(sim.position, destination);
        assert!(sim.rebirth.is_none() && sim.entry.is_none() && sim.current_move.is_none());
    }
}
#[test]
fn movement_releases_figurine_after_arrival_and_protection_expires_exactly() {
    let (mut sim, p, e) = setup();
    let mut host = FlatHost::default();
    sim.begin_rebirth(p.clone(), e, Vec3::ZERO);
    for _ in 0..p.ko_frames {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    assert_eq!(sim.position, Vec3::ZERO);
    assert_eq!(sim.rebirth.as_ref().unwrap().phase, Phase::Figurine);
    for _ in 0..p.arrival_frames - 1 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    assert!(sim.entry.unwrap().waiting);
    sim.advance(
        &mut host,
        Input {
            movement: Vec3::X,
            ..Input::idle()
        },
        1. / 60.,
    );
    assert_eq!(sim.rebirth.as_ref().unwrap().phase, Phase::Transform);
    while sim.rebirth.as_ref().unwrap().phase != Phase::Protected {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    assert_eq!(sim.rebirth.as_ref().unwrap().frame, 0);
    for _ in 0..p.invulnerable_frames - 1 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        assert_eq!(sim.rebirth.as_ref().unwrap().flags(), 2);
    }
    sim.advance(&mut host, Input::idle(), 1. / 60.);
    assert!(sim.rebirth.is_none());
}
#[test]
fn another_fall_during_invulnerability_can_recover_and_external_teleport_cleans_up() {
    let (mut sim, p, e) = setup();
    let mut host = FlatHost::default();
    sim.begin_rebirth(p.clone(), e, Vec3::ZERO);
    while sim.rebirth.as_ref().unwrap().phase != Phase::Protected {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    let remaining = sim.rebirth.as_ref().unwrap().frame;
    sim.correct_position(Vec3::X);
    assert_eq!(sim.rebirth.as_ref().unwrap().phase, Phase::Protected);
    assert_eq!(sim.rebirth.as_ref().unwrap().frame, remaining);
    assert!(sim.begin_rebirth(p, e, Vec3::Y));
    assert_eq!(sim.rebirth.as_ref().unwrap().phase, Phase::Ko);
    sim.teleport(Vec3::X);
    assert!(sim.rebirth.is_none() && sim.entry.is_none());
}

#[test]
fn initial_arrival_reuses_return_without_ko_or_interaction_at_all_rates() {
    for fps in [30, 60, 144] {
        for moving in [false, true] {
            let (mut sim, p, e) = setup();
            let mut host = FlatHost::default();
            sim.teleport(Vec3::ZERO);
            assert!(sim.begin_arrival(p, e));
            assert_eq!(sim.rebirth.as_ref().unwrap().phase, Phase::Figurine);
            let mut ko = 0;
            let mut transform = 0;
            let mut protected = false;
            for _ in 0..fps * 2 {
                let input = Input {
                    movement: if moving { Vec3::X } else { Vec3::ZERO },
                    ..Input::idle()
                };
                sim.advance(&mut host, input, 1. / fps as f64);
                ko += sim
                    .action
                    .effects
                    .iter()
                    .filter(|cue| cue.id == 0x42b)
                    .count();
                transform += sim
                    .action
                    .effects
                    .iter()
                    .filter(|cue| cue.id == 0x43e)
                    .count();
                assert!(!sim.action.sounds.iter().any(|cue| cue.sound == 0x61));
                protected |= sim
                    .rebirth
                    .as_ref()
                    .is_some_and(|state| state.phase == Phase::Protected);
            }
            assert_eq!((ko, transform), (0, 1));
            assert!(protected);
            assert!(sim.entry.is_none() && sim.rebirth.is_none());
        }
    }
}
