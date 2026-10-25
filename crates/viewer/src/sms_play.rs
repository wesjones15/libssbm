//! A playable Sunshine sim driving Luigi: the libsms `Sim` runs Mario's movement,
//! animation state and FLUDD on a flat floor; the current `ma_*` clip is retargeted
//! onto Luigi's Melee rig by the viewer.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::{Vec3, Vec2};
use libsms::{AnimTable, GroundHit, Host, Pad, Params, Sim, SurfaceFlags};
use libssbm_assets::sms;

struct Flat;

impl Host for Flat {
    fn ground(&self, pos: Vec3) -> Option<GroundHit> {
        // Like TMap::checkGround: a floor only counts at or below the probe.
        (0.0 <= pos.y).then_some(GroundHit {
            height: 0.0,
            normal: Vec3::Y,
            flags: SurfaceFlags::NONE,
        })
    }
}

pub struct Play {
    sim: Sim,
    host: Flat,
    bck_dir: PathBuf,
    clips: HashMap<String, sms::Clip>,
}

impl Play {
    pub fn new(sms_assets: &Path) -> Play {
        Play {
            sim: Sim::new(
                Params::default(),
                AnimTable::default(),
                Vec3::ZERO,
                0,
            ),
            host: Flat,
            bck_dir: sms_assets.join("bck"),
            clips: HashMap::new(),
        }
    }

    pub fn step(&mut self, dt: f32, pad: &Pad) {
        // Cap catch-up so a slow frame cannot run hundreds of ticks at once.
        self.sim.advance(&mut self.host, pad, 0, dt.min(0.05));
    }

    pub fn pos(&self) -> Vec3 {
        self.sim.mario.pos
    }

    pub fn yaw(&self) -> f32 {
        self.sim.mario.model_yaw as f32 * std::f32::consts::PI / 32768.0
    }

    /// The current body clip and frame, loaded by the `ma_<name>.bck` naming.
    pub fn clip(&mut self) -> Option<(&sms::Clip, f32)> {
        let name = self.sim.mario.anim.clip;
        if !self.clips.contains_key(name) {
            let data = std::fs::read(self.bck_dir.join(format!("ma_{name}.bck"))).ok()?;
            let clip = sms::Clip::from_bck(&data).ok()?;
            self.clips.insert(name.to_string(), clip);
        }
        let clip = self.clips.get(name)?;
        Some((clip, self.sim.mario.anim.frame))
    }

    /// The pad from the viewer's keys: WASD, Space jump, R spray, Shift dive.
    pub fn pad(&self) -> Pad {
        let mut pad = Pad::default();
        let mut stick = Vec2::ZERO;
        if is_key_down(KeyCode::W) {
            stick.y += 1.0;
        }
        if is_key_down(KeyCode::S) {
            stick.y -= 1.0;
        }
        if is_key_down(KeyCode::A) {
            stick.x -= 1.0;
        }
        if is_key_down(KeyCode::D) {
            stick.x += 1.0;
        }
        pad.stick = if stick.length() > 1.0 {
            stick.normalize()
        } else {
            stick
        };
        pad.a = is_key_down(KeyCode::Space);
        pad.b = is_key_down(KeyCode::LeftShift);
        pad.r = is_key_down(KeyCode::R);
        pad.r_analog = if pad.r { 255 } else { 0 };
        pad
    }
}

use macroquad::prelude::is_key_down;
use macroquad::prelude::KeyCode;

