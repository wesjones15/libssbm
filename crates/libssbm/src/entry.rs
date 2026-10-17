//! Shared trophy-to-fighter entry. Behavioral spec: ft_0C31.c
//! ftCo_800C6408, EntryStart_Phys, 800C6B6C and EntryEnd_Phys.
use glam::{Mat4, Vec3};
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub grow: u32,
    pub release: u32,
    pub flat: f32,
    pub platform: f32,
    pub color: [f32; 4],
    pub color_frames: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub params: Params,
    pub frame: u32,
}
impl State {
    pub fn new(params: Params) -> Self {
        Self { params, frame: 0 }
    }
    pub fn finished(&self) -> bool {
        self.frame >= self.params.grow + self.params.release
    }
    pub fn growth(&self) -> f32 {
        (self.frame as f32 / self.params.grow.max(1) as f32).min(1.)
    }
    pub fn platform_y(&self) -> f32 {
        if self.frame < self.params.grow {
            self.growth()
        } else {
            (1. - (self.frame - self.params.grow) as f32 / self.params.release.max(1) as f32)
                .max(0.)
        }
    }
    pub fn height(&self) -> f32 {
        1.497345 * self.params.platform * self.platform_y()
    }
    pub fn transform(&self) -> Mat4 {
        Mat4::from_translation(Vec3::Y * self.height())
            * Mat4::from_scale(Vec3::new(
                1.,
                self.params.flat + (1. - self.params.flat) * self.growth(),
                1.,
            ))
    }
}
