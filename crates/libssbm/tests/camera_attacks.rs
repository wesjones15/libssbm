//! Host camera policy; original attacks/command timing remain disc driven.
use glam::Vec3;
use libssbm::*;

fn input(pitch: f32, movement: bool, buttons: u32) -> Input {
    let angle = pitch.to_radians();
    Input {
        aim: Vec3::new(0., angle.sin(), angle.cos()),
        movement: if movement { Vec3::Z } else { Vec3::ZERO },
        buttons,
        allowed: 31,
    }
}
fn select(p: Input, air: bool, held: bool, fps: u32) -> Move {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    let mut host = FlatHost::default();
    s.advance(&mut host, Input::idle(), 1. / 60.);
    if held {
        for _ in 0..30 {
            s.advance(
                &mut host,
                Input {
                    buttons: p.buttons & !(ATTACK | SPECIAL),
                    ..p
                },
                1. / 60.,
            );
        }
    }
    if air {
        s.teleport(Vec3::Y * 100.);
    }
    for _ in 0..fps / 30 {
        s.advance(&mut host, p, 1. / fps as f64);
    }
    s.current_move.unwrap()
}

#[test]
fn directional_camera_bands_and_shoulders_are_consistent_at_render_rates() {
    for fps in [30, 60, 144] {
        for (pitch, moving, button, air, held, want) in [
            (30., true, ATTACK, false, false, Move::UpSmash),
            (30., true, ATTACK, false, true, Move::UpTilt),
            (30., true, ATTACK, true, false, Move::UpAir),
            (-40., true, ATTACK, false, true, Move::SideTilt),
            (-60., true, ATTACK, false, false, Move::DownSmash),
            (-60., true, ATTACK, false, true, Move::DownTilt),
            (-60., true, ATTACK, true, false, Move::DownAir),
            (80., false, ATTACK, false, false, Move::Jab),
            (80., false, ATTACK, true, false, Move::NeutralAir),
            (30., true, SPECIAL, false, false, Move::SpecialHi),
            (-60., true, SPECIAL, false, false, Move::SpecialLw),
            (80., false, SPECIAL, false, false, Move::SpecialN),
            (-80., true, ATTACK | UP, true, false, Move::UpAir),
            (80., true, ATTACK | DOWN, true, false, Move::DownAir),
            (-80., true, SPECIAL | UP, false, false, Move::SpecialHi),
            (80., true, SPECIAL | DOWN, false, false, Move::SpecialLw),
        ] {
            assert_eq!(
                select(input(pitch, moving, button), air, held, fps),
                want,
                "fps={fps}, pitch={pitch}, button={button}"
            );
        }
    }
}

#[test]
fn changing_camera_band_with_held_stick_counts_as_new_smash_direction() {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    let mut host = FlatHost::default();
    for _ in 0..30 {
        s.advance(&mut host, input(0., true, 0), 1. / 60.);
    }
    s.advance(&mut host, input(30., true, ATTACK), 1. / 60.);
    assert_eq!(s.current_move, Some(Move::UpSmash));
}

#[test]
fn throw_axes_ignore_camera_pitch() {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    let mut host = FlatHost::default();
    s.action.definitions.insert(
        "CatchWait".into(),
        libssbm::action::ActionDef {
            frames: 60.,
            looping: true,
            commands: Vec::new(),
        },
    );
    s.set_grabbed(true);
    s.advance(
        &mut host,
        Input {
            movement: Vec3::Y,
            ..input(-80., false, ATTACK)
        },
        1. / 60.,
    );
    assert_eq!(s.current_move, Some(Move::ThrowUp));
}

#[test]
fn host_yaw_faces_body_without_rotating_drift_or_latched_attacks() {
    for fps in [30, 60, 144] {
        let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
        let mut host = FlatHost::default();
        let mut p = Input {
            aim: Vec3::Z,
            movement: Vec3::X,
            allowed: 31,
            buttons: 0,
        };
        s.set_host_facing(Vec3::Z);
        for _ in 0..fps / 2 {
            s.advance(&mut host, p, 1. / fps as f64);
        }
        assert!(
            s.facing.dot(Vec3::Z) > 0.999 && s.position.x > 0. && s.position.z.abs() < 1e-4,
            "strafe must retain world drift"
        );
        p.movement = -Vec3::Z;
        for _ in 0..fps / 2 {
            s.advance(&mut host, p, 1. / fps as f64);
        }
        assert!(
            s.facing.dot(Vec3::Z) > 0.999 && s.velocity.z < 0.,
            "backpedal must face camera"
        );
        s.teleport(Vec3::Y * 200.);
        p.aim = Vec3::X;
        p.movement = -Vec3::X;
        s.set_host_facing(Vec3::X);
        for _ in 0..fps / 30 {
            s.advance(&mut host, p, 1. / fps as f64);
        }
        assert!(
            s.facing.dot(Vec3::X) > 0.999,
            "free aerial facing did not track yaw"
        );
        p.buttons = ATTACK;
        for _ in 0..fps / 30 {
            s.advance(&mut host, p, 1. / fps as f64);
        }
        assert_eq!(s.current_move, Some(Move::BackAir));
        p.aim = Vec3::Z;
        s.set_host_facing(Vec3::Z);
        for _ in 0..fps / 30 {
            s.advance(&mut host, p, 1. / fps as f64);
        }
        assert!(
            s.facing.dot(Vec3::X) > 0.999,
            "camera changed latched aerial heading"
        );
    }
}

#[test]
fn stand_in_strike_pose_follows_the_authored_jab_clock() {
    use libssbm::{builtin, sim::ATTACK, FlatHost, Input, Phase};
    let mut s = builtin::sim(0, glam::Vec3::ZERO).unwrap();
    let mut h = FlatHost::default();
    for _ in 0..30 {
        s.advance(&mut h, Input::idle(), 1. / 60.);
    }
    assert_eq!(s.strike_pose_pitch(), None, "standing is upright");
    let press = Input { buttons: ATTACK, ..Input::idle() };
    s.advance(&mut h, press, 1. / 60.);
    let mut pitches = vec![s.strike_pose_pitch().expect("jab strikes")];
    for _ in 0..17 {
        s.advance(&mut h, Input::idle(), 1. / 60.);
        pitches.push(s.strike_pose_pitch().unwrap_or(0.));
    }
    let peak = pitches.iter().cloned().fold(0., f32::max);
    assert!((peak - std::f32::consts::FRAC_PI_4).abs() < 1e-5, "peak {peak}: {pitches:?}");
    let at_peak = pitches.iter().position(|p| *p == peak).unwrap();
    // The jab's first hitbox is on motion frame 3; the tick the clip shows frame 3 is the peak.
    assert_eq!(at_peak, 2, "{pitches:?}");
    assert!(s.strike_pose_pitch().is_none() || *pitches.last().unwrap() < 0.05);
    // An interrupt clears it at once.
    s.advance(&mut h, press, 1. / 60.);
    assert!(s.strike_pose_pitch().is_some());
    s.interrupt();
    assert_eq!(s.strike_pose_pitch(), None);
    let _ = Phase::Free;
}
