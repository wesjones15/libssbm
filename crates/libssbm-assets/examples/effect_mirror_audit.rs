//! Development-only audit of texture mirroring across every extracted Melee effect bank.
//! Particles: the source sets MirrorS/MirrorT (psstructs.h kind bits 5/6) on a sprite whose texture is a half or
//! quarter of the picture, and psdisp.c draws it with GX_MIRROR and doubled coordinates; a host that clamps shows
//! only that part. Models: a TOBJ mirror wrap must reach the host (wrap 2) with UVs that span the mirrored range.
//! `cargo run --release -p libssbm-assets --example effect_mirror_audit -- <assets>`
use glam::Mat4;
use libssbm_assets::{effects::Effects, fighter};
use std::collections::BTreeMap;
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("assets"));
    let banks = [
        ("common (EfCo)", fighter::COMMON_EFFECTS),
        ("Samus", fighter::SAMUS.effects),
        ("Fox", fighter::FOX.effects),
        ("Luigi", fighter::LUIGI.effects),
        ("Pikachu", fighter::PIKACHU.effects),
        ("Captain Falcon", fighter::CAPTAIN.effects),
        ("Link", fighter::LINK.effects),
        ("Yoshi", fighter::YOSHI.effects),
        ("Donkey Kong", fighter::DONKEY.effects),
    ];
    for (name, archive) in banks {
        let fx = match Effects::load_archive(&dir, archive) {
            Ok(fx) => fx,
            Err(e) => {
                println!("== {name}: SKIP ({e})");
                continue;
            }
        };
        println!("== {name}: {} emitters, {} models", fx.emitters.len(), fx.models.len());
        // Particles: run each emitter and note which texture groups are drawn mirrored.
        let mut found: BTreeMap<u32, (u32, Vec<u16>)> = BTreeMap::new();
        let mut silent = 0;
        for &id in fx.emitters.keys() {
            let mut sys = libssbm::particle::System::default();
            sys.definitions = fx.emitters.clone();
            if sys.spawn(id, Mat4::IDENTITY, 1).is_err() {
                silent += 1;
                continue;
            }
            let mut modes = 0u32;
            let mut groups = Vec::new();
            for _ in 0..300 {
                sys.tick();
                for p in &sys.particles {
                    let m = p.kind & 0x60;
                    if m != 0 {
                        modes |= m;
                        if !groups.contains(&p.texture) {
                            groups.push(p.texture);
                        }
                    }
                }
            }
            if modes != 0 {
                found.insert(id, (modes, groups));
            }
        }
        for (id, (modes, groups)) in &found {
            let mode = match modes { 0x20 => "S", 0x40 => "T", 0x60 => "S+T", _ => "S and/or T (mixed)" };
            let sizes: Vec<String> = groups
                .iter()
                .map(|g| match fx.textures.get(*g as usize).and_then(|t| t.as_ref()).and_then(|t| t.frames.first().or_else(|| t.palettes.first().and_then(|p| p.first()))) {
                    Some(t) => format!("group {g} {}x{}", t.width, t.height),
                    None => format!("group {g}"),
                })
                .collect();
            println!("   particle emitter {id}: mirror {mode}; {}", sizes.join(", "));
        }
        println!("   {} of {} emitters draw mirrored sprites{}", found.len(), fx.emitters.len(), if silent > 0 { format!(" ({silent} failed to spawn)") } else { String::new() });
        // Models: texture wrap and UV range per batch.
        let mut bad = 0;
        let mut mirrored = 0;
        for (i, m) in fx.models.iter().enumerate() {
            for b in &m.model.batches {
                let Some(t) = b.texture.as_ref() else { continue };
                let (mut lo, mut hi) = (glam::Vec2::splat(f32::INFINITY), glam::Vec2::splat(f32::NEG_INFINITY));
                for v in &b.vertices {
                    lo = lo.min(v.uv);
                    hi = hi.max(v.uv);
                }
                let span = hi - lo;
                if t.wrap_s == 2 || t.wrap_t == 2 {
                    mirrored += 1;
                    let s_ok = t.wrap_s != 2 || span.x > 1.001;
                    let t_ok = t.wrap_t != 2 || span.y > 1.001;
                    if !s_ok || !t_ok {
                        bad += 1;
                        println!("   model {i}: mirror wrap {}/{} but UV span {:.2} x {:.2} (the mirrored half is never reached)", t.wrap_s, t.wrap_t, span.x, span.y);
                    }
                } else if (t.wrap_s == 1 && span.x > 1.05) || (t.wrap_t == 1 && span.y > 1.05) {
                    println!("   model {i}: clamp wrap but UV span {:.2} x {:.2} (edge smear)", span.x, span.y);
                }
            }
        }
        println!("   models: {mirrored} mirrored texture batches, {bad} where the UVs never reach the mirror");
    }
}
