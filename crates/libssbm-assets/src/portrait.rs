//! Original CSS photos. mnCharSel_8025D1C4/8025D5AC selects the menu's
//! costume material at frame ICONHUD + costume*30. HSD_TObjUpdateFunc samples
//! TIMG/TCLT channels to choose the ImageDesc/TlutDesc; no model is rendered.
use crate::{
    Result,
    animation::{Track, keys},
    dat::Dat,
    model::Texture,
};
use std::{collections::HashSet, path::Path};
#[derive(Clone, Copy, Debug)]
pub struct Portrait<'a> {
    pub archive: crate::fighter::Archive<'a>,
    pub joint: usize,
    pub frame: u32,
}
fn sample(d: &Dat, aobj: usize, channel: u8, frame: f32) -> Result<Option<f32>> {
    let mut fobj = d.ptr(aobj + 8)?;
    let mut seen = HashSet::new();
    while let Some(f) = fobj {
        if !seen.insert(f) || seen.len() > 64 {
            return Err("cyclic portrait tracks".into());
        }
        if d.bytes(f + 12, 1)?[0] == channel {
            let frac = d.bytes(f + 13, 2)?;
            return Ok(Track {
                joint: 0,
                channel,
                keys: keys(
                    d.bytes(d.required_ptr(f + 16)?, d.u32(f + 4)? as usize)?,
                    frac[0],
                    frac[1],
                    d.f32(f + 8)?,
                )?,
            }
            .sample(frame));
        }
        fobj = d.ptr(f)?;
    }
    Ok(None)
}
pub fn load(
    dir: &Path,
    fighter: &crate::fighter::FighterAssets<'_>,
    costume: usize,
) -> Result<Texture> {
    fighter.required()?;
    if costume >= fighter.costumes.len() {
        return Err("invalid portrait costume".into());
    }
    let p = fighter.portrait;
    let bytes = std::fs::read(dir.join(p.archive.file))?;
    let d = Dat::parse(&bytes)?;
    let root = d.root(p.archive.root)?;
    // MnSelectChrDataTable.models.menu.matanim_joint (0x10+0x30+8).
    let mut stack = vec![d.required_ptr(root + 0x48)?];
    let mut seen = HashSet::new();
    let mut index = 0;
    while let Some(at) = stack.pop() {
        if !seen.insert(at) || seen.len() > 2048 {
            return Err("cyclic portrait material tree".into());
        }
        if index == p.joint {
            let material = d.required_ptr(at + 8)?;
            let tex = d.required_ptr(material + 8)?;
            let aobj = d.required_ptr(tex + 8)?;
            let frame = (p.frame + costume as u32 * 30) as f32;
            let image_index =
                sample(&d, aobj, 1, frame)?.ok_or("portrait TIMG channel missing")? as usize;
            if image_index >= d.u16(tex + 20)? as usize {
                return Err("portrait image index outside table".into());
            }
            let image = d.required_ptr(d.required_ptr(tex + 12)? + image_index * 4)?;
            let palette = if let Some(table) = d.ptr(tex + 16)? {
                let i =
                    sample(&d, aobj, 10, frame)?.ok_or("portrait TCLT channel missing")? as usize;
                if i >= d.u16(tex + 22)? as usize {
                    return Err("portrait palette index outside table".into());
                }
                let pal = d.required_ptr(table + i * 4)?;
                Some((
                    d.bytes(d.required_ptr(pal)?, d.u16(pal + 12)? as usize * 2)?,
                    d.u32(pal + 4)?,
                ))
            } else {
                None
            };
            let width = d.u16(image + 4)?;
            let height = d.u16(image + 6)?;
            return Ok(Texture {
                width,
                height,
                rgba: crate::texture::decode(
                    d.data,
                    d.required_ptr(image)?,
                    width,
                    height,
                    d.u32(image + 8)?,
                    palette,
                )?,
            });
        }
        if let Some(next) = d.ptr(at + 4)? {
            stack.push(next);
        }
        if let Some(child) = d.ptr(at)? {
            stack.push(child);
        }
        index += 1;
    }
    Err("portrait joint missing".into())
}
