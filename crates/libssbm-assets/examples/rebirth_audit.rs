use libssbm_assets::{Result, dat::Dat};
fn main() -> Result<()> {
    let b = std::fs::read("assets/PlCo.dat")?;
    let d = Dat::parse(&b)?;
    let root = d.root("ftLoadCommonData")?;
    let p = d.required_ptr(root)?;
    for offset in [0x4f4, 0x500, 0x5d0, 0x5d4, 0x5d8] {
        println!(
            "common {offset:x} u32 {} f32 {}",
            d.u32(p + offset)?,
            d.f32(p + offset)?
        );
    }
    let colors = d.required_ptr(root + 0x18)?;
    for id in [9, 10] {
        let at = d.required_ptr(colors + id * 8)?;
        println!(
            "cycle{id}: {:?}",
            (0..24)
                .map(|i| d.u32(at + i * 4).map(|w| format!("{w:08x}")))
                .collect::<Vec<_>>()
        );
        println!(
            "decoded {id}: {:?}",
            libssbm_assets::params::color_cycle(std::path::Path::new("assets"), id)
        );
    }
    let p = libssbm_assets::presentation::Presentation::effect(std::path::Path::new("assets"), 25)?;
    let b = std::fs::read("assets/EfCoData.dat")?;
    let ef = Dat::parse(&b)?;
    for batch in &p.model.batches {
        let joint = &p.model.joints[batch.owner];
        let dobj = ef.required_ptr(joint.address + 16)?;
        let mat = ef.required_ptr(dobj + 8)?;
        let tex = ef.required_ptr(mat + 8)?;
        println!(
            "batch owner {} color {:?} tev {:?}",
            batch.owner,
            batch.color,
            ef.ptr(tex + 0x58)?.map(|at| ef.bytes(at, 32))
        );
    }
    let c1 = d.required_ptr(root + 0x48)?;
    let c2 = d.required_ptr(root + 0x4c)?;
    println!("P1 colours {:?} {:?}", d.bytes(c1, 4)?, d.bytes(c2, 4)?);
    println!(
        "KO model batches {} duration {}",
        p.model.batches.len(),
        p.duration
    );
    Ok(())
}
