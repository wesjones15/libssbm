//! Local-disc proof of Samus's ball pose and the held roll lifecycle.
//! cargo run --release -p ssbm-viewer --example morph_roll -- assets build/morph-roll.png raw
use libssbm_assets::{presentation::Presentation, fighter};
use libssbm::{Sim, Input, FlatHost, DOWN_B};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;
fn config() -> Conf { Conf { window_title: "Samus Morph Ball Roll proof".into(), window_width: 1200, window_height: 720, ..Default::default() } }
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
    let raw = args.get(2).is_some_and(|s| s == "raw");
    let mut view = Presentation::load(dir, 0).unwrap();
    let params = libssbm_assets::params::load_common(dir, &fighter::SAMUS).unwrap();
    let specials = libssbm_assets::params::load_samus_specials(dir).unwrap();
    let mut sim = Sim::with_behavior(params, Box::new(libssbm::samus::Samus::new(specials, fighter::SAMUS.upper_body_root)), ::glam::Vec3::ZERO);
    sim.motion.definitions = view.animations.motion_definitions();
    sim.action.definitions = libssbm_assets::action::load(dir, &view.animations).unwrap();
    let mut host = FlatHost::default();
    sim.advance(&mut host, Input::idle(), 1./60.);
    let air = args.iter().any(|s| s == "--air");
    if air { sim.teleport(::glam::Vec3::Y * 1000.); }
    sim.host_flags = libssbm::samus::HOST_MORPH_ROLL;
    let mut samples = Vec::new();
    let mut trail = Vec::new();
    let times = [1, 7, 24, 55, 95, 96];
    for tick in 1..=96 {
        let breaking = args.get(2).is_some_and(|s| s == "break");
        if breaking && tick == 96 { sim.roll_break = true; }
        let buttons = if tick < 96 || breaking { DOWN_B } else { 0 };
        sim.advance(&mut host, Input { buttons, movement: ::glam::Vec3::Z, aim: ::glam::Vec3::Z, ..Input::idle() }, 1./60.);
        for cue in sim.action.effects.drain(..) {
            if cue.id == 1023 { trail.push((tick, cue.origin)); }
        }
        if times.contains(&tick) {
            view.update(&sim, 0.);
            if raw {
                let frame = [0., 7., 13., 30., 54., 0.][samples.len()];
                view.ball = samples.len() < 5;
                view.clip = if view.ball { "SpecialLw" } else { "Wait" }.into();
                view.frame = frame;
                view.world = view.animations.pose(&view.model, &view.clip, frame, false);
            }
            println!("tick {tick}: {} frame {} ball {}",view.clip,view.frame,view.ball);
            if samples.len()==2 { for (i,b) in view.model.batches.iter().enumerate() { if view.visible(i) { let points=view.model.skin(i,&view.world); let lo=points.iter().fold(::glam::Vec3::splat(f32::INFINITY),|v,(p,_)|v.min(*p)); let hi=points.iter().fold(::glam::Vec3::splat(f32::NEG_INFINITY),|v,(p,_)|v.max(*p)); println!("ball batch {i} DObj {} bounds {lo:?} {hi:?}",b.dobj); } } }
            let dust = trail.iter().filter(|(birth,_)| tick-birth < 12)
                .map(|(birth,at)| ((tick-birth) as f32, *at-sim.position)).collect::<Vec<_>>();
            samples.push((view.world.clone(),view.clip.clone(),view.frame,view.ball,dust));
        }
    }
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut dust = Presentation::effect(dir, 5).unwrap();
    let mut rendered=0;
    loop {
        clear_background(Color::from_rgba(35,40,55,255));
        for (i,(world,clip,frame,ball,trail)) in samples.iter().enumerate() {
            let col=(i%3) as i32; let row=(i/3) as i32;
            let camera=Camera3D { position: vec3(32.,14.,27.), target: vec3(0.,7.,0.), up:Vec3::Y, viewport:Some((col*400,(1-row)*330,400,330)), ..Default::default() };
            set_camera(&camera);
            draw_plane(vec3(0.,if air {-20.} else {-0.2},0.),vec2(40.,40.),None,Color::from_rgba(60,65,80,255));
            view.world=world.clone(); view.clip=clip.clone(); view.frame=*frame; view.ball=*ball;
            draw(&view,::glam::Mat4::IDENTITY,&alpha,&additive);
            if !raw { for (age,at) in trail {
                dust.object_pose(*age);
                draw(&dust,::glam::Mat4::from_translation(*at),&alpha,&additive);
            } }
            set_default_camera();
            draw_text(&format!("tick {} | {} f{frame:.1}",times[i],clip),col as f32*400.+12.,row as f32*330.+88.,19.,WHITE);
        }
        draw_text("Samus: PlSsNr.dat DObj 19 / PlSsAJ.dat SpecialLw / choice 1",12.,24.,23.,WHITE);
        draw_text("Source: ftsamusspeciallw1.c::ftSs_SpecialLw_Enter; ftparts.c::ftParts_80074B0C",12.,48.,19.,WHITE);
        draw_text("Trail: efasync.c case 0x3FF -> EfCoData model 5 (ground + air)",12.,708.,19.,WHITE);
        if rendered==2 { if let Some(path)=args.get(1) { get_screen_data().export_png(path); break; } }
        rendered+=1; next_frame().await;
    }
}
