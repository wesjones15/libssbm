//! Lists every effect id that resolves to a model with the model's source lifetime
//! (ef/eflib.c efLib_Create: EF_EffectDesc.lifetime frames, 0 = lives until its parent destroys
//! it) and whether the host treats the id as joint-bound. Fighter ids (1160..) are listed under
//! the fighter whose archive they index; shared ids once, under "common".
use libssbm::effect::{attached, companion, resolve, Target};
use libssbm_assets::{effects::Effects, fighter::*, Result};
fn main() -> Result<()> {
    let dir = std::path::Path::new("assets");
    let common = Effects::load_archive(dir, COMMON_EFFECTS)?;
    for id in 1000..1320u32 {
        for (second, t) in [Some(resolve(id)), companion(id)].into_iter().enumerate() {
            if let Some(Target::Model(m)) = t {
                if m < 2000 {
                    if let Some(model) = common.models.get(m as usize) {
                        println!("common: effect {id}{} -> model {m} lifetime {} attached {}", if second == 1 { " (companion)" } else { "" }, model.lifetime, attached(id));
                    }
                }
            }
        }
    }
    for (name, f, ids) in [
        ("samus", &SAMUS, 1154..=1159u32), ("fox", &FOX, 1160..=1165), ("captain", &CAPTAIN, 1167..=1171),
        ("link", &LINK, 1211..=1212), ("luigi", &LUIGI, 1289..=1290), ("donkey", &DONKEY, 1222..=1228), ("yoshi", &YOSHI, 1229..=1232),
    ] {
        if !f.complete(dir) { println!("SKIP {name}"); continue; }
        let bank = Effects::load_archive(dir, f.effects)?;
        for id in ids {
            for (second, t) in [Some(resolve(id)), companion(id)].into_iter().enumerate() {
                if let Some(Target::Model(m)) = t {
                    if m >= 2000 {
                        if let Some(model) = bank.models.get(m as usize - 2000) {
                            let anim = model.animations.clips.get("Object").map(|c| c.frames);
                            println!("{name}: effect {id}{} -> model {m} lifetime {} anim {anim:?} attached {}", if second == 1 { " (companion)" } else { "" }, model.lifetime, attached(id));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
