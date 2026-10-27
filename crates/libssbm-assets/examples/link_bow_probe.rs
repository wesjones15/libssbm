//! Diagnose source skinning matrices for Link's exact slot-4 bow.
use libssbm_assets::{article::Article, dat::Dat, fighter::LINK, held, Result};
use glam::{Mat4, Vec3, Vec4};
fn main() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !LINK.complete(&dir) { println!("SKIP: Link disc cache absent"); return Ok(()); }
    let bytes = std::fs::read(dir.join("PlLk.dat"))?;
    let d = Dat::parse(&bytes)?;
    let a = Article::load_for(&dir, held::LINK_BOW, 1, &LINK)?;
    for (i, j) in a.model.joints.iter().enumerate() {
        if let Some(at) = d.ptr(j.address + 0x38)? {
            let mut m = [0.; 12];
            for (k,v) in m.iter_mut().enumerate() { *v = d.f32(at+k*4)?; }
            let source = Mat4::from_cols(Vec4::new(m[0],m[4],m[8],0.), Vec4::new(m[1],m[5],m[9],0.), Vec4::new(m[2],m[6],m[10],0.), Vec4::new(m[3],m[7],m[11],1.));
            println!("joint {i}: inverse difference {:.3}, source inverse {:?}", (source-j.inverse).to_cols_array().iter().map(|v|v.abs()).fold(0f32,f32::max),source);
        }
    }
    for (i,b) in a.model.batches.iter().enumerate() {
        println!("batch {i}: owner {}, vertex weights {:?}, vertex bounds {:?}", b.owner, b.vertices[0].weights,
            b.vertices.iter().fold((Vec3::splat(f32::INFINITY),Vec3::splat(f32::NEG_INFINITY)), |(lo,hi),v|(lo.min(v.position),hi.max(v.position))));
    }
    let clip = &a.animations.clips["Object"];
    for t in &clip.tracks { println!("track joint {} channel {} f0 {:?} f8 {:?}", t.joint,t.channel,t.sample(0.),t.sample(8.)); }
    Ok(())
}
