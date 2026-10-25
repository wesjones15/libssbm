//! Development scratch: common particle emitters that could be a big smoke cloud.
use libssbm_assets::{effects::Effects, fighter::COMMON_EFFECTS};

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("assets"));
    let bank = Effects::load_archive(&dir, COMMON_EFFECTS).unwrap();
    for (id, e) in &bank.emitters {
        if e.size >= 5.0 || e.life >= 15 {
            println!(
                "{id}\ttexture {}\tlife {}\tglife {}\tsize {}\tkind {}\tcommands {}",
                e.texture, e.life, e.generator_life, e.size, e.kind, e.commands.len()
            );
        }
    }
}
