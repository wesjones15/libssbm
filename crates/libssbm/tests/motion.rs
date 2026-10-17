use glam::Vec3;
use libssbm::{FlatHost, Input, JUMP, Params, SPRINT, Sim, motion::MotionDef};

fn fighter() -> Sim {
    let mut sim = Sim::new(Params::fixture(), Vec3::ZERO);
    for (name, frames, looping) in [
        ("Wait", 60., true),
        ("WalkSlow", 90., true),
        ("WalkMiddle", 50., true),
        ("WalkFast", 40., true),
        ("Run", 25., true),
        ("JumpF", 50., false),
        ("JumpAerialF", 50., false),
        ("Fall", 60., true),
        ("Landing", 4., false),
        ("KneeBend", 3., false),
    ] {
        sim.motion.definitions.insert(
            name.into(),
            MotionDef {
                frames,
                looping,
                root_forward: None,
            },
        );
    }
    sim
}

#[test]
fn gait_clock_matches_at_30_60_and_144_hz() {
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut sim = fighter();
        let mut host = FlatHost::default();
        for _ in 0..fps * 4 {
            sim.advance(
                &mut host,
                Input {
                    movement: Vec3::X,
                    buttons: SPRINT,
                    ..Input::idle()
                },
                1. / fps as f64,
            );
        }
        results.push((sim.frame, sim.motion.clip, sim.motion.frame, sim.position));
    }
    for value in &results[1..] {
        assert_eq!(value.0, results[0].0);
        assert_eq!(value.1, "Run");
        assert!((value.2 - results[0].2).abs() < 1e-4);
        assert!((value.3 - results[0].3).length() < 1e-4);
    }
}

#[test]
fn jump_pose_continues_after_apex_and_gaits_keep_cycle_phase() {
    let mut sim = fighter();
    let mut host = FlatHost::default();
    for _ in 0..5 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    sim.advance(
        &mut host,
        Input {
            buttons: JUMP,
            ..Input::idle()
        },
        1. / 60.,
    );
    for _ in 0..38 {
        sim.advance(
            &mut host,
            Input {
                buttons: JUMP,
                ..Input::idle()
            },
            1. / 60.,
        );
    }
    assert!(!sim.grounded);
    assert!(sim.velocity.y < 0.);
    assert_eq!(sim.motion.clip, "JumpF");
    assert!(sim.motion.frame > 20.);
    sim.motion.select("WalkSlow", 1.);
    sim.motion.frame = 45.;
    sim.motion.select("WalkFast", 2.);
    assert_eq!(sim.motion.frame, 20.);
    assert_eq!(sim.motion.render_frame(0.5), 21.);
}
