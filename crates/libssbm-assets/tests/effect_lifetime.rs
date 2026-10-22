//! Effect lifetime rules against the real particle banks (skipped without the player's cache):
//! a projectile's trail lives exactly as long as its projectile, and the model effects the
//! source gives a lifetime of 0 are the ones it ends with `efLib_DestroyAll`.
use libssbm::effect::{companion, lifetime_override, resolve, Target};
use libssbm_assets::{effects::Effects, fighter::*, Result};

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn trail(key: u64, sustain: bool, id: u32, ticks: usize) -> Result<Option<(Option<usize>, bool)>> {
    let dir = dir();
    if !SAMUS.complete(&dir) {
        eprintln!("SKIP: Samus disc cache absent");
        return Ok(None);
    }
    let bank = Effects::load_archive(&dir, SAMUS.effects)?;
    let mut system = libssbm::particle::System::default();
    system.definitions = bank.emitters.clone();
    system.spawn(id, glam::Mat4::IDENTITY, key)?;
    if sustain {
        system.sustain(key);
    }
    let mut ended = None;
    for t in 0..ticks {
        system.tick();
        if !system.running(key) {
            ended = Some(t);
            break;
        }
    }
    let running = system.running(key);
    system.stop(key);
    assert!(!system.running(key), "stop must end the trail and its particles");
    Ok(Some((ended, running)))
}

#[test]
fn a_missile_trail_ends_with_its_generator_unless_the_projectile_sustains_it() -> Result<()> {
    // The disc's smoke trail generators (EfSsData 2011, 2014) stop after about 120 ticks: the
    // missile of the source is gone by then. The host flies it further, so it sustains the trail.
    for id in [2011, 2014] {
        let Some((ended, _)) = trail(1, false, id, 400)? else { return Ok(()) };
        let ended = ended.expect("unsustained trail generator must expire");
        assert!((100..=140).contains(&ended), "generator {id} ended at tick {ended}");
        let (ended, running) = trail(2, true, id, 1500)?.unwrap();
        assert_eq!(ended, None, "generator {id} must keep running while its projectile lives");
        assert!(running);
    }
    Ok(())
}

#[test]
fn a_sustained_trail_stops_the_moment_its_key_is_stopped() -> Result<()> {
    let dir = dir();
    if !SAMUS.complete(&dir) {
        eprintln!("SKIP: Samus disc cache absent");
        return Ok(());
    }
    let bank = Effects::load_archive(&dir, SAMUS.effects)?;
    let mut system = libssbm::particle::System::default();
    system.definitions = bank.emitters.clone();
    system.spawn(2011, glam::Mat4::IDENTITY, 7)?;
    system.spawn(2011, glam::Mat4::IDENTITY, 8)?;
    system.sustain(7);
    for _ in 0..300 {
        system.tick();
    }
    assert!(system.running(7), "sustained trail still runs");
    assert!(!system.running(8), "another projectile's unsustained trail expired on its own");
    system.stop(7);
    assert!(!system.running(7));
    // A later projectile reusing the key starts unsustained again.
    system.spawn(2011, glam::Mat4::IDENTITY, 7)?;
    for _ in 0..300 {
        system.tick();
    }
    assert!(!system.running(7), "stop forgets the sustain");
    Ok(())
}

/// Every model effect the host draws needs an end: the descriptor's lifetime, a lifetime the
/// spawn code sets, or (lifetime 0, joint-bound) the destroy-all of its move. Shell bits and the
/// KO flash are the only unbounded ones that are not joint-bound; the shell bits are overridden.
#[test]
fn every_drawn_model_effect_has_an_end() -> Result<()> {
    let dir = dir();
    if !CAPTAIN.complete(&dir) || !DONKEY.complete(&dir) {
        eprintln!("SKIP: effect archives absent");
        return Ok(());
    }
    let common = Effects::load_archive(&dir, COMMON_EFFECTS)?;
    let mut unbounded = Vec::new();
    for id in 1000..1320u32 {
        for target in [Some(resolve(id)), companion(id)].into_iter().flatten() {
            if let Target::Model(m) = target {
                if m < 2000 {
                    let life = common.models.get(m as usize).map_or(1., |model| model.lifetime);
                    if life == 0. && lifetime_override(id).is_none() {
                        unbounded.push(id);
                    }
                }
            }
        }
    }
    // 1068 is the alternate KO presentation (MeleeDeath owns its end).
    assert_eq!(unbounded, vec![1068], "common model effects with no end: {unbounded:?}");
    assert_eq!(lifetime_override(1231), Some(50));
    assert_eq!(lifetime_override(1232), Some(50));
    Ok(())
}
