//! Development scratch: particle-generator joints (JOBJ_PTCL) in fighter models.
use libssbm_assets::{fighter, model::Model};

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("assets"));
    for (name, f) in [("Samus", &fighter::SAMUS), ("Luigi", &fighter::LUIGI)] {
        match Model::load_for(&dir, 0, f) {
            Ok(m) => {
                let ptcl: Vec<_> = m
                    .joints
                    .iter()
                    .enumerate()
                    .filter(|(_, j)| j.flags & (1 << 5) != 0)
                    .map(|(i, j)| (i, j.flags, j.address))
                    .collect();
                println!("{name}: {} joints, PTCL {:?}", m.joints.len(), ptcl);
            }
            Err(e) => println!("{name}: {e}"),
        }
    }
}
