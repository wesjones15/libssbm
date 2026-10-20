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

/// Fox's `ext_attr` special tuning. Prototype fixture until the ftFx_DatAttrs
/// offsets are sourced from ftFox/ftfoxspecial{n,s,hi,lw}.c.
pub fn load_fox_specials(_dir: &Path) -> Result<libssbm::fox::Specials> {
    Ok(libssbm::fox::Specials::fixture())
}
