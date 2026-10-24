//! Real-disc regression for the held ball, ground/air continuity and bomb exclusion.
use glam::Vec3;
use libssbm::{samus::{Samus, HOST_MORPH_ROLL}, FlatHost, Input, Move, Sim, DOWN_B, JUMP, EventKind};
use libssbm_assets::{fighter::SAMUS, presentation::Presentation, Result};

#[test]
fn ball_drops_one_bomb_freezes_spins_jumps_and_exits() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !SAMUS.complete(&dir) { eprintln!("SKIP: Samus disc cache absent"); return Ok(()); }
    let mut view = Presentation::load(&dir, 0)?;
    for fps in [30, 60, 144] {
        for armor_break in [false, true] {
            let params = libssbm_assets::params::load_common(&dir, &SAMUS)?;
            let specials = libssbm_assets::params::load_samus_specials(&dir)?;
            let mut sim = Sim::with_behavior(params, Box::new(Samus::new(specials, SAMUS.upper_body_root)), Vec3::ZERO);
            sim.motion.definitions = view.animations.motion_definitions();
            sim.action.definitions = libssbm_assets::action::load(&dir, &view.animations)?;
            let mut host = FlatHost::default();
            sim.advance(&mut host, Input::idle(), 1./60.);
            if armor_break { sim.teleport(Vec3::Y * 1000.); }
            let clip = if armor_break { "SpecialAirLw" } else { "SpecialLw" };
            sim.host_flags = HOST_MORPH_ROLL;
            let mut pad = Input { buttons: DOWN_B, movement: Vec3::Z, aim: Vec3::Z, ..Input::idle() };
            let mut air_trail = false;
            for _ in 0..fps {
                sim.advance(&mut host, pad, 1./fps as f64);
                air_trail |= sim.action.effects.iter().any(|e| e.id==1023 && e.origin.y>0.);
            }
            view.update(&sim, 0.);
            assert!(view.ball);
            assert_eq!(sim.motion.frame, 13., "ball pose was not frozen at {fps} FPS");
            assert_eq!(sim.motion.rate, 0.);
            assert_eq!(view.frame, 13., "render interpolation advanced the ball clip");
            let held_pose = view.world.clone();
            if armor_break { assert!(air_trail, "airborne roll lost its Yoshi smoke trail"); }
            let travel = sim.morph_roll_distance;
            // Jump and land while held: neither transition may restart the morph.
            pad.buttons |= JUMP;
            sim.advance(&mut host, pad, 1./30.);
            pad.buttons = DOWN_B;
            for _ in 0..fps * 11 {
                sim.advance(&mut host, pad, 1./fps as f64);
                assert_eq!(sim.current_move, Some(Move::SpecialLw));
                assert_eq!(sim.motion.clip, clip);
                assert_eq!(sim.motion.frame, 13., "ground/air or timeout restarted the morph");
                view.update(&sim, 0.);
                assert!(view.ball);
                for (i, batch) in view.model.batches.iter().enumerate() {
                    if view.visible(i) { assert_eq!(batch.dobj, 19, "folded/normal suit visible in held ball"); }
                }
            }
            assert!(sim.morph_roll_distance > travel + 10.);
            assert!(held_pose[3].to_cols_array().iter().zip(view.world[3].to_cols_array()).any(|(a,b)| (a-b).abs()>0.1), "ball never rotated");
            assert_eq!(host.events.iter().filter(|e| e.kind == EventKind::Hit && e.move_id == 24).count(), 1, "combined roll must drop exactly one startup bomb");
            if armor_break { sim.roll_break = true; } else { pad.buttons = 0; }
            sim.advance(&mut host, pad, 1./30.);
            view.update(&sim, 0.);
            assert!(sim.current_move.is_none());
            assert!(!sim.morph_roll_hold && !view.ball, "exit left the folded alternate model visible");
            assert!(!sim.motion.clip.contains("Special"));
            // Reset/interrupt must not leave a stale ball visibility latch.
            sim.interrupt();
            view.update(&sim, 0.);
            assert!(!view.ball);
        }
    }
    // Ordinary down-B still emits exactly one bomb (ftSs_SpecialLw_8012ADF0).
    let params = libssbm_assets::params::load_common(&dir, &SAMUS)?;
    let specials = libssbm_assets::params::load_samus_specials(&dir)?;
    let mut sim = Sim::with_behavior(params, Box::new(Samus::new(specials, SAMUS.upper_body_root)), Vec3::ZERO);
    sim.motion.definitions = view.animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load(&dir, &view.animations)?;
    let mut host = FlatHost::default();
    sim.advance(&mut host, Input::idle(), 1./60.);
    for _ in 0..90 { sim.advance(&mut host, Input { buttons: DOWN_B, ..Input::idle() }, 1./60.); }
    assert_eq!(host.events.iter().filter(|e|e.kind == EventKind::Hit && e.move_id == 24).count(), 1);
    Ok(())
}
