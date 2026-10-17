use libssbm_assets::{Result, presentation::Presentation};
fn main() -> Result<()> {
    let dir = std::path::Path::new("assets");
    let animations = libssbm_assets::animation::Animations::load(dir)?;
    let actions = libssbm_assets::action::load(dir, &animations)?;
    for name in [
        "Catch",
        "CatchDash",
        "AirCatch",
        "CatchPull",
        "CatchWait",
        "CatchAttack",
        "ThrowF",
        "ThrowB",
        "ThrowHi",
        "ThrowLw",
    ] {
        println!("action {name}: {:?}", actions.get(name));
    }
    let co = std::fs::read(dir.join("PlCo.dat"))?;
    let co = libssbm_assets::dat::Dat::parse(&co)?;
    let colors = co.required_ptr(co.root("ftLoadCommonData")? + 0x18)?;
    let program = co.required_ptr(colors + 53 * 8)?;
    println!(
        "colour53 {program:x}: {:?}",
        (0..36)
            .map(|i| co.u32(program + i * 4).map(|v| format!("{v:08x}")))
            .collect::<Vec<_>>()
    );
    let bytes = std::fs::read(dir.join("EfSsData.dat"))?;
    let d = libssbm_assets::dat::Dat::parse(&bytes)?;
    let root = d.root("effSamusDataTable")?;
    let model = libssbm_assets::model::Model::from_joint(&d, d.required_ptr(root + 12)?)?;
    for j in &model.joints {
        if let Some(obj) = d.ptr(j.address + 16)? {
            let m = d.required_ptr(obj + 8)?;
            let t = d.required_ptr(m + 8)?;
            println!(
                "mobj {m:x} mode {:x} tobjflags {:x} blending {}",
                d.u32(m + 4)?,
                d.u32(t + 0x40)?,
                d.f32(t + 0x44)?
            );
        }
    }
    for i in 0..3 {
        let desc = root + 8 + i * 20;
        println!(
            "desc {i}: {:?}",
            (0..5).map(|n| d.u32(desc + n * 4)).collect::<Vec<_>>()
        );
    }
    let mut stack = d.ptr(root + 8 + 12)?.map(|p| vec![p]).unwrap_or_default();
    let mut joint = 0;
    while let Some(at) = stack.pop() {
        println!(
            "matjoint {joint} at {at:x}: {:?}",
            (0..3).map(|n| d.u32(at + n * 4)).collect::<Vec<_>>()
        );
        if let Some(next) = d.ptr(at + 4)? {
            stack.push(next);
        }
        if let Some(child) = d.ptr(at)? {
            stack.push(child);
        }
        joint += 1;
    }
    for id in [2000, 2001, 2002] {
        let mut p = Presentation::effect(dir, id)?;
        println!(
            "effect {id}: duration {} clips {:?} tracks {}",
            p.duration,
            p.animations.clips.keys().collect::<Vec<_>>(),
            p.material_tracks.len()
        );
        for frame in [5.] {
            p.object_pose(frame);
            for (i, b) in p.model.batches.iter().enumerate() {
                let skin = p.model.skin(i, &p.world);
                let lo = skin
                    .iter()
                    .fold(glam::Vec3::splat(f32::INFINITY), |a, (v, _)| a.min(*v));
                let hi = skin
                    .iter()
                    .fold(glam::Vec3::splat(f32::NEG_INFINITY), |a, (v, _)| a.max(*v));
                println!(
                    " frame {frame} batch {i} owner {} dobj {} vertices {} color {:?} blend {} bounds {lo:?} {hi:?} texture {:?}",
                    b.owner,
                    b.dobj,
                    skin.len(),
                    p.color(i),
                    b.blend,
                    b.texture.as_ref().map(|t| (
                        t.width,
                        t.height,
                        t.rgba.chunks_exact(4).filter(|p| p[3] > 0).count()
                    ))
                );
            }
        }
    }
    for (kind, state) in [(1, 0), (1, 8), (3, 0)] {
        let a = libssbm_assets::article::Article::load(dir, kind, state)?;
        println!(
            "article {kind} state {state}: common {:?} special {:?} commands {:?}",
            a.common, a.special, a.commands
        );
        for b in &a.model.batches {
            println!(
                " alpha {} mode {} blending {}",
                b.color[3], b.texture_alpha, b.texture_blend
            );
        }
    }
    Ok(())
}
