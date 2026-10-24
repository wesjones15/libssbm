//! The voices the fighter data's sound table names (ftData +0x4C): each fighter's smash grunt
//! (script command 18), hurt voices (ftCo_Damage.c) and blast-zone KO voice (ft_0D31.c) load,
//! decode to audible PCM and play at the right moments (skipped without the player's cache).
use glam::Vec3;
use libssbm::{
    sim::{FlatHost, VOICE_CHANNEL},
    Input, Move, Sim,
};
use libssbm_assets::{
    animation::Animations,
    audio::{Bank, Mixer, RATE},
    fighter::{FighterAssets, CAPTAIN, DONKEY, FOX, LINK, LUIGI, PIKACHU, SAMUS, YOSHI},
    Result,
};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

const ALL: [&FighterAssets<'static>; 8] = [&SAMUS, &FOX, &LUIGI, &PIKACHU, &CAPTAIN, &LINK, &YOSHI, &DONKEY];

fn fighter(f: &FighterAssets<'_>) -> Result<Option<(Sim, FlatHost)>> {
    let dir = dir();
    if !f.complete(&dir) {
        eprintln!("SKIP: disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(&dir, f)?;
    let mut sim = Sim::new(params, Vec3::ZERO);
    let animations = Animations::load_for(&dir, f)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(&dir, &animations, f)?;
    for (name, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(name) {
            motion.looping |= action.looping;
        }
    }
    sim.grounded = true;
    sim.facing = Vec3::X;
    let mut host = FlatHost::default();
    for _ in 0..4 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    Ok(Some((sim, host)))
}

/// The voice-slot sounds the sim queued over `ticks` steps.
fn voices(sim: &mut Sim, host: &mut FlatHost, ticks: u32) -> Vec<u32> {
    let mut heard = Vec::new();
    for _ in 0..ticks {
        sim.advance(host, Input::idle(), 1. / 60.);
        heard.extend(sim.action.sounds.iter().filter(|c| c.channel == VOICE_CHANNEL).map(|c| c.sound));
    }
    heard
}

#[test]
fn every_fighter_loads_and_decodes_its_voice_table() -> Result<()> {
    let dir = dir();
    for f in ALL {
        if !f.complete(&dir) {
            eprintln!("SKIP: disc cache absent");
            return Ok(());
        }
        let v = libssbm_assets::params::load_common(&dir, f)?.voices;
        assert!(!v.smash.is_empty(), "{} has no smash voices", f.data.file);
        assert!(!v.ko.is_empty(), "{} has no KO voice", f.data.file);
        // Samus's hurt cries are not in this table (both arrays are empty).
        assert!(f.data.file == "PlSs.dat" || !v.hurt_heavy.is_empty(), "{} has no heavy hurt voice", f.data.file);
        let mut mixer = Mixer::new(Bank::load_for(&dir, f)?);
        for &id in v.smash.iter().chain(&v.ko).chain(&v.hurt_light).chain(&v.hurt_heavy) {
            assert!(!mixer.bank.sample_ids(id)?.is_empty(), "{}: voice {id} has no samples", f.data.file);
            mixer.clear();
            mixer.play(id, 127, 64, 0)?;
            let mut pcm = vec![0.; RATE as usize * 2];
            mixer.render(&mut pcm);
            assert!(pcm.iter().any(|x| x.abs() > 0.0001), "{}: voice {id} is silent", f.data.file);
        }
    }
    Ok(())
}

#[test]
fn a_smash_attack_grunts_once_with_one_of_the_fighters_smash_voices() -> Result<()> {
    for f in ALL {
        let Some((mut sim, mut host)) = fighter(f)? else { return Ok(()) };
        let mut voiced = 0;
        for (m, clip) in [(Move::SideSmash, "AttackS4S"), (Move::UpSmash, "AttackHi4"), (Move::DownSmash, "AttackLw4")] {
            // The grunt is script command 18 or a behavior-2 sound command (ft_800881D8, the same
            // voice slot); a smash clip without either (Falcon's forward smash calls its own
            // voice) stays silent here. Command 18 picks from `voices.smash`; a behavior-2 voice
            // is a specific sound the clip names, so it may sit outside that list.
            let commands = sim.action.definitions[clip]
                .commands
                .iter()
                .filter(|e| match &e.command {
                    libssbm::action::Command::Raw { opcode: 18, .. } => true,
                    libssbm::action::Command::Raw { opcode: 17, words } => (words[0] >> 18 & 255) == 2,
                    _ => false,
                })
                .count();
            sim.interrupt();
            assert!(sim.start_move(&mut host, m, Input::idle()));
            let heard = voices(&mut sim, &mut host, 120);
            assert_eq!(heard.len(), commands, "{} {clip}: smash voices heard {heard:?}", f.data.file);
            voiced += heard.len();
        }
        assert!(voiced >= 2, "{}: only {voiced} smash voices over three smashes", f.data.file);
    }
    Ok(())
}

#[test]
fn only_a_big_enough_hit_cries_out_and_a_bigger_one_cries_harder() -> Result<()> {
    for f in ALL {
        let Some((mut sim, mut host)) = fighter(f)? else { return Ok(()) };
        let (light, heavy) = (sim.params.voices.hurt_light.clone(), sim.params.voices.hurt_heavy.clone());
        sim.hurt_scaled(0.01);
        assert!(voices(&mut sim, &mut host, 3).is_empty(), "{}: a scratch made a sound", f.data.file);
        let Some((mut sim, mut host)) = fighter(f)? else { return Ok(()) };
        sim.hurt_scaled(0.1);
        let heard = voices(&mut sim, &mut host, 3);
        if light.is_empty() {
            assert!(heard.is_empty(), "{}: {heard:?}", f.data.file);
        } else {
            assert!(heard.len() == 1 && light.contains(&heard[0]), "{}: {heard:?} vs {light:?}", f.data.file);
        }
        let Some((mut sim, mut host)) = fighter(f)? else { return Ok(()) };
        sim.hurt_scaled(0.5);
        let heard = voices(&mut sim, &mut host, 3);
        if heavy.is_empty() {
            assert!(heard.is_empty(), "{}: {heard:?}", f.data.file);
        } else {
            assert!(heard.len() == 1 && heavy.contains(&heard[0]), "{}: {heard:?} vs {heavy:?}", f.data.file);
        }
    }
    Ok(())
}

#[test]
fn the_blast_zone_ko_plays_the_ko_voice() -> Result<()> {
    let dir = dir();
    for f in ALL {
        let Some((mut sim, mut host)) = fighter(f)? else { return Ok(()) };
        let ko = sim.params.voices.ko.clone();
        assert!(sim.begin_rebirth(
            libssbm_assets::params::rebirth(&dir)?,
            libssbm_assets::params::entry(&dir, f)?,
            Vec3::ZERO
        ));
        let heard = voices(&mut sim, &mut host, 10);
        assert_eq!(heard, ko, "{}", f.data.file);
    }
    Ok(())
}
