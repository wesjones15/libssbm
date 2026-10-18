use glam::Vec3;
use libssbm::{
    ATTACK, EventKind, FlatHost, Input, Move, Params, Sim, motion::MotionDef, reaction::Stage,
};
#[test]
fn airborne_damage_falls_bounces_and_stands_before_releasing_control() {
    let mut s = Sim::new(Params::fixture(), Vec3::Y * 5.);
    for (clip, frames) in [
        ("DamageFlyN", 3.),
        ("DamageFall", 60.),
        ("DownBoundU", 5.),
        ("DownStandU", 5.),
        ("Wait", 60.),
    ] {
        s.motion.definitions.insert(
            clip.into(),
            MotionDef {
                root_forward: None,
                frames,
                looping: clip == "Wait" || clip == "DamageFall",
            },
        );
    }
    s.hurt();
    let mut host = FlatHost::default();
    let mut seen = Vec::new();
    for _ in 0..120 {
        s.advance(&mut host, Input::idle(), 1. / 60.);
        if !seen.contains(&s.motion.clip) {
            seen.push(s.motion.clip);
        }
    }
    assert!(
        seen.contains(&"DamageFlyN")
            && seen.contains(&"DamageFall")
            && seen.contains(&"DownBoundU")
            && seen.contains(&"DownStandU")
    );
    assert_eq!(s.reaction, None);
    assert!(s.grounded);
    assert_eq!(s.motion.clip, "Wait");
    s.hurt();
    assert_eq!(s.reaction, Some(Stage::GroundHit));
    s.teleport(Vec3::ZERO);
    assert_eq!(s.reaction, None);
}

#[test]
fn contextual_attack_tapped_in_hitstun_executes_once_when_recovered() {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    for (clip, frames) in [("DamageN2", 4.), ("Wait", 60.), ("Attack11", 20.)] {
        s.motion.definitions.insert(
            clip.into(),
            MotionDef {
                root_forward: None,
                frames,
                looping: clip == "Wait",
            },
        );
    }
    let mut host = FlatHost::default();
    s.advance(&mut host, Input::idle(), 1. / 60.);
    s.hurt();
    s.advance(
        &mut host,
        Input {
            buttons: ATTACK,
            ..Input::idle()
        },
        1. / 60.,
    );
    for _ in 0..8 {
        s.advance(&mut host, Input::idle(), 1. / 60.);
    }
    assert_eq!(s.current_move, Some(Move::Jab));
    assert_eq!(
        host.events
            .iter()
            .filter(|e| e.kind == EventKind::Started && e.move_id == Move::Jab as u32)
            .count(),
        1
    );
}
