//! Development audit of Pikachu's extracted disc data: clip names, special
//! attributes (ftPikachuAttributes) and article attributes. Skips without a cache.
use libssbm_assets::{Result, dat::Dat, fighter::PIKACHU};
fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--effects") {
        let a = libssbm_assets::effects::Effects::load_archive(std::path::Path::new("assets"), PIKACHU.effects)?;
        println!("emitter ids {:?}", a.emitters.keys().collect::<Vec<_>>());
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--articles") {
        let dir = std::path::Path::new("assets");
        for kind in [0u32, 2] {
            let a = libssbm_assets::article::Article::load_for(dir, kind, 0, &PIKACHU)?;
            println!("article {kind}: {} joints {} batches clips {:?}", a.model.joints.len(), a.model.batches.len(), a.animations.clips.iter().map(|(k, c)| (k.clone(), c.frames)).collect::<Vec<_>>());
            for (i, j) in a.model.joints.iter().enumerate() {
                println!("  joint {i} parent {:?} t {:?} r {:?} s {:?}", j.parent, j.translation, j.rotation, j.scale);
            }
            for (i, b) in a.model.batches.iter().enumerate() {
                let (mut lo, mut hi) = (glam::Vec3::splat(1e9), glam::Vec3::splat(-1e9));
                for v in &b.vertices { lo = lo.min(v.position); hi = hi.max(v.position); }
                println!("  batch {i} owner {} verts {} tex {} bbox {lo:?}..{hi:?}", b.owner, b.vertices.len(), b.texture.is_some());
            }
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--motion") {
        let dir = std::path::Path::new("assets");
        let anims = libssbm_assets::animation::Animations::load_for(dir, &PIKACHU)?;
        let defs = anims.motion_definitions();
        let model = libssbm_assets::model::Model::load_for(dir, 0, &PIKACHU)?;
        for (name, clip) in &anims.clips {
            if !(name.starts_with("Attack") || name.starts_with("Special") || name.starts_with("Catch") || name.starts_with("Throw") || name.starts_with("Escape")) { continue; }
            let d = &defs[name];
            let travel = d.forward_delta(0., d.frames);
            // Drift of the hips (joint 3/4) relative to frame 0, in model space.
            let a = anims.pose(&model, name, 0., false);
            let b = anims.pose(&model, name, d.frames, false);
            let j = 3.min(a.len() - 1);
            let drift = b[j].w_axis.truncate() - a[j].w_axis.truncate();
            println!("{name:<20} frames {:>5} flagged {:?} travel {:?} joint{j} drift {:.2?}", d.frames, clip.motion_joint, travel, drift);
        }
        return Ok(());
    }
    if std::env::args().len() > 1 { return scripts(); }
    let dir = std::path::Path::new("assets");
    if !PIKACHU.complete(dir) {
        println!("SKIP: Pikachu disc cache absent");
        return Ok(());
    }
    let anims = libssbm_assets::animation::Animations::load_for(dir, &PIKACHU)?;
    println!("{} clips", anims.clips.len());
    for (name, clip) in &anims.clips {
        println!("CLIP {name} {}", clip.frames);
    }
    let bytes = std::fs::read(dir.join("PlPk.dat"))?;
    let d = Dat::parse(&bytes)?;
    println!("symbols {:?}", d.symbols);
    let root = d.root(PIKACHU.data.root)?;
    let attrs = d.required_ptr(root + 4)?;
    for i in 0..0xE0 / 4 {
        println!(
            "attr {:02X}: f32 {:>12} u32 {:#x}",
            i * 4,
            d.f32(attrs + i * 4)?,
            d.u32(attrs + i * 4)?
        );
    }
    for kind in 0..3u32 {
        let layout = PIKACHU.articles[kind as usize];
        let table = d.required_ptr(root + 0x48)?;
        let article = d.required_ptr(table + kind as usize * 4)?;
        let attr = d.required_ptr(article)?;
        let special = d.required_ptr(article + 4)?;
        println!("article {kind} raw common {:?} special {:?}",
            (0..16).map(|i| d.f32(attr + i * 4)).collect::<Result<Vec<_>>>()?,
            (0..layout.special_words.max(4)).map(|i| d.f32(special + i * 4)).collect::<Result<Vec<_>>>()?);
        let Ok(a) = libssbm_assets::article::Article::load_for(dir, kind, 0, &PIKACHU) else {
            println!("  (no loadable model: effect-only article)");
            continue;
        };
        println!("article {kind} states {} common {:?} special {:?}", layout.states, a.common, a.special);
        for state in 0..layout.states {
            let a = libssbm_assets::article::Article::load_for(dir, kind, state, &PIKACHU)?;
            println!("  state {state}: {} commands, clips {:?}", a.commands.len(), a.animations.clips.keys().collect::<Vec<_>>());
            for c in &a.commands {
                println!("    {} {:?}", c.frame, c.command);
            }
        }
    }
    Ok(())
}
// Special-move action scripts (ftData motion state subaction commands).
#[allow(dead_code)]
pub fn scripts() -> Result<()> {
    let dir = std::path::Path::new("assets");
    let anims = libssbm_assets::animation::Animations::load_for(dir, &PIKACHU)?;
    let defs = libssbm_assets::action::load_for(dir, &anims, &PIKACHU)?;
    for name in std::env::args().skip(1) {
        let Some(def) = defs.get(&name) else { println!("NO ACTION {name}"); continue };
        println!("ACTION {name} frames {} looping {}", def.frames, def.looping);
        for c in &def.commands {
            match &c.command {
                libssbm::action::Command::Raw { opcode, words } => println!("  {:>5} raw op{opcode} {:08x?}", c.frame, words),
                other => println!("  {:>5} {:?}", c.frame, other),
            }
        }
    }
    Ok(())
}
