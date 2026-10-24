use libssbm_assets::effects::Effects;
use macroquad::{
    miniquad::{BlendFactor, BlendState, BlendValue, Equation},
    prelude::*,
};
pub struct Renderer {
    textures: Vec<Vec<Vec<Texture2D>>>,
    alpha: Material,
    additive: Material,
    radial: Texture2D,
    glow: Option<(u32, f32, f32)>,
}
const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying lowp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main(){gl_Position=Projection*Model*vec4(position,1.0);uv=texcoord;}
"#;
const FRAGMENT: &str = r#"#version 100
precision lowp float;
varying vec2 uv;
uniform sampler2D Texture;
uniform vec4 Primary;
uniform vec4 Environment;
void main(){gl_FragColor=mix(Environment,Primary,texture2D(Texture,uv));}
"#;
fn radial_texture() -> Texture2D {
    let n = 64usize;
    let mut bytes = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let dx = (x as f32 + 0.5) / n as f32 * 2. - 1.;
            let dy = (y as f32 + 0.5) / n as f32 * 2. - 1.;
            let r = (dx * dx + dy * dy).sqrt().min(1.);
            let a = 1. - r;
            let a = a * a * (3. - 2. * a);
            let i = (y * n + x) * 4;
            bytes[i] = 255;
            bytes[i + 1] = 255;
            bytes[i + 2] = 255;
            bytes[i + 3] = (a * 255.).round() as u8;
        }
    }
    Texture2D::from_rgba8(n as u16, n as u16, &bytes)
}
impl Renderer {
    pub fn new(bank: &Effects) -> libssbm_assets::Result<Self> {
        let material = |additive| {
            load_material(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                MaterialParams {
                    uniforms: vec![
                        UniformDesc::new("Primary", UniformType::Float4),
                        UniformDesc::new("Environment", UniformType::Float4),
                    ],
                    pipeline_params: PipelineParams {
                        depth_test: Comparison::LessOrEqual,
                        depth_write: false,
                        color_blend: Some(BlendState::new(
                            Equation::Add,
                            BlendFactor::Value(BlendValue::SourceAlpha),
                            if additive {
                                BlendFactor::One
                            } else {
                                BlendFactor::OneMinusValue(BlendValue::SourceAlpha)
                            },
                        )),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
        };
        let textures = bank
            .textures
            .iter()
            .map(|g| {
                g.as_ref()
                    .map(|g| {
                        g.palettes
                            .iter()
                            .map(|frames| {
                                frames
                                    .iter()
                                    .map(|t| Texture2D::from_rgba8(t.width, t.height, &t.rgba))
                                    .collect()
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();
        Ok(Self {
            textures,
            alpha: material(false)?,
            additive: material(true)?,
            radial: radial_texture(),
            glow: None,
        })
    }
    /// Draw `id` with a soft centred radial mask instead of its source texture, and
    /// scale/soften it. Mirrors the RoR2 host's Pikachu 0x4BD aura tuning.
    pub fn set_glow(&mut self, id: u32, scale: f32, alpha: f32) {
        self.glow = Some((id, scale, alpha));
    }
    pub fn draw(&self, system: &libssbm::particle::System, camera: Vec3) {
        for p in &system.particles {
            // Trail particles (bit 22) have no billboard flag; draw each as a sprite so the
            // normal Super Scope smoke (generator 0x140's children) is visible.
            if p.kind & ((1 << 10) | (1 << 22)) == 0 {
                continue;
            }
            let palettes = &self.textures[p.texture as usize];
            let palette = if p.palette == 255 {
                0
            } else {
                p.palette as usize
            };
            let Some(texture) = palettes
                .get(palette)
                .and_then(|pals| pals.get(p.frame as usize))
            else {
                continue;
            };
            let glow = self.glow.filter(|(id, _, _)| *id == p.emitter);
            let (texture, size_mul, alpha_mul) = match glow {
                Some((_, scale, alpha)) => (&self.radial, scale, alpha),
                None => (texture, 1., 1.),
            };
            let at = p.transform.transform_point3(p.position);
            let at = vec3(at.x, at.y, at.z);
            let look = (camera - at).normalize();
            let right = Vec3::Y.cross(look).normalize();
            let up = look.cross(right);
            let right2 = right * p.rotation.cos() + up * p.rotation.sin();
            let up2 = up * p.rotation.cos() - right * p.rotation.sin();
            let right = right2 * p.size * size_mul * 0.5;
            let up = up2 * p.size * size_mul * 0.5;
            let corners = [
                at - right - up,
                at + right - up,
                at + right + up,
                at - right + up,
            ];
            let mut uv = [vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)];
            for uv in &mut uv {
                if p.kind & (1 << 18) != 0 {
                    uv.x = 1. - uv.x;
                }
                if p.kind & (1 << 19) != 0 {
                    uv.y = 1. - uv.y;
                }
            }
            let material = if (p.kind >> 22) & 3 == 1 {
                &self.additive
            } else {
                &self.alpha
            };
            let mut color = p.color;
            color.w *= alpha_mul;
            material.set_uniform("Primary", color.to_array());
            material.set_uniform(
                "Environment",
                if p.kind & 128 != 0 {
                    p.environment.to_array()
                } else {
                    [0.; 4]
                },
            );
            gl_use_material(material);
            draw_mesh(&Mesh {
                vertices: corners
                    .iter()
                    .zip(uv)
                    .map(|(p, uv)| Vertex::new(p.x, p.y, p.z, uv.x, uv.y, WHITE))
                    .collect(),
                indices: vec![0, 1, 2, 0, 2, 3],
                texture: Some(texture.clone()),
            });
        }
        gl_use_default_material();
    }
}
