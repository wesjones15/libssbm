//! Link hookshot proof: PlLk.dat item slot 3 (It_Kind_Link_HShot), public article 2.
//! Spec: it/kinds/itlinkhookshot.c::it_802A2568 (chain joints x54/x58/x5C) and
//! ft/kinds/ftCommon/ftCo_0D8E.c::fn_800D8EC8 (standing grab launches the item).
//! cargo run --release -p ssbm-viewer --example link_hookshot -- assets build/link-hookshot.png
use libssbm_assets::{fighter::LINK, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf {
    Conf { window_title: "Link hookshot - slot 3 proof".into(), window_width: 1200, window_height: 820, ..Default::default() }
}

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
    let mut hook = Presentation::article_for(dir, 2, 0, &LINK).unwrap();
    hook.object_pose(0.);
    println!("hookshot article 2: {} joints, {} batches, attach {:?}", hook.model.joints.len(), hook.model.batches.len(), hook.attach_joint);
    for (i, b) in hook.model.batches.iter().enumerate() {
        let (lo, hi) = b.vertices.iter().fold((::glam::Vec3::splat(f32::INFINITY), ::glam::Vec3::splat(f32::NEG_INFINITY)), |(lo, hi), v| (lo.min(v.position), hi.max(v.position)));
        println!("batch {i}: owner {}, verts {}, bounds {lo:?}..{hi:?}, blend {}", b.owner, b.vertices.len(), b.blend);
    }
    println!("clip frames {:?}", hook.animations.clips.get("Object").map(|c| c.frames));
    let bind = hook.model.bind_world();
    for (i, m) in bind.iter().enumerate() {
        println!("joint {i}: translation {:?}", m.to_cols_array()[12..15].to_vec());
    }
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35, 40, 55, 255));
        // A row of the hook at four sizes/scales, so its silhouette and scale are legible.
        // One article by itself, then the same article repeated along +X at the
        // spacing and orientation the host would use for a chain.
        set_camera(&Camera3D { position: vec3(0., 20., 100.), target: vec3(0., 8., 0.), up: Vec3::Y,
            viewport: Some((0, 80, 300, 620)), ..Default::default() });
        draw(&hook, ::glam::Mat4::from_scale(::glam::Vec3::splat(1.6)), &alpha, &additive);
        set_default_camera();
        draw_text("article 2 x1.6", 10., 50., 20., WHITE);
        set_camera(&Camera3D { position: vec3(38., 26., 82.), target: vec3(38., 7., 0.), up: Vec3::Y,
            viewport: Some((300, 80, 900, 620)), ..Default::default() });
        for i in 0..12 {
            let t = ::glam::Mat4::from_scale_rotation_translation(
                ::glam::Vec3::splat(1.0),
                ::glam::Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                ::glam::Vec3::new(i as f32 * 6.4, 7., 0.),
            );
            draw(&hook, t, &alpha, &additive);
        }
        set_default_camera();
        draw_text("article 2 repeated at 6.4 units, local +Y along the chain", 310., 50., 20., WHITE);
        draw_text("PlLk.dat slot 3 | It_Kind_Link_HShot | public article 2 | it_802A2568", 12., 760., 22., YELLOW);
        draw_text("itlinkhookshot.c chain joints x54/x58/x5C; ftCo_0D8E.c::fn_800D8EC8", 12., 790., 20., WHITE);
        if rendered == 2 { if let Some(path) = args.get(1) { get_screen_data().export_png(path); } break; }
        rendered += 1; next_frame().await;
    }
}
