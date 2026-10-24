//! Export a contact sheet of local-disc particle textures (PPM; never commit its output).
use libssbm_assets::{effects::Effects, fighter::COMMON_EFFECTS, Result};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let bank = Effects::load_archive(std::path::Path::new(&args[0]), COMMON_EFFECTS)?;
    let width = 1024usize;
    let height = bank.textures.len().div_ceil(8) * 128;
    let mut pixels = vec![48u8; width * height * 3];
    for (id, group) in bank.textures.iter().enumerate() {
        let Some(group) = group else { continue };
        let tex = &group.frames[0];
        println!("texture {id}: {} frames; emitters {:?}", group.frames.len(), bank.emitters.values().filter(|e| e.texture as usize == id).map(|e| e.id).collect::<Vec<_>>());
        for y in 0..112usize { for x in 0..112usize {
            let src = ((y * tex.height as usize / 112) * tex.width as usize + x * tex.width as usize / 112) * 4;
            let dst = (((id / 8) * 128 + y) * width + (id % 8) * 128 + x) * 3;
            for c in 0..3 { pixels[dst+c] = ((tex.rgba[src+c] as u32 * tex.rgba[src+3] as u32 + 48 * (255-tex.rgba[src+3] as u32))/255) as u8; }
        }}
    }
    use std::io::Write;
    let mut out = std::fs::File::create(&args[1])?;
    write!(out, "P6\n{width} {height}\n255\n")?;
    out.write_all(&pixels)?;
    Ok(())
}
