//! C ABI v2. Main-thread handles; caller owns all pointers passed in.
//! Panics poison a handle and never unwind into the host.

use glam::Vec3;
use libssbm::{Contact, Event, Host, Input, Sim};
use libssbm_assets::presentation::Presentation;
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};
pub const SSBM_ABI_VERSION: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl From<Vec3> for V3 {
    fn from(v: Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}
impl From<V3> for Vec3 {
    fn from(v: V3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pad {
    pub movement: V3,
    pub aim: V3,
    pub buttons: u32,
    pub allowed: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub position: V3,
    pub velocity: V3,
    pub facing: V3,
    pub grounded: u32,
    pub phase: u32,
    pub move_id: u32,
    pub jumps_used: u32,
    pub shot_charge: u32,
    pub smash_charge: u32,
    pub frame: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Collision {
    pub position: V3,
    pub grounded: u32,
    pub ceiling: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeEvent {
    pub kind: u32,
    pub move_id: u32,
    pub strength: f32,
    pub position: V3,
    pub direction: V3,
}
#[repr(C)]
pub struct HostTable {
    pub user: *mut c_void,
    pub collide: Option<unsafe extern "C" fn(*mut c_void, *const V3, *const V3, *mut Collision)>,
    pub event: Option<unsafe extern "C" fn(*mut c_void, *const NativeEvent)>,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tuning {
    pub speed_scale: f32,
    pub jump_scale: f32,
    pub extra_jumps: u32,
    pub hold_charge: u32,
    pub smash_max_frames: u32,
}
// ViewSource.cs shared layout. Preserve this order in ssbm_struct_sizes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ModelInfo {
    pub vertex_count: u32,
    pub index_count: u32,
    pub batch_count: u32,
    pub texture_count: u32,
    pub lo: V3,
    pub hi: V3,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Batch {
    pub first_index: u32,
    pub index_count: u32,
    pub first_vertex: u32,
    pub vertex_count: u32,
    pub texture: i32,
    pub blend: u8,
    pub lit: u8,
    pub cull: u8,
    pub env: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TextureInfo {
    pub width: u32,
    pub height: u32,
    pub wrap_s: u8,
    pub wrap_t: u8,
    pub pad0: u8,
    pub pad1: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Draw {
    pub key: u64,
    pub model: u32,
    pub kind: u8,
    pub flags: u8,
    pub pad0: u8,
    pub pad1: u8,
    pub pos: V3,
    pub rot_x: f32,
    pub rot_y: f32,
    pub rot_z: f32,
    pub rot_w: f32,
    pub first_vertex: u32,
    pub first_color: u32,
    pub alpha: f32,
    pub id: u32,
}
pub struct Handle {
    sim: Sim,
    rig: Rig,
    audio: libssbm_assets::audio::Mixer,
    poisoned: bool,
}
struct Rig {
    model: libssbm_assets::model::Model,
    animations: libssbm_assets::animation::Animations,
    common_bones: Vec<u8>,
}
impl Rig {
    fn load(dir: &Path) -> libssbm_assets::Result<Self> {
        let bytes = std::fs::read(dir.join("PlCo.dat"))?;
        let common = libssbm_assets::dat::Dat::parse(&bytes)?;
        let table = common.required_ptr(common.root("ftLoadCommonData")? + 0x10)?;
        // FighterKind::Samus = 13; FighterPartsTable, loaded by Fighter_LoadCommonData.
        let parts = common.required_ptr(table + 13 * 4)?;
        let mapping = common.required_ptr(parts + 4)?;
        Ok(Self {
            model: libssbm_assets::model::Model::load(dir, 0)?,
            animations: libssbm_assets::animation::Animations::load(dir)?,
            common_bones: common.bytes(mapping, 53)?.to_vec(),
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct HitboxSample {
    pub serial: u64,
    pub generation: u32,
    pub group: u32,
    pub id: u32,
    pub damage: f32,
    pub radius: f32,
    pub local_position: V3,
    pub origin: V3,
    pub facing: V3,
    pub angle: u32,
    pub growth: u32,
    pub fixed_knockback: u32,
    pub base_knockback: u32,
    pub element: u32,
    pub flags: u32,
}
pub struct View {
    actor: Presentation,
    poisoned: bool,
    info: ModelInfo,
    batches: Vec<Batch>,
    indices: Vec<i32>,
    uvs: Vec<f32>,
    positions: Vec<V3>,
    normals: Vec<V3>,
    colors: Vec<[f32; 4]>,
    texture_ids: Vec<i32>,
}
thread_local! {static ERROR:RefCell<CString>=RefCell::new(CString::default());}
fn error(e: impl std::fmt::Display) {
    ERROR.with(|s| {
        *s.borrow_mut() = CString::new(e.to_string().replace('\0', " ")).unwrap_or_default()
    });
}
fn guarded<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| {
        error("native panic");
        fallback
    })
}
unsafe fn path<'a>(p: *const c_char) -> Result<&'a Path, Box<dyn std::error::Error + Send + Sync>> {
    if p.is_null() {
        return Err("null path".into());
    }
    Ok(Path::new(unsafe { CStr::from_ptr(p) }.to_str()?))
}
unsafe fn copy<T: Copy>(data: &[T], out: *mut T, max: u32) -> u32 {
    if !out.is_null() {
        let n = data.len().min(max as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), out, n);
        }
    }
    data.len() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn ssbm_abi_version() -> u32 {
    SSBM_ABI_VERSION
}
#[unsafe(no_mangle)]
pub extern "C" fn ssbm_last_error() -> *const c_char {
    ERROR.with(|e| e.borrow().as_ptr())
}
/// # Safety
/// Caller supplies `max` writable u32 slots, or null to query count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_struct_sizes(out: *mut u32, max: u32) -> u32 {
    let sizes = [
        size_of::<V3>(),
        size_of::<Pad>(),
        size_of::<State>(),
        size_of::<Collision>(),
        size_of::<NativeEvent>(),
        size_of::<HostTable>(),
        size_of::<Tuning>(),
        size_of::<ModelInfo>(),
        size_of::<Batch>(),
        size_of::<TextureInfo>(),
        size_of::<Draw>(),
        size_of::<HitboxSample>(),
    ]
    .map(|s| s as u32);
    unsafe { copy(&sizes, out, max) }
}
/// # Safety
/// UTF-8 NUL-terminated cache folder path; returns 1 only for a complete cache.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_assets_complete(dir: *const c_char) -> u32 {
    guarded(0, || match unsafe { path(dir) } {
        Ok(p) => libssbm_assets::complete(p) as u32,
        Err(e) => {
            error(e);
            0
        }
    })
}
/// # Safety
/// Paths must be valid UTF-8 NUL-terminated strings for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_extract(disc: *const c_char, dir: *const c_char) -> u32 {
    guarded(0, || {
        let result =
            (|| libssbm_assets::disc::extract(unsafe { path(disc) }?, unsafe { path(dir) }?))();
        match result {
            Ok(()) => 1,
            Err(e) => {
                error(e);
                0
            }
        }
    })
}
/// # Safety
/// A valid cache path and finite starting position. Free once with ssbm_sim_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_new(dir: *const c_char, pos: V3) -> *mut Handle {
    guarded(std::ptr::null_mut(), || {
        let result = (|| -> libssbm_assets::Result<(Sim, Rig, libssbm_assets::audio::Mixer)> {
            let dir = unsafe { path(dir) }?;
            let mut sim = Sim::new(libssbm_assets::params::load(dir)?, pos.into());
            let rig = Rig::load(dir)?;
            sim.motion.definitions = rig.animations.motion_definitions();
            sim.action.definitions = libssbm_assets::action::load(dir, &rig.animations)?;
            for (name, action) in &sim.action.definitions {
                if let Some(motion) = sim.motion.definitions.get_mut(name) {
                    motion.looping |= action.looping;
                }
            }
            let audio = libssbm_assets::audio::Mixer::new(libssbm_assets::audio::Bank::load(dir)?);
            Ok((sim, rig, audio))
        })();
        match result {
            Ok((sim, rig, audio)) if Vec3::from(pos).is_finite() => {
                Box::into_raw(Box::new(Handle {
                    sim,
                    rig,
                    audio,
                    poisoned: false,
                }))
            }
            Ok(_) => {
                error("invalid position");
                std::ptr::null_mut()
            }
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// Samples every active hitbox on every fighter tick of the last step. Positions
/// are posed model-local coordinates; the engine adapter owns world conversion.
/// # Safety
/// Live handle and writable `max`-element output (or null to query count).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_hitboxes(
    handle: *const Handle,
    out: *mut HitboxSample,
    max: u32,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    if h.poisoned {
        return 0;
    }
    let count = h.sim.action.samples.len() as u32;
    if out.is_null() || max == 0 {
        return count;
    }
    guarded(0, || {
        for (i, sample) in h.sim.action.samples.iter().take(max as usize).enumerate() {
            let hit = &sample.hitbox;
            let joint = if hit.common_bone {
                h.rig.common_bones.get(hit.bone).copied().map(usize::from)
            } else {
                Some(hit.bone)
            };
            let pose = h
                .rig
                .animations
                .pose(&h.rig.model, sample.clip, sample.frame, false);
            let Some(joint) = joint.filter(|&j| j < pose.len()) else {
                error("hitbox bone outside Samus rig");
                return 0;
            };
            let local = pose[joint].transform_point3(hit.offset);
            let result = HitboxSample {
                serial: sample.serial,
                generation: sample.generation,
                group: hit.group,
                id: hit.id,
                damage: hit.damage * sample.strength,
                radius: hit.radius,
                local_position: local.into(),
                origin: sample.origin.into(),
                facing: sample.facing.into(),
                angle: hit.angle,
                growth: hit.growth,
                fixed_knockback: hit.fixed_knockback,
                base_knockback: hit.base_knockback,
                element: hit.element,
                flags: u32::from(hit.ground) | (u32::from(hit.air) << 1),
            };
            unsafe {
                *out.add(i) = result;
            }
        }
        count
    })
}
/// # Safety
/// `handle` is null or a live unique handle; consume it exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_free(handle: *mut Handle) {
    if !handle.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(handle) }));
    }
}
struct Adapter<'a>(&'a HostTable);
impl Host for Adapter<'_> {
    fn collide(&mut self, from: Vec3, to: Vec3) -> Contact {
        let mut c = Collision {
            position: to.into(),
            grounded: 0,
            ceiling: 0,
        };
        if let Some(f) = self.0.collide {
            unsafe {
                f(self.0.user, &from.into(), &to.into(), &mut c);
            }
        }
        Contact {
            position: c.position.into(),
            grounded: c.grounded != 0,
            ceiling: c.ceiling != 0,
        }
    }
    fn event(&mut self, e: Event) {
        if let Some(f) = self.0.event {
            let e = NativeEvent {
                kind: e.kind as u32,
                move_id: e.move_id,
                strength: e.strength,
                position: e.position.into(),
                direction: e.direction.into(),
            };
            unsafe {
                f(self.0.user, &e);
            }
        }
    }
}
/// # Safety
/// All pointers must be valid throughout the call; callbacks must not throw/reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_step(
    handle: *mut Handle,
    host: *const HostTable,
    pad: *const Pad,
    dt: f64,
) -> u32 {
    if handle.is_null() || host.is_null() || pad.is_null() {
        return 0;
    }
    let h = unsafe { &mut *handle };
    if h.poisoned {
        return 0;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let p = unsafe { *pad };
        let ticks = h.sim.advance(
            &mut Adapter(unsafe { &*host }),
            Input {
                movement: p.movement.into(),
                aim: p.aim.into(),
                buttons: p.buttons,
                allowed: p.allowed,
            },
            dt,
        );
        for cue in &h.sim.action.sounds {
            if cue.stop {
                h.audio.stop(cue.channel);
            } else if let Err(e) = h.audio.play(cue.sound, cue.volume, cue.pan, cue.channel) {
                error(e);
            }
        }
        ticks
    })) {
        Ok(n) => n,
        Err(_) => {
            h.poisoned = true;
            error("sim panic: handle poisoned");
            0
        }
    }
}
/// Render stereo float PCM at 32 kHz. Call after stepping, outside host callbacks.
/// # Safety
/// Unique live handle and `frames * 2` writable f32 slots. No concurrent calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_audio(handle: *mut Handle, out: *mut f32, frames: u32) -> u32 {
    if handle.is_null() || out.is_null() || frames > 32000 {
        return 0;
    }
    let h = unsafe { &mut *handle };
    if h.poisoned {
        return 0;
    }
    guarded(0, || {
        h.audio
            .render(unsafe { std::slice::from_raw_parts_mut(out, frames as usize * 2) });
        if let Some(e) = h.audio.errors.pop() {
            error(e);
            return 0;
        }
        frames
    })
}
/// # Safety
/// Live handle and writable State pointer. Returns 0 for poisoned handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_state(handle: *const Handle, out: *mut State) -> u32 {
    if handle.is_null() || out.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    if h.poisoned {
        return 0;
    }
    let s = &h.sim;
    unsafe {
        *out = State {
            position: s.position.into(),
            velocity: s.velocity.into(),
            facing: s.facing.into(),
            grounded: s.grounded as u32,
            phase: s.phase as u32,
            move_id: s.current_move.map(|m| m as u32).unwrap_or(0),
            jumps_used: s.jumps_used,
            shot_charge: s.shot_charge,
            smash_charge: s.smash_charge,
            frame: s.frame,
        };
    }
    1
}
/// # Safety
/// Live handle and finite/bounded tuning. Invalid tuning leaves previous values intact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_tuning(handle: *mut Handle, t: Tuning) -> u32 {
    if handle.is_null()
        || !t.speed_scale.is_finite()
        || !t.jump_scale.is_finite()
        || !(0. ..=100.).contains(&t.speed_scale)
        || !(0. ..=100.).contains(&t.jump_scale)
        || t.extra_jumps > 100
        || t.smash_max_frames == 0
    {
        return 0;
    }
    let h = unsafe { &mut *handle };
    if h.poisoned {
        return 0;
    }
    h.sim.options = libssbm::Options {
        speed_scale: t.speed_scale,
        jump_scale: t.jump_scale,
        extra_jumps: t.extra_jumps,
        hold_charge: t.hold_charge != 0,
        smash_max_frames: t.smash_max_frames,
    };
    1
}
/// # Safety
/// Live handle; finite vector. kind=0 teleports, kind=1 applies velocity in units/tick.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_place(handle: *mut Handle, v: V3, kind: u32) {
    if handle.is_null() || !Vec3::from(v).is_finite() {
        return;
    }
    let h = unsafe { &mut *handle };
    if !h.poisoned {
        if kind == 0 {
            h.sim.teleport(v.into());
        } else {
            h.sim.launch(v.into());
        }
    }
}

impl View {
    fn new(actor: Presentation) -> Self {
        let mut v = Self {
            actor,
            poisoned: false,
            info: ModelInfo::default(),
            batches: vec![],
            indices: vec![],
            uvs: vec![],
            positions: vec![],
            normals: vec![],
            colors: vec![],
            texture_ids: vec![],
        };
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for (i, b) in v.actor.model.batches.iter().enumerate() {
            let first_vertex = v.positions.len() as u32;
            let texture = if b.texture.is_some() { i as i32 } else { -1 };
            v.batches.push(Batch {
                first_index: v.indices.len() as u32,
                index_count: b.indices.len() as u32,
                first_vertex,
                vertex_count: b.vertices.len() as u32,
                texture,
                blend: 0,
                lit: 1,
                cull: 2,
                env: 0,
            });
            v.indices
                .extend(b.indices.iter().map(|&n| (first_vertex + n) as i32));
            for vertex in &b.vertices {
                v.positions.push(vertex.position.into());
                v.normals.push(vertex.normal.into());
                v.uvs.extend([vertex.uv.x, vertex.uv.y]);
                lo = lo.min(vertex.position);
                hi = hi.max(vertex.position);
            }
            v.colors.push(b.color);
            v.texture_ids.push(texture);
        }
        v.info = ModelInfo {
            vertex_count: v.positions.len() as u32,
            index_count: v.indices.len() as u32,
            batch_count: v.batches.len() as u32,
            texture_count: v.batches.len() as u32,
            lo: lo.into(),
            hi: hi.into(),
        };
        v
    }
}
/// # Safety
/// UTF-8 cache path; costume index 0..4. Free with ssbm_view_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_new(dir: *const c_char, costume: u32) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        let result = (|| Presentation::load(unsafe { path(dir) }?, costume as usize))();
        match result {
            Ok(a) => Box::into_raw(Box::new(View::new(a))),
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// # Safety
/// Unique live view or null; consume exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_free(v: *mut View) {
    if !v.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(v) }));
    }
}
/// # Safety
/// Live view and simulation, main thread only. Positions remain valid until next update/free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_update(v: *mut View, h: *const Handle, dt: f32) -> u32 {
    if v.is_null() || h.is_null() || !dt.is_finite() || dt < 0. {
        return 0;
    }
    let v = unsafe { &mut *v };
    let h = unsafe { &*h };
    if v.poisoned || h.poisoned {
        return 0;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        v.actor.update(&h.sim, dt.min(0.25));
        let mut offset = 0;
        for i in 0..v.actor.model.batches.len() {
            let skin = v.actor.model.skin(i, &v.actor.world);
            let visible = v.actor.visible(i);
            for (p, n) in skin {
                v.positions[offset] = if visible { p.into() } else { V3::default() };
                v.normals[offset] = n.into();
                offset += 1;
            }
            let flash = h.sim.phase == libssbm::Phase::SmashCharge && (h.sim.frame / 4) % 2 == 0;
            v.colors[i] = if flash {
                [1., 0.9, 0.15, 1.]
            } else {
                v.actor.model.batches[i].color
            };
        }
        1
    })) {
        Ok(n) => n,
        Err(_) => {
            v.poisoned = true;
            error("view panic: handle poisoned");
            0
        }
    }
}
/// # Safety
/// Live view and writable model-info pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_model(v: *const View, out: *mut ModelInfo) -> u32 {
    if v.is_null() || out.is_null() {
        return 0;
    }
    unsafe {
        *out = (*v).info;
    }
    1
}
macro_rules! array_export {
    ($name:ident,$field:ident,$ty:ty) => {
        #[doc = "# Safety\nLive view; output has `max` writable elements, or is null to query count."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(v: *const View, out: *mut $ty, max: u32) -> u32 {
            if v.is_null() {
                return 0;
            }
            unsafe { copy(&(*v).$field, out, max) }
        }
    };
}
array_export!(ssbm_view_batches, batches, Batch);
array_export!(ssbm_view_indices, indices, i32);
array_export!(ssbm_view_uvs, uvs, f32);
macro_rules! pointer_export {
    ($name:ident,$field:ident,$ty:ty) => {
        #[doc = "# Safety\nLive view and writable count. Borrowed pointer valid until update/free."]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(v: *const View, count: *mut u32) -> *const $ty {
            if v.is_null() || count.is_null() {
                return std::ptr::null();
            }
            let v = unsafe { &*v };
            unsafe {
                *count = v.$field.len() as u32;
            }
            v.$field.as_ptr()
        }
    };
}
pointer_export!(ssbm_view_positions, positions, V3);
pointer_export!(ssbm_view_normals, normals, V3);
pointer_export!(ssbm_view_colors, colors, [f32; 4]);
pointer_export!(ssbm_view_texture_ids, texture_ids, i32);
/// # Safety
/// Live view; texture id is batch index. Null pixels queries dimensions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_texture(
    v: *const View,
    id: u32,
    out: *mut TextureInfo,
    pixels: *mut u8,
    len: u32,
) -> u32 {
    if v.is_null() || out.is_null() {
        return 0;
    }
    let v = unsafe { &*v };
    let Some(t) = v
        .actor
        .model
        .batches
        .get(id as usize)
        .and_then(|b| b.texture.as_ref())
    else {
        return 0;
    };
    unsafe {
        *out = TextureInfo {
            width: t.width as u32,
            height: t.height as u32,
            wrap_s: 0,
            wrap_t: 0,
            pad0: 0,
            pad1: 0,
        };
    }
    if !pixels.is_null() {
        if len < t.rgba.len() as u32 {
            return 0;
        }
        unsafe {
            copy(&t.rgba, pixels, len);
        }
    }
    1
}

/// # Safety
/// # Safety
/// `handle` must be a live uniquely borrowed simulation, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_interrupt(handle: *mut Handle) {
    if !handle.is_null() {
        let h = unsafe { &mut *handle };
        if !h.poisoned {
            h.sim.interrupt();
        }
    }
}
/// # Safety
/// # Safety
/// Live unique handle or null; velocity is units/tick, gravity units/tick squared.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_launch_pad(handle: *mut Handle, velocity: V3, gravity: V3) {
    if handle.is_null() || !Vec3::from(velocity).is_finite() || !Vec3::from(gravity).is_finite() {
        return;
    }
    let h = unsafe { &mut *handle };
    if !h.poisoned {
        h.sim.launch_pad(velocity.into(), gravity.into());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abi_layouts_are_pinned() {
        assert_eq!(
            [
                size_of::<V3>(),
                size_of::<Pad>(),
                size_of::<State>(),
                size_of::<Collision>(),
                size_of::<NativeEvent>(),
                size_of::<HostTable>(),
                size_of::<Tuning>(),
                size_of::<ModelInfo>(),
                size_of::<Batch>(),
                size_of::<TextureInfo>(),
                size_of::<Draw>(),
                size_of::<HitboxSample>()
            ],
            [12, 32, 72, 20, 36, 24, 20, 40, 24, 12, 64, 88]
        );
    }
    #[test]
    fn null_inputs_fail_without_dereferencing() {
        unsafe {
            assert_eq!(
                ssbm_sim_step(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), 0.),
                0
            );
            assert_eq!(
                ssbm_view_texture(
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0
                ),
                0
            );
        }
    }
}
