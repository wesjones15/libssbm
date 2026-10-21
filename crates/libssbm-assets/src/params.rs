//! ft/types.h ftData.x0 / ext_attr; ftSamus/types.h ftSs_DatAttrs.
use crate::{Result, dat::Dat};
use libssbm::Params;
use std::path::Path;
/// Colour-only portion of a common colour-animation cycle. The full-charge
/// program also animates material parameters; those remain a renderer concern.
pub fn color_cycle(dir: &Path, index: usize) -> Result<Vec<[f32; 4]>> {
    let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
    let d = Dat::parse(&bytes)?;
    let colors = d.required_ptr(d.root(crate::fighter::COMMON_DATA.root)? + 0x18)?;
    let mut pc = d.required_ptr(colors + index * 8)?;
    let mut out = Vec::new();
    let mut color = [0.; 4];
    let mut target = color;
    let mut fade = 0u32;
    let rgba = |v: u32| {
        [
            ((v >> 24) & 255) as f32 / 255.,
            ((v >> 16) & 255) as f32 / 255.,
            ((v >> 8) & 255) as f32 / 255.,
            (v & 255) as f32 / 255.,
        ]
    };
    for _ in 0..128 {
        let word = d.u32(pc)?;
        pc += 4;
        match word >> 26 {
            21 => pc += 16, // material parameter animation (lb_80013BB8)
            3 => (),        // repeat count; return one cycle at the matching end
            18 => {
                color = rgba(d.u32(pc)?);
                pc += 4;
                fade = 0;
            }
            19 => {
                target = rgba(d.u32(pc)?);
                pc += 4;
                fade = word & 0x03ffffff;
            }
            11 => {
                let count = word & 0x03ffffff;
                if count > 600 {
                    return Err("colour wait exceeds budget".into());
                }
                for _ in 0..count {
                    out.push(color);
                    if fade > 0 {
                        for c in 0..4 {
                            color[c] += (target[c] - color[c]) / fade as f32;
                        }
                        fade -= 1;
                    }
                }
            }
            12 => {
                color = [0.; 4];
                fade = 0;
            }
            0 | 4 | 7 | 10 => break,
            _ => return Err("unsupported colour cycle command".into()),
        }
    }
    if out.is_empty() {
        return Err("empty colour cycle".into());
    }
    Ok(out)
}
pub fn entry(
    dir: &Path,
    profile: &crate::fighter::FighterAssets<'_>,
) -> Result<libssbm::entry::Params> {
    profile.required()?;
    let b = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&b)?;
    let co = fighter.required_ptr(fighter.root(profile.data.root)?)?;
    let b = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
    let common = Dat::parse(&b)?;
    let attrs = common.required_ptr(common.root(crate::fighter::COMMON_DATA.root)?)?;
    // ftCo_800C6408 starts common colour animation 0x75. GALE01's
    // lb_80014014 / lb_80013BB8 program sets RGBA, waits, then finishes.
    let colors = common.required_ptr(common.root(crate::fighter::COMMON_DATA.root)? + 0x18)?;
    let program = common.required_ptr(colors + 0x75 * 8)?;
    if common.u32(program)? >> 26 != 18
        || common.u32(program + 8)? >> 26 != 11
        || common.u32(program + 12)? >> 26 != 10
    {
        return Err("unsupported common entry colour program".into());
    }
    let rgba = common.u32(program + 4)?;
    let p = libssbm::entry::Params {
        grow: common.u32(attrs + 0x6bc)?,
        release: common.u32(attrs + 0x6c0)?,
        flat: common.f32(attrs + 0x6c4)?,
        platform: fighter.f32(co + 0x110)?,
        color: [
            ((rgba >> 24) & 255) as f32 / 255.,
            ((rgba >> 16) & 255) as f32 / 255.,
            ((rgba >> 8) & 255) as f32 / 255.,
            (rgba & 255) as f32 / 255.,
        ],
        color_frames: common.u32(program + 8)? & 0x03ffffff,
    };
    if p.grow == 0
        || p.grow > 600
        || p.release == 0
        || p.release > 600
        || !(0. ..=1.).contains(&p.flat)
        || p.platform <= 0.
    {
        return Err("invalid entry parameters".into());
    }
    Ok(p)
}

pub fn rebirth(dir: &Path) -> Result<libssbm::rebirth::Params> {
    let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
    let d = Dat::parse(&bytes)?;
    let attrs = d.required_ptr(d.root(crate::fighter::COMMON_DATA.root)?)?;
    let params = libssbm::rebirth::Params {
        ko_frames: d.u32(attrs + 0x500)?,
        arrival_frames: d.u32(attrs + 0x5d0)?,
        wait_frames: d.u32(attrs + 0x5d4)?,
        invulnerable_frames: d.u32(attrs + 0x5d8)?,
        ko_scale: d.f32(attrs + 0x4f4)?,
        flash: color_cycle(dir, 9)?,
    };
    if [
        params.ko_frames,
        params.arrival_frames,
        params.wait_frames,
        params.invulnerable_frames,
    ]
    .iter()
    .any(|&n| n == 0 || n > 3600)
        || !params.ko_scale.is_finite()
        || params.ko_scale <= 0.
    {
        return Err("invalid common rebirth parameters".into());
    }
    Ok(params)
}
pub fn defense(dir: &Path) -> Result<libssbm::defense::Params> {
    defense_for(dir, &crate::fighter::SAMUS)
}
pub fn defense_for(
    dir: &Path,
    profile: &crate::fighter::FighterAssets<'_>,
) -> Result<libssbm::defense::Params> {
    profile.required()?;
    let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
    let d = Dat::parse(&bytes)?;
    let a = d.required_ptr(d.root(crate::fighter::COMMON_DATA.root)?)?;
    let fighter_bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&fighter_bytes)?;
    let attributes = fighter.required_ptr(fighter.root(profile.data.root)?)?;
    let duration = (d.f32(a + 0x2f8)? + d.f32(a + 0x2fc)?) / d.f32(a + 0x300)?;
    if !duration.is_finite() || duration <= 0. || duration > 3600. {
        return Err("invalid shield break duration".into());
    }
    let p = libssbm::defense::Params {
        maximum: d.f32(a + 0x260)?,
        drain: d.f32(a + 0x278)? * d.f32(a + 0x2f0)?,
        regen: d.f32(a + 0x27c)?,
        hit_mul: d.f32(a + 0x284)? * (1. - d.f32(a + 0x2e0)?),
        hit_add: d.f32(a + 0x288)?,
        restore: d.f32(a + 0x280)?,
        air_force: d.f32(a + 0x338)?,
        air_decay: d.f32(a + 0x33c)?,
        broken_frames: duration.ceil() as u32,
        shield_size: fighter.f32(attributes + 0x90)?,
        minimum_scale: d.f32(a + 0x264)?,
        hard_scale: d.f32(a + 0x2d4)?,
    };
    if ![
        p.maximum,
        p.drain,
        p.regen,
        p.hit_mul,
        p.hit_add,
        p.restore,
        p.air_force,
        p.air_decay,
        p.shield_size,
        p.minimum_scale,
        p.hard_scale,
    ]
    .iter()
    .all(|n| n.is_finite() && *n >= 0.)
        || p.maximum <= 0.
        || p.air_decay > 1.
    {
        return Err("invalid common defense tuning".into());
    }
    Ok(p)
}

pub fn load(dir: &Path) -> Result<Params> {
    load_common(dir, &crate::fighter::SAMUS)
}
pub fn load_common(dir: &Path, profile: &crate::fighter::FighterAssets<'_>) -> Result<Params> {
    profile.required()?;
    let fighter_bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&fighter_bytes)?;
    let root = fighter.root(profile.data.root)?;
    let co = fighter.required_ptr(root)?;
    let sounds = fighter.required_ptr(root + 0x4c)?;
    let common_bytes = std::fs::read(dir.join("PlCo.dat"))?;
    let common = Dat::parse(&common_bytes)?;
    let common_root = common.root("ftLoadCommonData")?;
    // ftDataCommon.common is its first pointer; not the root structure itself.
    let global = common.required_ptr(common_root)?;
    let p = Params {
        walk_accel: fighter.f32(co)?,
        walk_base: fighter.f32(co + 4)?,
        walk_speed: fighter.f32(co + 8)?,
        // ftwalkcommon::ftWalkCommon_800DFDDC and ftCo_Run_Anim.
        walk_animation_rates: [
            fighter.f32(co + 0xc)?,
            fighter.f32(co + 0x10)?,
            fighter.f32(co + 0x14)?,
        ],
        walk_animation_thresholds: [common.f32(global + 0x28)?, common.f32(global + 0x2c)?],
        run_animation_rate: fighter.f32(co + 0x2c)?,
        friction: fighter.f32(co + 0x18)?,
        dash_initial: fighter.f32(co + 0x1c)?,
        dash_accel: fighter.f32(co + 0x20)?,
        dash_base: fighter.f32(co + 0x24)?,
        run_speed: fighter.f32(co + 0x28)?,
        jump_squat: fighter.f32(co + 0x38)?.round() as u32,
        jump_speed: fighter.f32(co + 0x40)?,
        jump_horizontal: fighter.f32(co + 0x3c)?,
        jump_momentum: fighter.f32(co + 0x44)?,
        jump_horizontal_max: fighter.f32(co + 0x48)?,
        air_jump_horizontal: fighter.f32(co + 0x54)?,
        jump_sound: fighter.u32(sounds + 0x10)?,
        air_jump_sound: fighter.u32(sounds + 0x14)?,
        short_hop_speed: fighter.f32(co + 0x4c)?,
        air_jump_scale: fighter.f32(co + 0x50)?,
        jumps: fighter.u32(co + 0x58)?,
        gravity: fighter.f32(co + 0x5c)?,
        fall_speed: fighter.f32(co + 0x60)?,
        air_accel: fighter.f32(co + 0x64)?,
        air_base: fighter.f32(co + 0x68)?,
        air_speed: fighter.f32(co + 0x6c)?,
        air_horizontal_max: fighter.f32(co + 0x78)?,
        fall_pose_threshold: common.f32(global + 0x444)?,
        fall_blend_response: common.f32(global + 0x448)?,
        air_friction: fighter.f32(co + 0x70)?,
        fast_fall: fighter.f32(co + 0x74)?,
        smash_threshold: common.f32(global + 0x3c)?,
        smash_window: common.u32(global + 0x40)?,
    };
    p.validate()?;
    Ok(p)
}

/// Samus's `ext_attr` special tuning (ftSs_DatAttrs and charge-shot article
/// attributes). Other fighters supply their own behavior parameters.
pub fn load_samus_specials(dir: &Path) -> Result<libssbm::samus::Specials> {    let profile = &crate::fighter::SAMUS;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let ss = fighter.required_ptr(root + 4)?;
    let articles = fighter.required_ptr(root + 0x48)?;
    let charge = fighter.required_ptr(articles + 4)?;
    let charge_attrs = fighter.required_ptr(charge + 4)?;
    let p = libssbm::samus::Specials {
        charge_steps: fighter.f32(ss + 0x18)?.round() as u32,
        charge_visual_min: fighter.f32(charge_attrs + 0x18)?,
        charge_visual_max: fighter.f32(charge_attrs + 0x1c)?,
        charge_interval: fighter.u32(ss + 0x20)?.saturating_add(1),
        missile_smash_window: fighter.f32(ss + 0x28)?.round() as u32,
        missile_spawn_offset: fighter.f32(ss + 0x34)?,
        bomb_spawn_offset: [
            fighter.f32(ss + 0x74)?,
            fighter.f32(ss + 0x78)?,
            fighter.f32(ss + 0x7c)?,
        ],
        screw_velocity: fighter.f32(ss + 0x44)?,
        bomb_velocity: fighter.f32(ss + 0x58)?,
    };
    Ok(p)
}

/// Fox's `ext_attr` special tuning. ftFox/types.h::ftFox_DatAttrs; the pointer
/// is `ftDataFox + 4`, same layout as Samus's special attributes.
pub fn load_fox_specials(dir: &Path) -> Result<libssbm::fox::Specials> {
    let profile = &crate::fighter::FOX;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let fx = fighter.required_ptr(root + 4)?;
    Ok(libssbm::fox::Specials {
        blaster_angle: fighter.f32(fx + 0x10)?,
        blaster_speed: fighter.f32(fx + 0x14)?,
        illusion_velocity: fighter.f32(fx + 0x28)?,
        fire_fox_velocity: fighter.f32(fx + 0x74)?,
        reflector_frames: fighter.f32(fx + 0x98)?.round() as u32,
        illusion_ground_end: fighter.f32(fx + 0x34)?,
        illusion_friction: fighter.f32(fx + 0x38)?,
        illusion_air_end: fighter.f32(fx + 0x3c)?,
        fire_fox_duration: fighter.f32(fx + 0x68)?,
        fire_fox_duration_end: fighter.f32(fx + 0x70)?,
        fire_fox_reverse_accel: fighter.f32(fx + 0x78)?,
        reflector_gravity_delay: fighter.u32(fx + 0xa4)?,
    })
}

/// Pikachu's `ext_attr` special tuning (ftPikachuAttributes; the pointer is
/// `ftDataPikachu + 4`). The Thunder strike delay is derived from the cloud
/// height (xD0), the bolt's fall speed (xC0), the item's tip offset
/// (itPikachuthunderAttributes.x8) and the strike window of
/// ftPk_SpecialLw_8012765C (|y + |xBC| - tip| < xC8).
pub fn load_pikachu_specials(dir: &Path) -> Result<libssbm::pikachu::Specials> {
    let profile = &crate::fighter::PIKACHU;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let d = Dat::parse(&bytes)?;
    let root = d.root(profile.data.root)?;
    let pk = d.required_ptr(root + 4)?;
    let items = d.required_ptr(root + 0x48)?;
    let thunder = d.required_ptr(d.required_ptr(items)? + 4)?;
    let tip_offset = d.f32(thunder + 8)?;
    let height = d.f32(pk + 0xd0)?;
    let speed = d.f32(pk + 0xc0)?.abs().max(0.01);
    let window_top = d.f32(pk + 0xbc)?.abs() + d.f32(pk + 0xc8)?;
    let strike = ((height - tip_offset - window_top) / speed).ceil().max(1.) as u32;
    Ok(libssbm::pikachu::Specials {
        bash_max_charge: d.f32(pk + 0x24)?.round() as u32,
        bash_base_damage: d.f32(pk + 0x28)?,
        bash_damage_per_frame: d.f32(pk + 0x2c)?,
        bash_speed_base: d.f32(pk + 0x3c)?,
        bash_speed_per_frame: d.f32(pk + 0x40)?,
        bash_launch_y: d.f32(pk + 0x44)?,
        bash_gravity: d.f32(pk + 0x48)?,
        bash_terminal: d.f32(pk + 0x4c)?,
        bash_end_divisor: d.f32(pk + 0x50)?,
        bash_late_decel: d.f32(pk + 0x54)?,
        bash_late_gravity: d.f32(pk + 0x58)?,
        zip_frames: d.u32(pk + 0x60)?,
        zip_stick_min: d.f32(pk + 0x8c)?,
        zip_speed_slope: d.f32(pk + 0x90)?,
        zip_speed_base: d.f32(pk + 0x94)?,
        zip_second_decay: d.f32(pk + 0x98)?,
        zip_residual: d.f32(pk + 0xa4)?,
        zip_min_angle: d.u32(pk + 0xa8)? as f32,
        thunder_hop: d.f32(pk + 0xb4)?,
        thunder_gravity: d.f32(pk + 0xb8)?,
        thunder_height: height,
        thunder_cloud_offset: d.f32(pk + 0xcc)?,
        thunder_speed: speed,
        thunder_strike_frames: strike,
    })
}

/// Luigi's `ext_attr` special tuning (ftLuigiAttributes; the pointer is
/// `ftDataLuigi + 4`, same layout as the other special attributes). Fireball
/// speed lives in the item's own special attributes (itLuigiFireballAttributes).
pub fn load_luigi_specials(dir: &Path) -> Result<libssbm::luigi::Specials> {
    let profile = &crate::fighter::LUIGI;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let lg = fighter.required_ptr(root + 4)?;
    Ok(libssbm::luigi::Specials {
        missile_velocity: fighter.f32(lg + 0x24)?,
        super_jump_velocity: fighter.f32(lg + 0x6c)?,
        cyclone_momentum: fighter.f32(lg + 0x74)?,
    })
}

/// Captain Falcon's `ext_attr` special tuning (ftCaptain_DatAttrs; the pointer
/// is `ftDataCaptain + 4`, same layout as the other special attributes).
pub fn load_captain_specials(dir: &Path) -> Result<libssbm::captain::Specials> {
    let profile = &crate::fighter::CAPTAIN;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let ca = fighter.required_ptr(root + 4)?;
    Ok(libssbm::captain::Specials {
        punch_momentum: fighter.f32(ca + 0xc)?,
        raptor_velocity: fighter.f32(ca + 0x14)?,
        dive_velocity: fighter.f32(ca + 0x44)?,
    })
}

/// Link's `ext_attr` special tuning (ftLk_DatAttrs; the pointer is
/// `ftDataLink + 4`, same layout as the other special attributes).
pub fn load_link_specials(dir: &Path) -> Result<libssbm::link::Specials> {
    let profile = &crate::fighter::LINK;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let lk = fighter.required_ptr(root + 4)?;
    Ok(libssbm::link::Specials {
        bow_charge_steps: fighter.f32(lk)?.round() as u32,
        spin_rise_velocity: fighter.f32(lk + 0x40)?,
        // ftlinkspecials.c::calcAnglePos reads the stick deadzone (x14), the
        // angle scale (x18 * x1C) and the throw speeds (x20 / x24).
        boomerang_stick_threshold: fighter.f32(lk + 0x14)?,
        boomerang_max_angle: fighter.f32(lk + 0x18)? * fighter.f32(lk + 0x1c)?,
        boomerang_speed_a: fighter.f32(lk + 0x20)?,
        boomerang_speed_b: fighter.f32(lk + 0x24)?,
    })
}

/// Yoshi's `ext_attr` special tuning (ftYoshiAttributes / ftYs_DatAttrs; the
/// pointer is `ftDataYoshi + 4`). Egg Roll reads the ground/air speed at x54/x58
/// and gravity at x6C; Egg Throw reads base speed at xFC; Yoshi Bomb reads its
/// downward terminal speed at x114.
pub fn load_yoshi_specials(dir: &Path) -> Result<libssbm::yoshi::Specials> {
    let profile = &crate::fighter::YOSHI;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let ys = fighter.required_ptr(root + 4)?;
    Ok(libssbm::yoshi::Specials {
        egg_roll_speed: fighter.f32(ys + 0x54)?,
        egg_throw_speed: fighter.f32(ys + 0xfc)?,
        bomb_fall_speed: fighter.f32(ys + 0x114)?,
    })
}

/// Donkey Kong's `ext_attr` special tuning (ftDonkeyAttributes; the pointer is
/// `ftDataDonkey + 4`). Giant Punch momentum is x34; Headbutt uses x3C; Spinning
/// Kong rise is x4C with horizontal speed at x54 (grounded).
pub fn load_donkey_specials(dir: &Path) -> Result<libssbm::donkey::Specials> {
    let profile = &crate::fighter::DONKEY;
    profile.required()?;
    let bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&bytes)?;
    let root = fighter.root(profile.data.root)?;
    let dk = fighter.required_ptr(root + 4)?;
    Ok(libssbm::donkey::Specials {
        punch_velocity: fighter.f32(dk + 0x34)?,
        headbutt_velocity: fighter.f32(dk + 0x3c)?,
        spin_rise_velocity: fighter.f32(dk + 0x4c)?,
        spin_horizontal: fighter.f32(dk + 0x54)?,
    })
}
