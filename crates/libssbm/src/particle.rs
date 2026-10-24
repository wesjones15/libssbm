//! Engine-independent HSD particle runtime. Behavioral specifications:
//! generator.c::hsd_8039DAD4/EE24, particle.c::hsd_8039930C and psstructs.h.
//! Fixed 60-Hz clocks, bounded commands/children, deterministic local RNG.
use glam::{Mat4, Quat, Vec3, Vec4};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct Emitter {
    pub id: u32,
    pub kind: u32,
    pub shape: u16,
    pub texture: u16,
    pub generator_life: u16,
    pub life: u16,
    pub gravity: f32,
    pub friction: f32,
    pub velocity: Vec3,
    pub radius: f32,
    pub angle: f32,
    pub random: f32,
    pub size: f32,
    pub params: Vec3,
    pub commands: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Particle {
    pub emitter: u32,
    pub texture: u16,
    pub frame: u8,
    pub palette: u8,
    pub kind: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub size: f32,
    pub rotation: f32,
    /// Trail length from 0xE8; `kind` bit 22 marks it on. The host draws a trail only
    /// when it implements one, but parsing this keeps the rest of the particle alive.
    pub trail: f32,
    pub color: Vec4,
    pub environment: Vec4,
    pub transform: Mat4,
    pub attachment: u64,
    life: u32,
    gravity: f32,
    friction: f32,
    pc: usize,
    wait: u32,
    loop_at: usize,
    loop_count: u8,
    mark: usize,
    size_target: f32,
    size_time: u32,
    rotation_target: f32,
    rotation_time: u32,
    color_target: Vec4,
    color_time: u32,
    env_target: Vec4,
    env_time: u32,
}
struct Generator {
    id: u32,
    transform: Mat4,
    attachment: u64,
    life: u32,
    count: f32,
    kind: Option<u32>,
}
enum Child {
    Particle(u32, Vec3, Vec3, bool),
    Generator(u32, Vec3, Option<u32>),
}
#[derive(Default)]
pub struct System {
    pub definitions: BTreeMap<u32, Emitter>,
    pub particles: Vec<Particle>,
    pub errors: Vec<String>,
    generators: Vec<Generator>,
    random: u32,
}
impl System {
    pub fn spawn(&mut self, id: u32, transform: Mat4, attachment: u64) -> Result<(), String> {
        self.generator(id, transform, attachment, None)
    }
    fn generator(
        &mut self,
        id: u32,
        transform: Mat4,
        attachment: u64,
        kind: Option<u32>,
    ) -> Result<(), String> {
        let d = self
            .definitions
            .get(&id)
            .ok_or_else(|| format!("missing emitter {id}"))?;
        if self.generators.len() >= 256 {
            return Err("particle generator budget exceeded".into());
        }
        self.generators.push(Generator {
            id,
            transform,
            attachment,
            life: d.generator_life as u32,
            count: if d.kind & 0x100 != 0 {
                if d.random < 0. {
                    if 1. + d.random > f32::EPSILON { 1. } else { 0. }
                } else {
                    0.9999999
                }
            } else {
                0.
            },
            kind,
        });
        Ok(())
    }
    pub fn attach(&mut self, key: u64, transform: Mat4) {
        for g in &mut self.generators {
            if g.attachment == key {
                g.transform = transform;
            }
        }
        for p in &mut self.particles {
            if p.attachment == key {
                p.transform = transform;
            }
        }
    }
    pub fn stop(&mut self, key: u64) {
        self.generators.retain(|g| g.attachment != key);
        self.particles.retain(|p| p.attachment != key);
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.generators.clear();
    }
    pub fn active(&self) -> bool {
        !self.particles.is_empty() || !self.generators.is_empty()
    }
    pub fn tick(&mut self) {
        let mut generators = std::mem::take(&mut self.generators);
        for mut g in generators.drain(..) {
            let Some(d) = self.definitions.get(&g.id).cloned() else {
                continue;
            };
            g.count += if d.random < 0. {
                -d.random
            } else {
                d.random * rand(&mut self.random)
            };
            let count = (g.count as usize).min(128);
            g.count -= count as f32;
            for _ in 0..count {
                if self.particles.len() >= 4096 {
                    self.errors.push("particle count budget exceeded".into());
                    break;
                }
                let (position, velocity) = emission(&d, &mut self.random);
                let mut p = Particle::new(&d, g.transform, g.attachment, position, velocity);
                if let Some(kind) = g.kind {
                    p.kind = (p.kind & 0xf1ffffff) | kind;
                }
                self.particles.push(p);
            }
            if g.life > 0 {
                g.life -= 1;
                if g.life == 0 {
                    continue;
                }
            }
            self.generators.push(g);
        }
        let mut particles = std::mem::take(&mut self.particles);
        let mut children = Vec::new();
        for mut p in particles.drain(..) {
            let Some(d) = self.definitions.get(&p.emitter) else {
                continue;
            };
            match p.tick(d, &mut self.random) {
                Ok(events) => {
                    for child in events {
                        children.push((child, p.transform, p.attachment));
                    }
                }
                Err(e) => {
                    self.errors
                        .push(format!("particle {} pc {}: {e}", p.emitter, p.pc));
                    continue;
                }
            }
            if p.life > 0 {
                self.particles.push(p);
            }
        }
        for (child, transform, attachment) in children {
            match child {
                Child::Generator(id, position, kind) => {
                    if let Err(e) = self.generator(
                        id,
                        transform * Mat4::from_translation(position),
                        attachment,
                        kind,
                    ) {
                        self.errors.push(e);
                    }
                }
                Child::Particle(id, position, velocity, inherit) => {
                    if let Some(d) = self.definitions.get(&id) {
                        if self.particles.len() < 4096 {
                            self.particles.push(Particle::new(
                                d,
                                transform,
                                attachment,
                                position,
                                if inherit { velocity } else { d.velocity },
                            ));
                        }
                    } else {
                        self.errors.push(format!("missing child particle {id}"));
                    }
                }
            }
        }
    }
}
fn rand(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state >> 8) as f32 / 16777216.
}
fn emission(d: &Emitter, rng: &mut u32) -> (Vec3, Vec3) {
    let azimuth = rand(rng) * std::f32::consts::TAU;
    let radius = if d.radius < 0. {
        -d.radius
    } else {
        d.radius * rand(rng).sqrt()
    };
    let speed = d.velocity.length();
    let orient = if speed > 0.00001 {
        Quat::from_rotation_arc(Vec3::Z, d.velocity / speed)
    } else {
        Quat::IDENTITY
    };
    let (pos, vel) = match d.shape & 15 {
        1 => (d.params * rand(rng), d.velocity),
        8 => {
            let polar = std::f32::consts::FRAC_PI_2 * rand(rng).sqrt();
            let polar = if rand(rng) < 0.5 {
                std::f32::consts::PI - polar
            } else {
                polar
            };
            let dir = Vec3::new(
                polar.sin() * azimuth.cos(),
                polar.sin() * azimuth.sin(),
                polar.cos(),
            );
            (dir * radius, dir * speed)
        }
        _ => {
            let pos = Vec3::new(azimuth.cos() * radius, azimuth.sin() * radius, 0.);
            let angle = d.angle.abs() * if d.angle < 0. { radius } else { rand(rng) };
            let vel = Vec3::new(
                angle.sin() * azimuth.cos(),
                angle.sin() * azimuth.sin(),
                angle.cos(),
            ) * speed;
            (pos, vel)
        }
    };
    (
        orient * pos,
        if d.shape & 15 == 1 { vel } else { orient * vel },
    )
}
struct Reader<'a> {
    bytes: &'a [u8],
    pc: usize,
}
impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self
            .bytes
            .get(self.pc)
            .ok_or("truncated particle bytecode")?;
        self.pc += 1;
        Ok(b)
    }
    fn word(&mut self) -> Result<u32, String> {
        Ok((self.byte()? as u32) << 8 | self.byte()? as u32)
    }
    fn count(&mut self) -> Result<u32, String> {
        let n = self.byte()? as u32;
        Ok(if n & 128 != 0 {
            ((n & 127) << 8) | self.byte()? as u32
        } else {
            n
        })
    }
    fn float(&mut self) -> Result<f32, String> {
        let v = f32::from_be_bytes([self.byte()?, self.byte()?, self.byte()?, self.byte()?]);
        if !v.is_finite() {
            return Err("nonfinite particle parameter".into());
        }
        Ok(v)
    }
    fn vector(&mut self) -> Result<Vec3, String> {
        Ok(Vec3::new(self.float()?, self.float()?, self.float()?))
    }
}
impl Particle {
    fn new(d: &Emitter, transform: Mat4, attachment: u64, position: Vec3, velocity: Vec3) -> Self {
        Self {
            emitter: d.id,
            texture: d.texture,
            frame: 0,
            palette: 255,
            kind: d.kind,
            position,
            velocity,
            size: d.size,
            rotation: 0.,
            trail: 0.,
            color: Vec4::ONE,
            environment: Vec4::ZERO,
            transform,
            attachment,
            life: d.life as u32 + 1,
            gravity: d.gravity,
            friction: d.friction,
            pc: 0,
            wait: 1,
            loop_at: 0,
            loop_count: 0,
            mark: 0,
            size_target: d.size,
            size_time: 0,
            rotation_target: 0.,
            rotation_time: 0,
            color_target: Vec4::ONE,
            color_time: 0,
            env_target: Vec4::ZERO,
            env_time: 0,
        }
    }
    fn tick(&mut self, d: &Emitter, rng: &mut u32) -> Result<Vec<Child>, String> {
        if self.size_time > 0 {
            self.size += (self.size_target - self.size) / self.size_time as f32;
            self.size_time -= 1;
        }
        if self.rotation_time > 0 {
            self.rotation += (self.rotation_target - self.rotation) / self.rotation_time as f32;
            self.rotation_time -= 1;
        }
        if self.color_time > 0 {
            self.color += (self.color_target - self.color) / self.color_time as f32;
            self.color_time -= 1;
        }
        if self.env_time > 0 {
            self.environment += (self.env_target - self.environment) / self.env_time as f32;
            self.env_time -= 1;
        }
        let mut children = Vec::new();
        if self.wait > 0 {
            self.wait -= 1;
            if self.wait == 0 {
                let mut r = Reader {
                    bytes: &d.commands,
                    pc: self.pc,
                };
                let mut terminated = false;
                for _ in 0..1024 {
                    let op = r.byte()?;
                    if op < 0x80 {
                        let mut wait = (op & 31) as u32;
                        if op & 32 != 0 {
                            wait = (wait << 8) | r.byte()? as u32;
                        }
                        if op & 64 != 0 {
                            self.frame = r.byte()?;
                            self.kind |= 1 << 10;
                        }
                        if wait > 0 {
                            self.wait = wait;
                            terminated = true;
                            break;
                        }
                        continue;
                    }
                    let cls = if op < 0xa0 {
                        op & 0xf8
                    } else if op & 0xe0 == 0xc0 {
                        op & 0xf0
                    } else {
                        op
                    };
                    match cls {
                        0x80 | 0x88 | 0x90 | 0x98 => {
                            let target = if cls < 0x90 {
                                &mut self.position
                            } else {
                                &mut self.velocity
                            };
                            for axis in 0..3 {
                                if op & (1 << axis) != 0 {
                                    let v = r.float()?;
                                    if cls & 8 != 0 {
                                        target[axis] += v
                                    } else {
                                        target[axis] = v
                                    }
                                }
                            }
                        }
                        0xa0 | 0xac => {
                            self.size_time = r.count()?;
                            self.size_target = r.float()?;
                            if cls == 0xac {
                                self.size_target += r.float()? * rand(rng);
                            }
                            if self.size_time == 0 {
                                self.size = self.size_target;
                            }
                        }
                        0xa1 => self.kind &= !(1 << 10),
                        0xa2 => self.gravity = r.float()?,
                        0xa3 => self.friction = r.float()?,
                        0xa4 | 0xb9 => children.push(Child::Particle(
                            r.word()?,
                            self.position,
                            self.velocity,
                            cls == 0xb9,
                        )),
                        0xa5 | 0xef | 0xf0 => {
                            let id = r.word()?;
                            let kind = if cls == 0xa5 {
                                None
                            } else {
                                Some(((r.byte()? & 7) as u32) << 25)
                            };
                            children.push(Child::Generator(id, self.position, kind));
                        }
                        0xa6 => {
                            self.life = r.word()?;
                            self.life += (r.word()? as f32 * rand(rng)) as u32;
                        }
                        0xa7 => {
                            if r.byte()? as f32 >= 100. * rand(rng) {
                                self.life = 1;
                                terminated = true;
                                break;
                            }
                        }
                        0xa8 => {
                            let range = r.vector()?;
                            self.position += range
                                * Vec3::new(
                                    2. * rand(rng) - 1.,
                                    2. * rand(rng) - 1.,
                                    2. * rand(rng) - 1.,
                                );
                        }
                        0xab => self.velocity *= r.float()?,
                        0xad => self.kind |= 1 << 7,
                        0xae..=0xb1 => {
                            self.kind &= !0x60;
                            self.kind |= match cls {
                                0xaf => 0x20,
                                0xb0 => 0x40,
                                0xb1 => 0x60,
                                _ => 0,
                            };
                        }
                        0xb2 => {
                            self.position = self.transform.transform_point3(self.position);
                            self.velocity = self.transform.transform_vector3(self.velocity);
                            self.transform = Mat4::IDENTITY;
                            self.attachment = 0;
                        }
                        0xb3 => {
                            r.count()?;
                            r.byte()?;
                            r.byte()?;
                            r.byte()?;
                        }
                        0xb4 => self.kind |= 1 << 9,
                        0xb5 => self.kind &= !(1 << 9),
                        0xb6 => {
                            self.rotation_time = r.count()?;
                            self.rotation_target += r.float()?;
                            if self.rotation_time == 0 {
                                self.rotation = self.rotation_target;
                            }
                        }
                        // particle.c: random source texture pose, not rotation.
                        0xbc => {
                            let base = r.byte()?;
                            let range = r.byte()?;
                            self.frame = (base as f32 + range as f32 * rand(rng)) as u8;
                        }
                        0xbd => {
                            let speed = r.float()? + r.float()? * rand(rng);
                            self.velocity = self.velocity.normalize_or_zero() * speed;
                        }
                        0xbe => self.velocity *= r.vector()?,
                        0xc0 | 0xd0 => {
                            let count = r.count()?;
                            let (color, target, time) = if cls == 0xc0 {
                                (
                                    &mut self.color,
                                    &mut self.color_target,
                                    &mut self.color_time,
                                )
                            } else {
                                (
                                    &mut self.environment,
                                    &mut self.env_target,
                                    &mut self.env_time,
                                )
                            };
                            *time = count;
                            *target = *color;
                            for axis in 0..4 {
                                if op & (1 << axis) != 0 {
                                    target[axis] = r.byte()? as f32 / 255.;
                                }
                            }
                            if count == 0 {
                                *color = *target;
                            }
                        }
                        0xe2 => self.kind |= 1 << 3,
                        0xe3 => self.palette = r.byte()?,
                        0xe4 | 0xe5 => {
                            let mode = r.byte()? & 3;
                            let flag = 1 << if cls == 0xe4 { 18 } else { 19 };
                            match mode {
                                0 => self.kind &= !flag,
                                1 => self.kind |= flag,
                                2 => self.kind ^= flag,
                                _ => {
                                    if rand(rng) < 0.5 {
                                        self.kind &= !flag
                                    } else {
                                        self.kind |= flag
                                    }
                                }
                            }
                        }
                        0xe6 => self.kind |= 1 << 21,
                        0xe7 => self.kind &= !(1 << 21),
                        0xe8 => {
                            // particle.c: Trail control. A negative value disables the
                            // trail; a non-negative one enables it with that length.
                            let trail = r.float()?;
                            if trail < 0. {
                                self.kind &= !(1 << 22);
                            } else {
                                self.kind |= 1 << 22;
                                self.trail = trail;
                            }
                        }
                        0xed => {
                            let base = r.float()?;
                            let range = r.float()?;
                            let timing = r.byte()? as u32;
                            let random = rand(rng);
                            let step = if timing > 0 {
                                (random * (timing + 1) as f32).floor() / timing as f32
                            } else {
                                random
                            };
                            let angle = base + range * step;
                            self.rotation += angle;
                            self.rotation_target += angle;
                        }
                        0xfa => {
                            self.loop_count = r.byte()?;
                            self.loop_at = r.pc;
                        }
                        0xfb => {
                            self.loop_count = self.loop_count.wrapping_sub(1);
                            if self.loop_count != 0 {
                                r.pc = self.loop_at;
                            }
                        }
                        0xfc => self.mark = r.pc,
                        0xfd => r.pc = self.mark,
                        0xfe | 0xff => {
                            self.life = 1;
                            terminated = true;
                            break;
                        }
                        _ => return Err(format!("unimplemented particle opcode {op:#x}")),
                    }
                }
                if !terminated {
                    return Err("particle command budget exceeded".into());
                }
                self.pc = r.pc;
            }
        }
        if self.kind & 1 != 0 {
            self.velocity.y -= self.gravity;
        }
        if self.kind & 2 != 0 {
            self.velocity *= self.friction;
        }
        self.position += self.velocity;
        self.life = self.life.saturating_sub(1);
        Ok(children)
    }
}
