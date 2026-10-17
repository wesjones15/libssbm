use libssbm_assets::effects::Effects;
use macroquad::{
    miniquad::{BlendFactor, BlendState, BlendValue, Equation},
    prelude::*,
};
pub struct Renderer {
    textures: Vec<Vec<Vec<Texture2D>>>,
    alpha: Material,
    additive: Material,
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
        })
    }
    pub fn draw(&self, system: &libssbm::particle::System, camera: Vec3) {
        for p in &system.particles {
            if p.kind & (1 << 10) == 0 {
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
            let at = p.transform.transform_point3(p.position);
            let at = vec3(at.x, at.y, at.z);
            let look = (camera - at).normalize();
            let right = Vec3::Y.cross(look).normalize();
            let up = look.cross(right);
            let right2 = right * p.rotation.cos() + up * p.rotation.sin();
            let up2 = up * p.rotation.cos() - right * p.rotation.sin();
            let right = right2 * p.size * 0.5;
            let up = up2 * p.size * 0.5;
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
            material.set_uniform("Primary", p.color.to_array());
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
