//! VR arms proof (libssbm_assets::vr_arms over libcommon's cut): each Melee fighter costume's
//! forearms and hands, cut from its own body by FtPart_RArmJ/RHandN and LArmJ/LHandN through
//! the common bone table. One picture per costume (libcommon's `vr_arms::capture`): the body
//! from the front and its side in the bind pose (right arm red, left blue), then each arm alone
//! in its arm space (+z toward the fingers, right; +y up).
//!   cargo run --release -p ssbm-viewer --example vr_arms -- <melee cache> <out dir>

use std::path::PathBuf;

use libcommon::vr_arms;
use libssbm_assets::vr_arms as hsd;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().unwrap_or_else(|| "assets".into()));
    let out = PathBuf::from(args.next().unwrap_or_else(|| "vr_arms".into()));
    std::fs::create_dir_all(&out).unwrap();
    for kind in hsd::kinds() {
        let fighter = kind.assets().unwrap();
        for costume in 0..fighter.costumes.len() {
            if !dir.join(fighter.costumes[costume]).is_file() {
                println!("SKIP {} {costume}: not extracted", kind.name());
                continue;
            }
            match hsd::cut_fighter(&dir, kind, costume) {
                Ok((mesh, arms)) => {
                    let label = format!("MELEE {} {} {}", kind.name(), costume, fighter.costumes[costume]);
                    let (w, h, px) = vr_arms::capture(&mesh, &arms, &label);
                    let path = out.join(format!("{}.png", arms.name));
                    std::fs::write(&path, vr_arms::png(w, h, &px)).unwrap();
                    let tris: Vec<usize> = arms.arms.iter().map(|a| a.triangles.len()).collect();
                    println!("{}: {} arms {tris:?} of {} -> {}", arms.name, arms.arms.len(), mesh.triangles.len(), path.display());
                }
                Err(e) => println!("FAIL {} {costume}: {e}", kind.name()),
            }
        }
    }
}
