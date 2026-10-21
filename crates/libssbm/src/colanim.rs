//! Colour animations: the glows and flashes a fighter's scripts start with command 46
//! (ftCo_800BFFD0 -> lb_800144C8 starts, lb_80014258 runs them each tick). A program is a
//! small command list in PlCo.dat: generic loop and timer commands (lbcommand.c), colour
//! and light commands (lb_013B.c), and the fighter's own GFX and sound commands. Two things
//! come out: the colour overlay (`x2C_hex`, drawn over the whole model) and the light the
//! fighter is lit with (`x50_light_color` and its direction, see [`Light`]).
use std::sync::Arc;

/// One entry of the table at PlCo `ftLoadCommonData + 0x18`: 8 bytes, a program pointer, a
/// priority (`unk4`) and the channel that plays it (`unk5`).
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub program: Option<usize>,
    pub priority: u8,
    pub channel: u8,
}

/// The shared table with the file's bytes, so a program's gotos and calls (data-section
/// offsets) stay readable.
#[derive(Debug)]
pub struct Table {
    pub data: Vec<u8>,
    pub entries: Vec<Entry>,
}
impl Table {
    fn word(&self, at: usize) -> u32 {
        self.data
            .get(at..at + 4)
            .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// The light a program sets on the fighter (commands 13 to 17, ftCo_09F4.c and ftmaterial.c).
/// With `additive` set (the light-enable bit of command 13) the light is not a lamp: the
/// materials add `color.rgb` scaled by what the light lights of them, with `color.a` as the
/// intensity (ftMaterial_800BF534). Without it the colour replaces a stage light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// RGBA, 0 to 1.
    pub color: [f32; 4],
    /// Degrees around the fighter and up from the horizontal plane that the light comes from.
    pub x: f32,
    pub yz: f32,
    pub additive: bool,
}

/// A program's side effects for the sim to apply.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Command 21: the same five words as a script's GFX spawn.
    Gfx([u32; 5]),
    /// Command 22: the same words as a script's sound.
    Sound([u32; 3]),
}

#[derive(Clone, Debug, Default)]
struct Channel {
    id: usize,
    pc: Option<usize>,
    timer: i64,
    /// Frames left before it stops itself (the start's second argument; 0 runs until the
    /// program ends).
    life: u32,
    loops: Vec<(usize, i64)>,
    calls: Vec<usize>,
    color_enable: bool,
    color: [f32; 4],
    blend: [f32; 4],
    hex: [u8; 4],
    light: [f32; 4],
    light_color: [u8; 4],
    light_blend: [f32; 4],
    light_on: bool,
    light_additive: bool,
    light_rot: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct ColorAnim {
    pub table: Option<Arc<Table>>,
    channels: [Option<Channel>; 2],
}

const FOREVER: i64 = i64::MAX / 4;

impl ColorAnim {
    /// Starts animation `id` for `life` frames (0: until it ends) if its priority is at least that
    /// of the one running on its channel. Returns whether it started.
    pub fn start(&mut self, id: usize, life: u32) -> bool {
        let Some(table) = self.table.clone() else { return false };
        let Some(entry) = table.entries.get(id).copied() else { return false };
        let slot = usize::from(entry.channel != 0);
        if let Some(current) = &self.channels[slot] {
            let running = table.entries.get(current.id).map_or(0, |e| e.priority);
            if running > entry.priority {
                return false;
            }
        }
        self.channels[slot] = Some(Channel {
            id,
            pc: entry.program,
            life,
            ..Channel::default()
        });
        true
    }

    pub fn clear(&mut self) {
        self.channels = [None, None];
    }

    /// Whether anything is playing.
    pub fn active(&self) -> bool {
        self.channels.iter().any(Option::is_some)
    }

    /// Runs one tick; returns the GFX and sound commands the programs issued.
    pub fn tick(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let Some(table) = self.table.clone() else { return events };
        for slot in &mut self.channels {
            let Some(channel) = slot.as_mut() else { continue };
            if channel.run(&table, &mut events) {
                *slot = None;
            }
        }
        events
    }

    /// The light the playing programs have on (the fighter-wide channel's, else the other's).
    pub fn light(&self) -> Option<Light> {
        [1usize, 0].into_iter().find_map(|i| {
            let c = self.channels[i].as_ref()?;
            c.light_on.then(|| Light {
                color: c.light_color.map(|v| v as f32 / 255.),
                x: c.light_rot[0],
                yz: c.light_rot[1],
                additive: c.light_additive,
            })
        })
    }

    /// The colour laid over the model (RGBA, 0 to 1): the fighter-wide channel when it is on,
    /// otherwise the other.
    pub fn overlay(&self) -> Option<[f32; 4]> {
        [1usize, 0].into_iter().find_map(|i| {
            let c = self.channels[i].as_ref()?;
            c.color_enable.then(|| {
                [
                    c.hex[0] as f32 / 255.,
                    c.hex[1] as f32 / 255.,
                    c.hex[2] as f32 / 255.,
                    c.hex[3] as f32 / 255.,
                ]
            })
        })
    }
}

/// A `bits`-wide two's-complement field as a number.
fn signed(value: u32, bits: u32) -> f32 {
    (((value << (32 - bits)) as i32) >> (32 - bits)) as f32
}

impl Channel {
    /// lb_80014258. True when the program has finished and the channel should clear.
    fn run(&mut self, t: &Table, events: &mut Vec<Event>) -> bool {
        if self.pc.is_some() && self.timer != 0 {
            self.timer -= 1;
        }
        let mut guard = 0;
        while let Some(pc) = self.pc {
            if self.timer != 0 {
                break;
            }
            guard += 1;
            if guard > 256 {
                return true;
            }
            let word = t.word(pc);
            let op = word >> 26;
            let value = (word & 0x03ff_ffff) as i64;
            match op {
                0 => self.pc = None,
                1 => {
                    self.timer += value;
                    self.pc = Some(pc + 4);
                }
                2 => {
                    self.timer = value;
                    self.pc = Some(pc + 4);
                }
                3 => {
                    self.loops.push((pc + 4, value));
                    self.pc = Some(pc + 4);
                }
                4 => match self.loops.last_mut() {
                    Some((start, count)) => {
                        *count -= 1;
                        if *count != 0 {
                            self.pc = Some(*start);
                        } else {
                            self.loops.pop();
                            self.pc = Some(pc + 4);
                        }
                    }
                    None => self.pc = Some(pc + 4),
                },
                5 => {
                    self.calls.push(pc + 8);
                    self.pc = Some(t.word(pc + 4) as usize);
                }
                6 => self.pc = self.calls.pop(),
                7 => self.pc = Some(t.word(pc + 4) as usize),
                8 => {
                    self.timer = FOREVER;
                    self.pc = Some(pc + 8);
                }
                9 => self.pc = Some(pc + 4),
                10 => return true,
                11 => {
                    self.timer += value;
                    self.pc = Some(pc + 4);
                }
                12 => {
                    self.color_enable = false;
                    self.light_on = false;
                    self.pc = Some(pc + 4);
                }
                13 => {
                    // lb_80013C18: the light-enable bit and the direction, then the colour.
                    self.light_additive = word >> 25 & 1 != 0;
                    self.light_rot = [signed(word >> 12 & 0xfff, 12), signed(word & 0xfff, 12)];
                    let c = t.word(pc + 4).to_be_bytes();
                    self.light_color = c;
                    self.light = c.map(f32::from);
                    self.light_blend = [0.; 4];
                    self.light_on = true;
                    self.pc = Some(pc + 8);
                }
                14 => {
                    let c = t.word(pc + 4).to_be_bytes();
                    self.light_color = c;
                    self.light = c.map(f32::from);
                    self.light_blend = [0.; 4];
                    self.pc = Some(pc + 8);
                }
                15 => {
                    let f = value.max(1) as f32;
                    let target = t.word(pc + 4).to_be_bytes();
                    for i in 0..4 {
                        self.light_blend[i] = ((0.5 + target[i] as f32) - self.light_color[i] as f32) / f;
                    }
                    self.pc = Some(pc + 8);
                }
                16 => {
                    // lb_80013F78: turn the light to a new direction.
                    self.light_rot = [signed(word >> 13 & 0x1fff, 13), signed(word & 0x1fff, 13)];
                    self.pc = Some(pc + 4);
                }
                17 => {
                    self.light_on = false;
                    self.pc = Some(pc + 4);
                }
                18 => {
                    let c = t.word(pc + 4).to_be_bytes();
                    self.color_enable = true;
                    self.hex = c;
                    self.color = c.map(f32::from);
                    self.blend = [0.; 4];
                    self.pc = Some(pc + 8);
                }
                19 => {
                    let f = value.max(1) as f32;
                    let target = t.word(pc + 4).to_be_bytes();
                    for i in 0..4 {
                        self.blend[i] = ((0.5 + target[i] as f32) - self.hex[i] as f32) / f;
                    }
                    self.pc = Some(pc + 8);
                }
                20 => {
                    self.color_enable = false;
                    self.pc = Some(pc + 4);
                }
                21 => {
                    events.push(Event::Gfx([
                        word,
                        t.word(pc + 4),
                        t.word(pc + 8),
                        t.word(pc + 12),
                        t.word(pc + 16),
                    ]));
                    self.pc = Some(pc + 20);
                }
                22 => {
                    events.push(Event::Sound([word, t.word(pc + 4), t.word(pc + 8)]));
                    let behavior = word >> 18 & 0xff;
                    let words = if behavior <= 6 || (10..=15).contains(&behavior) { 3 } else { 2 };
                    self.pc = Some(pc + 4 * words);
                }
                23 => self.pc = Some(pc + 4),
                _ => return true,
            }
        }
        if self.color_enable {
            for i in 0..4 {
                self.color[i] += self.blend[i];
                self.hex[i] = self.color[i].clamp(0., 255.) as u8;
            }
        }
        if self.light_on {
            for i in 0..4 {
                self.light[i] += self.light_blend[i];
                self.light_color[i] = self.light[i].clamp(0., 255.) as u8;
            }
        }
        if self.life != 0 {
            self.life -= 1;
            if self.life == 0 {
                return true;
            }
        }
        self.pc.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(words: &[u32]) -> Arc<Table> {
        Arc::new(Table {
            data: words.iter().flat_map(|w| w.to_be_bytes()).collect(),
            entries: vec![Entry { program: Some(0), priority: 60, channel: 1 }],
        })
    }

    #[test]
    fn a_colour_holds_for_its_timer_then_fades_out_and_ends() {
        // colour (255,0,0,128) for 2 frames, then end.
        let program = [0x4800_0000, 0xff00_0080, 0x2c00_0002, 0x2800_0000];
        let mut anim = ColorAnim { table: Some(table(&program)), ..Default::default() };
        assert!(anim.start(0, 0));
        anim.tick();
        assert_eq!(anim.overlay(), Some([1., 0., 0., 128. / 255.]));
        anim.tick();
        assert!(anim.overlay().is_some(), "still on at the second frame");
        anim.tick();
        assert!(!anim.active(), "the program ended");
    }

    #[test]
    fn a_loop_repeats_its_body() {
        // loop 3 times: colour on for 1 frame, off for 1 frame; then end.
        let program = [0x0c00_0003, 0x4800_0000, 0xffff_ffff, 0x2c00_0001, 0x3000_0000, 0x2c00_0001, 0x1000_0000, 0x2800_0000];
        let mut anim = ColorAnim { table: Some(table(&program)), ..Default::default() };
        anim.start(0, 0);
        let mut on = 0;
        for _ in 0..12 {
            anim.tick();
            if anim.overlay().is_some() {
                on += 1;
            }
        }
        assert_eq!(on, 3, "three flashes");
        assert!(!anim.active());
    }

    #[test]
    fn a_light_has_a_direction_a_colour_and_fades() {
        // light on: additive, x 75, yz -10, colour (255,100,0,200); fade to alpha 0 over 4
        // frames; hold 1; light off; end.
        let on = 13u32 << 26 | 1 << 25 | (75 & 0xfff) << 12 | ((-10i32) as u32 & 0xfff);
        let program = [on, 0xff64_00c8, 15 << 26 | 4, 0xff64_0000, 2 << 26 | 6, 17 << 26, 0x2800_0000];
        let mut anim = ColorAnim { table: Some(table(&program)), ..Default::default() };
        anim.start(0, 0);
        anim.tick();
        let light = anim.light().expect("light on");
        assert!(light.additive);
        assert_eq!((light.x, light.yz), (75., -10.));
        assert_eq!(&light.color[..3], &[1., 100. / 255., 0.]);
        let start = light.color[3];
        for _ in 0..3 {
            anim.tick();
        }
        assert!(anim.light().unwrap().color[3] < start / 2., "the light did not fade");
        for _ in 0..4 {
            anim.tick();
        }
        assert!(anim.light().is_none(), "light off did not end it");
    }

    #[test]
    fn a_light_rotation_command_turns_it() {
        let on = 13u32 << 26 | (10 & 0xfff) << 12;
        let rotate = 16u32 << 26 | ((-45i32) as u32 & 0x1fff) << 13 | (20 & 0x1fff);
        let program = [on, 0xffff_ff80, 2 << 26 | 1, rotate, 2 << 26 | 5, 0x2800_0000];
        let mut anim = ColorAnim { table: Some(table(&program)), ..Default::default() };
        anim.start(0, 0);
        anim.tick();
        assert_eq!(anim.light().map(|l| (l.x, l.additive)), Some((10., false)));
        anim.tick();
        anim.tick();
        assert_eq!(anim.light().map(|l| (l.x, l.yz)), Some((-45., 20.)));
    }

    #[test]
    fn a_lower_priority_does_not_replace_a_higher_one() {
        let mut table = Arc::try_unwrap(table(&[0x4800_0000, 0xffff_ffff, 0x2c00_0010, 0x2800_0000])).unwrap();
        table.entries.push(Entry { program: Some(0), priority: 10, channel: 1 });
        let mut anim = ColorAnim { table: Some(Arc::new(table)), ..Default::default() };
        assert!(anim.start(0, 0));
        assert!(!anim.start(1, 0), "priority 10 must not displace 60");
    }
}
