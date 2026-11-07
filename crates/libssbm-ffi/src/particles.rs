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
        // Only the common bank is required. Each fighter's bank is loaded when its file is in
        // the cache: a fighter that isn't extracted (hidden as unfinished, or never asked for)
        // must not take every other fighter's effects down with it. Pikachu (7000..), Captain
        // Falcon (4000..) and Yoshi (9000..) were optional already; Samus, Fox and Luigi used to
        // be required, which broke every Melee view once Fox stopped being extracted by default.
        // Link's and Donkey Kong's archives hold effect models only.
        let optional = [
            libssbm_assets::fighter::SAMUS.effects,
            libssbm_assets::fighter::FOX.effects,
            libssbm_assets::fighter::LUIGI.effects,
            libssbm_assets::fighter::PIKACHU.effects,
            libssbm_assets::fighter::CAPTAIN.effects,
            libssbm_assets::fighter::YOSHI.effects,
        ];
        for archive in [libssbm_assets::fighter::COMMON_EFFECTS].into_iter().chain(optional) {
            if archive.file != libssbm_assets::fighter::COMMON_EFFECTS.file && !dir.join(archive.file).exists() {
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
    /// Host presentation overrides for one emitter: (size multiplier, alpha multiplier,
    /// radial glow). The source's 0x4BD aura has a 27.5-unit additive sprite whose texture
    /// holds its deepest colour in a corner; Pikachu draws a soft centred radial mask.
    tuning: BTreeMap<u32, (f32, f32, bool)>,
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
    /// Host presentation: draw a soft radial mask instead of the source texture.
    pub glow: u32,
}
/// # Safety
/// UTF-8 NUL-terminated cache directory. Free returned handle exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_new(dir: *const c_char) -> *mut Particles {
    guarded("ssbm_particles_new", std::ptr::null_mut(), || {
        let result = (|| Bank::load(unsafe { path(dir) }?))();
        match result {
            Ok(bank) => {
                let mut system = System::default();
                system.definitions = bank.definitions.clone();
                Box::into_raw(Box::new(Particles {
                    system,
                    bank,
                    clock: 0.,
                    tuning: BTreeMap::new(),
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
        guarded("ssbm_particles_free", (), || drop(unsafe { Box::from_raw(handle) }));
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
    guarded("ssbm_particles_spawn", 0, || {
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
    guarded("ssbm_particles_attach", 0, || {
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
        guarded("ssbm_particles_stop", (), || unsafe { &mut *handle }.system.stop(key));
    }
}
/// # Safety
/// Unique handle, or null. Host presentation override for one emitter's size, alpha and
/// whether to replace the source texture with a soft radial glow mask.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_tune(
    handle: *mut Particles,
    id: u32,
    scale: f32,
    alpha: f32,
    glow: u32,
) -> u32 {
    if handle.is_null() || !scale.is_finite() || !alpha.is_finite() || scale <= 0. || alpha < 0. {
        return 0;
    }
    guarded("ssbm_particles_tune", 0, || {
        unsafe { &mut *handle }.tuning.insert(
            id,
            (scale.clamp(0.01, 4.), alpha.clamp(0., 1.), glow != 0),
        );
        1
    })
}
/// # Safety
/// Unique live handle. dt is seconds; fixed stepping retains fractional ticks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_particles_step(handle: *mut Particles, dt: f64) -> u32 {
    if handle.is_null() || !dt.is_finite() || dt < 0. {
        return 0;
    }
    guarded("ssbm_particles_step", 0, || {
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
        .filter(|p| p.kind & ((1 << 10) | (1 << 22)) != 0)
        .count() as u32;
    if out.is_null() {
        return count;
    }
    guarded("ssbm_particles_draws", 0, || {
        for (i, p) in h
            .system
            .particles
            .iter()
            .filter(|p| p.kind & ((1 << 10) | (1 << 22)) != 0)
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
            let (tune_size, tune_alpha, tune_glow) =
                h.tuning.get(&p.emitter).copied().unwrap_or((1., 1., false));
            let mut color = p.color.to_array();
            color[3] *= tune_alpha;
            let result = Sprite {
                position: p.transform.transform_point3(p.position).into(),
                velocity: p.transform.transform_vector3(p.velocity).into(),
                size: p.size * size_scale * tune_size,
                rotation: p.rotation,
                color,
                environment: p.environment.to_array(),
                texture,
                flags: p.kind,
                glow: tune_glow as u32,
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
    guarded("ssbm_particles_texture", 0, || {
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

#[cfg(test)]
mod tests {
    use super::Bank;
    use std::path::{Path, PathBuf};

    /// A Melee cache with the common and fighter effect banks (`SSBM_ASSETS`, or this checkout's
    /// assets/), if there is one.
    fn cache() -> Option<PathBuf> {
        let dir = std::env::var_os("SSBM_ASSETS")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"));
        (dir.join("EfCoData.dat").is_file() && dir.join("EfSsData.dat").is_file()).then_some(dir)
    }

    /// A release install only extracts the finished fighters (Samus and Yoshi), so Fox's and
    /// Luigi's banks are missing. The bank must still load from what is there; only the common
    /// bank is required.
    #[test]
    fn the_bank_loads_without_the_hidden_fighters_effects() {
        let Some(src) = cache() else {
            eprintln!("SKIP: no Melee cache with EfCoData.dat and EfSsData.dat");
            return;
        };
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/test-particles-partial");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for file in ["EfCoData.dat", "EfSsData.dat"] {
            std::fs::copy(src.join(file), dir.join(file)).unwrap();
        }
        let bank = Bank::load(&dir).expect("common + Samus only must load");
        assert!(bank.definitions.keys().any(|id| *id < 2000), "common emitters");
        assert!(bank.definitions.keys().any(|id| (2000..3000).contains(id)), "Samus's emitters");
        assert!(!bank.definitions.keys().any(|id| (3000..4000).contains(id)), "no Fox emitters without his file");

        std::fs::remove_file(dir.join("EfCoData.dat")).unwrap();
        let empty = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/test-particles-nocommon");
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::copy(src.join("EfSsData.dat"), empty.join("EfSsData.dat")).unwrap();
        assert!(Bank::load(&empty).is_err(), "the common bank is still required");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
    }
}