//! Procedural strike-pose proof (no disc, no assets): the authored Samus jab driven through the
//! real sim, the stand-in cylinder tipped by `Sim::strike_pose_pitch`, one body per sampled tick.
//! cargo run --release -p ssbm-viewer --example strike_pose -- [capture.png]
//! Label: PROCEDURAL stand-in (libcommon::strike_pose, 45 degrees at the first active frame,
//! authored Attack11: first hitbox on frame 3, 18 frames long). Not an original Melee pose.
use libssbm::{builtin, sim::ATTACK, FlatHost, Input};
use macroquad::prelude::*;

fn config() -> Conf {
    Conf { window_title: "Procedural strike pose (jab)".into(), window_width: 1280, window_height: 480, ..Default::default() }
}

/// Samples of the jab: (ticks after the press, motion frame, pitch radians, hit volumes active).
fn samples() -> Vec<(u32, f32, f32, usize)> {
    let mut s = builtin::sim(0, ::glam::Vec3::ZERO).unwrap();
    let mut host = FlatHost::default();
    for _ in 0..30 {
        s.advance(&mut host, Input::idle(), 1. / 60.);
    }
    let mut out = Vec::new();
    for t in 0..20 {
        let input = if t == 0 { Input { buttons: ATTACK, ..Input::idle() } } else { Input::idle() };
        s.advance(&mut host, input, 1. / 60.);
        out.push((t, s.motion.frame, s.strike_pose_pitch().unwrap_or(0.), s.action.samples.len()));
    }
    out
}

/// A cylinder body tipped about its center and local right axis (+X), with a facing marker.
fn stand_in(center: Vec3, pitch: f32, active: bool) {
    let height = 1.6;
    let rot = Quat::from_rotation_x(pitch);
    let color = if active { ORANGE } else { RED };
    let mut mesh = Mesh { vertices: Vec::new(), indices: Vec::new(), texture: None };
    for i in 0..=24 {
        let a = i as f32 * std::f32::consts::TAU / 24.;
        for end in [-0.5, 0.5] {
            let p = center + rot * vec3(a.cos() * 0.3, end * height, a.sin() * 0.3);
            mesh.vertices.push(Vertex::new(p.x, p.y, p.z, 0., 0., color));
        }
        if i < 24 {
            let n = i * 2;
            mesh.indices.extend([n, n + 1, n + 2, n + 1, n + 3, n + 2]);
        }
    }
    draw_mesh(&mesh);
    // The facing marker rides the body: forward is +Z, pitching forward tips the top toward +Z.
    draw_sphere(center + rot * vec3(0., height * 0.35, 0.32), 0.1, None, WHITE);
}

#[macroquad::main(config)]
async fn main() {
    let capture = std::env::args().nth(1);
    let all = samples();
    let shown: Vec<_> = [0u32, 2, 3, 5, 8, 12, 17].iter().map(|&t| all[t as usize]).collect();
    for frame in 0..3 {
        clear_background(Color::new(0.12, 0.13, 0.16, 1.));
        set_camera(&Camera3D { position: vec3(0., 1.5, -2.0 - 9.0), target: vec3(0., 1.2, 0.), up: Vec3::Y, fovy: 0.55, ..Default::default() });
        // Side-on from -X so the forward (+Z) tip reads as a lean.
        set_camera(&Camera3D { position: vec3(-14., 1.6, 0.), target: vec3(0., 1.0, 0.), up: Vec3::Y, fovy: 0.6, ..Default::default() });
        draw_plane(vec3(0., 0., 0.), vec2(30., 30.), None, Color::new(0.2, 0.22, 0.26, 1.));
        for (i, (_, _, pitch, boxes)) in shown.iter().enumerate() {
            let z = (i as f32 - 3.) * 1.9;
            // Shift each body along the camera's horizontal axis (world +Z reads left to right from -X).
            stand_in(vec3(0., 1.0 + 0.8 * pitch.cos().max(0.7) - 0.8 * 0.7, z), *pitch, *boxes > 0);
        }
        set_default_camera();
        draw_text("PROCEDURAL stand-in strike pose: authored Samus jab (Attack11), libcommon::strike_pose", 16., 26., 24., WHITE);
        draw_text("45 deg forward at the first active frame (motion frame 3); orange = hit volumes active. Not an original Melee pose.", 16., 52., 20., LIGHTGRAY);
        for (i, (t, mf, pitch, boxes)) in shown.iter().enumerate() {
            let x = 640. + (i as f32 - 3.) * 168. - 60.;
            draw_text(&format!("tick {t}"), x, 380., 22., WHITE);
            draw_text(&format!("frame {mf:.0}"), x, 404., 20., LIGHTGRAY);
            draw_text(&format!("{:.0} deg", pitch.to_degrees()), x, 428., 20., LIGHTGRAY);
            draw_text(&format!("{boxes} boxes"), x, 452., 20., LIGHTGRAY);
        }
        if let (Some(path), true) = (&capture, frame == 2) {
            get_screen_data().export_png(path);
            break;
        }
        next_frame().await;
    }
}
