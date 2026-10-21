//! Every fighter's own effect archive loads, and every effect id a fighter's moves spawn
//! resolves to something the host can draw (skipped without the player's extracted cache).
use libssbm::effect::{companion, resolve, Target};
use libssbm_assets::{effects::Effects, fighter::*, presentation::Presentation, Result};
use std::collections::BTreeSet;

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

#[test]
fn every_fighter_effect_archive_loads() -> Result<()> {
    let dir = dir();
    for (name, f) in [("captain", &CAPTAIN), ("link", &LINK), ("yoshi", &YOSHI), ("donkey", &DONKEY), ("fox", &FOX), ("luigi", &LUIGI), ("pikachu", &PIKACHU), ("samus", &SAMUS)] {
        if !dir.join(f.effects.file).exists() {
            eprintln!("SKIP: {name} effect archive absent");
            continue;
        }
        let effects = Effects::load_archive(&dir, f.effects).map_err(|e| format!("{name}: {e}"))?;
        assert!(!effects.models.is_empty(), "{name} has no effect models");
    }
    Ok(())
}

#[test]
fn the_effects_each_fighter_spawns_resolve_to_something_drawable() -> Result<()> {
    let dir = dir();
    if !dir.join(CAPTAIN.effects.file).exists() || !dir.join(DONKEY.effects.file).exists() {
        eprintln!("SKIP: effect archives absent");
        return Ok(());
    }
    let mut bank: BTreeSet<u32> = BTreeSet::new();
    for a in [COMMON_EFFECTS, SAMUS.effects, FOX.effects, LUIGI.effects, PIKACHU.effects, CAPTAIN.effects, YOSHI.effects] {
        bank.extend(Effects::load_archive(&dir, a)?.emitters.keys().copied());
    }
    // (fighter, effect ids its moves or hits spawn)
    let spawned: &[(&FighterAssets<'_>, &[u32])] = &[
        (&FOX, &[1160, 1161, 1162, 1163, 1164, 1165]),
        (&CAPTAIN, &[1094, 1167, 1168, 1169, 1170, 1171]),
        (&LINK, &[1211, 1212]),
        (&LUIGI, &[1289, 1290]),
        (&PIKACHU, &[1213, 1214, 1215, 1216, 1219]),
        (&DONKEY, &[1222, 1223, 1224, 1225, 1226, 1227, 1228]),
        (&YOSHI, &[1023, 1030, 1231]),
        // The hit sparks every fighter shares (ftColl_8007A06C).
        (&FOX, &[1000, 1001, 1002, 1004, 1005, 1011, 1046]),
    ];
    for (fighter, ids) in spawned {
        for &id in *ids {
            for target in std::iter::once(Some(resolve(id))).chain([companion(id)]).flatten() {
                match target {
                    Target::Particle(p) => assert!(bank.contains(&p), "effect {id}: no emitter {p} in the host bank"),
                    Target::Model(m) => {
                        Presentation::effect_for(&dir, m, fighter).map_err(|e| format!("effect {id}: model {m}: {e}"))?;
                    }
                    Target::Unknown => panic!("effect {id} is unresolved"),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
