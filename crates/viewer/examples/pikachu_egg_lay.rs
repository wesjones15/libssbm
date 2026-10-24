//! Local-disc preview for Pikachu's borrowed Egg Lay:
//! `cargo run --release -p ssbm-viewer --example pikachu_egg_lay -- assets [out.png] [clip] [frame] [pitch-deg]`
//!
//! `SpecialLwStart` is Pikachu's Thunder start, where he tenses up; the borrowed Egg Lay
//! uses it for both its tongue and swallow stages. The upper body pitches with camera
//! elevation and the tongue leaves the forward snout joint (16).
use libssbm_assets::{fighter, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf {
    Conf { window_title: "Pikachu Egg Lay pose".into(), window_width: 1000, window_height: 760, ..Default::default() }
}

fn draw(p: &Presentation, transform: ::glam::Mat4, alpha: &Material, additive: &Material) {
    for (i, b) in p.model.batches.iter().enumerate() {
        if !p.visible(i) { continue; }
        let texture = p.texture(i).map(|t| {
            let tex = Texture2D::from_rgba8(t.width, t.height, &t.rgba);
            tex.set_filter(FilterMode::Linear);
            let wrap = |n| match n { 1 => miniquad::TextureWrap::Clamp, 2 => miniquad::TextureWrap::Mirror, _ => miniquad::TextureWrap::Repeat };
            unsafe { get_internal_gl().quad_context.texture_set_wrap(tex.raw_miniquad_id(), wrap(t.wrap_s), wrap(t.wrap_t)); }
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
    let args: Vec<_> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let capture = args.get(1).cloned();
    let clip = args.get(2).cloned().unwrap_or_else(|| "SpecialLwStart".into());
    let frame = args.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(15.);
    let pitch = args.get(4).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.).to_radians();
    let mut view = Presentation::load_for(dir, 0, &fighter::PIKACHU).unwrap();
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let Some(c) = view.animations.clips.get(&clip) else { panic!("no clip {clip}") };
    let f = frame.min(c.frames);
    view.clip = clip.clone();
    view.frame = f;
    view.world = view.animations.pose(&view.model, &clip, f, false);
    if let Some(root) = fighter::PIKACHU.upper_body_root {
        libssbm_assets::presentation::pitch_upper_body(&view.model, &mut view.world, root, pitch);
    }
    let mouth = view.world[16].transform_point3(::glam::Vec3::ZERO);
    let aim = ::glam::Vec3::new(0., pitch.sin(), pitch.cos());
    let tip = mouth + aim * 16.;
    println!("{clip} f{f:.0}: mouth {mouth:?} pitch {:.3} frames {}", pitch, c.frames);
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35, 40, 55, 255));
        set_camera(&Camera3D {
            position: vec3(0., 5.0, 12.0),
            target: vec3(0., 4.0, 0.),
            up: Vec3::Y,
            fovy: 40.,
            ..Default::default()
        });
        draw_plane(vec3(0., -0.2, 0.), vec2(80., 80.), None, Color::from_rgba(55, 60, 78, 255));
        draw(&view, ::glam::Mat4::IDENTITY, &alpha, &additive);
        draw_line_3d(vec3(mouth.x, mouth.y, mouth.z), vec3(tip.x, tip.y, tip.z), Color::new(0.92, 0.27, 0.36, 0.95));
        set_default_camera();
        draw_text(
            &format!("Pikachu Egg Lay | {clip} frame {f:.0} | mouth joint 16 | pitch {:.0} deg", pitch.to_degrees()),
            14., 28., 21., WHITE,
        );
        draw_text("Borrowed Yoshi Egg Lay: Pikachu tenses with his Thunder start", 14., screen_height() - 16., 18., LIGHTGRAY);
        rendered += 1;
        if rendered == 3 && let Some(path) = capture.as_deref() {
            get_screen_data().export_png(path);
            println!("wrote {path}");
            break;
        }
        if is_key_pressed(KeyCode::Escape) { break; }
        next_frame().await;
    }
}
