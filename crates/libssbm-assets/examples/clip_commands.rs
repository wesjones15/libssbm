//! Prints a clip's script (frame, command) for a fighter: `clip_commands captain SpecialLw`.
use libssbm_assets::{animation::Animations, fighter::*, Result};
fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let f = match args[0].as_str() {
        "samus" => &SAMUS, "fox" => &FOX, "luigi" => &LUIGI, "pikachu" => &PIKACHU,
        "captain" => &CAPTAIN, "link" => &LINK, "yoshi" => &YOSHI, "donkey" => &DONKEY, o => panic!("{o}"),
    };
    let animations = Animations::load_for(&dir, f)?;
    let actions = libssbm_assets::action::load_for(&dir, &animations, f)?;
    for clip in &args[1..] {
        let a = actions.get(clip.as_str()).ok_or("no clip")?;
        println!("{clip}: {} frames", a.frames);
        for c in &a.commands { println!("  f{:>5} {:?}", c.frame, c.command); }
    }
    Ok(())
}
