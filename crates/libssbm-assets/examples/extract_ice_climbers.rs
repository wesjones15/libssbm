//! `cargo run -p libssbm-assets --example extract_ice_climbers -- <Melee Rev 2 image> <cache folder>`:
//! the Ice Climbers' models, for Olimar's Pikachu skin in RoR2.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: extract_ice_climbers <Melee .iso/.ciso> <out folder>");
        std::process::exit(2);
    }
    match libssbm_assets::disc::extract_ice_climbers(std::path::Path::new(&a[1]), std::path::Path::new(&a[2])) {
        Ok(()) => println!("ok"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
