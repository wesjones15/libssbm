//! The Super Scope's disc data (ItCo.dat and PlCo.dat): skipped when the player has not extracted the items archive.
use libssbm_assets::{
    article::{Article, Item},
    params,
};
use std::path::PathBuf;

fn dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if libssbm_assets::disc::items_present(&dir) && dir.join("PlCo.dat").exists() {
        Some(dir)
    } else {
        eprintln!("SKIP: the items archive is not extracted");
        None
    }
}

#[test]
fn the_scope_tuning_is_the_discs() {
    let Some(dir) = dir() else { return };
    let p = params::load_scope(&dir).unwrap();
    assert_eq!(p.ammo, 48);
    assert_eq!((p.charge_floor, p.charge_levels), (5, 8));
    assert_eq!(p.costs, [1, 2, 3, 4, 6, 8, 10, 12, 14, 16]);
    println!("hold_max {} rapid_shots {} muzzle {:?}", p.hold_max, p.rapid_shots, p.muzzle);
    assert!(p.hold_max > 8. && p.hold_max < 1000.);
    assert!(p.rapid_shots > 0 && p.rapid_shots < 100);
}

#[test]
fn the_beam_has_ten_levels_that_grow() {
    let Some(dir) = dir() else { return };
    let beams = params::load_scope_beams(&dir).unwrap();
    for (level, b) in beams.iter().enumerate() {
        println!("level {level}: {b:?}");
        assert!(b.velocity > 0. && b.lifetime > 0. && b.scale > 0., "level {level}: {b:?}");
    }
    assert!(beams[9].scale > beams[0].scale);
    assert!(beams[9].damage >= beams[0].damage);
}

#[test]
fn the_scope_and_its_beam_models_load() {
    let Some(dir) = dir() else { return };
    let scope = Article::load_item(&dir, Item::SuperScope, 0).unwrap();
    assert!(!scope.model.batches.is_empty(), "the scope has no model");
    for level in 0..10 {
        let beam = Article::load_item(&dir, Item::SuperScopeBeam, level).unwrap();
        assert!(!beam.model.batches.is_empty(), "beam level {level} has no model");
    }
}
