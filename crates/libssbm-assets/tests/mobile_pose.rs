use libssbm::{Move, Params, Sim};
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
    }
    Ok(())
}
