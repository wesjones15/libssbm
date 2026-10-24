use glam::Vec3;
use libssbm::{
    action::{ActionDef, Command, TimedCommand},
    motion::MotionDef,
    *,
};

fn fighter() -> Sim {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    for (name, frames, looping) in [
        ("GuardOn", 4., false),
        ("Guard", 30., true),
        ("GuardOff", 5., false),
        ("EscapeAir", 30., false),
        ("Furafura", 60., true),
    ] {
        s.motion.definitions.insert(
            name.into(),
            MotionDef {
                frames,
                looping,
                root_forward: None,
                root_up: None,
            },
        );
        s.action.definitions.insert(
            name.into(),
            ActionDef {
                frames,
                looping,
                commands: vec![],
            },
        );
    }
    s
}
fn tick(s: &mut Sim, h: &mut FlatHost, buttons: u32) {
    s.advance(
        h,
        Input {
            buttons,
            ..Input::idle()
        },
        1. / 60.,
    );
}

#[test]
fn explicit_skills_ignore_camera_and_stick_and_respect_five_stock_masks() {
    for fps in [30, 60, 144] {
        for (button, want, slot) in [
            (NEUTRAL_B, Move::SpecialN, 0),
            (SIDE_B, Move::SpecialS2, 2),
            (UP_B, Move::SpecialHi, 3),
            (DOWN_B, Move::SpecialLw, 1),
        ] {
            for ready in [false, true] {
                let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
                let mut h = FlatHost::default();
                tick(&mut s, &mut h, 0);
                let p = Input {
                    buttons: button,
                    movement: Vec3::Z,
                    aim: Vec3::new(0., -0.99, 0.1),
                    allowed: if ready { 1 << slot } else { 31 & !(1 << slot) },
                };
                for _ in 0..fps / 30 {
                    s.advance(&mut h, p, 1. / fps as f64);
                }
                assert_eq!(
                    s.current_move,
                    if ready { Some(want) } else { None },
                    "fps={fps} button={button}"
                );
            }
        }
    }
}

#[test]
fn samus_morph_roll_alternate_rolls_and_releases() {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    let mut h = FlatHost::default();
    tick(&mut s, &mut h, 0);
    s.host_flags = libssbm::samus::HOST_MORPH_ROLL;
    let roll = Input {
        buttons: DOWN_B,
        allowed: 2,
        ..Input::idle()
    };
    for _ in 0..8 {
        s.advance(&mut h, roll, 1. / 60.);
    }
    assert_eq!(s.current_move, Some(Move::SpecialLw), "morph roll did not start");
    assert!(
        s.velocity.x.abs() > 0.1 || s.velocity.z.abs() > 0.1,
        "morph roll is not moving"
    );
    assert!(!s.action.active.is_empty(), "morph roll has no hitbox");
    let hop = Input {
        buttons: DOWN_B | JUMP,
        allowed: 2,
        ..Input::idle()
    };
    s.advance(&mut h, hop, 1. / 60.);
    assert!(s.velocity.y > 0., "morph ball cannot jump");
    assert_eq!(s.current_move, Some(Move::SpecialLw), "jumping ended the ball");
    let idle = Input { allowed: 0, ..Input::idle() };
    for _ in 0..20 {
        s.advance(&mut h, idle, 1. / 60.);
    }
    assert!(s.current_move.is_none(), "morph roll did not end on release");
}

#[test]
fn host_jump_height_matches_short_full_and_aerial_apices() {
    for fps in [30, 60, 144] {
        for (aerial, short) in [(false, false), (false, true), (true, false)] {
            let mut s = fighter();
            let mut h = FlatHost::default();
            s.host_jump_height = Some(40.);
            tick(&mut s, &mut h, 0);
            if aerial {
                s.teleport(Vec3::Y * 100.);
            }
            let base = s.position.y;
            let mut max = base;
            // Collect every native collision point so a low render rate cannot hide the apex.
            struct Peak<'a> {
                host: &'a mut FlatHost,
                peak: f32,
            }
            impl Host for Peak<'_> {
                fn collide(&mut self, a: Vec3, b: Vec3) -> Contact {
                    self.peak = self.peak.max(b.y);
                    self.host.collide(a, b)
                }
                fn event(&mut self, e: Event) {
                    self.host.event(e);
                }
            }
            let mut peak = Peak {
                host: &mut h,
                peak: max,
            };
            for frame in 0..fps * 2 {
                let buttons = if frame == 0 || (!short && frame < fps / 2) {
                    JUMP
                } else {
                    0
                };
                s.advance(
                    &mut peak,
                    Input {
                        buttons,
                        ..Input::idle()
                    },
                    1. / fps as f64,
                );
                max = max.max(peak.peak);
            }
            assert!(
                (max - base - 40.).abs() < 0.002,
                "fps={fps} aerial={aerial} short={short}: {}",
                max - base
            );
        }
    }
}

#[test]
fn host_jump_height_matches_the_up_special_apex() {
    for fps in [30, 60, 144] {
        let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
        let mut h = FlatHost::default();
        s.host_jump_height = Some(40.);
        s.advance(&mut h, Input::idle(), 1. / 60.);
        let base = s.position.y;
        let mut max = base;
        for frame in 0..fps * 2 {
            let buttons = if frame < fps / 30 { UP_B } else { 0 };
            s.advance(
                &mut h,
                Input {
                    buttons,
                    ..Input::idle()
                },
                1. / fps as f64,
            );
            max = max.max(s.position.y);
        }
        assert!(
            (max - base - 40.).abs() < 0.002,
            "fps={fps}: {}",
            max - base
        );
    }
}

#[test]
fn shield_uses_native_meter_break_release_and_jump_cancel() {
    let mut s = fighter();
    let mut h = FlatHost::default();
    tick(&mut s, &mut h, 0);
    tick(&mut s, &mut h, DEFEND);
    assert_eq!(s.motion.clip, "GuardOn");
    assert_ne!(s.defense_flags() & 1, 0);
    let before = s.defense.health;
    assert!(s.shield_hit(10.));
    assert!(s.defense.health <= before - 10.);
    tick(&mut s, &mut h, 0);
    assert_eq!(s.motion.clip, "GuardOff");
    assert!(!s.shield_hit(10.));
    for _ in 0..10 {
        tick(&mut s, &mut h, 0);
    }
    tick(&mut s, &mut h, DEFEND);
    assert!(s.shield_hit(1000.));
    assert_ne!(s.defense_flags() & 4, 0);
    for _ in 0..s.defense.params.broken_frames + 2 {
        tick(&mut s, &mut h, 0);
    }
    assert_eq!(s.defense_flags(), 0);
    assert!(s.defense.health >= s.defense.params.restore);
    tick(&mut s, &mut h, DEFEND);
    tick(&mut s, &mut h, DEFEND | JUMP);
    for _ in 0..10 {
        tick(&mut s, &mut h, JUMP);
    }
    assert!(s.velocity.y > 0. && s.defense.mode.is_none());
}

#[test]
fn air_feint_invulnerability_follows_source_commands_and_ends_helpless() {
    for fps in [30, 60, 144] {
        let mut s = fighter();
        let mut h = FlatHost::default();
        s.teleport(Vec3::Y * 1000.);
        s.action.definitions.get_mut("EscapeAir").unwrap().commands = vec![
            TimedCommand {
                frame: 3.,
                command: Command::Raw {
                    opcode: 26,
                    words: vec![1],
                },
            },
            TimedCommand {
                frame: 12.,
                command: Command::Raw {
                    opcode: 26,
                    words: vec![0],
                },
            },
        ];
        let mut protected = false;
        let mut vulnerable_after = false;
        for _ in 0..fps {
            s.advance(
                &mut h,
                Input {
                    buttons: DEFEND,
                    movement: Vec3::X,
                    ..Input::idle()
                },
                1. / fps as f64,
            );
            let flags = s.defense_flags();
            protected |= flags & 2 != 0;
            if protected && flags & 2 == 0 {
                vulnerable_after = true;
            }
        }
        assert!(protected && vulnerable_after, "fps={fps}");
        assert_eq!(s.phase, Phase::Helpless);
        assert!(s.position.x > 0.);
        tick(&mut s, &mut h, 0);
        tick(&mut s, &mut h, DEFEND);
        assert_eq!(s.defense_flags(), 0, "second air dodge");
    }
}
