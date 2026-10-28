//! Fighter_WaitAnimData and the bounded subaction interpreter.
//! Reimplemented from lb/lbcommand.c::Command_Execute and ft/ftaction.c.
use crate::{Result, animation::Animations, dat::Dat};
use glam::Vec3;
use libssbm::action::{ActionDef, Command, Hitbox, TimedCommand};
use std::{collections::{BTreeMap, HashSet}, path::Path};

/// How far into the Super Scope's Start script to decode (its last event is at frame 88; every fighter's
/// script ends there).
const SCOPE_START_SCRIPT_FRAMES: f32 = 128.;

pub fn load(dir: &Path, animations: &Animations) -> Result<BTreeMap<String, ActionDef>> {
    load_for(dir, animations, &crate::fighter::SAMUS)
}
pub fn load_for(
    dir: &Path,
    animations: &Animations,
    fighter: &crate::fighter::FighterAssets<'_>,
) -> Result<BTreeMap<String, ActionDef>> {
    fighter.required()?;
    let bytes = std::fs::read(dir.join(fighter.data.file))?;
    parse_for(&Dat::parse(&bytes)?, animations, fighter.data.root, fighter.clip_aliases)
}
/// Like [`load_for`], and also keeps every duplicate motion-state script under
/// `name#0`, `name#1`, ... The plain name remains the last state, as before. Only a
/// behavior that selects a stage-specific script (`Behavior::script_key`) needs these.
pub fn load_for_variants(
    dir: &Path,
    animations: &Animations,
    fighter: &crate::fighter::FighterAssets<'_>,
) -> Result<BTreeMap<String, ActionDef>> {
    fighter.required()?;
    let bytes = std::fs::read(dir.join(fighter.data.file))?;
    parse_for_variants(&Dat::parse(&bytes)?, animations, fighter.data.root, fighter.clip_aliases)
}

pub fn parse(dat: &Dat, animations: &Animations) -> Result<BTreeMap<String, ActionDef>> {
    parse_for(dat, animations, crate::fighter::SAMUS.data.root, &[])
}
pub fn parse_for(
    dat: &Dat,
    animations: &Animations,
    root: &str,
    aliases: &[(&str, &str)],
) -> Result<BTreeMap<String, ActionDef>> {
    parse_inner(dat, animations, root, aliases, false)
}
pub fn parse_for_variants(
    dat: &Dat,
    animations: &Animations,
    root: &str,
    aliases: &[(&str, &str)],
) -> Result<BTreeMap<String, ActionDef>> {
    parse_inner(dat, animations, root, aliases, true)
}
fn parse_inner(
    dat: &Dat,
    animations: &Animations,
    root: &str,
    aliases: &[(&str, &str)],
    variants: bool,
) -> Result<BTreeMap<String, ActionDef>> {
    let root = dat.root(root)?;
    let start = dat.required_ptr(root + 0xc)?;
    let end = dat.required_ptr(root + 0x14)?;
    if end <= start || !(end - start).is_multiple_of(24) || (end - start) / 24 > 2048 {
        return Err("invalid fighter motion descriptor table".into());
    }
    let mut result = BTreeMap::new();
    // Disc motion descriptors are not unique by clip name: two motion states can play
    // one animation clip with different scripts (Pikachu's Thunder wait and struck
    // loops both use `SpecialLwLoop`). Keep the last one under the plain name, as the
    // sim always has; opt-in variant keys keep every occurrence so a behavior can
    // select the state it is in.
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    for at in (start..end).step_by(24) {
        let Some(name_at) = dat.ptr(at)? else {
            continue;
        };
        let tail = dat.data.get(name_at..).ok_or("motion name outside DAT")?;
        let size = tail
            .iter()
            .take(256)
            .position(|&b| b == 0)
            .ok_or("unterminated motion name")?;
        let name = std::str::from_utf8(&tail[..size])?;
        let Some(name) = name
            .split("_ACTION_")
            .nth(1)
            .and_then(|s| s.strip_suffix("_figatree"))
        else {
            continue;
        };
        let name = crate::animation::canonical(aliases, name);
        let Some(clip) = animations.clips.get(name.as_ref()) else {
            continue;
        };
        // The disc lists each ItemScope motion twice: the loaded scope's (ftCo_MS_ItemScopeStart..AirEnd), then
        // the empty scope's (`+ 8`), whose scripts share the clip names. The loaded scope's come first and win;
        // the sim mutes the effect cues the empty scope's scripts lack.
        if name.starts_with("ItemScope") && result.contains_key(name.as_ref()) {
            continue;
        }
        // The Super Scope's charge scripts outlast their 15-frame clip: the clip holds its last pose while
        // the script's loop spawns the ground dust (cue 0x3FF) until the charge tops out, so decode
        // them to their own end instead of the clip's.
        let horizon = if matches!(name.as_ref(), "ItemScopeStart" | "ItemScopeAirStart") {
            clip.frames.max(SCOPE_START_SCRIPT_FRAMES)
        } else {
            clip.frames
        };
        let commands = match dat.ptr(at + 12)? {
            Some(script) => script_commands(dat, script, horizon)
                .map_err(|e| format!("{name}: {e}"))?,
            None => Vec::new(),
        };
        let base = name.into_owned();
        let alternate = if variants {
            let count = seen.entry(base.clone()).or_insert(0);
            let key = format!("{base}#{count}");
            *count += 1;
            Some(key)
        } else {
            None
        };
        let action = ActionDef {
            frames: clip.frames,
            looping: dat.u32(at + 16)? & 0x40000000 != 0,
            commands,
        };
        if let Some(key) = alternate {
            result.insert(key, action.clone());
        }
        result.insert(base, action);
    }
    Ok(result)
}

fn event_words(opcode: u32) -> Result<usize> {
    // These are encoded record widths, not attack timings or embedded game data.
    Ok(match opcode {
        10 | 11 => 5,
        17 | 34 | 54 | 55 => 3,
        38 => 7,
        39 | 58 => 4,
        56 => 2,
        12..=58 => 1,
        _ => return Err(format!("unsupported fighter command {opcode}").into()),
    })
}
fn signed(value: u32, bits: u32) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

pub fn script_commands(dat: &Dat, start: usize, horizon: f32) -> Result<Vec<TimedCommand>> {
    decode_script(dat, start, horizon, false)
}
/// Item records share Command_Execute control flow, but not fighter opcodes.
/// Widths and bitfields follow itanimlist.c::it_803F22A8 and lb/types.h.
pub fn item_script_commands(dat: &Dat, start: usize, horizon: f32) -> Result<Vec<TimedCommand>> {
    decode_script(dat, start, horizon, true)
}
fn decode_script(dat: &Dat, start: usize, horizon: f32, item: bool) -> Result<Vec<TimedCommand>> {
    let mut pc = start;
    let mut frame = 0.;
    let mut loops: Vec<(usize, u32)> = Vec::new();
    let mut calls = Vec::new();
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for _ in 0..65536 {
        if frame > horizon + 1. {
            return Ok(result);
        }
        // Looping actions (Run, Wait, ...) cycle via an AsynchronousTimer that
        // wraps back to an earlier animation frame; their script is a `goto`
        // loop with no SetTimerAnimation terminator. Once the interpreter
        // returns to a (pc, frame) it has already visited, one full cycle has
        // been flattened and the remaining commands are duplicates.
        if !seen.insert((pc, frame as u32)) {
            return Ok(result);
        }
        let word = dat.u32(pc)?;
        let opcode = word >> 26;
        let value = word & 0x03ffffff;
        match opcode {
            0 => return Ok(result),
            1 => {
                frame += value as f32;
                pc += 4;
                continue;
            }
            2 => {
                frame = frame.max(value as f32);
                pc += 4;
                continue;
            }
            3 => {
                if value == 0 || loops.len() >= 16 {
                    return Err("invalid fighter command loop".into());
                }
                loops.push((pc + 4, value));
                pc += 4;
                continue;
            }
            4 => {
                let (target, left) = loops.last_mut().ok_or("fighter loop stack underflow")?;
                if *left > 1 {
                    *left -= 1;
                    pc = *target;
                } else {
                    loops.pop();
                    pc += 4;
                }
                continue;
            }
            5 => {
                if calls.len() >= 16 {
                    return Err("fighter subroutine stack overflow".into());
                }
                calls.push(pc + 8);
                pc = dat.required_ptr(pc + 4)?;
                continue;
            }
            6 => {
                pc = calls.pop().ok_or("fighter subroutine stack underflow")?;
                continue;
            }
            7 => {
                pc = dat.required_ptr(pc + 4)?;
                continue;
            }
            // Wait until the animation rewinds: the next cycle replays the script.
            8 => return Ok(result),
            9 => {
                pc += 4;
                continue;
            }
            _ => (),
        }
        let count = if item {
            match opcode {
                10 => 5,
                11 => 6,
                16 => 3,
                12..=25 => 1,
                _ => return Err(format!("unsupported item command {opcode}").into()),
            }
        } else {
            event_words(opcode)?
        };
        let words = (0..count)
            .map(|i| dat.u32(pc + i * 4))
            .collect::<Result<Vec<_>>>()?;
        let command = if item {
            match opcode {
                11 => Command::Hitbox(Hitbox {
                    id: word >> 23 & 7,
                    group: word >> 20 & 7,
                    bone: (word >> 13 & 127) as usize,
                    common_bone: false,
                    damage: (word & 8191) as f32,
                    radius: (words[1] >> 16) as f32 / 256.,
                    offset: Vec3::new(
                        signed(words[1] & 65535, 16) as f32,
                        signed(words[2] >> 16, 16) as f32,
                        signed(words[2] & 65535, 16) as f32,
                    ) / 256.,
                    angle: words[3] >> 23,
                    growth: words[3] >> 14 & 511,
                    fixed_knockback: words[3] >> 5 & 511,
                    base_knockback: words[4] >> 23,
                    element: words[4] >> 18 & 31,
                    sfx_kind: words[4] >> 2 & 15,
                    sfx_severity: words[4] >> 6 & 7,
                    ground: true,
                    air: true,
                }),
                12 => Command::HitboxDamage(word >> 23 & 7, (word & 8191) as f32),
                13 => Command::HitboxRadius(word >> 23 & 7, (word & 0x7fffff) as f32 / 256.),
                14 => Command::ClearHitbox(value),
                15 => Command::ClearHitboxes,
                // Normalize sound opcode only; item GFX has its own bone encoding.
                16 => Command::Raw { opcode: 17, words },
                _ => Command::Raw { opcode, words },
            }
        } else {
            match opcode {
                11 => Command::Hitbox(Hitbox {
                    id: word >> 23 & 7,
                    group: word >> 20 & 7,
                    bone: (word >> 11 & 255) as usize,
                    common_bone: word & 0x400 != 0,
                    damage: (word & 1023) as f32,
                    radius: (words[1] >> 16) as f32 / 256.,
                    // Model coordinates follow the source joint; the host owns axis conversion.
                    offset: Vec3::new(
                        signed(words[1] & 65535, 16) as f32,
                        signed(words[2] >> 16, 16) as f32,
                        signed(words[2] & 65535, 16) as f32,
                    ) / 256.,
                    angle: words[3] >> 23,
                    growth: words[3] >> 14 & 511,
                    fixed_knockback: words[3] >> 5 & 511,
                    base_knockback: words[4] >> 23,
                    element: words[4] >> 18 & 31,
                    sfx_kind: words[4] >> 2 & 31,
                    sfx_severity: words[4] >> 7 & 7,
                    ground: words[4] & 2 != 0,
                    air: words[4] & 1 != 0,
                }),
                12 => Command::HitboxDamage(word >> 23 & 7, (word & 0x7fffff) as f32),
                13 => Command::HitboxRadius(word >> 23 & 7, (word & 0x7fffff) as f32 / 256.),
                15 => Command::ClearHitbox(value),
                16 => Command::ClearHitboxes,
                19 => Command::Variable(word >> 24 & 3, word & 0xffffff),
                20 => Command::ThrowFlag(value),
                21 => Command::ThrowFlag(3),
                22 => Command::ThrowFlag(4),
                23 => Command::Interrupt,
                24 => Command::ThrowFlag(0),
                29 => Command::JabCombo(value == 0),
                30 => Command::JabRapid(value != 0),
                31 => Command::Visibility {
                    group: signed(word >> 19 & 127, 7),
                    choice: signed(word & 0x7ffff, 19),
                },
                56 => Command::SmashCharge {
                    max_frames: word >> 16 & 1023,
                    damage_per_frame: (word & 65535) as f32 / 256.,
                },
                _ => Command::Raw { opcode, words },
            }
        };
        result.push(TimedCommand { frame, command });
        pc += count * 4;
    }
    Err(format!(
        "fighter command execution budget exceeded at {start:#x}"
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    fn bytes(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }
    #[test]
    fn waits_loops_calls_and_signed_hitbox_offsets() -> Result<()> {
        let data = bytes(&[
            (2 << 26) | 3,
            (11 << 26) | (2 << 23) | (1 << 20) | (5 << 11) | 9,
            (1024 << 16) | 0xff00,
            (512 << 16) | 0xfe00,
            (45 << 23) | (100 << 14),
            (20 << 23) | 3,
            (3 << 26) | 2,
            (1 << 26) | 2,
            (19 << 26) | 1,
            4 << 26,
            5 << 26,
            52,
            0,
            (1 << 26) | 3,
            16 << 26,
            6 << 26,
        ]);
        let dat = Dat {
            data: &data,
            symbols: BTreeMap::new(),
            relocations: HashSet::from([44]),
        };
        let events = script_commands(&dat, 0, 30.)?;
        let Command::Hitbox(h) = &events[0].command else {
            panic!("Missing synthetic hitbox")
        };
        assert_eq!(events[0].frame, 3.);
        assert_eq!(
            (h.id, h.group, h.bone, h.damage, h.radius),
            (2, 1, 5, 9., 4.)
        );
        assert_eq!(h.offset, Vec3::new(-1., 2., -2.));
        assert_eq!(
            events.iter().map(|e| e.frame).collect::<Vec<_>>(),
            [3., 5., 7., 10.]
        );
        assert!(matches!(events[3].command, Command::ClearHitboxes));
        Ok(())
    }
    #[test]
    fn corrupt_control_flow_is_bounded() {
        // A `goto` to itself never advances frame; the cycle detector truncates
        // it rather than hanging or exhausting the interpreter budget.
        let data = bytes(&[7 << 26, 0]);
        let dat = Dat {
            data: &data,
            symbols: BTreeMap::new(),
            relocations: HashSet::from([4]),
        };
        assert!(script_commands(&dat, 0, 30.).is_ok());
        // An undersized hitbox record runs off the end of the DAT.
        let data = bytes(&[11 << 26]);
        let dat = Dat {
            data: &data,
            symbols: BTreeMap::new(),
            relocations: HashSet::new(),
        };
        assert!(script_commands(&dat, 0, 30.).is_err());
    }
}
