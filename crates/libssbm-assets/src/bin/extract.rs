fn main() -> libssbm_assets::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let disc = args
        .next()
        .ok_or("usage: ssbm-extract <disc.ciso> <assets-folder>")?;
    let out = args.next().ok_or("missing assets-folder")?;
    libssbm_assets::disc::extract(std::path::Path::new(&disc), std::path::Path::new(&out))?;
    println!("Extracted and validated Samus assets. Keep this folder out of Git and packages.");
    Ok(())
}
