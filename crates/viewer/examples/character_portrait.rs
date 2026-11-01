//! Character portrait proof, before the RoR2 adapter. No effect/emitter is used.
//! Models: Samus PlSsNr.dat/PlSsAJ.dat, DK PlDkNr.dat/PlDkAJ.dat, costume 0.
//! Spec: ftSamus/ftsamus.c::ftSs_Init_CostumeStrings/ftSs_Init_AnimDatFilename;
//! ftDonkey/ftdonkey.c::ftDk_Init_CostumeStrings/ftDk_Init_AnimDatFilename;
//! ftCommon/ftCo_Wait.c::ftCo_Wait_Anim. Existing host idle-only menu source poses Wait at 0.
//! Mirrors PortraitFraming.ThreeQuarter's upper-half crop, 45-degree left orbit and zoom.
//! cargo run --release -p ssbm-viewer --example character_portrait -- assets build/character-portraits.png
use libssbm_assets::{fighter, portrait, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf {
    Conf { window_title: "Melee character portrait proof".into(), window_width: 1280, window_height: 820, ..Default::default() }
}
fn gv(v: ::glam::Vec3) -> Vec3 { vec3(v.x, v.y, v.z) }
fn draw(p: &Presentation, alpha: &Material, additive: &Material) {
    for (i, b) in p.model.batches.iter().enumerate() {
        if !p.visible(i) { continue; }
        let texture = p.texture(i).map(|t| {
            let tex = Texture2D::from_rgba8(t.width, t.height, &t.rgba);
            tex.set_filter(FilterMode::Linear);
            tex
        });
        let c = p.color(i);
        let vertices = p.model.skin(i, &p.world).iter().zip(&b.vertices).map(|((at, normal), v)| {
            let uv = p.texture_matrix(i).transform_point3(v.uv.extend(0.));
            let light = if b.lit == 0 { 1. } else { 0.5 + 0.5 * normal.dot(::glam::Vec3::new(-0.4, 0.8, 0.5).normalize()).max(0.) };
            Vertex::new(at.x, at.y, at.z, uv.x, uv.y, Color::new(c[0]*light, c[1]*light, c[2]*light, c[3]))
        }).collect();
        if b.blend >= 2 { gl_use_material(if b.blend == 3 { additive } else { alpha }); }
        draw_mesh(&Mesh { vertices, indices: b.indices.iter().map(|&i| i as u16).collect(), texture });
        gl_use_default_material();
    }
}
fn camera(p: &Presentation, x: i32, y: i32) -> Camera3D {
    let points: Vec<_> = p.model.batches.iter().enumerate().filter(|(i, _)| p.visible(*i))
        .flat_map(|(i, _)| p.model.skin(i, &p.world).into_iter().map(|(p, _)| p)).collect();
    let top = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    let bottom = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let h = top - bottom;
    let upper: Vec<_> = points.into_iter().filter(|p| p.y >= top - h * 0.5).collect();
    let lo = upper.iter().fold(::glam::Vec3::splat(f32::INFINITY), |a, p| a.min(*p));
    let hi = upper.iter().fold(::glam::Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
    let bounds = hi - lo;
    let across = bounds.x.max(bounds.z).max((bounds.x + bounds.z) * std::f32::consts::FRAC_1_SQRT_2);
    let view = (h * 0.5).max(across) * 1.08 / 1.14;
    let mid = (lo + hi) * 0.5;
    let center = ::glam::Vec3::new(mid.x, top + view*0.04 - view*0.5 + view*0.02, mid.z);
    let distance = view*0.5 / (20f32.to_radians()*0.5).tan() + (bounds.x*0.5).max(bounds.z*0.5);
    // ModelView mirrors native X for Unity. Its -45-degree camera orbit is
    // therefore +45 degrees here in native coordinates.
    let orbit = ::glam::Vec3::new(1., 0., 1.).normalize();
    Camera3D { position: gv(center + orbit * distance), target: gv(center), up: Vec3::Y,
        fovy: 20f32.to_radians(), aspect: Some(1.), viewport: Some((x, y, 256, 256)), ..Default::default() }
}
#[macroquad::main(config)]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let out = args.get(1).map(String::as_str).unwrap_or("build/character-portraits.png");
    let mut views: Vec<_> = [&fighter::SAMUS, &fighter::DONKEY].into_iter()
        .map(|f| Presentation::load_for(dir, 0, f).unwrap()).collect();
    let photos: Vec<_> = [&fighter::SAMUS, &fighter::DONKEY].into_iter().map(|f| {
        let p = portrait::load_grid(dir, f).unwrap();
        let tex = Texture2D::from_rgba8(p.width, p.height, &p.rgba); tex.set_filter(FilterMode::Linear); tex
    }).collect();
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    for rendered in 0..3 {
        clear_background(Color::from_rgba(23, 28, 38, 255));
        draw_text("Character portraits: existing fallback / generated idle, 45 degrees left", 25., 32., 26., WHITE);
        for (row, p) in views.iter_mut().enumerate() {
            let top = 90 + row as i32 * 340;
            draw_text(if row == 0 { "Samus - PlSsNr / PlSsAJ" } else { "DK - PlDkNr / PlDkAJ" }, 25., (top-15) as f32, 21., WHITE);
            draw_texture_ex(&photos[row], 25., top as f32, WHITE,
                DrawTextureParams { dest_size: Some(vec2(256., 256.)), ..Default::default() });
            draw_text("Current CSS image", 25., (top+280) as f32, 20., GRAY);
            for (column, frame) in [0., 15., 30.].into_iter().enumerate() {
                p.clip = "Wait".into(); p.frame = frame;
                p.world = p.animations.pose(&p.model, "Wait", frame, false);
                let x = 345 + column as i32 * 300;
                set_camera(&camera(p, x, 820-top-256));
                draw(p, &alpha, &additive);
                set_default_camera();
                draw_text(&format!("Wait frame {frame:.0}"), x as f32, (top+280) as f32, 20., GRAY);
            }
        }
        draw_text("Source: ftSamus/ftsamus.c + ftDonkey/ftdonkey.c; ftCommon/ftCo_Wait.c::ftCo_Wait_Anim", 25., 795., 18., GRAY);
        if rendered == 2 { get_screen_data().export_png(out); println!("wrote {out}"); break; }
        next_frame().await;
    }
}
