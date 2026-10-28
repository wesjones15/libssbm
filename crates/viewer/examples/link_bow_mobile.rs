//! Viewer-first proof: Link's source slot-4 bow on the shared running/pitched rig.
//! cargo run --release -p ssbm-viewer --example link_bow_mobile -- assets build/link-bow-mobile.png
use libssbm::{FlatHost, Input, Sim, NEUTRAL_B, SPRINT};
use libssbm_assets::{fighter::LINK, held, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;
fn config() -> Conf { Conf { window_title: "Link running bow and camera pitch".into(), window_width: 1200, window_height: 820, ..Default::default() } }
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
    let mut samples = Vec::new();
    for degrees in [-45f32, 0., 45.] {
        let params = libssbm_assets::params::load_common(dir, &LINK).unwrap();
        let specials = libssbm_assets::params::load_link_specials(dir).unwrap();
        let mut sim = Sim::with_behavior(params, Box::new(libssbm::link::Link::new(specials)), ::glam::Vec3::ZERO);
        sim.motion.definitions = fighter.animations.motion_definitions();
        sim.action.definitions = libssbm_assets::action::load_for(dir, &fighter.animations, &LINK).unwrap();
        for (name, action) in &sim.action.definitions {
            if let Some(motion) = sim.motion.definitions.get_mut(name) { motion.looping |= action.looping; }
        }
        let pitch = degrees.to_radians();
        let pad = Input { buttons: NEUTRAL_B|SPRINT, movement: ::glam::Vec3::Z,
            aim: ::glam::Vec3::new(0.,pitch.sin(),pitch.cos()), ..Input::idle() };
        let mut host = FlatHost::default(); sim.grounded = true; sim.facing = ::glam::Vec3::Z;
        for tick in 1..=100 {
            sim.advance(&mut host, pad, 1./60.);
            if ![90,100].contains(&tick) { continue; }
            fighter.update(&sim, 0.);
            let state = held::link_bow_state(sim.motion.clip,sim.motion.frame).unwrap();
            let mut bow = Presentation::article_for(dir,held::LINK_BOW,state,&LINK).unwrap();
            bow.attach_to(sim.motion.frame, fighter.world[bones[held::LINK_BOW_PART] as usize], scale).unwrap();
            println!("pitch {degrees:+} tick {tick}: {} / {} speed {:.3} travel {:.3}",sim.motion.clip,sim.locomotion.clip,sim.velocity.z,sim.position.z);
            samples.push((bow,fighter.world.clone(),degrees,tick,sim.position.z));
        }
    }
    let alpha = models::blended(false).unwrap(); let additive = models::blended(true).unwrap();
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35,40,55,255));
        for (i,(bow,pose,degrees,tick,travel)) in samples.iter().enumerate() {
            let col = i/2; let row = i%2;
            set_camera(&Camera3D { position: vec3(32.,13.,25.),target: vec3(0.,10.,1.),up: Vec3::Y,
                viewport: Some((col as i32*400,(1-row) as i32*380+60,400,380)),..Default::default() });
            fighter.world = pose.clone(); draw(&fighter,::glam::Mat4::IDENTITY,&alpha,&additive);
            draw(bow,::glam::Mat4::IDENTITY,&alpha,&additive); set_default_camera();
            draw_text(&format!("aim {degrees:+} deg | run tick {tick}"),col as f32*400.+10.,row as f32*380.+25.,20.,WHITE);
            draw_text(&format!("travel {travel:.1} | held charge"),col as f32*400.+10.,row as f32*380.+48.,18.,GRAY);
        }
        draw_text("PlLk.dat slot 4 | bow article 3 | RThumbNb | shared Run legs + upper pitch",12.,780.,21.,YELLOW);
        draw_text("ftlinkspecialn.c::isDrawback; itlinkbow.c | running and camera pitch are host adaptations",12.,807.,19.,WHITE);
        if rendered == 2 { if let Some(path) = args.get(1) { get_screen_data().export_png(path); } break; }
        rendered += 1; next_frame().await;
    }
}
