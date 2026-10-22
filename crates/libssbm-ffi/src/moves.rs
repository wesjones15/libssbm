//! The move list of a fighter's sim, for host UIs such as a pause-menu moveset.
//!
//! The sim already knows which moves a fighter has: a common move exists when the fighter's
//! disc action table defines the source action that `motion::select` plays for it (Attack11,
//! AttackS3S, AttackAirF, Catch, ThrowF, ...), and a special exists when the fighter's
//! `Behavior` plays a clip for it. This export reports each move as the shared `Move` id and
//! the name of that source action, so a host never keeps a hand-written per-fighter list.
//! Human names and input bindings belong to the host (ror2-common `MeleeMoveset`).
use super::Handle;
use libssbm::Move;

/// One move of a fighter: the shared `Move` id and the NUL-padded source action name.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MoveEntry {
    pub id: u32,
    pub name: [u8; 28],
}

/// The common moves, in the order a pause sheet lists them, with the source action each plays
/// (the same names as `motion::select`; ftCo_Attack*, ftCo_Catch*, ftCo_Throw*).
const COMMON: [(Move, &str); 25] = [
    (Move::Jab, "Attack11"),
    (Move::Jab2, "Attack12"),
    (Move::Jab3, "Attack13"),
    (Move::RapidJab, "Attack100Start"),
    (Move::SideTilt, "AttackS3S"),
    (Move::UpTilt, "AttackHi3"),
    (Move::DownTilt, "AttackLw3"),
    (Move::DashAttack, "AttackDash"),
    (Move::SideSmash, "AttackS4S"),
    (Move::UpSmash, "AttackHi4"),
    (Move::DownSmash, "AttackLw4"),
    (Move::NeutralAir, "AttackAirN"),
    (Move::ForwardAir, "AttackAirF"),
    (Move::BackAir, "AttackAirB"),
    (Move::UpAir, "AttackAirHi"),
    (Move::DownAir, "AttackAirLw"),
    (Move::Grab, "Catch"),
    (Move::DashGrab, "CatchDash"),
    (Move::AirGrab, "AirCatch"),
    (Move::Pummel, "CatchAttack"),
    (Move::ThrowForward, "ThrowF"),
    (Move::ThrowBack, "ThrowB"),
    (Move::ThrowUp, "ThrowHi"),
    (Move::ThrowDown, "ThrowLw"),
    (Move::GrabHold, "CatchWait"),
];

/// The special-move slots, and the source action each is named for when the fighter has no
/// action of that name (staged specials play several: SpecialSStart, SpecialS, SpecialSEnd).
const SPECIALS: [(Move, &str); 5] = [
    (Move::SpecialN, "SpecialN"),
    (Move::SpecialS, "SpecialS"),
    (Move::SpecialS2, "SpecialS2"),
    (Move::SpecialHi, "SpecialHi"),
    (Move::SpecialLw, "SpecialLw"),
];

fn entry(id: Move, name: &str) -> MoveEntry {
    let mut out = MoveEntry { id: id as u32, name: [0; 28] };
    let bytes = name.as_bytes();
    let n = bytes.len().min(27);
    out.name[..n].copy_from_slice(&bytes[..n]);
    out
}

/// The moves a sim's fighter has, in sheet order (jabs, tilts, smashes, aerials, grabs, then
/// specials). Pure over the sim, so headless tests can check every fighter.
pub(crate) fn moves_of(sim: &libssbm::Sim) -> Vec<MoveEntry> {
    let defined = |name: &str| sim.action.definitions.contains_key(name);
    let mut list = Vec::new();
    for (m, name) in COMMON {
        // Throws, grabs and the rapid jab come from the fighter's own action table. The
        // GrabHold wait is not a move a player chooses, so it is left out of the list.
        if m != Move::GrabHold && defined(name) {
            list.push(entry(m, name));
        }
    }
    for (m, base) in SPECIALS {
        let clip = sim.behavior.clip(sim, m);
        if !clip.is_empty() {
            list.push(entry(m, if defined(base) { base } else { clip }));
        } else if defined(base) {
            list.push(entry(m, base));
        }
    }
    list
}

/// Writes the fighter's move list to `out` (up to `max` entries) and returns how many moves it
/// has; call with null to query the count. Each entry is a `Move` id and its source action name.
/// # Safety
/// A live sim handle (or null, which has no moves) and a writable `max`-element output or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_moves(handle: *const Handle, out: *mut MoveEntry, max: u32) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    if h.poisoned {
        return 0;
    }
    let list = moves_of(&h.sim);
    unsafe { super::copy(&list, out, max) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pad, V3, ssbm_sim_free, ssbm_sim_new_fighter};

    fn name(e: &MoveEntry) -> String {
        let n = e.name.iter().position(|&b| b == 0).unwrap_or(e.name.len());
        String::from_utf8_lossy(&e.name[..n]).into_owned()
    }

    #[test]
    fn entry_is_32_bytes() {
        assert_eq!(size_of::<MoveEntry>(), 32);
        let _ = Pad { movement: V3::default(), aim: V3::default(), buttons: 0, allowed: 0 };
    }

    #[test]
    fn every_fighter_reports_its_moves() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for kind in 0u32..8 {
            let fk = libssbm_assets::fighter::FighterKind::from_u32(kind).unwrap();
            if !fk.assets().unwrap().complete(&dir) {
                eprintln!("SKIP: fighter {kind} disc cache absent");
                continue;
            }
            let cdir = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
            let sim = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
            assert!(!sim.is_null(), "kind {kind} sim failed");
            let count = unsafe { ssbm_sim_moves(sim, std::ptr::null_mut(), 0) };
            let mut list = vec![MoveEntry { id: 0, name: [0; 28] }; count as usize];
            assert_eq!(unsafe { ssbm_sim_moves(sim, list.as_mut_ptr(), count) }, count);
            let line: Vec<String> = list.iter().map(|e| format!("{}={}", e.id, name(e))).collect();
            eprintln!("fighter {kind}: {} moves: {}", count, line.join(" "));
            let has = |m: Move| list.iter().any(|e| e.id == m as u32);
            for m in [
                Move::Jab,
                Move::SideTilt,
                Move::UpTilt,
                Move::DownTilt,
                Move::DashAttack,
                Move::SideSmash,
                Move::UpSmash,
                Move::DownSmash,
                Move::NeutralAir,
                Move::ForwardAir,
                Move::BackAir,
                Move::UpAir,
                Move::DownAir,
                Move::SpecialN,
                Move::SpecialHi,
                Move::SpecialLw,
            ] {
                assert!(has(m), "fighter {kind} lacks {m:?}");
            }
            unsafe { ssbm_sim_free(sim) };
        }
    }
}
