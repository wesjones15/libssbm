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
}
