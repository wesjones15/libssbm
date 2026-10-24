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
            max_bounces: 0,
            bounce_factor: 0.5,
            straight_distance: 0.,
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

#[test]
fn arc_shot_flies_level_until_its_straight_distance() {
    let mut p = Projectile::new(
        Params {
            kind: 1,
            state: 0,
            life: f32::INFINITY,
            speed: 10.,
            acceleration: 0.,
            max_speed: 10.,
            delay: 0.,
            homing_frames: 0.,
            decay: 1.,
            min_speed: 0.,
            turn_accel: 0.,
            max_turn: 0.,
            min_angle: 0.,
            gravity: 0.06,
            max_fall: 2.,
            initial_y: 0.,
            damage: 10.,
            radius: 1.,
            explosion_radius: 1.,
            explosion_frames: 0.,
            max_bounces: 10,
            bounce_factor: 1.,
            straight_distance: 200.,
        },
        Vec3::X,
    );
    p.arc = true;
    for _ in 0..19 {
        p.advance(1. / 60., None);
    }
    assert!(p.position.y.abs() < 1e-6, "arc began before 20 m: y {}", p.position.y);
    for _ in 0..3 {
        p.advance(1. / 60., None);
    }
    assert!(p.position.y < 0., "arc never began after 20 m: y {}", p.position.y);
}

// Luigi's Fireball: a downward ground contact reflects upward (damped) and the
// projectile terminates after its bounce budget, never bouncing a wall/ceiling.
#[test]
fn fireball_bounces_on_ground_then_terminates() {
    let mut p = Projectile::new(
        Params {
            kind: 0,
            state: 0,
            life: f32::INFINITY,
            speed: 10.,
            acceleration: 0.,
            max_speed: 10.,
            delay: 0.,
            homing_frames: 0.,
            decay: 1.,
            min_speed: 0.,
            turn_accel: 0.,
            max_turn: 0.,
            min_angle: 0.,
            gravity: 1.,
            max_fall: 5.,
            initial_y: 0.,
            damage: 5.,
            radius: 1.,
            explosion_radius: 1.,
            explosion_frames: 0.,
            max_bounces: 10,
            bounce_factor: 0.5,
            straight_distance: 0.,
        },
        Vec3::X,
    );
    p.velocity = Vec3::new(10., -4., 0.);
    assert!(p.bounce(), "first ground contact should bounce");
    assert!((p.velocity.y - 2.).abs() < 1e-3, "damped rise: {}", p.velocity.y);
    assert!(!p.impacted);
    for _ in 0..9 {
        assert!(p.bounce(), "should keep bouncing within budget");
    }
    assert!(!p.bounce(), "should terminate after the bounce budget");
    assert!(p.impacted);
}

// A projectile without a bounce budget never reflects; it just terminates.
#[test]
fn no_bounce_budget_terminates_immediately() {
    let mut p = missile(100.);
    assert!(!p.bounce(), "a missile has no ground bounce");
    assert!(!p.impacted, "bounce rejection is not an impact");
}
