//! Development-only: lifetime (frames) and size of every model in a fighter's effect bank.
//! `cargo run --release -p libssbm-assets --example effect_lifetimes -- <assets> [fighter index 0..7]`
use libssbm_assets::{effects::Effects, fighter};
fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("assets"));
    let kind: usize = args.next().map_or(0, |k| k.parse().unwrap());
    let f = [&fighter::SAMUS, &fighter::FOX, &fighter::LUIGI, &fighter::PIKACHU, &fighter::CAPTAIN, &fighter::LINK, &fighter::YOSHI, &fighter::DONKEY][kind];
    let fx = Effects::load_archive(&dir, f.effects).unwrap();
    for (i, m) in fx.models.iter().enumerate() {
        let (mut lo, mut hi) = (glam::Vec3::splat(f32::INFINITY), glam::Vec3::splat(f32::NEG_INFINITY));
        for b in &m.model.batches {
            for v in &b.vertices {
                lo = lo.min(v.position);
                hi = hi.max(v.position);
            }
        }
        println!("model {} (cue id {}): lifetime {} frames, extent {:?} .. {:?}", i, 2000 + i, m.lifetime, lo, hi);
    }
}
