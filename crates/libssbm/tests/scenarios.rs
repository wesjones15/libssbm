use glam::Vec3;
use libssbm::*;
fn scene() -> (Sim, FlatHost, Input) {
    let mut s = Sim::new(Params::fixture(), Vec3::ZERO);
    let mut h = FlatHost::default();
    let p = Input::idle();
    s.advance(&mut h, p, 1. / 60.);
    h.events.clear();
    (s, h, p)
}
fn frames(s: &mut Sim, h: &mut FlatHost, p: Input, n: u32) {
    for _ in 0..n {
        s.advance(h, p, 1. / 60.);
    }
}
#[test]
fn fixed_step_is_independent_of_render_rate() {
    let (mut a, mut ah, mut p) = scene();
    let (mut b, mut bh, _) = scene();
    p.movement = Vec3::X;
    for _ in 0..120 {
        a.advance(&mut ah, p, 1. / 120.);
    }
    for _ in 0..30 {
        b.advance(&mut bh, p, 1. / 30.);
    }
    assert_eq!(a.frame, b.frame);
    assert!((a.position - b.position).length() < 1e-5);
}
#[test]
fn jump_press_between_ticks_is_not_lost() {
    let (mut s, mut h, mut p) = scene();
    p.buttons = JUMP;
    assert_eq!(s.advance(&mut h, p, 0.001), 0);
    p.buttons = 0;
    s.advance(&mut h, p, 0.016);
    assert_eq!(s.phase, Phase::JumpSquat);
}
#[test]
fn short_hop_is_lower_and_air_jump_is_limited_then_resets() {
    let (mut a, mut ah, mut p) = scene();
    let (mut b, mut bh, _) = scene();
    p.buttons = JUMP;
    frames(&mut a, &mut ah, p, 4);
    frames(&mut b, &mut bh, p, 1);
    p.buttons = 0;
    frames(&mut b, &mut bh, p, 3);
    assert!(a.velocity.y > b.velocity.y);
    frames(&mut a, &mut ah, p, 1);
    p.buttons = JUMP;
    frames(&mut a, &mut ah, p, 1);
    assert_eq!(a.jumps_used, 2);
    p.buttons = 0;
    frames(&mut a, &mut ah, p, 1);
    p.buttons = JUMP;
    frames(&mut a, &mut ah, p, 1);
    assert_eq!(
        ah.events
            .iter()
            .filter(|e| e.kind == EventKind::Jump)
            .count(),
        2
    );
    p.buttons = 0;
    frames(&mut a, &mut ah, p, 150);
    assert_eq!(a.jumps_used, 0);
}
#[test]
fn recent_direction_charges_smash_but_old_direction_is_tilt() {
    let (mut s, mut h, mut p) = scene();
    p.movement = Vec3::X;
    p.buttons = ATTACK;
    frames(&mut s, &mut h, p, 25);
    assert_eq!(s.phase, Phase::SmashCharge);
    assert!(!h.events.iter().any(|e| e.kind == EventKind::Hit));
    p.buttons = 0;
    frames(&mut s, &mut h, p, 1);
    let e = h.events.iter().find(|e| e.kind == EventKind::Hit).unwrap();
    assert_eq!(e.move_id, Move::SideSmash as u32);
    assert!(e.strength > 1.);
    let (mut s, mut h, mut p) = scene();
    p.movement = Vec3::X;
    frames(&mut s, &mut h, p, 10);
    p.buttons = ATTACK;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.current_move, Some(Move::SideTilt));
}
#[test]
fn shoulder_modifiers_select_vertical_smashes() {
    for (button, m) in [(UP, Move::UpSmash), (DOWN, Move::DownSmash)] {
        let (mut s, mut h, mut p) = scene();
        p.buttons = ATTACK | button;
        frames(&mut s, &mut h, p, 1);
        assert_eq!(s.current_move, Some(m));
    }
}
#[test]
fn charge_can_be_stored_then_fired_and_air_shot_does_not_charge() {
    let (mut s, mut h, mut p) = scene();
    p.buttons = SPECIAL;
    frames(&mut s, &mut h, p, 1);
    p.buttons = 0;
    frames(&mut s, &mut h, p, 20);
    assert!(s.shot_charge > 0);
    let charge = s.shot_charge;
    p.buttons = CANCEL;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.phase, Phase::Free);
    assert_eq!(s.current_move, None);
    assert!(s.shot_charge >= charge);
    p.buttons = 0;
    frames(&mut s, &mut h, p, 1);
    p.buttons = SPECIAL;
    frames(&mut s, &mut h, p, 1);
    p.buttons = 0;
    frames(&mut s, &mut h, p, 13);
    p.buttons = SPECIAL;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.shot_charge, 0);
    assert!(
        h.events
            .iter()
            .any(|e| e.kind == EventKind::Hit && e.move_id == 20)
    );
    let (mut s, mut h, mut p) = scene();
    s.teleport(Vec3::Y * 50.);
    p.buttons = SPECIAL;
    frames(&mut s, &mut h, p, 20);
    assert_eq!(s.shot_charge, 0);
    assert_eq!(s.phase, Phase::Recovery);
}
#[test]
fn host_skill_gate_blocks_move_and_extra_jumps_extend_double_jump() {
    let (mut s, mut h, mut p) = scene();
    p.allowed = 0;
    p.buttons = SPECIAL | UP;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.phase, Phase::Free);
    assert!(h.events.is_empty());
    s.options.extra_jumps = 1;
    s.teleport(Vec3::Y * 50.);
    s.jumps_used = 2;
    p.buttons = JUMP;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.jumps_used, 3);
}
#[test]
fn missile_flick_and_screw_iframes_are_distinct() {
    let (mut s, mut h, mut p) = scene();
    p.movement = Vec3::X;
    p.buttons = SPECIAL;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.current_move, Some(Move::SuperMissile));
    let (mut s, mut h, mut p) = scene();
    p.buttons = SPECIAL | UP;
    frames(&mut s, &mut h, p, 40);
    // The up special no longer traps the fighter in helpless fall; its i-frames
    // stay active after the move while airborne.
    assert_eq!(s.phase, Phase::Free);
    assert_ne!(s.defense_flags() & 128, 0);
    // The up-B consumes the double jump, so a jump press is still a no-op.
    p.buttons = JUMP;
    frames(&mut s, &mut h, p, 1);
    assert!(!h.events.iter().any(|e| e.kind == EventKind::Jump));
    // Launching another attack gives up the up-B i-frames midair.
    p.buttons = ATTACK;
    frames(&mut s, &mut h, p, 1);
    assert_eq!(s.current_move, Some(Move::NeutralAir));
    assert_eq!(s.defense_flags() & 128, 0);
}

#[test]
fn crowd_control_interrupts_startup_and_pad_flight_preserves_momentum() {
    let (mut s, mut h, mut p) = scene();
    p.buttons = ATTACK;
    frames(&mut s, &mut h, p, 1);
    s.interrupt();
    p.buttons = 0;
    frames(&mut s, &mut h, p, 30);
    assert!(!h.events.iter().any(|e| e.kind == EventKind::Hit));
    s.teleport(Vec3::Y * 100.);
    s.launch_pad(Vec3::new(8., 5., 0.), Vec3::new(0., -0.02, 0.));
    frames(&mut s, &mut h, p, 60);
    assert!((s.velocity.x - 8.).abs() < 1e-4);
    assert!((s.velocity.y - 3.8).abs() < 1e-3);
}
