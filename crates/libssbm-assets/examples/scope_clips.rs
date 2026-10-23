//! Development-only: the Super Scope clips of a fighter (length, looping) and the script commands each carries.
//! `cargo run --release -p libssbm-assets --example scope_clips -- <assets> [kind]`
use libssbm_assets::{action, animation::Animations, fighter};
fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("assets"));
    let kind: usize = args.next().map_or(2, |k| k.parse().unwrap());
    let fighters = [&fighter::SAMUS, &fighter::FOX, &fighter::LUIGI, &fighter::PIKACHU, &fighter::CAPTAIN, &fighter::LINK, &fighter::YOSHI, &fighter::DONKEY];
    let f = fighters[kind];
    let animations = Animations::load_for(&dir, f).unwrap();
    let actions = action::load_for(&dir, &animations, f).unwrap();
    for name in ["ItemScopeStart", "ItemScopeFire", "ItemScopeRapid", "ItemScopeEnd", "ItemScopeAirStart", "ItemScopeAirFire", "ItemScopeAirRapid", "ItemScopeAirEnd"] {
        let clip = animations.clips.get(name);
        println!("{name}: frames {:?} looping {:?}", clip.map(|c| c.frames), clip.map(|c| c.looping));
        if let Some(def) = actions.get(name) {
            for c in &def.commands {
                println!("   @{:>5.1} {:?}", c.frame, c.command);
            }
        }
    }
}
