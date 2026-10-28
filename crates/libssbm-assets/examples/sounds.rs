//! Dump the sound commands an extracted fighter's action scripts carry.
//! Usage: sounds <assets dir> <samus|fox|luigi|pikachu|captain|link|yoshi|donkey> [name filter]
//! The fighter's script sound opcode is 17; its first word is the behavior and the
//! second is the SEM sound id (lb/lbcommand.c, ft/ftaction.c::ftAction_80071B50).
fn main() -> libssbm_assets::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let name = args.get(1).map(String::as_str).unwrap_or("yoshi");
    let filter = args.get(2).map(String::as_str).unwrap_or("");
    let fighter = match name {
        "samus" => &libssbm_assets::fighter::SAMUS,
        "fox" => &libssbm_assets::fighter::FOX,
        "luigi" => &libssbm_assets::fighter::LUIGI,
        "pikachu" | "pika" => &libssbm_assets::fighter::PIKACHU,
        "captain" | "falcon" => &libssbm_assets::fighter::CAPTAIN,
        "link" => &libssbm_assets::fighter::LINK,
        "yoshi" => &libssbm_assets::fighter::YOSHI,
        "donkey" | "dk" => &libssbm_assets::fighter::DONKEY,
        other => return Err(format!("unknown fighter {other}").into()),
    };
    let animations = libssbm_assets::animation::Animations::load_for(dir, fighter)?;
    let actions = libssbm_assets::action::load_for(dir, &animations, fighter)?;
    for (action, def) in &actions {
        if !action.contains(filter) {
            continue;
        }
        for timed in &def.commands {
            if let libssbm::action::Command::Raw { opcode: 17, words } = &timed.command {
                println!("{action} frame {} sound {:#x}", timed.frame, words[1]);
            }
        }
    }
    Ok(())
}
