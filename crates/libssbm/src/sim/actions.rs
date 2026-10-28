//! Runtime command execution. ftAction_80073240, ftCo_Attack1/AttackAir and
//! ftSamus's SpecialN/S/Hi/Lw animation callbacks are the behavioral spec.
use super::*;
use crate::action::{ActiveHitbox, Command, HitSample, SoundCue};

/// Host adaptation of the source's two hurt-voice knockback thresholds: the fraction of health
/// a hit takes.
const HURT_LIGHT: f32 = 0.05;
const HURT_HEAVY: f32 = 0.2;

impl Sim {
    pub(super) fn finish_disc_action(&mut self) {
        // A charge interrupted before it fires (a hit, a ledge grab) leaves no held cue behind.
        self.scope_stop_charge_sound(false);
        if self.grabbed {
            self.set_grabbed(true);
            return;
        }
        self.action.active.clear();
        self.afterimage = -1;
        let ended = self.current_move.take();
        // Up specials do not enter helpless fall: the host lets the fighter act
        // airborne, and its i-frames survive until landing or a new attack. A
        // behavior can ask for the source's special fall (Luigi's Super Jump
        // Punch): helpless drift, then landing lag.
        let fall = if self.grounded {
            None
        } else {
            ended.and_then(|m| self.behavior.special_fall(m))
        };
        if let Some(fall) = fall {
            self.special_fall = Some(fall);
            self.set_phase(Phase::Helpless);
        } else {
            self.special_fall = None;
            self.set_phase(Phase::Free);
        }
    }
    pub(super) fn update_disc_action(&mut self, host: &mut impl Host, input: Input, pressed: u32) {
        let m = self.current_move.unwrap();
        if m == Move::GrabHold {
            if pressed & (ATTACK | SPECIAL) != 0 {
                let m = if input.buttons & UP != 0 {
                    Move::ThrowUp
                } else if input.buttons & DOWN != 0 {
                    Move::ThrowDown
                } else if input.movement.y > 0.5 {
                    Move::ThrowUp
                } else if input.movement.y < -0.5 {
                    Move::ThrowDown
                } else if input.movement.x > 0.5 {
                    Move::ThrowForward
                } else if input.movement.x < -0.5 {
                    Move::ThrowBack
                } else {
                    Move::Pummel
                };
                self.start(host, m, input);
            }
            return;
        }
        if m == Move::SuperScope {
            self.update_scope(host, input, pressed);
            return;
        }
        if matches!(m, Move::Jab | Move::Jab2 | Move::Jab3) && self.jab_chain(host, input, pressed) {
            return;
        }
        if m == Move::RapidJab {
            // ftCo_Attack100Loop_IASA: A pressed or released since the last beat keeps it going.
            if (pressed | self.released) & ATTACK != 0 {
                self.jab.fed = true;
            }
            // ftCo_Attack100Start_Anim hands over to the loop; the loop ends on a beat
            // (see the ThrowFlag command); the end clip plays out.
            match self.stage {
                0 if self.motion.finished() => {
                    self.stage = 1;
                    self.jab.looping = true;
                }
                2 if self.motion.finished() => self.finish_disc_action(),
                _ => {}
            }
            return;
        }
        if self.action.interrupt && pressed & ATTACK != 0 {
            self.start(host, self.choose_attack(input), input);
            return;
        }
        if !self.motion.finished() {
            return;
        }
        // A held stage (Thunder's wait) has finished its clip but is still running on
        // its own clock; it must not end the move until the behavior advances it.
        if self.using_behavior(|b, s| b.stage_held(s)) {
            return;
        }
        // A multi-stage special (Pikachu's Skull Bash, Quick Attack, Thunder)
        // continues into its next clip instead of ending with this one.
        if self.using_behavior(|b, s| b.next_stage(s, host, m, input)) {
            return;
        }
        if m == Move::SpecialN && self.phase == Phase::Startup {
            if self.using_behavior(|b, s| b.resolve_startup(s, input)) {
                // The special entered its hold phase.
            } else if self.behavior.fire_replays(m) {
                self.fire(host);
            } else {
                self.finish_disc_action();
            }
        } else {
            self.finish_disc_action();
        }
    }

    /// The jab after `m` in its chain, from the clips the fighter has: Attack12, Attack13; a
    /// fighter with no Attack12 (Pikachu) repeats Attack11 (doAttack12Rapid).
    fn jab_next(&self, m: Move) -> Option<Move> {
        let has = |clip: &str| self.action.definitions.contains_key(clip);
        match m {
            Move::Jab if has("Attack12") => Some(Move::Jab2),
            Move::Jab => Some(Move::Jab),
            Move::Jab2 if has("Attack13") => Some(Move::Jab3),
            _ => None,
        }
    }

    /// A press after a jab has ended but inside its window (ftCo_Attack1_CheckInput).
    pub(super) fn jab_resume(&self) -> Option<Move> {
        let last = self.jab.last?;
        if self.jab.window > 0 && self.grounded && self.action.jab_combo {
            self.jab_next(last)
        } else {
            None
        }
    }

    /// The jab's input checks while it plays: ftCo_Attack_800D6A50 (every press and every release
    /// of A counts toward the rapid jab, once a script has allowed it) then checkAttack12/13 (a
    /// press inside the window, with the script's combo flag open, starts the next jab).
    fn jab_chain(&mut self, host: &mut impl Host, input: Input, pressed: u32) -> bool {
        if (pressed | self.released) & ATTACK != 0 {
            self.jab.presses += 1;
        }
        let serial = self.action.serial;
        if self.jab.rapid_ok
            && self.jab.presses >= self.params.rapid_jab_window.max(1)
            && self.action.definitions.contains_key("Attack100Loop")
        {
            self.start(host, Move::RapidJab, input);
            return self.action.serial != serial;
        }
        if self.jab.window > 0 && pressed & ATTACK != 0 {
            self.jab.queued = true;
        }
        if self.jab.queued && self.action.jab_combo {
            if let Some(next) = self.jab.last.and_then(|m| self.jab_next(m)) {
                self.start(host, next, input);
                return self.action.serial != serial;
            }
        }
        false
    }

    fn disc_projectile(&mut self, host: &mut impl Host) {
        if self.action.projectile_fired || self.behavior.script_muted(self) {
            return;
        }
        let Some(m) = self.current_move else {
            return;
        };
        if !self.behavior.projectile_move(m) {
            return;
        }
        self.action.projectile_fired = true;
        if let Some((id, bone, common)) = self.behavior.projectile_cue(m) {
            self.action.effects.push(crate::action::GfxCue {
                id,
                bone,
                common,
                cycle: 0,
                destroy_on_change: false,
                offset: Vec3::ZERO,
                param: 0.,
                clip: self.motion.clip,
                frame: self.motion.frame,
                origin: self.position,
                facing: self.facing,
            });
        }
        let strength = self.using_behavior(|b, s| b.fire_strength(s, m));
        self.emit(host, EventKind::Hit, m as u32, strength);
    }

    pub(crate) fn execute_disc_commands(&mut self, host: &mut impl Host, input: Input) {
        let clip = self.motion.clip;
        // A clip can stand for more than one disc motion state (Pikachu's Thunder wait
        // and struck loops share `SpecialLwLoop`); the behavior picks the state's script.
        // Fall back to the clip if the caller loaded only the base scripts.
        let script = self
            .behavior
            .script_key(self)
            .filter(|key| self.action.definitions.contains_key(*key))
            .unwrap_or(clip);
        // The Super Scope's Start script keeps running after its clip holds its last pose.
        let frame = self.scope_script_frame().unwrap_or(self.motion.frame);
        if self.action.script_clip != script {
            // ftAnim_80070F28 (on every motion change): part animations end.
            self.parts = [None; 5];
        }
        if self.action.script_clip != script || frame < self.action.script_frame {
            self.action.cursor = 0;
            self.action.script_clip = script;
            // ftAnim_80070654: a new motion puts the face back to its first frame.
            self.face = [0.; 8];
        }
        self.action.script_frame = frame;
        // A waiting stage (Thunder's cloud) replays a clip whose hitboxes and effects
        // belong to a later stage; consume them without acting. Sound commands still
        // play, so a held stage keeps its thunder rumble.
        let muted = self.behavior.script_muted(self);
        let mute_sounds = self.behavior.script_mute_sounds(self);
        while let Some(event) = self
            .action
            .definitions
            .get(script)
            .and_then(|a| a.commands.get(self.action.cursor))
            .cloned()
        {
            if event.frame > frame {
                break;
            }
            self.action.cursor += 1;
            if muted
                && matches!(
                    event.command,
                    Command::Hitbox(_)
                        | Command::Raw {
                            opcode: 10,
                            ..
                        }
                )
            {
                continue;
            }
            if mute_sounds
                && matches!(
                    event.command,
                    Command::Raw {
                        opcode: 17 | 18 | 54 | 55,
                        ..
                    }
                )
            {
                continue;
            }
            // The loaded scope's scripts spawn the charge dust, the recoil and the shot's ground dust; the empty
            // scope's (ftCo_MS_ItemScope*Empty, which shares their clips) spawn none, so an empty one mutes them.
            if self.current_move == Some(Move::SuperScope)
                && self.scope.as_ref().is_some_and(|scope| scope.state.ammo == 0)
                && matches!(event.command, Command::Raw { opcode: 10, .. })
            {
                continue;
            }
            // The Start script's second sound (0xFE, frame 16) is the held charge cue, which the scope's own
            // logic plays on a track of its own so firing can stop it (`update_scope`).
            if self.current_move == Some(Move::SuperScope)
                && self.scope.is_some()
                && matches!(&event.command, Command::Raw { opcode: 17, words } if words.get(1) == Some(&0xFE))
            {
                continue;
            }
            match event.command {
                Command::Hitbox(hitbox) => {
                    let generation = if let Some(old) = self
                        .action
                        .active
                        .values()
                        .find(|a| a.hitbox.group == hitbox.group)
                    {
                        old.generation
                    } else {
                        let slot = &mut self.action.generations[hitbox.group as usize];
                        *slot = slot.wrapping_add(1);
                        *slot
                    };
                    self.action
                        .active
                        .insert(hitbox.id, ActiveHitbox { hitbox, generation });
                }
                Command::ClearHitbox(id) => {
                    self.action.active.remove(&id);
                }
                Command::ClearHitboxes => self.action.active.clear(),
                Command::HitboxDamage(id, damage) => {
                    if let Some(h) = self.action.active.get_mut(&id) {
                        h.hitbox.damage = damage;
                    }
                }
                Command::HitboxRadius(id, radius) => {
                    if let Some(h) = self.action.active.get_mut(&id) {
                        h.hitbox.radius = radius;
                    }
                }
                Command::Variable(index, value) => {
                    self.action.variables[index as usize] = value;
                    // SpecialNHold_Anim selects one of five source charge voices
                    // when cmd_var2 requests it; replace the tracked charge slot.
                    if value == 1
                        && self.behavior.projectile_variable() == Some(index)
                        && self.current_move.is_some_and(|m| self.behavior.projectile_move(m))
                    {
                        self.disc_projectile(host);
                    }
                    if index == 2 && value == 1 && self.phase == Phase::ChargeShot {
                        let band = self.behavior.charge_voice_band(self);
                        self.action.sounds.push(SoundCue {
                            sound: 260006 + band * 3,
                            volume: 127,
                            pan: 64,
                            channel: 6,
                            stop: false,
                        });
                    }
                    if value == 1
                        && index == 1
                        && self.current_move.is_some_and(|m| self.behavior.is_charge_move(m))
                        && self.phase != Phase::ChargeShot
                    {
                        self.disc_projectile(host);
                    }
                }
                Command::ThrowFlag(_) => {
                    // ftCo_ItemScopeFire / Rapid: the clip's throw flag is the shot.
                    if self.current_move == Some(Move::SuperScope) {
                        self.scope_shoot(host);
                    }
                    // ftCo_Attack100Loop_Anim: the loop's beat. With no press or release since
                    // the last one, the rapid jab ends; otherwise it goes round again.
                    if self.current_move == Some(Move::RapidJab)
                        && self.stage == 1
                        && clip == "Attack100Loop"
                    {
                        if self.jab.looping && !self.jab.fed {
                            self.stage = 2;
                        } else {
                            self.jab.fed = false;
                        }
                    }
                    if self.grabbed
                        && matches!(
                            self.current_move,
                            Some(
                                Move::ThrowForward
                                    | Move::ThrowBack
                                    | Move::ThrowUp
                                    | Move::ThrowDown
                            )
                        )
                        && !self.action.projectile_fired
                    {
                        self.action.projectile_fired = true;
                        if let Some(hit) = self.action.throw_hit.clone() {
                            let angle = if hit.angle == 361 {
                                45.
                            } else {
                                hit.angle as f32
                            }
                            .to_radians();
                            self.move_aim = self.facing * angle.cos() + Vec3::Y * angle.sin();
                            self.emit(
                                host,
                                EventKind::Throw,
                                self.current_move.unwrap() as u32,
                                hit.damage,
                            );
                        }
                        self.grabbed = false;
                    }
                    if self
                        .current_move
                        .is_some_and(|m| self.behavior.projectile_move(m))
                    {
                        self.disc_projectile(host);
                    }
                }
                Command::Interrupt => self.action.interrupt = true,
                Command::JabCombo(value) => self.action.jab_combo = value,
                Command::JabRapid(on) => self.jab.rapid_ok = on,
                Command::SmashCharge {
                    max_frames,
                    damage_per_frame,
                } => {
                    if self.current_move.is_some_and(Move::is_smash)
                        && input.buttons & ATTACK != 0
                        && self.smash_charge == 0
                    {
                        self.action.smash_max_frames = max_frames;
                        self.action.smash_multiplier = damage_per_frame;
                        self.set_phase(Phase::SmashCharge);
                        self.motion.rate = 0.;
                    }
                }
                Command::Visibility { group, choice } => {
                    self.action.visibility.insert(group, choice);
                }
                Command::Raw { opcode, words } => {
                    // ftAction_80071CCC: the smash voice.
                    if opcode == 18 {
                        let ids = self.params.voices.smash.clone();
                        if let Some(cue) = self.pick_voice(&ids) {
                            self.action.sounds.push(cue);
                        }
                    }
                    // ftAction_80071A14/1A58: global override/all-capsule hurt state.
                    // EscapeAir uses 26 (not per-bone command 28).
                    if opcode == 26 || opcode == 27 {
                        self.action.hurt_state = words[0] & 0x03ffffff;
                    }
                    // lb/types.h set_throw_hitbox_0/1/2 and ftAction_80071E04.
                    if opcode == 34 && words[0] >> 23 & 7 == 0 {
                        self.action.throw_hit = Some(crate::action::Hitbox {
                            damage: (words[0] & 0x7fffff) as f32,
                            angle: words[1] >> 23,
                            growth: words[1] >> 14 & 511,
                            fixed_knockback: words[1] >> 5 & 511,
                            base_knockback: words[2] >> 23,
                            element: words[2] >> 19 & 15,
                            sfx_severity: words[2] >> 16 & 7,
                            sfx_kind: words[2] >> 12 & 15,
                            ..Default::default()
                        });
                    }
                    // ftAction_80072A5C -> ftCo_800BFFD0: start a colour animation.
                    if opcode == 46 {
                        let id = (words[0] >> 18 & 0xff) as usize;
                        if self.colanim.start(id, words[0] & 0x3ffff) {
                            self.colanim_move = self.current_move;
                            self.colanim_clip = clip;
                        }
                    }
                    // ftAction_800726F4: set the face's texture animation frame (one or two TObjs).
                    if opcode == 40 {
                        let frame = (((words[0] & 0x7ff) as i32) << 21 >> 21) as f32;
                        let idx = (words[0] >> 18 & 0x7f) as usize;
                        let idx2 = (words[0] >> 11 & 0x7f) as usize;
                        if idx < self.face.len() {
                            self.face[idx] = frame;
                        }
                        if words[0] >> 25 & 1 != 0 && idx2 < self.face.len() {
                            self.face[idx2] = frame;
                        }
                    }
                    // ftAction_800727C8: start part animation `index` in `slot` (hand poses, the
                    // mouth), blending in over `blend` frames.
                    if opcode == 41 {
                        let slot = (words[0] >> 19 & 0x7f) as usize;
                        if slot < self.parts.len() {
                            self.parts[slot] = Some(crate::sim::PartPlay {
                                index: (words[0] >> 12 & 0x7f) as u8,
                                age: 0.,
                                blend: (words[0] & 0xfff) as u16,
                            });
                        }
                    }
                    // ftAction_80072B3C: the sword trail on (0) or off (-1).
                    if opcode == 49 {
                        let raw = (words[0] & 0x01ff_ffff) as i32;
                        self.afterimage = (raw << 7) >> 7;
                    }
                    if opcode == 10 {
                        self.queue_gfx(&words, clip, frame);
                    }
                    // ftAction_80071B50: play/replace/stop a fighter sound slot.
                    // 54/55 supply the stored footstep/landing cue; world-surface
                    // substitution remains a host adaptation, using the disc cue.
                    if matches!(opcode, 17 | 54 | 55) {
                        self.queue_sound_command(opcode, &words);
                    }
                    // ftAction_80071FC8: a random pick of up to six named sounds (the
                    // pseudo-random SFX). Specials name their voice cries here.
                    if opcode == 38 {
                        self.queue_random_sfx(&words);
                    }
                }
            }
        }
        if muted {
            self.action.active.clear();
        }
        if self.current_move.is_some() {
            for active in self.action.active.values() {
                self.action.samples.push(HitSample {
                    hitbox: active.hitbox.clone(),
                    generation: active.generation,
                    serial: self.action.serial,
                    clip,
                    frame,
                    origin: self.position,
                    facing: self.facing,
                    strength: self.move_strength,
                });
            }
        }
    }
}

impl Sim {
    /// Runs the colour animations a tick and applies their GFX and sound commands. A glow ends
    /// with the motion it was started in, unless that motion goes on into the next clip of the
    /// same move (Fox's Reflector, Samus's charge).
    pub(super) fn tick_colanim(&mut self) {
        if !self.colanim.active() {
            return;
        }
        if self.motion.clip != self.colanim_clip {
            if self.colanim_move.is_some() && self.current_move == self.colanim_move {
                self.colanim_clip = self.motion.clip;
            } else {
                self.colanim.clear();
                return;
            }
        }
        for event in self.colanim.tick() {
            match event {
                crate::colanim::Event::Gfx(words) => {
                    let clip = self.motion.clip;
                    let frame = self.motion.frame;
                    self.queue_gfx(&words, clip, frame);
                }
                crate::colanim::Event::Sound(words) => self.queue_sound(&words),
            }
        }
    }
}

impl Sim {
    /// ftAction_80071028: a GFX spawn command's five words become a cue.
    /// A random number in -0.5..0.5 for a cue's spread (HSD_Randf - 0.5), from a generator separate from
    /// the gameplay one.
    fn gfx_unit(&mut self) -> f32 {
        let mut x = self.gfx_rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.gfx_rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32 - 0.5
    }
    fn queue_gfx(&mut self, words: &[u32], clip: &'static str, frame: f32) {
        let signed = |n: u32| (n as u16 as i16) as f32 / 256.;
        let bone = if words[0] & (1 << 15) != 0 {
            usize::MAX
        } else {
            (words[0] >> 18 & 255) as usize
        };
        // ftCo_8009F834 cycles the fighter's five source bones.
        // Save the selection on the cue so repeated FFI queries
        // cannot advance it again or produce different matrices.
        let cycle = self.action.effect_cycle;
        if bone == 0x8d {
            self.action.effect_cycle = (cycle + 1) % 5;
        }
        // ftCo_8009F834: the position is the offset plus a random point within +/- the range on each axis
        // (words 3-4: x, y, z as unsigned 8.8). The sim's own generator, so gameplay rolls never move.
        let range = |n: Option<&u32>, shift: u32| (n.map_or(0, |w| w >> shift & 65535) as f32) / 256.;
        let ranges = Vec3::new(range(words.get(3), 0), range(words.get(4), 16), range(words.get(4), 0));
        let spread = Vec3::new(self.gfx_unit(), self.gfx_unit(), self.gfx_unit()) * 2. * ranges;
        self.action.effects.push(crate::action::GfxCue {
            id: words[1] >> 16,
            bone,
            common: words[0] & (1 << 17) != 0,
            cycle,
            destroy_on_change: words[0] & (1 << 16) != 0,
            offset: Vec3::new(
                signed(words[2] >> 16),
                signed(words[2]),
                signed(words[3] >> 16),
            ) + spread,
            param: (words[1] & 65535) as f32,
            clip,
            frame,
            origin: self.position,
            facing: self.facing,
        });
    }
    /// A voice cue on the fighter's voice slot (ft_800881D8): a new voice replaces the last.
    fn voice_cue(id: u32) -> SoundCue {
        SoundCue { sound: id, volume: 127, pan: 64, channel: VOICE_CHANNEL, stop: false }
    }
    /// ft_800889F4: one of an array of voices, picked at random. Its own generator, so voices
    /// never move the sim's.
    fn pick_voice(&mut self, ids: &[u32]) -> Option<SoundCue> {
        if ids.is_empty() {
            return None;
        }
        let mut x = self.voice_rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.voice_rng = x;
        Some(Self::voice_cue(ids[x as usize % ids.len()]))
    }
    /// The blast-zone KO voices (ft_0D31.c playDeadSfx).
    pub(crate) fn ko_voice(&mut self) {
        for id in self.params.voices.ko.clone() {
            self.action.sounds.push(Self::voice_cue(id));
        }
    }
    /// ftCo_Damage.c: a big enough hit plays a hurt voice (a light or a heavy set) and the
    /// common sound 0x50 or 0x4F. The source thresholds knockback; the host adaptation
    /// thresholds the fraction of health the hit took. Queued, so it sounds on the next step.
    pub(crate) fn hurt_voice(&mut self, severity: f32) {
        let (sfx, ids) = if severity >= HURT_HEAVY {
            (0x4f, self.params.voices.hurt_heavy.clone())
        } else if severity >= HURT_LIGHT {
            (0x50, self.params.voices.hurt_light.clone())
        } else {
            return;
        };
        self.queued_sounds.push(SoundCue { sound: sfx, volume: 127, pan: 64, channel: 0, stop: false });
        if let Some(cue) = self.pick_voice(&ids) {
            self.queued_sounds.push(cue);
        }
    }
    /// The mixer channel a sound behavior targets: the fighter's replaceable voice slot
    /// (behavior 2, ft_800881D8), the replaceable x2148 slot (behavior 6, ft_80088328),
    /// and the shared untracked one-shot channel for the rest.
    fn sound_channel(behavior: u32) -> Option<u32> {
        if behavior <= 6 || (10..=15).contains(&behavior) {
            let slot = if behavior >= 10 { behavior - 9 } else { behavior };
            Some(match slot {
                2 => VOICE_CHANNEL,
                6 => 6,
                _ => 0,
            })
        } else {
            None
        }
    }
    /// ftAction_80071B50: play/replace/stop a fighter sound slot. 54/55 supply the stored
    /// footstep/landing cue; world-surface substitution remains a host adaptation, using the
    /// disc cue. The fighter has one replaceable voice slot (behavior 2, ft_800881D8) and one
    /// replaceable x2148 slot (behavior 6, ft_80088328) — a new sound replaces the last there.
    /// Behaviors 0/1/3/4/5 (ft_PlaySFX, ft_80088478, ft_80088510, ft_800885A8, ft_80088640)
    /// are one-shot slots that coexist, so they share the untracked overlap channel.
    fn queue_sound_command(&mut self, opcode: u32, words: &[u32]) {
        let behavior = if opcode == 17 { words[0] >> 18 & 255 } else { 0 };
        let Some(channel) = Self::sound_channel(behavior) else {
            return;
        };
        self.action.sounds.push(SoundCue {
            sound: words[1],
            volume: (words[2] >> 8) as u8,
            pan: words[2] as u8,
            channel,
            stop: behavior >= 10,
        });
    }
    /// ftAction_80071FC8: play one of up to six named sounds at random (the pseudo-random
    /// SFX). Specials name their launch cries here (Fox's Fire Fox "Hyaa!"), and the behavior
    /// picks the same replaceable voice slot a stored sound would. `voice_rng` keeps the choice
    /// out of the sim's RNG, exactly as the smash grunt's pick does.
    fn queue_random_sfx(&mut self, words: &[u32]) {
        let volume = (words[0] >> 18 & 0xff) as u8;
        let panning = (words[0] >> 10 & 0xff) as u8;
        let behavior = words[0] >> 6 & 0xf;
        let range = words[0] & 0x3f;
        let Some(channel) = Self::sound_channel(behavior) else {
            return;
        };
        let mut x = self.voice_rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.voice_rng = x;
        let index = if range == 0 { 0 } else { x as usize % range.min(6) as usize };
        self.action.sounds.push(SoundCue {
            sound: words[1 + index],
            volume,
            pan: panning,
            channel,
            stop: false,
        });
    }
    fn queue_sound(&mut self, words: &[u32]) {
        self.queue_sound_command(17, words);
    }
}
