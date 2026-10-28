//! Link hookshot proof: the blue held launcher (PlLk.dat item slot 2, public
//! article 4) and the separate chain-link / hook-head joint subtrees x54, x58,
//! x5C. Spec: it/kinds/itlinkhookshot.c::it_802A2568 / it_link_get_joint /
//! it_link_get_joint_c.
//! cargo run --release -p ssbm-viewer --example link_hookshot -- assets build/link-hookshot.png
use libssbm_assets::{fighter::LINK, hookshot, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;

fn config() -> Conf {
    Conf { window_title: "Link hookshot - item + chain proof".into(), window_width: 1400, window_height: 900, ..Default::default() }
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

fn bounds(p: &Presentation) -> ((::glam::Vec3, ::glam::Vec3), usize) {
    p.model.batches.iter().fold(((::glam::Vec3::splat(f32::INFINITY), ::glam::Vec3::splat(f32::NEG_INFINITY)), 0), |(b, n), batch| {
        let (lo, hi) = batch.vertices.iter().fold((b.0, b.1), |(lo, hi), v| (lo.min(v.position), hi.max(v.position)));
        ((lo, hi), n + batch.vertices.len())
    })
}

#[macroquad::main(config)]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let mut item = Presentation::article_for(dir, libssbm_assets::held::LINK_HOOKSHOT, 0, &LINK).unwrap();
    item.object_pose(0.);
    let mut parts = Vec::new();
    for p in [hookshot::LINK_A, hookshot::LINK_B, hookshot::HOOK] {
        let part = hookshot::part(dir, p).unwrap();
        let (b, verts) = bounds(&part);
        println!("hookshot part {p}: {} joints, {} batches, {verts} verts, bounds {:?}..{:?}", part.model.joints.len(), part.model.batches.len(), b.0, b.1);
        parts.push(part);
    }
    let (b, verts) = bounds(&item);
    println!("held item: {} joints, {} batches, {verts} verts, bounds {:?}..{:?}", item.model.joints.len(), item.model.batches.len(), b.0, b.1);
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(35, 40, 55, 255));
        // Top left: the held launcher by itself. Top right: the three source parts
        // side by side. Bottom: the assembled chain at sim scale, with the same
        // alternating links and orientation the host uses.
        set_camera(&Camera3D { position: vec3(0., 5., 32.), target: vec3(0., 1., 0.), up: Vec3::Y,
            viewport: Some((0, 560, 680, 300)), ..Default::default() });
        draw(&item, ::glam::Mat4::from_scale(::glam::Vec3::splat(1.5)), &alpha, &additive);
        set_default_camera();
        draw_text("held item: article 4 (blue launcher)", 12., 40., 20., WHITE);

        set_camera(&Camera3D { position: vec3(0., 5., 45.), target: vec3(0., 1., 0.), up: Vec3::Y,
            viewport: Some((700, 560, 700, 300)), ..Default::default() });
        for (i, p) in parts.iter().enumerate() {
            let t = ::glam::Mat4::from_scale_rotation_translation(
                ::glam::Vec3::splat(3.0),
                ::glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                ::glam::Vec3::new(-9. + i as f32 * 9., 1., 0.),
            );
            draw(p, t, &alpha, &additive);
        }
        set_default_camera();
        draw_text("source parts: chain A | chain B | hook head", 712., 40., 20., WHITE);

        set_camera(&Camera3D { position: vec3(0., 9., 48.), target: vec3(0., 1., 0.), up: Vec3::Y,
            viewport: Some((0, 60, 1400, 430)), ..Default::default() });
        // Held launcher at the left, then alternating links, then the hook head.
        let item_rot = ::glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        draw(&item, ::glam::Mat4::from_scale_rotation_translation(::glam::Vec3::splat(1.0), item_rot, ::glam::Vec3::new(-19., 1., 0.)), &alpha, &additive);
        let link_rot = ::glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        for i in 0..18 {
            let p = if i & 1 == 0 { &parts[0] } else { &parts[1] };
            let t = ::glam::Mat4::from_scale_rotation_translation(
                ::glam::Vec3::splat(1.0),
                link_rot,
                ::glam::Vec3::new(-17. + i as f32 * 1.9, 1., 0.),
            );
            draw(p, t, &alpha, &additive);
        }
        // The hook's claw is local -Z, so it points outward with -90 degrees.
        let hook_rot = ::glam::Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        draw(&parts[2], ::glam::Mat4::from_scale_rotation_translation(::glam::Vec3::splat(1.0), hook_rot, ::glam::Vec3::new(17.5, 1., 0.)), &alpha, &additive);
        set_default_camera();
        draw_text("assembled at sim scale: article 4 -> x54/x58 alternating -> x5C", 12., 530., 20., WHITE);

        draw_text("PlLk.dat slot 2 | It_Kind_Link_HShot | public article 4 + x54/x58/x5C", 12., 855., 22., YELLOW);
        draw_text("itlinkhookshot.c::it_802A2568 / it_link_get_joint / it_link_get_joint_c", 12., 882., 20., WHITE);
        if rendered == 2 { if let Some(path) = args.get(1) { get_screen_data().export_png(path); } break; }
        rendered += 1; next_frame().await;
    }
}
