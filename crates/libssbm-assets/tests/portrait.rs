use libssbm_assets::{Result, fighter::SAMUS, portrait};
#[test]
fn original_css_photos_decode_for_every_samus_costume() -> Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("assets");
    if !dir.join("MnSlChr.usd").exists() {
        eprintln!("SKIP: CSS archive absent");
        return Ok(());
    }
    let mut previous = None;
    for costume in 0..SAMUS.costumes.len() {
        let photo = portrait::load(&dir, &SAMUS, costume)?;
        assert!(photo.width >= 64 && photo.height >= 64);
        assert_eq!(
            photo.rgba.len(),
            photo.width as usize * photo.height as usize * 4
        );
        assert!(
            photo
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] > 128 && p[0] > 32)
        );
        if let Some(old) = previous {
            assert_ne!(old, photo.rgba, "costume reused another portrait");
        }
        if costume == 0 {
            std::fs::create_dir_all(root.join("build"))?;
            std::fs::write(root.join("build/samus-css-portrait.rgba"), &photo.rgba)?;
            std::fs::write(
                root.join("build/samus-css-portrait.size"),
                format!("{} {}", photo.width, photo.height),
            )?;
        }
        previous = Some(photo.rgba);
    }
    assert!(portrait::load(&dir, &SAMUS, 99).is_err());
    Ok(())
}
