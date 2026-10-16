fn main() -> libssbm_assets::Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets".into());
    let dir = std::path::Path::new(&dir);
    for name in libssbm_assets::REQUIRED {
        let bytes = std::fs::read(dir.join(name))?;
        if name == "PlSsAJ.dat" {
            println!("{name}: {} bytes (packed animation archives)", bytes.len());
            continue;
        }
        let dat = libssbm_assets::dat::Dat::parse(&bytes)?;
        println!(
            "{name}: {} bytes, {} relocations, publics {:?}",
            bytes.len(),
            dat.relocations.len(),
            dat.symbols
        );
    }
    println!("Disc tuning: {:#?}", libssbm_assets::params::load(dir)?);
    Ok(())
}
