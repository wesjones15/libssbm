//! Native article simulation; all positions are Melee units relative to spawn.
use super::*;
use libssbm::action::Command;
use libssbm::projectile::{Params, Projectile};
use libssbm_assets::article::Article;

pub struct Handle {
    sim: Projectile,
    element: u32,
    notified: bool,
    explosion: Vec<libssbm::action::TimedCommand>,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub position: V3,
    pub velocity: V3,
    pub frame: f32,
    pub animation_frame: f32,
    pub damage: f32,
    pub radius: f32,
    pub kind: u32,
    pub state: u32,
    pub flags: u32,
    pub element: u32,
}
/// Host adaptation for open-world travel. Zero disables the source offscreen
/// lifetime; positive values retain a finite fixed-tick lifetime.
/// # Safety
/// Unique live projectile handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_lifetime(h: *mut Handle, frames: f32) {
    if !h.is_null() && frames.is_finite() && frames >= 0. {
        unsafe {
            (&mut *h).sim.params.life = if frames == 0. { f32::INFINITY } else { frames };
        }
    }
}
/// # Safety
/// NUL-terminated asset path. New unique handle, freed with projectile_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_new(
    dir: *const c_char,
    fighter_kind: u32,
    kind: u32,
    state: u32,
    charge: f32,
    direction: V3,
) -> *mut Handle {
    guarded(std::ptr::null_mut(), || {
        let result = (|| -> libssbm_assets::Result<Handle> {
            let dir = unsafe { path(dir) }?;
            if !charge.is_finite() {
                return Err("invalid projectile request".into());
            }
            let fighter = super::fighter_assets(fighter_kind)?;
            let article = Article::load_for(dir, kind, state, fighter)?;
            let hit = article
                .commands
                .iter()
                .find_map(|c| match &c.command {
                    Command::Hitbox(h) => Some(h),
                    _ => None,
                })
                .ok_or("article has no source hitbox")?;
            let a = &article.special;
            let mut p = Params {
                kind,
                state,
                life: a[0],
                speed: 0.,
                acceleration: 0.,
                max_speed: 0.,
                delay: 0.,
                homing_frames: 0.,
                decay: 1.,
                min_speed: 0.,
                turn_accel: 0.,
                max_turn: 0.,
                min_angle: 0.,
                gravity: article.common[4],
                max_fall: article.common[5],
                initial_y: 0.,
                damage: hit.damage,
                radius: hit.radius,
                explosion_radius: hit.radius,
                explosion_frames: 0.,
            };
            match fighter_kind {
                0 => match kind {
                    0 => {
                        p.initial_y = article.common[6];
                        let explosion = Article::load_for(dir, 0, 1, fighter)?;
                        p.explosion_radius = explosion
                            .commands
                            .iter()
                            .find_map(|c| match &c.command {
                                Command::Hitbox(h) => Some(h.radius),
                                _ => None,
                            })
                            .ok_or("bomb explosion hitbox absent")?;
                        p.explosion_frames = 28.;
                    }
                    1 => {
                        p.speed = a[2] + (a[3] - a[2]) * charge.clamp(0., 1.);
                    }
                    2 if state == 0 => {
                        p.life = a[1];
                        p.homing_frames = a[2];
                        p.speed = a[3];
                        p.decay = a[4];
                        p.min_speed = a[5];
                        p.turn_accel = a[6];
                        p.max_turn = a[7];
                        p.min_angle = a[8];
                        p.explosion_frames = 30.;
                    }
                    2 => {
                        p.life = a[9];
                        p.delay = a[10];
                        p.speed = a[11];
                        p.acceleration = a[12];
                        p.max_speed = a[13];
                        p.explosion_frames = 40.;
                    }
                    _ => (),
                },
                // Fox's Blaster laser: a straight, fast projectile. The shot
                // article's own life; speed from ftFx_DatAttrs.x14.
                1 => {
                    p.life = a[0];
                    p.speed = libssbm_assets::params::load_fox_specials(dir)?.blaster_speed;
                }
                _ => (),
            }
            // Expiration is encoded by the source item's var2 command, rather
            // than a host timer. Bomb explosion uses animation 1, missile 2/3.
            let mut explosion_commands = Vec::new();
            if fighter_kind == 0 && kind != 1 {
                let explosion = Article::load_for(dir, kind, if kind == 0 { 1 } else { state + 2 }, fighter)?;
                let end=explosion.commands.iter().find(|c| matches!(&c.command,Command::Raw {opcode:19,words} if words[0]&0x03ffffff!=0)).ok_or("missing explosion end command")?;
                p.explosion_frames = end.frame;
                explosion_commands = explosion.commands;
            }
            Ok(Handle {
                sim: Projectile::new(p, direction.into()),
                element: hit.element,
                notified: false,
                explosion: explosion_commands,
            })
        })();
        match result {
            Ok(h) => Box::into_raw(Box::new(h)),
            Err(e) => {
                error(e);
                std::ptr::null_mut()
            }
        }
    })
}
/// # Safety
/// Unique live handle or null; target is relative to the projectile's spawn.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_step(
    h: *mut Handle,
    dt: f64,
    target: V3,
    has_target: u32,
) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || unsafe {
        (&mut *h).sim.advance(
            dt,
            if has_target != 0 {
                Some(target.into())
            } else {
                None
            },
        )
    })
}
/// # Safety
/// Live handle and writable State.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_state(h: *const Handle, out: *mut State) -> u32 {
    if h.is_null() || out.is_null() {
        return 0;
    }
    guarded(0, || unsafe {
        let h = &*h;
        let s = &h.sim;
        let mut damage = s.params.damage;
        let mut radius = if s.impacted {
            s.params.explosion_radius
        } else {
            s.params.radius
        };
        if s.params.kind == 0 && s.impacted {
            damage = 0.;
            for cue in h
                .explosion
                .iter()
                .take_while(|cue| cue.frame <= s.animation_frame as f32)
            {
                match &cue.command {
                    Command::Hitbox(hit) => {
                        damage = hit.damage;
                        radius = hit.radius;
                    }
                    Command::HitboxDamage(0, value) => damage = *value,
                    Command::HitboxRadius(0, value) => radius = *value,
                    Command::ClearHitboxes | Command::ClearHitbox(0) => damage = 0.,
                    _ => (),
                }
            }
        }
        *out = State {
            position: s.position.into(),
            velocity: s.velocity.into(),
            frame: s.frame as f32,
            animation_frame: s.animation_frame as f32,
            damage,
            radius,
            kind: s.params.kind,
            state: s.state(),
            flags: u32::from(s.impacted) | (u32::from(s.finished) << 1),
            element: h.element,
        };
        1
    })
}
/// # Safety
/// Unique live projectile. Position is relative to spawn; grounded bombs settle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_place(h: *mut Handle, position: V3, grounded: u32) {
    if !h.is_null() {
        guarded((), || unsafe {
            let s = &mut (&mut *h).sim;
            s.position = position.into();
            s.grounded = grounded != 0;
            if s.grounded {
                s.velocity = Vec3::ZERO;
            }
        });
    }
}
/// # Safety
/// Unique live projectile and optional unique fighter audio handle. Returns
/// source impact GFX ID (0 for none). Repeated calls never replay the sound.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_impact(h: *mut Handle, audio: *mut super::Handle) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || unsafe {
        let h = &mut *h;
        if h.notified {
            return 0;
        }
        h.notified = true;
        if !h.sim.impacted {
            h.sim.impact();
        }
        // Host calls once on impact transition. it_80272A60 / it_80272B40.
        if h.sim.params.kind != 1 {
            if !audio.is_null() {
                if let Err(e) = (&mut *audio).audio.play(116, 127, 64, 0) {
                    error(e);
                }
            }
            if h.sim.params.kind == 2 && h.sim.params.state == 1 {
                1038
            } else {
                1036
            }
        } else {
            1001
        }
    })
}
/// # Safety
/// Handle is null or unique, consumed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_free(h: *mut Handle) {
    if !h.is_null() {
        guarded((), || drop(unsafe { Box::from_raw(h) }));
    }
}

/// Shared fighter hit feedback. IDs follow ftcoll.c::hit_effect_ids and
/// lbcollision.c::lbColl_80005BB0. Unknown sound kinds are silent, never guessed.
/// # Safety
/// Optional unique fighter audio handle; call outside a sim callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_hit_feedback(
    audio: *mut super::Handle,
    element: u32,
    kind: u32,
    severity: u32,
) -> u32 {
    guarded(0, || {
        let severity = severity.min(2);
        let sound = match kind {
            1 => Some(91 - severity),
            2 => Some(88 - severity),
            3 => Some(111 + severity),
            4 => Some(84),
            5 => Some(if severity == 2 { 223 } else { 90 - severity }),
            6 => Some(225),
            7 => Some(98 + severity),
            8 => Some(101 + severity),
            11 => Some(94 - severity),
            _ => None,
        };
        if let Some(sound) = sound {
            if !audio.is_null() {
                if let Err(e) = unsafe { (&mut *audio).audio.play(sound, 127, 64, 0) } {
                    error(e);
                }
            }
        }
        match element {
            0 | 9 | 10 => 1000,
            1 => 1002,
            2 => 1001,
            3 => 1004,
            _ => 0,
        }
    })
}
