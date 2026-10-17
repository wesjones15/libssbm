//! Runtime command execution. ftAction_80073240, ftCo_Attack1/AttackAir and
//! ftSamus's SpecialN/S/Hi/Lw animation callbacks are the behavioral spec.
use super::*;
use crate::action::{ActiveHitbox, Command, HitSample, SoundCue};

impl Sim {
    fn finish_disc_action(&mut self) {
        if self.grabbed {
            self.set_grabbed(true);
            return;
        }
        self.action.active.clear();
        self.current_move = None;
        self.set_phase(if self.release_phase && !self.grounded {
            Phase::Helpless
        } else {
            Phase::Free
        });
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
        if m == Move::Jab && self.action.jab_combo && pressed & ATTACK != 0 {
            self.start(host, Move::Jab2, input);
            return;
        }
        if self.action.interrupt && pressed & ATTACK != 0 {
            self.start(host, self.choose_attack(input), input);
            return;
        }
        if !self.motion.finished() {
            return;
        }
        if m == Move::ChargeShot && self.phase == Phase::Startup {
            if self.grounded
                && self.shot_charge < self.params.charge_steps
                && !(self.options.hold_charge && input.buttons & SPECIAL == 0)
            {
                self.charge_clock = 0;
                self.set_phase(Phase::ChargeShot);
            } else {
                self.fire(host);
            }
        } else {
            self.finish_disc_action();
        }
    }

    fn disc_projectile(&mut self, host: &mut impl Host) {
        if self.action.projectile_fired {
            return;
        }
        let Some(m) = self.current_move else {
            return;
        };
        if !matches!(
            m,
            Move::ChargeShot | Move::Missile | Move::SuperMissile | Move::Bomb
        ) {
            return;
        }
        self.action.projectile_fired = true;
        let cue = match m {
            Move::ChargeShot => Some((1158, 51, false)),
            Move::Missile | Move::SuperMissile => Some((1155, 56, false)),
            _ => None,
        };
        if let Some((id, bone, common)) = cue {
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
        let strength = if m == Move::ChargeShot {
            let charge = self.shot_charge as f32 / self.params.charge_steps as f32;
            self.shot_charge = 0;
            charge
        } else {
            1.
        };
        self.emit(host, EventKind::Hit, m as u32, strength);
    }

    pub(super) fn execute_disc_commands(&mut self, host: &mut impl Host, input: Input) {
        let clip = self.motion.clip;
        let frame = self.motion.frame;
        if self.action.script_clip != clip || frame < self.action.script_frame {
            self.action.cursor = 0;
            self.action.script_clip = clip;
        }
        self.action.script_frame = frame;
        while let Some(event) = self
            .action
            .definitions
            .get(clip)
            .and_then(|a| a.commands.get(self.action.cursor))
            .cloned()
        {
            if event.frame > frame {
                break;
            }
            self.action.cursor += 1;
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
                    if index == 2 && value == 1 && self.phase == Phase::ChargeShot {
                        let band = (self.shot_charge * 5 / self.params.charge_steps.max(1)).min(4);
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
                        && self.current_move == Some(Move::ChargeShot)
                        && self.phase != Phase::ChargeShot
                    {
                        self.disc_projectile(host);
                    }
                }
                Command::ThrowFlag(_) => {
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
                    if matches!(
                        self.current_move,
                        Some(Move::Missile | Move::SuperMissile | Move::Bomb)
                    ) {
                        self.disc_projectile(host);
                    }
                }
                Command::Interrupt => self.action.interrupt = true,
                Command::JabCombo(value) => self.action.jab_combo = value,
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
                    if opcode == 10 {
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
                            ),
                            param: (words[1] & 65535) as f32,
                            clip,
                            frame,
                            origin: self.position,
                            facing: self.facing,
                        });
                    }
                    // ftAction_80071B50: play/replace/stop a fighter sound slot.
                    // 54/55 supply the stored footstep/landing cue; world-surface
                    // substitution remains a host adaptation, using the disc cue.
                    if matches!(opcode, 17 | 54 | 55) {
                        let behavior = if opcode == 17 {
                            words[0] >> 18 & 255
                        } else {
                            0
                        };
                        if behavior <= 6 || (10..=15).contains(&behavior) {
                            self.action.sounds.push(SoundCue {
                                sound: words[1],
                                volume: (words[2] >> 8) as u8,
                                pan: words[2] as u8,
                                channel: if behavior >= 10 {
                                    behavior - 9
                                } else {
                                    behavior
                                },
                                stop: behavior >= 10,
                            });
                        }
                    }
                }
            }
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
