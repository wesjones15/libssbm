//! Sunshine's rig reader and a shared-joint retarget onto a Melee fighter, for hosts that
//! want to try an SMS clip on an HSD skeleton.
//!
//! The clip side is J3D (`JNT1`/`INF1` + `ANK1`), which libssbm's HSD readers do not cover;
//! the target side is the same [`Model`] the viewer and hosts already draw. Nothing here is
//! used by the fighter simulation.
//!
//! Mario's motion is rotation-only: every `ma_*.bck` sampled has constant root translation.
//! Movement in Sunshine comes from the game code, so a host still owns root motion.
use crate::{model::Model, Result};
use glam::{EulerRot, Mat3, Mat4, Quat, Vec3};
use libcommon::anim::hermite;
use std::path::Path;

fn be_u16(d: &[u8], o: usize) -> Option<usize> {
    Some(u16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize)
}
fn be_i16(d: &[u8], o: usize) -> Option<i16> {
    Some(i16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn be_u32(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}
fn be_f32(d: &[u8], o: usize) -> Option<f32> {
    Some(f32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Bind transform of one joint.
#[derive(Clone, Copy, Debug)]
pub struct Srt {
    pub r: Vec3,
    pub t: Vec3,
    pub s: Vec3,
}

impl Srt {
    pub fn rot(&self) -> Mat3 {
        Mat3::from_quat(Quat::from_euler(EulerRot::ZYX, self.r.z, self.r.y, self.r.x))
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.s,
            Quat::from_euler(EulerRot::ZYX, self.r.z, self.r.y, self.r.x),
            self.t,
        )
    }
}

pub struct Rig {
    pub names: Vec<String>,
    /// Parent joint index; `None` for the root.
    pub parents: Vec<Option<usize>>,
    pub bind: Vec<Srt>,
}

impl Rig {
    pub fn find(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// `JNT1` bind rotations and names, parents from the `INF1` scene graph.
    pub fn from_bmd(d: &[u8]) -> Result<Rig> {
        if d.get(0..8) != Some(b"J3D2bmd3") {
            return Err("not a J3D2bmd3 model".into());
        }
        let chunks = be_u32(d, 12).ok_or("truncated BMD")?;
        let (mut jnt1, mut inf1) = (None, None);
        let mut o = 32;
        for _ in 0..chunks {
            let tag = d.get(o..o + 4).ok_or("truncated BMD chunk")?;
            let size = be_u32(d, o + 4).ok_or("truncated BMD chunk")?;
            match tag {
                b"JNT1" => jnt1 = Some(o),
                b"INF1" => inf1 = Some(o),
                _ => {}
            }
            o += size.max(8);
        }
        let (jnt1, inf1) = (jnt1.ok_or("no JNT1")?, inf1.ok_or("no INF1")?);

        let count = be_u16(d, jnt1 + 8).ok_or("truncated JNT1")?;
        let entries = jnt1 + be_u32(d, jnt1 + 12).ok_or("truncated JNT1")?;
        let remap = jnt1 + be_u32(d, jnt1 + 16).ok_or("truncated JNT1")?;
        let names_at = jnt1 + be_u32(d, jnt1 + 20).ok_or("truncated JNT1")?;
        let n_names = be_u16(d, names_at).ok_or("truncated JNT1 names")?;
        let mut names = Vec::with_capacity(count);
        for i in 0..n_names.min(count) {
            let off = be_u16(d, names_at + 6 + i * 4).ok_or("truncated JNT1 names")?;
            let s = d.get(names_at + off..).ok_or("JNT1 name outside file")?;
            let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
            names.push(String::from_utf8_lossy(&s[..end]).into_owned());
        }
        names.resize(count, String::new());

        let k = std::f32::consts::PI / 32768.0;
        let mut bind = Vec::with_capacity(count);
        for i in 0..count {
            let e = entries + be_u16(d, remap + i * 2).ok_or("truncated JNT1 remap")? * 0x40;
            bind.push(Srt {
                r: Vec3::new(
                    be_i16(d, e + 16).ok_or("truncated JNT1 entry")? as f32 * k,
                    be_i16(d, e + 18).ok_or("truncated JNT1 entry")? as f32 * k,
                    be_i16(d, e + 20).ok_or("truncated JNT1 entry")? as f32 * k,
                ),
                t: Vec3::new(
                    be_f32(d, e + 24).ok_or("truncated JNT1 entry")?,
                    be_f32(d, e + 28).ok_or("truncated JNT1 entry")?,
                    be_f32(d, e + 32).ok_or("truncated JNT1 entry")?,
                ),
                s: Vec3::new(
                    be_f32(d, e + 4).ok_or("truncated JNT1 entry")?,
                    be_f32(d, e + 8).ok_or("truncated JNT1 entry")?,
                    be_f32(d, e + 12).ok_or("truncated JNT1 entry")?,
                ),
            });
        }

        // 0x10 joint, 0x01 open, 0x02 close.
        let mut h = inf1 + be_u32(d, inf1 + 20).ok_or("truncated INF1")?;
        let mut parents = vec![None; count];
        let mut stack: Vec<Option<usize>> = Vec::new();
        let mut ctx = None;
        loop {
            let t = be_u16(d, h).ok_or("truncated INF1")?;
            let i = be_u16(d, h + 2).ok_or("truncated INF1")?;
            h += 4;
            match t {
                0 => break,
                1 => stack.push(ctx),
                2 => {
                    stack.pop();
                }
                0x10 => {
                    if i < count {
                        parents[i] = stack.last().copied().flatten();
                    }
                    ctx = Some(i);
                }
                _ => ctx = stack.last().copied().flatten(),
            }
        }
        Ok(Rig {
            names,
            parents,
            bind,
        })
    }

    pub fn from_file(path: &Path) -> Result<Rig> {
        Rig::from_bmd(&std::fs::read(path)?)
    }
}

/// One rotation channel: Hermite keys `(time, value, tangent_in, tangent_out)`.
pub struct Track {
    keys: Vec<(f32, f32, f32, f32)>,
}

impl Track {
    pub fn constant(value: f32) -> Track {
        Track {
            keys: vec![(0., value, 0., 0.)],
        }
    }

    pub fn sample(&self, t: f32) -> f32 {
        let k = &self.keys;
        if k.is_empty() {
            return 0.0;
        }
        if k.len() == 1 || t <= k[0].0 {
            return k[0].1;
        }
        if t >= k[k.len() - 1].0 {
            return k[k.len() - 1].1;
        }
        let i = (0..k.len() - 1).rev().find(|&i| k[i].0 <= t).unwrap();
        let (a, b) = (k[i], k[i + 1]);
        let dt = b.0 - a.0;
        if dt <= 0.0 {
            return a.1;
        }
        hermite(t, a.0, a.1, a.3, b.0, b.1, b.2)
    }
}

pub struct Clip {
    pub frames: f32,
    /// Per joint, X/Y/Z rotation.
    pub rot: Vec<[Track; 3]>,
}

impl Clip {
    /// `ANK1` rotation tracks from a `.bck`.
    pub fn from_bck(d: &[u8]) -> Result<Clip> {
        if d.get(0..8) != Some(b"J3D1bck1") {
            return Err("not a J3D1bck1 clip".into());
        }
        let a = 0x20;
        if d.get(a..a + 4) != Some(b"ANK1") {
            return Err("no ANK1".into());
        }
        let frames = be_u16(d, a + 10).ok_or("truncated ANK1")? as f32;
        let joints = be_u16(d, a + 12).ok_or("truncated ANK1")?;
        let ang = *d.get(a + 9).ok_or("truncated ANK1")? as i8;
        let scale = (std::f32::consts::PI / 32768.0) * 2f32.powi(ang as i32);
        let joints_at = a + be_u32(d, a + 20).ok_or("truncated ANK1")?;
        let rot_at = a + be_u32(d, a + 28).ok_or("truncated ANK1")?;

        let mut rot = Vec::with_capacity(joints);
        for j in 0..joints {
            let mut axes = Vec::with_capacity(3);
            for ax in 0..3 {
                let e = joints_at + j * 54 + (ax * 3 + 1) * 6;
                let count = be_u16(d, e).ok_or("truncated ANK1 track")?;
                let index = be_u16(d, e + 2).ok_or("truncated ANK1 track")?;
                let tangent = be_u16(d, e + 4).ok_or("truncated ANK1 track")?;
                let mut keys = Vec::new();
                if count <= 1 {
                    // A constant channel stores only its value at `index`.
                    let v = be_i16(d, rot_at + 2 * index).ok_or("truncated ANK1 keys")?;
                    keys.push((0.0, v as f32 * scale, 0.0, 0.0));
                } else {
                    let stride = if tangent == 0 { 3 } else { 4 };
                    for k in 0..count {
                        let i = index + k * stride;
                        let t = be_i16(d, rot_at + 2 * i).ok_or("truncated ANK1 keys")? as f32;
                        let v = be_i16(d, rot_at + 2 * (i + 1)).ok_or("truncated ANK1 keys")?
                            as f32
                            * scale;
                        let tin = be_i16(d, rot_at + 2 * (i + 2)).ok_or("truncated ANK1 keys")?
                            as f32
                            * scale;
                        let tout = if tangent == 0 {
                            tin
                        } else {
                            be_i16(d, rot_at + 2 * (i + 3)).ok_or("truncated ANK1 keys")? as f32
                                * scale
                        };
                        keys.push((t, v, tin, tout));
                    }
                }
                axes.push(Track { keys });
            }
            rot.push(axes.try_into().map_err(|_| "bad ANK1 axes")?);
        }
        Ok(Clip { frames, rot })
    }

    pub fn from_file(path: &Path) -> Result<Clip> {
        Clip::from_bck(&std::fs::read(path)?)
    }
}

pub fn bind_world(rig: &Rig) -> Vec<Mat3> {
    let mut out = Vec::with_capacity(rig.bind.len());
    for (i, b) in rig.bind.iter().enumerate() {
        let r = b.rot();
        out.push(match rig.parents[i] {
            Some(p) => out[p] * r,
            None => r,
        });
    }
    out
}

pub fn clip_world(rig: &Rig, clip: &Clip, frame: f32) -> Vec<Mat3> {
    let mut out = Vec::with_capacity(rig.bind.len());
    for i in 0..rig.bind.len().min(clip.rot.len()) {
        let t = &clip.rot[i];
        let r = Mat3::from_quat(Quat::from_euler(
            EulerRot::ZYX,
            t[2].sample(frame),
            t[1].sample(frame),
            t[0].sample(frame),
        ));
        out.push(match rig.parents[i] {
            Some(p) => out[p] * r,
            None => r,
        });
    }
    out
}

/// Full bind matrices (translation included) for a J3D rig.
pub fn bind_matrices(rig: &Rig) -> Vec<Mat4> {
    let mut out = Vec::with_capacity(rig.bind.len());
    for (i, b) in rig.bind.iter().enumerate() {
        let local = b.matrix();
        out.push(match rig.parents[i] {
            Some(p) => out[p] * local,
            None => local,
        });
    }
    out
}

/// Animation world matrices. SMS clips do not animate translation, so each joint keeps
/// its bind position and takes the clip's world rotation.
pub fn clip_world_mats(rig: &Rig, clip: &Clip, frame: f32) -> Vec<Mat4> {
    let bind = bind_matrices(rig);
    let rot = clip_world(rig, clip, frame);
    (0..rot.len())
        .map(|i| {
            Mat4::from_rotation_translation(
                Quat::from_mat3(&rot[i]),
                bind.get(i).map_or(Vec3::ZERO, |m| m.w_axis.truncate()),
            )
        })
        .collect()
}


/// Shared deforming joints, Mario -> Luigi. Luigi's indices come from `PlCo.dat`'s
/// FighterPartsTable (`HipN=4`) and the model's own joint order.
pub const MAP: &[(usize, usize)] = &[
    (1, 4),   // center      -> HipN
    (14, 5),  // jnt_chest   -> WaistN (Luigi has no separate BustN)
    (26, 22), // jnt_head    -> HeadN
    // right arm: sldr, arm1, arm2, hand -> clavicle, upper arm, elbow, wrist
    (15, 29),
    (16, 30),
    (17, 31),
    (18, 32),
    // left arm
    (20, 6),
    (22, 8),
    (23, 9),
    (24, 10),
    // right leg: thigh, shin, ankle, foot -> Luigi's R side (-X)
    (3, 53),
    (4, 55),
    (5, 56),
    (6, 57),
    // left leg -> Luigi's L side (+X)
    (8, 47),
    (9, 49),
    (10, 50),
    (11, 51),
];

/// A retarget prepared once for one Melee model: SRT bind rotations on both sides.
pub struct Retarget {
    sms_bind: Vec<Mat3>,
    lg_bind: Vec<Mat3>,
}

impl Retarget {
    /// Source bind world rotations (e.g. Mario's `jnt_chest` frame for FLUDD).
    pub fn source_bind(&self) -> &[Mat3] {
        &self.sms_bind
    }
    /// Target bind world rotations (e.g. Luigi's `WaistN` frame).
    pub fn target_bind(&self) -> &[Mat3] {
        &self.lg_bind
    }

    pub fn new(model: &Model, rig: &Rig) -> Retarget {
        let lg_bind = {
            let mut out = Vec::with_capacity(model.joints.len());
            for j in &model.joints {
                let local = Mat3::from_quat(Quat::from_euler(
                    EulerRot::ZYX,
                    j.rotation.z,
                    j.rotation.y,
                    j.rotation.x,
                ));
                out.push(match j.parent {
                    Some(p) => out[p] * local,
                    None => local,
                });
            }
            out
        };
        Retarget {
            sms_bind: bind_world(rig),
            lg_bind,
        }
    }

    /// The Melee model's world matrices for `frame` of `clip`: mapped joints get the
    /// source's world rotation delta, every other joint keeps its bind local rotation.
    pub fn pose(&self, model: &Model, rig: &Rig, clip: &Clip, frame: f32) -> Vec<Mat4> {
        let sms_world = clip_world(rig, clip, frame);
        let n = model.joints.len();
        let mut mapped = vec![None; n];
        for &(s, m) in MAP {
            if s >= sms_world.len() || m >= n {
                continue;
            }
            let delta = sms_world[s] * self.sms_bind[s].transpose();
            mapped[m] = Some(delta * self.lg_bind[m]);
        }
        let mut world = vec![Mat3::IDENTITY; n];
        for i in 0..n {
            let p = model.joints[i].parent;
            world[i] = match mapped[i] {
                Some(r) => r,
                None => {
                    let local = Quat::from_euler(
                        EulerRot::ZYX,
                        model.joints[i].rotation.z,
                        model.joints[i].rotation.y,
                        model.joints[i].rotation.x,
                    );
                    match p {
                        Some(p) => world[p] * Mat3::from_quat(local),
                        None => Mat3::from_quat(local),
                    }
                }
            };
        }

        let mut out = vec![Mat4::IDENTITY; n];
        for i in 0..n {
            let p = model.joints[i].parent;
            let q = match p {
                Some(p) => Quat::from_mat3(&(world[p].transpose() * world[i])),
                None => Quat::from_mat3(&world[i]),
            };
            let local = Mat4::from_scale_rotation_translation(
                model.joints[i].scale,
                q,
                model.joints[i].translation,
            );
            out[i] = match p {
                Some(p) => out[p] * local,
                None => local,
            };
        }
        out
    }
}

/// Drop or lift the whole pose so the lowest skinned vertex sits on Y=0. The SMS clips are
/// rotation-only (the game moves the root), so without this the pelvis stays at bind height
/// and the feet bounce through the floor. Applied after [`Retarget::pose`].
pub fn ground(model: &Model, world: &mut [Mat4]) {
    crate::presentation::ground(model, world);
}



#[cfg(test)]
mod hermite_baseline {
    use super::*;

    /// Sampled bits over random keys and times outside the range, pinned before the curve
    /// moved to libcommon.
    #[test]
    fn track_samples_match_baseline() {
        let mut s = 0x5A5A_1234u32;
        let mut next = || {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 8) as f32 / (1u32 << 24) as f32
        };
        let keys = [0.0f32, 4.0, 9.0, 15.0, 22.0, 30.0]
            .iter()
            .map(|&t| (t, next() * 8. - 4., next() * 2. - 1., next() * 2. - 1.))
            .collect();
        let track = Track { keys };
        let mut hash = 0xcbf29ce484222325u64;
        for step in 0..100 {
            let v = track.sample(-2.0 + step as f32 * 0.37);
            hash = (hash ^ v.to_bits() as u64).wrapping_mul(0x100000001b3);
        }
        println!("{hash:#x}");
        assert_eq!(hash, BASELINE);
    }

    // Recorded from the curve before it moved to libcommon::anim.
    const BASELINE: u64 = 0xc9ae9d3adf61b2a9;
}
