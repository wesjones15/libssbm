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
    pub looping: bool,
    pub tracks: Vec<Track>,
    /// TransN is consumed by fighter physics when animCurrFlags bit 31 is set.
    pub motion_joint: Option<usize>,
}
#[derive(Clone)]
pub struct Animations {
    pub clips: BTreeMap<String, Clip>,
}
/// A part animation laid over the main pose (ftAnim_ApplyPartAnim): the clip, how far into it
/// the pose is, and how much of it replaces the main pose's value (1 once the blend-in ends).
#[derive(Clone, Copy)]
pub struct Overlay<'a> {
    pub clip: &'a Clip,
    pub frame: f32,
    pub weight: f32,
}
/// The fighter's part animations (`ftData_x1C`, ft/types.h): five slots (hands, ears, tail,
/// mouth...), each a list of animations that script command 41 picks by index. A clip's tracks
/// already name model joints: the animation's joint tree walks the rig from the slot's start
/// bone in the same child-then-next order the model's joints are numbered in.
#[derive(Clone, Default)]
pub struct PartAnimations {
    pub slots: Vec<Vec<Clip>>,
}
impl PartAnimations {
    pub fn get(&self, slot: usize, index: usize) -> Option<&Clip> {
        self.slots.get(slot)?.get(index)
    }
    pub fn load(dir: &Path, fighter: &crate::fighter::FighterAssets<'_>) -> Result<Self> {
        let bytes = std::fs::read(dir.join(fighter.data.file))?;
        let d = Dat::parse(&bytes)?;
        Self::parse(&d, d.root(fighter.data.root)?)
    }
    /// Reads the table at the fighter data's +0x1C. The animation arrays are not counted, so a
    /// slot's list ends at the first entry that is not a valid animation tree.
    pub fn parse(d: &Dat, root: usize) -> Result<Self> {
        let Some(table) = d.ptr(root + 0x1c)? else { return Ok(Self::default()) };
        let mut slots = Vec::new();
        for slot in 0..5 {
            let mut clips = Vec::new();
            if let Ok(Some(entry)) = d.ptr(table + slot * 4)
                && let (Ok(start), Ok(Some(anims))) = (d.u16(entry), d.ptr(entry + 8))
            {
                for i in 0..16 {
                    let Ok(Some(tree)) = d.ptr(anims + i * 4) else { break };
                    match Self::tree(d, tree, start as usize) {
                        Ok(clip) => clips.push(clip),
                        Err(_) => break,
                    }
                }
            }
            slots.push(clips);
        }
        Ok(Self { slots })
    }
    /// ftAnim_80070904: the animation joint tree (child first, then next) from the start bone.
    fn tree(d: &Dat, root: usize, start: usize) -> Result<Clip> {
        let mut tracks = Vec::new();
        let mut frames = 0f32;
        let mut looping = false;
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![root];
        let mut index = 0;
        while let Some(at) = stack.pop() {
            if !seen.insert(at) || seen.len() > 512 {
                return Err("cyclic/oversized part animation tree".into());
            }
            if let Some(next) = d.ptr(at + 4)? {
                stack.push(next);
            }
            if let Some(child) = d.ptr(at)? {
                stack.push(child);
            }
            if let Some(aobj) = d.ptr(at + 8)? {
                let end = d.f32(aobj + 4)?;
                if !(0. ..=100000.).contains(&end) {
                    return Err("invalid part animation length".into());
                }
                frames = frames.max(end);
                looping |= d.u32(aobj)? & (1 << 29) != 0;
                let mut fobj = d.ptr(aobj + 8)?;
                let mut visited = std::collections::HashSet::new();
                while let Some(f) = fobj {
                    if !visited.insert(f) || visited.len() > 64 {
                        return Err("cyclic/oversized part tracks".into());
                    }
                    let raw = d.bytes(d.required_ptr(f + 16)?, d.u32(f + 4)? as usize)?;
                    let frac = d.bytes(f + 13, 2)?;
                    tracks.push(Track {
                        joint: start + index,
                        channel: d.bytes(f + 12, 1)?[0],
                        keys: keys(raw, frac[0], frac[1], d.f32(f + 8)?)?,
                    });
                    fobj = d.ptr(f)?;
                }
            }
            index += 1;
        }
        Ok(Clip { frames, looping, tracks, motion_joint: None })
    }
}

/// How much of a part animation shows `age` ticks after it started (ftAnim_800707B0): with
/// no blend frames it is all of it at once; otherwise each tick moves the display toward the
/// animation by `step / (step + remaining)`, where the step is the animation's length over the
/// blend frames and `remaining` counts down to the blend frame count. Returns the combined
/// weight of the ticks so far.
pub fn part_blend(age: f32, blend_frames: f32, clip_frames: f32) -> f32 {
    if blend_frames <= 0. {
        return 1.;
    }
    let step = clip_frames / blend_frames;
    if step <= 0. {
        return 0.;
    }
    let mut kept = 1f32;
    let mut done = 0f32;
    for _ in 0..age.max(0.) as usize {
        done += step;
        if done >= blend_frames {
            return 1.;
        }
        kept *= 1. - step / (step + (blend_frames - done));
    }
    1. - kept
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
pub(crate) fn keys(bytes: &[u8], value: u8, slope: u8, start: f32) -> Result<Vec<Key>> {
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
/// Maps a disc clip name to the canonical name the sim uses via the fighter's
/// alias list (e.g. "Wait1" -> "Wait"). Unaliased names pass through unchanged.
pub(crate) fn canonical<'n>(
    aliases: &[(&str, &str)],
    name: &'n str,
) -> std::borrow::Cow<'n, str> {
    match aliases.iter().find(|(_, actual)| *actual == name) {
        Some((canonical, _)) => std::borrow::Cow::Owned((*canonical).to_owned()),
        None => std::borrow::Cow::Borrowed(name),
    }
}
impl Animations {
    /// MatAnimJoint -> per-DObj material AObj tracks. Material channel definitions
    /// and application are specified by mobj.h/HSD_MObjUpdateFunc.
    pub fn materials(d: &Dat, root: Option<usize>, model: &mut Model) -> Result<Vec<Track>> {
        let mut result = Vec::new();
        let mut stack = root.map(|p| vec![p]).unwrap_or_default();
        let mut joint = 0;
        let mut seen = std::collections::HashSet::new();
        while let Some(at) = stack.pop() {
            if !seen.insert(at) || seen.len() > 2048 {
                return Err("cyclic material animation tree".into());
            }
            if let Some(next) = d.ptr(at + 4)? {
                stack.push(next);
            }
            if let Some(child) = d.ptr(at)? {
                stack.push(child);
            }
            let batches: Vec<_> = model
                .batches
                .iter()
                .enumerate()
                .filter(|(_, b)| b.owner == joint && b.instance.is_none())
                .map(|(i, _)| i)
                .collect();
            let mut material = d.ptr(at + 8)?;
            let mut index = 0;
            let mut visited = std::collections::HashSet::new();
            while let Some(m) = material {
                if !visited.insert(m) || visited.len() > 4096 {
                    return Err("cyclic material animation chain".into());
                }
                if let (Some(&batch_index), Some(aobj)) = (batches.get(index), d.ptr(m + 4)?) {
                    let batch = &model.batches[batch_index];
                    let mut fobj = d.ptr(aobj + 8)?;
                    let mut tracks = std::collections::HashSet::new();
                    while let Some(f) = fobj {
                        if !tracks.insert(f) || tracks.len() > 64 {
                            return Err("cyclic material tracks".into());
                        }
                        let frac = d.bytes(f + 13, 2)?;
                        result.push(Track {
                            joint: batch.material_dobj,
                            channel: d.bytes(f + 12, 1)?[0],
                            keys: keys(
                                d.bytes(d.required_ptr(f + 16)?, d.u32(f + 4)? as usize)?,
                                frac[0],
                                frac[1],
                                d.f32(f + 8)?,
                            )?,
                        });
                        fobj = d.ptr(f)?;
                    }
                }
                // HSD_MObjAddAnim delegates the separate TexAnim/AObj chain to
                // HSD_TObjAddAnimAll. Screw Attack fades via TObj blending, not
                // material alpha (which remains zero throughout the effect).
                if let (Some(&batch_index), Some(texanim)) = (batches.get(index), d.ptr(m + 8)?) {
                    let batch = &mut model.batches[batch_index];
                    let mut tex = Some(texanim);
                    let mut visited_tex = std::collections::HashSet::new();
                    while let Some(texanim) = tex {
                        if !visited_tex.insert(texanim) || visited_tex.len() > 16 {
                            return Err("cyclic texture animation chain".into());
                        }
                        let texture_id = d.u32(texanim + 4)?;
                        if batch.texture_transform.is_some_and(|t| t.id != texture_id) {
                            tex = d.ptr(texanim)?;
                            continue;
                        }
                        if let (Some(images), Some(base)) =
                            (d.ptr(texanim + 12)?, batch.texture.as_ref())
                        {
                            let count = d.u16(texanim + 20)? as usize;
                            if count > 256 {
                                return Err("texture animation frame budget exceeded".into());
                            }
                            let palette = d
                                .ptr(texanim + 16)?
                                .map(|table| d.ptr(table))
                                .transpose()?
                                .flatten();
                            batch.texture_frames = (0..count)
                                .map(|i| {
                                    crate::model::texture_frame(
                                        d,
                                        d.required_ptr(images + i * 4)?,
                                        palette,
                                        base,
                                    )
                                })
                                .collect::<Result<_>>()?;
                        }
                        if let Some(aobj) = d.ptr(texanim + 8)? {
                            let mut fobj = d.ptr(aobj + 8)?;
                            let mut tracks = std::collections::HashSet::new();
                            while let Some(f) = fobj {
                                if !tracks.insert(f) || tracks.len() > 64 {
                                    return Err("cyclic texture tracks".into());
                                }
                                let channel = d.bytes(f + 12, 1)?[0];
                                if (1..=9).contains(&channel) {
                                    let frac = d.bytes(f + 13, 2)?;
                                    result.push(Track {
                                        joint: batch.material_dobj,
                                        channel: 100 + channel,
                                        keys: keys(
                                            d.bytes(
                                                d.required_ptr(f + 16)?,
                                                d.u32(f + 4)? as usize,
                                            )?,
                                            frac[0],
                                            frac[1],
                                            d.f32(f + 8)?,
                                        )?,
                                    });
                                }
                                fobj = d.ptr(f)?;
                            }
                        }
                        tex = d.ptr(texanim)?;
                    }
                }
                material = d.ptr(m)?;
                index += 1;
            }
            joint += 1;
        }
        Ok(result)
    }
    /// HSD_AnimJoint/AObj/FObj descriptors used by articles and effect models,
    /// separate from fighter FigaTree clips. See HSD_JObjAddAnimAll/LoadDesc.
    pub fn object(d: &Dat, root: Option<usize>, model: &Model) -> Result<Self> {
        let mut tracks = Vec::new();
        let mut frames = 0f32;
        let mut looping = false;
        let mut seen = std::collections::HashSet::new();
        let mut stack = root.map(|p| vec![p]).unwrap_or_default();
        let mut joint = 0;
        while let Some(at) = stack.pop() {
            if !seen.insert(at) || seen.len() > 2048 {
                return Err("cyclic/oversized object animation tree".into());
            }
            if let Some(next) = d.ptr(at + 4)? {
                stack.push(next);
            }
            if let Some(child) = d.ptr(at)? {
                stack.push(child);
            }
            if let Some(aobj) = d.ptr(at + 8)? {
                let end = d.f32(aobj + 4)?;
                if !(0. ..=100000.).contains(&end) {
                    return Err("invalid object animation length".into());
                }
                frames = frames.max(end);
                looping |= d.u32(aobj)? & (1 << 29) != 0;
                let target = d
                    .ptr(aobj + 12)?
                    .and_then(|p| model.joints.iter().position(|j| j.address == p))
                    .unwrap_or(joint);
                let mut fobj = d.ptr(aobj + 8)?;
                let mut visited = std::collections::HashSet::new();
                while let Some(f) = fobj {
                    if !visited.insert(f) || visited.len() > 64 {
                        return Err("cyclic/oversized object tracks".into());
                    }
                    let length = d.u32(f + 4)? as usize;
                    let raw = d.bytes(d.required_ptr(f + 16)?, length)?;
                    let frac = d.bytes(f + 13, 2)?;
                    tracks.push(Track {
                        joint: target,
                        channel: d.bytes(f + 12, 1)?[0],
                        keys: keys(raw, frac[0], frac[1], d.f32(f + 8)?)?,
                    });
                    fobj = d.ptr(f)?;
                }
            }
            joint += 1;
        }
        Ok(Self {
            clips: BTreeMap::from([(
                "Object".into(),
                Clip {
                    motion_joint: None,
                    frames,
                    looping,
                    tracks,
                },
            )]),
        })
    }
    pub fn motion_definitions(&self) -> BTreeMap<String, libssbm::motion::MotionDef> {
        self.clips
            .iter()
            .map(|(name, clip)| {
                (
                    name.clone(),
                    libssbm::motion::MotionDef {
                        root_forward: clip.motion_joint.map(|joint| {
                            let track = clip
                                .tracks
                                .iter()
                                .find(|t| t.joint == joint && t.channel == 7);
                            (0..=clip.frames.ceil() as usize)
                                .map(|frame| {
                                    track.and_then(|t| t.sample(frame as f32)).unwrap_or(0.)
                                })
                                .collect()
                        }),
                        root_up: clip.motion_joint.map(|joint| {
                            let track = clip
                                .tracks
                                .iter()
                                .find(|t| t.joint == joint && t.channel == 6);
                            (0..=clip.frames.ceil() as usize)
                                .map(|frame| {
                                    track.and_then(|t| t.sample(frame as f32)).unwrap_or(0.)
                                })
                                .collect()
                        }),
                        frames: clip.frames,
                        looping: clip.looping
                            || name.starts_with("Fall")
                            || matches!(
                                name.as_str(),
                                "Wait"
                                    | "WalkSlow"
                                    | "WalkMiddle"
                                    | "WalkFast"
                                    | "Run"
                                    | "SquatWait"
                                    | "SpecialNHold"
                                    | "Fall"
                                    | "FallAerialF"
                                    | "FallAerialB"
                                    | "FallSpecial"
                            ),
                    },
                )
            })
            .collect()
    }
    pub fn load(dir: &Path) -> Result<Self> {
        Self::load_for(dir, &crate::fighter::SAMUS)
    }
    pub fn load_for(dir: &Path, fighter: &crate::fighter::FighterAssets<'_>) -> Result<Self> {
        fighter.required()?;
        let mut animations = Self::parse(&std::fs::read(dir.join(fighter.animations))?, fighter.clip_aliases)?;
        // Fighter_ChangeMotionState / ftAnim_8006E9B4 select TransN from the
        // common bone map and extract it only for flagged motion descriptors.
        let bytes = std::fs::read(dir.join(fighter.data.file))?;
        let d = Dat::parse(&bytes)?;
        let root = d.root(fighter.data.root)?;
        let start = d.required_ptr(root + 0xc)?;
        let end = d.required_ptr(root + 0x14)?;
        if end <= start || !(end - start).is_multiple_of(24) || (end - start) / 24 > 2048 {
            return Err("invalid fighter motion descriptor table".into());
        }
        let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
        let common = Dat::parse(&bytes)?;
        let table = common.required_ptr(common.root(crate::fighter::COMMON_DATA.root)? + 0x10)?;
        let parts = common.required_ptr(table + fighter.common_index * 4)?;
        let mapping = common.required_ptr(parts + 4)?;
        let joint = common.bytes(mapping + 1, 1)?[0] as usize;
        for at in (start..end).step_by(24) {
            if d.u32(at + 16)? & 0x80000000 == 0 {
                continue;
            }
            let Some(name_at) = d.ptr(at)? else {
                continue;
            };
            let tail = d.data.get(name_at..).ok_or("motion name outside DAT")?;
            let len = tail
                .iter()
                .take(256)
                .position(|&b| b == 0)
                .ok_or("unterminated motion name")?;
            let name = std::str::from_utf8(&tail[..len])?;
            if let Some(name) = name
                .split("_ACTION_")
                .nth(1)
                .and_then(|s| s.strip_suffix("_figatree"))
                && let Some(clip) = animations
                    .clips
                    .get_mut(canonical(fighter.clip_aliases, name).as_ref())
            {
                clip.motion_joint = Some(joint);
            }
        }
        Ok(animations)
    }
    pub fn parse(bytes: &[u8], aliases: &[(&str, &str)]) -> Result<Self> {
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
                let name = canonical(aliases, name);
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
                clips.insert(
                    name.into_owned(),
                    Clip {
                        motion_joint: None,
                        frames,
                        looping: d.u32(root + 4)? & (1 << 29) != 0,
                        tracks,
                    },
                );
            }
            at = (at + size + 31) & !31;
        }
        if clips.is_empty() {
            return Err("no fighter animation clips".into());
        }
        Ok(Self { clips })
    }
    pub fn pose(&self, model: &Model, name: &str, frame: f32, looping: bool) -> Vec<Mat4> {
        self.pose_inner(model, name, frame, looping, true, &[])
    }
    /// The pose with part animations laid over it: the bones they key take their values.
    pub fn pose_over(
        &self,
        model: &Model,
        name: &str,
        frame: f32,
        looping: bool,
        overlay: &[Overlay<'_>],
    ) -> Vec<Mat4> {
        self.pose_inner(model, name, frame, looping, true, overlay)
    }
    pub fn pose_object(&self, model: &Model, frame: f32) -> Vec<Mat4> {
        self.pose_inner(
            model,
            "Object",
            frame,
            self.clips.get("Object").is_some_and(|c| c.looping),
            false,
            &[],
        )
    }
    fn pose_inner(
        &self,
        model: &Model,
        name: &str,
        frame: f32,
        looping: bool,
        remove_root_motion: bool,
        overlay: &[Overlay<'_>],
    ) -> Vec<Mat4> {
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
        // ftAnim_800707B0: a bone a part animation keys stops following the main clip and
        // shows the part animation instead, easing in over its blend frames.
        for part in overlay {
            let frame = if part.clip.looping {
                part.frame.rem_euclid(part.clip.frames.max(1.))
            } else {
                part.frame.min(part.clip.frames)
            };
            for track in &part.clip.tracks {
                let j = track.joint;
                if j >= model.joints.len() {
                    continue;
                }
                let Some(v) = track.sample(frame) else { continue };
                let w = part.weight.clamp(0., 1.);
                let mix = |slot: &mut f32, v: f32| *slot += (v - *slot) * w;
                let floor = |v: f32| if v.abs() < 0.001 { 0.001 } else { v };
                match track.channel {
                    1 => mix(&mut r[j].x, v),
                    2 => mix(&mut r[j].y, v),
                    3 => mix(&mut r[j].z, v),
                    5 => mix(&mut t[j].x, v),
                    6 => mix(&mut t[j].y, v),
                    7 => mix(&mut t[j].z, v),
                    8 => mix(&mut s[j].x, floor(v)),
                    9 => mix(&mut s[j].y, floor(v)),
                    10 => mix(&mut s[j].z, floor(v)),
                    _ => (),
                }
            }
        }
        // Root translation is movement, already owned by the gameplay sim.
        if remove_root_motion && !t.is_empty() {
            t[0] = model.joints[0].translation;
            if let Some(joint) = self.clips.get(name).and_then(|clip| clip.motion_joint)
                && joint < t.len()
            {
                // ftAnim_8006E054 zeros the extracted translation in the pose.
                // Hitbox/effect attachments use this same pose, avoiding double travel.
                t[joint] = glam::Vec3::ZERO;
            }
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
