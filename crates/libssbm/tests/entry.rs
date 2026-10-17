use libssbm::{ATTACK, FlatHost, Input, Params, SPECIAL, Sim};
#[test]
fn entry_blocks_actions_and_uses_fixed_ticks_for_grow_and_release() {
    for fps in [30, 60, 144] {
        let mut sim = Sim::new(Params::fixture(), glam::Vec3::ZERO);
        sim.entry = Some(libssbm::entry::State::new(libssbm::entry::Params {
            grow: 30,
            release: 30,
            flat: 0.01,
            platform: 1.,
            color: [1., 1., 1., 0.5],
            color_frames: 100,
        }));
        let mut host = FlatHost::default();
        let mut input = Input::idle();
        input.buttons = ATTACK | SPECIAL;
        for _ in 0..fps {
            sim.advance(&mut host, input, 1. / fps as f64);
        }
        assert!(sim.entry.is_none());
        assert!(sim.current_move.is_none());
        assert_eq!(sim.position, glam::Vec3::ZERO);
    }
}
#[test]
fn figurine_waits_for_interact_then_releases_without_shrinking_again() {
    let mut sim = Sim::new(Params::fixture(), glam::Vec3::ZERO);
    let mut entry = libssbm::entry::State::new(libssbm::entry::Params {
        grow: 30,
        release: 30,
        flat: 0.01,
        platform: 1.,
        color: [1.; 4],
        color_frames: 100,
    });
    entry.waiting = true;
    sim.entry = Some(entry);
    let mut host = FlatHost::default();
    for _ in 0..600 {
        sim.advance(
            &mut host,
            Input {
                buttons: ATTACK | SPECIAL,
                ..Input::idle()
            },
            1. / 60.,
        );
    }
    let entry = sim.entry.as_mut().unwrap();
    assert_eq!(entry.frame, 30);
    assert!(!entry.started);
    assert_eq!(entry.transform().x_axis.x, 1.);
    assert_eq!(entry.transform().y_axis.y, 1.);
    entry.waiting = false;
    let mut sounds = 0;
    for _ in 0..30 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        sounds += sim.action.sounds.iter().filter(|s| s.sound == 139).count();
    }
    assert!(sim.entry.is_none());
    assert_eq!(sounds, 1);
}
