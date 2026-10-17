//! Engine-independent motion command data, populated from the player's disc.
//! Format/behavior specification: lb/lbcommand.c and ft/ftaction.c.
use glam::Vec3;
use std::collections::BTreeMap;

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

#[derive(Clone, Debug)]
pub struct ActiveHitbox {
    pub hitbox: Hitbox,
    pub generation: u32,
}
#[derive(Clone, Debug)]
pub struct HitSample {
    pub hitbox: Hitbox,
    pub generation: u32,
    pub serial: u64,
    pub clip: &'static str,
    pub frame: f32,
    pub origin: Vec3,
    pub facing: Vec3,
    pub strength: f32,
}
#[derive(Clone, Debug, Default)]
pub struct ActionState {
    pub definitions: BTreeMap<String, ActionDef>,
    pub active: BTreeMap<u32, ActiveHitbox>,
    pub samples: Vec<HitSample>,
    pub variables: [u32; 4],
    pub serial: u64,
    pub interrupt: bool,
    pub jab_combo: bool,
    pub visibility: BTreeMap<i32, i32>,
    pub projectile_fired: bool,
    pub smash_max_frames: u32,
    pub smash_multiplier: f32,
    pub(crate) cursor: usize,
    pub(crate) script_clip: &'static str,
    pub(crate) script_frame: f32,
    pub(crate) generations: [u32; 8],
}
impl ActionState {
    pub fn reset(&mut self) {
        self.active.clear();
        self.variables = [0; 4];
        self.serial = self.serial.wrapping_add(1);
        self.interrupt = false;
        self.jab_combo = false;
        self.visibility.clear();
        self.projectile_fired = false;
        self.cursor = 0;
        self.script_clip = "";
        self.script_frame = -1.;
    }
}
