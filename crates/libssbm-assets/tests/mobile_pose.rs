use libssbm::{Move, Params, Sim, side_special_upper_angles};
use libssbm_assets::{
    Result,
    animation::Animations,
    fighter::SAMUS,
    model::Model,
    presentation::{mobile_pose, splice_pose},
};
use std::path::PathBuf;

fn local(model: &Model, world: &[glam::Mat4], joint: usize) -> glam::Mat4 {
    model.joints[joint]
        .parent
        .map_or(world[joint], |p| world[p].inverse() * world[joint])
}

#[test]
fn samus_walk_legs_keep_the_action_torso_and_cannon() -> Result<()> {
    let (pitch, yaw) = side_special_upper_angles(glam::Vec3::Z, glam::Vec3::X);
    assert!(pitch.abs() < 1e-4);
    assert!((yaw.abs() - std::f32::consts::PI).abs() < 1e-4);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("assets");
    if !SAMUS.complete(&dir) {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    for costume in 0..SAMUS.costumes.len() {
        let model = Model::load(&dir, costume)?;
        let root = SAMUS.upper_body_root.unwrap();
        assert_eq!(model.joints[root].parent, Some(4));
        let action = animations.pose(&model, "SpecialNHold", 5., true);
        let gait = animations.pose(&model, "WalkFast", 11., true);
        let mixed = splice_pose(&model, &gait, &action, root);
        for joint in [7, 8, 14, 15] {
            assert!(
                (local(&model, &mixed, joint).w_axis - local(&model, &gait, joint).w_axis).length()
                    < 1e-4
            );
        }
        for joint in [19, 20, 49, 50, 51] {
            let diff = local(&model, &mixed, joint) - local(&model, &action, joint);
            assert!(
                diff.to_cols_array().iter().all(|v| v.abs() < 1e-4),
                "costume {costume}, joint {joint}"
            );
        }
        assert!(
            model
                .skin(0, &mixed)
                .iter()
                .all(|(p, n)| p.is_finite() && n.is_finite())
        );

        let mut sim = Sim::new(Params::fixture(), glam::Vec3::ZERO);
        sim.grounded = true;
        sim.current_move = Some(Move::ChargeShot);
        sim.motion.definitions = animations.motion_definitions();
        sim.locomotion.definitions = sim.motion.definitions.clone();
        sim.motion.select("SpecialNHold", 1.);
        sim.motion.frame = 5.;
        sim.locomotion.select("WalkFast", 1.);
        sim.locomotion.frame = 11.;
        sim.locomotion_weight = 1.;
        let presented = mobile_pose(&model, &animations, &sim, 5., Some(root));
        assert!(
            presented
                .iter()
                .zip(&mixed)
                .all(|(a, b)| (a.w_axis - b.w_axis).length() < 1e-4)
        );

        // Side-B: the disc clips fire toward local +X and bake that turn into the pelvis and
        // feet. It is layered over the idle legs even standing still, so those never turn, and
        // only the upper rig is turned so the cannon fires forward. In the host's mirrored
        // barrel space the forward aim reads as -Z (in-game the cannon points away with the
        // missile, verified against the RoR2 playtest).
        let barrel = |pose: &[glam::Mat4]| {
            let v = pose[56].w_axis.truncate() - pose[51].w_axis.truncate();
            glam::Vec3::new(-v.x, v.y, v.z).normalize_or_zero()
        };
        sim.current_move = Some(Move::Missile);
        sim.locomotion_weight = 0.; // standing still: no walking blend
        sim.locomotion.select("Wait", 1.);
        sim.locomotion.frame = 0.;
        sim.motion.select("SpecialS", 1.);
        sim.motion.frame = 18.;
        sim.facing = glam::Vec3::X; // The sim's default aim is +X: no heading difference.
        let source = animations.pose(&model, "SpecialS", 18., false);
        let idle = animations.pose(&model, "Wait", 0., true);
        let aimed = mobile_pose(&model, &animations, &sim, 18., Some(root));
        for joint in [4, 7, 8, 14, 15] {
            assert!(
                (aimed[joint] - idle[joint]).to_cols_array().iter().all(|v| v.abs() < 1e-3),
                "side-B changed the idle legs/pelvis (joint {joint})"
            );
        }
        assert!(
            (aimed[4] - source[4]).to_cols_array().iter().any(|v| v.abs() > 0.1),
            "the clip's sideways pelvis turn leaked into the pose"
        );
        let forward = barrel(&aimed);
        assert!(forward.z < -0.9 && forward.y.abs() < 0.3, "side-B cannon not forward: {forward:?}");
        let raised = (mobile_pose(&model, &animations, &sim, 12., Some(root))[51].w_axis.truncate()
            - mobile_pose(&model, &animations, &sim, 12., Some(root))[49].w_axis.truncate())
        .normalize_or_zero();
        assert!(raised.y > 0.8, "lost the source raised-cannon start: {raised:?}");
        // The body lags the camera by 90 degrees: in the body's frame the cannon turns away
        // with the camera aim, and the legs still do not follow it.
        sim.facing = glam::Vec3::Z;
        let turning = mobile_pose(&model, &animations, &sim, 18., Some(root));
        let aim = barrel(&turning);
        assert!(aim.x < -0.9, "side-B cannon did not track the camera aim: {aim:?}");
        for joint in [4, 7, 8, 14, 15] {
            assert!(
                (turning[joint] - idle[joint]).to_cols_array().iter().all(|v| v.abs() < 1e-3),
                "aim turn moved the legs (joint {joint})"
            );
        }
        assert!(turning[41].w_axis.y > turning[root].w_axis.y + 3.);
        // Airborne is layered the same way: the clip's turned pelvis never shows.
        sim.facing = glam::Vec3::X;
        sim.grounded = false;
        sim.motion.select("SpecialAirS", 1.);
        sim.motion.frame = 14.;
        let air = mobile_pose(&model, &animations, &sim, 14., Some(root));
        let air_barrel = barrel(&air);
        assert!(air_barrel.z < -0.9, "airborne side-B cannon not forward: {air_barrel:?}");
        assert!((air[4] - idle[4]).to_cols_array().iter().all(|v| v.abs() < 1e-3));
        let aimed = air;
        assert!(
            model
                .skin(0, &aimed)
                .iter()
                .all(|(p, n)| p.is_finite() && n.is_finite())
        );
    }
    Ok(())
}
