use macroquad::{
    miniquad::{BlendFactor, BlendState, BlendValue, Equation},
    prelude::*,
};
pub fn blended(additive: bool) -> libssbm_assets::Result<Material> {
    Ok(load_material(
        ShaderSource::Glsl {
            vertex: r#"#version 100
attribute vec3 position;attribute vec2 texcoord;attribute vec4 color0;
varying lowp vec2 uv;varying lowp vec4 tint;uniform mat4 Model;uniform mat4 Projection;
void main(){gl_Position=Projection*Model*vec4(position,1.0);uv=texcoord;tint=color0/255.0;}
"#,
            fragment: r#"#version 100
precision lowp float;varying vec2 uv;varying vec4 tint;uniform sampler2D Texture;
void main(){gl_FragColor=texture2D(Texture,uv)*tint;}
"#,
        },
        MaterialParams {
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
    )?)
}
