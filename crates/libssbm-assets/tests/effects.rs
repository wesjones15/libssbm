use libssbm_assets::{Result, effects::Effects};
use std::path::PathBuf;
#[test]
fn real_effect_models_emitters_and_all_texture_frames_decode() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("EfSsData.dat").exists() {
        eprintln!("SKIP: effect archives absent");
        return Ok(());
    }
    for samus in [false, true] {
        let bank = Effects::load(&dir, samus).map_err(|e| format!("effects samus={samus}: {e}"))?;
        assert!(!bank.models.is_empty());
        assert!(!bank.emitters.is_empty());
        for model in &bank.models {
            for frame in [0., model.lifetime * 0.5, model.lifetime] {
                assert!(
                    model
                        .animations
                        .pose_object(&model.model, frame)
                        .iter()
                        .all(|m| m.is_finite())
                );
            }
        }
        for group in bank.textures.iter().flatten() {
            for frame in group.palettes.iter().flatten() {
                assert_eq!(
                    frame.rgba.len(),
                    frame.width as usize * frame.height as usize * 4
                );
            }
        }
        eprintln!(
            "effects samus={samus}: {} models, {} emitters, {} groups",
            bank.models.len(),
            bank.emitters.len(),
            bank.textures.len()
        );
    }
    Ok(())
}
