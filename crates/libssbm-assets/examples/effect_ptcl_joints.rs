//! Development-only: particle joints (JOBJ_PTCL, baselib/jobj.h) inside the effect models of an archive. HSD_JObjDisp calls
//! efLib_Cb_SPtcl(bank, generator, jobj) for each entry of a joint's list once its 0x80000000 bit is set, so a model can
//! carry particle generators the mesh alone does not show.
//! `cargo run --release -p libssbm-assets --example effect_ptcl_joints -- <assets> [archive.dat root]`
use libssbm_assets::{dat::Dat, effects::Effects, fighter::COMMON_EFFECTS};
fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("assets"));
    let file = args.next().unwrap_or_else(|| COMMON_EFFECTS.file.to_string());
    let root = args.next().unwrap_or_else(|| COMMON_EFFECTS.root.to_string());
    let bytes = std::fs::read(dir.join(&file)).unwrap();
    let d = Dat::parse(&bytes).unwrap();
    let fx = Effects::parse(&d, &root).unwrap();
    for (i, m) in fx.models.iter().enumerate() {
        // Joint animation tracks 0x28 (dynamic particle: bank = value & 0x3F, generator = value >> 6), 0x29 (sound), 0x2A.
        for (name, clip) in &m.animations.clips {
            for t in clip.tracks.iter().filter(|t| t.channel >= 0x14) {
                println!("model {i} clip {name} joint {} track {:#x}: {:?}", t.joint, t.channel, t.keys.iter().map(|k| (k.time, format!("bank {} gen {:#x} raw {:#x}", k.value.to_bits() & 0x3f, k.value.to_bits() >> 6 & 0xff_ffff, k.value.to_bits()))).collect::<Vec<_>>());
            }
        }
        for (j, joint) in m.model.joints.iter().enumerate() {
            if joint.flags & (1 << 5) == 0 {
                continue;
            }
            let mut entries = Vec::new();
            let mut at = d.ptr(joint.address + 0x10).ok().flatten();
            while let Some(node) = at {
                let data = d.u32(node).unwrap();
                entries.push(format!("{}{}:{:#x}", if data & 0x8000_0000 != 0 { "armed " } else { "" }, data & 0x3f, data >> 6 & 0xff_ffff));
                at = d.ptr(node + 4).ok().flatten();
            }
            println!("model {i} lifetime {} joint {j}: ptcl [bank:generator] {entries:?}", m.lifetime);
        }
    }
}
