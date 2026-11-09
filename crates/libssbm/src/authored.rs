//! Authored gameplay definitions for the no-assets profile: per-fighter move schedules written
//! in this crate, in the same `ActionDef`/`MotionDef` form the disc loader fills, so the shared
//! executor (`sim/actions.rs`) runs them unchanged. The values are approximations validated
//! against measured behavior of the asset-backed fighter (see docs/samus-reference.md), never
//! extracted data. A fighter migrates by listing its clips here; clips not listed keep the
//! generic schedule from `builtin.rs`.
use crate::{
    action::{ActionDef, Command, Hitbox, TimedCommand},
    motion::MotionDef,
    Sim,
};
use glam::Vec3;

mod samus_rest;

fn at(frame: f32, command: Command) -> TimedCommand {
    TimedCommand { frame, command }
}
fn sphere(id: u32, damage: f32, radius: f32, offset: Vec3, base_knockback: u32) -> Command {
    Command::Hitbox(Hitbox {
        id,
        damage,
        radius,
        offset,
        angle: 361,
        growth: 100,
        base_knockback,
        ground: true,
        air: true,
        ..Default::default()
    })
}
/// Root travel (sim units, forward, cumulative per 60 Hz tick) for a clip that carries the fighter
/// along: an ease-out reaching `total` at the clip's end, sharper for a larger `power`.
fn travel(frames: f32, total: f32, power: f32) -> Vec<f32> {
    (0..=frames as u32).map(|n| total * (1. - (1. - n as f32 / frames).powf(power))).collect()
}
fn install(sim: &mut Sim, name: &str, frames: f32, looping: bool, commands: Vec<TimedCommand>) {
    sim.motion.definitions.insert(
        name.into(),
        MotionDef { frames, looping, root_forward: None, root_up: None },
    );
    sim.action
        .definitions
        .insert(name.into(), ActionDef { frames, looping, commands });
}

/// Samus's two-hit jab and Charge Shot, authored.
///
/// Behavior being matched (ftSamus attack1/2 and specialn, as measured on the asset-backed sim):
/// the jab strikes on its fourth frame with three spheres along the arm, can chain until its
/// last third, and Samus has no third jab or rapid jab; the Charge Shot is a 14-frame start
/// whose end flag hands over to the hold, and the shot itself comes only from the release clip.
pub fn samus(sim: &mut Sim) {
    // No third jab or rapid jab: the chain follows the clips a fighter has.
    for gone in ["Attack13", "Attack100Start", "Attack100Loop", "Attack100End"] {
        sim.action.definitions.remove(gone);
        sim.motion.definitions.remove(gone);
    }
    install(sim, "Attack11", 18., false, vec![
        at(3., sphere(0, 3., 3.5, Vec3::new(0., 12.5, 15.), 0)),
        at(3., sphere(1, 3., 3.5, Vec3::new(0., 12.5, 10.), 0)),
        at(3., sphere(2, 3., 3.5, Vec3::new(0., 12.5, 4.5), 0)),
        at(5., Command::ClearHitboxes),
        at(6., Command::JabCombo(false)),
        at(12., Command::JabCombo(true)),
        at(18., Command::Interrupt),
    ]);
    install(sim, "Attack12", 30., false, vec![
        at(4., sphere(0, 7., 4.3, Vec3::new(0., 10., 8.), 15)),
        at(4., sphere(1, 7., 3.7, Vec3::new(0., 10., 3.), 15)),
        at(7., Command::ClearHitboxes),
        at(30., Command::Interrupt),
    ]);
    // Charge Shot. The start clip only flags its end; the release clip carries the shot (its
    // variable 1), and the hold's variable 2 is the charge voice.
    for start in ["SpecialNStart", "SpecialAirNStart"] {
        install(sim, start, 14., false, vec![at(13., Command::Variable(0, 1))]);
    }
    for fire in ["SpecialN", "SpecialAirN"] {
        install(sim, fire, 30., false, vec![at(0., Command::Variable(1, 1))]);
    }
    install(sim, "SpecialNHold", 10., true, vec![at(0., Command::Variable(2, 1))]);
    samus_rest::normals_and_aerials(sim);
    samus_rest::specials(sim);
    samus_rest::grapple(sim);
    samus_rest::motion_lengths(sim);
}

/// Samus's articles for the no-assets profile: the bomb (kind 0), Charge Shot (kind 1, states 1 to
/// 8 for the charge level) and the homing (state 0) and super (state 1) missiles (kind 2).
/// Rounded approximations of how the asset-backed articles fly, never extracted data. `charge`
/// is the shot's charge ratio 0..1.
pub fn samus_article(kind: u32, state: u32, charge: f32) -> Option<crate::projectile::Params> {
    use crate::projectile::Params;
    let charge = charge.clamp(0., 1.);
    let base = Params {
        kind,
        state,
        life: 180.,
        speed: 0.,
        acceleration: 0.,
        max_speed: 0.,
        delay: 0.,
        homing_frames: 0.,
        decay: 1.,
        min_speed: 0.,
        turn_accel: 0.,
        max_turn: 0.,
        min_angle: 0.,
        gravity: 0.,
        max_fall: 2.,
        initial_y: 0.,
        damage: 4.,
        radius: 2.,
        explosion_radius: 2.,
        explosion_frames: 0.,
        max_bounces: 0,
        bounce_factor: 0.5,
        straight_distance: 0.,
    };
    Some(match (kind, state) {
        // Bomb: a short fuse, a small hop, then it falls and bursts wide.
        (0, _) => Params {
            life: 72.,
            gravity: 0.03,
            max_fall: 1.2,
            initial_y: 0.5,
            explosion_radius: 6.,
            explosion_frames: 28.,
            ..base
        },
        // Charge Shot: flat flight, faster and bigger with charge.
        (1, _) => Params {
            life: 70.,
            speed: 1.5 + 1.5 * charge,
            damage: 3. + 22. * ((state.clamp(1, 8) - 1) as f32 / 7.),
            radius: 1.2 + 3.5 * ((state.clamp(1, 8) - 1) as f32 / 7.),
            explosion_radius: 1.2 + 3.5 * ((state.clamp(1, 8) - 1) as f32 / 7.),
            gravity: 0.036,
            max_fall: 1.65,
            ..base
        },
        // Homing missile: slow, steers toward a target for a while, then flies on.
        (2, 0) => Params {
            life: 110.,
            homing_frames: 60.,
            speed: 1.3,
            decay: 0.95,
            min_speed: 0.5,
            turn_accel: 1.1_f32.to_radians(),
            max_turn: 50_f32.to_radians(),
            min_angle: 1_f32.to_radians(),
            gravity: 0.05,
            damage: 5.,
            radius: 2.,
            explosion_radius: 2.,
            explosion_frames: 30.,
            ..base
        },
        // Super missile: waits, then accelerates fast and straight.
        (2, _) => Params {
            life: 75.,
            delay: 10.,
            speed: 0.6,
            acceleration: 0.06,
            max_speed: 2.2,
            gravity: 0.05,
            damage: 12.,
            radius: 2.3,
            explosion_radius: 2.3,
            explosion_frames: 40.,
            ..base
        },
        _ => return None,
    })
}

/// Samus's arrival and fall-off rebirth for the no-assets profile: the shared trophy-to-fighter
/// entry (platform growth and release, a white flash) and the KO, figurine, transform and protected
/// timings. Rounded approximations of the asset-backed sequence, never extracted data.
pub fn samus_return() -> (crate::rebirth::Params, crate::entry::Params) {
    (
        crate::rebirth::Params {
            ko_frames: 60,
            arrival_frames: 60,
            wait_frames: 240,
            invulnerable_frames: 120,
            ko_scale: 0.8,
            flash: vec![
                [1., 1., 1., 0.5],
                [1., 1., 1., 0.5],
                [1., 0.96, 0.94, 0.44],
                [1., 0.93, 0.88, 0.37],
                [1., 0.89, 0.81, 0.31],
                [1., 0.86, 0.75, 0.25],
                [1., 0.82, 0.69, 0.18],
                [0., 0., 0., 0.],
                [0., 0., 0., 0.],
                [0.33, 0.33, 0.33, 0.21],
                [0.67, 0.67, 0.67, 0.42],
            ],
        },
        crate::entry::Params {
            grow: 30,
            release: 30,
            flat: 0.01,
            platform: 0.9,
            color: [1., 1., 1., 0.47],
            color_frames: 100,
        },
    )
}
