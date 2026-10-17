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
