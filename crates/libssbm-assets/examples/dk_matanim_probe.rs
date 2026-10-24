//! Dumps DK's _matanim_joint hierarchy: which joints it has, and each AObj's FObj
//! type (TYPE_JOBJ=12 joint animation vs TYPE_ROBJ=1 material), target and channel.
//! cargo run --release -p libssbm-assets --example dk_matanim_probe
use libssbm_assets::{dat::Dat, fighter::DONKEY, Result};

fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !DONKEY.complete(&dir) {
        println!("SKIP: Donkey disc cache absent");
        return Ok(());
    }
    let bytes = std::fs::read(dir.join(DONKEY.costumes[0]))?;
    let d = Dat::parse(&bytes)?;
    let Some((name, root)) = d.symbols.iter().find(|(s, _)| s.ends_with("_matanim_joint")) else {
        println!("no matanim_joint symbol");
        return Ok(());
    };
    println!("matanim_joint root {name} at {root:#x}");
    // Walk the joint tree: child at +0, next at +4, AObj at +8.
    let mut stack = vec![*root];
    let mut seen = std::collections::HashSet::new();
    let mut joint = 0usize;
    while let Some(at) = stack.pop() {
        if !seen.insert(at) || seen.len() > 2048 {
            continue;
        }
        if let Some(next) = d.ptr(at + 4)? {
            stack.push(next);
        }
        if let Some(child) = d.ptr(at)? {
            stack.push(child);
        }
        if let Some(aobj) = d.ptr(at + 8)? {
            let end = d.f32(aobj + 4)?;
            let target = d.ptr(aobj + 12)?;
            let mut fobj = d.ptr(aobj + 8)?;
            let mut types = Vec::new();
            while let Some(f) = fobj {
                types.push((d.bytes(f + 12, 1)?[0], d.bytes(f + 13, 2)?[0]));
                fobj = d.ptr(f)?;
            }
            println!("  matanim joint {joint} aobj end {end:.1} target {target:?} fobj types {types:?}");
        } else {
            println!("  matanim joint {joint} (no AObj)");
        }
        joint += 1;
    }
    Ok(())
}
