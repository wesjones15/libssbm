//! Development audit: for every fighter's four specials, print the source
//! effect cues (efSync/efAlt ids) the move emits and what each resolves to.
//! This is the asset shortlist for generated skill portraits. Skips without a
//! disc cache. Usage: `cargo run --release --example skill_cues [fighter...]`.
use glam::Vec3;
use libssbm::{
    Input, Move, Sim,
    behavior::Behavior,
    effect::{resolve, Target},
    sim::{DOWN_B, FlatHost, NEUTRAL_B, SIDE_B, UP_B},
};
use libssbm_assets::{
    Result,
    animation::Animations,
    fighter::{CAPTAIN, DONKEY, FOX, FighterAssets, LINK, LUIGI, PIKACHU, YOSHI, SAMUS},
};

fn build(name: &str, dir: &std::path::Path) -> Result<Option<(Sim, &'static FighterAssets<'static>)>> {
    let (assets, behavior): (&'static FighterAssets<'static>, Box<dyn Behavior>) = match name {
        "samus" => (
            &SAMUS,
            Box::new(libssbm::samus::Samus::new(
                libssbm_assets::params::load_samus_specials(dir)?,
                SAMUS.upper_body_root,
            )),
        ),
        "fox" => (
            &FOX,
            Box::new(libssbm::fox::Fox::new(
                libssbm_assets::params::load_fox_specials(dir)?,
            )),
        ),
        "luigi" => (
            &LUIGI,
            Box::new(libssbm::luigi::Luigi::new(
                libssbm_assets::params::load_luigi_specials(dir)?,
            )),
        ),
        "pikachu" => (
            &PIKACHU,
            Box::new(libssbm::pikachu::Pikachu::new(
                libssbm_assets::params::load_pikachu_specials(dir)?,
            )),
        ),
        "captain" => (
            &CAPTAIN,
            Box::new(libssbm::captain::Captain::new(
                libssbm_assets::params::load_captain_specials(dir)?,
            )),
        ),
        "link" => (
            &LINK,
            Box::new(libssbm::link::Link::new(
                libssbm_assets::params::load_link_specials(dir)?,
            )),
        ),
        "yoshi" => (
            &YOSHI,
            Box::new(libssbm::yoshi::Yoshi::new(
                libssbm_assets::params::load_yoshi_specials(dir)?,
            )),
        ),
        "donkey" => (
            &DONKEY,
            Box::new(libssbm::donkey::Donkey::new(
                libssbm_assets::params::load_donkey_specials(dir)?,
            )),
        ),
        other => {
            println!("unknown fighter {other}");
            return Ok(None);
        }
    };
    if !assets.complete(dir) {
        println!("SKIP {name}: disc cache absent");
        return Ok(None);
    }
    let params = libssbm_assets::params::load_common(dir, assets)?;
    let mut sim = Sim::with_behavior(params, behavior, Vec3::ZERO);
    let (v, g) = (sim.params.jump_speed, sim.params.gravity);
    sim.host_jump_height = Some(v * v / (2. * g));
    let animations = Animations::load_for(dir, assets)?;
    sim.motion.definitions = animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load_for(dir, &animations, assets)?;
    for (n, action) in &sim.action.definitions {
        if let Some(motion) = sim.motion.definitions.get_mut(n) {
            motion.looping |= action.looping;
        }
    }
    Ok(Some((sim, assets)))
}

fn button(m: Move) -> u32 {
    match m {
        Move::SpecialN => NEUTRAL_B,
        Move::SpecialS => SIDE_B,
        Move::SpecialHi => UP_B,
        _ => DOWN_B,
    }
}

fn target(t: Target) -> String {
    match t {
        Target::Particle(id) => format!("particle {id}"),
        Target::Model(id) => format!("model {id}"),
        Target::NoOp => "no-op".into(),
        Target::CameraShake(id) => format!("shake {id}"),
        Target::Unknown => "unknown".into(),
    }
}

fn main() -> Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets".into());
    let dir = std::path::Path::new(&dir);
    let names: Vec<String> = std::env::args().skip(2).collect();
    let names: Vec<&str> = if names.is_empty() {
        vec!["samus", "fox", "luigi", "pikachu", "captain", "link", "yoshi", "donkey"]
    } else {
        names.iter().map(String::as_str).collect()
    };
    for name in names {
        let Some((mut sim, _)) = build(name, dir)? else { continue };
        println!("=== {name} ===");
        for m in [Move::SpecialN, Move::SpecialS, Move::SpecialHi, Move::SpecialLw] {
            let mut host = FlatHost::default();
            sim.grounded = true;
            sim.position = Vec3::ZERO;
            sim.velocity = Vec3::ZERO;
            sim.facing = Vec3::X;
            sim.current_move = None;
            sim.action.effects.clear();
            for _ in 0..4 {
                sim.advance(&mut host, Input::idle(), 1. / 60.);
            }
            let mut input = Input::idle();
            input.aim = Vec3::X;
            input.buttons = button(m);
            sim.advance(&mut host, input, 1. / 60.);
            let mut cues = std::collections::BTreeMap::new();
            for tick in 0..75 {
                let mut next = Input::idle();
                next.aim = Vec3::X;
                if tick < 20 {
                    next.buttons = button(m);
                }
                sim.advance(&mut host, next, 1. / 60.);
                for cue in std::mem::take(&mut sim.action.effects) {
                    cues.insert(cue.id, resolve(cue.id));
                }
                if sim.current_move.is_none() && tick > 2 {
                    break;
                }
            }
            print!("{m:?}:");
            if cues.is_empty() {
                print!(" (no effect cues)");
            }
            for (id, resolved) in cues {
                print!(" 0x{id:X}->{}", target(resolved));
            }
            println!();
        }
    }
    Ok(())
}
