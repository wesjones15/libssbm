//! Assembled Sunshine FLUDD for the Luigi alt-skin preview: body (`wg_mdl1`) + Spray
//! nozzle (`normal_wg`), each posed by its own housed clip, following
//! `TWaterGun::init`/`setBaseTRMtx` (base = chest, body clip, nozzle on `nozzle_center`).
//! Distinct from the earlier single-mesh mock: both parts are here, whole.
use libsms::Clip;
use libsms_assets::bmd::{Bmd, Skin};
use macroquad::prelude::*;
use std::path::Path;

extern crate glam as g;
use g::{Mat4, Vec3};

struct Vert {
    p: Vec3,
    uv: [f32; 2],
    joint: usize,
    color: [u8; 4],
    /// Envelope weights (joint, weight); empty for joint-skinned vertices.
    env: Vec<(u16, f32)>,
}

struct Part {
    bmd: Bmd,
    batches: Vec<(Vec<Vert>, Vec<u16>, Option<Texture2D>)>,
    texels: Vec<Option<(u32, u32, Vec<u8>)>>,
    inverse_bind: Vec<Mat4>,
}

impl Part {
    fn load(dir: &Path, rel: &str) -> Option<Part> {
        let bmd = Bmd::parse(&std::fs::read(dir.join(rel)).ok()?).ok()?;
        let decoded = bmd.batches().ok()?;
        let inverse_bind = bmd.inverse_bind.clone();
        let envelopes = bmd.envelopes.clone();
        let mut batches = Vec::with_capacity(decoded.len());
        let mut texels = Vec::with_capacity(decoded.len());
        for batch in &decoded {
            let image = bmd
                .materials
                .get(batch.material)
                .and_then(|m| m.textures[0])
                .and_then(|t| bmd.textures.get(t as usize))
                .map(|t| (t.image.width, t.image.height, t.image.rgba.clone()));
            let texture = bmd
                .materials
                .get(batch.material)
                .and_then(|m| m.textures[0])
                .and_then(|t| bmd.textures.get(t as usize))
                .map(|t| {
                    // FLUDD's shell and nozzle are opaque. The disc textures carry an
                    // alpha channel for TEV passes; drawing it straight makes the
                    // trumpet translucent and drops alpha-zero parts (the yellow head).
                    let mut rgba = t.image.rgba.clone();
                    for px in rgba.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                    let tex = Texture2D::from_rgba8(
                        t.image.width as u16,
                        t.image.height as u16,
                        &rgba,
                    );
                    tex.set_filter(FilterMode::Linear);
                    tex
                });
            let verts = batch
                .vertices
                .iter()
                .map(|v| Vert {
                    p: Vec3::new(v.position.x, v.position.y, v.position.z),
                    uv: [v.uv[0].x, v.uv[0].y],
                    color: v.colors[0],
                    joint: match v.skin {
                        Skin::Joint(j) => j as usize,
                        Skin::Envelope(_) => 0,
                    },
                    env: match v.skin {
                        Skin::Joint(_) => Vec::new(),
                        Skin::Envelope(e) => envelopes
                            .get(e as usize)
                            .cloned()
                            .unwrap_or_default(),
                    },
                })
                .collect();
            let indices = batch.indices.iter().map(|&i| i as u16).collect();
            batches.push((verts, indices, texture));
            texels.push(image);
        }
        Some(Part {
            bmd,
            batches,
            texels,
            inverse_bind,
        })
    }

    fn joints(&self, clip: &Clip, frame: f32) -> Vec<Mat4> {
        self.bmd.pose(|j| clip.sample(j, frame), |_| None)
    }
}

pub struct Fludd {
    body: Part,
    nozzle: Part,
    body_clip: Clip,
    nozzle_clip: Clip,
    /// Lowest local x of the yellow cap's triangles: the cap sits from here up.
    cap_min: f32,
}

impl Fludd {
    pub fn load(dir: &Path) -> Option<Fludd> {
        let body = Part::load(dir, "watergun2/body/wg_mdl1.bmd")?;
        let nozzle = Part::load(dir, "watergun2/normal_wg/normal_wg.bmd")?;
        let body_clip =
            Clip::parse(&std::fs::read(dir.join("watergun2/body/wg_house.bck")).ok()?)?;
        let nozzle_clip = Clip::parse(
            &std::fs::read(dir.join("watergun2/normal_wg/normal_wg_change_end.bck")).ok()?,
        )?;
        let mut cap_min = f32::INFINITY;
        for (bi, batch) in body.batches.iter().enumerate() {
            let Some(tex) = body.texels.get(bi) else { continue };
            for v in batch.0.iter() {
                if v.p.x > 50.0 && is_yellow(tex, v.uv) {
                    cap_min = cap_min.min(v.p.x);
                }
            }
        }
        if !cap_min.is_finite() {
            cap_min = 56.0;
        }
        Some(Fludd {
            body,
            nozzle,
            body_clip,
            nozzle_clip,
            cap_min,
        })
    }

    /// `base` maps FLUDD model space into Luigi model space (chest frame + scale).
    /// `segments` repeats the collar band `slice` (blue rings + yellow cap) so the
    /// top copy carries the cap; the nozzle sits on that top copy.
    pub fn draw(
        &self,
        base: Mat4,
        rotation: Mat4,
        at: Vec3,
        segments: u32,
        slice: (f32, f32),
        step: f32,
        nozzle_yaw: f32,
        neck_lean: f32,
    ) {
        let bj = self.body.joints(&self.body_clip, 0.);
        // Lean the whole neck (collar + drum + nozzle) about the pack's top, not the
        // bell alone. `NECK_PIVOT` is the collar's base in FLUDD model units.
        const NECK_PIVOT: f32 = 45.0;
        let neck = base
            * Mat4::from_translation(Vec3::new(NECK_PIVOT, 0.0, 0.0))
            * Mat4::from_rotation_y(neck_lean.to_radians())
            * Mat4::from_translation(Vec3::new(-NECK_PIVOT, 0.0, 0.0));
        draw_body(
            &self.body,
            &bj,
            base,
            neck,
            NECK_PIVOT,
            rotation,
            at,
            segments,
            slice,
            step,
            self.cap_min,
        );
        let Some(nozzle_center) = self.body.bmd.joint("nozzle_center") else {
            return;
        };
        // The nozzle sits on top of the cap, which itself sits on the top dome.
        let step = if step > 0.0 { step } else { (slice.1 - slice.0).max(0.0) };
        let mut mount = bj[nozzle_center];
        mount.w_axis.x += step * segments as f32;
        let nozzle_at = neck * mount * Mat4::from_rotation_z(nozzle_yaw.to_radians());
        let nj = self.nozzle.bmd.bind_world();
        draw_part(&self.nozzle, &nj, nozzle_at, rotation, at);
    }
}

fn is_blue(tex: &Option<(u32, u32, Vec<u8>)>, uv: [f32; 2]) -> bool {
    let Some((w, h, d)) = tex else {
        return false;
    };
    let (w, h) = ((*w).max(1), (*h).max(1));
    let at = |v: f32| {
        let x = (uv[0].rem_euclid(1.0) * w as f32) as u32 % w;
        let y = (v.rem_euclid(1.0) * h as f32) as u32 % h;
        let i = ((y * w + x) * 4) as usize;
        if i + 2 >= d.len() {
            return false;
        }
        let (r, g, b) = (d[i] as i32, d[i + 1] as i32, d[i + 2] as i32);
        b > 130 && b > r + 40 && b > g + 30
    };
    at(uv[1]) || at(1.0 - uv[1])
}

fn is_yellow(tex: &Option<(u32, u32, Vec<u8>)>, uv: [f32; 2]) -> bool {
    let Some((w, h, d)) = tex else {
        return false;
    };
    let (w, h) = ((*w).max(1), (*h).max(1));
    let at = |v: f32| {
        let x = (uv[0].rem_euclid(1.0) * w as f32) as u32 % w;
        let y = (v.rem_euclid(1.0) * h as f32) as u32 % h;
        let i = ((y * w + x) * 4) as usize;
        if i + 2 >= d.len() {
            return false;
        }
        let (r, g, b) = (d[i] as i32, d[i + 1] as i32, d[i + 2] as i32);
        r > 150 && g > 110 && b < 110 && r > b + 60
    };
    // GX textures may be sampled with V either way up; the cap is the same colour.
    at(uv[1]) || at(1.0 - uv[1])
}

/// Joint- or envelope-skinned model-space position. Envelope vertices are stored in
/// model space (`Skin::Envelope`), so blend `joint * inverse_bind` by weight.
fn skinned(part: &Part, joints: &[Mat4], v: &Vert) -> Vec3 {
    if v.env.is_empty() {
        return joints
            .get(v.joint)
            .copied()
            .unwrap_or(Mat4::IDENTITY)
            .transform_point3(v.p);
    }
    let mut acc = Vec3::ZERO;
    let mut wsum = 0.0;
    for &(j, w) in &v.env {
        let j = j as usize;
        let m = joints.get(j).copied().unwrap_or(Mat4::IDENTITY)
            * part.inverse_bind.get(j).copied().unwrap_or(Mat4::IDENTITY);
        acc += m.transform_point3(v.p) * w;
        wsum += w;
    }
    if wsum > 1e-6 { acc / wsum } else { acc }
}

fn draw_body(
    part: &Part,
    joints: &[Mat4],
    base: Mat4,
    neck: Mat4,
    pivot: f32,
    rotation: Mat4,
    at: Vec3,
    segments: u32,
    slice: (f32, f32),
    step: f32,
    cap_min: f32,
) {
    let step = if step > 0.0 { step } else { (slice.1 - slice.0).max(0.0) };
    for (bi, (verts, indices, texture)) in part.batches.iter().enumerate() {
        let tex = part.texels.get(bi);
        let mut mesh = Mesh {
            vertices: Vec::with_capacity(verts.len()),
            indices: Vec::new(),
            texture: texture.clone(),
        };
        let mut push = |v: &Vert, offset: f32, upper: bool| {
            let p = skinned(part, joints, v) + Vec3::X * offset;
            let p = if upper {
                neck.transform_point3(p)
            } else {
                base.transform_point3(p)
            };
            let p = rotation.transform_point3(p) + at;
            let i = mesh.vertices.len() as u16;
            mesh.vertices.push(Vertex::new(
                p.x,
                p.y,
                p.z,
                v.uv[0],
                v.uv[1],
                Color::from_rgba(v.color[0], v.color[1], v.color[2], 255),
            ));
            mesh.indices.push(i);
        };
        for tri in indices.chunks_exact(3) {
            let (a, b, c) = (
                &verts[tri[0] as usize],
                &verts[tri[1] as usize],
                &verts[tri[2] as usize],
            );
            let (pa, pb, pc) = (
                skinned(part, joints, a),
                skinned(part, joints, b),
                skinned(part, joints, c),
            );
            // Band planes are authored in the body's local space; use the bind x.
            let in_collar = |v: &Vert| v.p.x >= 40.0;
            // The cap test uses the posed x, which is what matched it two versions ago.
            let cap_region = pa.x > 52.0 && pb.x > 52.0 && pc.x > 52.0;
            let yellow = cap_region
                && tex.is_some_and(|t| {
                    is_yellow(t, a.uv) && is_yellow(t, b.uv) && is_yellow(t, c.uv)
                });
            let blue = tex.is_some_and(|t| {
                is_blue(t, a.uv) && is_blue(t, b.uv) && is_blue(t, c.uv)
            });
            // Everything above the pack top leans as one assembly.
            let upper = (a.p.x > pivot && a.p.y > -10.0)
                || (b.p.x > pivot && b.p.y > -10.0)
                || (c.p.x > pivot && c.p.y > -10.0);
            if yellow {
                // Cap: once, on top of the highest blue ring. No extra lift.
                let offset = step * segments as f32;
                for v in [a, b, c] {
                    push(v, offset, upper);
                }
                continue;
            }
            // Any vertex in the collar selects the whole triangle: the dome's outer
            // triangles must not be split at the band's top edge.
            let repeats = if blue && (in_collar(a) || in_collar(b) || in_collar(c)) {
                segments + 1
            } else {
                1
            };
            for n in 0..repeats {
                let offset = step * n as f32;
                push(a, offset, upper);
                push(b, offset, upper);
                push(c, offset, upper);
            }
        }
        draw_mesh(&mut mesh);
    }
}

fn draw_part(part: &Part, joints: &[Mat4], base: Mat4, rotation: Mat4, at: Vec3) {
    for (verts, indices, texture) in &part.batches {
        // The nozzle's shell is authored single-sided; draw both windings so a
        // reversed face does not punch holes through the drum.
        let mut both = indices.clone();
        for t in indices.chunks_exact(3) {
            both.extend_from_slice(&[t[0], t[2], t[1]]);
        }
        let mut mesh = Mesh {
            vertices: verts
                .iter()
                .map(|v| {
                    let p = skinned(part, joints, v);
                    let p = base.transform_point3(p);
                    let p = rotation.transform_point3(p) + at;
                    Vertex::new(
                p.x,
                p.y,
                p.z,
                v.uv[0],
                v.uv[1],
                Color::from_rgba(v.color[0], v.color[1], v.color[2], 255),
            )
                })
                .collect(),
            indices: both,
            texture: texture.clone(),
        };
        draw_mesh(&mut mesh);
    }
}

























