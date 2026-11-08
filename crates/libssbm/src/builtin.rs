//! Authored no-disc gameplay profile. Approximate timings, never extracted data.
//! Shares the same behaviors/executor as disc play; defaults began as synthetic fixtures.
use crate::{
    action::{ActionDef, Command, Hitbox, TimedCommand},
    motion::MotionDef,
    *,
};
use glam::Vec3;
pub fn sim(kind: u32, position: Vec3) -> Option<Sim> {
    let behavior: Box<dyn behavior::Behavior> = match kind {
        0 => Box::new(samus::Samus::fixture()),
        1 => Box::new(fox::Fox::fixture()),
        2 => Box::new(luigi::Luigi::fixture()),
        3 => Box::new(pikachu::Pikachu::fixture()),
        4 => Box::new(captain::Captain::fixture()),
        5 => Box::new(link::Link::fixture()),
        6 => Box::new(yoshi::Yoshi::fixture()),
        7 => Box::new(donkey::Donkey::fixture()),
        8 => Box::new(mario::Mario::new()),
        _ => return None,
    };
    let mut s = Sim::with_behavior(Params::fixture(), behavior, position);
    // A full action schedule is required: multi-stage behaviors advance via clip completion.
    for name in CLIPS {
        let looping = *name == "CatchWait";
        let mut commands = vec![];
        if name.starts_with("Attack")
            || name.starts_with("Special")
                && !name.contains("Hold")
                && !name.contains("End")
                && !name.contains("Start")
        {
            commands.push(TimedCommand {
                frame: 6.,
                command: Command::Hitbox(Hitbox {
                    damage: if name.contains("4") { 12. } else { 6. },
                    radius: 5.,
                    offset: Vec3::new(0., 8., 5.),
                    angle: 45,
                    growth: 70,
                    base_knockback: 25,
                    ground: true,
                    air: true,
                    ..Default::default()
                }),
            });
        }
        commands.extend([
            TimedCommand {
                frame: 6.,
                command: Command::Variable(0, 1),
            },
            TimedCommand {
                frame: 6.,
                command: Command::Variable(1, 1),
            },
            TimedCommand {
                frame: 6.,
                command: Command::ThrowFlag(1),
            },
            TimedCommand {
                frame: 12.,
                command: Command::ClearHitboxes,
            },
            TimedCommand {
                frame: 15.,
                command: Command::JabCombo(true),
            },
        ]);
        let frames = if name.contains("Loop") { 12. } else { 30. };
        s.motion.definitions.insert(
            (*name).into(),
            MotionDef {
                frames,
                looping,
                root_forward: None,
                root_up: if kind == 2 && matches!(*name, "SpecialHi" | "SpecialAirHi") {
                    Some((0..=30).map(|n| n as f32 * 1.2).collect())
                } else {
                    None
                },
            },
        );
        s.action.definitions.insert(
            (*name).into(),
            ActionDef {
                frames,
                looping,
                commands,
            },
        );
    }
    for name in [
        "Wait",
        "WalkSlow",
        "WalkMiddle",
        "WalkFast",
        "Run",
        "Fall",
        "FallF",
        "FallB",
        "SquatWait",
        "Guard",
    ] {
        s.motion.definitions.insert(
            name.into(),
            MotionDef {
                frames: 30.,
                looping: true,
                root_forward: None,
                root_up: None,
            },
        );
    }
    Some(s)
}
const CLIPS: &[&str] = &[
    "Attack100End",
    "Attack100Loop",
    "Attack100Start",
    "Attack11",
    "Attack12",
    "Attack13",
    "AttackAirB",
    "AttackAirF",
    "AttackAirHi",
    "AttackAirLw",
    "AttackAirN",
    "AttackDash",
    "AttackHi3",
    "AttackHi4",
    "AttackLw3",
    "AttackLw4",
    "AttackS3S",
    "AttackS4S",
    "CatchAttack",
    "CatchDash",
    "CatchWait",
    "ItemScopeAirEnd",
    "ItemScopeAirFire",
    "ItemScopeAirRapid",
    "ItemScopeAirStart",
    "ItemScopeEnd",
    "ItemScopeFire",
    "ItemScopeRapid",
    "ItemScopeStart",
    "SpecialAir",
    "SpecialAirHi",
    "SpecialAirHiEnd",
    "SpecialAirHiStart",
    "SpecialAirLw",
    "SpecialAirLwEnd",
    "SpecialAirLwLoop",
    "SpecialAirLwStart",
    "SpecialAirN",
    "SpecialAirN2",
    "SpecialAirNCancel",
    "SpecialAirNEnd",
    "SpecialAirNLoop",
    "SpecialAirNStart",
    "SpecialAirS",
    "SpecialAirS1Empty",
    "SpecialAirSEnd",
    "SpecialAirSHold",
    "SpecialAirSStart",
    "SpecialHi",
    "SpecialHiCatch",
    "SpecialHiEnd",
    "SpecialHiFall",
    "SpecialHiHold",
    "SpecialHiHoldAir",
    "SpecialHiLanding",
    "SpecialHiStart",
    "SpecialHiThrow",
    "SpecialLw",
    "SpecialLwEnd",
    "SpecialLwLanding",
    "SpecialLwLoop",
    "SpecialLwLoop#0",
    "SpecialLwStart",
    "SpecialN",
    "SpecialN2",
    "SpecialNCancel",
    "SpecialNCansel",
    "SpecialNEnd",
    "SpecialNHold",
    "SpecialNLoop",
    "SpecialNStart",
    "SpecialS",
    "SpecialS1Empty",
    "SpecialSEnd",
    "SpecialSHold",
    "SpecialSLoop",
    "SpecialSStart",
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_fighter_moves_attacks_and_recovers_without_files() {
        for kind in 0..8 {
            for button in [ATTACK, NEUTRAL_B, SIDE_B, UP_B, DOWN_B] {
                let mut s = sim(kind, Vec3::ZERO).unwrap();
                let mut host = FlatHost::default();
                let mut damaging = false;
                for frame in 0..900 {
                    s.advance(
                        &mut host,
                        Input {
                            aim: Vec3::Z,
                            movement: Vec3::ZERO,
                            buttons: if frame < 45 { button } else { 0 },
                            allowed: u32::MAX,
                        },
                        1. / 60.,
                    );
                    assert!(s.position.is_finite(), "kind {kind}, button {button}");
                    damaging |= s.action.samples.iter().any(|h| h.hitbox.damage > 0.);
                }
                assert!(
                    host.events.iter().any(|e| e.kind == EventKind::Started),
                    "kind {kind} button {button} never activated"
                );
                assert!(
                    s.current_move.is_none(),
                    "kind {kind} button {button} stuck in {:?}, stage {}, clip {}",
                    s.current_move,
                    s.stage,
                    s.motion.clip
                );
                if button == ATTACK {
                    assert!(
                        damaging,
                        "kind {kind} normal never created a damaging volume"
                    );
                }
            }
            let mut s = sim(kind, Vec3::ZERO).unwrap();
            let mut host = FlatHost::default();
            for _ in 0..120 {
                s.advance(
                    &mut host,
                    Input {
                        movement: Vec3::Z,
                        aim: Vec3::Z,
                        buttons: 0,
                        allowed: u32::MAX,
                    },
                    1. / 60.,
                );
            }
            assert!(s.position.z > 20., "kind {kind} did not move");
        }
    }
}
