//! Development audit of Pikachu's extracted disc data: clip names, special
//! attributes (ftPikachuAttributes) and article attributes. Skips without a cache.
use libssbm_assets::{Result, dat::Dat, fighter::PIKACHU};
fn main() -> Result<()> {
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
