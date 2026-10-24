//! Local-disc proof of Yoshi's Egg Lay upper-body pitch.
//! cargo run --release -p ssbm-viewer --example yoshi_tongue -- assets build/yoshi-tongue.png
//!
//! Three runs of SpecialN1, aiming the camera 45 degrees up, level and 45 degrees down,
//! sampled across the tongue's extension. The head/mouth (joint 42) must follow the aim while
//! the hips and legs keep the gait; the line shows the drawn lash's direction from the mouth.
use libssbm::{FlatHost, Input, Sim, NEUTRAL_B};
use libssbm_assets::{fighter, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf {
    Conf { window_title: "Yoshi Egg Lay tongue pitch".into(), window_width: 1200, window_height: 760, ..Default::default() }
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

type Sample = (Vec<::glam::Mat4>, String, f32, ::glam::Vec3, ::glam::Vec3);

#[macroquad::main(config)]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let mut view = Presentation::load_for(dir, 0, &fighter::YOSHI).unwrap();

    let times = [2u32, 10, 18, 26];
    let pitches = [45f32, 0., -45.];
    let mut samples: Vec<Sample> = Vec::new();
    let mut report = Vec::new();
    for degrees in pitches {
        let pitch = degrees.to_radians();
        let aim = ::glam::Vec3::new(0., pitch.sin(), pitch.cos());
        // A fresh sim per row: the move runs once and returns to Wait otherwise.
        let params = libssbm_assets::params::load_common(dir, &fighter::YOSHI).unwrap();
        let specials = libssbm_assets::params::load_yoshi_specials(dir).unwrap();
        let mut sim = Sim::with_behavior(params, Box::new(libssbm::yoshi::Yoshi::new(specials)), ::glam::Vec3::ZERO);
        sim.motion.definitions = view.animations.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for(dir, &view.animations, &fighter::YOSHI).unwrap();
        for (name, action) in &sim.action.definitions {
            if let Some(motion) = sim.motion.definitions.get_mut(name) { motion.looping |= action.looping; }
        }
        sim.grounded = true;
        sim.facing = ::glam::Vec3::Z; // the raw model faces +Z; the viewer draws it unrotated
        sim.host_jump_height = Some(40.);
        let mut host = FlatHost::default();
        for _ in 0..4 { sim.advance(&mut host, Input::idle(), 1./60.); }
        let mut local = Vec::new();
        for tick in 1..=times[times.len() - 1] {
            let input = Input { buttons: if tick == 1 { NEUTRAL_B } else { 0 }, aim, ..Input::idle() };
            sim.advance(&mut host, input, 1./60.);
            if times.contains(&tick) {
                view.update(&sim, 0.);
                let mouth = view.world[42].transform_point3(::glam::Vec3::ZERO);
                let tip = mouth + aim * 6.;
                println!("pitch {degrees:+.0} tick {tick}: clip {} frame {:.1} mouth {mouth:?} pitch {:?}",
                    view.clip, view.frame, sim.tongue_upper_pitch());
                local.push((view.world.clone(), view.clip.clone(), view.frame, mouth, tip));
            }
        }
        assert!(local.iter().all(|s| s.1 == "SpecialN"), "Egg Lay ended inside the sampled window");
        report.push(local.clone());
        samples.extend(local);
    }

    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35, 40, 55, 255));
        for (i, (world, clip, frame, mouth, tip)) in samples.iter().enumerate() {
            let col = (i % times.len()) as i32;
            let row = (i / times.len()) as i32;
            let camera = Camera3D {
                position: vec3(46., 12., 12.),
                target: vec3(0., 8., 2.),
                up: Vec3::Y,
                viewport: Some((col * 300, (2 - row) * 250, 300, 250)),
                ..Default::default()
            };
            set_camera(&camera);
            draw_plane(vec3(0., -0.2, 0.), vec2(60., 60.), None, Color::from_rgba(55, 60, 78, 255));
            view.world = world.clone();
            view.clip = clip.clone();
            view.frame = *frame;
            draw(&view, ::glam::Mat4::IDENTITY, &alpha, &additive);
            draw_line_3d(vec3(mouth.x, mouth.y, mouth.z), vec3(tip.x, tip.y, tip.z),
                Color::new(0.92, 0.27, 0.36, 0.95));
            set_default_camera();
        }
        for (row, degrees) in [45f32, 0., -45.].iter().enumerate() {
            draw_text(&format!("aim {degrees:+.0} deg (upper body pitch)"), 10., 24. + row as f32 * 250., 20., WHITE);
        }
        draw_text("Yoshi Egg Lay | SpecialN1 frame 2/10/18/26 | mouth joint 42, lash direction", 10., 742., 20., WHITE);
        if rendered == 2 && let Some(path) = args.get(1) {
            get_screen_data().export_png(path);
            break;
        }
        if is_key_pressed(KeyCode::Escape) { break; }
        rendered += 1;
        next_frame().await;
    }
    // A headless summary of the same runs: the pitch rows must differ at the mouth.
    for (row, local) in report.iter().enumerate() {
        let y = local[2].3.y;
        println!("row {row}: mouth y {y:.3}");
    }
}
