use libssbm::action::Command;
use libssbm_assets::{
    Result, action,
    animation::Animations,
    audio::{Bank, Mixer, RATE},
};
use std::path::PathBuf;
#[test]
fn real_banks_decode_and_action_cues_resolve_to_audible_pcm() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("audio/us/samus.ssm").exists() {
        eprintln!("SKIP: sound banks absent");
        return Ok(());
    }
    let bank = Bank::load(&dir)?;
    assert_eq!(bank.samples.len(), 323);
    assert!(bank.samples.values().all(|s| {
        !s.pcm.is_empty()
            && s.pcm
                .iter()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1.)
    }));
    let actions = action::load(&dir, &Animations::load(&dir)?)?;
    let mut sounds = std::collections::BTreeSet::from([260021]);
    for action in actions.values() {
        for event in &action.commands {
            if let Command::Raw { opcode: 17, words } = &event.command
                && words[0] >> 18 & 255 <= 6
            {
                sounds.insert(words[1]);
            }
        }
    }
    let mut mixer = Mixer::new(bank);
    let mut checked = 0;
    for sound in sounds {
        // Other fighters' copy/voice events can reference unloaded banks; Samus's
        // own bank and shared bank are mandatory, not guessed by sample index.
        if sound / 10000 != 0 && sound / 10000 != 26 {
            continue;
        }
        assert!(
            !mixer.bank.sample_ids(sound)?.is_empty(),
            "empty sequence {sound}"
        );
        mixer.clear();
        mixer.play(sound, 127, 64, 0)?;
        let mut pcm = vec![0.; RATE as usize * 2];
        mixer.render(&mut pcm);
        assert!(
            pcm.iter().any(|x| x.abs() > 0.0001),
            "silent sequence {sound}"
        );
        assert!(mixer.errors.is_empty(), "{sound}: {:?}", mixer.errors);
        checked += 1;
    }
    assert!(checked >= 20, "insufficient disc cue coverage {checked}");
    Ok(())
}
