//! HSD_Joint/DObjDesc/PObjDesc/GX display lists from baselib headers.
//! Diffuse materials and bind-space envelope skinning; TEV is approximated.
use crate::{Result, dat::Dat};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
#[derive(Clone, Debug)]
pub struct Joint {
    pub flags: u32,
    pub address: usize,
    pub parent: Option<usize>,
    pub translation: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub bind: Mat4,
    pub inverse: Mat4,
}
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    /// GX CLR0, the source's per-vertex colour (the flat model part of an untextured
    /// material; the ghost and projectile models rely on it).
    pub color: [f32; 4],
    pub weights: Vec<(usize, f32)>,
}
/// GX colour attribute formats (GXEnum.h GXColorFormat), not texture formats.
/// RGB565, RGB888, RGB8, RGBA4, RGBA6, RGBA8.
fn vertex_color(format: u32, b: &[u8]) -> Result<[f32; 4]> {
    let r = |v: u32, max: u32| v as f32 / max as f32;
    Ok(match format {
        0 => {
            let v = u16::from_be_bytes(b.try_into()?) as u32;
            [r(v >> 11, 31), r((v >> 5) & 63, 63), r(v & 31, 31), 1.]
        }
        1 | 2 => [b[0] as f32 / 255., b[1] as f32 / 255., b[2] as f32 / 255., 1.],
        3 => {
            let v = u16::from_be_bytes(b.try_into()?) as u32;
            [r(v >> 12, 15), r((v >> 8) & 15, 15), r((v >> 4) & 15, 15), r(v & 15, 15)]
        }
        4 => {
            let v = u32::from_be_bytes([0, b[0], b[1], b[2]]);
            [r(v >> 18, 63), r((v >> 12) & 63, 63), r((v >> 6) & 63, 63), r(v & 63, 63)]
        }
        5 => [b[0] as f32 / 255., b[1] as f32 / 255., b[2] as f32 / 255., b[3] as f32 / 255.],
        _ => return Err("bad GX vertex colour format".into()),
    })
}
#[derive(Clone, Debug)]
pub struct Texture {
    pub wrap_s: u8,
    pub wrap_t: u8,
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Batch {
    pub texture_transform: Option<TextureTransform>,
    pub texture_frames: Vec<Texture>,
    pub instance: Option<(usize, usize)>,
    pub material_dobj: usize,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
    pub texture: Option<Texture>,
    pub dobj: usize,
    pub owner: usize,
    pub blend: u8,
    pub lit: u8,
    /// HSD TObj alpha operation and its independently animated interpolation.
    pub texture_alpha: u8,
    pub texture_blend: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TextureTransform {
    pub id: u32,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub translation: Vec3,
    pub repeat: Vec2,
    pub mirror_t: bool,
}
impl TextureTransform {
    /// baselib/tobj.c::MakeTextureMtx, also used by animated TObj channels.
    pub fn matrix(&self) -> Mat4 {
        let x = if self.scale.x.abs() < f32::EPSILON { 0. } else { self.repeat.x / self.scale.x };
        let y = if self.scale.y.abs() < f32::EPSILON { 0. } else { self.repeat.y / self.scale.y };
        let mirror = if self.mirror_t && y != 0. { 1. / y } else { 0. };
        Mat4::from_scale(Vec3::new(x, y, self.scale.z))
            * Mat4::from_quat(Quat::from_euler(EulerRot::ZYX, -self.rotation.z, self.rotation.y, self.rotation.x))
            * Mat4::from_translation(Vec3::new(-self.translation.x, -self.translation.y - mirror, self.translation.z))
    }
}
#[derive(Clone)]
pub struct Model {
    pub joints: Vec<Joint>,
    pub batches: Vec<Batch>,
    pub normal_visible: Vec<bool>,
    pub ball_visible: Vec<bool>,
}
struct Attr {
    kind: u32,
    storage: u32,
    count: u32,
    format: u32,
    frac: u8,
    stride: usize,
    data: usize,
}
fn vector(d: &Dat, at: usize) -> Result<Vec3> {
    Ok(Vec3::new(d.f32(at)?, d.f32(at + 4)?, d.f32(at + 8)?))
}
pub fn local(t: Vec3, r: Vec3, s: Vec3) -> Mat4 {
    Mat4::from_scale_rotation_translation(s, Quat::from_euler(EulerRot::ZYX, r.z, r.y, r.x), t)
}
impl Model {
    pub fn load(dir: &Path, costume: usize) -> Result<Self> {
        Self::load_for(dir, costume, &crate::fighter::SAMUS)
    }
    pub fn load_for(
        dir: &Path,
        costume: usize,
        fighter: &crate::fighter::FighterAssets<'_>,
    ) -> Result<Self> {
        fighter.required()?;
        let file = fighter.costumes.get(costume).ok_or("invalid costume")?;
        let bytes = std::fs::read(dir.join(file))?;
        let mut model = Self::parse(&Dat::parse(&bytes)?)?;
        let bytes = std::fs::read(dir.join(fighter.data.file))?;
        let d = Dat::parse(&bytes)?;
        let root = d.root(fighter.data.root)?;
        let desc = d.required_ptr(root + 8)?;
        // ftCo_DatAttrs.model_scaling (co + 0x8C) normalizes each fighter's raw
        // model to the shared in-game scale (Luigi/Link authored small, Samus and
        // Pikachu authored large). Apply it so relative heights match the source.
        let co = d.required_ptr(root)?;
        let model_scaling = d.f32(co + 0x8c)?;
        if !model_scaling.is_finite() || model_scaling <= 0. {
            return Err("invalid fighter model scaling".into());
        }
        model.scale(model_scaling);
        model.normal_visible = visibility(&d, desc, costume, model.normal_visible.len(), 0)?;
        model.ball_visible = if let Some(choice) = fighter.morph_visibility {
            visibility(&d, desc, costume, model.ball_visible.len(), choice)?
        } else {
            model.normal_visible.clone()
        };
        Ok(model)
    }
    pub fn parse(d: &Dat) -> Result<Self> {
        let root = *d
            .symbols
            .iter()
            .find(|(s, _)| s.ends_with("_Share_joint"))
            .ok_or("missing shared joint")?
            .1;
        let model = Self::from_joint(d, root)?;
        if model.batches.is_empty() {
            return Err("fighter model has no triangles".into());
        }
        Ok(model)
    }
    pub fn from_joint(d: &Dat, root: usize) -> Result<Self> {
        let mut joints = Vec::new();
        let mut visited = HashSet::new();
        fn walk(
            d: &Dat,
            at: usize,
            parent: Option<usize>,
            j: &mut Vec<Joint>,
            seen: &mut HashSet<usize>,
            depth: u32,
        ) -> Result<()> {
            if depth > 128 || j.len() > 2048 || !seen.insert(at) {
                return Err("cyclic/oversized joint tree".into());
            }
            let t = vector(d, at + 0x2c)?;
            let r = vector(d, at + 0x14)?;
            let s = vector(d, at + 0x20)?;
            let bind = parent.map(|p| j[p].bind).unwrap_or(Mat4::IDENTITY) * local(t, r, s);
            if !bind.is_finite() || bind.determinant().abs() < 1e-10 {
                return Err("singular joint transform".into());
            }
            let i = j.len();
            j.push(Joint {
                flags: d.u32(at + 4)?,
                address: at,
                parent,
                translation: t,
                rotation: r,
                scale: s,
                bind,
                inverse: bind.inverse(),
            });
            // HSD JOBJ_INSTANCE's child is a reference to another joint, not
            // an owned subtree. Traversing it here invents a cyclic hierarchy.
            if d.u32(at + 4)? & (1 << 12) == 0 {
                if let Some(c) = d.ptr(at + 8)? {
                    walk(d, c, Some(i), j, seen, depth + 1)?;
                }
            }
            if let Some(n) = d.ptr(at + 12)? {
                walk(d, n, parent, j, seen, depth + 1)?;
            }
            Ok(())
        }
        walk(d, root, None, &mut joints, &mut visited, 0)?;
        let lookup: HashMap<_, _> = joints
            .iter()
            .enumerate()
            .map(|(i, j)| (j.address, i))
            .collect();
        let mut batches = Vec::new();
        let mut dobj_index = 0;
        for (owner, j) in joints.iter().enumerate() {
            if d.u32(j.address + 4)? & ((1 << 5) | (1 << 12) | (1 << 14)) != 0 {
                continue;
            }
            let mut dobj = d.ptr(j.address + 16)?;
            let mut seen = HashSet::new();
            while let Some(at) = dobj {
                if !seen.insert(at) || seen.len() > 4096 {
                    return Err("cyclic DObj".into());
                }
                let (color, ambient, texture, texture_alpha, texture_blend) = material(d, d.ptr(at + 8)?)?;
                let (blend, lit) = render_flags(d, d.ptr(at + 8)?)?;
                let texture_transform = texture_transform(d, d.ptr(at + 8)?)?;
                let uv_matrix = texture_transform.map_or(Mat4::IDENTITY, |t| t.matrix());
                let mut batch = Batch {
                    texture_transform,
                    texture_frames: Vec::new(),
                    instance: None,
                    material_dobj: dobj_index,
                    texture_alpha,
                    texture_blend,
                    vertices: vec![],
                    indices: vec![],
                    color,
                    texture,
                    dobj: dobj_index,
                    owner,
                    blend,
                    lit,
                };
                dobj_index += 1;
                let mut pobj = d.ptr(at + 12)?;
                let mut seen = HashSet::new();
                while let Some(p) = pobj {
                    if !seen.insert(p) || seen.len() > 4096 {
                        return Err("cyclic PObj".into());
                    }
                    polygon(d, p, owner, &joints, &lookup, &mut batch)?;
                    pobj = d.ptr(p + 4)?;
                }
                if !batch.indices.is_empty() {
                    for vertex in &mut batch.vertices {
                        let uv = uv_matrix.transform_point3(vertex.uv.extend(0.));
                        vertex.uv = uv.truncate();
                    }
                    if batch.texture.is_none() {
                        apply_untextured_color(&mut batch, ambient);
                    }
                    batches.push(batch);
                }
                dobj = d.ptr(at + 4)?;
            }
        }
        // HSD_JObjDispAll instances apply instance * inverse(target) to the
        // target geometry. Retain animation joint indices; share the geometry
        // definition while adding independent render batches for each instance.
        let templates = batches.clone();
        for (instance, joint) in joints.iter().enumerate() {
            if d.u32(joint.address + 4)? & (1 << 12) == 0 {
                continue;
            }
            let target = *lookup
                .get(&d.required_ptr(joint.address + 8)?)
                .ok_or("instance target outside joint hierarchy")?;
            for source in &templates {
                let mut ancestor = Some(source.owner);
                let mut belongs = false;
                while let Some(index) = ancestor {
                    if index == target {
                        belongs = true;
                        break;
                    }
                    ancestor = joints[index].parent;
                }
                if belongs {
                    let mut batch = source.clone();
                    batch.instance = Some((instance, target));
                    batch.dobj = dobj_index;
                    dobj_index += 1;
                    batches.push(batch);
                }
            }
        }
        Ok(Self {
            joints,
            batches,
            normal_visible: vec![true; dobj_index],
            ball_visible: vec![true; dobj_index],
        })
    }
    pub fn bind_world(&self) -> Vec<Mat4> {
        self.joints.iter().map(|j| j.bind).collect()
    }
    /// Uniformly scale the whole model (joint skeleton and bind-space vertices).
    /// ftCo_DatAttrs.model_scaling applies this in the source; both the joint
    /// translations and the authored vertex positions scale so skinning and bone
    /// attachments stay consistent.
    pub fn scale(&mut self, factor: f32) {
        if !factor.is_finite() || factor <= 0. || (factor - 1.).abs() < 1e-6 {
            return;
        }
        for joint in &mut self.joints {
            joint.translation *= factor;
        }
        // `from_joint` walks parents before children, so an index-ordered pass
        // rebuilds each bind from an already-rescaled parent.
        for i in 0..self.joints.len() {
            let j = &self.joints[i];
            let parent = j.parent;
            let bind = parent.map_or(Mat4::IDENTITY, |p| self.joints[p].bind)
                * local(j.translation, j.rotation, j.scale);
            let joint = &mut self.joints[i];
            joint.bind = bind;
            joint.inverse = bind.inverse();
        }
        for batch in &mut self.batches {
            for vertex in &mut batch.vertices {
                vertex.position *= factor;
            }
        }
    }
    pub fn skin(&self, batch: usize, world: &[Mat4]) -> Vec<(Vec3, Vec3)> {
        self.batches[batch]
            .vertices
            .iter()
            .map(|v| {
                let mut p = Vec3::ZERO;
                let mut n = Vec3::ZERO;
                for &(j, w) in &v.weights {
                    let m = world[j] * self.joints[j].inverse;
                    p += m.transform_point3(v.position) * w;
                    n += m.inverse().transpose().transform_vector3(v.normal) * w;
                }
                if let Some((instance, target)) = self.batches[batch].instance {
                    let matrix = world[instance] * world[target].inverse();
                    p = matrix.transform_point3(p);
                    n = matrix.inverse().transpose().transform_vector3(n);
                }
                (p, n.normalize_or_zero())
            })
            .collect()
    }
    /// HSD camera-facing joint modes. RBILLBOARD retains the authored Z spin;
    /// the host supplies the camera in this model's coordinate space.
    pub fn face_camera(&self, world: &mut [Mat4], camera: Vec3) {
        for i in 0..self.joints.len() {
            let mode = self.joints[i].flags & 0xe00;
            if mode == 0 {
                continue;
            }
            let old = world[i];
            let (scale, rotation, position) = old.to_scale_rotation_translation();
            let mut look = camera - position;
            if mode == 0x400 || mode == 0x600 {
                look.y = 0.;
            }
            let Some(forward) = look.try_normalize() else {
                continue;
            };
            let right = Vec3::Y.cross(forward).try_normalize().unwrap_or(Vec3::X);
            let up = forward.cross(right).normalize_or_zero();
            let facing = Quat::from_mat3(&glam::Mat3::from_cols(right, up, forward));
            let spin = if mode == 0x800 {
                Quat::from_rotation_z(rotation.to_euler(EulerRot::YXZ).2)
            } else {
                Quat::IDENTITY
            };
            let next = Mat4::from_scale_rotation_translation(scale, facing * spin, position);
            let delta = next * old.inverse();
            world[i] = next;
            for j in i + 1..self.joints.len() {
                let mut parent = self.joints[j].parent;
                while let Some(p) = parent {
                    if p == i {
                        world[j] = delta * world[j];
                        break;
                    }
                    parent = self.joints[p].parent;
                }
            }
        }
    }
}
// ftParts_8007487C / 80074B6C: costume fallback and high-detail DObj alternatives.
fn visibility(
    d: &Dat,
    desc: usize,
    costume: usize,
    count: usize,
    choice: usize,
) -> Result<Vec<bool>> {
    let mut visible = vec![true; count];
    let groups = d.u32(desc)? as usize;
    if groups > 11 {
        return Err("invalid visibility groups".into());
    }
    let table = d.required_ptr(desc + 4)?;
    for mode in [1, 0] {
        let lookup = d
            .ptr(table + costume * 16 + mode * 4)?
            .or(d.ptr(table + mode * 4)?);
        let Some(lookup) = lookup else {
            continue;
        };
        for group in 0..groups {
            let n = d.u32(lookup + group * 8)? as usize;
            if n > 64 {
                return Err("invalid visibility alternatives".into());
            }
            // A group with no alternatives (Pikachu has some) carries no row table.
            if n == 0 {
                continue;
            }
            let rows = d.required_ptr(lookup + group * 8 + 4)?;
            for alt in 0..n {
                let n = d.u32(rows + alt * 8)? as usize;
                if let Some(ids) = d.ptr(rows + alt * 8 + 4)? {
                    for &id in d.bytes(ids, n)? {
                        *visible
                            .get_mut(id as usize)
                            .ok_or("visibility DObj outside model")? = mode == 0 && alt == choice;
                    }
                }
            }
        }
    }
    Ok(visible)
}
fn material(d: &Dat, at: Option<usize>) -> Result<([f32; 4], [f32; 3], Option<Texture>, u8, f32)> {
    let Some(at) = at else {
        return Ok(([1.; 4], [0.5; 3], None, 3, 1.));
    };
    let mut color = [1.; 4];
    let mut ambient = [0.5; 3];
    if let Some(m) = d.ptr(at + 12)? {
        let a = d.bytes(m, 3)?;
        for i in 0..3 {
            ambient[i] = a[i] as f32 / 255.;
        }
        let c = d.bytes(m + 4, 4)?;
        for i in 0..3 {
            color[i] = c[i] as f32 / 255.;
        }
        color[3] = d.f32(m + 12)?.clamp(0., 1.);
    }
    let mut tex = d.ptr(at + 8)?;
    let mut seen = HashSet::new();
    while let Some(t) = tex {
        if !seen.insert(t) || seen.len() > 16 {
            return Err("cyclic texture chain".into());
        }
        if d.u32(t + 0x40)? & 15 == 0
            && let Some(image) = d.ptr(t + 0x4c)?
        {
            let start = d.required_ptr(image)?;
            let width = d.u16(image + 4)?;
            let height = d.u16(image + 6)?;
            let fmt = d.u32(image + 8)?;
            let pal = d
                .ptr(t + 0x50)?
                .map(|p| -> Result<_> {
                    Ok((
                        d.bytes(d.required_ptr(p)?, d.u16(p + 12)? as usize * 2)?,
                        d.u32(p + 4)?,
                    ))
                })
                .transpose()?;
            let rgba = crate::texture::decode(d.data, start, width, height, fmt, pal)?;
            return Ok((
                color,
                ambient,
                Some(Texture {
                    wrap_s: wrap_mode(d.u32(t + 0x34)?),
                    wrap_t: wrap_mode(d.u32(t + 0x38)?),
                    width,
                    height,
                    rgba,
                }),
                ((d.u32(t + 0x40)? >> 20) & 15) as u8,
                d.f32(t + 0x44)?.clamp(0., 1.),
            ));
        }
        tex = d.ptr(t + 4)?;
    }
    Ok((color, ambient, None, 3, 1.))
}
/// Untextured effect models carry their colour in the vertex stream (Fox's red Blaster
/// bolts) or in the material's ambient register (the blue Illusion ghosts); neither is
/// the diffuse colour the textured path uses. The host draws one colour per batch, so
/// fold the per-vertex colour and ambient into it.
fn apply_untextured_color(batch: &mut Batch, ambient: [f32; 3]) {
    if batch.vertices.is_empty() {
        return;
    }
    let n = batch.vertices.len() as f32;
    let mut v = [0f32; 4];
    for vertex in &batch.vertices {
        for c in 0..4 {
            v[c] += vertex.color[c];
        }
    }
    for c in &mut v {
        *c /= n;
    }
    let white = v[..3].iter().all(|c| (c - 1.).abs() < 0.01);
    let gray_ambient = ambient.iter().all(|c| (c - ambient[0]).abs() < 0.01);
    let rgb = if !white {
        [v[0], v[1], v[2]]
    } else if !gray_ambient {
        ambient
    } else {
        return;
    };
    batch.color = [rgb[0], rgb[1], rgb[2], batch.color[3] * v[3]];
}
pub(crate) fn texture_frame(d: &Dat, image: usize, palette: Option<usize>, base: &Texture) -> Result<Texture> {
    let palette = palette.map(|p| -> Result<_> {
        Ok((d.bytes(d.required_ptr(p)?, d.u16(p + 12)? as usize * 2)?, d.u32(p + 4)?))
    }).transpose()?;
    let width = d.u16(image + 4)?;
    let height = d.u16(image + 6)?;
    let rgba = crate::texture::decode(d.data, d.required_ptr(image)?, width, height, d.u32(image + 8)?, palette)?;
    Ok(Texture { width, height, rgba, wrap_s: base.wrap_s, wrap_t: base.wrap_t })
}
fn wrap_mode(gx: u32) -> u8 {
    match gx {
        0 => 1,
        2 => 2,
        _ => 0,
    }
}
/// baselib/tobj.c MakeTextureMtx: repeat/scale * rotation * translation,
/// including the mirrored-T offset. A quarter texture repeated/mirrored twice
/// is the complete Charge Shot core, rather than a cropped rectangular patch.
fn texture_transform(d: &Dat, material: Option<usize>) -> Result<Option<TextureTransform>> {
    let Some(material) = material else {
        return Ok(None);
    };
    let mut at = d.ptr(material + 8)?;
    let mut seen = HashSet::new();
    while let Some(t) = at {
        if !seen.insert(t) || seen.len() > 16 {
            return Err("cyclic texture matrix chain".into());
        }
        if d.u32(t + 0x40)? & 15 == 0 {
            let r = vector(d, t + 0x10)?;
            let s = vector(d, t + 0x1c)?;
            let tr = vector(d, t + 0x28)?;
            let repeat = d.bytes(t + 0x3c, 2)?;
            return Ok(Some(TextureTransform { id: d.u32(t + 8)?, rotation: r, scale: s, translation: tr,
                repeat: Vec2::new(repeat[0] as f32, repeat[1] as f32), mirror_t: d.u32(t + 0x38)? == 2 }));
        }
        at = d.ptr(t + 4)?;
    }
    Ok(None)
}
fn render_flags(d: &Dat, at: Option<usize>) -> Result<(u8, u8)> {
    let Some(at) = at else {
        return Ok((0, 1));
    };
    let mode = d.u32(at + 4)?;
    let mut blend = if mode & (1 << 30) != 0 { 2 } else { 0 };
    if let Some(pe) = d.ptr(at + 20)? {
        let state = d.bytes(pe, 12)?;
        if state[4] == 1 {
            blend = if state[6] == 1 { 3 } else { 2 };
        }
    }
    Ok((blend, if mode & 4 != 0 { 1 } else { 0 }))
}
fn polygon(
    d: &Dat,
    p: usize,
    owner: usize,
    joints: &[Joint],
    lookup: &HashMap<usize, usize>,
    batch: &mut Batch,
) -> Result<()> {
    let mut attrs = Vec::new();
    let mut at = d.required_ptr(p + 8)?;
    for _ in 0..32 {
        let kind = d.u32(at)?;
        if kind == 255 {
            break;
        }
        attrs.push(Attr {
            kind,
            storage: d.u32(at + 4)?,
            count: d.u32(at + 8)?,
            format: d.u32(at + 12)?,
            frac: d.bytes(at + 16, 1)?[0],
            stride: d.u16(at + 18)? as usize,
            data: d.ptr(at + 20)?.unwrap_or(0),
        });
        at += 24;
    }
    if attrs.len() == 32 {
        return Err("unterminated vertex descriptor".into());
    }
    let flags = d.u16(p + 12)?;
    let mut palettes = Vec::<Vec<(usize, f32)>>::new();
    match flags & 0x3000 {
        0 => {
            let j = d
                .ptr(p + 20)?
                .and_then(|a| lookup.get(&a).copied())
                .unwrap_or(owner);
            palettes.push(vec![(owner, 1.)]);
            if j != owner {
                palettes.push(vec![(j, 1.)]);
            }
        }
        0x2000 => {
            let table = d.required_ptr(p + 20)?;
            for i in 0..10 {
                let Some(env) = d.ptr(table + i * 4)? else {
                    break;
                };
                let mut weights = Vec::new();
                for n in 0..32 {
                    let Some(j) = d.ptr(env + n * 8)? else {
                        break;
                    };
                    let j = *lookup.get(&j).ok_or("missing envelope joint")?;
                    let w = d.f32(env + n * 8 + 4)?;
                    if !(0. ..=1.).contains(&w) {
                        return Err("bad weight".into());
                    }
                    weights.push((j, w));
                }
                if (weights.iter().map(|v| v.1).sum::<f32>() - 1.).abs() > 0.01 {
                    return Err("bad weight sum".into());
                }
                palettes.push(weights);
            }
        }
        _ => return Err("shape animation polygon unsupported".into()),
    }
    let stream = d.bytes(d.required_ptr(p + 16)?, d.u16(p + 14)? as usize * 32)?;
    let mut cursor = 0;
    while cursor < stream.len() {
        let command = stream[cursor];
        cursor += 1;
        if command == 0 {
            continue;
        }
        let primitive = command & 0xf8;
        if !matches!(primitive, 0x80 | 0x90 | 0x98 | 0xa0) {
            return Err(format!("unsupported GX command {command:x}").into());
        }
        let count = u16::from_be_bytes(
            stream
                .get(cursor..cursor + 2)
                .ok_or("truncated GX count")?
                .try_into()?,
        ) as usize;
        cursor += 2;
        if batch.vertices.len() + count > 1_000_000 {
            return Err("vertex budget exceeded".into());
        }
        let base = batch.vertices.len() as u32;
        for _ in 0..count {
            let mut pos = Vec3::ZERO;
            let mut normal = Vec3::Y;
            let mut uv = Vec2::ZERO;
            let mut color = [1.; 4];
            let mut matrix = 0;
            for a in &attrs {
                if a.storage == 0 {
                    continue;
                }
                if a.kind <= 8 {
                    if a.kind == 0 {
                        matrix = *stream.get(cursor).ok_or("truncated GX matrix")? as usize / 3;
                    }
                    cursor += 1;
                    continue;
                }
                let is_color = matches!(a.kind, 11 | 12);
                let components = match a.kind {
                    9 => {
                        if a.count == 0 {
                            2
                        } else {
                            3
                        }
                    }
                    10 | 25 => {
                        if a.count == 1 {
                            9
                        } else {
                            3
                        }
                    }
                    11 | 12 => 1,
                    13..=20 => {
                        if a.count == 0 {
                            1
                        } else {
                            2
                        }
                    }
                    _ => return Err("unknown GX attribute".into()),
                };
                let width = if is_color {
                    match a.format {
                        0 | 3 => 2,
                        1 | 4 => 3,
                        2 | 5 => 4,
                        _ => return Err("bad GX color format".into()),
                    }
                } else {
                    (match a.format {
                        0 | 1 => 1,
                        2 | 3 => 2,
                        4 => 4,
                        _ => return Err("bad GX scalar format".into()),
                    }) * components
                };
                let data = match a.storage {
                    1 => {
                        let v = stream
                            .get(cursor..cursor + width)
                            .ok_or("truncated GX direct")?;
                        cursor += width;
                        v
                    }
                    2 | 3 => {
                        let index = if a.storage == 2 {
                            let n = *stream.get(cursor).ok_or("truncated GX index")? as usize;
                            cursor += 1;
                            n
                        } else {
                            let n = u16::from_be_bytes(
                                stream
                                    .get(cursor..cursor + 2)
                                    .ok_or("truncated GX index")?
                                    .try_into()?,
                            ) as usize;
                            cursor += 2;
                            n
                        };
                        if matches!(a.kind, 10 | 25) && a.count == 2 {
                            cursor += if a.storage == 2 { 2 } else { 4 };
                        }
                        d.bytes(a.data + index * a.stride, width)?
                    }
                    _ => return Err("bad GX storage".into()),
                };
                if is_color {
                    // CLR0 is the diffuse vertex colour; CLR1 is a secondary/specular
                    // channel the port's single-colour materials do not use.
                    if a.kind == 11 {
                        color = vertex_color(a.format, data)?;
                    }
                    continue;
                }
                let mut value = [0.; 9];
                let scalar = width / components;
                for (i, out) in value.iter_mut().enumerate().take(components) {
                    let b = &data[i * scalar..(i + 1) * scalar];
                    *out = match a.format {
                        0 => b[0] as f32,
                        1 => (b[0] as i8) as f32,
                        2 => u16::from_be_bytes(b.try_into()?) as f32,
                        3 => i16::from_be_bytes(b.try_into()?) as f32,
                        4 => f32::from_be_bytes(b.try_into()?),
                        _ => unreachable!(),
                    };
                    if a.format != 4 {
                        *out /= 2f32.powi(a.frac as i32);
                    }
                    if !out.is_finite() {
                        return Err("nonfinite GX vertex".into());
                    }
                }
                match a.kind {
                    9 => pos = Vec3::new(value[0], value[1], value[2]),
                    10 | 25 => normal = Vec3::new(value[0], value[1], value[2]),
                    13 => uv = Vec2::new(value[0], value[1]),
                    _ => (),
                }
            }
            if cursor > stream.len() {
                return Err("truncated GX stream".into());
            }
            let weights = palettes
                .get(matrix)
                .ok_or("GX matrix outside palette")?
                .clone();
            if flags & 0x3000 == 0
                || (weights.len() == 1 && d.u32(joints[owner].address + 4)? & 2 != 0)
            {
                let m = joints[weights[0].0].bind;
                pos = m.transform_point3(pos);
                normal = m
                    .inverse()
                    .transpose()
                    .transform_vector3(normal)
                    .normalize_or_zero();
            }
            batch.vertices.push(Vertex {
                position: pos,
                normal,
                uv,
                color,
                weights,
            });
        }
        let mut tri = |a: usize, b: usize, c: usize| {
            batch
                .indices
                .extend([base + a as u32, base + b as u32, base + c as u32])
        };
        match primitive {
            0x80 => {
                for i in (0..count.saturating_sub(3)).step_by(4) {
                    tri(i, i + 1, i + 2);
                    tri(i, i + 2, i + 3);
                }
            }
            0x90 => {
                for i in (0..count.saturating_sub(2)).step_by(3) {
                    tri(i, i + 1, i + 2);
                }
            }
            0x98 => {
                for i in 2..count {
                    if i % 2 == 0 {
                        tri(i - 2, i - 1, i)
                    } else {
                        tri(i - 1, i - 2, i)
                    }
                }
            }
            0xa0 => {
                for i in 2..count {
                    tri(0, i - 1, i);
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
