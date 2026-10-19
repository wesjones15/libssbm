use glam::Vec3;
use libssbm::projectile::{Params, Projectile};

fn missile(life: f32) -> Projectile {
    Projectile::new(
        Params {
            kind: 2,
            state: 0,
            life,
            speed: 10.,
            acceleration: 0.,
            max_speed: 10.,
            delay: 0.,
            homing_frames: 5.,
            decay: 0.5,
            min_speed: 1.,
            turn_accel: 0.,
            max_turn: 0.,
            min_angle: 0.,
            gravity: 0.,
            max_fall: 0.,
            initial_y: 0.,
            damage: 5.,
            radius: 1.,
            explosion_radius: 1.,
            explosion_frames: 10.,
        },
        Vec3::X,
    )
}

fn run(p: &mut Projectile, seconds: f64) {
    for _ in 0..(seconds * 60.) as u32 {
        p.advance(1. / 60., None);
    }
}

// A host that lifts the lifetime cap (unlimited range) must not leave the missile crawling
// at its post-homing minimum speed: it cruises on until the host removes it.
#[test]
fn unlimited_missile_keeps_its_cruise_speed_past_the_homing_window() {
    let mut p = missile(f32::INFINITY);
    run(&mut p, 2.);
    assert!(!p.impacted && !p.finished);
    assert!((p.velocity.length() - 10.).abs() < 1e-3, "slowed to {}", p.velocity.length());
    assert!(p.position.x > 1000.);
}

#[test]
fn limited_missile_still_slows_after_homing_and_detonates_at_its_life() {
    let mut p = missile(100.);
    run(&mut p, 1.);
    assert!(p.velocity.length() <= 1.01, "disc decay lost: {}", p.velocity.length());
    run(&mut p, 2.);
    assert!(p.impacted);
}
