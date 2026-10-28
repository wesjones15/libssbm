//! Local-disc preview: cargo run --release -p ssbm-viewer --example scope_dust -- assets [capture.png] [frame] [pitch-degrees]
//! The Start script's real dust timing (frames 24..88, every 8) using the original common ground-smoke model.
use libssbm_assets::{presentation::Presentation, fighter, dat::Dat};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;
fn config() -> Conf { Conf { window_title: "Super Scope ground smoke".into(), window_width: 960, window_height: 720, ..Default::default() } }
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
    let mut luigi = Presentation::load_for(dir, 0, &fighter::LUIGI).unwrap();
    luigi.clip = "ItemScopeStart".into();
    luigi.frame = 15.;
    luigi.world = luigi.animations.pose(&luigi.model, &luigi.clip, 15., false);
    if let Some(jump) = args.get(4) {
        let lower = luigi.animations.pose(&luigi.model, jump, 7., false);
        luigi.world = libssbm_assets::presentation::splice_pose(&luigi.model, &lower,
            &luigi.world, fighter::LUIGI.upper_body_root.unwrap());
    }
    let pitch = args.get(3).map(|s| s.parse::<f32>().unwrap()).unwrap_or(0.).to_radians();
    libssbm_assets::presentation::pitch_upper_body(&luigi.model, &mut luigi.world,
        fighter::LUIGI.upper_body_root.unwrap(), libssbm::scope_aim_pitch(::glam::Vec3::new(0., pitch.sin(), pitch.cos())));
    let mut scope = Presentation::item_for(dir, libssbm_assets::article::Item::SuperScope, 0).unwrap();
    scope.object_pose(0.);
    let bytes = std::fs::read(dir.join(fighter::COMMON_DATA.file)).unwrap();
    let common = Dat::parse(&bytes).unwrap();
    let table = common.required_ptr(common.root(fighter::COMMON_DATA.root).unwrap() + 0x10).unwrap();
    let parts = common.required_ptr(table + fighter::LUIGI.common_index * 4).unwrap();
    let mapping = common.required_ptr(parts + 4).unwrap();
    let hand = common.bytes(mapping + 49, 1).unwrap()[0] as usize;
    let mut dust = Presentation::effect(dir, 5).unwrap();
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let fixed = args.get(2).map(|a| a.parse::<f32>().unwrap());
    let mut rendered = 0;
    loop {
        let frame = fixed.unwrap_or((get_time() as f32 * 60.) % 120.);
        clear_background(Color::from_rgba(75, 70, 125, 255));
        set_camera(&Camera3D { position: vec3(48., 16., 18.), target: vec3(0., 8., 0.), up: Vec3::Y, ..Default::default() });
        draw_plane(vec3(0., -0.2, 0.), vec2(200., 200.), None, Color::from_rgba(55, 49, 112, 255));
        draw(&luigi, ::glam::Mat4::IDENTITY, &alpha, &additive);
        draw(&scope, luigi.world[hand], &alpha, &additive);
        // Both raw models face +Z here; the host rotates both toward the sim facing.
        // ItemScopeStart's script: cue 0x3FF every 8 frames from 24 to 88, scattered by +/-6 across and along the
        // fighter around (0, 0, -1.95). The viewer's spread is a fixed sequence, the sim's is random.
        for (n, birth) in (24..=88u32).step_by(8).enumerate().filter(|(_, b)| *b as f32 <= frame) {
            let age = frame - birth as f32;
            if age >= dust.duration { continue; }
            dust.object_pose(age);
            let spread = |k: f32| ((n as f32 * k).sin() * 6.).clamp(-6., 6.);
            let placement = ::glam::Mat4::from_translation(::glam::Vec3::new(spread(2.3), 0.2, -1.95 + spread(1.7)));
            draw(&dust, placement, &alpha, &additive);
        }
        set_default_camera();
        draw_text("Super Scope charge | EfCoData model 5 / cue 0x3FF", 20., 32., 24., WHITE);
        if rendered == 2 && let Some(path) = args.get(1) {
            get_screen_data().export_png(path); break;
        }
        if is_key_pressed(KeyCode::Escape) { break; }
        rendered += 1;
        next_frame().await;
    }
}
