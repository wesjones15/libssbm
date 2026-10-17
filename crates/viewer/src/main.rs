use libssbm::{
    ATTACK, CANCEL, DOWN, EventKind, FlatHost, Input, JUMP, Phase, SPECIAL, SPRINT, Sim, UP,
};
use libssbm_assets::presentation::Presentation;
use macroquad::prelude::*;
mod particles;
fn config() -> Conf {
    Conf {
        window_title: "libssbm - Samus from Melee".into(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}
fn gv(v: ::glam::Vec3) -> Vec3 {
    vec3(v.x, v.y, v.z)
}
fn sv(v: Vec3) -> ::glam::Vec3 {
    ::glam::Vec3::new(v.x, v.y, v.z)
}
struct Shot {
    at: Vec3,
    velocity: Vec3,
    life: f32,
    radius: f32,
    color: Color,
}
#[macroquad::main(config)]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Samus viewer: {e}");
    }
}
async fn run() -> libssbm_assets::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .filter(|s| !s.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "assets".into());
    let capture = args
        .iter()
        .position(|s| s == "--capture")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let selection = args.iter().any(|s| s == "--selection");
    let pose_clip = args
        .iter()
        .position(|s| s == "--clip")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .or_else(|| selection.then(|| "SpecialHi".into()));
    let pose_frame = args
        .iter()
        .position(|s| s == "--pose-frame")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<f32>())
        .transpose()?
        .unwrap_or(0.);
    let article = args
        .iter()
        .position(|s| s == "--article")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<u32>())
        .transpose()?;
    let article_state = args
        .iter()
        .position(|s| s == "--state")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<u32>())
        .transpose()?
        .unwrap_or(0);
    let emitter = args
        .iter()
        .position(|s| s == "--emitter")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<u32>())
        .transpose()?;
    let particle_bank = if let Some(id) = emitter {
        Some(libssbm_assets::effects::Effects::load(
            std::path::Path::new(&path),
            id >= 2000,
        )?)
    } else {
        None
    };
    let particle_renderer = particle_bank
        .as_ref()
        .map(particles::Renderer::new)
        .transpose()?;
    let mut particle_system = libssbm::particle::System::default();
    if let (Some(id), Some(bank)) = (emitter, particle_bank.as_ref()) {
        particle_system.definitions = bank.emitters.clone();
        particle_system.spawn(id, ::glam::Mat4::IDENTITY, 1)?;
        for _ in 0..pose_frame as u32 {
            particle_system.tick();
        }
    }
    let mut actor = if let Some(kind) = article {
        Presentation::article(std::path::Path::new(&path), kind, article_state)?
    } else {
        Presentation::load(std::path::Path::new(&path), 0)?
    };
    let mut sim = Sim::new(
        libssbm_assets::params::load(std::path::Path::new(&path))?,
        ::glam::Vec3::ZERO,
    );
    let mut host = FlatHost::default();
    let fighter_animations =
        libssbm_assets::animation::Animations::load(std::path::Path::new(&path))?;
    sim.motion.definitions = fighter_animations.motion_definitions();
    sim.action.definitions =
        libssbm_assets::action::load(std::path::Path::new(&path), &fighter_animations)?;
    let mut shots = Vec::<Shot>::new();
    let mut camera_yaw = args
        .iter()
        .position(|s| s == "--camera-yaw")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<f32>())
        .transpose()?
        .unwrap_or(0.7);
    let mut costume = 0;
    let mut rendered = 0;
    let mut meshes = make_meshes(&actor);
    let mut last_hit = String::new();
    let mut particle_clock = 0f64;
    loop {
        let dt = if capture.is_some() {
            1. / 60.
        } else {
            get_frame_time().min(0.1)
        };
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::Tab) {
            costume = (costume + 1) % 5;
            actor = Presentation::load(std::path::Path::new(&path), costume)?;
            meshes = make_meshes(&actor);
        }
        if is_key_pressed(KeyCode::R) {
            sim.teleport(::glam::Vec3::ZERO);
        }
        if is_key_down(KeyCode::Left) {
            camera_yaw -= dt;
        }
        if is_key_down(KeyCode::Right) {
            camera_yaw += dt;
        }
        let forward = vec3(-camera_yaw.sin(), 0., -camera_yaw.cos());
        let right = vec3(-forward.z, 0., forward.x);
        let axis = |negative, positive| {
            if is_key_down(positive) {
                1.
            } else if is_key_down(negative) {
                -1.
            } else {
                0.
            }
        };
        let movement =
            right * axis(KeyCode::A, KeyCode::D) + forward * axis(KeyCode::S, KeyCode::W);
        let mut input = Input {
            movement: sv(movement),
            aim: sv(forward),
            ..Input::idle()
        };
        for (key, bit) in [
            (KeyCode::Space, JUMP),
            (KeyCode::Q, UP),
            (KeyCode::E, DOWN),
            (KeyCode::LeftShift, SPRINT),
            (KeyCode::C, CANCEL),
        ] {
            if is_key_down(key) {
                input.buttons |= bit;
            }
        }
        if is_mouse_button_down(MouseButton::Left) {
            input.buttons |= ATTACK;
        }
        if is_mouse_button_down(MouseButton::Right) {
            input.buttons |= SPECIAL;
        }
        read_pad(&mut input, forward, right);
        sim.advance(&mut host, input, dt as f64);
        actor.update(&sim, dt);
        if let Some(clip) = pose_clip.as_deref() {
            if !actor.animations.clips.contains_key(clip) {
                return Err(format!("Unknown clip {clip}").into());
            }
            actor.clip = clip.into();
            actor.frame = pose_frame;
            actor.world = actor.animations.pose(&actor.model, clip, pose_frame, false);
        }
        for e in host.events.drain(..) {
            if e.kind == EventKind::Hit {
                last_hit = format!("Move {} strength {:.2}", e.move_id, e.strength);
                let projectile = matches!(e.move_id, 20..=22 | 24);
                let at = gv(e.position) + vec3(0., 10., 0.);
                let dir = gv(e.direction);
                shots.push(Shot {
                    at,
                    velocity: if projectile { dir * 100. } else { Vec3::ZERO },
                    life: if projectile { 2. } else { 0.2 },
                    radius: if projectile {
                        1.5 + e.strength * 2.
                    } else {
                        7.
                    },
                    color: if e.move_id == 20 { SKYBLUE } else { ORANGE },
                });
            }
        }
        for s in &mut shots {
            s.life -= dt;
            s.at += s.velocity * dt;
        }
        shots.retain(|s| s.life > 0.);
        clear_background(Color::from_rgba(10, 17, 28, 255));
        if article.is_some() {
            actor.object_pose(pose_frame);
        }
        let object = article.is_some() || emitter.is_some();
        let center = gv(sim.position) + vec3(0., if object { 0. } else { 12. }, 0.);
        let distance = if object { 25. } else { 65. };
        let camera = center
            + vec3(
                camera_yaw.sin() * distance,
                distance * 0.43,
                camera_yaw.cos() * distance,
            );
        set_camera(&Camera3D {
            position: camera,
            target: center,
            up: Vec3::Y,
            fovy: 45.,
            ..Default::default()
        });
        draw_grid(
            40,
            10.,
            Color::from_rgba(40, 57, 74, 255),
            Color::from_rgba(24, 36, 49, 255),
        );
        let yaw = sim.facing.z.atan2(sim.facing.x);
        // Samus's authored locomotion faces +Z; Melee supplies fighter heading.
        let model_yaw = args
            .iter()
            .position(|s| s == "--model-yaw")
            .and_then(|i| args.get(i + 1))
            .map(|s| s.parse::<f32>())
            .transpose()?;
        let rotation =
            ::glam::Mat4::from_rotation_y(model_yaw.unwrap_or(std::f32::consts::FRAC_PI_2 - yaw));
        // The common RoR2 selection policy anchors the body while up-special
        // plays. Match that policy here so jump poses stay in the preview camera.
        let preview_offset = if selection {
            let idle = actor.animations.pose(&actor.model, "Wait", 0., false);
            idle[4].w_axis.truncate() - actor.world[4].w_axis.truncate()
        } else {
            ::glam::Vec3::ZERO
        };
        for (i, mesh) in meshes.iter_mut().enumerate() {
            if emitter.is_some() {
                continue;
            }
            if !actor.visible(i) {
                continue;
            }
            let skin = actor.model.skin(i, &actor.world);
            let batch = &actor.model.batches[i];
            for (v, ((p, n), original)) in mesh
                .vertices
                .iter_mut()
                .zip(skin.into_iter().zip(&batch.vertices))
            {
                let p = rotation.transform_point3(p + preview_offset) + sim.position;
                v.position = gv(p);
                let n = rotation.transform_vector3(n);
                let light = 0.4 + 0.6 * n.dot(::glam::Vec3::new(0.4, 0.8, 0.5).normalize()).max(0.);
                let flash = sim.phase == Phase::SmashCharge && (sim.frame / 4) & 1 == 0;
                v.color = if flash {
                    [255, 240, 70, 255]
                } else {
                    [
                        (batch.color[0] * light * 255.) as u8,
                        (batch.color[1] * light * 255.) as u8,
                        (batch.color[2] * light * 255.) as u8,
                        255,
                    ]
                };
                v.uv = vec2(original.uv.x, original.uv.y);
            }
            draw_mesh(mesh);
        }
        for s in &shots {
            draw_sphere(s.at, s.radius, None, s.color);
        }
        if let Some(renderer) = &particle_renderer {
            if capture.is_none() {
                particle_clock += dt as f64;
                while particle_clock >= 1. / 60. {
                    particle_clock -= 1. / 60.;
                    particle_system.tick();
                }
            }
            if !particle_system.errors.is_empty() {
                return Err(particle_system.errors.join("; ").into());
            }
            renderer.draw(&particle_system, camera);
        }
        set_default_camera();
        draw_rectangle(
            0.,
            0.,
            screen_width(),
            106.,
            Color::from_rgba(7, 11, 18, 230),
        );
        draw_text("SAMUS / MELEE LAB", 28., 38., 30., ORANGE);
        draw_text(
            "WASD move | Space jump | LMB attack | RMB special | Q/E up/down | Shift sprint | C store charge",
            28.,
            66.,
            18.,
            LIGHTGRAY,
        );
        draw_text(
            "Xbox: LS move | A jump | RT attack | LT special | LB/RB up/down | B store | Tab costume | R reset",
            28.,
            89.,
            18.,
            LIGHTGRAY,
        );
        let y = screen_height() - 100.;
        draw_rectangle(18., y, 600., 82., Color::from_rgba(7, 11, 18, 230));
        draw_text(
            format!(
                "{:?}  {}  costume {}",
                sim.phase,
                actor.clip,
                libssbm_assets::COSTUMES[costume]
            ),
            30.,
            y + 25.,
            22.,
            WHITE,
        );
        draw_text(
            format!(
                "Charge {}/{}  jumps {}/{}  {}",
                sim.shot_charge,
                sim.params.charge_steps,
                sim.jumps_used,
                sim.params.jumps,
                last_hit
            ),
            30.,
            y + 49.,
            18.,
            SKYBLUE,
        );
        draw_text(
            "Prototype timing / diffuse materials - game assets read from your disc",
            30.,
            y + 72.,
            16.,
            GRAY,
        );
        rendered += 1;
        if let Some(path) = capture.as_deref()
            && rendered == 5
        {
            get_screen_data().export_png(path);
            println!("Rendered Samus screenshot with disc model/animation");
            break;
        }
        next_frame().await;
    }
    Ok(())
}
fn make_meshes(actor: &Presentation) -> Vec<Mesh> {
    actor
        .model
        .batches
        .iter()
        .map(|b| Mesh {
            vertices: b
                .vertices
                .iter()
                .map(|v| {
                    Vertex::new(
                        v.position.x,
                        v.position.y,
                        v.position.z,
                        v.uv.x,
                        v.uv.y,
                        WHITE,
                    )
                })
                .collect(),
            indices: b
                .indices
                .iter()
                .map(|&i| u16::try_from(i).expect("viewer batch exceeds u16 indices"))
                .collect(),
            texture: b.texture.as_ref().map(|t| {
                let tex = Texture2D::from_rgba8(t.width, t.height, &t.rgba);
                tex.set_filter(FilterMode::Linear);
                tex
            }),
        })
        .collect()
}
#[cfg(windows)]
fn read_pad(input: &mut Input, forward: Vec3, right: Vec3) {
    #[repr(C)]
    #[derive(Default)]
    struct Gamepad {
        buttons: u16,
        lt: u8,
        rt: u8,
        lx: i16,
        ly: i16,
        rx: i16,
        ry: i16,
    }
    #[repr(C)]
    #[derive(Default)]
    struct State {
        packet: u32,
        pad: Gamepad,
    }
    #[link(name = "xinput1_4", kind = "raw-dylib")]
    unsafe extern "system" {
        fn XInputGetState(index: u32, state: *mut State) -> u32;
    }
    let mut s = State::default();
    if unsafe { XInputGetState(0, &mut s) } != 0 {
        return;
    }
    let axis = |v: i16| {
        let v = v as f32 / 32767.;
        if v.abs() < 0.18 { 0. } else { v }
    };
    input.movement = sv(right * axis(s.pad.lx) + forward * axis(s.pad.ly));
    for (mask, bit) in [
        (0x1000, JUMP),
        (0x2000, CANCEL),
        (0x100, UP),
        (0x200, DOWN),
        (0x40, SPRINT),
    ] {
        if s.pad.buttons & mask != 0 {
            input.buttons |= bit;
        }
    }
    if s.pad.rt > 40 {
        input.buttons |= ATTACK;
    }
    if s.pad.lt > 40 {
        input.buttons |= SPECIAL;
    }
}
#[cfg(not(windows))]
fn read_pad(_: &mut Input, _: Vec3, _: Vec3) {}
