use libssbm::{
    ATTACK, CANCEL, DOWN, DOWN_B, EventKind, FlatHost, GRAB, Input, JUMP, NEUTRAL_B, Phase, SIDE_B,
    SPECIAL, SPRINT, Sim, UP, UP_B,
};
use libssbm_assets::presentation::Presentation;
use macroquad::prelude::*;
mod models;
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
    let dash_preview = args.iter().any(|s| s == "--dash-preview");
    let sideb_preview = args.iter().any(|s| s == "--sideb-preview");
    let mobile_preview = args.iter().any(|s| s == "--mobile-preview");
    let rebirth_preview = args.iter().any(|s| s == "--rebirth-preview");
    let move_after = args
        .iter()
        .position(|s| s == "--move-after")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<usize>())
        .transpose()?;
    let entry_mode = args.iter().any(|s| s == "--entry");
    let effect = args
        .iter()
        .position(|s| s == "--effect")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<u32>())
        .transpose()?;
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
    let mut actor = if let Some(id) = effect {
        Presentation::effect(std::path::Path::new(&path), id)?
    } else if let Some(kind) = article {
        Presentation::article(std::path::Path::new(&path), kind, article_state)?
    } else {
        Presentation::load(std::path::Path::new(&path), 0)?
    };
    let mut sim = Sim::with_behavior(
        libssbm_assets::params::load_common(
            std::path::Path::new(&path),
            &libssbm_assets::fighter::SAMUS,
        )?,
        Box::new(libssbm::samus::Samus::new(
            libssbm_assets::params::load_samus_specials(std::path::Path::new(&path))?,
            libssbm_assets::fighter::SAMUS.upper_body_root,
        )),
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
    let mut texture_frames = vec![None; meshes.len()];
    let model_alpha = models::blended(false)?;
    let model_additive = models::blended(true)?;
    let mut platform = if entry_mode || rebirth_preview {
        Some(Presentation::platform(std::path::Path::new(&path))?)
    } else {
        None
    };
    let mut platform_meshes = platform.as_ref().map(make_meshes);
    let mut last_hit = String::new();
    let mut particle_clock = 0f64;
    if dash_preview {
        sim.grounded = true;
        sim.start_move(&mut host, libssbm::Move::DashAttack, Input::idle());
        for _ in 0..pose_frame as usize {
            sim.advance(&mut host, Input::idle(), 1. / 60.);
        }
        println!(
            "Dash preview tick {}: {:?}, position {:?}",
            pose_frame, sim.motion.clip, sim.position
        );
    }
    if sideb_preview {
        sim.grounded = true;
        sim.start_move(&mut host, libssbm::Move::Missile, Input::idle());
        for _ in 0..pose_frame as usize {
            sim.advance(&mut host, Input::idle(), 1. / 60.);
        }
        println!("Side-B preview tick {pose_frame}: {:?}", sim.motion.clip);
    }
    if mobile_preview {
        sim.advance(&mut host, Input::idle(), 1. / 60.);
        for _ in 0..(pose_frame as usize).max(20) {
            sim.advance(
                &mut host,
                Input {
                    movement: ::glam::Vec3::X,
                    buttons: libssbm::NEUTRAL_B,
                    ..Input::idle()
                },
                1. / 60.,
            );
        }
        println!(
            "Mobile preview: {:?} {:?}, gait {:?} {:?}, world {:?}",
            sim.phase, sim.motion.clip, sim.locomotion.clip, sim.locomotion.frame, sim.position
        );
    }
    if rebirth_preview {
        let dir = std::path::Path::new(&path);
        sim.begin_rebirth(
            libssbm_assets::params::rebirth(dir)?,
            libssbm_assets::params::entry(dir, &libssbm_assets::fighter::SAMUS)?,
            ::glam::Vec3::ZERO,
        );
        for tick in 0..pose_frame as usize {
            let mut input = Input::idle();
            if move_after.is_some_and(|frame| tick >= frame) {
                input.movement = ::glam::Vec3::X;
            }
            sim.advance(&mut host, input, 1. / 60.);
        }
        println!(
            "Rebirth preview tick {pose_frame}: {:?}",
            sim.rebirth.as_ref().map(|s| (&s.phase, s.frame, s.flags()))
        );
    }
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
            texture_frames = vec![None; meshes.len()];
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
        if !dash_preview
            && !sideb_preview
            && !rebirth_preview
            && !(mobile_preview && capture.is_some())
        {
            sim.advance(&mut host, input, dt as f64);
        }
        actor.update(&sim, dt);
        if rebirth_preview && let (Some(entry), Some(platform)) = (sim.entry, platform.as_mut()) {
            let scale = entry.params.platform;
            let transform = ::glam::Mat4::from_scale(::glam::Vec3::new(
                scale,
                (scale * entry.platform_y()).max(0.001),
                scale,
            ));
            platform.world = platform
                .model
                .bind_world()
                .into_iter()
                .map(|m| transform * m)
                .collect();
        }
        if entry_mode {
            let params = libssbm_assets::params::entry(
                std::path::Path::new(&path),
                &libssbm_assets::fighter::SAMUS,
            )?;
            let entry = libssbm::entry::State {
                waiting: false,
                started: false,
                params,
                frame: pose_frame as u32,
            };
            sim.entry = Some(entry);
            sim.motion.select("Entry", 0.);
            actor.update(&sim, 0.);
            if let Some(platform) = platform.as_mut() {
                let scale = params.platform;
                let transform = ::glam::Mat4::from_scale(::glam::Vec3::new(
                    scale,
                    (scale * entry.platform_y()).max(0.001),
                    scale,
                ));
                platform.world = platform
                    .model
                    .bind_world()
                    .into_iter()
                    .map(|m| transform * m)
                    .collect();
            }
        }
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
        if article.is_some() || effect.is_some() {
            actor.object_pose(pose_frame);
        }
        let object = article.is_some() || effect.is_some() || emitter.is_some();
        let mut center = gv(sim.position)
            + vec3(
                0.,
                if effect == Some(2000) {
                    8.
                } else if object {
                    0.
                } else {
                    12.
                },
                0.,
            );
        let mut distance: f32 = if effect == Some(2000) {
            65.
        } else if object {
            25.
        } else {
            65.
        };
        if article.is_some() || effect == Some(25) {
            let points: Vec<_> = (0..actor.model.batches.len())
                .flat_map(|i| {
                    actor
                        .model
                        .skin(i, &actor.world)
                        .into_iter()
                        .map(|(p, _)| p)
                })
                .collect();
            let lo = points
                .iter()
                .fold(::glam::Vec3::splat(f32::INFINITY), |a, p| a.min(*p));
            let hi = points
                .iter()
                .fold(::glam::Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
            center = gv((lo + hi) * 0.5 + sim.position);
            distance = distance.max((hi - lo).max_element() * 1.7);
        }
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
        // The selection view keeps the fighter's actual upward displacement.
        // It begins and ends on the floor while Screw Attack can rise freely.
        let preview_offset = ::glam::Vec3::ZERO;
        if object {
            actor.face_camera(
                rotation
                    .inverse()
                    .transform_point3(sv(camera) - sim.position)
                    - preview_offset,
            );
        }
        for (i, mesh) in meshes.iter_mut().enumerate() {
            if emitter.is_some() {
                continue;
            }
            if !actor.visible(i) {
                continue;
            }
            let skin = actor.model.skin(i, &actor.world);
            let batch = &actor.model.batches[i];
            let frame = actor.texture_frame(i);
            if frame != texture_frames[i] {
                if let (Some(texture), Some(image)) = (&mesh.texture, actor.texture(i)) {
                    texture.update_from_bytes(image.width as u32, image.height as u32, &image.rgba);
                }
                texture_frames[i] = frame;
            }
            let texture_matrix = actor.texture_matrix(i);
            let mut color = actor.color(i);
            if rebirth_preview && let Some(overlay) = sim.rebirth.as_ref().and_then(|s| s.overlay())
            {
                for i in 0..3 {
                    color[i] = color[i] * (1. - overlay[3]) + overlay[i] * overlay[3];
                }
            }
            for (v, ((p, n), original)) in mesh
                .vertices
                .iter_mut()
                .zip(skin.into_iter().zip(&batch.vertices))
            {
                let p = rotation.transform_point3(p + preview_offset) + sim.position;
                v.position = gv(p);
                let n = rotation.transform_vector3(n);
                let light = if batch.lit == 0 {
                    1.
                } else {
                    0.4 + 0.6 * n.dot(::glam::Vec3::new(0.4, 0.8, 0.5).normalize()).max(0.)
                };
                let flash = sim.phase == Phase::SmashCharge && (sim.frame / 4) & 1 == 0;
                v.color = if flash {
                    [255, 240, 70, 255]
                } else {
                    [
                        (color[0] * light * 255.) as u8,
                        (color[1] * light * 255.) as u8,
                        (color[2] * light * 255.) as u8,
                        (color[3] * 255.) as u8,
                    ]
                };
                let uv = texture_matrix.transform_point3(original.uv.extend(0.));
                v.uv = vec2(uv.x, uv.y);
            }
            if batch.blend >= 2 {
                gl_use_material(if batch.blend == 3 {
                    &model_additive
                } else {
                    &model_alpha
                });
            }
            draw_mesh(mesh);
            gl_use_default_material();
        }
        for s in &shots {
            draw_sphere(s.at, s.radius, None, s.color);
        }
        if let (Some(platform), Some(meshes)) = (&platform, &mut platform_meshes)
            && (!rebirth_preview || sim.entry.is_some())
        {
            for (i, mesh) in meshes.iter_mut().enumerate() {
                for (v, (p, n)) in mesh
                    .vertices
                    .iter_mut()
                    .zip(platform.model.skin(i, &platform.world))
                {
                    v.position = gv(rotation.transform_point3(p) + sim.position);
                    let light = 0.4 + 0.6 * n.dot(::glam::Vec3::Y).max(0.);
                    let c = platform.model.batches[i].color;
                    v.color = [
                        (c[0] * light * 255.) as u8,
                        (c[1] * light * 255.) as u8,
                        (c[2] * light * 255.) as u8,
                        255,
                    ];
                }
                draw_mesh(mesh);
            }
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
            "Xbox: LS move/click sprint | A jump | RT neutral B | LT contextual A | RB side B | LB up B | B down B | X grab",
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
                sim.behavior.charge_steps(),
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
                let wrap = |w| match w {
                    1 => miniquad::TextureWrap::Clamp,
                    2 => miniquad::TextureWrap::Mirror,
                    _ => miniquad::TextureWrap::Repeat,
                };
                unsafe {
                    macroquad::window::get_internal_gl()
                        .quad_context
                        .texture_set_wrap(tex.raw_miniquad_id(), wrap(t.wrap_s), wrap(t.wrap_t));
                }
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
        (0x2000, DOWN_B),
        (0x4000, GRAB),
        (0x100, UP_B),
        (0x200, SIDE_B),
        (0x40, SPRINT),
    ] {
        if s.pad.buttons & mask != 0 {
            input.buttons |= bit;
        }
    }
    if s.pad.rt > 40 {
        input.buttons |= NEUTRAL_B;
    }
    if s.pad.lt > 40 {
        input.buttons |= ATTACK;
    }
}
#[cfg(not(windows))]
fn read_pad(_: &mut Input, _: Vec3, _: Vec3) {}
