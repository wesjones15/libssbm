use libssbm_assets::{Result, presentation::Presentation};
#[test]
fn common_ko_palette_flash_and_return_assets_are_original_and_visible() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlCo.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let params = libssbm_assets::params::rebirth(&dir)?;
    assert_eq!(
        (
            params.ko_frames,
            params.arrival_frames,
            params.wait_frames,
            params.invulnerable_frames
        ),
        (60, 60, 240, 120)
    );
    assert!(params.flash.iter().any(|c| c[3] > 0.4) && params.flash.iter().any(|c| c[3] == 0.));
    assert_eq!(
        libssbm::effect::resolve(0x42b),
        libssbm::effect::Target::Model(25)
    );
    let mut ko = Presentation::effect(&dir, 25)?;
    ko.object_pose(12.);
    assert!(ko.duration > 0. && ko.model.batches.len() >= 2);
    let primary = &ko
        .model
        .batches
        .iter()
        .find(|b| b.owner == 1)
        .unwrap()
        .texture
        .as_ref()
        .unwrap()
        .rgba;
    assert!(
        primary.chunks_exact(4).any(|p| p[0] > p[1] && p[3] > 0),
        "source P1 palette missing from KO beam"
    );
    assert!(ko.model.batches.iter().enumerate().any(|(i, b)| {
        ko.color(i)[3] > 0.
            && b.texture
                .as_ref()
                .is_some_and(|t| t.rgba.chunks_exact(4).any(|p| p[3] > 0))
    }));
    let mut actor = Presentation::load(&dir, 0)?;
    let mut sim = libssbm::Sim::new(libssbm_assets::params::load(&dir)?, glam::Vec3::ZERO);
    sim.begin_rebirth(
        params,
        libssbm_assets::params::entry(&dir, &libssbm_assets::fighter::SAMUS)?,
        glam::Vec3::ZERO,
    );
    actor.update(&sim, 0.);
    assert!(
        actor
            .model
            .batches
            .iter()
            .enumerate()
            .all(|(i, _)| !actor.visible(i)),
        "KO fighter remained visible"
    );
    for _ in 0..60 {
        sim.advance(
            &mut libssbm::FlatHost::default(),
            libssbm::Input::idle(),
            1. / 60.,
        );
    }
    actor.update(&sim, 0.);
    assert!(
        actor
            .model
            .batches
            .iter()
            .enumerate()
            .any(|(i, _)| actor.visible(i)),
        "returning figurine remained hidden"
    );
    Ok(())
}
