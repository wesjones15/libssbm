//! VR arms for HSD bodies (libcommon's `vr_arms`): every Melee fighter and costume, and the
//! Melee bodies other cores borrow as skins (Mario's Luigi, Olimar's Captain Pikachu).
//!
//! The arm is the common bone table's forearm and hand: FtPart_RArmJ / FtPart_RHandN (38 / 39)
//! and FtPart_LArmJ / FtPart_LHandN (20 / 21) (ft/forward.h), resolved per fighter through
//! ftCommonData's parts table ([`crate::held::common_bones`]), so one rule covers the whole
//! roster. The mesh is laid out as the native view draws it (libssbm-ffi `View::new`): every
//! batch's vertices in order, each batch's triangles offset by its first vertex.

use std::path::Path;

use libcommon::vr_arms::{self, Arms, Mesh, Side};

use crate::fighter::FighterKind;
use crate::model::Model;
use crate::presentation::Presentation;

/// FtPart indices (ft/forward.h) of each side's forearm and hand.
pub const R_ARM_J: usize = 38;
pub const R_HAND_N: usize = 39;
pub const L_ARM_J: usize = 20;
pub const L_HAND_N: usize = 21;

/// `model` in its bind pose as the view lays it out. A batch `shown` rejects keeps its
/// vertices and triangles (the order must match) but has no weights, so none of it is cut.
pub fn mesh(model: &Model, shown: impl Fn(usize) -> bool) -> Mesh {
    let bind = model.bind_world();
    let mut m = Mesh {
        parents: model.joints.iter().map(|j| j.parent).collect(),
        bind: bind.iter().map(|b| b.to_cols_array()).collect(),
        ..Mesh::default()
    };
    for (i, b) in model.batches.iter().enumerate() {
        let first = m.positions.len() as u32;
        let on = shown(i);
        for ((p, _), v) in model.skin(i, &bind).into_iter().zip(&b.vertices) {
            m.positions.push(p.into());
            m.weights.push(if on { v.weights.iter().map(|&(j, w)| (j as u16, w)).collect() } else { Vec::new() });
        }
        for t in b.indices.chunks_exact(3) {
            m.triangles.push([first + t[0], first + t[1], first + t[2]]);
        }
    }
    m
}

/// Each side's forearm and hand joints from a fighter's common bone table (right, left).
pub fn sides(bones: &[u8]) -> [Option<Side>; 2] {
    let side = |arm: usize, hand: usize| match (bones.get(arm), bones.get(hand)) {
        (Some(&a), Some(&h)) if a != 0xFF && h != 0xFF && a != h => Some(Side { elbow: a as usize, wrist: h as usize }),
        _ => None,
    };
    [side(R_ARM_J, R_HAND_N), side(L_ARM_J, L_HAND_N)]
}

/// The cache name of a fighter costume's cut.
pub fn name(kind: FighterKind, costume: usize) -> String {
    format!("melee-{}-{}", kind.name().to_ascii_lowercase(), costume)
}

/// A fighter costume's body (as the native view draws it, its default parts showing) and its cut.
pub fn cut_fighter(dir: &Path, kind: FighterKind, costume: usize) -> crate::Result<(Mesh, Arms)> {
    let fighter = kind.assets().ok_or("unknown fighter")?;
    let p = Presentation::load_for(dir, costume, fighter)?;
    let m = mesh(&p.model, |i| p.visible(i));
    let bones = crate::held::common_bones(dir, fighter)?;
    let arms = vr_arms::cut(&name(kind, costume), &m, sides(&bones));
    Ok((m, arms))
}

/// Every fighter kind, in ABI order.
pub fn kinds() -> impl Iterator<Item = FighterKind> {
    (0..).map_while(FighterKind::from_u32)
}

/// First run: cuts every fighter costume in the Melee cache `dir` that has no current cut yet
/// into `dir/vr_arms/` (a fighter missing from the cache is skipped). One note per cut written
/// or failed; Ok with none when every one was already there.
pub fn bake(dir: &Path) -> crate::Result<Vec<String>> {
    let mut notes = Vec::new();
    for kind in kinds() {
        let Some(fighter) = kind.assets() else { continue };
        if fighter.required().map_or(true, |files| files.iter().any(|f| !dir.join(f).is_file())) {
            continue;
        }
        for costume in 0..fighter.costumes.len() {
            let name = name(kind, costume);
            if Arms::load(dir, &name).is_some() {
                continue;
            }
            match cut_fighter(dir, kind, costume) {
                Ok((_, arms)) => {
                    arms.save(dir)?;
                    let tris: Vec<usize> = arms.arms.iter().map(|a| a.triangles.len()).collect();
                    notes.push(format!("{name}: {} arms ({tris:?} triangles)", arms.arms.len()));
                }
                Err(e) => notes.push(format!("{name}: {e}")),
            }
        }
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every fighter's cut keeps two arms of a plausible size, the hand ahead of the forearm.
    #[test]
    fn every_fighter_has_both_arms() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !dir.join(crate::fighter::COMMON_DATA.file).is_file() {
            eprintln!("SKIP: no Melee cache in {}", dir.display());
            return;
        }
        for kind in kinds() {
            let fighter = kind.assets().unwrap();
            if fighter.required().map_or(true, |files| files.iter().any(|f| !dir.join(f).is_file())) {
                eprintln!("SKIP: {} not extracted", kind.name());
                continue;
            }
            let (m, arms) = cut_fighter(&dir, kind, 0).unwrap();
            assert_eq!(arms.arms.len(), 2, "{}", kind.name());
            for arm in &arms.arms {
                let n = arm.triangles.len();
                assert!(n > 10 && n < m.triangles.len() / 3, "{} side {}: {n} of {}", kind.name(), arm.side, m.triangles.len());
            }
        }
    }
}
