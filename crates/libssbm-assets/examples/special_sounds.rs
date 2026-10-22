//! Lists the sound commands (ftAction_80071B50 opcode 17, the smash voice 18) in every Special*
//! clip's script, per fighter: the cues a special move should play when its clip script runs.
use libssbm::action::Command;
use libssbm_assets::{animation::Animations, fighter::*, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (name, f) in [("samus", &SAMUS), ("fox", &FOX), ("luigi", &LUIGI), ("pikachu", &PIKACHU), ("captain", &CAPTAIN), ("link", &LINK), ("yoshi", &YOSHI), ("donkey", &DONKEY)] {
        if !f.complete(&dir) { println!("{name}: ABSENT"); continue; }
        let animations = Animations::load_for(&dir, f)?;
        let actions = libssbm_assets::action::load_for(&dir, &animations, f)?;
        println!("== {name}");
        for (clip, action) in &actions {
            if !clip.starts_with("Special") { continue; }
            let mut sounds = Vec::new();
            for c in &action.commands {
                if let Command::Raw { opcode, words } = &c.command {
                    match opcode {
                        17 | 54 | 55 => sounds.push(format!("f{}:op{}:b{}:{}", c.frame, opcode, words[0] >> 18 & 255, words[1])),
                        18 => sounds.push(format!("f{}:smash", c.frame)),
                        _ => {}
                    }
                }
            }
            println!("  {clip} ({} frames): {}", action.frames, if sounds.is_empty() { "none".into() } else { sounds.join(" ") });
        }
    }
    Ok(())
}
