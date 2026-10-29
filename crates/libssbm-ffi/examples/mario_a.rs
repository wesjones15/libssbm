//! Drive Mario's contextual A attacks through the shared sim and print the clip,
//! frame and any hitboxes it spawns — a headless proof before the animation port.
use glam::Vec3;
use libssbm::{sim::{FlatHost, Input, ATTACK}, Move, Sim};
use std::path::Path;

fn main() -> libssbm_assets::Result<()> {
    let dir = Path::new("assets");
    let f = &libssbm_assets::fighter::MARIO;
    if !f.complete(dir) {
        eprintln!("SKIP: Mario assets absent (run ssbm-extract ... mario)");
        return Ok(());
    }
    let anim = libssbm_assets::animation::Animations::load_for(dir, f)?;
    let attacks = [
        ("jab", Move::Jab),
        ("side tilt", Move::SideTilt),
        ("up tilt", Move::UpTilt),
        ("down tilt", Move::DownTilt),
        ("side smash", Move::SideSmash),
        ("up smash", Move::UpSmash),
        ("down smash", Move::DownSmash),
        ("dash attack", Move::DashAttack),
        ("neutral air", Move::NeutralAir),
        ("forward air", Move::ForwardAir),
        ("back air", Move::BackAir),
        ("up air", Move::UpAir),
        ("down air", Move::DownAir),
    ];
    for (label, mv) in attacks {
        let mut sim = Sim::with_behavior(
            libssbm_assets::params::load_common(dir, f)?,
            Box::new(libssbm::mario::Mario::new()),
            Vec3::ZERO,
        );
        sim.motion.definitions = anim.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for_variants(dir, &anim, f)?;
        for (name, action) in &sim.action.definitions {
            if let Some(motion) = sim.motion.definitions.get_mut(name) {
                motion.looping |= action.looping;
            }
        }
        let mut host = FlatHost::default();
        let idle = Input::idle();
        for _ in 0..5 {
            sim.advance(&mut host, idle, 1.0 / 60.0);
        }
        let mut pressed = Input::idle();
        pressed.buttons = ATTACK;
        sim.start_move(&mut host, mv, pressed);
        let mut hits = 0;
        let mut first = None;
        let mut last_clip = "";
        for t in 0..90 {
            sim.advance(&mut host, idle, 1.0 / 60.0);
            for h in &sim.action.samples {
                if first.is_none() {
                    first = Some((t, h.hitbox.damage, h.hitbox.radius, h.hitbox.angle));
                }
                hits += 1;
            }
            last_clip = sim.motion.clip;
        }
        println!("{label:12} -> clip {last_clip:12} samples {hits:3} first {first:?}");
    }
    Ok(())
}
