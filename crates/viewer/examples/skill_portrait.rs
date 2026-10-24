//! Local-disc proof for generated skill portraits: a close-up of the move's
//! source effect or article, with part of the fighter posed behind it.
//!
//! `cargo run --release -p ssbm-viewer --example skill_portrait -- assets out.png samus charge`
//! Skills: charge, bomb, missile, screw, shield, fireball, blaster, ...
//! The icon is composed from the same extracted assets the host loads, so no
//! Nintendo content is baked into the repository.
use libssbm_assets::{effects::Effects, fighter, presentation::Presentation};
use macroquad::prelude::*;
#[path = "../src/models.rs"] mod models;
#[path = "../src/particles.rs"] mod particles;

fn config() -> Conf {
    Conf { window_title: "Melee skill portrait".into(), window_width: 512, window_height: 512, ..Default::default() }
}

fn sv(v: ::glam::Vec3) -> Vec3 { vec3(v.x, v.y, v.z) }

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

struct Spec { fighter: &'static fighter::FighterAssets<'static>, clip: &'static str, frame: f32, effect: EffectSpec, offset: ::glam::Vec3, char_offset: ::glam::Vec3, yaw: f32, zoom: f32, show_body: bool, trail: Option<u32>, trail_offset: ::glam::Vec3 }
enum EffectSpec { Article(u32, u32), Effect(u32), None }

fn spec(name: &str, skill: &str) -> Option<Spec> {
    let s = match name {
        "samus" => match skill {
            "charge" => Spec { fighter: &fighter::SAMUS, clip: "SpecialNHold", frame: 20., effect: EffectSpec::Article(1, 0), char_offset: ::glam::Vec3::new(-2., 0., 1.), show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(3., 8., 7.), yaw: 0.45, zoom: 1.5 },
            // The bomb is a small flat disc at icon size; Morph Ball Samus reads
            // as the move and is the form the bomb belongs to.
            "bomb" => Spec { fighter: &fighter::SAMUS, clip: "SpecialLw", frame: 20., effect: EffectSpec::None, char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::ZERO, yaw: 0.5, zoom: 1.0 },
            "missile" => Spec { fighter: &fighter::SAMUS, clip: "SpecialS", frame: 8., effect: EffectSpec::Article(2, 0), char_offset: ::glam::Vec3::new(-2., 0., 1.), show_body: false, trail: Some(2014), trail_offset: ::glam::Vec3::new(2., 7., 2.), offset: ::glam::Vec3::new(2., 7., 8.), yaw: 0.35, zoom: 1.4 },
            "screw" => Spec { fighter: &fighter::SAMUS, clip: "SpecialHi", frame: 10., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::new(0., 8., 0.), show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            "shield" => Spec { fighter: &fighter::SAMUS, clip: "GuardOn", frame: 4., effect: EffectSpec::Effect(14), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.2 },
            _ => return None,
        },
        "fox" => match skill {
            "blaster" => Spec { fighter: &fighter::FOX, clip: "SpecialNStart", frame: 2., effect: EffectSpec::Effect(18), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::ZERO, yaw: 0.4, zoom: 1.2 },
            "reflector" => Spec { fighter: &fighter::FOX, clip: "SpecialLwStart", frame: 4., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            "illusion" => Spec { fighter: &fighter::FOX, clip: "SpecialS", frame: 2., effect: EffectSpec::Article(2, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 4.), yaw: 0.4, zoom: 1.0 },
            "firefox" => Spec { fighter: &fighter::FOX, clip: "SpecialHi", frame: 8., effect: EffectSpec::Effect(2004), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            _ => return None,
        },
        "link" => match skill {
            "bow" => Spec { fighter: &fighter::LINK, clip: "SpecialNStart", frame: 8., effect: EffectSpec::Article(0, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 8., 8.), yaw: 0.4, zoom: 0.9 },
            "bomb" => Spec { fighter: &fighter::LINK, clip: "SpecialLw", frame: 6., effect: EffectSpec::Article(2, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 4.), yaw: 0.4, zoom: 1.0 },
            "boomerang" => Spec { fighter: &fighter::LINK, clip: "SpecialS1", frame: 4., effect: EffectSpec::Article(1, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 8., 7.), yaw: 0.4, zoom: 0.9 },
            "spin" => Spec { fighter: &fighter::LINK, clip: "SpecialHi", frame: 8., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            _ => return None,
        },
        "pikachu" => match skill {
            "jolt" => Spec { fighter: &fighter::PIKACHU, clip: "SpecialN", frame: 4., effect: EffectSpec::Article(1, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 6.), yaw: 0.4, zoom: 1.0 },
            "thunder" => Spec { fighter: &fighter::PIKACHU, clip: "SpecialLwStart", frame: 4., effect: EffectSpec::Article(0, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 10., 0.), yaw: 0.4, zoom: 0.8 },
            "skullbash" => Spec { fighter: &fighter::PIKACHU, clip: "SpecialS", frame: 8., effect: EffectSpec::None, char_offset: ::glam::Vec3::ZERO, show_body: true, trail: Some(7005), trail_offset: ::glam::Vec3::new(0., 0., 2.), offset: ::glam::Vec3::ZERO, yaw: 0.4, zoom: 1.0 },
            "quick" => Spec { fighter: &fighter::PIKACHU, clip: "SpecialHiStart", frame: 2., effect: EffectSpec::Effect(1042), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            _ => return None,
        },
        "luigi" => match skill {
            "fireball" => Spec { fighter: &fighter::LUIGI, clip: "SpecialN", frame: 4., effect: EffectSpec::Article(0, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 6.), yaw: 0.4, zoom: 1.0 },
            "cyclone" => Spec { fighter: &fighter::LUIGI, clip: "SpecialLwStart", frame: 6., effect: EffectSpec::Effect(2001), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            "missile" => Spec { fighter: &fighter::LUIGI, clip: "SpecialS", frame: 8., effect: EffectSpec::None, char_offset: ::glam::Vec3::ZERO, show_body: true, trail: Some(95), trail_offset: ::glam::Vec3::new(0., 0., 2.), offset: ::glam::Vec3::ZERO, yaw: 0.4, zoom: 1.0 },
            "punch" => Spec { fighter: &fighter::LUIGI, clip: "SpecialHi", frame: 6., effect: EffectSpec::Effect(1154), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            _ => return None,
        },
        "captain" => match skill {
            "punch" => Spec { fighter: &fighter::CAPTAIN, clip: "SpecialN", frame: 20., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 7., 6.), yaw: 0.4, zoom: 1.0 },
            "kick" => Spec { fighter: &fighter::CAPTAIN, clip: "SpecialLw", frame: 10., effect: EffectSpec::Effect(2004), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 5., 6.), yaw: 0.4, zoom: 1.0 },
            "raptor" => Spec { fighter: &fighter::CAPTAIN, clip: "SpecialS", frame: 8., effect: EffectSpec::Effect(2005), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 4.), yaw: 0.4, zoom: 1.0 },
            "dive" => Spec { fighter: &fighter::CAPTAIN, clip: "SpecialHi", frame: 10., effect: EffectSpec::Effect(2002), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            _ => return None,
        },
        "yoshi" => match skill {
            "egglay" => Spec { fighter: &fighter::YOSHI, clip: "SpecialN1", frame: 14., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 5.), yaw: 0.4, zoom: 1.0 },
            "bomb" => Spec { fighter: &fighter::YOSHI, clip: "SpecialLw", frame: 8., effect: EffectSpec::Article(1, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 4., 4.), yaw: 0.4, zoom: 1.0 },
            "roll" => Spec { fighter: &fighter::YOSHI, clip: "SpecialSStart", frame: 6., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 5., 0.), yaw: 0.4, zoom: 1.0 },
            "egg" => Spec { fighter: &fighter::YOSHI, clip: "SpecialHi", frame: 6., effect: EffectSpec::Article(0, 0), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 8., 6.), yaw: 0.4, zoom: 0.9 },
            _ => return None,
        },
        "donkey" => match skill {
            "punch" => Spec { fighter: &fighter::DONKEY, clip: "SpecialN", frame: 12., effect: EffectSpec::Effect(2002), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 7., 6.), yaw: 0.4, zoom: 1.0 },
            "slap" => Spec { fighter: &fighter::DONKEY, clip: "SpecialLwStart", frame: 6., effect: EffectSpec::Effect(2000), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 3., 5.), yaw: 0.4, zoom: 1.0 },
            "headbutt" => Spec { fighter: &fighter::DONKEY, clip: "SpecialS", frame: 6., effect: EffectSpec::Effect(2006), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 7., 5.), yaw: 0.4, zoom: 1.0 },
            "kong" => Spec { fighter: &fighter::DONKEY, clip: "SpecialHi", frame: 8., effect: EffectSpec::Effect(2004), char_offset: ::glam::Vec3::ZERO, show_body: true, trail: None, trail_offset: ::glam::Vec3::ZERO, offset: ::glam::Vec3::new(0., 6., 0.), yaw: 0.4, zoom: 1.0 },
            _ => return None,
        },
        _ => return None,
    };
    Some(s)
}

fn tint(fighter: &str, skill: &str) -> (f32, f32, f32) {
    match fighter {
        "samus" => match skill {
            "charge" => (0.95, 0.45, 0.10),
            "bomb" => (0.95, 0.55, 0.15),
            "missile" => (0.85, 0.25, 0.20),
            "screw" => (0.30, 0.75, 0.45),
            "shield" => (0.25, 0.55, 0.95),
            _ => (0.90, 0.50, 0.20),
        },
        "fox" => (0.30, 0.75, 0.40),
        "luigi" => (0.30, 0.75, 0.35),
        "pikachu" => (0.95, 0.78, 0.18),
        "captain" => (0.90, 0.25, 0.20),
        "link" => (0.30, 0.70, 0.35),
        "yoshi" => (0.35, 0.75, 0.40),
        "donkey" => (0.65, 0.42, 0.22),
        _ => (0.40, 0.50, 0.60),
    }
}

#[macroquad::main(config)]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::Path::new(args.first().map(String::as_str).unwrap_or("assets"));
    let out = args.get(1).cloned().unwrap_or_else(|| "build/skill-portrait.png".into());
    let fighter_name = args.get(2).map(String::as_str).unwrap_or("samus");
    let skill = args.get(3).map(String::as_str).unwrap_or("charge");
    let char_yaw: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.);
    let no_body = args.iter().any(|s| s == "nobody");
    let Some(spec) = spec(fighter_name, skill) else {
        eprintln!("unknown skill {fighter_name}/{skill}");
        return;
    };
    let ball = skill == "bomb" || skill == "roll";
    let mut body = Presentation::load_for_variant(dir, 0, spec.fighter, ball).unwrap();
    body.clip = spec.clip.into();
    body.frame = spec.frame;
    body.world = body.animations.pose(&body.model, spec.clip, spec.frame, false);
    body.ball = ball;
    let mut effect = match spec.effect {
        EffectSpec::Article(kind, state) => Some(Presentation::article_for(dir, kind, state, spec.fighter).unwrap()),
        EffectSpec::Effect(id) => Some(Presentation::effect_for(dir, id, spec.fighter).unwrap()),
        EffectSpec::None => None,
    };
    if let Some(e) = effect.as_mut() { e.object_pose(4.); }
    let mut particle_renderer = None;
    let mut particle_system = None;
    if let Some(id) = spec.trail {
        let bank = if id >= 2000 {
            Effects::load_archive(dir, spec.fighter.effects).unwrap()
        } else {
            Effects::load(dir, false).unwrap()
        };
        let mut system = libssbm::particle::System::default();
        system.definitions = bank.emitters.clone();
        system.spawn(id, ::glam::Mat4::from_translation(spec.trail_offset), 1).unwrap();
        let ticks = match (fighter_name, skill) { ("samus", "missile") => 10, ("luigi", "missile") | ("pikachu", "skullbash") => 3, _ => 20 };
        for _ in 0..ticks { system.tick(); }
        particle_renderer = Some(particles::Renderer::new(&bank).unwrap());
        particle_system = Some(system);
    }
    let alpha = models::blended(false).unwrap();
    let additive = models::blended(true).unwrap();

    // The effect's posed bounds give the icon's framing; the fighter sits behind it.
    // With no effect (Morph Ball) the posed character is the subject.
    let source = effect.as_ref().unwrap_or(&body);
    let points: Vec<_> = (0..source.model.batches.len())
        .flat_map(|i| source.model.skin(i, &source.world).into_iter().map(|(p, _)| p))
        .collect();
    let lo = points.iter().fold(::glam::Vec3::splat(f32::INFINITY), |a, p| a.min(*p));
    let hi = points.iter().fold(::glam::Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
    let effect_center = (lo + hi) * 0.5 + spec.offset;
    let radius = ((hi - lo).max_element() * 0.5).max(1.);
    let distance = radius * 1.55 / spec.zoom;
    eprintln!("effect bounds {lo:?}..{hi:?} center {effect_center:?} radius {radius:.2} distance {distance:.2}");

    loop {
        clear_background(Color::from_rgba(18, 22, 30, 255));
        // RoR2 skill icons are square. A full-bleed square gradient in the
        // skill's colour gives a small effect something to sit on.
        let (tr, tg, tb) = tint(fighter_name, skill);
        for i in (0..240).rev() {
            let t = i as f32 / 240.;
            let core = 1. - t;
            let c = Color::new(0.035 + tr * (0.08 + 0.30 * core), 0.050 + tg * (0.08 + 0.30 * core), 0.075 + tb * (0.08 + 0.30 * core), 1.);
            draw_rectangle(256. - i as f32, 256. - i as f32, (i * 2) as f32, (i * 2) as f32, c);
        }
        let camera = sv(effect_center) + vec3(spec.yaw.sin() * distance, distance * 0.20, spec.yaw.cos() * distance);
        let missile = fighter_name == "samus" && skill == "missile";
        if let Some(e) = effect.as_mut() {
            if !missile { e.face_camera(::glam::Vec3::new(camera.x, camera.y, camera.z) - spec.offset); }
        }
        set_camera(&Camera3D { position: camera, target: sv(effect_center), up: Vec3::Y, fovy: 30., ..Default::default() });
        if spec.show_body && !no_body {
            draw(&body, ::glam::Mat4::from_translation(spec.char_offset), &alpha, &additive);
        }
        if let Some(e) = effect.as_ref() {
            let xform = if missile {
                ::glam::Mat4::from_translation(spec.offset) * ::glam::Mat4::from_rotation_y(-1.45) * ::glam::Mat4::from_rotation_z(0.22)
            } else { ::glam::Mat4::from_translation(spec.offset) };
            draw(e, xform, &alpha, &additive);
        }
        if let (Some(renderer), Some(system)) = (&particle_renderer, &particle_system) {
            renderer.draw(system, camera);
        }
        set_default_camera();
        // A square frame, matching RoR2's skill slots.
        draw_rectangle_lines(8., 8., 496., 496., 3., Color::from_rgba(230, 180, 100, 200));
        if get_frame_time() > 0. {
            get_screen_data().export_png(&out);
            println!("wrote {out}");
            break;
        }
        next_frame().await;
    }
}
