//! Local-disc preview: Pikachu's Thunder Jolt.
//! `cargo run --release -p ssbm-viewer --example pikachu_jolt -- assets [out.png] [frame] [article|trail]`
//!
//! Source identity:
//!   * `itpikachutjoltground.c` `it_802B3554` calls `efSync_Spawn(0x4BD, gobj, jobj)`.
//!   * `efsync.c` `case 0x4BD` calls `hsd_8039EFAC(0, 7, 0x1B58, jobj)` -> generator 7000.
//!   * `itpikachutjoltair.c` / the article table give the jolt's model (article 2, state 0).
//!
//! The trail is generator 7000 from EfPkData; the orb itself is the item article model.
//! This example renders both without the RoR2 host so the asset can be judged first.
use libssbm_assets::{effects::Effects, fighter, presentation::Presentation};
use macroquad::prelude::*;

#[path = "../src/models.rs"]
mod models;
#[path = "../src/particles.rs"]
mod particles;

fn config() -> Conf {
    Conf {
        window_title: "Pikachu Thunder Jolt".into(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

fn draw_model(p: &Presentation, transform: ::glam::Mat4, alpha: &Material, additive: &Material) {
    draw_model_tint(p, transform, alpha, additive, [1.; 3]);
}
fn draw_model_tint(
    p: &Presentation,
    transform: ::glam::Mat4,
    alpha: &Material,
    additive: &Material,
    tint: [f32; 3],
) {
    for (i, b) in p.model.batches.iter().enumerate() {
        if !p.visible(i) {
            continue;
        }
        let texture = p.texture(i).map(|t| {
            let tex = Texture2D::from_rgba8(t.width, t.height, &t.rgba);
            tex.set_filter(FilterMode::Linear);
            let wrap = |n| match n {
                1 => miniquad::TextureWrap::Clamp,
                2 => miniquad::TextureWrap::Mirror,
                _ => miniquad::TextureWrap::Repeat,
            };
            unsafe {
                get_internal_gl().quad_context.texture_set_wrap(
                    tex.raw_miniquad_id(),
                    wrap(t.wrap_s),
                    wrap(t.wrap_t),
                );
            }
            tex
        });
        let c = p.color(i);
        let vertices = p
            .model
            .skin(i, &p.world)
            .iter()
            .zip(&b.vertices)
            .map(|((at, normal), v)| {
                let at = transform.transform_point3(*at);
                let uv = p.texture_matrix(i).transform_point3(v.uv.extend(0.));
                let light = if b.lit == 0 {
                    1.
                } else {
                    0.5 + 0.5
                        * normal
                            .dot(::glam::Vec3::new(0.4, 0.8, 0.5).normalize())
                            .max(0.)
                };
                Vertex::new(
                    at.x,
                    at.y,
                    at.z,
                    uv.x,
                    uv.y,
                    Color::new(
                        c[0] * v.color[0] * light * tint[0],
                        c[1] * v.color[1] * light * tint[1],
                        c[2] * v.color[2] * light * tint[2],
                        c[3] * v.color[3],
                    ),
                )
            })
            .collect();
        if b.blend >= 2 {
            gl_use_material(if b.blend == 3 { additive } else { alpha });
        }
        draw_mesh(&Mesh {
            vertices,
            indices: b.indices.iter().map(|&i| i as u16).collect(),
            texture,
        });
        gl_use_default_material();
    }
}

fn texture_stats(t: &libssbm_assets::model::Texture) -> (usize, u8, u8, usize) {
    let mut visible = 0;
    let mut min_alpha = 255u8;
    let mut max_alpha = 0u8;
    let mut colors = std::collections::BTreeSet::new();
    for px in t.rgba.chunks_exact(4) {
        if px[3] > 8 {
            visible += 1;
            min_alpha = min_alpha.min(px[3]);
            max_alpha = max_alpha.max(px[3]);
            colors.insert((px[0] / 16, px[1] / 16, px[2] / 16));
        }
    }
    (visible, min_alpha, max_alpha, colors.len())
}

#[macroquad::main(config)]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let capture = args.get(1).filter(|s| !s.is_empty()).cloned();
    let frame = args
        .get(2)
        .map(|s| s.parse::<f32>().unwrap())
        .unwrap_or(20.);
    let mode = args.get(3).map(String::as_str).unwrap_or("article");

    // --- Diagnostics: what is in the article and the trail bank? ------------------------------
    if let Ok(bytes) = std::fs::read(dir.join(fighter::PIKACHU.data.file))
        && let Ok(d) = libssbm_assets::dat::Dat::parse(&bytes)
        && let Ok(root) = d.root(fighter::PIKACHU.data.root)
        && let Ok(pk) = d.required_ptr(root + 4)
    {
        println!(
            "ftPikachuAttributes: specialn_itkind {} specialairn_itkind {} spawn offsets {:?} {:?}",
            d.u32(pk + 0x14).unwrap_or(0),
            d.u32(pk + 0x18).unwrap_or(0),
            d.f32(pk).unwrap_or(0.),
            d.f32(pk + 8).unwrap_or(0.)
        );
    }
    for kind in 0..3u32 {
        match Presentation::article_for(dir, kind, 0, &fighter::PIKACHU) {
            Ok(a) => {
                let tracks = a
                    .animations
                    .clips
                    .get("Object")
                    .map(|c| (c.frames, c.looping, c.tracks.len()))
                    .unwrap_or((0., false, 0));
                println!(
                    "article {kind}/0: joints {} batches {} object frames {:.1} loop {} joint-tracks {} material-tracks {} duration {:.1}",
                    a.model.joints.len(),
                    a.model.batches.len(),
                    tracks.0,
                    tracks.1,
                    tracks.2,
                    a.material_tracks.len(),
                    a.duration
                );
                for (i, b) in a.model.batches.iter().enumerate() {
                    println!(
                        "  batch {i}: dobj {} material {} owner {} texture {} frames {} blend {} tex-alpha {}",
                        b.dobj,
                        b.material_dobj,
                        b.owner,
                        b.texture.is_some(),
                        b.texture_frames.len(),
                        b.blend,
                        b.texture_alpha
                    );
                }
                for t in &a.material_tracks {
                    println!(
                        "  material track joint {} channel {}",
                        t.joint, t.channel
                    );
                }
            }
            Err(e) => println!("article {kind}/0 unavailable: {e}"),
        }
    }
    let bank = match Effects::load_archive(dir, fighter::PIKACHU.effects) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("EfPkData unavailable: {e}");
            return;
        }
    };
    println!(
        "EfPkData: models {}, emitters {}, texture groups {}",
        bank.models.len(),
        bank.emitters.len(),
        bank.textures.len()
    );
    if let Some(e) = bank.emitters.get(&7000) {
        println!(
            "emitter 7000: kind {:#x} shape {} tex {} gen-life {} life {} size {:.3} radius {:.3} random {:.3} velocity {:?} params {:?} gravity {:.4} friction {:.4} commands {}",
            e.kind,
            e.shape,
            e.texture,
            e.generator_life,
            e.life,
            e.size,
            e.radius,
            e.random,
            e.velocity,
            e.params,
            e.gravity,
            e.friction,
            e.commands.len()
        );
        println!("  commands {:02x?}", e.commands);
        if let Some(Some(group)) = bank.textures.get(e.texture as usize) {
            println!(
                "  texture group {}: frames {}, palettes {}",
                e.texture,
                group.frames.len(),
                group.palettes.len()
            );
            for (i, t) in group.frames.iter().enumerate().take(8) {
                let (visible, lo, hi, colors) = texture_stats(t);
                println!(
                    "  frame {i}: {}x{} visible px {} alpha {lo}..{hi} colors {colors}",
                    t.width, t.height, visible
                );
            }
        }
    }
    for id in 7000u32..=7009 {
        if let Some(e) = bank.emitters.get(&id) {
            println!(
                "emitter {id}: kind {:#x} shape {} tex {} gen-life {} life {} size {:.3} radius {:.3} angle {:.3} random {:.3} velocity {:?} params {:?} gravity {:.4} friction {:.4} commands {:02x?}",
                e.kind,
                e.shape,
                e.texture,
                e.generator_life,
                e.life,
                e.size,
                e.radius,
                e.angle,
                e.random,
                e.velocity,
                e.params,
                e.gravity,
                e.friction,
                e.commands
            );
        }
    }

    let mut article = Presentation::article_for(dir, if mode == "bolt" || mode == "boltstretch" || mode == "boltstack" { 0 } else { 2 }, 0, &fighter::PIKACHU)
        .expect("Pikachu article");
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();
    let mut renderer = particles::Renderer::new(&bank).unwrap();
    // Match the host: 7002 is drawn as a soft, centred radial glow instead of its
    // corner-centred source texture.
    renderer.set_glow(7002, 0.45, 0.60);
    let mut system = libssbm::particle::System::default();
    system.definitions = bank.emitters.clone();

    // Fixed simulation of the shot: a straight line at the disc's x8 speed (host view).
    let speed = 3.4f32; // itPikachutJoltGroundAttributes.x8 (sim units/tick)
    let mut point = if mode == "trail" {
        ::glam::Vec3::new(-34., 8., 0.)
    } else {
        ::glam::Vec3::new(0., 8., 0.)
    };
    let mut tick = 0f32;
    let emitter_id = mode
        .strip_prefix("emitter:")
        .and_then(|s| s.parse::<u32>().ok());
    if mode == "trail" || mode == "orb" {
        system.attach(1, ::glam::Mat4::from_translation(point));
        system.spawn(7000, ::glam::Mat4::from_translation(point), 1).unwrap();
        while tick < frame {
            system.tick();
            tick += 1.;
            if mode == "trail" {
                point.x += speed;
            }
            system.attach(1, ::glam::Mat4::from_translation(point));
        }
        println!("{mode} tick {frame}: {} particles", system.particles.len());
        for p in &system.particles {
            println!(
                "  p emitter {} attach {} pos {:?} tx {:?}",
                p.emitter,
                p.attachment,
                p.position,
                p.transform.w_axis
            );
            println!(
                "    tex {} frame {} palette {} kind {:#x} size {} rot {} color {:?} env {:?}",
                p.texture,
                p.frame,
                p.palette,
                p.kind,
                p.size,
                p.rotation,
                p.color,
                p.environment
            );
        }
    } else if let Some(id) = emitter_id {
        system.spawn(id, ::glam::Mat4::IDENTITY, 1).unwrap();
        while tick < frame {
            system.tick();
            tick += 1.;
            if tick % 10. == 0. {
                println!(
                    "emitter {id} tick {tick}: {} particles",
                    system.particles.len()
                );
            }
        }
        println!(
            "emitter {id} tick {frame}: {} particles",
            system.particles.len()
        );
    }

    let mut rendered = 0;
    loop {
        clear_background(Color::from_rgba(30, 34, 48, 255));
        let camera = match mode {
            "trail" => Camera3D {
                position: vec3(10., 28., 95.),
                target: vec3(10., 8., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            _ if emitter_id.is_some() => Camera3D {
                position: vec3(0., 8., 34.),
                target: vec3(0., 4., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            "orb" => Camera3D {
                position: vec3(0., 10., 45.),
                target: vec3(0., 6., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            "arc" => Camera3D {
                position: vec3(0., 12., 38.),
                target: vec3(0., 10., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            "bolt" => Camera3D {
                position: vec3(0., 24., 70.),
                target: vec3(0., 20., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            "boltstretch" => Camera3D {
                position: vec3(0., 35., 120.),
                target: vec3(0., 30., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            "boltstack" => Camera3D {
                position: vec3(0., 10., 45.),
                target: vec3(0., 8., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
            _ => Camera3D {
                position: vec3(0., 12., 48.),
                target: vec3(0., 10., 0.),
                up: Vec3::Y,
                fovy: 42.,
                ..Default::default()
            },
        };
        set_camera(&camera);
        draw_grid(
            120,
            10.,
            Color::from_rgba(55, 62, 82, 255),
            Color::from_rgba(40, 46, 62, 255),
        );
        if mode == "trail" {
            article.object_pose(frame);
            draw_model(
                &article,
                ::glam::Mat4::from_translation(point),
                &alpha,
                &additive,
            );
            renderer.draw(&system, camera.position);
        } else if mode == "arc" {
            // The host's bounce flash: article 2 (air jolt) at 0.6 scale for 24 ticks.
            article.object_pose(frame);
            // The air-jolt item gets its blue from the item colour overlay, which the
            // model viewer does not carry; tint the white source material to that blue.
            draw_model_tint(
                &article,
                ::glam::Mat4::from_translation(::glam::Vec3::new(0., 10., 0.))
                    * ::glam::Mat4::from_scale(::glam::Vec3::splat(0.6)),
                &alpha,
                &additive,
                [0.18, 0.55, 1.0],
            );
        } else if mode == "bolt" {
            article.object_pose(frame);
            draw_model(
                &article,
                ::glam::Mat4::from_translation(::glam::Vec3::new(0., 10., 0.)),
                &alpha,
                &additive,
            );
        } else if mode == "boltstretch" {
            // Host Thunder: one article 0 bolt stretched from the cloud to Pikachu.
            article.object_pose(frame);
            draw_model(
                &article,
                ::glam::Mat4::from_translation(::glam::Vec3::new(0., 20., 0.))
                    * ::glam::Mat4::from_scale(::glam::Vec3::splat(3.)),
                &alpha,
                &additive,
            );
        } else if mode == "boltstack" {
            // Host Thunder: four fixed 4 m article-0 segments stacked into a 16 m column.
            // The host forces additive, which drops the texture's black outline.
            for b in &mut article.model.batches { b.blend = 3; }
            article.object_pose(frame);
            for i in 0..4 {
                draw_model_tint(
                    &article,
                    ::glam::Mat4::from_translation(::glam::Vec3::new(0., 20. + 40. * i as f32, 0.)),
                    &alpha,
                    &additive,
                    [1., 0.92, 0.3],
                );
            }
        } else if mode == "orb" {
            renderer.draw(&system, camera.position);
        } else if emitter_id.is_some() {
            renderer.draw(&system, camera.position);
        } else {
            // Four copies at different object-animation frames: a static image shows four
            // identical models, an animated one differs.
            for (i, x) in [-27f32, -9., 9., 27.].into_iter().enumerate() {
                let f = frame * i as f32 / 3.;
                article.object_pose(f);
                draw_model(
                    &article,
                    ::glam::Mat4::from_translation(::glam::Vec3::new(x, 10., 0.)),
                    &alpha,
                    &additive,
                );
            }
        }
        set_default_camera();
        let label = match mode {
            "trail" => "Thunder Jolt trail | efSync 0x4BD -> EfPkData generator 7000 | article 2 model".to_string(),
            "orb" => format!("Thunder Jolt orb | efSync 0x4BD -> EfPkData generator 7000 | tick {frame:.0}"),
            "arc" => format!("Thunder Jolt bounce arc | article 2 air jolt | 24-tick flash | frame {frame:.0} | 0.6 scale"),
            "bolt" => format!("Pikachu Thunder | article 0 bolt model | frame {frame:.0}"),
            "boltstretch" => format!("Pikachu Thunder | article 0 stretched 3x | frame {frame:.0}"),
            "boltstack" => format!("Pikachu Thunder | four fixed 4 m article-0 segments | frame {frame:.0}"),
            _ => "Thunder Jolt orb | article 2 (air-jolt trail model) | Object animation frames".to_string(),
        };
        draw_text(&label, 20., 30., 24., WHITE);
        if mode != "trail" && mode != "orb" && mode != "arc" && emitter_id.is_none() {
            for i in 0..4 {
                draw_text(
                    &format!("frame {:.0}", frame * i as f32 / 3.),
                    155. + i as f32 * 260.,
                    665.,
                    22.,
                    WHITE,
                );
            }
        }
        rendered += 1;
        if rendered == 4
            && let Some(path) = capture.as_deref()
        {
            get_screen_data().export_png(path);
            println!("wrote {path}");
            break;
        }
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        next_frame().await;
    }
}
