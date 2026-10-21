//! Audits every fighter's sounds: the bank decodes, every sound a clip script plays resolves to
//! audible PCM, and the voices the fighter data table (ftData +0x4C, `FtSFX`: smash, KO, jump,
//! hurt, tech, ledge, grab) names resolve too.
//!   cargo run --release -p libssbm-assets --example voice_audit
use libssbm::action::Command;
use libssbm_assets::{
    animation::Animations,
    audio::{Bank, Mixer, RATE},
    dat::Dat,
    fighter::*,
    Result,
};
use std::collections::{BTreeMap, BTreeSet};

const FIELDS: [(usize, &str, bool); 14] = [
    (0x00, "smash voice (array)", true),
    (0x04, "KO voice 1", false),
    (0x08, "KO voice 2", false),
    (0x0c, "star-KO / stamina KO voice", false),
    (0x10, "jump", false),
    (0x14, "air jump", false),
    (0x18, "ledge teeter", false),
    (0x1c, "hurt voices, light (array)", true),
    (0x20, "hurt voices, heavy (array)", true),
    (0x24, "tech / passive", false),
    (0x28, "ledge grab", false),
    (0x2c, "item pickup", false),
    (0x30, "grab connects", false),
    (0x34, "announcer / taunt id", false),
];

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
    for (name, f) in [
        ("samus", &SAMUS),
        ("fox", &FOX),
        ("luigi", &LUIGI),
        ("pikachu", &PIKACHU),
        ("captain", &CAPTAIN),
        ("link", &LINK),
        ("yoshi", &YOSHI),
        ("donkey", &DONKEY),
    ] {
        if !f.complete(&dir) {
            println!("{name}: ABSENT");
            continue;
        }
        let bank = Bank::load_for(&dir, f)?;
        let samples = bank.samples.len();
        let mut mixer = Mixer::new(bank);
        let name_call = audible(&mut mixer, f.announce_sound);
        let game_call = audible(&mut mixer, 0x7c855);
        println!(
            "== {name}: {samples} samples in {} banks, name call {:#x} {:?}, GAME! {:?}",
            f.samples.len(),
            f.announce_sound,
            name_call,
            game_call
        );

        // Sounds clip scripts play (command 17 and the footstep/landing stores 54, 55), and
        // the smash-voice command 18.
        let animations = Animations::load_for(&dir, f)?;
        let actions = libssbm_assets::action::load_for(&dir, &animations, f)?;
        let mut cues: BTreeMap<u32, usize> = BTreeMap::new();
        let mut smash_voice = 0;
        for action in actions.values() {
            for event in &action.commands {
                if let Command::Raw { opcode, words } = &event.command {
                    match opcode {
                        17 if words[0] >> 18 & 255 <= 6 || (10..=15).contains(&(words[0] >> 18 & 255)) => {
                            *cues.entry(words[1]).or_default() += 1;
                        }
                        18 => smash_voice += 1,
                        _ => {}
                    }
                }
            }
        }
        let mut bad = Vec::new();
        for &sound in cues.keys() {
            match audible(&mut mixer, sound) {
                Ok(true) => {}
                Ok(false) => bad.push(format!("{sound}: silent or missing sample")),
                Err(e) => bad.push(format!("{sound}: {e}")),
            }
        }
        let banks: BTreeSet<u32> = cues.keys().map(|s| s / 10000).collect();
        println!(
            "  script cues: {} distinct (sound banks {:?}), {} failed; smash-voice commands in scripts: {}",
            cues.len(),
            banks,
            bad.len(),
            smash_voice
        );
        for b in bad.iter().take(8) {
            println!("    BAD {b}");
        }

        // The fighter data's sound table.
        let bytes = std::fs::read(dir.join(f.data.file))?;
        let d = Dat::parse(&bytes)?;
        let root = d.root(f.data.root)?;
        let table = d.required_ptr(root + 0x4c)?;
        for (offset, label, array) in FIELDS {
            let mut ids = Vec::new();
            if array {
                if let Some(arr) = d.ptr(table + offset)? {
                    let n = d.u32(arr)? as usize;
                    if let Some(list) = d.ptr(arr + 4)? {
                        for i in 0..n.min(32) {
                            ids.push(d.u32(list + i * 4)?);
                        }
                    }
                }
            } else {
                ids.push(d.u32(table + offset)?);
            }
            let verdict: Vec<String> = ids
                .iter()
                .map(|&id| match audible(&mut mixer, id) {
                    Ok(true) => format!("{id}:ok"),
                    Ok(false) => format!("{id}:SILENT"),
                    Err(e) => format!("{id}:ERR({e})"),
                })
                .collect();
            let wired = matches!(offset, 0x10 | 0x14);
            println!("  +{offset:02x} {label:<28} {} {}", if wired { "WIRED " } else { "unwired" }, verdict.join(" "));
        }
    }
    Ok(())
}
