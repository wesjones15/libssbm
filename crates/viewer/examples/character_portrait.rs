//! Character portrait proof, before the RoR2 adapter. No effect/emitter is used.
//! Models: each fighter's PlXXNr.dat/PlXXAJ.dat, costume 0, posed at Wait 0.
//! Spec: ftSamus/ftsamus.c::ftSs_Init_CostumeStrings/ftSs_Init_AnimDatFilename;
//! ftCommon/ftCo_Wait.c::ftCo_Wait_Anim. Existing host idle-only menu source poses Wait at 0.
//! Top half: PortraitFraming.ThreeQuarter as it was (upper-half crop, 45-degree left
//! orbit, 1.14 zoom). Bottom half: the head-and-shoulders crop as a fraction of the
//! fighter's height, with the per-fighter pan. The fractions mirror
//! `MeleeSurvivor.PortraitFramingFor` in ror2-common; keep them in sync.
//! cargo run --release -p ssbm-viewer --example character_portrait -- assets build/character-portraits.png
use libssbm_assets::{fighter, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf {
    Conf { window_title: "Melee character portrait proof".into(), window_width: 1280, window_height: 1300, ..Default::default() }
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

/// The same framing controls PortraitFraming exposes: 45-degree turn, an optional
/// visible height as a fraction of the fighter's full height, and a vertical pan.
#[derive(Clone, Copy)]
struct Frame { zoom: f32, height: Option<f32>, pan: f32 }

struct Bounds { top: f32, h: f32, across: f32, mid_x: f32, mid_z: f32, depth: f32 }

fn bounds(p: &Presentation) -> Bounds {
    let points: Vec<_> = p.model.batches.iter().enumerate().filter(|(i, _)| p.visible(*i))
        .flat_map(|(i, _)| p.model.skin(i, &p.world).into_iter().map(|(p, _)| p)).collect();
    let top = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    let bottom = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let h = top - bottom;
    let upper: Vec<_> = points.into_iter().filter(|p| p.y >= top - h * 0.5).collect();
    let lo = upper.iter().fold(::glam::Vec3::splat(f32::INFINITY), |a, p| a.min(*p));
    let hi = upper.iter().fold(::glam::Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
    let size = hi - lo;
    let across = size.x.max(size.z).max((size.x + size.z) * std::f32::consts::FRAC_1_SQRT_2);
    let mid = (lo + hi) * 0.5;
    Bounds { top, h, across, mid_x: mid.x, mid_z: mid.z, depth: (size.x*0.5).max(size.z*0.5) }
}

fn camera(b: &Bounds, f: Frame, x: i32, y: i32) -> Camera3D {
    let view = match f.height {
        Some(fraction) => b.h * fraction,
        None => (b.h * 0.5).max(b.across) * 1.08 / f.zoom,
    };
    let center = ::glam::Vec3::new(b.mid_x, b.top + view*0.04 - view*0.5 + view*f.pan, b.mid_z);
    let distance = view*0.5 / (20f32.to_radians()*0.5).tan() + b.depth;
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
    let roster: [(&str, &fighter::FighterAssets); 8] = [
        ("Samus", &fighter::SAMUS), ("Fox", &fighter::FOX), ("Luigi", &fighter::LUIGI),
        ("Pikachu", &fighter::PIKACHU), ("Captain Falcon", &fighter::CAPTAIN),
        ("Link", &fighter::LINK), ("Yoshi", &fighter::YOSHI), ("Donkey Kong", &fighter::DONKEY),
    ];
    // Head-and-shoulders crop, as a fraction of each fighter's height. Pikachu, Yoshi
    // and DK have proportionally larger heads, so their crop is taller.
    let after = [("Samus", 0.58, 0.03), ("Fox", 0.58, 0.03), ("Luigi", 0.58, 0.03),
        ("Pikachu", 0.75, 0.05), ("Captain Falcon", 0.58, 0.03), ("Link", 0.58, 0.03),
        ("Yoshi", 0.66, 0.04), ("Donkey Kong", 0.90, 0.08)];
    let before = Frame { zoom: 1.14, height: None, pan: 0.02 };

    let mut views: Vec<_> = roster.iter().map(|(_, f)| Presentation::load_for(dir, 0, f).unwrap()).collect();
    for p in views.iter_mut() {
        p.clip = "Wait".into(); p.frame = 0.;
        p.world = p.animations.pose(&p.model, "Wait", 0., false);
    }
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let b: Vec<_> = views.iter().map(bounds).collect();
    for (i, x) in b.iter().enumerate() {
        eprintln!("{} h={:.3} across={:.3} current={:.3}h proposed={:.3}h pan={:.2}", roster[i].0, x.h, x.across,
            (x.h*0.5).max(x.across)*1.08/1.14/x.h, after[i].1, after[i].2);
    }

    for rendered in 0..3 {
        clear_background(Color::from_rgba(23, 28, 38, 255));
        draw_text("Character portraits at Wait 0, 45 degrees left. Top: current ThreeQuarter. Bottom: proposed face crop.", 25., 30., 22., WHITE);
        for group in 0..2 {
            draw_text(if group == 0 { "current" } else { "proposed" }, 25., (40 + group * 640) as f32, 20., GRAY);
            for row in 0..2 {
                for col in 0..4 {
                    let i = row * 4 + col;
                    let x = 20 + col as i32 * 310;
                    let top = 70 + (group as i32) * 640 + (row as i32) * 290;
                    let frame = if group == 0 { before }
                        else { Frame { zoom: 1.14, height: Some(after[i].1), pan: after[i].2 } };
                    set_camera(&camera(&b[i], frame, x, 1300 - top - 256));
                    draw(&views[i], &alpha, &additive);
                    set_default_camera();
                    draw_text(roster[i].0, x as f32, (top - 12) as f32, 17., WHITE);
                }
            }
        }
        draw_text("Source: fighter Wait clips (ftCo_Wait.c::ftCo_Wait_Anim); camera math mirrors Ror2Common.Portrait", 25., 1284., 15., GRAY);
        if rendered == 2 { get_screen_data().export_png(out); println!("wrote {out}"); break; }
        next_frame().await;
    }
}
