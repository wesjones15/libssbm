//! Development-only: dump animation clip names for a fighter's AJ archive so a
//! new FighterAssets clip_aliases table can be authored against the real disc.
use std::path::Path;

fn main() -> libssbm_assets::Result<()> {
    let disc = std::env::args().nth(1).expect("usage: dump_clips <disc.iso/ciso>");
    let mut disc = libssbm_assets::disc::Disc::open(Path::new(&disc))?;
    for name in ["PlYsAJ.dat", "PlDkAJ.dat"] {
        let bytes = disc.file(name)?;
        println!("=== {name} ===");
        let anims = libssbm_assets::animation::Animations::parse(&bytes, &[])?;
        let mut names: Vec<_> = anims.clips.keys().collect();
        names.sort();
        for n in names {
            println!("  {n}");
        }
    }
    Ok(())
}
