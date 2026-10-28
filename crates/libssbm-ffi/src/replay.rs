//! Multiplayer: a log of what the host did to a fighter's sim, and a replay of it on another machine.
//!
//! The machine that owns a player's fighter runs the real sim. With recording on, every call that
//! changes the sim (the pad and time of each step, placements, hurts, grabs, tuning and so on) is
//! appended to a byte log, followed after each step by where the sim ended up. The log goes to the
//! other machines, which replay it on a shadow sim of their own: the same fixed-rate code on the same
//! calls draws the same fighter, plays the same sounds and spawns the same effects. The shadow sim
//! has no enemies of its own, so the recorded position pulls it back when the two drift apart.
//! Nothing here touches the world: a replayed step calls the shadow host's own collision callback.
use super::*;

/// Sim units a shadow may drift from the recorded position before it is pulled back.
const DRIFT: f32 = 1.5;
/// The most bytes a single replay accepts.
const MAX_REPLAY: u32 = 1 << 16;

#[derive(Clone, Copy)]
pub(crate) enum Op {
    Step { pad: Pad, dt: f64 },
    Place { v: V3, kind: u32 },
    Facing(V3),
    JumpHeight(f32),
    Tuning(Tuning),
    HoldKnockdown,
    Hurt(f32),
    Interrupt,
    LockTarget { target: V3, valid: u32 },
    HostFlags(u32),
    Connect,
    Anchor { position: V3, grounded: u32 },
    SetGrabbed(u32),
    /// Yoshi's egg-roll armor ran out; end the roll on the next tick.
    BreakRoll,
    /// The Morph Ball hit an enemy; bounce it back on the next tick.
    BonkRoll,
    LaunchPad { velocity: V3, gravity: V3 },
    Rebirth(V3),
    Arrival,
    Sync { position: V3, grounded: u32 },
    /// The Super Scope given to (1) or taken from (0) the fighter, with its refill rate.
    Scope { on: u32, refill: f32 },
    /// Wax Quail: scale the horizontal speed of the jump in progress.
    BoostJump(f32),
    /// H3AD-5T v2: enable (1) / disable (0) the stomp.
    SetStomp(u32),
    /// RoR2 attack speed: the move-playback multiplier.
    SetAttackSpeed(f32),
    /// Link's hookshot took hold: restore the fighter's jumps without landing.
    ResetJumps,
}

#[derive(Default)]
pub(crate) struct NetLog {
    /// The asset cache the sim was made from (a rebirth or arrival reads its tuning again).
    pub dir: std::path::PathBuf,
    rec: Option<Vec<u8>>,
    facing: Option<[u32; 3]>,
    jump: Option<u32>,
    tuning: Option<[u32; 5]>,
    lock: Option<[u32; 4]>,
    flags: Option<u32>,
    scope: Option<[u32; 2]>,
    stomp: Option<u32>,
    attack: Option<u32>,
    steps: u32,
}

fn bits3(v: V3) -> [u32; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

impl NetLog {
    pub fn with_dir(dir: std::path::PathBuf) -> Self {
        Self { dir, ..Self::default() }
    }
    pub fn recording(&self) -> bool {
        self.rec.is_some()
    }
    /// Appends `op` while recording. Settings the host repeats every step (facing, tuning, the lock-on
    /// target, jump height, host flags) are logged only when they change.
    pub fn log(&mut self, op: Op) {
        let Some(buf) = self.rec.as_mut() else { return };
        if buf.len() > 4 * MAX_REPLAY as usize {
            // Nobody is taking the log: stop it growing for ever.
            buf.clear();
        }
        match op {
            Op::Step { .. } => {
                // Settings are logged when they change; a shadow that joined late has missed them,
                // so every two seconds they are logged again.
                self.steps = self.steps.wrapping_add(1);
                if self.steps % 120 == 0 {
                    self.facing = None;
                    self.jump = None;
                    self.tuning = None;
                    self.lock = None;
                    self.flags = None;
                    self.scope = None;
                    self.stomp = None;
                    self.attack = None;
                }
            }
            Op::Facing(v) => {
                if self.facing == Some(bits3(v)) {
                    return;
                }
                self.facing = Some(bits3(v));
            }
            Op::JumpHeight(h) => {
                if self.jump == Some(h.to_bits()) {
                    return;
                }
                self.jump = Some(h.to_bits());
            }
            Op::Tuning(t) => {
                let key = [t.speed_scale.to_bits(), t.jump_scale.to_bits(), t.extra_jumps, t.hold_charge, t.smash_max_frames];
                if self.tuning == Some(key) {
                    return;
                }
                self.tuning = Some(key);
            }
            Op::LockTarget { target, valid } => {
                let b = bits3(target);
                let key = [b[0], b[1], b[2], valid];
                if self.lock == Some(key) {
                    return;
                }
                self.lock = Some(key);
            }
            Op::HostFlags(f) => {
                if self.flags == Some(f) {
                    return;
                }
                self.flags = Some(f);
            }
            Op::Scope { on, refill } => {
                if self.scope == Some([on, refill.to_bits()]) {
                    return;
                }
                self.scope = Some([on, refill.to_bits()]);
            }
            Op::SetStomp(on) => {
                if self.stomp == Some(on) {
                    return;
                }
                self.stomp = Some(on);
            }
            Op::SetAttackSpeed(speed) => {
                if self.attack == Some(speed.to_bits()) {
                    return;
                }
                self.attack = Some(speed.to_bits());
            }
            _ => {}
        }
        encode(buf, op);
    }
}

fn f32w(b: &mut Vec<u8>, v: f32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn u32w(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn v3w(b: &mut Vec<u8>, v: V3) {
    f32w(b, v.x);
    f32w(b, v.y);
    f32w(b, v.z);
}
fn encode(b: &mut Vec<u8>, op: Op) {
    match op {
        Op::Step { pad, dt } => {
            b.push(1);
            v3w(b, pad.movement);
            v3w(b, pad.aim);
            u32w(b, pad.buttons);
            u32w(b, pad.allowed);
            b.extend_from_slice(&dt.to_le_bytes());
        }
        Op::Place { v, kind } => {
            b.push(2);
            v3w(b, v);
            u32w(b, kind);
        }
        Op::Facing(v) => {
            b.push(3);
            v3w(b, v);
        }
        Op::JumpHeight(h) => {
            b.push(4);
            f32w(b, h);
        }
        Op::Tuning(t) => {
            b.push(5);
            f32w(b, t.speed_scale);
            f32w(b, t.jump_scale);
            u32w(b, t.extra_jumps);
            u32w(b, t.hold_charge);
            u32w(b, t.smash_max_frames);
        }
        Op::HoldKnockdown => b.push(6),
        Op::Hurt(s) => {
            b.push(7);
            f32w(b, s);
        }
        Op::Interrupt => b.push(8),
        Op::LockTarget { target, valid } => {
            b.push(9);
            v3w(b, target);
            u32w(b, valid);
        }
        Op::HostFlags(f) => {
            b.push(10);
            u32w(b, f);
        }
        Op::Connect => b.push(11),
        Op::Anchor { position, grounded } => {
            b.push(12);
            v3w(b, position);
            u32w(b, grounded);
        }
        Op::SetGrabbed(held) => {
            b.push(13);
            u32w(b, held);
        }
        Op::BreakRoll => b.push(17),
        Op::BonkRoll => b.push(20),
        Op::LaunchPad { velocity, gravity } => {
            b.push(14);
            v3w(b, velocity);
            v3w(b, gravity);
        }
        Op::Rebirth(v) => {
            b.push(15);
            v3w(b, v);
        }
        Op::Arrival => b.push(16),
        Op::Sync { position, grounded } => {
            b.push(18);
            v3w(b, position);
            u32w(b, grounded);
        }
        Op::Scope { on, refill } => {
            b.push(19);
            u32w(b, on);
            f32w(b, refill);
        }
        Op::BoostJump(f) => {
            b.push(21);
            f32w(b, f);
        }
        Op::SetStomp(on) => {
            b.push(22);
            u32w(b, on);
        }
        Op::SetAttackSpeed(speed) => {
            b.push(23);
            f32w(b, speed);
        }
        Op::ResetJumps => b.push(24),
    }
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Some(head)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_bits(self.u32()?))
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn v3(&mut self) -> Option<V3> {
        Some(V3 { x: self.f32()?, y: self.f32()?, z: self.f32()? })
    }
    fn op(&mut self) -> Option<Op> {
        let tag = self.take(1)?[0];
        Some(match tag {
            1 => Op::Step {
                pad: Pad { movement: self.v3()?, aim: self.v3()?, buttons: self.u32()?, allowed: self.u32()? },
                dt: self.f64()?,
            },
            2 => Op::Place { v: self.v3()?, kind: self.u32()? },
            3 => Op::Facing(self.v3()?),
            4 => Op::JumpHeight(self.f32()?),
            5 => Op::Tuning(Tuning {
                speed_scale: self.f32()?,
                jump_scale: self.f32()?,
                extra_jumps: self.u32()?,
                hold_charge: self.u32()?,
                smash_max_frames: self.u32()?,
            }),
            6 => Op::HoldKnockdown,
            7 => Op::Hurt(self.f32()?),
            8 => Op::Interrupt,
            9 => Op::LockTarget { target: self.v3()?, valid: self.u32()? },
            10 => Op::HostFlags(self.u32()?),
            11 => Op::Connect,
            12 => Op::Anchor { position: self.v3()?, grounded: self.u32()? },
            13 => Op::SetGrabbed(self.u32()?),
            14 => Op::LaunchPad { velocity: self.v3()?, gravity: self.v3()? },
            15 => Op::Rebirth(self.v3()?),
            16 => Op::Arrival,
            17 => Op::BreakRoll,
            20 => Op::BonkRoll,
            18 => Op::Sync { position: self.v3()?, grounded: self.u32()? },
            19 => Op::Scope { on: self.u32()?, refill: self.f32()? },
            21 => Op::BoostJump(self.f32()?),
            22 => Op::SetStomp(self.u32()?),
            23 => Op::SetAttackSpeed(self.f32()?),
            24 => Op::ResetJumps,
            _ => return None,
        })
    }
}

impl Handle {
    /// After a recorded step: where the sim ended up, for shadows to be pulled back to.
    pub(crate) fn log_sync(&mut self) {
        if self.net.recording() {
            self.net.log(Op::Sync { position: self.sim.position.into(), grounded: self.sim.grounded as u32 });
        }
    }
}

/// Starts (1) or stops (0) recording the calls made on this sim. Starting clears the log.
/// Returns 1, or 0 for a null handle (the capability query).
/// # Safety
/// Unique live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_record(handle: *mut Handle, on: u32) -> u32 {
    let Some(h) = (unsafe { handle.as_mut() }) else { return 0 };
    let dir = std::mem::take(&mut h.net.dir);
    h.net = NetLog::with_dir(dir);
    if on != 0 {
        h.net.rec = Some(Vec::new());
    }
    1
}

/// Moves the recorded log into `out`. Returns its length; with a null `out` or too small a
/// `cap` it returns the length needed and keeps the log. An empty log returns 0.
/// # Safety
/// Unique live handle; `out` is null or holds `cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_ops_take(handle: *mut Handle, out: *mut u8, cap: u32) -> u32 {
    let Some(h) = (unsafe { handle.as_mut() }) else { return 0 };
    let Some(buf) = h.net.rec.as_mut() else { return 0 };
    let len = buf.len() as u32;
    if len == 0 || out.is_null() || len > cap {
        return len;
    }
    unsafe { std::ptr::copy_nonoverlapping(buf.as_ptr(), out, buf.len()) };
    buf.clear();
    len
}

/// Replays a recorded log on this (shadow) sim, calling `host` for the collision and events of its
/// steps. Returns the number of operations applied, or `u32::MAX` when the bytes are not a log.
/// # Safety
/// Unique live handle, a valid host table, and `len` readable bytes at `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ssbm_sim_replay(
    handle: *mut Handle,
    host: *const HostTable,
    bytes: *const u8,
    len: u32,
) -> u32 {
    if handle.is_null() || host.is_null() || bytes.is_null() || len > MAX_REPLAY {
        return u32::MAX;
    }
    let data = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    let mut reader = Reader(data);
    let mut applied = 0;
    // Each step clears the effect cues of the one before, but the host reads them once after the whole
    // replay: keep every step's.
    let mut effects = Vec::new();
    while !reader.0.is_empty() {
        let Some(op) = reader.op() else { return u32::MAX };
        unsafe {
            match op {
                Op::Step { pad, dt } => {
                    ssbm_sim_step(handle, host, &pad, dt);
                    effects.append(&mut (&mut *handle).sim.action.effects);
                }
                Op::Place { v, kind } => ssbm_sim_place(handle, v, kind),
                Op::Facing(v) => ssbm_sim_facing(handle, v),
                Op::JumpHeight(h) => ssbm_sim_jump_height(handle, h),
                Op::Tuning(t) => {
                    ssbm_sim_tuning(handle, t);
                }
                Op::HoldKnockdown => ssbm_sim_hold_knockdown(handle),
                Op::Hurt(s) => {
                    ssbm_sim_hurt_scaled(handle, s);
                }
                Op::Interrupt => ssbm_sim_interrupt(handle),
                Op::LockTarget { target, valid } => ssbm_sim_lock_target(handle, target, valid),
                Op::HostFlags(f) => ssbm_sim_set_host_flags(handle, f),
                Op::Connect => ssbm_sim_connect(handle),
                Op::Anchor { position, grounded } => ssbm_sim_anchor(handle, position, grounded),
                Op::SetGrabbed(held) => ssbm_sim_set_grabbed(handle, held),
                Op::BreakRoll => { ssbm_sim_break_roll(handle); }
                Op::BonkRoll => { ssbm_sim_bonk_roll(handle); }
                Op::LaunchPad { velocity, gravity } => ssbm_sim_launch_pad(handle, velocity, gravity),
                Op::Rebirth(dest) => {
                    if let Ok(dir) = CString::new((&*handle).net.dir.to_string_lossy().as_bytes()) {
                        ssbm_sim_begin_rebirth(handle, dir.as_ptr(), dest);
                    }
                }
                Op::Arrival => {
                    if let Ok(dir) = CString::new((&*handle).net.dir.to_string_lossy().as_bytes()) {
                        ssbm_sim_begin_arrival(handle, dir.as_ptr());
                    }
                }
                Op::Scope { on, refill } => {
                    ssbm_sim_scope(handle, on, refill);
                }
                Op::BoostJump(f) => {
                    ssbm_sim_boost_jump(handle, f);
                }
                Op::SetStomp(on) => {
                    ssbm_sim_set_stomp(handle, on);
                }
                Op::SetAttackSpeed(speed) => {
                    ssbm_sim_set_attack_speed(handle, speed);
                }
                Op::ResetJumps => {
                    ssbm_sim_reset_jumps(handle);
                }
                Op::Sync { position, grounded } => {
                    let h = &mut *handle;
                    let to: Vec3 = position.into();
                    if !h.poisoned
                        && to.is_finite()
                        && h.sim.entry.is_none()
                        && h.sim.rebirth.is_none()
                        && (h.sim.position - to).length() > DRIFT
                    {
                        h.sim.nudge(to, grounded != 0);
                    }
                }
            }
        }
        applied += 1;
    }
    if !effects.is_empty() {
        unsafe { (&mut *handle).sim.action.effects = effects };
    }
    applied
}
