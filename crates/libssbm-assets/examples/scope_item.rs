//! Development-only: the script commands of every state of the Super Scope item and its beam.
//! `cargo run --release -p libssbm-assets --example scope_item -- <assets>`
use libssbm_assets::article::{Article, Item};
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("assets"));
    for item in [Item::SuperScope, Item::SuperScopeBeam] {
        for state in 0..item.states() {
            match Article::load_item(&dir, item, state) {
                Ok(a) => {
                    println!("{item:?} state {state}: {} commands", a.commands.len());
                    for c in &a.commands {
                        println!("   @{:>5.1} {:?}", c.frame, c.command);
                    }
                }
                Err(e) => println!("{item:?} state {state}: {e}"),
            }
        }
    }
}
