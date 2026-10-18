use glam::Vec3;
use libssbm::*;
use libssbm_assets::{Result, animation::Animations};
use std::path::PathBuf;
#[test]
fn extracted_air_dodge_has_invulnerability_and_finishes() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    let mut s = Sim::new(libssbm_assets::params::load(&dir)?, Vec3::Y * 1000.);
    s.defense.params = libssbm_assets::params::defense(&dir)?;
    s.defense.health = s.defense.params.maximum;
    let full = s.shield_scale();
    assert!(full > 1. && full < 100., "invalid source shield radius");
    s.defense.health *= 0.25;
    assert!(s.shield_scale() < full);
    s.defense.health = s.defense.params.maximum;
    for id in [11, 13, 14] {
        let p = libssbm_assets::presentation::Presentation::effect(&dir, id)?;
        assert!(
            p.model
                .batches
                .iter()
                .filter_map(|b| b.texture.as_ref())
                .any(|t| t.rgba.chunks_exact(4).any(|c| c[0] != c[1] || c[1] != c[2])),
            "shield player palette missing for {id}"
        );
    }
    s.motion.definitions = animations.motion_definitions();
    s.action.definitions = libssbm_assets::action::load(&dir, &animations)?;
    let mut h = FlatHost::default();
    let mut immune = false;
    for _ in 0..100 {
        s.advance(
            &mut h,
            Input {
                buttons: DEFEND,
                ..Input::idle()
            },
            1. / 60.,
        );
        immune |= s.defense_flags() & 2 != 0;
    }
    assert!(immune, "source invulnerability not executed");
    assert_eq!(s.phase, Phase::Helpless);
    assert_eq!(s.defense_flags(), 0);
    Ok(())
}
