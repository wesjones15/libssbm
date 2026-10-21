//! C ABI v4. Main-thread handles; caller owns all pointers passed in.
//! Panics poison a handle and never unwind into the host.

use glam::Vec3;
use libssbm::{Contact, Event, Host, Input, Sim};
use libssbm_assets::presentation::Presentation;
mod particles;
mod projectile;
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};
pub const SSBM_ABI_VERSION: u32 = 5;
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
    kind: libssbm_assets::fighter::FighterKind,
    poisoned: bool,
}
struct Rig {
    grapple: Option<libssbm_assets::grapple::Tuning>,
    charge_flash: Vec<[f32; 4]>,
    model: libssbm_assets::model::Model,
    animations: libssbm_assets::animation::Animations,
    common_bones: Vec<u8>,
    effect_bone: usize,
    alternate_bone: usize,
    cycle_bones: Vec<usize>,
    upper_body_root: Option<usize>,
}
impl Rig {
    fn pose_for_sim(&self, sim: &Sim, clip: &str, frame: f32) -> Vec<glam::Mat4> {
        if clip == sim.motion.clip {
            libssbm_assets::presentation::mobile_pose(
                &self.model,
                &self.animations,
                sim,
                frame,
                self.upper_body_root,
            )
        } else {
            self.animations.pose(&self.model, clip, frame, false)
        }
    }
    fn load_for(dir: &Path, fighter: &libssbm_assets::fighter::FighterAssets<'_>) -> libssbm_assets::Result<Self> {
        let fighter_bytes = std::fs::read(dir.join(fighter.data.file))?;
        let dat = libssbm_assets::dat::Dat::parse(&fighter_bytes)?;
        let root = dat.root(fighter.data.root)?;
        let desc = dat.required_ptr(root + 8)?;
        let cycle = dat.ptr(root + 0x54)?;
        let bytes = std::fs::read(dir.join(libssbm_assets::fighter::COMMON_DATA.file))?;
        let common = libssbm_assets::dat::Dat::parse(&bytes)?;
        let table = common.required_ptr(common.root(libssbm_assets::fighter::COMMON_DATA.root)? + 0x10)?;
        // FighterPartsTable, loaded by Fighter_LoadCommonData; the profile's kind.
        let parts = common.required_ptr(table + fighter.common_index * 4)?;
        let mapping = common.required_ptr(parts + 4)?;
        Ok(Self {
            // Only fighters with a source grapple beam (Samus) carry this tuning.
            grapple: libssbm_assets::grapple::tuning(dir).ok(),
            charge_flash: libssbm_assets::params::color_cycle(dir, 53)?,
            model: libssbm_assets::model::Model::load_for(dir, 0, fighter)?,
            animations: libssbm_assets::animation::Animations::load_for(dir, fighter)?,
            common_bones: common.bytes(mapping, 53)?.to_vec(),
            effect_bone: dat.bytes(desc + 0x12, 1)?[0] as usize,
            alternate_bone: dat.bytes(desc + 0x10, 1)?[0] as usize,
            cycle_bones: if let Some(at) = cycle {
                (0..5)
                    .map(|i| dat.u32(at + i * 4).map(|n| n as usize))
                    .collect::<libssbm_assets::Result<Vec<_>>>()?
            } else {
                Vec::new()
            },
            upper_body_root: fighter.upper_body_root,
        })
    }
    fn grapple_points(&self, clip: &str, frame: f32) -> Vec<Vec3> {
        let Some(grapple) = &self.grapple else {
            return Vec::new();
        };
        let pose = self.animations.pose(&self.model, clip, frame, false);
        let anchor = pose[50].w_axis.truncate();
        let air = clip == "AirCatch";
        let start = if air { 8. } else { 17. };
        let end = if air { 28. } else { 41. };
        let age = (frame - start).max(0.);
        let maximum = grapple.length * (grapple.links - 1) as f32;
        let length = (age * grapple.speed).min(maximum) * ((end - frame) / 6.).clamp(0., 1.);
        (0..grapple.links)
            .map(|i| {
                let u = i as f32 / (grapple.links - 1) as f32;
                anchor + Vec3::Z * length * u
                    - Vec3::Y
                        * (grapple.gravity * age * age * 0.5).min(length * 0.15)
                        * (u * (1. - u) * 4.)
            })
            .collect()
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
    frame_uvs: Vec<f32>,
    positions: Vec<V3>,
    normals: Vec<V3>,
    colors: Vec<[f32; 4]>,
    alpha_sources: Vec<[f32; 3]>,
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
/// Resolve a fighter kind to its asset profile. `Fox` is reserved until its
/// `FighterAssets` is defined; the error keeps that path honest rather than
/// falling back to Samus assets.
fn fighter_assets(
    kind: u32,
) -> Result<&'static libssbm_assets::fighter::FighterAssets<'static>, Box<dyn std::error::Error + Send + Sync>>
{
    libssbm_assets::fighter::FighterKind::from_u32(kind)
        .and_then(|k| k.assets())
        .ok_or_else(|| format!("unsupported or unimplemented fighter kind {kind}").into())
}

#[unsafe(no_mangle)]
pub extern "C" fn ssbm_abi_version() -> u32 {
    SSBM_ABI_VERSION
}
/// Announcer name-call sound for the current fighter, played on the character
/// select screen. gm_1601.c::gm_80168C5C (invoked from mncharsel.c) maps the
/// CharacterKind to the 0x7C83X name bank; nr_name.ssm holds the samples.
#[unsafe(no_mangle)]
pub extern "C" fn ssbm_announce_sound(kind: u32) -> u32 {
    fighter_assets(kind).map_or(0, |f| f.announce_sound)
}
/// Decode the player's original character-select photo, rows top first.
/// # Safety
/// Valid cache path, writable info, and `len` writable output bytes (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_portrait(
    dir: *const c_char,
    kind: u32,
    costume: u32,
    info: *mut TextureInfo,
    rgba: *mut u8,
    len: u32,
) -> u32 {
    if info.is_null() {
        return 0;
    }
    guarded(0, || {
        let result = (|| -> libssbm_assets::Result<_> {
            let fighter = fighter_assets(kind)?;
            if costume >= fighter.costumes.len() as u32 {
                return Err("invalid costume".into());
            }
            libssbm_assets::portrait::load_grid(unsafe { path(dir) }?, fighter)
        })();
        match result {
            Ok(photo) => {
                unsafe {
                    *info = TextureInfo {
                        width: photo.width as u32,
                        height: photo.height as u32,
                        wrap_s: 1,
                        wrap_t: 1,
                        pad0: 0,
                        pad1: 0,
                    };
                }
                if !rgba.is_null() {
                    if (len as usize) < photo.rgba.len() {
                        error("portrait output buffer too small");
                        return 0;
                    }
                    unsafe {
                        std::ptr::copy_nonoverlapping(photo.rgba.as_ptr(), rgba, photo.rgba.len());
                    }
                }
                1
            }
            Err(e) => {
                error(e);
                0
            }
        }
    })
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
        size_of::<particles::Sprite>(),
        size_of::<projectile::State>(),
    ]
    .map(|s| s as u32);
    unsafe { copy(&sizes, out, max) }
}
/// # Safety
/// UTF-8 NUL-terminated cache folder path; returns 1 only for a complete cache.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_assets_complete(dir: *const c_char, kind: u32) -> u32 {
    guarded(0, || {
        let result = (|| -> libssbm_assets::Result<u32> {
            let fighter = fighter_assets(kind)?;
            Ok(fighter.complete(unsafe { path(dir) }?) as u32)
        })();
        match result {
            Ok(n) => n,
            Err(e) => {
                error(e);
                0
            }
        }
    })
}
/// # Safety
/// Paths must be valid UTF-8 NUL-terminated strings for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_extract(disc: *const c_char, dir: *const c_char, kind: u32) -> u32 {
    guarded(0, || {
        let result = (|| -> libssbm_assets::Result<()> {
            let fighter = fighter_assets(kind)?;
            libssbm_assets::disc::extract_for(
                unsafe { path(disc) }?,
                unsafe { path(dir) }?,
                fighter,
            )
        })();
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
    unsafe { ssbm_sim_new_fighter(dir, pos, 0) }
}
/// # Safety
/// A valid cache path, finite starting position and a supported fighter kind
/// (0 = Samus). Free once with ssbm_sim_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_new_fighter(dir: *const c_char, pos: V3, kind: u32) -> *mut Handle {
    guarded(std::ptr::null_mut(), || {
        let result = (|| -> libssbm_assets::Result<(
            Sim,
            Rig,
            libssbm_assets::audio::Mixer,
            libssbm_assets::fighter::FighterKind,
        )> {
            let dir = unsafe { path(dir) }?;
            let fk = libssbm_assets::fighter::FighterKind::from_u32(kind)
                .ok_or_else(|| format!("unsupported fighter kind {kind}"))?;
            let fighter = fk
                .assets()
                .ok_or_else(|| format!("fighter kind {kind} not implemented"))?;
            let (params, behavior): (
                libssbm::Params,
                Box<dyn libssbm::behavior::Behavior>,
            ) = match fk {
                libssbm_assets::fighter::FighterKind::Samus => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::samus::Samus::new(
                        libssbm_assets::params::load_samus_specials(dir)?,
                        fighter.upper_body_root,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Fox => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::fox::Fox::new(
                        libssbm_assets::params::load_fox_specials(dir)?,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Pikachu => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::pikachu::Pikachu::new(
                        libssbm_assets::params::load_pikachu_specials(dir)?,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Luigi => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::luigi::Luigi::new(
                        libssbm_assets::params::load_luigi_specials(dir)?,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Captain => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::captain::Captain::new(
                        libssbm_assets::params::load_captain_specials(dir)?,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Link => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::link::Link::new(
                        libssbm_assets::params::load_link_specials(dir)?,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Yoshi => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::yoshi::Yoshi::new(
                        libssbm_assets::params::load_yoshi_specials(dir)?,
                    )),
                ),
                libssbm_assets::fighter::FighterKind::Donkey => (
                    libssbm_assets::params::load_common(dir, fighter)?,
                    Box::new(libssbm::donkey::Donkey::new(
                        libssbm_assets::params::load_donkey_specials(dir)?,
                    )),
                ),
            };
            let mut sim = Sim::with_behavior(params, behavior, pos.into());
            sim.defense.params = libssbm_assets::params::defense_for(dir, fighter)?;
            sim.defense.health = sim.defense.params.maximum;
            let rig = Rig::load_for(dir, fighter)?;
            sim.motion.definitions = rig.animations.motion_definitions();
            sim.action.definitions = libssbm_assets::action::load_for(dir, &rig.animations, fighter)?;
            for (name, action) in &sim.action.definitions {
                if let Some(motion) = sim.motion.definitions.get_mut(name) {
                    motion.looping |= action.looping;
                }
            }
            let audio =
                libssbm_assets::audio::Mixer::new(libssbm_assets::audio::Bank::load_for(dir, fighter)?);
            Ok((sim, rig, audio, fk))
        })();
        match result {
            Ok((sim, rig, audio, kind)) if Vec3::from(pos).is_finite() => {
                Box::into_raw(Box::new(Handle {
                    sim,
                    rig,
                    audio,
                    kind,
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
            let pose = h.rig.pose_for_sim(&h.sim, sample.clip, sample.frame);
            if joint == Some(139) {
                let local = *h
                    .rig
                    .grapple_points(sample.clip, sample.frame)
                    .last()
                    .unwrap();
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
                    flags: u32::from(hit.ground)
                        | (u32::from(hit.air) << 1)
                        | (hit.sfx_kind << 8)
                        | (hit.sfx_severity << 16),
                };
                unsafe {
                    *out.add(i) = result;
                }
                continue;
            }
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
                flags: u32::from(hit.ground)
                    | (u32::from(hit.air) << 1)
                    | (hit.sfx_kind << 8)
                    | (hit.sfx_severity << 16),
            };
            unsafe {
                *out.add(i) = result;
            }
        }
        count
    })
}
fn fighter_matrix(origin: Vec3, facing: Vec3) -> glam::Mat4 {
    glam::Mat4::from_translation(origin)
        * glam::Mat4::from_rotation_y(facing.x.atan2(facing.z))
        * glam::Mat4::from_scale(Vec3::new(-1., 1., 1.))
}
/// # Safety
/// Live handle; writable max NativeEvents (or null). For GFX, kind is source
/// effect ID, move_id stores bone/common/destroy flags, strength is its parameter,
/// position is a posed world-sim-unit point, direction is fighter heading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_gfx(
    handle: *const Handle,
    out: *mut NativeEvent,
    max: u32,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    let count = h.sim.action.effects.len() as u32;
    if out.is_null() {
        return count;
    }
    guarded(0, || {
        for (i, cue) in h.sim.action.effects.iter().take(max as usize).enumerate() {
            let part = if cue.bone == usize::MAX {
                h.rig.effect_bone
            } else {
                cue.bone
            };
            let bone = if part == 0x8d {
                h.rig
                    .cycle_bones
                    .get(cue.cycle as usize)
                    .copied()
                    .unwrap_or(usize::MAX)
            } else if part == 0x8e {
                h.rig.alternate_bone
            } else if cue.common {
                h.rig
                    .common_bones
                    .get(part)
                    .copied()
                    .map(usize::from)
                    .unwrap_or(usize::MAX)
            } else {
                part
            };
            let pose = h.rig.pose_for_sim(&h.sim, cue.clip, cue.frame);
            let Some(joint) = pose.get(bone) else {
                error(format!(
                    "GFX bone outside rig: effect {} clip {} frame {} part {} common {} resolved {}",
                    cue.id, cue.clip, cue.frame, part, cue.common, bone
                ));
                return 0;
            };
            let position = fighter_matrix(cue.origin, cue.facing)
                .transform_point3(joint.transform_point3(cue.offset));
            unsafe {
                *out.add(i) = NativeEvent {
                    kind: cue.id,
                    move_id: bone as u32 | if cue.destroy_on_change { 1 << 17 } else { 0 },
                    strength: cue.param,
                    position: position.into(),
                    direction: cue.facing.into(),
                };
            }
        }
        count
    })
}
/// Like `ssbm_sim_bone`, but `part` is a common FtPart index (ft/forward.h:
/// HipN = 4, RThumbNb = 49, ...) resolved through the fighter's common bone
/// table. Returns 0 when the part has no joint.
/// # Safety
/// Live simulation and 16 writable floats for the posed bone matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_common_bone(
    handle: *const Handle,
    part: u32,
    out: *mut f32,
) -> u32 {
    if handle.is_null() || out.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    guarded(0, || {
        let Some(&joint) = h.rig.common_bones.get(part as usize) else {
            return 0;
        };
        unsafe { ssbm_sim_bone(handle, joint as u32, out) }
    })
}
/// # Safety
/// Live simulation and 16 writable floats for the posed physical bone matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_bone(handle: *const Handle, bone: u32, out: *mut f32) -> u32 {
    if handle.is_null() || out.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    guarded(0, || {
        if bone == 139 {
            let local = *h
                .rig
                .grapple_points(h.sim.motion.clip, h.sim.motion.frame)
                .last()
                .unwrap();
            let matrix =
                fighter_matrix(h.sim.position, h.sim.facing) * glam::Mat4::from_translation(local);
            unsafe {
                std::ptr::copy_nonoverlapping(matrix.to_cols_array().as_ptr(), out, 16);
            }
            return 1;
        }
        let pose = h
            .rig
            .pose_for_sim(&h.sim, h.sim.motion.clip, h.sim.motion.frame);
        let Some(joint) = pose.get(bone as usize) else {
            return 0;
        };
        let matrix = fighter_matrix(h.sim.position, h.sim.facing) * *joint;
        unsafe {
            std::ptr::copy_nonoverlapping(matrix.to_cols_array().as_ptr(), out, 16);
        }
        1
    })
}
/// # Safety
/// Unique live fighter; host reports a successful capture/release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_set_grabbed(h: *mut Handle, held: u32) {
    if !h.is_null() {
        unsafe {
            (&mut *h).sim.set_grabbed(held != 0);
        }
    }
}
/// # Safety
/// Live fighter or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_grabbed(h: *const Handle) -> u32 {
    if h.is_null() {
        0
    } else {
        unsafe { (&*h).sim.grabbed as u32 }
    }
}
/// # Safety
/// Live fighter or null; read-only source motion clock for accessory posing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_motion_frame(h: *const Handle) -> f32 {
    if h.is_null() {
        0.
    } else {
        unsafe { (&*h).sim.motion.frame }
    }
}
/// Posed source grapple chain for the RoR2 3D extension policy. Endpoint is also
/// used by the source virtual hitbox joint 0x8B, so contact follows the model.
/// # Safety
/// Live fighter, max writable V3 values or null to query.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_grapple_points(h: *const Handle, out: *mut V3, max: u32) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || unsafe {
        let h = &*h;
        let clip = h.sim.motion.clip;
        if !matches!(clip, "Catch" | "CatchDash" | "AirCatch") {
            return 0;
        }
        let start = if clip == "AirCatch" { 8. } else { 17. };
        let end = if clip == "AirCatch" { 28. } else { 41. };
        if h.sim.motion.frame < start || h.sim.motion.frame > end {
            return 0;
        }
        let matrix = fighter_matrix(h.sim.position, h.sim.facing);
        let points: Vec<V3> = h
            .rig
            .grapple_points(clip, h.sim.motion.frame)
            .into_iter()
            .map(|p| matrix.transform_point3(p).into())
            .collect();
        copy(&points, out, max)
    })
}
/// # Safety
/// Valid cache path; returned native view must be freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_grapple_new(dir: *const c_char, part: u32) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        match (|| libssbm_assets::grapple::part(unsafe { path(dir) }?, part))() {
            Ok(p) => Box::into_raw(Box::new(View::new(p))),
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// # Safety
/// Live simulation. Motion serial changes on every clip/state transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_motion_serial(handle: *const Handle) -> u64 {
    if handle.is_null() {
        0
    } else {
        unsafe { &*handle }.sim.motion.serial
    }
}
/// # Safety
/// Live sim and sixteen writable floats. Returns the original held charge
/// article's posed/scaled matrix, or zero outside the charging state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_charge_visual(h: *const Handle, out: *mut f32) -> u32 {
    if h.is_null() || out.is_null() {
        return 0;
    }
    guarded(0, || unsafe {
        let h = &*h;
        if h.sim.phase != libssbm::Phase::ChargeShot {
            return 0;
        }
        let mut matrix = [0.; 16];
        if ssbm_sim_bone(h, 50, matrix.as_mut_ptr()) != 1 {
            return 0;
        }
        let steps = h.sim.behavior.charge_steps();
        let (visual_min, visual_max) = h.sim.behavior.charge_visual();
        let amount = h.sim.shot_charge as f32 / steps.max(1) as f32;
        let scale = visual_min + (visual_max - visual_min) * amount;
        let posed = glam::Mat4::from_cols_array(&matrix)
            * glam::Mat4::from_translation(Vec3::new(0., 0., 4.))
            * glam::Mat4::from_scale(Vec3::splat(scale));
        std::ptr::copy_nonoverlapping(posed.to_cols_array().as_ptr(), out, 16);
        1
    })
}
/// # Safety
/// Live fighter or null. UI resource amount, independent of the charge article.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_charge_fraction(h: *const Handle) -> f32 {
    if h.is_null() {
        0.
    } else {
        unsafe {
            let h = &*h;
            h.sim.shot_charge as f32 / h.sim.behavior.charge_steps().max(1) as f32
        }
    }
}
/// # Safety
/// Writable max u32 IDs (or null). Host presentation supplies completed fighter
/// frame range; source accessory policy remains in Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_article_cues(
    kind: u32,
    state: u32,
    previous: u32,
    frame: u32,
    full_charge: u32,
    out: *mut u32,
    max: u32,
) -> u32 {
    guarded(0, || unsafe {
        copy(
            &libssbm::effect::article_cues(kind, state, previous, frame, full_charge != 0),
            out,
            max,
        )
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn ssbm_effect_target(id: u32) -> i32 {
    match libssbm::effect::resolve(id) {
        libssbm::effect::Target::Particle(id) => id as i32,
        libssbm::effect::Target::Model(id) => -(id as i32) - 1,
        libssbm::effect::Target::NoOp => i32::MIN,
        libssbm::effect::Target::CameraShake(id) => i32::MIN + id as i32 + 1,
        libssbm::effect::Target::Unknown => i32::MAX,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn ssbm_effect_flags(id: u32) -> u32 {
    // Attachment policy from ftCo_8009F834 and efAlt_Spawn.
    match id {
        1026 | 1027 | 1060 | 1086 | 1151 | 1154 | 1159 | 1214 | 1215 | 1216 | 1301 => 1,
        _ => 0,
    }
}
/// # Safety
/// `handle` is null or a live unique handle; consume it exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_free(handle: *mut Handle) {
    if !handle.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(handle) }));
    }
}
struct Adapter<'a>(&'a HostTable, &'a Rig);
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
            let position = if let Some(bone) = e.attachment {
                // An attachment at or above 0x10000 names a common FtPart (the
                // Blaster's right thumb), resolved through the common bone table.
                let bone = if bone >= 0x1_0000 {
                    self.1
                        .common_bones
                        .get(bone - 0x1_0000)
                        .copied()
                        .map(usize::from)
                        .unwrap_or(usize::MAX)
                } else {
                    bone
                };
                let action_loop = self
                    .1
                    .animations
                    .clips
                    .get(e.clip)
                    .is_some_and(|c| c.looping);
                let gait_loop = self
                    .1
                    .animations
                    .clips
                    .get(e.gait_clip)
                    .is_some_and(|c| c.looping);
                let mut pose = libssbm_assets::presentation::mobile_pose_layers(
                    &self.1.model,
                    &self.1.animations,
                    e.clip,
                    e.pose_frame,
                    action_loop,
                    e.gait_clip,
                    e.gait_frame,
                    gait_loop,
                    e.gait_weight,
                    self.1.upper_body_root,
                );
                if matches!(e.move_id, 21 | 22)
                    && let Some(root) = self.1.upper_body_root
                {
                    let (_, yaw) = libssbm::side_special_upper_angles(e.direction, e.facing);
                    libssbm_assets::presentation::aim_upper_body(
                        &self.1.model,
                        &mut pose,
                        root,
                        yaw,
                    );
                }
                pose.get(bone)
                    .map(|joint| {
                        (fighter_matrix(e.position, e.facing) * *joint).transform_point3(Vec3::ZERO)
                            + e.attachment_offset
                    })
                    .unwrap_or(e.position)
            } else {
                e.position
            };
            let e = NativeEvent {
                kind: e.kind as u32,
                move_id: e.move_id,
                strength: e.strength,
                position: position.into(),
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
            &mut Adapter(unsafe { &*host }, &h.rig),
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
/// Play one source sound from the fighter's extracted SEM/SSM banks.
/// # Safety
/// Unique live fighter; call outside host callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_play_sound(handle: *mut Handle, sound: u32) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &mut *handle };
    if h.poisoned {
        return 0;
    }
    guarded(0, || match h.audio.play(sound, 127, 64, 0) {
        Ok(()) => 1,
        Err(e) => {
            error(e);
            0
        }
    })
}
/// Whether any fighter audio (a name call, an action cue) is still playing.
/// # Safety
/// Unique live fighter; call outside host callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_audio_active(handle: *const Handle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    if h.poisoned {
        return 0;
    }
    h.audio.active() as u32
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
/// Deprecated host mobile-move override: the fighter's `Behavior` now owns which
/// moves steer with the upper/lower split. Kept for ABI compatibility; ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_mobile_moves(_handle: *mut Handle, _mask: u64) -> u32 {
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
        if kind == 2 {
            h.sim.correct_position(v.into());
        } else if kind == 0 {
            h.sim.teleport(v.into());
        } else {
            h.sim.launch(v.into());
        }
    }
}
/// Supply the host's resolved horizontal heading without changing momentum or
/// the aim ray. Existing ABI-4 layouts are unchanged.
/// # Safety
/// Unique live handle or null. No concurrent calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_facing(handle: *mut Handle, facing: V3) {
    if let Some(h) = unsafe { handle.as_mut() } {
        if !h.poisoned {
            h.sim.set_host_facing(facing.into());
        }
    }
}
/// Set the requested jump apex in sim units; zero restores source jump tuning.
/// # Safety
/// Unique live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_jump_height(handle: *mut Handle, height: f32) {
    if let Some(h) = unsafe { handle.as_mut() } {
        if !h.poisoned && height.is_finite() && height >= 0. {
            h.sim.host_jump_height = if height > 0. { Some(height) } else { None };
        }
    }
}
/// # Safety
/// Live fighter or null. Bits: shield=1, dodge invulnerability=2, broken=4,
/// air dodge=8, shield pose=16, guard-on=32, knockdown/get-up immunity=64,
/// up-special (Screw Attack) i-frames=128.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_defense_flags(h: *const Handle) -> u32 {
    unsafe { h.as_ref() }
        .filter(|h| !h.poisoned)
        .map(|h| h.sim.defense_flags())
        .unwrap_or(0)
}
/// Extend an existing knockdown's recovery immunity while host stun owns input.
/// Does nothing when no knockdown protection is active.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_hold_knockdown(h: *mut Handle) {
    if let Some(h) = unsafe { h.as_mut() }.filter(|h| !h.poisoned) {
        h.sim.hold_knockdown_protection();
    }
}
/// # Safety
/// Unique fighter or null, damage as a finite fraction of host maximum health.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_shield_hit(h: *mut Handle, damage: f32) -> u32 {
    unsafe { h.as_mut() }
        .filter(|h| !h.poisoned)
        .map(|h| h.sim.shield_hit(damage * h.sim.defense.params.maximum) as u32)
        .unwrap_or(0)
}
/// # Safety
/// Live fighter or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_shield_fraction(h: *const Handle) -> f32 {
    unsafe { h.as_ref() }
        .filter(|h| !h.poisoned)
        .map(|h| h.sim.defense.health / h.sim.defense.params.maximum)
        .unwrap_or(0.)
}
/// Disc shield radius/health scale for the host's extracted guard model.
/// # Safety
/// Live fighter or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_shield_scale(h: *const Handle) -> f32 {
    unsafe { h.as_ref() }
        .filter(|h| !h.poisoned)
        .map(|h| h.sim.shield_scale())
        .unwrap_or(0.)
}
/// Collision-resolved self placement for a held grapple; preserves action clocks.
/// # Safety
/// Unique live handle or null, finite position.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_anchor(handle: *mut Handle, position: V3, grounded: u32) {
    if let Some(h) = unsafe { handle.as_mut() } {
        let p: Vec3 = position.into();
        if !h.poisoned
            && p.is_finite()
            && h.sim.entry.is_none()
            && !h.sim.rebirth.as_ref().is_some_and(|s| s.flags() & 1 != 0)
        {
            h.sim.position = p;
            h.sim.velocity = Vec3::ZERO;
            h.sim.grounded = grounded != 0;
        }
    }
}
/// Whether authored motion/capture currently owns yaw. Null is unlocked.
/// # Safety
/// Live handle or null; no concurrent mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_facing_locked(handle: *const Handle) -> u32 {
    unsafe { handle.as_ref() }
        .map(|h| h.sim.facing_locked() as u32)
        .unwrap_or(0)
}

impl View {
    fn refresh(&mut self) {
        let mut offset = 0;
        for i in 0..self.actor.model.batches.len() {
            let visible = self.actor.visible(i);
            self.colors[i] = self.actor.color(i);
            self.alpha_sources[i] = self.actor.alpha_source(i);
            let texture_matrix = self.actor.texture_matrix(i);
            for (p, n) in self.actor.model.skin(i, &self.actor.world) {
                self.positions[offset] = if visible { p.into() } else { V3::default() };
                self.normals[offset] = n.into();
                let uv = texture_matrix.transform_point3(glam::Vec3::new(self.uvs[offset * 2], self.uvs[offset * 2 + 1], 0.));
                self.frame_uvs[offset * 2] = uv.x;
                self.frame_uvs[offset * 2 + 1] = uv.y;
                offset += 1;
            }
        }
    }
    fn new(actor: Presentation) -> Self {
        let mut v = Self {
            actor,
            poisoned: false,
            info: ModelInfo::default(),
            batches: vec![],
            indices: vec![],
            uvs: vec![],
            frame_uvs: vec![],
            positions: vec![],
            normals: vec![],
            colors: vec![],
            alpha_sources: vec![],
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
                blend: b.blend,
                lit: b.lit,
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
            v.alpha_sources.push([b.color[3], 0., 0.]);
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
        v.frame_uvs = v.uvs.clone();
        v.refresh();
        v
    }
}
/// # Safety
/// UTF-8 cache path; costume index 0..4. Free with ssbm_view_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_new(dir: *const c_char, kind: u32, costume: u32) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        let result = (|| {
            let fighter = fighter_assets(kind)?;
            Presentation::load_for(unsafe { path(dir) }?, costume as usize, fighter)
        })();
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
/// UTF-8 cache path and Samus article kind 0..3/state. Free with ssbm_view_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_article_new(
    dir: *const c_char,
    fighter_kind: u32,
    article_kind: u32,
    state: u32,
) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        let result = (|| {
            let fighter = fighter_assets(fighter_kind)?;
            Presentation::article_for(unsafe { path(dir) }?, article_kind, state, fighter)
        })();
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
/// NUL-terminated asset path, common model index or Samus model ID 2000..2002.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_effect_new(dir: *const c_char, fighter_kind: u32, id: u32) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        let result = (|| {
            let fighter = fighter_assets(fighter_kind)?;
            Presentation::effect_for(unsafe { path(dir) }?, id, fighter)
        })();
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
/// Valid UTF-8 cache path; returned platform is freed with ssbm_view_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_entry_platform_new(dir: *const c_char) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        match (|| Presentation::platform(unsafe { path(dir) }?))() {
            Ok(a) => Box::into_raw(Box::new(View::new(a))),
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// # Safety
/// Valid UTF-8 cache path; the results-screen GAME! burst, freed with ssbm_view_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_game_new(dir: *const c_char) -> *mut View {
    guarded(std::ptr::null_mut(), || {
        match (|| Presentation::game(unsafe { path(dir) }?))() {
            Ok(a) => Box::into_raw(Box::new(View::new(a))),
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// # Safety
/// Unique live sim and valid UTF-8 cache path. Call after spawn placement.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_begin_entry(h: *mut Handle, dir: *const c_char) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || {
        let kind = unsafe { (&*h).kind };
        let Some(fighter) = kind.assets() else {
            error(format!("fighter kind {kind:?} not implemented"));
            return 0;
        };
        match (|| libssbm_assets::params::entry(unsafe { path(dir) }?, fighter))() {
            Ok(params) => {
                let h = unsafe { &mut *h };
                h.sim.interrupt();
                // Entry is requested only after the host's clear standing probe.
                // Leaving the platform must enter Wait, not a spurious landing.
                h.sim.grounded = true;
                h.sim.jumps_used = 0;
                h.sim.entry = Some(libssbm::entry::State::new(params));
                1
            }
            Err(e) => {
                error(e);
                0
            }
        }
    })
}
/// Shared Melee falloff: caller supplies a destination from common Unstuck.FellEarly.
/// # Safety
/// Unique live fighter, valid UTF-8 cache path and finite destination.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_begin_rebirth(
    h: *mut Handle,
    dir: *const c_char,
    destination: V3,
) -> u32 {
    unsafe { begin_return(h, dir, destination, false) }
}
/// Initial spawn uses the same automatic figurine return, without the fall KO.
/// # Safety
/// Unique live fighter, valid UTF-8 cache path. Host has placed it on safe ground.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_begin_arrival(h: *mut Handle, dir: *const c_char) -> u32 {
    unsafe { begin_return(h, dir, V3::default(), true) }
}
unsafe fn begin_return(h: *mut Handle, dir: *const c_char, destination: V3, initial: bool) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || {
        let kind = unsafe { (&*h).kind };
        let result = (|| -> libssbm_assets::Result<_> {
            let dir = unsafe { path(dir) }?;
            let fighter = kind
                .assets()
                .ok_or_else(|| format!("fighter kind {kind:?} not implemented"))?;
            Ok((
                libssbm_assets::params::rebirth(dir)?,
                libssbm_assets::params::entry(dir, fighter)?,
            ))
        })();
        match result {
            Ok((params, entry)) => {
                let h = unsafe { &mut *h };
                let started = if initial {
                    h.sim.begin_arrival(params, entry)
                } else {
                    h.sim.begin_rebirth(params, entry, destination.into())
                };
                if started {
                    h.audio.clear();
                    1
                } else {
                    0
                }
            }
            Err(e) => {
                error(e);
                0
            }
        }
    })
}
/// # Safety
/// Live fighter or null. Bits: 1 recovering, 2 invulnerable, 4 hidden KO,
/// 8 waiting figurine. Zero means normal play. Existing ABI layouts unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_rebirth_flags(h: *const Handle) -> u32 {
    if h.is_null() {
        return 0;
    }
    unsafe { (&*h).sim.rebirth.as_ref().map_or(0, |s| s.flags()) }
}
/// # Safety
/// Unique live fighter. kind=0 waits as a figurine; kind=1 starts its original
/// entry transform. Returns 1 while entry exists, zero once complete.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_entry_control(h: *mut Handle, kind: u32) -> u32 {
    if h.is_null() {
        return 0;
    }
    unsafe {
        let h = &mut *h;
        if let Some(entry) = h.sim.entry.as_mut() {
            if kind == 0 {
                entry.waiting = true;
            } else if kind == 1 {
                entry.waiting = false;
                entry.frame = entry.frame.max(entry.params.grow);
                entry.started = false;
            }
            1
        } else {
            0
        }
    }
}
/// # Safety
/// Unique live platform view and live sim. Returns zero once entry ends.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_entry_platform_update(v: *mut View, h: *const Handle) -> u32 {
    if v.is_null() || h.is_null() {
        return 0;
    }
    let Some(entry) = unsafe { &*h }.sim.entry else {
        return 0;
    };
    guarded(0, || {
        let v = unsafe { &mut *v };
        let scale = entry.params.platform;
        let transform = glam::Mat4::from_scale(Vec3::new(
            scale,
            (scale * entry.platform_y()).max(0.001),
            scale,
        ));
        v.actor.world = v
            .actor
            .model
            .bind_world()
            .into_iter()
            .map(|m| transform * m)
            .collect();
        v.refresh();
        1
    })
}
/// Optional ABI-4 presentation extension. Entry colour overlay from PlCo.
/// # Safety
/// Live sim and four writable floats, or null for a capability query (returns 0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_overlay(h: *const Handle, rgba: *mut f32) -> u32 {
    if h.is_null() || rgba.is_null() {
        return 0;
    }
    guarded(0, || unsafe {
        let h = &*h;
        if let Some(color) = h.sim.rebirth.as_ref().and_then(|s| s.overlay()) {
            std::ptr::copy_nonoverlapping(color.as_ptr(), rgba, 4);
            return (color[3] > 0.) as u32;
        }
        let Some(entry) = h.sim.entry else {
            if h.sim.shot_charge >= h.sim.behavior.charge_steps() {
                let color = h.rig.charge_flash[h.sim.frame as usize % h.rig.charge_flash.len()];
                std::ptr::copy_nonoverlapping(color.as_ptr(), rgba, 4);
                return (color[3] > 0.) as u32;
            }
            return 0;
        };
        if entry.waiting {
            return 0;
        }
        if entry.frame >= entry.params.color_frames {
            return 0;
        }
        std::ptr::copy_nonoverlapping(entry.params.color.as_ptr(), rgba, 4);
        1
    })
}
/// # Safety
/// Live object view. Animation duration from the disc; zero if no animation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_object_duration(view: *const View) -> f32 {
    if view.is_null() {
        return 0.;
    }
    unsafe { &*view }.actor.duration
}
/// # Safety
/// Unique live object view and finite nonnegative frame, main thread only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_object_update(v: *mut View, frame: f32) -> u32 {
    if v.is_null() || !frame.is_finite() || frame < 0. {
        return 0;
    }
    let v = unsafe { &mut *v };
    if v.poisoned {
        return 0;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        v.actor.object_pose(frame);
        v.refresh();
        1
    })) {
        Ok(n) => n,
        Err(_) => {
            v.poisoned = true;
            error("object view panic");
            0
        }
    }
}
/// # Safety
/// Unique live object view; camera is in its unmirrored model coordinates.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_object_camera(v: *mut View, camera: V3) -> u32 {
    if v.is_null() || !Vec3::from(camera).is_finite() {
        return 0;
    }
    guarded(0, || unsafe {
        let v = &mut *v;
        v.actor.face_camera(camera.into());
        v.refresh();
        1
    })
}
/// # Safety
/// Pose a named fighter clip without running gameplay physics or landing state.
/// Used by silent menu portraits; no sound or hit events are executed.
/// # Safety
/// Unique live view, NUL-terminated UTF-8 clip name, finite nonnegative frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_pose(v: *mut View, clip: *const c_char, frame: f32) -> u32 {
    if v.is_null() || clip.is_null() || !frame.is_finite() || frame < 0. {
        return 0;
    }
    let v = unsafe { &mut *v };
    if v.poisoned {
        return 0;
    }
    guarded(0, || {
        let Ok(clip) = unsafe { CStr::from_ptr(clip) }.to_str() else {
            return 0;
        };
        if !v.actor.animations.clips.contains_key(clip) {
            error("unknown portrait clip");
            return 0;
        }
        v.actor.clip = clip.into();
        v.actor.frame = frame;
        v.actor.ball = false;
        v.actor.hidden = false;
        v.actor.world = v.actor.animations.pose(&v.actor.model, clip, frame, false);
        v.refresh();
        1
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
            v.colors[i] = if let Some(color) = h.sim.entry_flash() {
                color
            } else if flash {
                [1., 0.9, 0.15, 1.]
            } else {
                v.actor.color(i)
            };
            v.alpha_sources[i] = if flash {
                [1., 0., 0.]
            } else {
                v.actor.alpha_source(i)
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
/// # Safety
/// Live view. `max` counts UV pairs/vertices, not individual floats, matching
/// IViewSource and the Mario/Olimar ABI. Output has room for 2*max floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_uvs(v: *const View, out: *mut f32, max: u32) -> u32 {
    if v.is_null() {
        return 0;
    }
    unsafe {
        let uv = &(&*v).uvs;
        let count = uv.len() / 2;
        if !out.is_null() {
            let n = count.min(max as usize);
            std::ptr::copy_nonoverlapping(uv.as_ptr(), out, n * 2);
        }
        count as u32
    }
}
/// # Safety
/// Live view and writable vertex count. Pointer survives until update/free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_frame_uvs(v: *const View, count: *mut u32) -> *const f32 {
    if v.is_null() || count.is_null() { return std::ptr::null(); }
    unsafe { *count = ((*v).frame_uvs.len() / 2) as u32; (*v).frame_uvs.as_ptr() }
}
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
pointer_export!(ssbm_view_alpha_sources, alpha_sources, [f32; 3]);
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
    let Some(t) = v.actor.texture(id as usize)
    else {
        return 0;
    };
    unsafe {
        *out = TextureInfo {
            width: t.width as u32,
            height: t.height as u32,
            wrap_s: t.wrap_s,
            wrap_t: t.wrap_t,
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
/// Live view or null. Zero means an immutable texture.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_view_texture_version(v: *const View, id: u32) -> u32 {
    if v.is_null() { return 0; }
    unsafe { (*v).actor.texture_frame(id as usize).map_or(0, |frame| frame as u32 + 1) }
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
/// Host lock-on: the world position (sim units) of the enemy the player is
/// locked onto or aiming nearest; `valid` 0 clears it. Staged specials (Pikachu's
/// Quick Attack) steer to it. Refresh every tick while a move is active.
/// # Safety
/// Unique live sim or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_lock_target(handle: *mut Handle, target: V3, valid: u32) {
    if let Some(h) = unsafe { handle.as_mut() } {
        let t: Vec3 = target.into();
        h.sim.lock_target = if valid != 0 && t.is_finite() { Some(t) } else { None };
    }
}
/// World facts the host reports to the fighter's staged specials (Link's boomerang
/// out, a carried bomb); see `HOST_*` in libssbm's `link.rs`. Set every tick.
/// # Safety
/// Unique live sim or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_set_host_flags(handle: *mut Handle, flags: u32) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.sim.host_flags = flags;
    }
}
/// The current move's hitbox connected with an enemy (Skull Bash stops on it).
/// # Safety
/// Unique live sim or null. Call outside a native host callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_connect(handle: *mut Handle) {
    if !handle.is_null() {
        guarded((), || unsafe { (&mut *handle).sim.attack_connected() });
    }
}
/// # Safety
/// Unique live sim or null. Call outside a native host callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_hurt(handle: *mut Handle) {
    if !handle.is_null() {
        guarded((), || unsafe { (&mut *handle).sim.hurt() });
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
    fn game_voice_uses_extracted_common_audio_bank() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !libssbm_assets::complete(&dir) {
            eprintln!("SKIP: disc cache absent");
            return Ok(());
        }
        // if_2F6E.c::ifStatus_803F9628[5] selects the GAME! cue.
        let bank = libssbm_assets::audio::Bank::load(&dir)?;
        assert!(!bank.sample_ids(0x7c855)?.is_empty());
        let mut mixer = libssbm_assets::audio::Mixer::new(bank);
        mixer.play(0x7c855, 127, 64, 0)?;
        let mut pcm = vec![0.; 32000 * 2];
        mixer.render(&mut pcm);
        assert!(
            pcm.iter().any(|x| x.abs() > 0.0001),
            "GAME cue decoded silence"
        );
        Ok(())
    }
    #[test]
    fn announcer_name_uses_extracted_name_bank() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !libssbm_assets::complete(&dir) {
            eprintln!("SKIP: disc cache absent");
            return Ok(());
        }
        let sound = libssbm_assets::fighter::SAMUS.announce_sound;
        assert_eq!(sound, 0x7C84E);
        let bank = libssbm_assets::audio::Bank::load(&dir)?;
        assert!(!bank.sample_ids(sound)?.is_empty());
        let mut mixer = libssbm_assets::audio::Mixer::new(bank);
        mixer.play(sound, 127, 64, 0)?;
        let mut pcm = vec![0.; 32000 * 2];
        mixer.render(&mut pcm);
        assert!(
            pcm.iter().any(|x| x.abs() > 0.0001),
            "announcer name cue decoded silence"
        );
        Ok(())
    }
    #[test]
    fn fox_assets_load_and_sim_steps() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let fox = &libssbm_assets::fighter::FOX;
        if !fox.complete(&dir) {
            eprintln!("SKIP: Fox disc cache absent");
            return Ok(());
        }
        // The exact load path ssbm_sim_new_fighter takes for FighterKind::Fox.
        let params = libssbm_assets::params::load_common(&dir, fox)?;
        let behavior: Box<dyn libssbm::behavior::Behavior> = Box::new(libssbm::fox::Fox::new(
            libssbm_assets::params::load_fox_specials(&dir)?,
        ));
        let mut sim = Sim::with_behavior(params, behavior, Vec3::ZERO);
        sim.defense.params = libssbm_assets::params::defense_for(&dir, fox)?;
        sim.defense.health = sim.defense.params.maximum;
        let rig = Rig::load_for(&dir, fox)?;
        sim.motion.definitions = rig.animations.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for(&dir, &rig.animations, fox)?;
        for (name, action) in &sim.action.definitions {
            if let Some(motion) = sim.motion.definitions.get_mut(name) {
                motion.looping |= action.looping;
            }
        }
        let _audio =
            libssbm_assets::audio::Mixer::new(libssbm_assets::audio::Bank::load_for(&dir, fox)?);
        // Fire Fox's Blaster and step through startup + recovery.
        let mut host = libssbm::sim::FlatHost { events: Vec::new() };
        let mut input = Input::idle();
        input.buttons = libssbm::sim::NEUTRAL_B;
        for _ in 0..120 {
            sim.advance(&mut host, input, 1. / 60.);
        }
        assert!(sim.position.is_finite());
        assert!(!sim.motion.clip.is_empty());
        Ok(())
    }
    #[test]
    fn luigi_assets_load_and_sim_steps() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let luigi = &libssbm_assets::fighter::LUIGI;
        if !luigi.complete(&dir) {
            eprintln!("SKIP: Luigi disc cache absent");
            return Ok(());
        }
        let params = libssbm_assets::params::load_common(&dir, luigi)?;
        let behavior: Box<dyn libssbm::behavior::Behavior> = Box::new(libssbm::luigi::Luigi::new(
            libssbm_assets::params::load_luigi_specials(&dir)?,
        ));
        let mut sim = Sim::with_behavior(params, behavior, Vec3::ZERO);
        sim.defense.params = libssbm_assets::params::defense_for(&dir, luigi)?;
        sim.defense.health = sim.defense.params.maximum;
        let rig = Rig::load_for(&dir, luigi)?;
        sim.motion.definitions = rig.animations.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for(&dir, &rig.animations, luigi)?;
        for (name, action) in &sim.action.definitions {
            if let Some(motion) = sim.motion.definitions.get_mut(name) {
                motion.looping |= action.looping;
            }
        }
        let _audio =
            libssbm_assets::audio::Mixer::new(libssbm_assets::audio::Bank::load_for(&dir, luigi)?);
        let mut host = libssbm::sim::FlatHost { events: Vec::new() };
        let mut input = Input::idle();
        input.buttons = libssbm::sim::NEUTRAL_B;
        for _ in 0..120 {
            sim.advance(&mut host, input, 1. / 60.);
        }
        assert!(sim.position.is_finite());
        assert!(!sim.motion.clip.is_empty());
        Ok(())
    }
    #[test]
    fn captain_and_link_assets_load_and_sim_steps() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for kind in [4u32, 5u32] {
            let fighter = libssbm_assets::fighter::FighterKind::from_u32(kind).unwrap();
            let profile = fighter.assets().unwrap();
            if !profile.complete(&dir) {
                eprintln!("SKIP: {kind} disc cache absent");
                continue;
            }
            let cdir = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
            let sim = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
            assert!(!sim.is_null(), "kind {kind} sim failed: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let mut host = HostTable { collide: None, event: None, user: std::ptr::null_mut() };
            let pad = Pad { movement: V3::default(), aim: V3 { x: 0., y: 0., z: 1. }, buttons: 0, allowed: 0x1f };
            for _ in 0..120 {
                assert_ne!(unsafe { ssbm_sim_step(sim, &mut host, &pad, 1. / 60.) }, 0, "kind {kind}: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            }
            let view = unsafe { ssbm_view_new(cdir.as_ptr(), kind, 0) };
            assert!(!view.is_null(), "kind {kind} view failed: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            assert_eq!(unsafe { ssbm_view_update(view, sim, 1. / 60.) }, 1, "kind {kind} view update: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            unsafe { ssbm_view_free(view) };
            unsafe { ssbm_sim_free(sim) };
        }
        Ok(())
    }
    #[test]
    fn captain_falcon_punch_hits_via_ffi() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let captain = &libssbm_assets::fighter::CAPTAIN;
        if !captain.complete(&dir) {
            eprintln!("SKIP: Captain disc cache absent");
            return Ok(());
        }
        let cdir = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
        let sim = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), 4) };
        assert!(!sim.is_null(), "{}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
        let mut host = HostTable { collide: None, event: None, user: std::ptr::null_mut() };
        let idle = Pad { movement: V3::default(), aim: V3 { x: 0., y: 0., z: 1. }, buttons: 0, allowed: 0x1f };
        for _ in 0..30 {
            unsafe { ssbm_sim_step(sim, &mut host, &idle, 1. / 60.) };
        }
        let press = Pad { buttons: libssbm::sim::NEUTRAL_B, ..idle };
        for _ in 0..3 {
            unsafe { ssbm_sim_step(sim, &mut host, &press, 1. / 60.) };
        }
        let mut max_hitboxes = 0u32;
        let mut max_gfx = 0u32;
        for _ in 0..120 {
            unsafe { ssbm_sim_step(sim, &mut host, &idle, 1. / 60.) };
            max_hitboxes = max_hitboxes.max(unsafe { ssbm_sim_hitboxes(sim, std::ptr::null_mut(), 0) });
            max_gfx = max_gfx.max(unsafe { ssbm_sim_gfx(sim, std::ptr::null_mut(), 0) });
        }
        eprintln!("Falcon Punch: max hitboxes {max_hitboxes} max gfx {max_gfx} err {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
        assert!(max_hitboxes > 0, "Falcon Punch produced no FFI hitboxes");
        assert!(max_gfx > 0, "Falcon Punch produced no FFI effects");
        unsafe { ssbm_sim_free(sim) };
        Ok(())
    }
    #[test]
    fn yoshi_and_donkey_assets_load_and_sim_steps() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for kind in [6u32, 7u32] {
            let fighter = libssbm_assets::fighter::FighterKind::from_u32(kind).unwrap();
            let profile = fighter.assets().unwrap();
            if !profile.complete(&dir) {
                eprintln!("SKIP: {kind} disc cache absent");
                continue;
            }
            let cdir = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
            let sim = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
            assert!(!sim.is_null(), "kind {kind} sim failed: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let mut host = HostTable { collide: None, event: None, user: std::ptr::null_mut() };
            let pad = Pad { movement: V3::default(), aim: V3 { x: 0., y: 0., z: 1. }, buttons: 0, allowed: 0x1f };
            for _ in 0..120 {
                assert_ne!(unsafe { ssbm_sim_step(sim, &mut host, &pad, 1. / 60.) }, 0, "kind {kind}: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            }
            assert_eq!(unsafe { ssbm_sim_begin_arrival(sim, cdir.as_ptr()) }, 1, "kind {kind} arrival: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let view = unsafe { ssbm_view_new(cdir.as_ptr(), kind, 0) };
            assert!(!view.is_null(), "kind {kind} view failed: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            assert_eq!(unsafe { ssbm_view_update(view, sim, 1. / 60.) }, 1, "kind {kind} view update: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            unsafe { ssbm_view_free(view) };
            unsafe { ssbm_sim_free(sim) };
        }
        Ok(())
    }
    #[test]
    fn link_articles_load() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let link = &libssbm_assets::fighter::LINK;
        if !link.complete(&dir) {
            eprintln!("SKIP: Link disc cache absent");
            return Ok(());
        }
        // Hero's Bow arrow (0), Boomerang (1) and Bomb (2) resolve their
        // state/special-word layouts from itCharItems.h.
        for (kind, state) in [(0u32, 0u32), (1, 0), (2, 0)] {
            let article = libssbm_assets::article::Article::load_for(&dir, kind, state, link)
                .map_err(|e| format!("Link article {kind}/{state}: {e}"))?;
            eprintln!("Link article {kind}: {} special words, {} commands", article.special.len(), article.commands.len());
        }
        Ok(())
    }
    #[test]
    fn pikachu_assets_load_through_the_constructor_and_sim_steps() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let pikachu = &libssbm_assets::fighter::PIKACHU;
        if !pikachu.complete(&dir) {
            eprintln!("SKIP: Pikachu disc cache absent");
            return Ok(());
        }
        let c = std::ffi::CString::new(dir.to_str().unwrap())?;
        let handle = unsafe { ssbm_sim_new_fighter(c.as_ptr(), V3::default(), 3) };
        assert!(!handle.is_null(), "ssbm_sim_new_fighter(Pikachu) failed");
        let h = unsafe { &mut *handle };
        // The same hand-off Quick Attack takes from the host.
        unsafe { ssbm_sim_lock_target(handle, V3 { x: 40., y: 0., z: 0. }, 1) };
        assert!(h.sim.lock_target.is_some());
        unsafe { ssbm_sim_lock_target(handle, V3::default(), 0) };
        assert!(h.sim.lock_target.is_none());
        let mut host = libssbm::sim::FlatHost { events: Vec::new() };
        let mut input = Input::idle();
        for _ in 0..30 {
            h.sim.advance(&mut host, input, 1. / 60.);
        }
        input.buttons = libssbm::sim::NEUTRAL_B;
        h.sim.advance(&mut host, input, 1. / 60.);
        input.buttons = 0;
        for _ in 0..90 {
            h.sim.advance(&mut host, input, 1. / 60.);
        }
        assert!(h.sim.position.is_finite());
        assert!(
            host.events.iter().any(|e| e.kind == libssbm::sim::EventKind::Hit && e.move_id == 20),
            "Thunder Shock never fired its projectile event"
        );
        unsafe { ssbm_sim_free(handle) };
        Ok(())
    }
    #[test]
    fn non_samus_views_have_expected_clips_and_visibility() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for fighter in [
            &libssbm_assets::fighter::FOX,
            &libssbm_assets::fighter::LUIGI,
            &libssbm_assets::fighter::PIKACHU,
        ] {
            if !fighter.complete(&dir) {
                eprintln!("SKIP: {} disc cache absent", fighter.data.root);
                continue;
            }
            let p = libssbm_assets::presentation::Presentation::load_for(&dir, 0, fighter)?;
            assert!(p.animations.clips.contains_key("Wait"), "{} has no canonical Wait clip", fighter.data.root);
            assert!(p.model.batches.len() > 0, "no model batches");
            let visible = (0..p.model.batches.len()).any(|i| p.visible(i));
            assert!(visible, "no visible batches for {}", fighter.data.root);
            // The in-game arrival (figurine return) must resolve the same way.
            let entry = libssbm_assets::params::entry(&dir, fighter);
            assert!(entry.is_ok(), "{} entry params: {:?}", fighter.data.root, entry.err());
            eprintln!("{} entry: {:?}", fighter.data.root, entry.as_ref().map(|e| (e.grow, e.release, e.flat, e.platform)));
            let rebirth = libssbm_assets::params::rebirth(&dir);
            assert!(rebirth.is_ok(), "rebirth params: {:?}", rebirth.err());
        }
        Ok(())
    }
    #[test]
    fn non_samus_arrival_hitboxes_and_view_do_not_fail() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for kind in [1u32, 2u32, 4u32, 5u32] {
            let fighter = libssbm_assets::fighter::FighterKind::from_u32(kind).unwrap();
            if !fighter.assets().unwrap().complete(&dir) {
                eprintln!("SKIP: kind {kind} disc cache absent");
                continue;
            }
            let cdir = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
            let sim = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
            assert!(!sim.is_null(), "kind {kind} sim failed: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            assert_eq!(unsafe { ssbm_sim_begin_arrival(sim, cdir.as_ptr()) }, 1, "kind {kind} arrival: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let mut host = HostTable { collide: None, event: None, user: std::ptr::null_mut() };
            let pad = Pad { movement: V3::default(), aim: V3 { x: 0., y: 0., z: 1. }, buttons: 0, allowed: 0 };
            for _ in 0..600 {
                assert_ne!(unsafe { ssbm_sim_step(sim, &mut host, &pad, 1. / 60.) }, 0);
                let n = unsafe { ssbm_sim_hitboxes(sim, std::ptr::null_mut(), 0) };
                if n > 0 {
                    let mut out = vec![HitboxSample::default(); n as usize];
                    assert_eq!(unsafe { ssbm_sim_hitboxes(sim, out.as_mut_ptr(), n) }, n, "kind {kind} hitboxes: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
                }
            }
            let view = unsafe { ssbm_view_new(cdir.as_ptr(), kind, 0) };
            assert!(!view.is_null(), "kind {kind} view failed");
            assert_eq!(unsafe { ssbm_view_update(view, sim, 1. / 60.) }, 1, "kind {kind} view update: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let mut info = ModelInfo::default();
            assert_eq!(unsafe { ssbm_view_model(view, &mut info) }, 1);
            assert!(info.vertex_count > 0, "kind {kind} view has no vertices");
            unsafe { ssbm_view_free(view) };
            unsafe { ssbm_sim_free(sim) };
        }
        Ok(())
    }
    #[test]
    fn arrival_leaves_every_fighter_full_size_and_visible() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for kind in [0u32, 1u32, 2u32, 4u32, 5u32] {
            let fighter = libssbm_assets::fighter::FighterKind::from_u32(kind).unwrap();
            if !fighter.assets().unwrap().complete(&dir) {
                eprintln!("SKIP: kind {kind} disc cache absent");
                continue;
            }
            let cdir = std::ffi::CString::new(dir.to_str().unwrap()).unwrap();
            let sim = unsafe { ssbm_sim_new_fighter(cdir.as_ptr(), V3::default(), kind) };
            assert!(!sim.is_null(), "kind {kind} sim failed: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            assert_eq!(unsafe { ssbm_sim_begin_arrival(sim, cdir.as_ptr()) }, 1, "kind {kind} arrival: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let mut host = HostTable { collide: None, event: None, user: std::ptr::null_mut() };
            let pad = Pad { movement: V3::default(), aim: V3 { x: 0., y: 0., z: 1. }, buttons: 0, allowed: 0 };
            for _ in 0..600 {
                assert_ne!(unsafe { ssbm_sim_step(sim, &mut host, &pad, 1. / 60.) }, 0);
            }
            let flags = unsafe { ssbm_sim_rebirth_flags(sim) };
            eprintln!("kind {kind}: rebirth_flags after 600 frames = {flags}");
            assert_eq!(flags, 0, "kind {kind} arrival did not complete (flags={flags})");
            let view = unsafe { ssbm_view_new(cdir.as_ptr(), kind, 0) };
            assert!(!view.is_null(), "kind {kind} view failed");
            assert_eq!(unsafe { ssbm_view_update(view, sim, 1. / 60.) }, 1, "kind {kind} view update: {}", ERROR.with(|e| e.borrow().to_string_lossy().into_owned()));
            let mut count = 0u32;
            let positions = unsafe { ssbm_view_positions(view, &mut count) };
            assert!(count > 0, "kind {kind} view has no vertices");
            let slice = unsafe { std::slice::from_raw_parts(positions, count as usize) };
            let max_abs = slice.iter().map(|v| Vec3::from(*v).length()).fold(0f32, f32::max);
            let max_y = slice.iter().map(|v| v.y.abs()).fold(0f32, f32::max);
            eprintln!("kind {kind}: {count} vertices, max|pos| = {max_abs}, max|y| = {max_y}");
            assert!(max_abs > 0.5, "kind {kind} model is flat/invisible after arrival (max|pos|={max_abs})");
            unsafe { ssbm_view_free(view) };
            unsafe { ssbm_sim_free(sim) };
        }
        Ok(())
    }
    #[test]
    fn every_disc_gfx_cue_resolves_a_real_posed_attachment() -> libssbm_assets::Result<()> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !libssbm_assets::complete(&dir) {
            eprintln!("SKIP: disc cache absent");
            return Ok(());
        }
        let mut h = Handle {
            sim: Sim::with_behavior(
                libssbm_assets::params::load_common(&dir, &libssbm_assets::fighter::SAMUS)?,
                Box::new(libssbm::samus::Samus::new(
                    libssbm_assets::params::load_samus_specials(&dir)?,
                    libssbm_assets::fighter::SAMUS.upper_body_root,
                )),
                Vec3::ZERO,
            ),
            rig: Rig::load_for(&dir, &libssbm_assets::fighter::SAMUS)?,
            audio: libssbm_assets::audio::Mixer::new(libssbm_assets::audio::Bank::load(&dir)?),
            kind: libssbm_assets::fighter::FighterKind::Samus,
            poisoned: false,
        };
        let actions = libssbm_assets::action::load(&dir, &h.rig.animations)?;
        let mut tested = 0;
        let mut cycle = 0;
        for (name, action) in actions {
            let clip: &'static str = Box::leak(name.into_boxed_str());
            for event in action.commands {
                if let libssbm::action::Command::Raw { opcode: 10, words } = event.command {
                    let bone = if words[0] & (1 << 15) != 0 {
                        usize::MAX
                    } else {
                        (words[0] >> 18 & 255) as usize
                    };
                    h.sim.action.effects = vec![libssbm::action::GfxCue {
                        id: words[1] >> 16,
                        bone,
                        common: words[0] & (1 << 17) != 0,
                        cycle,
                        destroy_on_change: words[0] & (1 << 16) != 0,
                        offset: Vec3::ZERO,
                        param: 0.,
                        clip,
                        frame: event.frame,
                        origin: Vec3::ZERO,
                        facing: Vec3::Z,
                    }];
                    if bone == 0x8d {
                        cycle = (cycle + 1) % 5;
                    }
                    let mut out = NativeEvent {
                        kind: 0,
                        move_id: 0,
                        strength: 0.,
                        position: V3::default(),
                        direction: V3::default(),
                    };
                    let n = unsafe { ssbm_sim_gfx(&h, &mut out, 1) };
                    assert_eq!(
                        n,
                        1,
                        "{clip} frame {}: {}",
                        event.frame,
                        ERROR.with(|s| s.borrow().to_string_lossy().into_owned())
                    );
                    assert!(Vec3::from(out.position).is_finite());
                    assert!((out.move_id & 65535) < h.rig.model.joints.len() as u32);
                    tested += 1;
                }
            }
        }
        assert!(tested > 100);
        eprintln!("resolved {tested} source GFX attachments");
        Ok(())
    }
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
                size_of::<HitboxSample>(),
                size_of::<particles::Sprite>(),
                size_of::<projectile::State>()
            ],
            [12, 32, 72, 20, 36, 24, 20, 40, 24, 12, 64, 88, 72, 56]
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
