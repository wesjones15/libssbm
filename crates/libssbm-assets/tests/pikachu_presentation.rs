//! Pikachu's models, portrait, sounds and effect ids against the player's disc
//! cache (skipped when it is absent): every clip his specials name exists, every
//! script cue resolves, every script sound is playable.
use libssbm::effect::{self, Target};
use libssbm_assets::{
    Result, animation::Animations, audio, effects::Effects, fighter::PIKACHU, portrait,
    presentation::Presentation,
};

fn dir() -> Option<std::path::PathBuf> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if PIKACHU.complete(&dir) {
        Some(dir)
    } else {
        eprintln!("SKIP: Pikachu disc cache absent");
        None
    }
}

#[test]
fn every_costume_loads_with_the_clips_his_specials_use() -> Result<()> {
    let Some(dir) = dir() else { return Ok(()) };
    for costume in 0..PIKACHU.costumes.len() {
        let p = Presentation::load_for(&dir, costume, &PIKACHU)?;
        assert!((0..p.model.batches.len()).any(|i| p.visible(i)), "costume {costume} is invisible");
        for clip in [
            "Wait", "Attack11", "AttackDash", "AttackS3S", "AttackS4S", "AttackHi4", "AttackLw4",
            "SpecialN", "SpecialAirN", "SpecialSStart", "SpecialSHold", "SpecialS", "SpecialSEnd",
            "SpecialAirSStart", "SpecialAirSHold", "SpecialAirSEnd", "SpecialHiStart",
            "SpecialHiEnd", "SpecialAirHiStart", "SpecialAirHiEnd", "SpecialLwStart",
            "SpecialLwLoop", "SpecialLwEnd", "SpecialAirLwStart", "SpecialAirLwLoop",
            "SpecialAirLwEnd",
        ] {
            assert!(p.animations.clips.contains_key(clip), "Pikachu has no clip {clip}");
        }
    }
    Ok(())
}

#[test]
fn css_portraits_decode_for_every_costume() -> Result<()> {
    let Some(dir) = dir() else { return Ok(()) };
    let grid = portrait::load_grid(&dir, &PIKACHU)?;
    assert_eq!((grid.width, grid.height), (64, 56));
    assert!(grid.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 128 && p[0] > 32));
    let mut previous: Vec<Vec<u8>> = Vec::new();
    for costume in 0..PIKACHU.costumes.len() {
        let photo = portrait::load(&dir, &PIKACHU, costume)?;
        assert_eq!(photo.rgba.len(), photo.width as usize * photo.height as usize * 4);
        assert!(photo.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 128 && p[0] > 32));
        assert!(!previous.contains(&photo.rgba), "costume {costume} reused another photo");
        previous.push(photo.rgba);
    }
    Ok(())
}

#[test]
fn every_script_cue_and_sound_resolves() -> Result<()> {
    let Some(dir) = dir() else { return Ok(()) };
    let animations = Animations::load_for(&dir, &PIKACHU)?;
    let actions = libssbm_assets::action::load_for(&dir, &animations, &PIKACHU)?;
    let bank = Effects::load_archive(&dir, PIKACHU.effects)?;
    let mut mixer = audio::Mixer::new(audio::Bank::load_for(&dir, &PIKACHU)?);
    let mut cues = std::collections::BTreeSet::new();
    let mut sounds = std::collections::BTreeSet::new();
    // Cues the sim itself raises: Skull Bash charge/release, Thunder crackle, cloud.
    cues.extend([1214u32, 1215, 1216, 1219]);
    sounds.insert(240076u32); // Thunder Shock launch
    sounds.insert(240079u32); // Thunder Shock landing
    for (name, action) in &actions {
        if !(name.starts_with("Special") || name.starts_with("Attack")) {
            continue;
        }
        for event in &action.commands {
            if let libssbm::action::Command::Raw { opcode, words } = &event.command {
                if *opcode == 10 {
                    cues.insert(words[1] >> 16);
                }
                if matches!(*opcode, 17 | 54 | 55) {
                    let behavior = if *opcode == 17 { words[0] >> 18 & 255 } else { 0 };
                    if behavior <= 6 || (10..=15).contains(&behavior) {
                        sounds.insert(words[1]);
                    }
                }
            }
        }
    }
    for id in &cues {
        match effect::resolve(*id) {
            Target::Unknown => panic!("effect cue {id} has no dispatch"),
            Target::Particle(p) if p >= 7000 => {
                assert!(bank.emitters.contains_key(&p), "cue {id} -> particle {p} absent from EfPkData")
            }
            _ => (),
        }
    }
    for sound in &sounds {
        mixer.play(*sound, 127, 64, 0).unwrap_or_else(|e| panic!("sound {sound} unplayable: {e}"));
    }
    assert!(!cues.is_empty() && !sounds.is_empty());
    Ok(())
}
