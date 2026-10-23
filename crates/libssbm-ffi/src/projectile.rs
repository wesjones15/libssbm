//! Native article simulation; all positions are Melee units relative to spawn.
use super::*;
use libssbm::action::Command;
use libssbm::projectile::{Params, Projectile};
use libssbm_assets::article::Article;

/// Link's boomerang flies this much further than the source (host adaptation).
const BOOMERANG_HOST_SCALE: f32 = 2.;
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
/// One beam of the Super Scope at a charge level 0..=9, flying along `direction`: the disc's speed, life, size and
/// damage for that level (itsscopebeam.c). The hitbox is the level's radius times its scale.
/// # Safety
/// NUL-terminated asset path. New unique handle, freed with projectile_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_scope_beam_new(dir: *const c_char, level: u32, direction: V3) -> *mut Handle {
    guarded(std::ptr::null_mut(), || {
        let result = (|| -> libssbm_assets::Result<Handle> {
            let dir = unsafe { path(dir) }?;
            let beams = libssbm_assets::params::load_scope_beams(dir)?;
            let b = *beams.get(level as usize).ok_or("invalid scope beam level")?;
            let radius = b.radius * b.scale;
            let p = Params {
                kind: 0x26,
                state: level,
                life: b.lifetime,
                speed: b.velocity,
                acceleration: 0.,
                max_speed: 0.,
                delay: 0.,
                homing_frames: 0.,
                decay: 1.,
                min_speed: 0.,
                turn_accel: 0.,
                max_turn: 0.,
                min_angle: 0.,
                gravity: 0.,
                max_fall: 0.,
                initial_y: 0.,
                damage: b.damage,
                radius,
                explosion_radius: radius,
                explosion_frames: 0.,
                max_bounces: 0,
                bounce_factor: 0.,
            };
            Ok(Handle { sim: Projectile::new(p, direction.into()), element: b.element, notified: false, explosion: Vec::new() })
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
            // Several fighters (Link's arrow/bomb, Yoshi's egg, and most of the
            // rest) author their hitbox dynamically in the item C code rather
            // than in the item script, so `article.commands` has none. A shared
            // prototype fixture stands in for those instead of rejecting the
            // projectile outright: without it, only the few articles that happen
            // to carry a script hitbox (Luigi's fireball, Pikachu's jolt) ever
            // spawned, and every other fighter's projectile was destroyed here.
            let hit = article
                .commands
                .iter()
                .find_map(|c| match &c.command {
                    Command::Hitbox(h) => Some(h),
                    _ => None,
                });
            let (damage, radius, element) = match hit {
                Some(h) => (h.damage, h.radius, h.element),
                None => (8., 2.5, 0),
            };
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
                damage,
                radius,
                explosion_radius: radius,
                explosion_frames: 0.,
                max_bounces: 0,
                bounce_factor: 0.5,
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
                // Luigi's Fireball: speed and life from the item's special
                // attributes (itLuigiFireballAttributes x0/x4). It flies level
                // until the host's 200 m / map-bounds cap, falls onto the floor,
                // and bounces ten times at a constant height (bounce_factor 1)
                // before despawning; it dies early on an enemy or a wall.
                2 => {
                    p.speed = a[0] * 1.6;
                    p.life = a[1];
                    // The disc item carries no gravity; give it a gentle fall so
                    // it reaches the ground and bounces instead of flying level.
                    p.gravity = 0.06;
                    p.initial_y = -0.4;
                    p.max_bounces = 10;
                    p.bounce_factor = 1.0;
                }
                // Link's projectiles: Hero's Bow arrow (0), Boomerang (1) and
                // Bomb (2). Speeds are prototype fixtures; the arrow scales with
                // the bow's charge, and the bomb lobs down to detonate.
                5 => match kind {
                    // Hero's Bow arrow (itLinkArrow_802A850C): speed climbs with the
                    // charge from the item's min to max (words 1 and 2, stored as
                    // integers) and it flies for the item's life (word 0). The source
                    // authors the damage in code; 4% to 12% stands in. The arrow sags
                    // gently over distance (a tenth of the item's gravity).
                    0 => {
                        let word = |i: usize| a.get(i).map_or(0., |v| v.to_bits() as f32);
                        let charge = charge.clamp(0., 1.);
                        p.speed = word(1) + (word(2) - word(1)) * charge;
                        p.life = word(0).max(1.);
                        p.gravity = article.common[4] * 0.1;
                        p.damage = 4. + 8. * charge;
                    }
                    // The Boomerang (itLinkBoomerang): throw speed from the fighter's
                    // ext attrs, deceleration, floor and return speed from the item.
                    // Distances are doubled for open worlds (host adaptation): speeds,
                    // deceleration and the catch radius scale together so the path
                    // keeps its shape.
                    1 => {
                        let link = libssbm_assets::params::load_link_specials(dir)?;
                        let k = BOOMERANG_HOST_SCALE;
                        p.speed = link.boomerang_speed_b * k;
                        p.life = a[0].to_bits() as f32 + a[1].to_bits() as f32;
                        p.gravity = 0.;
                    }
                    2 => {
                        p.speed = 4.;
                        p.life = 60.;
                        p.gravity = 0.12;
                        p.initial_y = 0.4;
                        p.explosion_frames = 30.;
                    }
                    _ => (),
                },
                // Pikachu's Thunder Shock (article 1, the travelling orb): shot
                // along the crosshair, then bouncing off surfaces like Luigi's
                // fireball until its bounce budget is spent. Speed from
                // itPikachutJoltGroundAttributes.x8; the disc item has no
                // gravity, so the shot arcs with the same gentle fall.
                3 if kind == 1 => {
                    p.speed = a[2] * 1.6;
                    p.life = a[0];
                    p.gravity = 0.06;
                    p.max_fall = 2.;
                    p.max_bounces = 10;
                    p.bounce_factor = 1.0;
                }
                // Yoshi's Egg Throw (article 0, the thrown egg): a ballistic
                // lob that arcs down and bounces. Speed from ftYs_DatAttrs.xFC;
                // the disc item carries no gravity, so it falls with a gentle
                // arc like Luigi's fireball.
                6 if kind == 0 => {
                    // The source's throw speed grows with how long the reticle was up
                    // (x4 * x100 + xFC). The host owns the reticle, so it passes the
                    // launch speed it wants in `charge`; zero keeps the disc's base.
                    p.speed = if charge > 0. {
                        charge
                    } else {
                        libssbm_assets::params::load_yoshi_specials(dir)?.egg_throw_speed
                    };
                    p.life = a[0];
                    p.gravity = 0.08;
                    p.max_fall = 4.;
                    p.max_bounces = 4;
                    p.bounce_factor = 0.9;
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
            let mut sim = Projectile::new(p, direction.into());
            if fighter_kind == 5 && kind == 1 {
                let k = BOOMERANG_HOST_SCALE;
                sim.boomerang = Some(libssbm::projectile::Boomerang {
                    decel: a[3] * k,
                    min_speed: a[6] * k,
                    max_speed: a[5] * k,
                    // x24 degrees a tick in the source; quadrupled here so the return
                    // arc closes inside an open world (host adaptation).
                    turn: a[9].to_radians() * 4.,
                    catch_radius: a[11] * k,
                });
            }
            if fighter_kind == 3 && kind == 1 {
                sim.arc = true;
                // The orb hits harder in flight than the ground jolt it becomes.
                let ground = Article::load_for(dir, kind, 1, fighter)?;
                sim.bounce_damage = ground
                    .commands
                    .iter()
                    .find_map(|c| match &c.command {
                        Command::Hitbox(h) => Some(h.damage),
                        _ => None,
                    })
                    .unwrap_or(0.);
            }
            Ok(Handle {
                sim,
                element,
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
        let mut damage = if s.bounce_damage > 0. && s.bounced() > 0 {
            s.bounce_damage
        } else {
            s.params.damage
        };
        let mut radius = if s.impacted {
            s.params.explosion_radius
        } else {
            s.params.radius
        };
        // An impacted kind-0 article that has explosion commands (Samus's bomb) takes
        // its damage and radius from them. One without (Fox's laser, Link's arrow,
        // Yoshi's egg) has no explosion animation to supply any, so it keeps the
        // damage and radius it was authored with.
        if s.params.kind == 0 && s.impacted && !h.explosion.is_empty() {
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
            flags: u32::from(s.impacted) | (u32::from(s.finished) << 1) | (u32::from(s.returning()) << 2),
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
/// Reflect a projectile off the ground. Returns 1 while it keeps flying and 0
/// once its bounce budget is exhausted (the projectile has terminated). The host
/// calls this on a downward ground contact and re-places the projectile on the
/// bounce point.
/// # Safety
/// Unique live projectile or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_bounce(h: *mut Handle) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || unsafe { (&mut *h).sim.bounce() as u32 })
}
/// Reflect a ballistic shot off a wall or ceiling (surface normal `n`). Returns
/// 1 while it keeps flying and 0 once its surface-contact budget is exhausted
/// (the projectile has terminated), matching [`ssbm_projectile_bounce`].
/// # Safety
/// Unique live projectile or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_projectile_reflect(h: *mut Handle, n: V3) -> u32 {
    if h.is_null() {
        return 0;
    }
    guarded(0, || unsafe { (&mut *h).sim.reflect(n.into()) as u32 })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn assets() -> Option<(std::path::PathBuf, CString)> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !libssbm_assets::fighter::LINK.complete(&dir) {
            eprintln!("SKIP: Link disc cache absent");
            return None;
        }
        let c = CString::new(dir.to_str().unwrap()).unwrap();
        Some((dir, c))
    }
    fn v(x: f32, y: f32, z: f32) -> V3 {
        V3 { x, y, z }
    }

    /// State of a freshly made article right after `impact` and again `later` ticks on
    /// (the explosion cues are timed by the article's animation). `None` when the fighter's
    /// disc cache is absent.
    fn after_impact(fighter: &libssbm_assets::fighter::FighterAssets, kind: u32, article: u32, state: u32, charge: f32, later: usize) -> Option<(State, State)> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        if !fighter.complete(&dir) {
            eprintln!("SKIP: fighter {kind} disc cache absent");
            return None;
        }
        let c = CString::new(dir.to_str().unwrap()).unwrap();
        unsafe {
            let h = ssbm_projectile_new(c.as_ptr(), kind, article, state, charge, v(0., 0., 1.));
            assert!(!h.is_null(), "fighter {kind} article {article} did not spawn");
            let mut first = State::default();
            let mut then = State::default();
            ssbm_projectile_impact(h, std::ptr::null_mut());
            assert_eq!(ssbm_projectile_state(h, &mut first), 1);
            for _ in 0..later {
                ssbm_projectile_step(h, 1. / 60., v(0., 0., 0.), 0);
            }
            assert_eq!(ssbm_projectile_state(h, &mut then), 1);
            ssbm_projectile_free(h);
            Some((first, then))
        }
    }

    #[test]
    fn fox_laser_keeps_its_damage_and_radius_after_impact() {
        let Some((first, then)) = after_impact(&libssbm_assets::fighter::FOX, 1, 0, 0, 0., 5) else { return };
        eprintln!("fox laser after impact: damage {} radius {} (+5 ticks: {} {})", first.damage, first.radius, then.damage, then.radius);
        assert!(first.damage > 0. && first.radius > 0., "{} {}", first.damage, first.radius);
        assert!(then.damage > 0. && then.radius > 0.);
    }

    #[test]
    fn samus_bomb_still_takes_its_damage_from_the_explosion() {
        let Some((first, then)) = after_impact(&libssbm_assets::fighter::SAMUS, 0, 0, 0, 0., 8) else { return };
        eprintln!("samus bomb after impact: damage {} radius {} (+8 ticks: {} {})", first.damage, first.radius, then.damage, then.radius);
        // The explosion script opens its hitbox a few frames in, with its own damage and radius.
        assert!(then.damage > 0. && then.radius > 0., "{} {}", then.damage, then.radius);
        // And they are the explosion article's own hitbox, not the bomb's defaults.
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let explosion = Article::load_for(&dir, 0, 1, &libssbm_assets::fighter::SAMUS).unwrap();
        let hit = explosion.commands.iter().find_map(|c| match &c.command {
            Command::Hitbox(h) => Some((h.damage, h.radius)),
            _ => None,
        }).expect("explosion hitbox");
        assert_eq!((then.damage, then.radius), hit);
    }

    #[test]
    fn yoshi_egg_burst_keeps_its_damage() {
        let Some((first, _)) = after_impact(&libssbm_assets::fighter::YOSHI, 6, 0, 0, 0., 0) else { return };
        eprintln!("yoshi egg after impact: damage {} radius {}", first.damage, first.radius);
        assert!(first.damage > 0. && first.radius > 0.);
    }

    #[test]
    fn link_arrow_speed_and_damage_climb_with_the_charge() {
        let Some((_, dir)) = assets() else { return };
        let read = |charge: f32| unsafe {
            let h = ssbm_projectile_new(dir.as_ptr(), 5, 0, 0, charge, v(0., 0., 1.));
            assert!(!h.is_null(), "arrow did not spawn");
            let mut s = State::default();
            ssbm_projectile_step(h, 1. / 60., v(0., 0., 0.), 0);
            assert_eq!(ssbm_projectile_state(h, &mut s), 1);
            ssbm_projectile_free(h);
            (s.velocity.z, s.damage)
        };
        let (slow, weak) = read(0.);
        let (fast, strong) = read(1.);
        assert!(fast > slow + 1., "speed {slow} -> {fast}");
        assert!(strong > weak + 4., "damage {weak} -> {strong}");
    }

    #[test]
    fn link_arrow_keeps_its_damage_when_it_bursts() {
        let Some((_, dir)) = assets() else { return };
        unsafe {
            let h = ssbm_projectile_new(dir.as_ptr(), 5, 0, 0, 1., v(0., 0., 1.));
            ssbm_projectile_impact(h, std::ptr::null_mut());
            let mut s = State::default();
            ssbm_projectile_state(h, &mut s);
            ssbm_projectile_free(h);
            assert!(s.damage > 0., "a burst arrow does {} damage", s.damage);
        }
    }

    #[test]
    fn link_boomerang_goes_out_turns_around_and_is_caught() {
        let Some((_, dir)) = assets() else { return };
        unsafe {
            let h = ssbm_projectile_new(dir.as_ptr(), 5, 1, 0, 0., v(0., 0., 1.));
            assert!(!h.is_null(), "boomerang did not spawn");
            let mut s = State::default();
            let (mut furthest, mut turned, mut caught) = (0f32, None, None);
            for tick in 0..900 {
                // The thrower stands where the boomerang started.
                ssbm_projectile_step(h, 1. / 60., v(0., 0., 0.), 1);
                ssbm_projectile_state(h, &mut s);
                furthest = furthest.max(s.position.z);
                if turned.is_none() && s.flags & 4 != 0 {
                    turned = Some((tick, s.position.z));
                }
                if s.flags & 2 != 0 {
                    caught = Some(tick);
                    break;
                }
            }
            ssbm_projectile_free(h);
            let (turn_tick, turn_at) = turned.expect("never turned around");
            assert!(turn_at > 20., "turned after only {turn_at} units");
            assert!(furthest >= turn_at, "never flew out");
            let caught = caught.expect("never caught");
            eprintln!("boomerang: turned at tick {turn_tick} ({turn_at} units out), furthest {furthest}, caught at tick {caught}");
            assert!(caught > turn_tick, "caught before it turned");
        }
    }
}