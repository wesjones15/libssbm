use libssbm_assets::{Result, fighter::SAMUS, presentation::Presentation};
use std::path::PathBuf;

#[test]
fn game_burst_model_loads_and_poses() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("assets");
    if !SAMUS.complete(&source) {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let mut game = Presentation::game(&source)?;
    assert!(!game.model.batches.is_empty(), "GAME! model has no triangles");
    assert!(
        game.model
            .batches
            .iter()
            .filter_map(|b| b.texture.as_ref())
            .any(|t| t.width > 0 && t.height > 0),
        "GAME! model has no texture"
    );
    for frame in [0., 4., 8., 16., 32.] {
        game.object_pose(frame);
        assert!(
            game.world.iter().all(|m| m.is_finite()),
            "GAME! pose not finite at frame {frame}"
        );
    }
    Ok(())
}
