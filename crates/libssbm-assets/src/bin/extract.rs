fn main() -> libssbm_assets::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let disc = args
        .next()
        .ok_or("usage: ssbm-extract <disc.ciso> <assets-folder> [samus|fox|luigi|pikachu]")?;
    let out = args.next().ok_or("missing assets-folder")?;
    let kind = args
        .next()
        .map(|k| k.to_string_lossy().into_owned())
        .unwrap_or_else(|| "samus".into());
    let fighter = match kind.as_str() {
        "samus" | "Samus" => &libssbm_assets::fighter::SAMUS,
        "fox" | "Fox" => &libssbm_assets::fighter::FOX,
        "luigi" | "Luigi" => &libssbm_assets::fighter::LUIGI,
        "pikachu" | "Pikachu" => &libssbm_assets::fighter::PIKACHU,
        other => return Err(format!("unknown fighter {other}").into()),
    };
    libssbm_assets::disc::extract_for(
        std::path::Path::new(&disc),
        std::path::Path::new(&out),
        fighter,
    )?;
    println!("Extracted and validated {kind} assets. Keep this folder out of Git and packages.");
    Ok(())
}
