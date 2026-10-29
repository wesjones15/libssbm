//! Mario's Melee A clips posed onto the Sunshine rig (the animation port).
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn sms_dir() -> PathBuf {
    std::env::var("SMS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../libsms/assets"))
}

#[test]
fn mario_a_clips_pose_onto_the_sunshine_rig() -> libssbm_assets::Result<()> {
    let assets = dir();
    let dir = assets.as_path();
    let mario = &libssbm_assets::fighter::MARIO;
    if !mario.complete(dir) {
        eprintln!("SKIP: Mario disc cache absent");
        return Ok(());
    }
    let sms = sms_dir();
    if !sms.join("bmd/ma_mdl1.bmd").exists() {
        eprintln!("SKIP: Sunshine assets absent");
        return Ok(());
    }
    let model = libssbm_assets::model::Model::load_for(dir, 0, mario)?;
    let animations = libssbm_assets::animation::Animations::load_for(dir, mario)?;
    let rig = libssbm_assets::sms::Rig::from_file(&sms.join("bmd/ma_mdl1.bmd"))?;
    let retarget = libssbm_assets::sms::Retarget::new(&model, &rig);
    let bind = retarget.pose_on_sms(&model, &rig, &animations, "Wait", 0.);
    assert_eq!(bind.len(), rig.bind.len());
    assert!(bind.iter().flat_map(|m| m.to_cols_array()).all(f32::is_finite));
    let attack = retarget.pose_on_sms(&model, &rig, &animations, "AttackS3S", 10.);
    assert_eq!(attack.len(), rig.bind.len());
    assert!(
        attack.iter().flat_map(|m| m.to_cols_array()).all(f32::is_finite),
        "non-finite Sunshine pose"
    );
    // Scale is dropped, not accumulated: every joint stays a rigid rotation.
    // (Taking `Animations::pose` as local matrices and multiplying the chain
    // again produced a 44x leg and a body lying on its back.)
    for (i, m) in attack.iter().enumerate() {
        let s = m.x_axis.truncate().length();
        assert!((s - 1.0).abs() < 0.02, "joint {i} scale {s}");
    }
    // The right hand is joint 18 (jnt_hand_R): a side tilt must move it well
    // clear of the idle pose.
    let moved = (attack[18].w_axis.truncate() - bind[18].w_axis.truncate()).length();
    eprintln!("side tilt moves jnt_hand_R by {moved:.2} Sunshine units");
    assert!(moved > 1.0, "side tilt barely moved the hand ({moved})");
    Ok(())
}
