use glam::Vec3;
use libssbm::{
    FlatHost, Input, JUMP, Move, NEUTRAL_B, Params, Phase, SPRINT, Sim, motion::MotionDef,
};

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

#[test]
fn mobile_neutral_b_walks_on_an_independent_gait_and_sprint_stores_charge() {
    for fps in [30, 60, 144] {
        let mut sim = fighter();
        sim.options.mobile_moves = 1 << Move::ChargeShot as u32;
        let mut host = FlatHost::default();
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        let start = sim.position.x;
        let walk = Input {
            movement: Vec3::X,
            buttons: NEUTRAL_B,
            ..Input::idle()
        };
        for _ in 0..fps * 7 / 20 {
            sim.advance(&mut host, walk, 1. / fps as f64);
        }
        assert_eq!(sim.current_move, Some(Move::ChargeShot));
        assert_eq!(sim.phase, Phase::ChargeShot);
        assert!(
            sim.position.x > start + 1.,
            "fps={fps}: no walk during charge"
        );
        assert!(sim.locomotion.clip.starts_with("Walk"));
        assert!(sim.locomotion_weight > 0.99);
        let charge = sim.shot_charge;
        assert!(charge > 0);
        sim.advance(
            &mut host,
            Input {
                movement: Vec3::X,
                buttons: SPRINT,
                ..Input::idle()
            },
            1. / 30.,
        );
        assert_eq!(sim.phase, Phase::Free);
        assert_eq!(sim.current_move, None);
        assert_eq!(sim.shot_charge, charge);
        assert_eq!(sim.motion.clip, "Run");
        assert!((sim.motion.frame - sim.locomotion.frame).abs() < 1e-4);
    }
}
