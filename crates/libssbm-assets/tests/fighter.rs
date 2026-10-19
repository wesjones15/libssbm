use libssbm_assets::{
    Result,
    fighter::{Archive, SAMUS},
};
use std::path::PathBuf;

#[test]
fn dependencies_and_path_bounds_are_character_owned() -> Result<()> {
    assert_eq!(SAMUS.required()?, libssbm_assets::REQUIRED);
    for bad in [
        "../outside.dat",
        "/outside.dat",
        "C:/outside.dat",
        "nested\\outside.dat",
        "a//b.dat",
        "./a.dat",
    ] {
        let mut fighter = SAMUS;
        fighter.data.file = bad;
        assert!(fighter.required().is_err(), "accepted {bad}");
    }
    let fighter = libssbm_assets::fighter::FighterAssets {
        costumes: &[],
        ..SAMUS
    };
    assert!(fighter.required().is_err());
    Ok(())
}

#[test]
fn renamed_profile_loads_without_any_canonical_samus_asset_names() -> Result<()> {
    use libssbm_assets::{
        animation::Animations, article::Article, audio::Bank, model::Model,
        presentation::Presentation,
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("assets");
    if !SAMUS.complete(&source) {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let dir = root.join("build/renamed-fighter-profile");
    std::fs::create_dir_all(&dir)?;
    let fighter = libssbm_assets::fighter::FighterAssets {
        data: Archive {
            file: "Other.dat",
            root: "ftDataOther",
        },
        animations: "OtherAJ.dat",
        costumes: &["OtherNr.dat"],
        effects: Archive {
            file: "OtherEffects.dat",
            root: SAMUS.effects.root,
        },
        samples: &["samples/fighter.ssm", "samples/common.ssm"],
        sequences: "samples/sequences.sem",
        ..SAMUS
    };
    // Only renamed assets are created. A hardcoded Samus fallback must fail here.
    for (from, to) in [
        (SAMUS.animations, fighter.animations),
        (SAMUS.costumes[0], fighter.costumes[0]),
        (SAMUS.effects.file, fighter.effects.file),
        (SAMUS.samples[0], fighter.samples[0]),
        (SAMUS.samples[1], fighter.samples[1]),
        (SAMUS.sequences, fighter.sequences),
        ("PlCo.dat", "PlCo.dat"),
        ("MnSlChr.usd", "MnSlChr.usd"),
        ("EfCoData.dat", "EfCoData.dat"),
        (libssbm_assets::fighter::RESULTS_SCENE.file, "IfAll.usd"),
    ] {
        let target = dir.join(to);
        std::fs::create_dir_all(target.parent().unwrap())?;
        std::fs::copy(source.join(from), target)?;
    }
    let mut bytes = std::fs::read(source.join(SAMUS.data.file))?;
    let old = SAMUS.data.root.as_bytes();
    let at = bytes
        .windows(old.len())
        .position(|b| b == old)
        .ok_or("fighter symbol missing")?;
    assert_eq!(old.len(), fighter.data.root.len());
    bytes[at..at + old.len()].copy_from_slice(fighter.data.root.as_bytes());
    std::fs::write(dir.join(fighter.data.file), bytes)?;
    assert!(!dir.join(SAMUS.data.file).exists());
    assert!(fighter.complete(&dir));
    let animations = Animations::load_for(&dir, &fighter)?;
    assert!(
        libssbm_assets::action::load_for(&dir, &animations, &fighter)?.contains_key("Attack11")
    );
    assert!(!Model::load_for(&dir, 0, &fighter)?.batches.is_empty());
    assert!(Model::load_for(&dir, 1, &fighter).is_err());
    assert_eq!(Bank::load_for(&dir, &fighter)?.samples.len(), 271);
    let mut view = Presentation::load_for(&dir, 0, &fighter)?;
    view.object_pose(2.);
    assert!(view.world.iter().all(|m| m.is_finite()));
    for (kind, layout) in fighter.articles.iter().enumerate() {
        for state in 0..layout.states {
            let article = Article::load_for(&dir, kind as u32, state, &fighter)?;
            assert!(!article.model.batches.is_empty());
        }
        assert!(Article::load_for(&dir, kind as u32, layout.states, &fighter).is_err());
    }
    let mut effect = Presentation::effect_from(&dir, fighter.effects, 0)?;
    effect.object_pose(8.);
    assert!(effect.world.iter().all(|m| m.is_finite()));
    Ok(())
}
