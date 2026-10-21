//! Lists the part animations (script command 41) each fighter's clips start: clip, frame,
//! slot, animation index and blend frames.
use libssbm::action::Command;
use libssbm_assets::{animation::Animations, fighter::*, Result};
use std::collections::BTreeMap;

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (name, f) in [("fox", &FOX), ("luigi", &LUIGI), ("pikachu", &PIKACHU), ("captain", &CAPTAIN), ("link", &LINK), ("yoshi", &YOSHI), ("donkey", &DONKEY), ("samus", &SAMUS)] {
        if !f.complete(&dir) {
            println!("{name}: absent");
            continue;
        }
        let animations = Animations::load_for(&dir, f)?;
        let actions = libssbm_assets::action::load_for(&dir, &animations, f)?;
        let mut used: BTreeMap<(u32, u32), Vec<String>> = BTreeMap::new();
        for (clip, action) in &actions {
            for event in &action.commands {
                if let Command::Raw { opcode: 41, words } = &event.command {
                    let w = words[0];
                    let key = (w >> 19 & 0x7f, w >> 12 & 0x7f);
                    used.entry(key).or_default().push(format!("{clip}@{} blend {}", event.frame, w & 0xfff));
                }
            }
        }
        println!("{name}:");
        for ((slot, index), at) in used {
            println!("  slot {slot} anim {index}: {} uses, e.g. {:?}", at.len(), &at[..at.len().min(4)]);
        }
    }
    Ok(())
}
