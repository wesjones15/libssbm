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
    pub weights: Vec<(usize, f32)>,
}
#[derive(Clone, Debug)]
pub struct Texture {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Batch {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
    pub texture: Option<Texture>,
    pub dobj: usize,
}
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
        let c = *crate::COSTUMES.get(costume).ok_or("invalid costume")?;
        let bytes = std::fs::read(dir.join(format!("PlSs{c}.dat")))?;
        let mut model = Self::parse(&Dat::parse(&bytes)?)?;
        let fighter = std::fs::read(dir.join("PlSs.dat"))?;
        let d = Dat::parse(&fighter)?;
        let desc = d.required_ptr(d.root("ftDataSamus")? + 8)?;
        model.normal_visible = visibility(&d, desc, costume, model.normal_visible.len(), 0)?;
        model.ball_visible = visibility(&d, desc, costume, model.ball_visible.len(), 1)?;
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
                address: at,
                parent,
                translation: t,
                rotation: r,
                scale: s,
                bind,
                inverse: bind.inverse(),
            });
            if let Some(c) = d.ptr(at + 8)? {
                walk(d, c, Some(i), j, seen, depth + 1)?;
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
            if d.u32(j.address + 4)? & ((1 << 5) | (1 << 14)) != 0 {
                continue;
            }
            let mut dobj = d.ptr(j.address + 16)?;
            let mut seen = HashSet::new();
            while let Some(at) = dobj {
                if !seen.insert(at) || seen.len() > 4096 {
                    return Err("cyclic DObj".into());
                }
                let (color, texture) = material(d, d.ptr(at + 8)?)?;
                let mut batch = Batch {
                    vertices: vec![],
                    indices: vec![],
                    color,
                    texture,
                    dobj: dobj_index,
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
                    batches.push(batch);
                }
                dobj = d.ptr(at + 4)?;
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
                (p, n.normalize_or_zero())
            })
            .collect()
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
fn material(d: &Dat, at: Option<usize>) -> Result<([f32; 4], Option<Texture>)> {
    let Some(at) = at else {
        return Ok(([1.; 4], None));
    };
    let mut color = [1.; 4];
    if let Some(m) = d.ptr(at + 12)? {
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
                Some(Texture {
                    width,
                    height,
                    rgba,
                }),
            ));
        }
        tex = d.ptr(t + 4)?;
    }
    Ok((color, None))
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
                let color = matches!(a.kind, 11 | 12);
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
                let width = if color {
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
                if color {
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
