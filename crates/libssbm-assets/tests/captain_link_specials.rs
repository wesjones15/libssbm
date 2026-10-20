//! Regression guard: Captain Falcon's four specials and Link's Spin Attack
//! resolve to disc action scripts that actually carry hitboxes, so the moves
//! deal damage end-to-end through the shared hitbox sampler. Link's Bow,
//! Boomerang and Bomb intentionally have no body hitboxes: they are projectiles
//! whose damage lives on the item, not the fighter's action script.
use std::{collections::BTreeMap, path::PathBuf};

fn hitbox_count(actions: &BTreeMap<String, libssbm::action::ActionDef>, clip: &str) -> Option<usize> {
    actions.get(clip).map(|a| {
        a.commands
            .iter()
            .filter(|c| matches!(c.command, libssbm::action::Command::Hitbox(_)))
            .count()
    })
}

#[test]
fn captain_and_link_specials_carry_hitboxes() -> libssbm_assets::Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for kind in [4u32, 5u32] {
        let fighter = libssbm_assets::fighter::FighterKind::from_u32(kind).unwrap();
        let profile = fighter.assets().unwrap();
        if !profile.complete(&dir) {
            eprintln!("SKIP: kind {kind} cache absent");
            continue;
        }
        let animations = libssbm_assets::animation::Animations::load_for(&dir, profile)?;
        let actions = libssbm_assets::action::load_for(&dir, &animations, profile)?;
        if kind == 4 {
            for clip in ["SpecialN", "SpecialS", "SpecialHi", "SpecialLw"] {
                let hits = hitbox_count(&actions, clip)
                    .unwrap_or_else(|| panic!("Captain {clip} has no action script"));
                assert!(hits > 0, "Captain {clip} carries no hitboxes");
            }
        } else {
            for clip in ["SpecialHi", "SpecialAirHi"] {
                let hits = hitbox_count(&actions, clip)
                    .unwrap_or_else(|| panic!("Link {clip} has no action script"));
                assert!(hits > 0, "Link {clip} carries no hitboxes");
            }
        }
    }
    Ok(())
}
