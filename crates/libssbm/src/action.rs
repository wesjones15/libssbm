//! Engine-independent motion command data, populated from the player's disc.
//! Format/behavior specification: lb/lbcommand.c and ft/ftaction.c.
use glam::Vec3;

#[derive(Clone, Debug, Default)]
pub struct Hitbox {
    pub id: u32,
    pub group: u32,
    pub bone: usize,
    pub common_bone: bool,
    pub damage: f32,
    pub radius: f32,
    pub offset: Vec3,
    pub angle: u32,
    pub growth: u32,
    pub fixed_knockback: u32,
    pub base_knockback: u32,
    pub element: u32,
    pub ground: bool,
    pub air: bool,
}
#[derive(Clone, Debug)]
pub enum Command {
    Hitbox(Hitbox),
    ClearHitbox(u32),
    ClearHitboxes,
    HitboxDamage(u32, f32),
    HitboxRadius(u32, f32),
    Variable(u32, u32),
    Interrupt,
    JabCombo(bool),
    ThrowFlag(u32),
    SmashCharge {
        max_frames: u32,
        damage_per_frame: f32,
    },
    Visibility {
        group: i32,
        choice: i32,
    },
    Raw {
        opcode: u32,
        words: Vec<u32>,
    },
}
#[derive(Clone, Debug)]
pub struct TimedCommand {
    pub frame: f32,
    pub command: Command,
}
#[derive(Clone, Debug)]
pub struct ActionDef {
    pub frames: f32,
    pub looping: bool,
    pub commands: Vec<TimedCommand>,
}
