//! Donkey Kong's tie: render the Wait (idle) pose to show the tie's orientation,
//! against the bind pose for comparison. Capture with:
//!   cargo run --release -p ssbm-viewer --example dk_tie -- assets build/dk-tie.png wait
//! (last arg: "bind" or "wait"; also accepts "walk" for the Walk clip.)
use libssbm_assets::{presentation::Presentation, fighter::DONKEY};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf { Conf { window_title: "DK tie proof".into(), window_width: 900, window_height: 900, ..Default::default() } }

fn draw(p: &Presentation, transform: ::glam::Mat4, alpha: &Material, additive: &Material) {
    for (i, b) in p.model.batches.iter().enumerate() {
        if !p.visible(i) { continue; }
        let texture = p.texture(i).map(|t| {
            let tex = Texture2D::from_rgba8(t.width, t.height, &t.rgba);
            tex.set_filter(FilterMode::Linear);
            tex
        });
        let c = p.color(i);
        let vertices = p.model.skin(i, &p.world).iter().zip(&b.vertices).map(|((at, normal), v)| {
            let at = transform.transform_point3(*at);
            let uv = p.texture_matrix(i).transform_point3(v.uv.extend(0.));
            let light = if b.lit == 0 { 1. } else { 0.5 + 0.5 * normal.dot(::glam::Vec3::new(0.4, 0.8, 0.5).normalize()).max(0.) };
            Vertex::new(at.x, at.y, at.z, uv.x, uv.y, Color::new(c[0]*light, c[1]*light, c[2]*light, c[3]))
        }).collect();
        if b.blend >= 2 { gl_use_material(if b.blend == 3 { additive } else { alpha }); }
        draw_mesh(&Mesh { vertices, indices: b.indices.iter().map(|&i| i as u16).collect(), texture });
        gl_use_default_material();
    }
}

#[macroquad::main(config)]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let mut view = Presentation::load_for(dir, 0, &DONKEY).unwrap();
    let mode = args.get(2).map(String::as_str).unwrap_or("wait");
    if mode == "bind" {
        view.clip = "Wait".into();
        view.frame = 0.;
        view.world = view.model.bind_world();
    } else if mode == "walk" {
        view.clip = "Walk1".into();
        view.frame = 0.;
        view.world = view.animations.pose(&view.model, "Walk1", 0., false);
    } else {
        view.clip = "Wait".into();
        view.frame = 0.;
        view.world = view.animations.pose(&view.model, "Wait", 0., false);
    }
    // Side view so the tie's forward/back orientation is obvious.
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35, 40, 55, 255));
        let camera = Camera3D { position: vec3(0., 16., 42.), target: vec3(0., 16., 0.), up: Vec3::Y, ..Default::default() };
        set_camera(&camera);
        draw_plane(vec3(0., -0.2, 0.), vec2(60., 60.), None, Color::from_rgba(60, 65, 80, 255));
        draw(&view, ::glam::Mat4::IDENTITY, &alpha, &additive);
        set_default_camera();
        draw_text(&format!("DK tie: {} (DObj 33 skinned to joints 67/68/70)", mode), 16., 24., 22., WHITE);
        if rendered == 2 {
            if let Some(path) = args.get(1) { get_screen_data().export_png(path); }
            break;
        }
        rendered += 1;
        next_frame().await;
    }
}
