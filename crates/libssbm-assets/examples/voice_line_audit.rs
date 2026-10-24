//! Per-clip voice-line audit: every command that plays a fighter's own voice during a move,
//! so an attack cry can never silently disappear again. Three commands produce voices:
//!   * opcode 17 behavior 2 (ftAction_80071B50 -> ft_800881D8): a named sound on the
//!     replaceable voice slot (the clip authors its own cry);
//!   * opcode 18 (ftAction_80071CCC): the smash grunt, one picked at random from the fighter
//!     data's smash-voice list (ftData +0x4C);
//!   * opcode 38 (ftAction_80071FC8): a random pick of up to six named sounds (the
//!     pseudo-random SFX); specials name their launch cry here (Fox's Fire Fox "Hyaa!").
//! Each named sound is checked against the fighter's extracted SEM bank so a cue that points
//! at a missing or silent sample is called out. Run:
//!   cargo run --release -p libssbm-assets --example voice_line_audit [fighter...]
use libssbm::action::Command;
use libssbm_assets::{
    animation::Animations,
    audio::{Bank, Mixer, RATE},
    fighter::*,
    Result,
};
use std::collections::BTreeMap;

fn audible(mixer: &mut Mixer, sound: u32) -> std::result::Result<bool, String> {
    let ids = mixer.bank.sample_ids(sound).map_err(|e| e.to_string())?;
    if ids.is_empty() || !ids.iter().all(|i| mixer.bank.samples.contains_key(i)) {
        return Ok(false);
    }
    mixer.clear();
    mixer.play(sound, 127, 64, 0).map_err(|e| e.to_string())?;
    let mut pcm = vec![0.; RATE as usize * 2];
    mixer.render(&mut pcm);
    Ok(pcm.iter().any(|x| x.abs() > 0.0001) && mixer.errors.is_empty())
}

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let mut names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        names = ["samus", "fox", "luigi", "pikachu", "captain", "link", "yoshi", "donkey"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
    let all: [(&str, &FighterAssets); 8] = [
        ("samus", &SAMUS),
        ("fox", &FOX),
        ("luigi", &LUIGI),
        ("pikachu", &PIKACHU),
        ("captain", &CAPTAIN),
        ("link", &LINK),
        ("yoshi", &YOSHI),
        ("donkey", &DONKEY),
    ];
    for name in &names {
        let Some((_, f)) = all.iter().find(|(n, _)| *n == name.as_str()) else {
            eprintln!("unknown fighter {name}");
            continue;
        };
        if !f.complete(&dir) {
            println!("== {name}: ABSENT");
            continue;
        }
        let animations = Animations::load_for(&dir, f)?;
        let actions = libssbm_assets::action::load_for(&dir, &animations, f)?;
        let mut mixer = Mixer::new(Bank::load_for(&dir, f)?);
        // Clip name -> [(command, description, sounds)].
        let mut lines: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (clip, action) in &actions {
            let mut out = Vec::new();
            for event in &action.commands {
                let Command::Raw { opcode, words } = &event.command else { continue };
                match *opcode {
                    17 if words[0] >> 18 & 255 == 2 => {
                        let s = words[1];
                        let ok = audible(&mut mixer, s).unwrap_or(false);
                        out.push(format!("voice {s} ({})", if ok { "ok" } else { "BAD" }));
                    }
                    18 => out.push("smash grunt (random from ftData)".to_string()),
                    38 => {
                        let range = words[0] & 0x3f;
                        let list: Vec<String> = words[1..1 + range.min(6) as usize]
                            .iter()
                            .map(|&s| {
                                let ok = audible(&mut mixer, s).unwrap_or(false);
                                format!("{s}{}", if ok { "" } else { " BAD" })
                            })
                            .collect();
                        out.push(format!("random voice [{}]", list.join(", ")));
                    }
                    _ => {}
                }
            }
            if !out.is_empty() {
                lines.insert(clip.clone(), out);
            }
        }
        println!("== {name} ({})", f.data.file);
        for (clip, out) in &lines {
            println!("  {clip:<22} {}", out.join("; "));
        }
        if lines.is_empty() {
            println!("  (no script voice commands)");
        }
    }
    Ok(())
}
