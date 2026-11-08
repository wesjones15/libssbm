//! Procedural vocabulary, no original asset/model/emitter IDs or external files.
//! cargo run --release -p ssbm-viewer --example session_fallbacks -- <output-dir>
use macroquad::prelude::*;
fn config() -> Conf { Conf { window_title: "Optional-assets fallback proof".into(), window_width: 1100, window_height: 760, ..Default::default() } }
fn cylinder(center: Vec3, radius: f32, height: f32, color: Color) { draw_cylinder(center, radius, radius, height, None, color); }
#[macroquad::main(config)]
async fn main() {
    let out = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&out).unwrap();
    for frame in 0..=241 {
        clear_background(Color::from_rgba(23,29,40,255));
        set_camera(&Camera3D { position: vec3(13.,11.,15.), target: vec3(0.,1.6,0.), up: Vec3::Y, ..Default::default() });
        draw_grid(24,1.,GRAY,DARKGRAY);
        // Idle -> ascending -> riding hidden -> dropped. Illustration of the host state contract.
        let flying = frame >= 100;
        let onion = vec3(-4., if flying { 4.8 } else { 1.5 }, 0.);
        draw_sphere(onion,0.7,None,BLUE);
        if !flying {
            for i in 0..3 { let a = i as f32 * std::f32::consts::TAU / 3.;
                draw_line_3d(onion, vec3(-4.+a.cos(),0.,a.sin()), WHITE); }
        }
        if frame < 100 || frame >= 190 {
            let y = if frame < 40 || frame >= 190 { 0.7 } else { 0.7+(frame-40) as f32/60.*1.2 };
            cylinder(vec3(-4.,y,0.),0.25,1.4,GOLD);
        }
        cylinder(vec3(0.,0.8,0.),0.35,1.6,RED);
        cylinder(vec3(0.,0.9,-0.4),0.2,0.8,YELLOW); // FLUDD backpack
        cylinder(vec3(0.,0.9,0.5),0.08,0.2,BLUE); // nozzle marker
        for (x,n) in [(2.,1),(4.,3),(6.,1)] { for i in 0..n { cylinder(vec3(x,0.2+i as f32*0.42,0.),0.3,0.36,ORANGE); } }
        draw_sphere(vec3(2.,2.4,-4.),0.45,None,YELLOW); // Shine
        cylinder(vec3(0.,0.02,3.),0.8,0.025,Color::new(0.08,0.1,0.12,0.6)); // wet footprint
        cylinder(vec3(2.,0.04,3.),0.8,0.06,Color::new(0.45,0.2,0.1,0.8)); // goop
        set_default_camera();
        draw_text("PROCEDURAL FALLBACKS | no original asset / cue / model IDs",24.,35.,25.,WHITE);
        draw_text("Orange flowers/tower | yellow Shine + FLUDD | wet/goop surface stand-ins",24.,66.,20.,LIGHTGRAY);
        let phase = if frame < 40 { "landed: legs + captain visible" } else if frame < 100 { "captain ascending" } else if frame < 190 { "flying: legs + captain hidden" } else { "dropped: captain visible again" };
        draw_text(&format!("Onion contract frame {frame}: {phase}"),24.,710.,22.,WHITE);
        if [20,70,140,220].contains(&frame) { get_screen_data().export_png(&format!("{out}/fallbacks-{frame}.png")); }
        next_frame().await;
    }
}
