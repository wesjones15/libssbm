//! Samus's remaining move schedules for the no-assets profile, authored from what the
//! asset-backed fighter measurably does (clip lengths, hit windows, volume counts, the script
//! flags the shared executor reads), with rounded approximate numbers. Never extracted data.
//! Companion to `authored::samus` (jab chain and Charge Shot); see docs/samus-reference.md.
use super::{at, install, travel};
use crate::{
    action::{Command, Hitbox, TimedCommand},
    Sim,
};
use glam::Vec3;

/// A hit volume: id, damage, radius, height and forward reach from the body, launch angle,
/// knockback growth, base knockback, fixed knockback, element (1 fire, 2 electric, 8 grapple).
#[derive(Clone, Copy)]
struct V(u32, f32, f32, f32, f32, u32, u32, u32, u32, u32);

fn vol(v: V) -> Command {
    Command::Hitbox(Hitbox {
        id: v.0,
        damage: v.1,
        radius: v.2,
        offset: Vec3::new(0., v.3, v.4),
        angle: v.5,
        growth: v.6,
        base_knockback: v.7,
        fixed_knockback: v.8,
        element: v.9,
        ground: true,
        air: true,
        ..Default::default()
    })
}

/// Hit windows: at `open` the volumes appear, at `close` they are cleared.
fn windows(list: &[(f32, f32, &[V])]) -> Vec<TimedCommand> {
    let mut out = Vec::new();
    for (open, close, volumes) in list {
        for v in *volumes {
            out.push(at(*open, vol(*v)));
        }
        out.push(at(*close, Command::ClearHitboxes));
    }
    out
}

fn with(mut base: Vec<TimedCommand>, extra: Vec<TimedCommand>) -> Vec<TimedCommand> {
    base.extend(extra);
    base.sort_by(|a, b| a.frame.total_cmp(&b.frame));
    base
}

/// Throw hitbox command (opcode 34, slot 0) with `damage` and launch `angle`: the packed form
/// the executor reads (ftAction set_throw_hitbox); growth and knockback are authored round numbers.
fn throw_hit(damage: f32, angle: u32) -> TimedCommand {
    at(0., Command::Raw {
        opcode: 34,
        words: vec![damage as u32 & 0x7f_ffff, angle << 23 | 100 << 14, 50 << 23],
    })
}

pub fn normals_and_aerials(sim: &mut Sim) {
    let hi = 10.;
    install(sim, "AttackDash", 38., false, with(
        // The weaker late hit replaces the early one in place (one hit generation).
        vec![
            at(7., vol(V(0, 13., 5., hi, 6., 60, 105, 22, 0, 0))),
            at(10., vol(V(0, 9., 4.3, hi, 6., 60, 105, 22, 0, 0))),
            at(17., Command::ClearHitboxes),
        ],
        vec![at(38., Command::Interrupt)],
    ));
    // The dash attack carries Samus forward in a surge that dies away.
    if let Some(motion) = sim.motion.definitions.get_mut("AttackDash") {
        motion.root_forward = Some(travel(38., 47., 1.9));
    }
    for clip in ["AttackS3S", "AttackS3Hi", "AttackS3Lw", "AttackS3HiS", "AttackS3LwS"] {
        let damage = match clip {
            "AttackS3Hi" | "AttackS3HiS" => 11.,
            "AttackS3Lw" | "AttackS3LwS" => 9.,
            _ => 10.,
        };
        install(sim, clip, 32., false, with(
            windows(&[(6., 9., &[
                V(0, damage, 3.9, hi, 7., 361, 100, 10, 0, 0),
                V(1, damage, 3.3, hi, 0., 361, 100, 10, 0, 0),
                V(2, damage, 3.1, hi, 4.5, 361, 100, 10, 0, 0),
                V(3, damage, 2.3, hi, 0., 361, 100, 10, 0, 0),
            ])]),
            vec![at(30., Command::Interrupt)],
        ));
    }
    install(sim, "AttackHi3", 40., false, with(
        windows(&[(14., 18., &[
            V(0, 13., 5., hi, 0., 270, 100, 40, 0, 0),
            V(1, 13., 5., hi, 6., 270, 100, 40, 0, 0),
            V(2, 12., 5., hi, 0., 361, 100, 40, 0, 0),
            V(3, 12., 5., hi, 6., 361, 100, 40, 0, 0),
        ])]),
        vec![at(35., Command::Interrupt)],
    ));
    install(sim, "AttackLw3", 40., false, windows(&[(6., 9., &[
        V(0, 14., 3., hi, 0., 100, 60, 80, 0, 1),
        V(1, 14., 3.5, hi, 0., 100, 60, 80, 0, 1),
        V(2, 14., 6., hi, 18., 100, 60, 80, 0, 1),
    ])]));

    // Smashes: the charge command, then the strike windows.
    let charge = |f: f32| at(f, Command::SmashCharge { max_frames: 60, damage_per_frame: 1.4 });
    for clip in ["AttackS4S", "AttackS4Hi", "AttackS4Lw", "AttackS4HiS", "AttackS4LwS"] {
        let damage = match clip {
            "AttackS4Hi" | "AttackS4HiS" => 15.,
            "AttackS4Lw" | "AttackS4LwS" => 13.,
            _ => 14.,
        };
        install(sim, clip, 44., false, with(
            windows(&[(10., 14., &[
                V(0, damage, 4.3, hi, 6., 361, 104, 30, 0, 0),
                V(1, damage, 2.5, hi, 0., 361, 104, 30, 0, 0),
                V(2, damage, 3.5, hi, -2., 361, 104, 30, 0, 0),
            ])]),
            vec![charge(4.), at(44., Command::Interrupt)],
        ));
    }
    let arm = |dmg: f32, ang: u32, bkb: u32| [
        V(0, dmg, 3.2, hi, -2.5, ang, 50, bkb, 0, 1),
        V(1, dmg, 6.7, hi, 7., ang, 50, bkb, 0, 1),
    ];
    install(sim, "AttackHi4", 60., false, with(
        windows(&[
            (12., 15., &arm(4., 130, 50)),
            (16., 19., &arm(4., 120, 35)),
            (20., 23., &arm(4., 90, 25)),
            (24., 27., &arm(5., 85, 30)),
            (28., 30., &[V(0, 6., 3.3, hi, -2.5, 80, 120, 50, 0, 1), V(1, 6., 7., hi, 7., 80, 120, 50, 0, 1)]),
        ]),
        vec![charge(8.), at(58., Command::Interrupt)],
    ));
    install(sim, "AttackLw4", 49., false, with(
        windows(&[
            (6., 9., &[V(0, 16., 3.9, hi, -2., 120, 50, 110, 0, 0), V(1, 16., 4.7, hi, 8., 120, 50, 110, 0, 0)]),
            (14., 17., &[V(0, 15., 3.9, hi, -2., 120, 40, 90, 0, 0), V(1, 15., 4.7, hi, 8., 120, 40, 90, 0, 0)]),
        ]),
        vec![charge(3.), at(45., Command::Interrupt)],
    ));

    // Aerials. Variable 0 is the script's "attack active" flag the executor reads.
    let flag = |on: f32, off: f32, end: f32| vec![
        at(on, Command::Variable(0, 1)),
        at(off, Command::Variable(0, 0)),
        at(end, Command::Interrupt),
    ];
    install(sim, "AttackAirN", 50., false, with(
        vec![
            at(5., vol(V(0, 14., 4., hi, -1., 361, 100, 10, 0, 0))),
            at(5., vol(V(1, 14., 4.7, hi, 2., 361, 100, 10, 0, 0))),
            at(9., vol(V(0, 10., 4., hi, -1., 361, 100, 10, 0, 0))),
            at(9., vol(V(1, 10., 4.7, hi, 2., 361, 100, 10, 0, 0))),
            at(30., Command::ClearHitboxes),
        ],
        flag(5., 35., 40.),
    ));
    install(sim, "AttackAirB", 40., false, with(
        windows(&[(9., 13., &[
            V(0, 10., 3.9, hi, -1., 361, 100, 30, 0, 0),
            V(1, 10., 3.1, hi, 3.7, 361, 100, 30, 0, 0),
            V(2, 14., 3.9, hi, 0., 361, 100, 42, 0, 0),
        ])]),
        flag(9., 31., 37.),
    ));
    let fair = |ang: u32| [V(0, 5., 3.8, hi, -3.7, ang, 100, 20, 0, 1), V(1, 5., 5.8, hi, 6.7, ang, 100, 20, 0, 1)];
    install(sim, "AttackAirF", 56., false, with(
        windows(&[
            (5., 7., &fair(85)),
            (12., 14., &fair(85)),
            (19., 21., &fair(85)),
            (26., 28., &fair(85)),
            (31., 33., &fair(361)),
        ]),
        flag(0., 47., 50.),
    ));
    let up = |dmg: f32, ang: u32, fkb: u32| [
        V(0, dmg, 5.3, 11.5, -1.7, ang, 100, 0, fkb, 0),
        V(1, dmg, 6.2, 11., 3.3, ang, 100, 0, fkb, 0),
        V(2, dmg, 4.7, hi, 0., if ang == 90 { 361 } else { ang }, 130, 0, 0, 0),
    ];
    install(sim, "AttackAirHi", 40., false, with(
        windows(&[
            (5., 7., &up(3., 90, 30)),
            (8., 10., &up(1., 80, 30)),
            (11., 13., &up(1., 80, 30)),
            (14., 16., &up(1., 80, 30)),
            (17., 19., &up(1., 80, 30)),
            (20., 22., &[
                V(0, 4., 4.8, 11.5, -1.7, 361, 120, 40, 0, 0),
                V(1, 4., 5.9, 11., 2., 361, 120, 40, 0, 0),
                V(2, 4., 4.3, hi, 0., 361, 120, 40, 0, 0),
            ]),
        ]),
        flag(5., 34., 39.),
    ));
    install(sim, "AttackAirLw", 55., false, with(
        windows(&[(18., 23., &[
            V(0, 16., 3.7, hi, -2., 270, 100, 30, 0, 0),
            V(1, 16., 4.2, hi, 4.7, 270, 100, 30, 0, 0),
        ])]),
        flag(3., 34., 49.),
    ));
}

/// Missiles, Screw Attack and the bomb. The missile clips carry only the throw flag that
/// launches them; the Screw Attack is a multi-hit that ends in a burst; the bomb clip swaps the
/// morph-ball model in and out (visibility) and flags the drop.
pub fn specials(sim: &mut Sim) {
    for (clip, frames, flag) in [
        ("SpecialS", 60., 22.),
        ("SpecialAirS", 60., 24.),
        ("Special", 50., 22.),
        ("SpecialAir", 50., 15.),
    ] {
        install(sim, clip, frames, false, vec![at(flag, Command::ThrowFlag(0))]);
    }
    // Screw Attack: four volumes (two each side) pulsing every other frame, ending in one burst.
    // Ground: low pair and high pair; air: the pair on the head angles differently.
    let quad = |dmg: f32, r: f32, low: (u32, u32), high: (u32, u32), low_y: f32| [
        V(0, dmg, r, low_y, 6., low.0, 100, low.1, 0, 0),
        V(1, dmg, r, low_y, -6., low.0, 100, low.1, 0, 0),
        V(2, dmg, r, 9.4, 6., high.0, 100, high.1, 0, 0),
        V(3, dmg, r, 9.4, -6., high.0, 100, high.1, 0, 0),
    ];
    let burst = [V(0, 1., 7.8, 5., 1.6, 70, 100, 0, 80, 2)];
    let ground_pulses = |out: &mut Vec<TimedCommand>| {
        // The first two windows hand over without a clear (one hit generation); later ones clear.
        for v in quad(2., 3.1, (100, 130), (100, 100), -1.2) {
            out.push(at(4., vol(v)));
        }
        for v in quad(1., 2.7, (105, 110), (100, 70), -1.2) {
            out.push(at(6., vol(v)));
        }
        out.push(at(8., Command::ClearHitboxes));
        for frame in [8., 10.] {
            out.extend(windows(&[(frame, frame + 2., &quad(1., 2.7, (105, 110), (100, 70), -1.2))]));
        }
        for frame in (12..=26).step_by(2) {
            out.extend(windows(&[(frame as f32, frame as f32 + 2., &quad(1., 2.3, (110, 70), (200, 70), -0.8))]));
        }
    };
    let mut ground = vec![at(4., Command::Variable(0, 1)), at(12., Command::Variable(1, 1))];
    ground_pulses(&mut ground);
    ground.extend(windows(&[(28., 30., &burst)]));
    ground.sort_by(|a, b| a.frame.total_cmp(&b.frame));
    install(sim, "SpecialHi", 50., false, ground);
    let mut air = vec![at(4., Command::Variable(0, 1)), at(10., Command::Variable(1, 1))];
    for frame in (4..=24).step_by(2) {
        let (r, bkb) = if frame < 10 { (2.7, 100) } else { (2.3, 80) };
        // On the air clip the head pair is the high angle (140) and the low pair 110.
        air.extend(windows(&[(frame as f32, frame as f32 + 2., &[
            V(0, 1., 2.3, 9.4, 6., 140, 0, 25, 0, 0),
            V(1, 1., 2.3, 9.4, -6., 140, 0, 25, 0, 0),
            V(2, 1., r, -0.8, 6., 110, 0, bkb, 0, 0),
            V(3, 1., r, -0.8, -6., 110, 0, bkb, 0, 0),
        ])]));
    }
    air.extend(windows(&[(26., 28., &burst)]));
    air.sort_by(|a, b| a.frame.total_cmp(&b.frame));
    install(sim, "SpecialAirHi", 48., false, air);

    // Down special: the ball model comes out at 3, the bomb is flagged at 10, the unfold from 43.
    let bomb = || vec![
        at(3., Command::Visibility { group: 0, choice: 1 }),
        at(10., Command::ThrowFlag(0)),
        at(10., Command::Visibility { group: 0, choice: 2 }),
        at(10., Command::Variable(0, 1)),
        at(43., Command::Variable(0, 0)),
        at(43., Command::Visibility { group: 0, choice: 1 }),
        at(49., Command::Visibility { group: 0, choice: 0 }),
    ];
    let mut ground = bomb();
    ground.push(at(3., Command::Variable(1, 1)));
    ground.push(at(46., Command::Variable(2, 1)));
    ground.sort_by(|a, b| a.frame.total_cmp(&b.frame));
    install(sim, "SpecialLw", 54., false, ground);
    install(sim, "SpecialAirLw", 54., false, bomb());
    install(sim, "SpecialNCancel", 8., false, vec![at(0., Command::Variable(0, 0))]);
}

/// The grapple, its pummel and throws. The catch beam's volumes pulse until the clip's throw
/// flag; each throw releases at its own flag.
pub fn grapple(sim: &mut Sim) {
    let pulses = |from: u32, to: u32| {
        let mut out = Vec::new();
        let mut f = from;
        while f <= to {
            out.push(at(f as f32, vol(V(0, 0., 3.5, 9., 4.7, 361, 100, 0, 0, 8))));
            f += 3;
        }
        out
    };
    let mut catch = windows(&[(17., 20., &[V(0, 0., 4.7, 9., 4.7, 361, 100, 0, 0, 8), V(1, 0., 4.7, 9., 0., 361, 100, 0, 0, 8)])]);
    catch.extend(pulses(20, 35));
    catch.push(at(41., Command::ClearHitboxes));
    catch.push(at(41., Command::ThrowFlag(0)));
    install(sim, "Catch", 94., false, catch.clone());
    let mut dash = windows(&[(17., 20., &[V(0, 0., 4.7, 9., 4.7, 361, 100, 0, 0, 8), V(1, 0., 4.7, 9., 0., 361, 100, 0, 0, 8)])]);
    dash.extend(pulses(20, 29));
    dash.push(at(33., Command::ClearHitboxes));
    dash.push(at(33., Command::ThrowFlag(0)));
    install(sim, "CatchDash", 74., false, dash);
    install(sim, "AirCatch", 60., false, windows(&[(9., 28., &[
        V(0, 4., 2., 0., 0., 361, 40, 15, 0, 2),
        V(1, 4., 2., 0., 0., 361, 40, 15, 0, 2),
    ])]));
    install(sim, "AirCatchHit", 25., false, vec![]);
    install(sim, "CatchWait", 30., true, vec![]);
    install(sim, "CatchAttack", 24., false, windows(&[(9., 10., &[V(0, 3., 5.9, 10., 4., 80, 100, 0, 30, 0)])]));
    // Each throw sets its throw hitbox at frame 0 (damage and launch angle; the executor emits
    // them as the Throw event at the release flag).
    install(sim, "ThrowF", 42., false, vec![throw_hit(9., 42), at(15., Command::ThrowFlag(0))]);
    install(sim, "ThrowB", 42., false, vec![throw_hit(8., 140), at(11., Command::ThrowFlag(0))]);
    install(sim, "ThrowLw", 42., false, vec![throw_hit(6., 80), at(21., Command::ThrowFlag(0))]);
    let mut up = Vec::new();
    for f in (13..=23).step_by(2) {
        up.push(at(f as f32, vol(V(0, 0., 2.3, 10., 0., 361, 100, 0, 0, 0))));
        up.push(at((f + 1) as f32, Command::ClearHitboxes));
    }
    up.push(throw_hit(7., 90));
    up.push(at(31., Command::ThrowFlag(0)));
    install(sim, "ThrowHi", 42., false, up);
}

/// Clip lengths (frames) and looping for the clips the shared executor waits on or blends:
/// gait, jump, fall, landing, guard and dodge. Lengths only; gait and pose come from the host.
pub fn motion_lengths(sim: &mut Sim) {
    // The air dodge's intangible window (hurt-state commands) and its end flag.
    install(sim, "EscapeAir", 50., false, vec![
        at(4., Command::Raw { opcode: 26, words: vec![0x6800_0002] }),
        at(30., Command::Raw { opcode: 26, words: vec![0x6800_0000] }),
        at(30., Command::Variable(0, 1)),
    ]);
    use crate::motion::MotionDef;
    const CLIPS: &[(&str, f32, bool)] = &[
        ("Wait", 81., false), ("WalkSlow", 90., true), ("WalkMiddle", 50., true), ("WalkFast", 40., true),
        ("Run", 25., true), ("RunBrake", 21., false), ("Dash", 23., false), ("Turn", 12., false),
        ("TurnRun", 26., false), ("Squat", 8., false), ("SquatRv", 8., false), ("SquatWait", 60., false),
        ("Fall", 8., true), ("FallF", 8., true), ("FallB", 8., true), ("FallAerial", 8., true),
        ("FallAerialF", 8., true), ("FallAerialB", 8., true), ("FallSpecial", 50., true),
        ("JumpF", 40., false), ("JumpB", 60., false), ("JumpAerialF", 48., false), ("JumpAerialB", 48., false),
        ("Landing", 30., false), ("LandingAirN", 30., false), ("LandingAirF", 30., false),
        ("LandingAirB", 30., false), ("LandingAirHi", 30., false), ("LandingAirLw", 30., false),
        ("GuardOn", 8., false), ("Guard", 370., false), ("GuardOff", 16., false), ("GuardDamage", 20., false),
        ("DamageHi1", 12., false), ("DamageHi2", 24., false), ("DamageHi3", 30., false),
        ("DamageLw1", 12., false), ("DamageLw2", 24., false), ("DamageLw3", 30., false),
        ("DamageN1", 12., false), ("DamageN2", 24., false), ("DamageN3", 30., false),
        ("DamageAir1", 12., false), ("DamageAir2", 24., false), ("DamageAir3", 30., false),
        ("DamageFall", 29., true), ("DamageFlyHi", 29., false), ("DamageFlyLw", 29., false),
        ("DamageFlyN", 29., false), ("DamageFlyRoll", 16., true), ("DamageFlyTop", 60., false),
        ("DownAttackD", 50., false), ("DownAttackU", 50., false), ("DownBackD", 36., false),
        ("DownBackU", 36., false), ("DownBoundD", 26., false), ("DownBoundU", 26., false),
        ("DownDamageD", 14., false), ("DownDamageU", 14., false), ("DownFowardD", 36., false),
        ("DownFowardU", 36., false), ("DownSpotD", 20., false), ("DownStandD", 30., false),
        ("DownStandU", 30., false), ("DownWaitD", 100., true), ("DownWaitU", 100., true),
        ("EscapeN", 23., false), ("EscapeF", 45., false), ("EscapeB", 45., false),
    ];
    for (name, frames, looping) in CLIPS {
        sim.motion.definitions.insert(
            (*name).into(),
            MotionDef { frames: *frames, looping: *looping, root_forward: None, root_up: None },
        );
    }
}
