//! ft/types.h ftData.x0 / ext_attr; ftSamus/types.h ftSs_DatAttrs.
use crate::{Result, dat::Dat};
use libssbm::Params;
use std::path::Path;
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

pub fn load(dir: &Path) -> Result<Params> {
    load_for(dir, &crate::fighter::SAMUS)
}
pub fn load_for(dir: &Path, profile: &crate::fighter::FighterAssets<'_>) -> Result<Params> {
    profile.required()?;
    let fighter_bytes = std::fs::read(dir.join(profile.data.file))?;
    let fighter = Dat::parse(&fighter_bytes)?;
    let root = fighter.root(profile.data.root)?;
    let co = fighter.required_ptr(root)?;
    let ss = fighter.required_ptr(root + 4)?;
    let sounds = fighter.required_ptr(root + 0x4c)?;
    let articles = fighter.required_ptr(root + 0x48)?;
    let charge = fighter.required_ptr(articles + 4)?;
    let charge_attrs = fighter.required_ptr(charge + 4)?;
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
        smash_threshold: common.f32(global + 0x3c)?,
        smash_window: common.u32(global + 0x40)?,
    };
    p.validate()?;
    Ok(p)
}
