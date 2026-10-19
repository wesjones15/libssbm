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
    assert!(yaw.abs() < 1e-4);
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
        sim.options.mobile_moves = 1 << Move::ChargeShot as u32;
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

        // Preserve the original raised-to-forward cannon motion. At its forward
        // firing pose, camera yaw turns the torso while head height stays put.
        sim.current_move = Some(Move::Missile);
        sim.locomotion_weight = 0.;
        sim.motion.select("SpecialS", 1.);
        sim.motion.frame = 18.;
        let source = animations.pose(&model, "SpecialS", 18., false);
        let aimed = mobile_pose(&model, &animations, &sim, 18., Some(root));
        for joint in [4, 7, 8, 14, 15] {
            assert!(
                (aimed[joint] - source[joint])
                    .to_cols_array()
                    .iter()
                    .all(|v| v.abs() < 1e-3)
            );
        }
        assert!((aimed[root].w_axis - source[root].w_axis).length() < 1e-3);
        assert!((aimed[41].w_axis.y - source[41].w_axis.y).abs() < 1e-3);
        let aimed_start = mobile_pose(&model, &animations, &sim, 12., Some(root));
        let raised = (aimed_start[51].w_axis.truncate() - aimed_start[49].w_axis.truncate())
            .normalize_or_zero();
        assert!(raised.y > 0.8, "lost the source raised-cannon start: {raised:?}");
        sim.facing = glam::Vec3::Z; // The body turn lags the camera's +X aim.
        let turning = mobile_pose(&model, &animations, &sim, 18., Some(root));
        let torso_turn = (turning[20].x_axis.truncate() - source[20].x_axis.truncate()).length();
        assert!(torso_turn > 0.5, "side-B torso did not track camera yaw");
        assert!(turning[41].w_axis.y > turning[root].w_axis.y + 3.);
        assert!(
            model
                .skin(0, &aimed)
                .iter()
                .all(|(p, n)| p.is_finite() && n.is_finite())
        );
    }
    Ok(())
}
