//! Development audit: starts every generic and special move for the first-pass
//! fighters against the player's extracted disc cache and prints what the sim
//! actually does with it (clip chain, length, hitboxes, travel, events). Skips
//! without a cache. Usage: `cargo run --release --example move_audit [fighter...]`
//! with fighter in pikachu, captain, link, yoshi, donkey (fox and luigi for comparison).
use glam::Vec3;
use libssbm::{
    Input, Move, Sim,
    action::HitSample,
    behavior::Behavior,
    sim::{DOWN_B, Event, EventKind, FlatHost, NEUTRAL_B, SIDE_B, UP_B},
};
use libssbm_assets::{
    Result,
    animation::Animations,
    fighter::{CAPTAIN, DONKEY, FOX, FighterAssets, LINK, LUIGI, PIKACHU, YOSHI},
};
use std::collections::BTreeMap;

const GROUND: [Move; 22] = [
    Move::Jab,
    Move::Jab2,
    Move::SideTilt,
    Move::UpTilt,
    Move::DownTilt,
    Move::DashAttack,
    Move::SideSmash,
    Move::UpSmash,
    Move::DownSmash,
    Move::Grab,
    Move::DashGrab,
    Move::Pummel,
    Move::ThrowForward,
    Move::ThrowBack,
    Move::ThrowUp,
    Move::ThrowDown,
    Move::SpecialN,
    Move::SpecialS,
    Move::SpecialHi,
    Move::SpecialLw,
    Move::GrabHold,
    Move::AirGrab,
];
const AIR: [Move; 9] = [
    Move::NeutralAir,
    Move::ForwardAir,
    Move::BackAir,
    Move::UpAir,
    Move::DownAir,
    Move::SpecialN,
    Move::SpecialS,
    Move::SpecialHi,
    Move::SpecialLw,
];

fn build(
    name: &str,
    dir: &std::path::Path,
) -> Result<Option<(Sim, &'static FighterAssets<'static>)>> {
    let (assets, behavior): (&'static FighterAssets<'static>, Box<dyn Behavior>) = match name {
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
    // The RoR2 host supplies its own jump apex; stand in the disc's full hop so
    // rising recoveries (Falcon Dive, Spin Attack, Egg Throw...) climb as in game.
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

fn special_button(m: Move) -> u32 {
    match m {
        Move::SpecialN => NEUTRAL_B,
        Move::SpecialS | Move::SpecialS2 => SIDE_B,
        Move::SpecialHi => UP_B,
        _ => DOWN_B,
    }
}

struct Report {
    clips: Vec<(String, u32)>,
    ticks: u32,
    ended: bool,
    boxes: BTreeMap<(String, u32), (f32, u32, u32, u32, u32)>,
    events: Vec<String>,
    travel: Vec3,
    peak: f32,
    sounds: usize,
    effects: usize,
    generations: std::collections::BTreeSet<(u32, u32)>,
}

fn run(sim: &mut Sim, m: Move, air: bool, hold: u32, max: u32) -> Report {
    let mut host = FlatHost { events: Vec::new() };
    sim.grounded = !air;
    sim.position = if air {
        Vec3::new(0., 200., 0.)
    } else {
        Vec3::ZERO
    };
    sim.velocity = Vec3::ZERO;
    sim.facing = Vec3::X;
    sim.current_move = None;
    for _ in 0..4 {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
    }
    host.events.clear();
    sim.action.samples.clear();
    sim.action.sounds.clear();
    sim.action.effects.clear();
    let start = sim.position;
    let special = matches!(
        m,
        Move::SpecialN | Move::SpecialS | Move::SpecialHi | Move::SpecialLw
    );
    let mut input = Input::idle();
    input.aim = Vec3::X;
    let mut report = Report {
        clips: Vec::new(),
        ticks: 0,
        ended: false,
        boxes: BTreeMap::new(),
        events: Vec::new(),
        travel: Vec3::ZERO,
        peak: 0.,
        sounds: 0,
        effects: 0,
        generations: Default::default(),
    };
    if special {
        input.buttons = special_button(m);
        sim.advance(&mut host, input, 1. / 60.);
    } else if !sim.start_move(&mut host, m, input) {
        report.events.push("start_move refused".into());
        return report;
    }
    let mut last = String::new();
    for tick in 0..max {
        let mut next = Input::idle();
        next.aim = Vec3::X;
        if special && tick < hold {
            next.buttons = special_button(m) | libssbm::sim::SPECIAL;
        }
        sim.advance(&mut host, next, 1. / 60.);
        report.ticks = tick + 1;
        let clip = sim.motion.clip.to_string();
        if clip != last {
            report.clips.push((clip.clone(), tick));
            last = clip;
        }
        for HitSample {
            hitbox,
            clip,
            generation,
            ..
        } in std::mem::take(&mut sim.action.samples)
        {
            report.generations.insert((hitbox.group, generation));
            let e = report
                .boxes
                .entry((clip.to_string(), hitbox.id))
                .or_insert((0., 0, 0, 0, 0));
            if hitbox.damage > e.0 {
                *e = (
                    hitbox.damage,
                    hitbox.angle,
                    hitbox.base_knockback,
                    hitbox.growth,
                    hitbox.element,
                );
            }
        }
        report.sounds += std::mem::take(&mut sim.action.sounds).len();
        report.effects += std::mem::take(&mut sim.action.effects).len();
        report.peak = report.peak.max(sim.position.y);
        if sim.current_move.is_none() && tick > 2 {
            report.ended = true;
            break;
        }
    }
    for Event {
        kind,
        move_id,
        strength,
        ..
    } in host.events.drain(..)
    {
        if !matches!(kind, EventKind::Started) {
            report
                .events
                .push(format!("{kind:?}(move {move_id}, {strength:.2})"));
        }
    }
    report.travel = sim.position - start;
    report
}

/// `move_audit script <fighter> <clip>...` prints a clip's authored commands
/// (clip names are the sim's canonical names, after `clip_aliases`).
fn scripts(dir: &std::path::Path, args: &[String]) -> Result<()> {
    let Some((sim, _)) = build(&args[0], dir)? else {
        return Ok(());
    };
    for name in &args[1..] {
        let Some(def) = sim.action.definitions.get(name.as_str()) else {
            println!("NO ACTION {name}");
            continue;
        };
        println!(
            "ACTION {name} frames {} looping {}",
            def.frames, def.looping
        );
        for c in &def.commands {
            match &c.command {
                libssbm::action::Command::Raw { opcode, words } => {
                    println!("  {:>5} raw op{opcode} {:08x?}", c.frame, words)
                }
                libssbm::action::Command::Hitbox(h) => println!(
                    "  {:>5} hitbox #{} {}% r{} ang{} bkb{} kbg{} el{}",
                    c.frame,
                    h.id,
                    h.damage,
                    h.radius,
                    h.angle,
                    h.base_knockback,
                    h.growth,
                    h.element
                ),
                other => println!("  {:>5} {:?}", c.frame, other),
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("script") {
        return scripts(&dir, &args[1..]);
    }
    if args.first().map(String::as_str) == Some("articles") {
        let Some((_, assets)) = build(&args[1], &dir)? else { return Ok(()) };
        for kind in 0..assets.articles.len() as u32 {
            let layout = assets.articles[kind as usize];
            for state in 0..layout.states {
                match libssbm_assets::article::Article::load_for(&dir, kind, state, assets) {
                    Ok(a) => {
                        println!("article {kind} state {state}: {} joints {} batches clips {:?} common {:?} special {:?}", a.model.joints.len(), a.model.batches.len(), a.animations.clips.keys().collect::<Vec<_>>(), a.common, a.special);
                        for c in &a.commands {
                            println!("    {} {:?}", c.frame, c.command);
                        }
                    }
                    Err(e) => println!("article {kind} state {state}: no model ({e})"),
                }
            }
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("motion") {
        let Some((sim, _)) = build(&args[1], &dir)? else { return Ok(()) };
        for (name, d) in &sim.motion.definitions {
            if name.starts_with("Special") || name.starts_with("Catch") {
                println!("{name:<22} frames {:>5} looping {} travel {:?}", d.frames, d.looping, d.forward_delta(0., d.frames));
            }
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("specials") {
        use libssbm_assets::params::*;
        println!("{:?}", load_fox_specials(&dir)?);
        println!("{:?}", load_pikachu_specials(&dir)?);
        println!("{:?}", load_captain_specials(&dir)?);
        println!("{:?}", load_link_specials(&dir)?);
        println!("{:?}", load_yoshi_specials(&dir)?);
        println!("{:?}", load_donkey_specials(&dir)?);
        return Ok(());
    }
    let mut names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        names = ["pikachu", "captain", "link", "yoshi", "donkey"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
    for name in names {
        let Some((_, assets)) = build(&name, &dir)? else {
            continue;
        };
        println!("######## {name} ({})", assets.data.file);
        for (air, moves) in [(false, &GROUND[..]), (true, &AIR[..])] {
            for &m in moves {
                if air && m == Move::AirGrab {
                    continue;
                }
                let special = matches!(
                    m,
                    Move::SpecialN | Move::SpecialS | Move::SpecialHi | Move::SpecialLw
                );
                let holds: &[u32] = if special { &[0, 50, 150] } else { &[0] };
                for &hold in holds {
                    // A fresh sim per run so a held phase never leaks into the next move.
                    let Some((mut sim, _)) = build(&name, &dir)? else {
                        continue;
                    };
                    let r = run(&mut sim, m, air, hold, 420);
                    let clips: Vec<String> =
                        r.clips.iter().map(|(c, t)| format!("{c}@{t}")).collect();
                    let boxes: Vec<String> = r
                        .boxes
                        .iter()
                        .map(|((c, id), (d, a, b, g, el))| {
                            format!("{c}#{id}:{d:.0}% ang{a} bkb{b} kbg{g} el{el}")
                        })
                        .collect();
                    println!(
                        "{}{:?}{} | clips [{}] | {} ticks{} | travel x{:.1} y{:.1} peak {:.1} | hit boxes [{}] | events [{}] | sounds {} effects {} | hit windows {}",
                        if air { "AIR " } else { "GND " },
                        m,
                        if hold > 0 { " (held)" } else { "" },
                        clips.join(" "),
                        r.ticks,
                        if r.ended { "" } else { " NOT-ENDED" },
                        r.travel.x,
                        r.travel.y,
                        r.peak,
                        boxes.join("; "),
                        r.events.join(", "),
                        r.sounds,
                        r.effects,
                        r.generations.len()
                    );
                }
            }
        }
    }
    Ok(())
}
