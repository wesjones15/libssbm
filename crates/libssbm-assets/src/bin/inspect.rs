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
    for costume in 0..5 {
        let m = libssbm_assets::model::Model::load(dir, costume)?;
        println!(
            "Costume {costume}: {} joints, {} batches, {} triangles, {} textures",
            m.joints.len(),
            m.batches.len(),
            m.batches.iter().map(|b| b.indices.len() / 3).sum::<usize>(),
            m.batches.iter().filter(|b| b.texture.is_some()).count()
        );
    }
    let a = libssbm_assets::animation::Animations::load(dir)?;
    println!(
        "{} animation clips; Wait: {:?}",
        a.clips.len(),
        a.clips.get("Wait").map(|c| (c.frames, c.tracks.len()))
    );
    Ok(())
}
