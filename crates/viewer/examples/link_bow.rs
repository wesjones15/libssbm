//! Link bow proof: PlLk.dat item slot 4 (It_Kind_Link_Bow), public article 3.
//! Spec: ftLink/ftlinkspecialn.c::isDrawback and it/kinds/itlinkbow.c::it_803F6E98.
//! cargo run --release -p ssbm-viewer --example link_bow -- assets build/link-bow.png
use libssbm_assets::{fighter::LINK, held, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;
fn config() -> Conf { Conf { window_title: "Link bow - slot 4 proof".into(), window_width: 1200, window_height: 820, ..Default::default() } }
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
    let args: Vec<_> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let mut fighter = Presentation::load_for(dir, 0, &LINK).unwrap();
    let bones = held::common_bones(dir, &LINK).unwrap();
    let scale = held::model_scale(dir, &LINK).unwrap();
    let samples = [("SpecialN", 0.), ("SpecialN", 10.), ("SpecialN", 18.),
        ("SpecialNLoop", 8.), ("SpecialNEnd", 8.), ("SpecialAirNLoop", 8.)];
    let mut views = Vec::new();
    for (clip, frame) in samples {
        let state = held::link_bow_state(clip, frame).unwrap();
        let mut bow = Presentation::article_for(dir, held::LINK_BOW, state, &LINK).unwrap();
        assert!(fighter.animations.clips.contains_key(clip), "missing fighter clip {clip}");
        let pose = fighter.animations.pose(&fighter.model, clip, frame, false);
        let hand = pose[bones[held::LINK_BOW_PART] as usize];
        bow.attach_to(frame, hand, scale).unwrap();
        println!("{clip} frame {frame}: bow state {state}, attach joint {:?}, RThumbNb physical joint {}, scale {scale}, batches {}", bow.attach_joint, bones[held::LINK_BOW_PART], bow.model.batches.len());
        views.push((bow, pose));
    }
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35, 40, 55, 255));
        for (i, (bow, pose)) in views.iter().enumerate() {
            set_camera(&Camera3D { position: vec3(37., 16., 30.), target: vec3(0., 9., 1.), up: Vec3::Y,
                viewport: Some(((i%3) as i32*400, (1-i/3) as i32*380+60, 400, 380)), ..Default::default() });
            fighter.world = pose.clone();
            fighter.clip = samples[i].0.into(); fighter.frame = samples[i].1;
            draw(&fighter, ::glam::Mat4::IDENTITY, &alpha, &additive);
            draw(bow, ::glam::Mat4::IDENTITY, &alpha, &additive);
            set_default_camera();
            draw_text(&format!("{} f{} | bow {}", samples[i].0, samples[i].1, held::link_bow_state(samples[i].0, samples[i].1).unwrap()),
                (i%3) as f32*400.+10., (i/3) as f32*380.+27., 20., WHITE);
        }
        draw_text("PlLk.dat slot 4 | It_Kind_Link_Bow | article 3 | RThumbNb (part 49)", 12., 780., 22., YELLOW);
        draw_text("ftlinkspecialn.c::isDrawback; itlinkbow.c::it_803F6E98 (ground/air draw, hold, release)", 12., 807., 20., WHITE);
        if rendered == 2 { if let Some(path) = args.get(1) { get_screen_data().export_png(path); } break; }
        rendered += 1; next_frame().await;
    }
}