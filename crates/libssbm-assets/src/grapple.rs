//! Source grapple accessory geometry and tuning (itSamusGrappleAttributes,
//! it_802B75FC / ftSs_Init_CreateThrowGrappleBeam). RoR2 capture is a host policy.
use crate::{Result, animation::Animations, dat::Dat, model::Model, presentation::Presentation};
use std::path::Path;
pub struct Tuning {
    pub links: usize,
    pub length: f32,
    pub speed: f32,
    pub gravity: f32,
}
fn attributes(d: &Dat) -> Result<usize> {
    let items = d.required_ptr(d.root("ftDataSamus")? + 0x48)?;
    d.required_ptr(d.required_ptr(items + 12)? + 4)
}
pub fn tuning(dir: &Path) -> Result<Tuning> {
    let bytes = std::fs::read(dir.join("PlSs.dat"))?;
    let d = Dat::parse(&bytes)?;
    let a = attributes(&d)?;
    let coeff = d.f32(a + 0x5c)?;
    let result = Tuning {
        links: ((d.u32(a + 12)? as f32 * coeff) as usize).clamp(2, 64),
        length: d.f32(a + 0x10)?,
        speed: d.f32(a + 0x18)? * coeff,
        gravity: d.f32(a + 0x1c)?,
    };
    if result.length <= 0. || result.speed <= 0. {
        return Err("invalid grapple tuning".into());
    }
    Ok(result)
}
pub fn part(dir: &Path, part: u32) -> Result<Presentation> {
    if part > 7 {
        return Err("invalid grapple part".into());
    }
    let bytes = std::fs::read(dir.join("PlSs.dat"))?;
    let d = Dat::parse(&bytes)?;
    let a = attributes(&d)?;
    let beam = if part >= 4 {
        let items = d.required_ptr(d.root("ftDataSamus")? + 0x48)?;
        Some(d.required_ptr(items + 16)?)
    } else {
        None
    };
    let model = Model::from_joint(
        &d,
        if let Some(beam) = beam {
            d.required_ptr(beam)?
        } else {
            d.required_ptr(a + 0x64 + part as usize * 4)?
        },
    )?;
    let mut animations = Animations::object(
        &d,
        beam.map(|b| d.ptr(b + 8)).transpose()?.flatten(),
        &model,
    )?;
    let mut material_tracks = vec![];
    if let Some(beam) = beam {
        let table = d.required_ptr(beam + 4)?;
        let extra = Animations::object(&d, d.ptr(table + (part - 4) as usize * 4)?, &model)?;
        let base = animations.clips.get_mut("Object").unwrap();
        let extra = &extra.clips["Object"];
        base.frames = base.frames.max(extra.frames);
        base.tracks.extend(extra.tracks.clone());
        material_tracks = Animations::materials(&d, d.ptr(beam + 12)?, &model)?;
    }
    let world = model.bind_world();
    Ok(Presentation {
        model,
        animations,
        clip: "Object".into(),
        frame: 0.,
        world,
        ball: false,
        hidden: false,
        material_tracks,
        duration: 0.,
        commands: vec![],
        object_scale: 1.,
    })
}
