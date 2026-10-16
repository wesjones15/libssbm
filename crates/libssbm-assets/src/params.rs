//! ft/types.h ftData.x0 / ext_attr; ftSamus/types.h ftSs_DatAttrs.
use crate::{Result, dat::Dat};
use libssbm::Params;
use std::path::Path;

pub fn load(dir: &Path) -> Result<Params> {
    let fighter_bytes = std::fs::read(dir.join("PlSs.dat"))?;
    let fighter = Dat::parse(&fighter_bytes)?;
    let root = fighter.root("ftDataSamus")?;
    let co = fighter.required_ptr(root)?;
    let ss = fighter.required_ptr(root + 4)?;
    let common_bytes = std::fs::read(dir.join("PlCo.dat"))?;
    let common = Dat::parse(&common_bytes)?;
    let common_root = common.root("ftLoadCommonData")?;
    // ftDataCommon.common is its first pointer; not the root structure itself.
    let global = common.required_ptr(common_root)?;
    let p = Params {
        walk_accel: fighter.f32(co)?,
        walk_base: fighter.f32(co + 4)?,
        walk_speed: fighter.f32(co + 8)?,
        friction: fighter.f32(co + 0x18)?,
        dash_initial: fighter.f32(co + 0x1c)?,
        dash_accel: fighter.f32(co + 0x20)?,
        dash_base: fighter.f32(co + 0x24)?,
        run_speed: fighter.f32(co + 0x28)?,
        jump_squat: fighter.f32(co + 0x38)?.round() as u32,
        jump_speed: fighter.f32(co + 0x40)?,
        short_hop_speed: fighter.f32(co + 0x4c)?,
        air_jump_scale: fighter.f32(co + 0x50)?,
        jumps: fighter.u32(co + 0x58)?,
        gravity: fighter.f32(co + 0x5c)?,
        fall_speed: fighter.f32(co + 0x60)?,
        air_accel: fighter.f32(co + 0x64)?,
        air_base: fighter.f32(co + 0x68)?,
        air_speed: fighter.f32(co + 0x6c)?,
        air_friction: fighter.f32(co + 0x70)?,
        fast_fall: fighter.f32(co + 0x74)?,
        charge_steps: fighter.f32(ss + 0x18)?.round() as u32,
        charge_interval: fighter.u32(ss + 0x20)?.saturating_add(1),
        missile_smash_window: fighter.f32(ss + 0x28)?.round() as u32,
        screw_velocity: fighter.f32(ss + 0x44)?,
        bomb_velocity: fighter.f32(ss + 0x58)?,
        smash_threshold: common.f32(global + 0x3c)?,
        smash_window: common.u32(global + 0x40)?,
    };
    p.validate()?;
    Ok(p)
}
