use libssbm_assets::disc::Disc;
use std::path::Path;

fn main() -> libssbm_assets::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: audio_sample_locator <disc> <sample-id>")?;
    let sample: u32 = std::env::args()
        .nth(2)
        .ok_or("missing sample id")?
        .parse()?;
    let mut disc = Disc::open(Path::new(&path))?;
    let paths: Vec<_> = disc
        .files
        .keys()
        .filter(|p| p.starts_with("audio/us/") && p.ends_with(".ssm"))
        .cloned()
        .collect();
    for path in paths {
        let bytes = disc.file(&path)?;
        if bytes.len() < 16 {
            continue;
        }
        let count = u32::from_be_bytes(bytes[8..12].try_into()?);
        let base = u32::from_be_bytes(bytes[12..16].try_into()?);
        if base <= sample && sample < base + count {
            println!("{path}: samples {base}..{}", base + count - 1);
        }
    }
    Ok(())
}
