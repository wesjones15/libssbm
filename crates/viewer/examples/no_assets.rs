//! Procedural fallback preview. No disc discovery, files, textures or audio.
//! cargo run --release -p ssbm-viewer --example no_assets -- <output-directory>
//! Original-asset IDs do not apply: these are deliberately generic shapes.
use libsms::{Event, Pad, Params, Sim, WaterEmit};
use macroquad::prelude::*;
#[derive(Default)]
struct Floor {
    events: Vec<Event>,
}
impl libsms::Host for Floor {
    fn ground(&self, _: libsms::Vec3) -> Option<libsms::GroundHit> {
        Some(libsms::GroundHit {
            height: 0.,
            normal: libsms::Vec3::Y,
            flags: libsms::SurfaceFlags::NONE,
        })
    }
    fn event(&mut self, e: Event) {
        self.events.push(e);
    }
}

fn config() -> Conf {
    Conf {
        window_title: "No-assets procedural preview".into(),
        window_width: 1100,
        window_height: 760,
        ..Default::default()
    }
}
fn body(at: Vec3, color: Color, height: f32) {
    draw_cylinder(at, 0.3, 0.3, height, None, color);
    draw_sphere(at + vec3(0., height * 0.85, 0.32), 0.09, None, WHITE);
}
fn mario_pose(feet: Vec3, horizontal: bool, pound: bool, spin: bool) {
    let height = if pound { 1.6 * 0.55 } else { 1.6 };
    if horizontal {
        let center = feet + vec3(0., 0.35 * 0.75, height * 0.25);
        let mut mesh = Mesh { vertices: Vec::new(), indices: Vec::new(), texture: None };
        for i in 0..=24 {
            let a = i as f32 * std::f32::consts::TAU / 24.;
            for end in [-0.5, 0.5] {
                let p = center + vec3(a.cos() * 0.35, a.sin() * 0.35, end * height);
                mesh.vertices.push(Vertex::new(p.x, p.y, p.z, 0., 0., RED));
            }
            if i < 24 { let n = i * 2; mesh.indices.extend([n,n+1,n+2,n+1,n+3,n+2]); }
        }
        draw_mesh(&mesh);
        draw_sphere(center + vec3(0., 0., height * 0.5), 0.09, None, WHITE);
        if spin { draw_cylinder(feet + vec3(0., height * 0.5, 0.), 0.7, 0.7, height * 0.4, None, PINK); }
    } else {
        body(feet + vec3(0., height * 0.5, 0.), RED, height);
        if spin { draw_cylinder(feet + vec3(0., height * 0.5, 0.), 0.7, 0.7, height * 0.4, None, PINK); }
    }
}
fn fighter_model(actor: &libssbm_assets::presentation::Presentation, offset: Vec3) {
    for (i, batch) in actor.model.batches.iter().enumerate() {
        if !actor.visible(i) {
            continue;
        }
        let color = Color::new(
            batch.color[0],
            batch.color[1],
            batch.color[2],
            batch.color[3],
        );
        let vertices = actor
            .model
            .skin(i, &actor.world)
            .into_iter()
            .map(|(p, _)| {
                let at = vec3(p.x, p.y, p.z) * 0.1 + offset;
                Vertex::new(at.x, at.y, at.z, 0., 0., color)
            })
            .collect();
        draw_mesh(&Mesh {
            vertices,
            indices: batch.indices.iter().map(|&i| i as u16).collect(),
            texture: None,
        });
    }
}
#[macroquad::main(config)]
async fn main() {
    let output = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&output).unwrap();
    let mut mario = Sim::new(
        Params::builtin(),
        libsms::AnimTable::builtin(),
        libsms::Vec3::ZERO,
        0,
    );
    mario.mario.give_fludd();
    let mut water = libsms::water::WaterManager::new(&mario.mario.params);
    let mut host = Floor::default();
    let mut fighter = libssbm::builtin::sim(0, ::glam::Vec3::ZERO).unwrap();
    let mut fighter_host = libssbm::FlatHost::default();
    let mut fighter_view = libssbm_assets::procedural::presentation(0, false);
    let mut captured = 0;
    let mut emits = 0;
    for frame in 0..185 {
        fighter.advance(
            &mut fighter_host,
            libssbm::Input {
                aim: ::glam::Vec3::Z,
                buttons: if frame == 30 {
                    libssbm::ATTACK
                } else if (115..145).contains(&frame) {
                    libssbm::DOWN_B
                } else {
                    0
                },
                allowed: u32::MAX,
                ..Default::default()
            },
            1. / 60.,
        );
        fighter_view.update(&fighter, 1. / 60.);
        let mut emitted: Vec<WaterEmit> = Vec::new();
        mario.advance_observed(
            &mut host,
            &Pad {
                r: true,
                r_analog: 255,
                a: frame == 120,
                ..Pad::default()
            },
            0,
            1. / 60.,
            |_, h| {
                for event in h.events.drain(..) {
                    if let Event::WaterEmit(e) = event {
                        emitted.push(e);
                    }
                }
            },
        );
        for e in emitted {
            water.emit(&e);
            emits += 1;
        }
        if frame % 2 == 0 {
            water.step(&host, mario.mario.pos);
        }
        clear_background(Color::from_rgba(23, 29, 40, 255));
        set_camera(&Camera3D {
            position: vec3(15., 12., 18.),
            target: vec3(1., 0.8, 3.),
            up: Vec3::Y,
            ..Default::default()
        });
        draw_grid(24, 1., GRAY, DARKGRAY);
        // Olimar's existing stand-ins: shape/color vocabulary only; swarm remains libpikmin-owned.
        body(vec3(-5., 0.8, 0.), GOLD, 1.6);
        // Authored skin stand-ins: Luigi green, Louie blue; no source asset IDs.
        body(vec3(-8., 0.8, 0.), BLUE, 1.6);
        body(vec3(0., 0.8, -4.), GREEN, 1.6);
        // Pose vocabulary only: deliberately generic geometry, no original clip/model.
        mario_pose(vec3(-5., 0., -5.), true, false, false);
        mario_pose(vec3(-3., 0., -5.), false, true, false);
        mario_pose(vec3(-1., 0., -7.), false, false, true);
        for (i, color) in [RED, BLUE, YELLOW].into_iter().enumerate() {
            let at = vec3(-6. + i as f32, 0.3, 2.);
            body(at, color, 0.6);
            draw_sphere(at + vec3(0., 0.65, 0.), 0.12, None, GREEN);
        }
        let p = mario.mario.pos;
        body(vec3(p.x / 100., p.y / 100. + 0.8, p.z / 100.), RED, 1.6);
        for p in &water.particles {
            let a = p.pos / 100.;
            let radius = (p.size / 100.).clamp(0.025, 0.12);
            draw_sphere(vec3(a.x, a.y, a.z), radius, None, SKYBLUE);
        }
        fighter_model(&fighter_view, vec3(5., fighter.position.y * 0.1, 0.));
        // Generic projectile trajectory and transformed body presentation, no effect-model substitution.
        draw_sphere(
            vec3(5., 1.1, 1. + (frame % 90) as f32 * 0.06),
            0.22,
            None,
            SKYBLUE,
        );
        draw_sphere(vec3(7., 0.5, 0.), 0.5, None, ORANGE);
        set_default_camera();
        draw_text(
            "PROCEDURAL FALLBACK | no extracted asset / cue / model ID",
            24.,
            35.,
            25.,
            WHITE,
        );
        draw_text(
            "Olimar: stand-ins    Mario: live 120 Hz sim + water    Melee: live 60 Hz sim + generated model",
            24.,
            67.,
            20.,
            LIGHTGRAY,
        );
        draw_text("Skin stand-ins: Luigi = green cylinder; Louie = blue cylinder", 24., 95., 20., LIGHTGRAY);
        draw_text("Mario poses: horizontal dive/slide | short ground pound | wide spin core", 24., 122., 20., LIGHTGRAY);
        draw_text(
            &format!(
                "Frame {frame} | FLUDD emissions {emits} | live droplets {}",
                water.particles.len()
            ),
            24.,
            710.,
            22.,
            WHITE,
        );
        if [90, 120, 150].contains(&frame) {
            assert!(emits > 0, "FLUDD must actually emit without assets");
            get_screen_data().export_png(&format!("{output}/procedural-{frame}.png"));
            captured += 1;
        }
        next_frame().await;
    }
    assert_eq!(captured, 3);
}
