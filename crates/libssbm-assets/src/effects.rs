//! EffectDataTable/EF_EffectDesc and embedded HSD particle banks.
//! Layout: ef/types.h, psstructs.h; relocation: particle.c::psInitDataBankLocate.
//! Raw bank offsets are relative to their bank, independently of DAT relocation.
use crate::{
    Result,
    animation::Animations,
    dat::Dat,
    model::{Model, Texture},
};
use glam::Vec3;
use std::{collections::BTreeMap, path::Path};
#[derive(Clone)]
pub struct EffectModel {
    pub lifetime: f32,
    pub model: Model,
    pub animations: Animations,
    pub material_tracks: Vec<crate::animation::Track>,
}
pub use libssbm::particle::Emitter;
pub struct TextureGroup {
    pub frames: Vec<Texture>,
    pub palettes: Vec<Vec<Texture>>,
}
pub struct Effects {
    pub models: Vec<EffectModel>,
    pub emitters: BTreeMap<u32, Emitter>,
    pub textures: Vec<Option<TextureGroup>>,
}
/// efasync.c 0x42B and efLib_SetTevKonstColor: KO colour interpolation.
/// Bake the source's (tev0*(1-texture)+konst*texture) stage into the texture;
/// alpha and source animation remain separate. Offline local fighter uses P1.
pub fn ko_palette(dir: &Path, model: &mut Model, player: usize) -> Result<()> {
    player_palette(dir, model, player, false, 0)
}
/// efasync.c 0x417..0x41A: white highlights and the common player colour.
pub fn shield_palette(
    dir: &Path,
    model: &mut Model,
    player: usize,
    texture_stage: usize,
) -> Result<()> {
    player_palette(dir, model, player, true, texture_stage)
}
fn player_palette(
    dir: &Path,
    model: &mut Model,
    player: usize,
    shield: bool,
    texture_stage: usize,
) -> Result<()> {
    if player >= 4 {
        return Err("KO player palette outside common table".into());
    }
    let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
    let common = Dat::parse(&bytes)?;
    let root = common.root(crate::fighter::COMMON_DATA.root)?;
    let primary = common.bytes(common.required_ptr(root + 0x48)? + player * 4, 3)?;
    let secondary = common.bytes(common.required_ptr(root + 0x4c)? + player * 4, 3)?;
    let bytes = std::fs::read(dir.join(crate::fighter::COMMON_EFFECTS.file))?;
    let d = Dat::parse(&bytes)?;
    for batch in &mut model.batches {
        let Some(dobj) = d.ptr(model.joints[batch.owner].address + 16)? else {
            continue;
        };
        let m = d.required_ptr(dobj + 8)?;
        let mut t = d.required_ptr(m + 8)?;
        for _ in 0..texture_stage {
            t = d.required_ptr(t + 4)?;
        }
        let Some(tev) = d.ptr(t + 0x58)? else {
            continue;
        };
        if d.u32(tev + 28)? & (1 << 30) == 0 {
            continue;
        }
        if d.bytes(tev, 1)?[0] != 0
            || d.bytes(tev + 2, 1)?[0] != 0
            || d.bytes(tev + 4, 1)?[0] != 0
            || d.bytes(tev + 8, 4)? != [133, 128, 8, 15]
        {
            return Err("unsupported KO colour combiner".into());
        }
        let (konst, tev0) = if shield {
            (&[255u8, 255, 255][..], primary)
        } else if batch.owner == 1 {
            (primary, secondary)
        } else {
            (d.bytes(tev + 16, 3)?, d.bytes(tev + 20, 3)?)
        };
        if let Some(texture) = batch.texture.as_mut() {
            for pixel in texture.rgba.chunks_exact_mut(4) {
                for channel in 0..3 {
                    let amount = pixel[channel] as u32;
                    pixel[channel] = ((tev0[channel] as u32 * (255 - amount)
                        + konst[channel] as u32 * amount
                        + 127)
                        / 255) as u8;
                }
            }
        }
    }
    Ok(())
}
impl Effects {
    pub fn shared(dir: &Path, samus: bool) -> Result<std::sync::Arc<Self>> {
        Self::shared_archive(
            dir,
            if samus {
                crate::fighter::SAMUS.effects
            } else {
                crate::fighter::COMMON_EFFECTS
            },
        )
    }
    pub fn shared_archive(
        dir: &Path,
        archive: crate::fighter::Archive<'_>,
    ) -> Result<std::sync::Arc<Self>> {
        use std::{
            path::PathBuf,
            sync::{Arc, Mutex, OnceLock},
        };
        type Cache = Mutex<BTreeMap<(PathBuf, String, String), Arc<Effects>>>;
        static CACHE: OnceLock<Cache> = OnceLock::new();
        let key = (
            dir.canonicalize()?,
            archive.file.into(),
            archive.root.into(),
        );
        let mut cache = CACHE
            .get_or_init(|| Mutex::new(BTreeMap::new()))
            .lock()
            .map_err(|_| "effect cache poisoned")?;
        if let Some(bank) = cache.get(&key) {
            return Ok(bank.clone());
        }
        let bank = Arc::new(Self::load_archive(dir, archive)?);
        if cache.len() >= 8
            && let Some(key) = cache.keys().next().cloned()
        {
            cache.remove(&key);
        }
        cache.insert(key, bank.clone());
        Ok(bank)
    }
    pub fn load(dir: &Path, samus: bool) -> Result<Self> {
        Self::load_archive(
            dir,
            if samus {
                crate::fighter::SAMUS.effects
            } else {
                crate::fighter::COMMON_EFFECTS
            },
        )
    }
    pub fn load_archive(dir: &Path, archive: crate::fighter::Archive<'_>) -> Result<Self> {
        Self::parse(
            &Dat::parse(&std::fs::read(dir.join(archive.file))?)?,
            archive.root,
        )
    }
    pub fn parse(d: &Dat, symbol: &str) -> Result<Self> {
        let root = d.root(symbol)?;
        let cmd = d.required_ptr(root)?;
        let tex = d.required_ptr(root + 4)?;
        let mut models = Vec::new();
        // Descriptor extent is implied by relocated joint pointers; do not read
        // alignment padding as another descriptor or bake in a disc asset count.
        for i in 0..1000 {
            let desc = root + 8 + i * 20;
            if !d.relocations.contains(&(desc + 4)) {
                break;
            }
            let model = Model::from_joint(d, d.required_ptr(desc + 4)?)?;
            let animations = Animations::object(d, d.ptr(desc + 8)?, &model)?;
            let material_tracks = Animations::materials(d, d.ptr(desc + 12)?, &model)?;
            models.push(EffectModel {
                lifetime: d.f32(desc)?,
                model,
                animations,
                material_tracks,
            });
        }
        let texture_count = d.u32(tex)? as usize;
        if texture_count > 256 {
            return Err("particle texture group budget exceeded".into());
        }
        let mut textures = Vec::new();
        for i in 0..texture_count {
            let offset = d.u32(tex + 4 + i * 4)? as usize;
            if offset == 0 {
                textures.push(None);
                continue;
            }
            let g = tex
                .checked_add(offset)
                .ok_or("particle texture offset overflow")?;
            let count = d.u32(g)? as usize;
            let format = d.u32(g + 4)?;
            let palette_format = d.u32(g + 8)?;
            let width = u16::try_from(d.u32(g + 12)?)?;
            let height = u16::try_from(d.u32(g + 16)?)?;
            let palnum = d.u16(g + 20)? as usize;
            let flags = d.u16(g + 22)?;
            if count > 1024 || palnum > 256 {
                return Err("particle texture frame/palette budget exceeded".into());
            }
            let palette_count = if matches!(format, 8..=10) {
                if flags & 1 != 0 {
                    1
                } else if palnum > 0 {
                    palnum
                } else {
                    count
                }
            } else {
                0
            };
            let mut palettes = Vec::new();
            let mut frames = Vec::new();
            for pal in 0..palette_count.max(1) {
                let mut images = Vec::new();
                let palette = if palette_count > 0 {
                    let entries = match format {
                        8 => 16,
                        9 => 256,
                        _ => 16384,
                    };
                    let offset = d.u32(g + 24 + (count + pal) * 4)? as usize;
                    Some((
                        d.bytes(
                            tex.checked_add(offset).ok_or("particle palette overflow")?,
                            entries * 2,
                        )?,
                        palette_format,
                    ))
                } else {
                    None
                };
                for frame in 0..count {
                    let offset = d.u32(g + 24 + frame * 4)? as usize;
                    let rgba = crate::texture::decode(
                        d.data,
                        tex.checked_add(offset).ok_or("particle image overflow")?,
                        width,
                        height,
                        format,
                        palette,
                    )?;
                    images.push(Texture {
                        wrap_s: 1,
                        wrap_t: 1,
                        width,
                        height,
                        rgba,
                    });
                }
                if pal == 0 {
                    frames = images.clone();
                }
                palettes.push(images);
            }
            textures.push(Some(TextureGroup { frames, palettes }));
        }
        let version = d.u16(cmd)?;
        let (base, count, table) = match version {
            0 => (0, d.u32(cmd + 4)?, cmd + 8),
            0x40..=0x43 => (d.u32(cmd + 4)?, d.u32(cmd + 8)?, cmd + 12),
            _ => return Err("unsupported particle bank version".into()),
        };
        if count > 4096 {
            return Err("particle emitter budget exceeded".into());
        }
        let offsets = (0..count as usize)
            .map(|i| d.u32(table + i * 4).map(|n| n as usize))
            .collect::<Result<Vec<_>>>()?;
        let mut emitters = BTreeMap::new();
        for (i, &offset) in offsets.iter().enumerate() {
            if offset == 0 {
                continue;
            }
            let at = cmd.checked_add(offset).ok_or("particle emitter overflow")?;
            let end = offsets
                .iter()
                .filter(|&&n| n > offset)
                .min()
                .map(|n| cmd + n)
                .unwrap_or(tex);
            let script_at = at + 60;
            if end < script_at || end - script_at > 65536 {
                return Err("particle command extent invalid".into());
            }
            let vector =
                |at| -> Result<Vec3> { Ok(Vec3::new(d.f32(at)?, d.f32(at + 4)?, d.f32(at + 8)?)) };
            let texture = d.u16(at + 2)?;
            if texture as usize >= textures.len() || textures[texture as usize].is_none() {
                return Err("particle emitter texture missing".into());
            }
            let id = base + i as u32;
            emitters.insert(
                id,
                Emitter {
                    id,
                    shape: d.u16(at)?,
                    texture,
                    generator_life: d.u16(at + 4)?,
                    life: d.u16(at + 6)?,
                    kind: d.u32(at + 8)?,
                    gravity: d.f32(at + 12)?,
                    friction: d.f32(at + 16)?,
                    velocity: vector(at + 20)?,
                    radius: d.f32(at + 32)?,
                    angle: d.f32(at + 36)?,
                    random: d.f32(at + 40)?,
                    size: d.f32(at + 44)?,
                    params: vector(at + 48)?,
                    commands: d.bytes(script_at, end - script_at)?.to_vec(),
                },
            );
        }
        Ok(Self {
            models,
            emitters,
            textures,
        })
    }
}
