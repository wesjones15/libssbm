//! Development-only: the ItemScope motion scripts decoded with no clip-length horizon, plus their raw words.
//! `cargo run --release -p libssbm-assets --example scope_script -- <assets> [kind]`
use libssbm_assets::{action::script_commands, dat::Dat, fighter};
fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("assets"));
    let kind: usize = args.next().map_or(2, |k| k.parse().unwrap());
    let fighters = [&fighter::SAMUS, &fighter::FOX, &fighter::LUIGI, &fighter::PIKACHU, &fighter::CAPTAIN, &fighter::LINK, &fighter::YOSHI, &fighter::DONKEY];
    let f = fighters[kind];
    let bytes = std::fs::read(dir.join(f.data.file)).unwrap();
    let d = Dat::parse(&bytes).unwrap();
    let root = d.root(f.data.root).unwrap();
    let start = d.required_ptr(root + 0xc).unwrap();
    let end = d.required_ptr(root + 0x14).unwrap();
    for at in (start..end).step_by(24) {
        let Some(name_at) = d.ptr(at).unwrap() else { continue };
        let tail = &d.data[name_at..];
        let len = tail.iter().position(|&b| b == 0).unwrap();
        let name = std::str::from_utf8(&tail[..len]).unwrap();
        if !name.contains("ItemScope") {
            continue;
        }
        let flags = d.u32(at + 16).unwrap();
        println!("{name} flags {flags:#x} script {:?}", d.ptr(at + 12).unwrap());
        let Some(script) = d.ptr(at + 12).unwrap() else { continue };
        // Raw words until the terminator, whatever the clip length.
        let words: Vec<String> = (0..70).map(|i| format!("{:08x}", d.u32(script + i * 4).unwrap_or(0))).collect();
        println!("   raw {}", words.join(" "));
        match script_commands(&d, script, 100000.) {
            Ok(c) => {
                for c in c {
                    println!("   @{:>6.1} {:?}", c.frame, c.command);
                }
            }
            Err(e) => println!("   decode error: {e}"),
        }
    }
}
