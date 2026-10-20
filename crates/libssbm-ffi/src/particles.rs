//! Particle presentation boundary. All simulation/bytecode stays in libssbm;
//! the host receives snapshots and disc textures for its billboard renderer.
use super::*;
use libssbm::particle::System;
use libssbm_assets::{effects::Effects, model::Texture};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, Weak},
};
struct Bank {
    definitions: BTreeMap<u32, libssbm::particle::Emitter>,
    textures: Vec<Texture>,
    mapping: Vec<Vec<Vec<u32>>>,
}
impl Bank {
    fn load(dir: &Path) -> libssbm_assets::Result<Arc<Self>> {
        static BANKS: OnceLock<Mutex<BTreeMap<PathBuf, Weak<Bank>>>> = OnceLock::new();
        let path = dir.canonicalize()?;
        let mut cache = BANKS
            .get_or_init(|| Mutex::new(BTreeMap::new()))
            .lock()
            .map_err(|_| "particle bank cache poisoned")?;
        if let Some(bank) = cache.get(&path).and_then(Weak::upgrade) {
            return Ok(bank);
        }
        let mut bank = Self {
            definitions: BTreeMap::new(),
            textures: vec![],
            mapping: vec![],
        };
        // Every fighter's own effect bank, not just Samus's: a melee character's
        // ground/move VFX lives in its EfXxData.dat alongside the common EfCoData
        // bank. Emitter ids are bank-relative (common 0..591, Samus 2000..,
        // Fox 3000.., Luigi 18000..), so the banks merge without collisions.
        let archives = [
            libssbm_assets::fighter::COMMON_EFFECTS,
            libssbm_assets::fighter::SAMUS.effects,
            libssbm_assets::fighter::FOX.effects,
            libssbm_assets::fighter::LUIGI.effects,
        ];
        for archive in archives
            .into_iter()
            .chain(std::iter::once(libssbm_assets::fighter::PIKACHU.effects))
        {
            // Pikachu (7000..) is an optional bank: caches extracted before he
            // existed keep working for the other fighters.
            if archive.file == libssbm_assets::fighter::PIKACHU.effects.file
                && !dir.join(archive.file).exists()
            {
                continue;
            }
            let archive = Effects::load_archive(dir, archive)?;
            let base = bank.mapping.len() as u16;
            for group in archive.textures {
                let mut palettes = Vec::new();
                if let Some(group) = group {
                    for frames in group.palettes {
                        let mut ids = Vec::new();
                        for frame in frames {
                            ids.push(bank.textures.len() as u32);
                            bank.textures.push(frame);
                        }
                        palettes.push(ids);
                    }
                }
                bank.mapping.push(palettes);
            }
            for (id, mut emitter) in archive.emitters {
                emitter.texture += base;
                bank.definitions.insert(id, emitter);
            }
        }
        let bank = Arc::new(bank);
        cache.insert(path, Arc::downgrade(&bank));
        Ok(bank)
    }
}
pub struct Particles {
    system: System,
    bank: Arc<Bank>,
    clock: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Sprite {
    pub position: V3,
    pub velocity: V3,
    pub size: f32,
    pub rotation: f32,
    pub color: [f32; 4],
    pub environment: [f32; 4],
    pub texture: u32,
    pub flags: u32,
}
/// # Safety
/// UTF-8 NUL-terminated cache directory. Free returned handle exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_new(dir: *const c_char) -> *mut Particles {
    guarded(std::ptr::null_mut(), || {
        let result = (|| Bank::load(unsafe { path(dir) }?))();
        match result {
            Ok(bank) => {
                let mut system = System::default();
                system.definitions = bank.definitions.clone();
                Box::into_raw(Box::new(Particles {
                    system,
                    bank,
                    clock: 0.,
                }))
            }
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// # Safety
/// Live unique handle or null; consume once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_free(handle: *mut Particles) {
    if !handle.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(handle) }));
    }
}
unsafe fn matrix(ptr: *const f32) -> Option<glam::Mat4> {
    if ptr.is_null() {
        return None;
    }
    let a = unsafe { std::slice::from_raw_parts(ptr, 16) };
    let m = glam::Mat4::from_cols_array(a.try_into().ok()?);
    m.is_finite().then_some(m)
}
/// # Safety
/// Unique handle; transform points to 16 column-major f32 elements in sim units.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_spawn(
    handle: *mut Particles,
    id: u32,
    transform: *const f32,
    key: u64,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    guarded(0, || {
        let Some(transform) = (unsafe { matrix(transform) }) else {
            error("invalid particle transform");
            return 0;
        };
        match unsafe { &mut *handle }.system.spawn(id, transform, key) {
            Ok(()) => 1,
            Err(e) => {
                error(e);
                0
            }
        }
    })
}
/// # Safety
/// Unique handle and 16 readable column-major f32 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_attach(
    handle: *mut Particles,
    key: u64,
    transform: *const f32,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    guarded(0, || {
        let Some(transform) = (unsafe { matrix(transform) }) else {
            return 0;
        };
        unsafe { &mut *handle }.system.attach(key, transform);
        1
    })
}
/// # Safety
/// Unique handle, or null. Stop generators and their attached particles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_stop(handle: *mut Particles, key: u64) {
    if !handle.is_null() {
        guarded((), || unsafe { &mut *handle }.system.stop(key));
    }
}
/// # Safety
/// Unique live handle. dt is seconds; fixed stepping retains fractional ticks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_step(handle: *mut Particles, dt: f64) -> u32 {
    if handle.is_null() || !dt.is_finite() || dt < 0. {
        return 0;
    }
    guarded(0, || {
        let h = unsafe { &mut *handle };
        h.clock += dt.min(0.25);
        while h.clock + 1e-10 >= 1. / 60. {
            h.clock -= 1. / 60.;
            h.system.tick();
        }
        if !h.system.errors.is_empty() {
            error(h.system.errors.remove(0));
            return 0;
        }
        1
    })
}
/// # Safety
/// Live handle, writable max sprites or null to query count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_draws(
    handle: *const Particles,
    out: *mut Sprite,
    max: u32,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    let count = h
        .system
        .particles
        .iter()
        .filter(|p| p.kind & (1 << 10) != 0)
        .count() as u32;
    if out.is_null() {
        return count;
    }
    guarded(0, || {
        for (i, p) in h
            .system
            .particles
            .iter()
            .filter(|p| p.kind & (1 << 10) != 0)
            .take(max as usize)
            .enumerate()
        {
            let Some(palettes) = h.bank.mapping.get(p.texture as usize) else {
                error("particle texture group outside bank");
                return 0;
            };
            let palette = if p.palette == 255 {
                0
            } else {
                p.palette as usize
            };
            let Some(texture) = palettes
                .get(palette)
                .and_then(|frames| frames.get(p.frame as usize))
                .copied()
            else {
                error("particle texture frame/palette outside group");
                return 0;
            };
            let size_scale = p.transform.transform_vector3(Vec3::Y).length();
            let result = Sprite {
                position: p.transform.transform_point3(p.position).into(),
                velocity: p.transform.transform_vector3(p.velocity).into(),
                size: p.size * size_scale,
                rotation: p.rotation,
                color: p.color.to_array(),
                environment: p.environment.to_array(),
                texture,
                flags: p.kind,
            };
            unsafe {
                *out.add(i) = result;
            }
        }
        count
    })
}
/// # Safety
/// Live handle, writable texture info and len RGBA bytes; null rgba queries size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_texture(
    handle: *const Particles,
    id: u32,
    info: *mut TextureInfo,
    rgba: *mut u8,
    len: u32,
) -> u32 {
    if handle.is_null() || info.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    guarded(0, || {
        let Some(texture) = h.bank.textures.get(id as usize) else {
            return 0;
        };
        unsafe {
            *info = TextureInfo {
                width: texture.width as u32,
                height: texture.height as u32,
                wrap_s: 0,
                wrap_t: 0,
                pad0: 0,
                pad1: 0,
            };
        }
        if !rgba.is_null() {
            if len < texture.rgba.len() as u32 {
                return 0;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(texture.rgba.as_ptr(), rgba, texture.rgba.len());
            }
        }
        1
    })
}
