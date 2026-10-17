//! lb/lbanim.h FigaTree/FigaTrack, baselib/fobj.c packed channels.
//! A bounded keyframe reader and Hermite sampler; gameplay commands are separate.
use crate::{
    Result,
    dat::Dat,
    model::{Model, local},
};
use glam::Mat4;
use std::{collections::BTreeMap, path::Path};
#[derive(Clone, Debug)]
pub struct Key {
    pub time: f32,
    pub value: f32,
    pub slope: f32,
    pub op: u8,
}
#[derive(Clone, Debug)]
pub struct Track {
    pub joint: usize,
    pub channel: u8,
    pub keys: Vec<Key>,
}
#[derive(Clone, Debug)]
pub struct Clip {
    pub frames: f32,
    pub tracks: Vec<Track>,
}
pub struct Animations {
    pub clips: BTreeMap<String, Clip>,
}
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn byte(&mut self) -> Result<u8> {
        let b = *self.data.get(self.at).ok_or("truncated animation stream")?;
        self.at += 1;
        Ok(b)
    }
    fn var(&mut self) -> Result<u32> {
        let mut n = 0;
        for shift in (0..28).step_by(7) {
            let b = self.byte()?;
            n |= ((b & 127) as u32) << shift;
            if b & 128 == 0 {
                return Ok(n);
            }
        }
        Err("animation varint overflow".into())
    }
    fn value(&mut self, frac: u8) -> Result<f32> {
        let n = match frac & 0xe0 {
            0 => {
                if frac != 0 {
                    return Err("bad float fraction".into());
                }
                f32::from_le_bytes([self.byte()?, self.byte()?, self.byte()?, self.byte()?])
            }
            0x20 => i16::from_le_bytes([self.byte()?, self.byte()?]) as f32,
            0x40 => u16::from_le_bytes([self.byte()?, self.byte()?]) as f32,
            0x60 => (self.byte()? as i8) as f32,
            0x80 => self.byte()? as f32,
            _ => return Err("bad animation scalar format".into()),
        };
        if !n.is_finite() {
            return Err("nonfinite animation value".into());
        }
        Ok(if frac == 0 {
            n
        } else {
            n / 2f32.powi((frac & 31) as i32)
        })
    }
}
fn keys(bytes: &[u8], value: u8, slope: u8, start: f32) -> Result<Vec<Key>> {
    let mut r = Reader { data: bytes, at: 0 };
    let mut keys: Vec<Key> = Vec::new();
    let mut time = -start;
    while r.at < r.data.len() {
        let head = r.byte()?;
        let op = head & 15;
        if !(1..=6).contains(&op) {
            return Err("unsupported animation opcode".into());
        }
        let mut count = ((head >> 4) & 7) as u32 + 1;
        if head & 128 != 0 {
            count = count
                .checked_add(r.var()?.checked_mul(8).ok_or("animation pack overflow")?)
                .ok_or("animation pack overflow")?;
        }
        if count > 100000 {
            return Err("animation pack exceeds budget".into());
        }
        for _ in 0..count {
            if op == 5 {
                let s = r.value(slope)?;
                if let Some(k) = keys.last_mut() {
                    k.slope = s;
                }
                continue;
            }
            let v = r.value(value)?;
            let s = if op == 4 { r.value(slope)? } else { 0. };
            keys.push(Key {
                time,
                value: v,
                slope: s,
                op,
            });
            if keys.len() > 100000 {
                return Err("animation key budget exceeded".into());
            }
            // Last value may omit its wait; there is no following interval.
            if r.at < r.data.len() {
                time += r.var()? as f32;
            }
        }
    }
    Ok(keys)
}
impl Track {
    pub fn sample(&self, frame: f32) -> Option<f32> {
        let first = self.keys.first()?;
        if frame <= first.time {
            return Some(first.value);
        }
        let upper = self.keys.partition_point(|k| k.time <= frame);
        let a = &self.keys[upper - 1];
        let Some(b) = self.keys.get(upper) else {
            return Some(a.value);
        };
        let dt = b.time - a.time;
        if dt <= 0. {
            return Some(b.value);
        }
        let t = ((frame - a.time) / dt).clamp(0., 1.);
        Some(match a.op {
            1 | 6 => a.value,
            2 => a.value + (b.value - a.value) * t,
            _ => {
                let t2 = t * t;
                let t3 = t2 * t;
                (2. * t3 - 3. * t2 + 1.) * a.value
                    + (t3 - 2. * t2 + t) * dt * a.slope
                    + (-2. * t3 + 3. * t2) * b.value
                    + (t3 - t2) * dt * b.slope
            }
        })
    }
}
impl Animations {
    pub fn load(dir: &Path) -> Result<Self> {
        Self::parse(&std::fs::read(dir.join("PlSsAJ.dat"))?)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut at = 0;
        let mut clips = BTreeMap::new();
        while at + 32 <= bytes.len() {
            let size = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
            if size == 0 && bytes[at..].iter().all(|&b| b == 0) {
                break;
            }
            if size < 32 {
                return Err("bad packed animation size".into());
            }
            let d = Dat::parse(
                bytes
                    .get(at..at.checked_add(size).ok_or("animation size overflow")?)
                    .ok_or("packed animation truncated")?,
            )?;
            for (name, &root) in &d.symbols {
                let Some(name) = name
                    .split("_ACTION_")
                    .nth(1)
                    .and_then(|s| s.strip_suffix("_figatree"))
                else {
                    continue;
                };
                let frames = d.f32(root + 8)?;
                if !(0. ..=100000.).contains(&frames) {
                    return Err("bad clip length".into());
                }
                let nodes = d.required_ptr(root + 12)?;
                let table = d.required_ptr(root + 16)?;
                let mut track_index = 0;
                let mut tracks = Vec::new();
                for joint in 0..2048 {
                    let count = d.bytes(nodes + joint, 1)?[0];
                    if count == 255 {
                        break;
                    }
                    if count > 64 {
                        return Err("too many joint tracks".into());
                    }
                    for _ in 0..count {
                        let t = table + track_index * 12;
                        track_index += 1;
                        let length = d.u16(t)? as usize;
                        let raw = d.bytes(d.required_ptr(t + 8)?, length)?;
                        let frac = d.bytes(t + 5, 2)?;
                        tracks.push(Track {
                            joint,
                            channel: d.bytes(t + 4, 1)?[0],
                            keys: keys(raw, frac[0], frac[1], d.u16(t + 2)? as i16 as f32)?,
                        });
                    }
                }
                clips.insert(name.to_owned(), Clip { frames, tracks });
            }
            at = (at + size + 31) & !31;
        }
        if clips.is_empty() {
            return Err("no Samus animation clips".into());
        }
        Ok(Self { clips })
    }
    pub fn pose(&self, model: &Model, name: &str, frame: f32, looping: bool) -> Vec<Mat4> {
        let mut t: Vec<_> = model.joints.iter().map(|j| j.translation).collect();
        let mut r: Vec<_> = model.joints.iter().map(|j| j.rotation).collect();
        let mut s: Vec<_> = model.joints.iter().map(|j| j.scale).collect();
        if let Some(clip) = self.clips.get(name) {
            let frame = if looping {
                frame.rem_euclid(clip.frames.max(1.))
            } else {
                frame.min(clip.frames)
            };
            for track in &clip.tracks {
                let j = track.joint;
                if j >= model.joints.len() {
                    continue;
                }
                let Some(v) = track.sample(frame) else {
                    continue;
                };
                match track.channel {
                    1 => r[j].x = v,
                    2 => r[j].y = v,
                    3 => r[j].z = v,
                    5 => t[j].x = v,
                    6 => t[j].y = v,
                    7 => t[j].z = v,
                    // HSD_JObjUpdateFunc clamps near-zero animated scale.
                    8 => s[j].x = if v.abs() < 0.001 { 0.001 } else { v },
                    9 => s[j].y = if v.abs() < 0.001 { 0.001 } else { v },
                    10 => s[j].z = if v.abs() < 0.001 { 0.001 } else { v },
                    _ => (),
                }
            }
        }
        // Root translation is movement, already owned by the gameplay sim.
        if !t.is_empty() {
            t[0] = model.joints[0].translation;
        }
        let mut world = Vec::with_capacity(model.joints.len());
        for (i, j) in model.joints.iter().enumerate() {
            let parent = j.parent.map(|p| world[p]).unwrap_or(Mat4::IDENTITY);
            world.push(parent * local(t[i], r[i], s[i]));
        }
        world
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packed_little_endian_values_and_interpolation() {
        let mut b = vec![0x12];
        b.extend(2f32.to_le_bytes());
        b.push(10);
        b.extend(4f32.to_le_bytes());
        b.push(0);
        let k = keys(&b, 0, 0, 0.).unwrap();
        let t = Track {
            joint: 0,
            channel: 1,
            keys: k,
        };
        assert_eq!(t.sample(5.), Some(3.));
        assert!(keys(&[0x84, 255], 0, 0, 0.).is_err());
    }
}
