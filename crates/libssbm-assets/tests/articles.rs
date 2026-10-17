use libssbm_assets::{Result, article::Article};
use std::path::PathBuf;
#[test]
fn disc_article_meshes_textures_and_object_animations() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: disc cache absent");
        return Ok(());
    }
    for (kind, count) in [(0, 2), (1, 9), (2, 4), (3, 1)] {
        for state in 0..count {
            let article = Article::load(&dir, kind, state)
                .map_err(|e| format!("article {kind}/{state}: {e}"))?;
            assert!(
                !article.model.batches.is_empty(),
                "article {kind}/{state} empty"
            );
            assert!(article.special.iter().all(|x| x.is_finite()));
            if kind < 3 && !(kind == 1 && state == 0) && !(kind == 2 && state >= 2) {
                assert!(
                    article
                        .commands
                        .iter()
                        .any(|c| matches!(c.command, libssbm::action::Command::Hitbox(_))),
                    "missing item hitbox {kind}/{state}"
                );
            }
            if kind == 0 && state == 1 || kind == 2 && state >= 2 {
                assert!(article.commands.iter().any(|c|matches!(&c.command,libssbm::action::Command::Raw {opcode:19,words} if words[0]&0x03ffffff!=0)),"missing source explosion end {kind}/{state}");
            }
            let clip = &article.animations.clips["Object"];
            for frame in [0., clip.frames * 0.5, clip.frames] {
                let pose = article.animations.pose_object(&article.model, frame);
                assert!(
                    pose.iter().all(|m| m.is_finite()),
                    "article {kind}/{state} frame {frame}"
                );
                for batch in 0..article.model.batches.len() {
                    assert!(
                        article
                            .model
                            .skin(batch, &pose)
                            .iter()
                            .all(|(p, n)| p.is_finite() && n.is_finite())
                    );
                }
            }
        }
    }
    Ok(())
}
