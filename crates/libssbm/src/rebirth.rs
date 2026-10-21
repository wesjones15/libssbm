//! Shared falloff presentation. ft_0D31::{spawnDeadEffect,DeadDown_Anim},
//! ft_0D4D::{Rebirth_Anim,RebirthWait_Anim,RebirthWait_IASA} and
//! ftColl_8007B7A4 specify KO timing, wait/input release and protection.
//! Returning to host-safe ground as a figurine is the requested RoR2 adaptation.
use crate::{Input, Sim};
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Params {
    pub ko_frames: u32,
    pub arrival_frames: u32,
    pub wait_frames: u32,
    pub invulnerable_frames: u32,
    pub ko_scale: f32,
    pub flash: Vec<[f32; 4]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ko,
    Figurine,
    Transform,
    Protected,
}
#[derive(Clone, Debug)]
pub struct State {
    pub params: Params,
    pub entry: crate::entry::Params,
    pub phase: Phase,
    pub frame: u32,
    pub destination: Vec3,
}
impl State {
    pub fn flags(&self) -> u32 {
        2 | match self.phase {
            Phase::Ko => 1 | 4,
            Phase::Figurine => 1 | 8,
            Phase::Transform => 1,
            Phase::Protected => 0,
        }
    }
    pub fn overlay(&self) -> Option<[f32; 4]> {
        if self.phase != Phase::Protected || self.params.flash.is_empty() {
            return None;
        }
        Some(self.params.flash[self.frame as usize % self.params.flash.len()])
    }
}
impl Sim {
    /// A normal host collision correction preserves an already released
    /// fighter's remaining protection; external teleports cancel the sequence.
    pub fn correct_position(&mut self, position: Vec3) {
        let protection = self.rebirth.take().filter(|s| s.phase == Phase::Protected);
        self.teleport(position);
        self.rebirth = protection;
    }
    /// Moves the fighter to `position` without touching its action, velocity or clocks: a
    /// multiplayer shadow sim is pulled back to where the owner's sim really is.
    pub fn nudge(&mut self, position: Vec3, grounded: bool) {
        if position.is_finite() {
            self.position = position;
            self.grounded = grounded;
        }
    }
    pub fn begin_rebirth(
        &mut self,
        params: Params,
        entry: crate::entry::Params,
        destination: Vec3,
    ) -> bool {
        if self.rebirth.as_ref().is_some_and(|s| s.flags() & 1 != 0) || !destination.is_finite() {
            return false;
        }
        self.teleport(self.position);
        self.shot_charge = 0;
        self.defense.mode = None;
        self.defense.air_used = false;
        self.action.reset();
        self.rebirth = Some(State {
            params,
            entry,
            phase: Phase::Ko,
            frame: 0,
            destination,
        });
        true
    }
    /// Initial spawn enters the same figurine return as a fall, without a KO.
    pub fn begin_arrival(&mut self, params: Params, entry: crate::entry::Params) -> bool {
        if !self.begin_rebirth(params, entry, self.position) {
            return false;
        }
        let mut state = self.rebirth.take().unwrap();
        self.return_as_figurine(&mut state);
        self.rebirth = Some(state);
        true
    }
    fn return_as_figurine(&mut self, state: &mut State) {
        self.position = state.destination;
        self.grounded = true;
        self.jumps_used = 0;
        let mut entry = crate::entry::State::new(state.entry);
        entry.waiting = true;
        self.entry = Some(entry);
        self.motion.select("Entry", 0.);
        self.motion.frame = 0.;
        state.phase = Phase::Figurine;
        state.frame = 0;
    }
    /// True while the KO phase owns the tick; entry owns figurine/transform.
    pub(crate) fn tick_rebirth(&mut self, input: Input) -> bool {
        let Some(mut state) = self.rebirth.take() else {
            return false;
        };
        match state.phase {
            Phase::Ko => {
                self.velocity = Vec3::ZERO;
                self.current_move = None;
                self.action.active.clear();
                if state.frame == 0 {
                    self.action.sounds.push(crate::action::SoundCue {
                        sound: 0x61,
                        volume: 127,
                        pan: 64,
                        channel: 0,
                        stop: false,
                    });
                    self.ko_voice();
                    self.action.effects.push(crate::action::GfxCue {
                        id: 0x42b,
                        bone: 0,
                        common: false,
                        cycle: 0,
                        destroy_on_change: false,
                        offset: Vec3::ZERO,
                        param: state.params.ko_scale,
                        clip: "Wait",
                        frame: 0.,
                        origin: self.position,
                        facing: self.facing,
                    });
                }
                state.frame += 1;
                if state.frame >= state.params.ko_frames {
                    self.return_as_figurine(&mut state);
                }
                self.rebirth = Some(state);
                return true;
            }
            Phase::Figurine => {
                if state.frame == 0 {
                    // The vanilla rebirth ring (ft_0D4D.c::ftCo_800D4F24 spawns
                    // 0x43F as the trophy appears).
                    self.action.effects.push(crate::action::GfxCue {
                        id: 0x43f,
                        bone: 0,
                        common: false,
                        cycle: 0,
                        destroy_on_change: false,
                        offset: Vec3::ZERO,
                        param: 1.,
                        clip: "Entry",
                        frame: 0.,
                        origin: self.position,
                        facing: self.facing,
                    });
                }
                state.frame += 1;
                let ready = state.frame >= state.params.arrival_frames
                    && self.entry.is_some_and(|e| e.frame >= e.params.grow);
                let moved = input.movement.with_y(0.).length_squared() > 0.09;
                if ready
                    && (moved
                        || state.frame >= state.params.arrival_frames + state.params.wait_frames)
                {
                    if let Some(entry) = self.entry.as_mut() {
                        entry.waiting = false;
                        entry.frame = entry.frame.max(entry.params.grow);
                        entry.started = false;
                    }
                    state.phase = Phase::Transform;
                    state.frame = 0;
                }
            }
            Phase::Transform => {
                if self.entry.is_none() {
                    state.phase = Phase::Protected;
                    state.frame = 0;
                }
            }
            Phase::Protected => {
                state.frame += 1;
                if state.frame >= state.params.invulnerable_frames {
                    return false;
                }
            }
        }
        self.rebirth = Some(state);
        false
    }
}
