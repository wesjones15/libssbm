//! Using the Super Scope (ftCo_ItemScopeStart / Fire / Rapid / End and their air twins). The button is held to
//! charge and let go to fire the charged shot, or pressed quickly for a burst of small ones; the disc's clips
//! supply the poses, lengths and the frame each shot leaves on.
//!
//! Host adaptations: the source's A button is the skill's button, the beam goes where the camera aims, and
//! a spent magazine refills slowly (the host asks for the rate).
use super::*;
use crate::action::SoundCue;
use crate::scope::{Scope, ScopeParams};

/// `ItemScopeStart` while charging, `Fire`, `Rapid` (taps), `End`.
const STAGE_START: u32 = 0;
const STAGE_FIRE: u32 = 1;
const STAGE_RAPID: u32 = 2;
const STAGE_END: u32 = 3;

/// What `ft_PlaySFX` plays: the small shot, the mid and large charges, and the click of an empty scope
/// (ftCo_ItemScopeFire.c fn_800D84D4). The Start clip's own command is the cocking chirp (SOUND_COCK);
/// the disc's separate charge cue (SOUND_CHARGE, `SFX_IT_SCORPE_TAME`) plays once when that clip runs out.
const SOUND_SMALL: u32 = 0xFC;
const SOUND_CHARGE: u32 = 0xFE;
const SOUND_MEDIUM: u32 = 0xFF;
const SOUND_LARGE: u32 = 0x100;
const SOUND_EMPTY: u32 = 0x101;
/// A track of its own, so firing can stop the charge cue without touching the shot or voice slots.
const SCOPE_SOUND_CHANNEL: u32 = 7;

fn scope_sound(sound: u32, stop: bool) -> SoundCue {
    SoundCue {
        sound,
        volume: 127,
        pan: 64,
        channel: SCOPE_SOUND_CHANNEL,
        stop,
    }
}

impl Sim {
    /// Host adaptation: aim the Scope's upper rig with camera elevation while
    /// retaining the source ItemScope clips and lower-body locomotion layer.
    pub fn scope_upper_pitch(&self) -> Option<f32> {
        self.scope_stage().map(|_| scope_aim_pitch(self.move_aim))
    }
    /// Gives the fighter the Super Scope (with a full magazine), or takes it away.
    pub fn set_scope(&mut self, params: Option<ScopeParams>) {
        self.scope_stop_charge_sound(true);
        self.scope = params.map(|params| Scope { state: crate::scope::ScopeState::new(&params), params });
    }

    /// Stops the held-charge cue if it is playing. `queued` is for calls outside a tick (the stop must survive
    /// the next `advance`'s sound-cue clear).
    pub(super) fn scope_stop_charge_sound(&mut self, queued: bool) {
        let was = match self.scope.as_mut() {
            Some(scope) => std::mem::replace(&mut scope.state.charge_sound, false),
            None => false,
        };
        if !was {
            return;
        }
        let cue = scope_sound(SOUND_CHARGE, true);
        if queued {
            self.queued_sounds.push(cue);
        } else {
            self.action.sounds.push(cue);
        }
    }

    /// The disc clip for the scope's stage, ground or air.
    pub(crate) fn scope_clip(&self) -> &'static str {
        match (self.stage, self.grounded) {
            (STAGE_START, true) => "ItemScopeStart",
            (STAGE_START, false) => "ItemScopeAirStart",
            (STAGE_FIRE, true) => "ItemScopeFire",
            (STAGE_FIRE, false) => "ItemScopeAirFire",
            (STAGE_RAPID, true) => "ItemScopeRapid",
            (STAGE_RAPID, false) => "ItemScopeAirRapid",
            (_, true) => "ItemScopeEnd",
            (_, false) => "ItemScopeAirEnd",
        }
    }

    /// Where the scope is in its use, for a HUD: (`ScopeStage`-like code 0 charging, 1 firing, 2 burst, 3 ending), or
    /// `None` when the scope is not being used.
    pub fn scope_stage(&self) -> Option<u32> {
        (self.current_move == Some(Move::SuperScope)).then_some(self.stage)
    }

    /// A move start: the hold begins again; the magazine is as it was.
    pub(crate) fn scope_begin(&mut self) {
        self.scope_stop_charge_sound(false);
        if !self.grounded && self.motion.clip.starts_with("Jump") {
            self.locomotion.select(self.motion.clip, self.motion.rate);
            self.locomotion.frame = self.motion.frame;
        }
        if let Some(scope) = self.scope.as_mut() {
            scope.state.jump_squat = None;
            scope.state.held = 0;
            scope.state.released = false;
            scope.state.level = None;
            scope.state.shots_left = 0;
            scope.state.charge_sound = false;
        }
    }

    /// Ticks the scope's use each step: ftCo_ItemScopeStart_IASA (the charge and what it becomes once its clip has
    /// played), the rapid burst's beat and the end.
    pub(super) fn update_scope(&mut self, host: &mut impl Host, input: Input, pressed: u32) {
        let Some(mut scope) = self.scope.take() else {
            self.finish_disc_action();
            return;
        };
        // Host adaptation: reuse the normal jump physics/sounds, but keep the
        // Scope action clock and run KneeBend/Jump on the lower-body clock.
        if pressed & JUMP != 0 {
            if self.grounded && scope.state.jump_squat.is_none() {
                scope.state.jump_squat = Some(0);
                self.short_hop = false;
            } else if !self.grounded && self.jumps_used < self.params.jumps + self.options.extra_jumps {
                self.jump(host, true, input);
            }
        }
        if let Some(tick) = scope.state.jump_squat {
            self.short_hop |= input.buttons & JUMP == 0;
            if !self.grounded { scope.state.jump_squat = None; }
            else if tick >= self.params.jump_squat {
                scope.state.jump_squat = None;
                self.jump(host, false, input);
            } else { scope.state.jump_squat = Some(tick + 1); }
        }
        let held = input.buttons & NEUTRAL_B != 0;
        match self.stage {
            STAGE_START => {
                if !scope.state.released {
                    if held {
                        scope.state.held += 1;
                    } else {
                        scope.state.released = true;
                    }
                }
                if self.motion.finished() {
                    // The Start clip (and its cocking chirp) has run out. A trigger still down is a charge:
                    // play the disc's charge cue once for this use, on its own track so firing can stop it.
                    if held && !scope.state.released && !scope.state.charge_sound {
                        scope.state.charge_sound = true;
                        self.action.sounds.push(scope_sound(SOUND_CHARGE, false));
                    }
                    let too_long = scope.state.held as f32 >= scope.params.hold_max;
                    if too_long || scope.state.released {
                        if !too_long && scope.params.level(scope.state.held) == 0 {
                            self.scope_rapid(&mut scope);
                        } else {
                            let level = scope.params.affordable(scope.state.ammo, scope.state.held);
                            self.scope_fire(&mut scope, level);
                        }
                    }
                }
            }
            STAGE_FIRE | STAGE_END => {
                if self.motion.finished() {
                    self.finish_disc_action();
                }
            }
            STAGE_RAPID => {
                // ftCo_ItemScopeRapid_IASA: a press tops the burst up.
                if pressed & NEUTRAL_B != 0 {
                    scope.state.shots_left = scope.params.rapid_shots;
                }
                // The next beat of the loop: the clip has come round (or ended, if the sim must restart it).
                let wrapped = self.motion.frame < scope.state.last_frame;
                scope.state.last_frame = self.motion.frame;
                if wrapped || self.motion.finished() {
                    self.motion.frame = 0.;
                    scope.state.last_frame = 0.;
                    self.action.projectile_fired = false;
                    self.scope_beat(&mut scope);
                }
            }
            _ => {}
        }
        if self.scope.is_none() {
            self.scope = Some(scope);
        }
    }

    fn scope_fire(&mut self, scope: &mut Scope, level: Option<u32>) {
        if scope.state.charge_sound {
            scope.state.charge_sound = false;
            self.action.sounds.push(scope_sound(SOUND_CHARGE, true));
        }
        self.stage = STAGE_FIRE;
        self.motion.frame = 0.;
        self.action.projectile_fired = false;
        scope.state.level = level;
    }

    fn scope_rapid(&mut self, scope: &mut Scope) {
        if scope.state.charge_sound {
            scope.state.charge_sound = false;
            self.action.sounds.push(scope_sound(SOUND_CHARGE, true));
        }
        self.stage = STAGE_RAPID;
        self.motion.frame = 0.;
        self.action.projectile_fired = false;
        scope.state.shots_left = scope.params.rapid_shots;
        scope.state.last_frame = 0.;
        self.scope_beat(scope);
    }

    /// ftCo_ItemScopeRapid_Anim, at the start of a loop: one fewer tap left, and none left is the end.
    fn scope_beat(&mut self, scope: &mut Scope) {
        scope.state.shots_left = scope.state.shots_left.saturating_sub(1);
        if scope.state.shots_left == 0 {
            self.stage = STAGE_END;
            self.motion.frame = 0.;
        }
    }

    /// The shot: the Fire clip's throw flag, or each Rapid loop's (fn_800D84D4). The beam is the host's; it gets
    /// a `Hit` event whose strength is the charge level.
    pub(super) fn scope_shoot(&mut self, host: &mut impl Host) {
        if self.action.projectile_fired || !matches!(self.stage, STAGE_FIRE | STAGE_RAPID) {
            return;
        }
        self.action.projectile_fired = true;
        let Some(mut scope) = self.scope.take() else { return };
        let level = if self.stage == STAGE_FIRE {
            scope.state.level
        } else {
            scope.params.affordable(scope.state.ammo, 0)
        };
        let sound = |sound| SoundCue { sound, volume: 127, pan: 64, channel: 0, stop: false };
        match level {
            Some(level) => {
                scope.state.spend(&scope.params, level);
                self.action.sounds.push(sound(match level {
                    0 => SOUND_SMALL,
                    1..=4 => SOUND_MEDIUM,
                    _ => SOUND_LARGE,
                }));
                self.emit(host, EventKind::Hit, Move::SuperScope as u32, level as f32);
            }
            // An empty scope only clicks (efSync 0x405 and ft_PlaySFX 0x101).
            None => self.action.sounds.push(sound(SOUND_EMPTY)),
        }
        self.scope = Some(scope);
    }

    /// The magazine's slow refill (`ScopeParams::refill_per_second`), only while the scope is not in use; each tick.
    pub(super) fn scope_refill(&mut self, dt: f32) {
        let in_use = self.current_move == Some(Move::SuperScope);
        if let Some(scope) = self.scope.as_mut() {
            let per_second = scope.params.refill_per_second;
            if in_use {
                scope.state.idle = 0;
                return;
            }
            scope.state.idle = scope.state.idle.saturating_add(1);
            // A second's pause after the last shot before it starts.
            if scope.state.idle > 60 && scope.state.ammo < scope.params.ammo && per_second > 0. {
                scope.state.refill_carry += per_second * dt;
                let whole = scope.state.refill_carry.floor();
                if whole >= 1. {
                    scope.state.refill_carry -= whole;
                    scope.state.ammo = (scope.state.ammo + whole as u32).min(scope.params.ammo);
                }
            }
        }
    }
}

/// Bounded bend of a +Z-forward upper rig toward a camera aim; projectiles keep
/// the full aim. Used by the Super Scope's torso and Yoshi's tongue head.
pub fn aim_pitch(aim: Vec3) -> f32 {
    if !aim.is_finite() { return 0.; }
    aim.y.atan2(aim.with_y(0.).length()).clamp(-60_f32.to_radians(), 60_f32.to_radians())
}

/// The Super Scope's torso bend (see [`aim_pitch`]).
pub fn scope_aim_pitch(aim: Vec3) -> f32 {
    aim_pitch(aim)
}
