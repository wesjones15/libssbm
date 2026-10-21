//! Which PlCo colour programs use the light commands (13 to 17), with their colours and
//! directions: the rim light the colour animations set besides the overlay.
use libssbm_assets::{params::load_colanim, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let table = load_colanim(&dir)?;
    let word = |at: usize| u32::from_be_bytes(table.data[at..at + 4].try_into().unwrap());
    for (id, entry) in table.entries.iter().enumerate() {
        let Some(start) = entry.program else { continue };
        let mut pc = start;
        let mut notes = Vec::new();
        let mut colour = false;
        for _ in 0..400 {
            let w = word(pc);
            let op = w >> 26;
            match op {
                0 | 10 => break,
                5 | 7 | 8 => {
                    // goto / call / wait-forever: stop the linear scan here.
                    if op != 5 {
                        break;
                    }
                    pc += 8;
                    continue;
                }
                13 => {
                    let enable = w >> 25 & 1;
                    let x = ((w >> 12 & 0xfff) as i32) << 20 >> 20;
                    let yz = ((w & 0xfff) as i32) << 20 >> 20;
                    notes.push(format!("light on (enable {enable}, x {x}, yz {yz}) colour {:02x?}", word(pc + 4).to_be_bytes()));
                    pc += 8;
                    continue;
                }
                14 => {
                    notes.push(format!("light colour {:02x?}", word(pc + 4).to_be_bytes()));
                    pc += 8;
                    continue;
                }
                15 => {
                    notes.push(format!("light blend {} frames to {:02x?}", w & 0x3ff_ffff, word(pc + 4).to_be_bytes()));
                    pc += 8;
                    continue;
                }
                16 => {
                    let x = ((w >> 13 & 0x1fff) as i32) << 19 >> 19;
                    let yz = ((w & 0x1fff) as i32) << 19 >> 19;
                    notes.push(format!("light rotate x {x} yz {yz}"));
                }
                17 => notes.push("light off".into()),
                18 => {
                    colour = true;
                    pc += 8;
                    continue;
                }
                19 | 11 | 12 | 20 | 1..=4 | 6 | 9 | 23 => {
                    if op == 19 {
                        pc += 8;
                        continue;
                    }
                }
                21 => {
                    pc += 20;
                    continue;
                }
                22 => {
                    let behavior = w >> 18 & 0xff;
                    pc += 4 * if behavior <= 6 || (10..=15).contains(&behavior) { 3 } else { 2 };
                    continue;
                }
                _ => {}
            }
            pc += 4;
        }
        if !notes.is_empty() {
            println!("colanim {id} (priority {}, channel {}, colour overlay {colour}):", entry.priority, entry.channel);
            for n in notes.iter().take(8) {
                println!("    {n}");
            }
        }
    }
    Ok(())
}
