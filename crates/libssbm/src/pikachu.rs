//! Pikachu's special moves: Thunder Shock, Skull Bash, Quick Attack and Thunder.
//! Behavioral spec: ftPikachu/ftpikachuspecial{n,s,hi,lw}.c with the item
//! articles itpikachuthunder.c / itpikachutjolt{ground,air}.c. Tuning comes from
//! ftPikachuAttributes on the player's disc; the multi-state motion machines
//! (Start/Hold/Release/End, Start/Zip/End/Zip, Start/Loop/Strike/End) are
//! reimplemented as stages of one staged special (`Sim::stage`).
//!
//! RoR2 adaptations, all marked where they occur: stick angles become the
//! camera-relative stick plus camera pitch; Quick Attack steers to the host's
//! lock-on target and still arrives within its two source zips; Skull Bash and
//! Quick Attack scale their travel for open worlds.
use crate::{
    action::{GfxCue, SoundCue},
    behavior::Behavior,
    sim::{
        EventKind, Host, Input, Move, Phase, Sim, DOWN_B, NEUTRAL_B, SIDE_B, UP_B,
    },
};
use glam::Vec3;

/// Pikachu's special tuning, separate from the shared `Params`. Field comments
/// give the `ftPikachuAttributes` offset (ft/kinds/ftPikachu/types.h).
#[derive(Clone, Debug)]
pub struct Specials {
    pub bash_max_charge: u32,        // x24: charge frames before auto-release
    pub bash_base_damage: f32,       // x28
    pub bash_damage_per_frame: f32,  // x2C
    pub bash_speed_base: f32,        // x3C
    pub bash_speed_per_frame: f32,   // x40
    pub bash_launch_y: f32,          // x44
    pub bash_gravity: f32,           // x48 (before the late phase)
    pub bash_terminal: f32,          // x4C
    pub bash_end_divisor: f32,       // x50: speed divisor entering SpecialSEnd
    pub bash_late_decel: f32,        // x54
    pub bash_late_gravity: f32,      // x58
    pub zip_frames: u32,             // x60
    pub zip_stick_min: f32,          // x8C
    pub zip_speed_slope: f32,        // x90
    pub zip_speed_base: f32,         // x94
    pub zip_second_decay: f32,       // x98
    pub zip_residual: f32,           // xA4: velocity kept when a zip ends
    pub zip_min_angle: f32,          // xA8: degrees between the two zips
    pub thunder_hop: f32,            // xB4: vertical hop when struck in the air
    pub thunder_gravity: f32,        // xB8
    pub thunder_height: f32,         // xD0: cloud height above Pikachu
    pub thunder_cloud_offset: f32,   // xCC: cloud poof offset from that height
    pub thunder_speed: f32,          // |xC0|: bolt fall speed per tick
    /// Ticks from the cloud spawn until the bolt tip reaches Pikachu, derived
    /// from xD0 (cloud height), xC0 (bolt speed) and the article's tip offset.
    pub thunder_strike_frames: u32,
}
impl Specials {
    /// Synthetic fixture only; runtime hosts read the disc.
    pub fn fixture() -> Self {
        Self {
            bash_max_charge: 90,
            bash_base_damage: 4.,
            bash_damage_per_frame: 0.2777778,
            bash_speed_base: 1.4,
            bash_speed_per_frame: 0.016666668,
            bash_launch_y: 0.8,
            bash_gravity: 0.01,
            bash_terminal: 1.,
            bash_end_divisor: 1.5,
            bash_late_decel: 0.05,
            bash_late_gravity: 0.1,
            zip_frames: 5,
            zip_stick_min: 0.5,
            zip_speed_slope: 4.,
            zip_speed_base: 5.9,
            zip_second_decay: 0.9,
            zip_residual: 0.1,
            zip_min_angle: 38.,
            thunder_hop: 0.66,
            thunder_gravity: 0.016,
            thunder_height: 150.,
            thunder_cloud_offset: -18.,
            thunder_speed: 5.,
            thunder_strike_frames: 19,
        }
    }
}

// Stages. Skull Bash.
const BASH_START: u32 = 0;
const BASH_HOLD: u32 = 1;
const BASH_GO: u32 = 2;
const BASH_END: u32 = 3;
// Quick Attack.
const QUICK_START: u32 = 0;
const QUICK_ZIP: u32 = 1;
const QUICK_END: u32 = 2;
// Thunder.
const THUNDER_START: u32 = 0;
const THUNDER_WAIT: u32 = 1;
const THUNDER_STRUCK: u32 = 2;
const THUNDER_END: u32 = 3;
// Borrowed Yoshi Egg Lay, on the LT slot (SpecialLw) while the loadout selects it.
// The two stages reuse Pikachu's tense Thunder start animation.
const EGG_TONGUE: u32 = 0;
const EGG_SWALLOW: u32 = 1;

/// Host flag: the LT slot's loadout holds Yoshi's Egg Lay instead of Thunder.
/// `Pikachu::select_special`, the stage machine and the presentation read it.
pub const HOST_EGG_LAY: u32 = 1;

/// Host adaptation: open-world travel multipliers over the 60 Hz source speeds.
const ZIP_HOST_SCALE: f32 = 1.6;
const BASH_HOST_SCALE: f32 = 1.5;
/// Longest zip step per tick (Melee units); a farther lock-on stretches the zip
/// over more ticks instead, so a fast step never tunnels the capsule sweep.
const MAX_ZIP_STEP: f32 = 15.;
/// Pikachu stops this far short of a locked enemy so he does not end inside it.
const ZIP_STANDOFF: f32 = 12.;
/// Ticks between a zip ending and the decision to zip again: the source reads
/// its second-zip input on the End clip's frame 9, after a 5-tick zip.
const ZIP_DECISION_TICKS: u32 = 4;
/// Skull Bash motion frame at which the launch script clears its hitbox and the
/// late gravity/deceleration (cmd_vars[0] in ftPk_SpecialAirS1_Phys) begins.
const BASH_LATE_FRAME: f32 = 30.;
/// Thunder Shock sound (ftpikachuspecialn.c: ft_PlaySFX 240076 for Pikachu).
const SHOCK_SOUND: u32 = 240076;
/// Host adaptation: a Thunder Jolt aimed above this pitch (sine of the angle, about 11
/// degrees) from the ground plays the airborne clip. The grounded `SpecialN` clip is the
/// source's down-diagonal throw (ftPk_SpecialN_Anim); `SpecialAirN` (ftPk_SpecialAirN_Anim)
/// has the level throw, which fits a crosshair-aimed shot. Latched on entry in
/// `Sim::stage_value` so the clip cannot change while the aim moves.
const JOLT_UP_AIM: f32 = 0.2;

pub struct Pikachu {
    specials: Specials,
}
impl Pikachu {
    pub fn new(specials: Specials) -> Self {
        Self { specials }
    }
    /// Synthetic Pikachu behavior for headless tests.
    pub fn fixture() -> Self {
        Self::new(Specials::fixture())
    }
    pub fn specials(&self) -> &Specials {
        &self.specials
    }

    fn hold_gravity(sim: &mut Sim, gravity: f32) {
        sim.pad_gravity = Some(Vec3::new(0., -gravity, 0.));
        sim.owns_gravity = true;
    }
    /// The LT slot's loadout holds the borrowed Egg Lay instead of Thunder.
    fn egg_lay(sim: &Sim) -> bool {
        sim.host_flags & HOST_EGG_LAY != 0
    }
    /// Egg Lay's two stages reuse the tense Thunder start clip; the host drives the
    /// tongue and the egg. Grounded, the source Egg Lay plants the feet.
    fn tick_egg_lay(sim: &mut Sim) {
        sim.stage_frame += 1;
        if sim.grounded {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
        }
    }
    fn release_gravity(sim: &mut Sim) {
        if sim.owns_gravity {
            sim.pad_gravity = None;
            sim.owns_gravity = false;
        }
    }
    fn cue(sim: &Sim, id: u32, bone: usize, destroy_on_change: bool) -> GfxCue {
        GfxCue {
            id,
            bone,
            common: true, // FtPart index into the common bone table
            cycle: 0,
            destroy_on_change,
            offset: Vec3::ZERO,
            param: 0.,
            clip: sim.motion.clip,
            frame: sim.motion.frame,
            origin: sim.position,
            facing: sim.facing,
        }
    }

    // --- Skull Bash (ftpikachuspecials.c) -------------------------------------
    fn enter_hold(&self, sim: &mut Sim) {
        sim.stage = BASH_HOLD;
        sim.stage_frame = 0;
        sim.shot_charge = 0;
        // ftPk_SpecialN_SpawnEffect0 (efSync 1214) on the hip while charging.
        let flame = Self::cue(sim, 1214, 4, true);
        sim.action.effects.push(flame);
    }
    fn begin_bash(&self, sim: &mut Sim) {
        sim.stage = BASH_GO;
        sim.stage_frame = 0;
        sim.stage_value = 0.;
        sim.action.variables[0] = 0;
        // ftPk_SpecialS0_Anim: damage = frames * x2C + x28 replaces the capsule's.
        let damage = self.specials.bash_base_damage
            + self.specials.bash_damage_per_frame * sim.shot_charge as f32;
        sim.move_strength = damage / self.specials.bash_base_damage.max(0.01);
        // ftPk_SpecialN_SpawnEffect1 (efSync 1215): the release burst.
        let burst = Self::cue(sim, 1215, 4, false);
        sim.action.effects.push(burst);
    }
    /// ftPk_SpecialS_ChangeMotion_Unk10: launch speed grows with the charge.
    fn launch_bash(&self, sim: &mut Sim) {
        let s = &self.specials;
        let frames = sim.shot_charge as f32;
        let speed = (s.bash_speed_per_frame * frames + s.bash_speed_base)
            * BASH_HOST_SCALE
            * sim.options.speed_scale;
        let climb = 0.5 * s.bash_launch_y
            + s.bash_launch_y * 0.5 * frames / s.bash_max_charge.max(1) as f32;
        let heading = sim.facing.with_y(0.).normalize_or_zero();
        sim.velocity = heading * speed + Vec3::Y * climb * sim.options.jump_scale;
        sim.grounded = false;
        sim.stage_value = speed;
        sim.stage_frame = 0;
        Self::hold_gravity(sim, s.bash_gravity);
    }
    fn end_bash(&self, sim: &mut Sim) {
        if sim.stage == BASH_END {
            return;
        }
        let divisor = self.specials.bash_end_divisor.max(1.);
        sim.velocity.x /= divisor;
        sim.velocity.z /= divisor;
        sim.stage = BASH_END;
        sim.stage_frame = 0;
        sim.shot_charge = 0;
        sim.stage_dir = if sim.grounded { Vec3::X } else { Vec3::ZERO };
        Self::release_gravity(sim);
    }
    fn tick_bash(&self, sim: &mut Sim, input: Input) {
        match sim.stage {
            BASH_HOLD => {
                sim.shot_charge = (sim.shot_charge + 1).min(self.specials.bash_max_charge);
                if input.buttons & SIDE_B == 0 || sim.shot_charge >= self.specials.bash_max_charge
                {
                    self.begin_bash(sim);
                }
            }
            BASH_GO => {
                if sim.stage_value <= 0. {
                    // The clip's frame-4 script (cmd_vars[0]) starts the lunge.
                    if sim.action.variables[0] == 1 {
                        self.launch_bash(sim);
                    }
                    return;
                }
                sim.stage_frame += 1;
                let late = sim.motion.frame >= BASH_LATE_FRAME;
                let s = &self.specials;
                Self::hold_gravity(sim, if late { s.bash_late_gravity } else { s.bash_gravity });
                if sim.velocity.y < -s.bash_terminal {
                    sim.velocity.y = -s.bash_terminal;
                }
                let scale = sim.options.speed_scale;
                if late {
                    // ftCommon_CalcSelfAccel_Deaccel(x54)
                    let horizontal = sim.velocity.with_y(0.);
                    let step = s.bash_late_decel * scale;
                    let slowed = horizontal.length() - step;
                    let next = if slowed > 0. { horizontal.normalize() * slowed } else { Vec3::ZERO };
                    sim.velocity.x = next.x;
                    sim.velocity.z = next.z;
                }
                let landed = sim.stage_frame > 2 && sim.grounded;
                let wall = sim.stage_frame > 2
                    && sim.velocity.with_y(0.).length() < sim.stage_value * 0.35
                    && !late;
                if landed || wall {
                    self.end_bash(sim);
                }
            }
            _ => {}
        }
    }

    // --- Quick Attack (ftpikachuspecialhi.c) ----------------------------------
    /// Direction for a zip without a lock-on: the camera-relative stick sets the
    /// heading and the camera pitch the elevation (the source reads a 2D stick
    /// angle); with no stick it follows the camera aim.
    fn stick_direction(&self, sim: &Sim, input: Input) -> (Vec3, f32) {
        let stick = input.movement.with_y(0.).clamp_length_max(1.);
        let aim = if input.aim.length_squared() > 0.001 {
            input.aim.normalize()
        } else {
            sim.facing
        };
        let magnitude = stick.length();
        if magnitude >= self.specials.zip_stick_min {
            let pitch = aim.y.clamp(-1., 1.).asin();
            let dir = stick.normalize() * pitch.cos() + Vec3::Y * pitch.sin();
            (dir.normalize_or_zero(), magnitude.min(0.999))
        } else {
            (aim, 0.999)
        }
    }
    fn zip_capacity(&self, magnitude: f32, second: bool) -> f32 {
        let s = &self.specials;
        let speed = (s.zip_speed_slope * magnitude + s.zip_speed_base)
            * if second { s.zip_second_decay } else { 1. };
        speed * s.zip_frames as f32 * ZIP_HOST_SCALE * 1.0
    }
    /// Starts a zip (the first, or the second when `sim.stage_value == 1`).
    /// Returns false when a locked enemy is already within reach.
    fn begin_zip(&self, sim: &mut Sim, input: Input) -> bool {
        let second = sim.stage_value >= 1.;
        let zips_left = if second { 1. } else { 2. };
        let (mut dir, magnitude) = self.stick_direction(sim, input);
        let mut capacity = self.zip_capacity(magnitude, second) * sim.options.speed_scale;
        let mut travel = capacity;
        if let Some(target) = sim.lock_target {
            // Steer to the locked enemy and arrive within the source's two zips:
            // each zip covers its usual reach or an even share of what remains,
            // whichever is longer, and the last one covers all of it.
            let to = target - sim.position;
            let distance = to.length();
            let want = (distance - ZIP_STANDOFF).max(0.);
            if want <= 0.5 {
                return false;
            }
            dir = to / distance.max(1e-4);
            travel = if zips_left <= 1. { want } else { capacity.max(want / zips_left).min(want) };
            capacity = travel;
        }
        if sim.grounded && dir.y < 0. {
            dir.y = 0.;
            dir = dir.normalize_or_zero();
        }
        if dir == Vec3::ZERO {
            dir = Vec3::Y;
        }
        let frames = ((capacity.max(travel) / MAX_ZIP_STEP).ceil() as u32).max(self.specials.zip_frames);
        let step = travel / frames as f32;
        let heading = dir.with_y(0.);
        if heading.length_squared() > 0.01 {
            sim.facing = heading.normalize();
        }
        sim.stage = QUICK_ZIP;
        sim.stage_frame = 0;
        sim.stage_len = frames;
        sim.stage_dir = dir;
        sim.stage_vec = dir * step;
        // The source spends the double jumps on the first zip.
        sim.jumps_used = sim.params.jumps + sim.options.extra_jumps;
        // Only a rising zip leaves the floor; a level one slides along it, and
        // clearing `grounded` there would read as a fresh landing and end the move.
        if dir.y > 0.01 {
            sim.grounded = false;
        }
        sim.velocity = sim.stage_vec;
        Self::hold_gravity(sim, 0.);
        if second {
            // The End clip restarts, replaying its spark: Fighter_ChangeMotionState
            // back to SpecialHiStart1 at frame 12.
            sim.motion.frame = 0.;
        }
        true
    }
    fn end_zip(&self, sim: &mut Sim) {
        sim.stage_value += 1.;
        sim.stage = QUICK_END;
        sim.stage_frame = 0;
        // ftPk_SpecialHi_MotionChangeUpdateVel: a tenth of the speed carries on.
        sim.velocity = sim.stage_vec * self.specials.zip_residual;
    }
    fn tick_quick(&self, sim: &mut Sim, input: Input, pressed: u32) {
        match sim.stage {
            QUICK_START => {
                sim.velocity = Vec3::ZERO;
            }
            QUICK_ZIP => {
                sim.stage_frame += 1;
                sim.velocity = sim.stage_vec;
                let arrived = sim
                    .lock_target
                    .is_some_and(|t| (t - sim.position).length() <= ZIP_STANDOFF);
                if sim.stage_frame >= sim.stage_len || arrived {
                    self.end_zip(sim);
                }
            }
            QUICK_END => {
                sim.stage_frame += 1;
                // Air hover decay: self_vel.y -= self_vel.y / 9 (SpecialAirHiEnd_Phys).
                sim.velocity.y -= sim.velocity.y / 9.;
                sim.velocity.x *= 0.9;
                sim.velocity.z *= 0.9;
                if sim.stage_frame == ZIP_DECISION_TICKS && sim.stage_value < 2. {
                    if self.wants_second_zip(sim, input, pressed) && self.begin_zip(sim, input) {
                        return;
                    }
                    Self::release_gravity(sim);
                } else if sim.stage_frame > ZIP_DECISION_TICKS {
                    Self::release_gravity(sim);
                }
            }
            _ => {}
        }
    }
    /// ftPk_SpecialHi_80127064: a second zip needs a new direction at least
    /// `zip_min_angle` from the first. A lock-on short of its enemy always
    /// continues; re-pressing the button also counts (host adaptation, since a
    /// camera-relative angle is awkward to flick).
    fn wants_second_zip(&self, sim: &Sim, input: Input, pressed: u32) -> bool {
        if let Some(target) = sim.lock_target {
            return (target - sim.position).length() > ZIP_STANDOFF + 0.5;
        }
        if pressed & UP_B != 0 {
            return true;
        }
        let (dir, _) = self.stick_direction(sim, input);
        let magnitude = input.movement.with_y(0.).length();
        magnitude >= self.specials.zip_stick_min
            && sim.stage_dir.dot(dir).clamp(-1., 1.).acos()
                > self.specials.zip_min_angle.to_radians()
    }

    // --- Thunder (ftpikachuspeciallw.c, itpikachuthunder.c) -------------------
    fn tick_thunder(&self, sim: &mut Sim, host: &mut dyn Host) {
        match sim.stage {
            THUNDER_START => sim.velocity = Vec3::ZERO,
            THUNDER_WAIT => {
                sim.velocity = Vec3::ZERO;
                sim.stage_frame += 1;
                if sim.stage_frame >= self.specials.thunder_strike_frames {
                    // The bolt reaches Pikachu (ftPk_SpecialLw_8012765C): the loop
                    // restarts unmuted with its 17% electric hitbox.
                    sim.stage = THUNDER_STRUCK;
                    sim.stage_frame = 0;
                    sim.motion.frame = 0.;
                    // efAsync 1216 on TopN: the strike crackling over Pikachu.
                    let crackle = Self::cue(sim, 1216, 0, true);
                    sim.action.effects.push(crackle);
                    if !sim.grounded {
                        sim.velocity.y = self.specials.thunder_hop;
                    }
                    Self::hold_gravity(sim, self.specials.thunder_gravity);
                    sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 2.);
                }
            }
            THUNDER_STRUCK => {
                sim.stage_frame += 1;
                if sim.stage_frame >= 10 {
                    sim.stage = THUNDER_END;
                    sim.stage_frame = 0;
                    Self::release_gravity(sim);
                }
            }
            _ => {}
        }
    }
}

impl Behavior for Pikachu {
    fn slot(&self, m: Move) -> u32 {
        // Thunder is the down special (LT); Skull Bash is the side special (RB).
        match m {
            Move::SpecialN => 0,
            Move::SpecialLw => 1,
            Move::SpecialHi => 3,
            Move::SpecialS | Move::SpecialS2 => 2,
            _ => 4,
        }
    }
    fn mobile_moves(&self) -> u64 {
        1u64 << Move::SpecialN as u32
    }
    fn upper_body_root(&self) -> Option<usize> {
        None
    }
    fn is_up_special(&self, m: Move) -> bool {
        m == Move::SpecialHi
    }

    fn select_special(&self, _sim: &Sim, input: Input, _movement: Vec3) -> Option<Move> {
        Some(if input.buttons & UP_B != 0 {
            Move::SpecialHi
        } else if input.buttons & DOWN_B != 0 {
            Move::SpecialLw
        } else if input.buttons & SIDE_B != 0 {
            Move::SpecialS
        } else {
            let _ = NEUTRAL_B;
            Move::SpecialN
        })
    }
    fn enter_special(&self, sim: &mut Sim, _host: &mut dyn Host, m: Move, _input: Input) {
        match m {
            Move::SpecialHi => {
                // Start is a hover: ftPk_SpecialAirHiStart0_Phys.
                sim.velocity = Vec3::ZERO;
                Self::hold_gravity(sim, 0.);
            }
            Move::SpecialLw => {
                // Thunder Start is a hover: ftPk_SpecialLwStart_Phys.
                sim.velocity = Vec3::ZERO;
                Self::hold_gravity(sim, 0.);
            }
            Move::SpecialN => {
                if Self::egg_lay(sim) {
                    // Borrowed Yoshi Egg Lay on the RT slot: the tense Thunder start is
                    // the animation, and the host drives the tongue and egg.
                    sim.stage = EGG_TONGUE;
                    sim.stage_frame = 0;
                    sim.stage_len = 0;
                    sim.stage_value = 0.;
                    if sim.grounded {
                        sim.velocity.x = 0.;
                        sim.velocity.z = 0.;
                    }
                } else {
                    sim.stage_value = if sim.move_aim.y > JOLT_UP_AIM { 1. } else { 0. };
                }
            }
            Move::SpecialS => {
                // ftPk_SpecialS_Enter: gr_vel /= x30 (start friction), no air lift.
                sim.velocity.x /= 1.25;
                sim.velocity.z /= 1.25;
                if sim.velocity.y > 0. {
                    sim.velocity.y = 0.;
                }
                sim.shot_charge = 0;
            }
            _ => {}
        }
    }
    fn aimable(&self, m: Move) -> bool {
        matches!(m, Move::SpecialN | Move::SpecialS)
    }
    fn aim_frozen(&self, sim: &Sim) -> bool {
        sim.current_move == Some(Move::SpecialS) && sim.stage >= BASH_GO
    }
    fn is_charge_move(&self, _m: Move) -> bool {
        false
    }
    fn charge_voice_band(&self, _sim: &Sim) -> u32 {
        0
    }
    /// Skull Bash's charge frames, for the HUD fraction.
    fn charge_steps(&self) -> u32 {
        self.specials.bash_max_charge.max(1)
    }
    fn charge_visual(&self) -> (f32, f32) {
        (1., 1.)
    }
    fn store_special(&self, _sim: &mut Sim, _host: &mut dyn Host, _pressed: u32) -> bool {
        false
    }
    fn fire_replays(&self, _m: Move) -> bool {
        false
    }
    fn resolve_startup(&self, _sim: &mut Sim, _input: Input) -> bool {
        false
    }
    fn fire_strength(&self, sim: &mut Sim, m: Move) -> f32 {
        if m == Move::SpecialN {
            if Self::egg_lay(sim) {
                // 1: the tongue is out; 2: the egg is laid (the borrowed Yoshi contract).
                return if sim.stage == EGG_SWALLOW { 2. } else { 1. };
            }
            sim.action.sounds.push(SoundCue {
                sound: SHOCK_SOUND,
                volume: 127,
                pan: 64,
                channel: 0,
                stop: false,
            });
        }
        sim.move_strength
    }
    fn tick_charge(&self, _sim: &mut Sim, _host: &mut dyn Host, _input: Input, _pressed: u32) {}
    fn tick_charge_cancel(&self, sim: &mut Sim) -> bool {
        if sim.motion.finished() {
            sim.set_phase(Phase::Free);
            sim.current_move = None;
            true
        } else {
            false
        }
    }

    fn tick_special(&self, sim: &mut Sim, host: &mut dyn Host, input: Input, pressed: u32) {
        let mine = matches!(
            sim.current_move,
            Some(Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
        ) || (Self::egg_lay(sim) && sim.current_move == Some(Move::SpecialN));
        if !mine || sim.phase == Phase::Free {
            // Ended or interrupted: never leave a held gravity override behind.
            Self::release_gravity(sim);
            return;
        }
        match sim.current_move {
            Some(Move::SpecialS) => self.tick_bash(sim, input),
            Some(Move::SpecialHi) => self.tick_quick(sim, input, pressed),
            Some(Move::SpecialN) if Self::egg_lay(sim) => Self::tick_egg_lay(sim),
            Some(Move::SpecialLw) => self.tick_thunder(sim, host),
            _ => {}
        }
    }
    fn next_stage(&self, sim: &mut Sim, host: &mut dyn Host, m: Move, input: Input) -> bool {
        if Self::egg_lay(sim) && m == Move::SpecialN {
            // Tongue -> swallow. Both stages replay the tense Thunder start: the clip is
            // 18 frames, so the second stage restarts it rather than holding the last pose.
            if sim.stage == EGG_TONGUE {
                sim.stage = EGG_SWALLOW;
                sim.stage_frame = 0;
                sim.motion.frame = 0.;
                sim.action.projectile_fired = false;
                return true;
            }
            return false;
        }
        match (m, sim.stage) {
            (Move::SpecialS, BASH_START) => {
                if input.buttons & SIDE_B != 0 {
                    self.enter_hold(sim);
                } else {
                    self.begin_bash(sim);
                }
                true
            }
            (Move::SpecialS, BASH_GO) => {
                // The lunge clip ran out in the air (or before it launched).
                if sim.stage_value <= 0. {
                    self.launch_bash(sim);
                }
                self.end_bash(sim);
                true
            }
            (Move::SpecialHi, QUICK_START) => {
                if !self.begin_zip(sim, input) {
                    // Already in reach of the locked enemy: skip straight to the end.
                    sim.stage_value = 2.;
                    sim.stage = QUICK_END;
                    sim.stage_frame = 0;
                    sim.stage_dir = sim.facing;
                }
                true
            }
            (Move::SpecialLw, THUNDER_START) => {
                sim.stage = THUNDER_WAIT;
                sim.stage_frame = 0;
                // efSync 1219: the cloud poof, xD0 + xCC above Pikachu.
                let mut cloud = Self::cue(sim, 1219, 0, false);
                cloud.common = false;
                cloud.offset =
                    Vec3::Y * (self.specials.thunder_height + self.specials.thunder_cloud_offset);
                sim.action.effects.push(cloud);
                // The bolt spawns here (ftPk_SpecialLw_SpawnEffect, it_802B1DF8). The
                // event's direction carries (cloud height, ticks until the strike,
                // fall speed per tick) in Melee units so the host can draw the
                // bolt in its own world, reaching Pikachu on the strike tick.
                let aim = sim.move_aim;
                sim.move_aim = Vec3::new(
                    self.specials.thunder_height,
                    self.specials.thunder_strike_frames as f32,
                    self.specials.thunder_speed,
                );
                sim.emit(host, EventKind::Hit, Move::SpecialLw as u32, 1.);
                sim.move_aim = aim;
                true
            }
            _ => false,
        }
    }
    fn script_muted(&self, sim: &Sim) -> bool {
        if Self::egg_lay(sim) && sim.current_move == Some(Move::SpecialN) {
            // The borrowed animation is Pikachu's Thunder start; mute its cloud/bolt
            // sounds and effects. The host plays Yoshi's Egg Lay cues instead.
            return true;
        }
        sim.current_move == Some(Move::SpecialLw) && sim.stage == THUNDER_WAIT
    }
    fn on_connect(&self, sim: &mut Sim) {
        // Borrowed Egg Lay: the tongue caught something; start the swallow stage now.
        if Self::egg_lay(sim)
            && sim.current_move == Some(Move::SpecialN)
            && sim.stage == EGG_TONGUE
        {
            sim.stage = EGG_SWALLOW;
            sim.stage_frame = 0;
            sim.motion.frame = 0.;
            sim.action.projectile_fired = false;
            return;
        }
        // ftPk_SpecialS_ZeroVelocity (deal_dmg_cb): Skull Bash stops on its victim.
        if sim.current_move == Some(Move::SpecialS) && sim.stage == BASH_GO && sim.stage_value > 0.
        {
            sim.velocity.x = 0.;
            sim.velocity.z = 0.;
            if sim.velocity.y > 0. {
                sim.velocity.y = 0.;
            }
            self.end_bash(sim);
        }
    }
    fn projectile_variable(&self) -> Option<u32> {
        Some(0)
    }

    fn clip(&self, sim: &Sim, m: Move) -> &'static str {
        let started = sim.move_started_grounded;
        let now = sim.grounded;
        if Self::egg_lay(sim) && m == Move::SpecialN {
            // The user-facing choice: Pikachu's Thunder start, where he tenses up.
            return if started { "SpecialLwStart" } else { "SpecialAirLwStart" };
        }
        match (m, sim.stage) {
            (Move::SpecialN, _) => {
                if started && sim.stage_value == 0. { "SpecialN" } else { "SpecialAirN" }
            }
            (Move::SpecialS, BASH_START) => {
                if now { "SpecialSStart" } else { "SpecialAirSStart" }
            }
            (Move::SpecialS, BASH_HOLD) => {
                if now { "SpecialSHold" } else { "SpecialAirSHold" }
            }
            (Move::SpecialS, BASH_GO) => "SpecialS",
            (Move::SpecialS, _) => {
                if sim.stage_dir.x > 0. { "SpecialSEnd" } else { "SpecialAirSEnd" }
            }
            (Move::SpecialHi, QUICK_START) => {
                if started { "SpecialHiStart" } else { "SpecialAirHiStart" }
            }
            (Move::SpecialHi, _) => {
                if started { "SpecialHiEnd" } else { "SpecialAirHiEnd" }
            }
            (Move::SpecialLw, THUNDER_START) => {
                if started { "SpecialLwStart" } else { "SpecialAirLwStart" }
            }
            (Move::SpecialLw, THUNDER_WAIT | THUNDER_STRUCK) => {
                if started { "SpecialLwLoop" } else { "SpecialAirLwLoop" }
            }
            (Move::SpecialLw, _) => {
                if started { "SpecialLwEnd" } else { "SpecialAirLwEnd" }
            }
            _ => "",
        }
    }
    fn landing(&self, _sim: &Sim, m: Move) -> &'static str {
        if m == Move::SpecialHi { "LandingFallSpecial" } else { "Landing" }
    }
    fn facing_locked(&self, m: Move) -> bool {
        matches!(m, Move::SpecialHi | Move::SpecialLw)
    }
    fn facing_locked_now(&self, sim: &Sim, m: Move) -> bool {
        self.facing_locked(m) || (m == Move::SpecialS && sim.stage >= BASH_GO)
    }
    fn spins_whole_body(&self, m: Move) -> bool {
        matches!(m, Move::SpecialS | Move::SpecialHi | Move::SpecialLw)
    }
    fn side_special_layered(&self, _m: Move) -> bool {
        false
    }
    fn side_special_upper_aim(&self, _sim: &Sim) -> Option<(f32, f32)> {
        None
    }
    fn hit_attachment(&self, _sim: &Sim, _m: Move) -> (Option<usize>, Vec3) {
        (None, Vec3::ZERO)
    }
    fn projectile_move(&self, m: Move) -> bool {
        m == Move::SpecialN
    }
    fn projectile_cue(&self, _m: Move) -> Option<(u32, usize, bool)> {
        None
    }
    fn phase_effect(&self, _sim: &Sim, _phase: Phase) -> Option<GfxCue> {
        None
    }
    fn phase_stop(&self, _phase: Phase) -> Option<SoundCue> {
        None
    }

    fn startup(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(18),
            Move::SpecialS => Some(14),
            Move::SpecialHi => Some(14),
            Move::SpecialLw => Some(18),
            _ => None,
        }
    }
    fn recovery(&self, m: Move) -> Option<u32> {
        match m {
            Move::SpecialN => Some(40),
            Move::SpecialS => Some(40),
            Move::SpecialHi => Some(50),
            Move::SpecialLw => Some(36),
            _ => None,
        }
    }
}
