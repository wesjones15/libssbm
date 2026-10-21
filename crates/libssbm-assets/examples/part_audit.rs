//! Dumps each fighter's part-animation table (ftData + 0x1C, ftData_x1C in ft/types.h):
//! slots, the bone each starts at, how many animations and which bones they key.
use libssbm_assets::{dat::Dat, fighter::*, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let common_bytes = std::fs::read(dir.join(COMMON_DATA.file))?;
    let common = Dat::parse(&common_bytes)?;
    let croot = common.root(COMMON_DATA.root)?;
    let kinds = common.required_ptr(croot + 0x14)?;
    for (name, f) in [("fox", &FOX), ("luigi", &LUIGI), ("pikachu", &PIKACHU), ("captain", &CAPTAIN), ("link", &LINK), ("yoshi", &YOSHI), ("donkey", &DONKEY), ("samus", &SAMUS)] {
        let bytes = std::fs::read(dir.join(f.data.file))?;
        let d = Dat::parse(&bytes)?;
        let root = d.root(f.data.root)?;
        let table = match d.ptr(root + 0x1c) {
            Ok(Some(t)) => t,
            other => {
                println!("{name}: no part table ({other:?}); root {root:#x} word {:#x}", d.u32(root + 0x1c)?);
                continue;
            }
        };
        println!("{name}: part table at {table:#x}");
        match common.ptr(kinds + f.common_index * 4) {
            Ok(Some(e)) => {
                let list = common.ptr(e)?;
                let count = common.u32(e + 4)? as usize;
                let parts: Vec<u8> = match list {
                    Some(l) if count < 64 => (0..count).map(|i| common.bytes(l + i * 4, 1).map(|b| b[0])).collect::<std::result::Result<_, _>>()?,
                    _ => Vec::new(),
                };
                println!("  special bones (PlCo x14[{}]): {count} {parts:?}", f.common_index);
            }
            other => println!("  special bones: none ({other:?})"),
        }
        for slot in 0..5 {
            let Ok(Some(entry)) = d.ptr(table + slot * 4) else { println!("  slot {slot}: -"); continue };
            let start = d.u16(entry)?;
            let count = d.u16(entry + 2)?;
            let list: Vec<u8> = match d.ptr(entry + 4).unwrap_or(None) {
                Some(l) => d.bytes(l, count as usize)?.to_vec(),
                None => Vec::new(),
            };
            let anims = d.ptr(entry + 8).unwrap_or_else(|e| { println!("   (anims: {e})"); None });
            print!("  slot {slot}: start bone {start}, {count} listed {list:?}");
            let Some(anims) = anims else {
                println!(", no animations");
                continue;
            };
            println!();
            for i in 0..16 {
                let Ok(p) = d.ptr(anims + i * 4) else { break };
                let Some(tree) = p else {
                    println!("    anim {i}: none");
                    continue;
                };
                let walk = || -> Result<(usize, f32, Vec<(usize, Vec<u8>)>)> {
                // Preorder walk (child, then next), counting keyed bones.
                let mut stack = vec![tree];
                let mut index = 0usize;
                let mut keyed = Vec::new();
                let mut end = 0f32;
                while let Some(at) = stack.pop() {
                    if index > 512 {
                        break;
                    }
                    if let Some(n) = d.ptr(at + 4)? {
                        stack.push(n);
                    }
                    if let Some(c) = d.ptr(at)? {
                        stack.push(c);
                    }
                    if let Some(a) = d.ptr(at + 8)? {
                        end = end.max(d.f32(a + 4)?);
                        let mut channels = Vec::new();
                        let mut fobj = d.ptr(a + 8)?;
                        while let Some(fo) = fobj {
                            channels.push(d.bytes(fo + 12, 1)?[0]);
                            fobj = d.ptr(fo)?;
                        }
                        keyed.push((start as usize + index, channels));
                    }
                    index += 1;
                }
                Ok((index, end, keyed))
                };
                match walk() {
                    Ok((index, end, keyed)) => println!("    anim {i}: {index} bones, {end} frames, keyed {keyed:?}"),
                    Err(e) => {
                        println!("    anim {i}: end of array ({e})");
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
