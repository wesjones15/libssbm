//! SSM DSP-ADPCM samples and SEM sound sequences from the player's disc.
//! Behavioral specifications: baselib/synth.c (sample loading, envelope and
//! master clock), axdriver.c::AXDriverInterp/Exec and HSD_AudioSFXStartParam.
//! Own bounded interpreter; no source audio or sequence data is compiled in.
use crate::Result;
use std::{collections::BTreeMap, path::Path};

pub const RATE: u32 = 32_000;
// GALE01's libs/dolphin/ax/AXOut.c queues 160 stereo frames at 32 kHz (5 ms).
const CLOCK: usize = 160;
fn u16be(b: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(
        b.get(at..at + 2)
            .ok_or("truncated audio word")?
            .try_into()?,
    ))
}
fn u32be(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        b.get(at..at + 4)
            .ok_or("truncated audio word")?
            .try_into()?,
    ))
}

#[derive(Clone)]
pub struct Sample {
    pub rate: u32,
    pub pcm: Vec<[f32; 2]>,
    pub loop_pcm: Vec<[f32; 2]>,
}
pub struct Bank {
    pub samples: BTreeMap<u32, Sample>,
    sequences: Vec<u32>,
    banks: Vec<u32>,
    words: Vec<u32>,
}
pub fn ssm(bytes: &[u8]) -> Result<BTreeMap<u32, Sample>> {
    let header = u32be(bytes, 0)? as usize;
    let size = u32be(bytes, 4)? as usize;
    let count = u32be(bytes, 8)? as usize;
    let base = u32be(bytes, 12)?;
    let data_at = (header.checked_add(47).ok_or("SSM header overflow")?) & !31;
    let data = bytes
        .get(data_at..data_at.checked_add(size).ok_or("SSM size overflow")?)
        .ok_or("truncated SSM samples")?;
    if count > 4096 || size > 64 * 1024 * 1024 {
        return Err("SSM budget exceeded".into());
    }
    let mut at = 16;
    let mut samples = BTreeMap::new();
    for i in 0..count {
        let channels = u32be(bytes, at)? as usize;
        let rate = u32be(bytes, at + 4)?;
        if !(1..=2).contains(&channels) || !(4000..=96000).contains(&rate) {
            return Err("invalid SSM channel/rate".into());
        }
        at += 8;
        let mut pcm = Vec::new();
        let mut loops = Vec::new();
        for ch in 0..channels {
            let info = bytes.get(at..at + 64).ok_or("truncated SSM voice")?;
            if u16be(info, 2)? != 0 {
                return Err("unsupported SSM sample format".into());
            }
            let end = u32be(info, 8)? as usize;
            let decode = |start: usize, h1: i16, h2: i16| -> Result<Vec<f32>> {
                decode_dsp(data, start, end, &info[16..48], h1, h2)
            };
            let voice = decode(
                u32be(info, 12)? as usize,
                u16be(info, 52)? as i16,
                u16be(info, 54)? as i16,
            )?;
            merge(&mut pcm, voice, ch, channels)?;
            if u16be(info, 0)? != 0 {
                let voice = decode(
                    u32be(info, 4)? as usize,
                    u16be(info, 58)? as i16,
                    u16be(info, 60)? as i16,
                )?;
                merge(&mut loops, voice, ch, channels)?;
            }
            at += 64;
        }
        samples.insert(
            base.checked_add(i as u32).ok_or("SSM sample id overflow")?,
            Sample {
                rate,
                pcm,
                loop_pcm: loops,
            },
        );
    }
    if at != header + 16 {
        return Err("SSM header length mismatch".into());
    }
    Ok(samples)
}
fn merge(out: &mut Vec<[f32; 2]>, channel: Vec<f32>, ch: usize, channels: usize) -> Result<()> {
    if ch == 0 {
        out.resize(channel.len(), [0.; 2]);
    }
    if out.len() != channel.len() {
        return Err("SSM stereo lengths differ".into());
    }
    for (frame, value) in out.iter_mut().zip(channel) {
        frame[ch] = value;
        if channels == 1 {
            frame[1] = value;
        }
    }
    Ok(())
}
fn decode_dsp(
    data: &[u8],
    start: usize,
    end: usize,
    coefs: &[u8],
    h1: i16,
    h2: i16,
) -> Result<Vec<f32>> {
    if start > end || end / 2 >= data.len() || end - start > 32 * 1024 * 1024 {
        return Err("DSP sample outside data/budget".into());
    }
    let mut result = Vec::with_capacity((end - start + 1) * 7 / 8);
    let (mut prev, mut prev2) = (h1 as i64, h2 as i64);
    for address in start..=end {
        if address % 16 < 2 {
            continue;
        }
        let header = data[address / 16 * 8];
        let predictor = (header >> 4) as usize;
        if predictor >= 8 {
            return Err("DSP predictor outside coefficient table".into());
        }
        let coef1 = u16be(coefs, predictor * 4)? as i16 as i64;
        let coef2 = u16be(coefs, predictor * 4 + 2)? as i16 as i64;
        let byte = data[address / 2];
        let nibble = if address % 2 == 0 {
            byte >> 4
        } else {
            byte & 15
        };
        let signed = if nibble >= 8 {
            nibble as i64 - 16
        } else {
            nibble as i64
        };
        let value = ((signed * (1 << (header & 15)) * 2048 + coef1 * prev + coef2 * prev2 + 1024)
            >> 11)
            .clamp(-32768, 32767);
        result.push(value as f32 / 32768.);
        prev2 = prev;
        prev = value;
    }
    Ok(result)
}

impl Bank {
    pub fn sequence(bytes: &[u8]) -> Result<Self> {
        if !bytes.len().is_multiple_of(4) {
            return Err("unaligned SEM file".into());
        }
        let mut at = 0;
        let mut tables = Vec::new();
        for _ in 0..5 {
            let count = u32be(bytes, at)? as usize;
            at += 4;
            if count > 65536 {
                return Err("SEM table exceeds budget".into());
            }
            let t = (0..count)
                .map(|i| u32be(bytes, at + i * 4))
                .collect::<Result<Vec<_>>>()?;
            at += count * 4;
            tables.push(t);
        }
        let sequences = tables.remove(3);
        let banks = tables.remove(2);
        for &offset in &sequences {
            if offset as usize >= bytes.len() || offset % 4 != 0 || (offset as usize) < at {
                return Err("SEM sequence outside command region".into());
            }
        }
        if banks.windows(2).any(|b| b[0] > b[1])
            || banks.iter().any(|&n| n as usize > sequences.len())
        {
            return Err("SEM bank range invalid".into());
        }
        Ok(Self {
            samples: BTreeMap::new(),
            banks,
            sequences,
            words: bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_be_bytes(*b))
                .collect(),
        })
    }
    pub fn load(dir: &Path) -> Result<Self> {
        let mut bank = Self::sequence(&std::fs::read(dir.join("audio/us/smash2.sem"))?)?;
        for name in ["main.ssm", "samus.ssm"] {
            for (id, sample) in ssm(&std::fs::read(dir.join("audio/us").join(name))?)? {
                if bank.samples.insert(id, sample).is_some() {
                    return Err("duplicate SSM sample id".into());
                }
            }
        }
        Ok(bank)
    }
    fn script(&self, sound: u32) -> Result<usize> {
        let bank = sound as usize / 10000;
        let index =
            *self.banks.get(bank).ok_or("SEM bank missing")? as usize + sound as usize % 10000;
        if self
            .banks
            .get(bank + 1)
            .is_some_and(|&end| index >= end as usize)
        {
            return Err("SEM sound outside bank".into());
        }
        Ok(*self.sequences.get(index).ok_or("SEM sound missing")? as usize / 4)
    }
    pub fn sample_ids(&self, sound: u32) -> Result<Vec<u32>> {
        let mut v = Voice::new(self.script(sound)?, 127, 64, 0);
        let mut ids = Vec::new();
        // Dependency walk executes bounded commands, including source loops.
        for _ in 0..20000 {
            v.tick(self)?;
            if let Some(id) = v.sample
                && !ids.contains(&id)
            {
                ids.push(id);
            }
            if v.sleep || v.dead {
                return Ok(ids);
            }
        }
        Err("SEM dependency walk exceeds budget".into())
    }
}
struct Voice {
    pc: usize,
    wait: u32,
    loop_count: u32,
    infinite: bool,
    sample: Option<u32>,
    pending: Option<u32>,
    position: f64,
    looping: bool,
    volume: u8,
    gain: i32,
    current_gain: f32,
    pitch: i32,
    pan: u8,
    channel: u32,
    sleep: bool,
    dead: bool,
}
impl Voice {
    fn new(pc: usize, volume: u8, pan: u8, channel: u32) -> Self {
        Self {
            pc,
            wait: 0,
            loop_count: 0,
            infinite: false,
            sample: None,
            pending: None,
            position: 0.,
            looping: false,
            volume,
            gain: 255,
            current_gain: 0.,
            pitch: 0,
            pan,
            channel,
            sleep: false,
            dead: false,
        }
    }
    fn flush(&mut self) {
        if let Some(sample) = self.pending.take() {
            self.sample = Some(sample);
            self.position = 0.;
            self.looping = false;
            self.current_gain = self.gain as f32;
        }
    }
    fn tick(&mut self, bank: &Bank) -> Result<()> {
        if self.sleep || self.dead {
            return Ok(());
        }
        if self.wait > 0 {
            self.wait -= 1;
            if self.wait > 0 {
                return Ok(());
            }
        }
        for _ in 0..4096 {
            let word = *bank.words.get(self.pc).ok_or("SEM command outside file")?;
            let op = word >> 24;
            let value = word & 0xffffff;
            let wait = match op {
                0 => value,
                6..=11 | 16..=19 => value >> 8,
                12..=13 => value >> 16,
                _ => 0,
            };
            if wait > 0 {
                self.flush();
            }
            self.pc += 1;
            match op {
                0 | 0xfd => (),
                1 => {
                    if !bank.samples.contains_key(&value) {
                        return Err(format!("SEM sample {value} was not extracted").into());
                    }
                    self.pending = Some(value);
                }
                2 => {
                    self.loop_count = value;
                    self.infinite = value == 0;
                }
                3 => {
                    if self.infinite || self.loop_count > 0 {
                        self.pc = (self.pc - 1)
                            .checked_sub(value as usize)
                            .ok_or("SEM loop before file")?
                            + 1;
                        self.loop_count = self.loop_count.saturating_sub(1);
                    }
                }
                4..=5 | 10..=11 | 16..=21 => (), // Priority, surround and mix routing do not change stereo PCM.
                6 => self.gain = value as u8 as i32,
                7 => self.gain = (self.gain + value as u8 as i8 as i32).clamp(0, 255),
                8 => self.pan = ((value as u8 as u32 * 127) / 255) as u8,
                9 => {
                    self.pan = (self.pan as i32 + value as u8 as i8 as i32 / 2).clamp(0, 127) as u8
                }
                12 => self.pitch = value as u16 as i16 as i32,
                13 => self.pitch = (self.pitch + value as u16 as i16 as i32).clamp(-10800, 2400),
                14 => {
                    self.flush();
                    self.sleep = true;
                    return Ok(());
                }
                15 => {
                    self.dead = true;
                    return Ok(());
                }
                _ => return Err(format!("unsupported SEM opcode {op}").into()),
            }
            if wait > 0 {
                self.wait = wait;
                return Ok(());
            }
        }
        Err("SEM zero-time command loop exceeds budget".into())
    }
    fn frame(&mut self, bank: &Bank) -> [f32; 2] {
        let Some(sample) = self.sample.and_then(|id| bank.samples.get(&id)) else {
            return [0.; 2];
        };
        let mut pcm = if self.looping {
            &sample.loop_pcm
        } else {
            &sample.pcm
        };
        if self.position >= pcm.len() as f64 {
            if sample.loop_pcm.is_empty() {
                self.sample = None;
                if self.sleep {
                    self.dead = true;
                }
                return [0.; 2];
            }
            self.position -= pcm.len() as f64;
            self.looping = true;
            pcm = &sample.loop_pcm;
            self.position %= pcm.len() as f64;
        }
        let idx = self.position as usize;
        let frac = (self.position - idx as f64) as f32;
        let a = pcm[idx];
        let b = pcm.get(idx + 1).copied().unwrap_or(a);
        self.position += sample.rate as f64 / RATE as f64 * 2f64.powf(self.pitch as f64 / 1200.);
        // synth.c::HSD_SynthSFXVolumeEnvelope ramps gain over an AX frame.
        self.current_gain +=
            (self.gain as f32 - self.current_gain).clamp(-255. / CLOCK as f32, 255. / CLOCK as f32);
        let gain = self.current_gain / 255. * self.volume as f32 / 127.;
        let pan = self.pan as f32 / 127.;
        [
            (a[0] + (b[0] - a[0]) * frac) * gain * (2. * (1. - pan)).sqrt(),
            (a[1] + (b[1] - a[1]) * frac) * gain * (2. * pan).sqrt(),
        ]
    }
}
pub struct Mixer {
    pub bank: Bank,
    voices: Vec<Voice>,
    clock: usize,
    pub errors: Vec<String>,
}
impl Mixer {
    pub fn new(bank: Bank) -> Self {
        Self {
            bank,
            voices: Vec::new(),
            clock: 0,
            errors: Vec::new(),
        }
    }
    pub fn play(&mut self, sound: u32, volume: u8, pan: u8, channel: u32) -> Result<()> {
        let pc = self.bank.script(sound)?;
        if channel > 0 {
            self.stop(channel);
        }
        if self.voices.len() >= 64 {
            self.voices.remove(0);
        }
        let mut v = Voice::new(pc, volume.min(127), pan.min(127), channel);
        v.tick(&self.bank)?;
        self.voices.push(v);
        Ok(())
    }
    pub fn stop(&mut self, channel: u32) {
        self.voices.retain(|v| v.channel != channel);
    }
    pub fn clear(&mut self) {
        self.voices.clear();
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for frame in out.as_chunks_mut::<2>().0 {
            if self.clock == CLOCK {
                self.clock = 0;
                for voice in &mut self.voices {
                    if let Err(e) = voice.tick(&self.bank) {
                        voice.dead = true;
                        self.errors.push(e.to_string());
                    }
                }
                self.voices.retain(|v| !v.dead);
            }
            let mut sum = [0.; 2];
            for voice in &mut self.voices {
                let x = voice.frame(&self.bank);
                sum[0] += x[0];
                sum[1] += x[1];
            }
            frame.copy_from_slice(&sum);
            self.clock += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dsp_nibbles_sign_saturation_and_history() -> Result<()> {
        let mut data = [0x11; 8];
        data[0] = 0;
        data[1] = 0x78;
        let decoded = decode_dsp(&data, 2, 15, &[0; 32], 0, 0)?;
        assert_eq!(decoded.len(), 14);
        assert_eq!(decoded[0], 7. / 32768.);
        assert_eq!(decoded[1], -8. / 32768.);
        let mut coefs = [0; 32];
        coefs[..2].copy_from_slice(&2048u16.to_be_bytes());
        assert_eq!(decode_dsp(&data, 2, 2, &coefs, 100, 0)?[0], 107. / 32768.);
        data[0] = 0x80;
        assert!(decode_dsp(&data, 2, 15, &coefs, 0, 0).is_err());
        assert!(decode_dsp(&data, 2, 16, &coefs, 0, 0).is_err());
        Ok(())
    }
    #[test]
    fn sequence_gain_pitch_wait_loop_and_stop() -> Result<()> {
        let mut bank = Bank {
            samples: BTreeMap::new(),
            sequences: vec![0],
            banks: vec![0],
            words: vec![
                0x01000001, 0x06000080, 0x0c0004b0, 0x02000002, 0x00000002, 0x03000001, 0x0f000000,
            ],
        };
        bank.samples.insert(
            1,
            Sample {
                rate: RATE,
                pcm: vec![[0.25; 2]; 4000],
                loop_pcm: vec![],
            },
        );
        let mut mixer = Mixer::new(bank);
        mixer.play(0, 127, 64, 1)?;
        let mut out = vec![0.; CLOCK * 2 * 10];
        mixer.render(&mut out);
        assert!(out[..CLOCK * 8].iter().any(|v| *v > 0.));
        assert!(out[CLOCK * 2 * 7..].iter().all(|v| *v == 0.));
        assert!(mixer.errors.is_empty());
        mixer.play(0, 127, 64, 1)?;
        mixer.stop(1);
        mixer.render(&mut out);
        assert!(out.iter().all(|v| *v == 0.));
        assert!(Bank::sequence(&[0; 19]).is_err());
        assert!(ssm(&[0; 32]).is_ok());
        Ok(())
    }
}
