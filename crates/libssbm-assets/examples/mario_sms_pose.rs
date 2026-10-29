//! Write Mario's Melee A clip posed onto the Sunshine rig as one 4x4 matrix per
//! joint, for `libsms`'s viewer (`SMS_JOINTS`) to draw on the real Sunshine body.
//!
//! Usage: cargo run -p libssbm-assets --example mario_sms_pose -- <clip> <frame> <out.txt>
use std::path::PathBuf;

fn main() -> libssbm_assets::Result<()> {
    let mut args = std::env::args().skip(1);
    let clip = args.next().unwrap_or_else(|| "AttackS3S".into());
    let frame = args.next().and_then(|v| v.parse::<f32>().ok()).unwrap_or(10.0);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "mario_sms_pose.txt".into()));
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let sms = std::env::var("SMS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../libsms/assets"));
    let mario = &libssbm_assets::fighter::MARIO;
    if !mario.complete(&dir) {
        return Err("Mario assets absent".into());
    }
    let model = libssbm_assets::model::Model::load_for(&dir, 0, mario)?;
    let animations = libssbm_assets::animation::Animations::load_for(&dir, mario)?;
    let rig = libssbm_assets::sms::Rig::from_file(&sms.join("bmd/ma_mdl1.bmd"))?;
    let retarget = libssbm_assets::sms::Retarget::new(&model, &rig);
    let joints = retarget.pose_on_sms(&model, &rig, &animations, &clip, frame);
    let mut text = format!("# {clip} {frame}\n");
    for m in &joints {
        let c = m.to_cols_array();
        text.push_str(
            &c.iter()
                .map(|v| format!("{v:.7}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
        text.push('\n');
    }
    std::fs::write(&out, text)?;
    println!(
        "wrote {clip} frame {frame}: {} joints, hand_R {:?} -> {}",
        joints.len(),
        joints[18].w_axis.truncate(),
        out.display()
    );
    Ok(())
}
