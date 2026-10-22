//! Lists sound/effect commands in every fighter article's item script: articles whose own
//! script plays a sound (Pikachu's Thunder bolt, the missile...) need the host to play it.
use libssbm::action::Command;
use libssbm_assets::{article::Article, fighter::*, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (name, f) in [("samus", &SAMUS), ("fox", &FOX), ("luigi", &LUIGI), ("pikachu", &PIKACHU), ("captain", &CAPTAIN), ("link", &LINK), ("yoshi", &YOSHI), ("donkey", &DONKEY)] {
        if !f.complete(&dir) { continue; }
        for (kind, layout) in f.articles.iter().enumerate() {
            for state in 0..layout.states {
                let a = match Article::load_for(&dir, kind as u32, state, f) { Ok(a) => a, Err(e) => { println!("{name} article {kind}/{state}: {e}"); continue; } };
                let mut items = Vec::new();
                for c in &a.commands {
                    if let Command::Raw { opcode, words } = &c.command {
                        items.push(format!("f{}:op{}:{:x?}", c.frame, opcode, words));
                    }
                }
                if !items.is_empty() { println!("{name} article {kind}/{state}: {}", items.join(" ")); }
            }
        }
    }
    Ok(())
}
