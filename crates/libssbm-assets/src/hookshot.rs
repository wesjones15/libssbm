//! Link's source hookshot chain and hook head. The held item model is public
//! article 4 (PlLk.dat item slot 2, It_Kind_Link_HShot); the links are separate
//! joint subtrees in the item's special attributes. Source:
//! it/kinds/itlinkhookshot.c::it_802A2568 / it_link_get_joint / it_link_get_joint_c
//! (x54 and x58 alternate along the chain, x5C is the last link).
use crate::{Result, animation::Animations, dat::Dat, model::Model, presentation::Presentation};
use std::path::Path;

/// Chain link models alternate between `LinkA` and `LinkB`.
pub const LINK_A: u32 = 0;
pub const LINK_B: u32 = 1;
/// The hook head at the far end (`it_link_get_joint_c`, x5C).
pub const HOOK: u32 = 2;

fn attributes(d: &Dat) -> Result<usize> {
    let items = d.required_ptr(d.root("ftDataLink")? + 0x48)?;
    // The public host kinds map through article.rs::disc_slot: slot 2 is Hookshot.
    let article = d.required_ptr(items + 2 * 4)?;
    d.required_ptr(article + 4)
}

pub fn part(dir: &Path, part: u32) -> Result<Presentation> {
    if part > HOOK {
        return Err("invalid hookshot part".into());
    }
    let bytes = std::fs::read(dir.join("PlLk.dat"))?;
    let d = Dat::parse(&bytes)?;
    let a = attributes(&d)?;
    let joint = d.required_ptr(a + 0x54 + part as usize * 4)?;
    let mut model = Model::from_joint(&d, joint)?;
    let animations = Animations::object(&d, None, &model)?;
    let material_tracks = Animations::materials(&d, None, &mut model)?;
    let world = model.bind_world();
    Ok(Presentation {
        attach_joint: None,
        model,
        animations,
        clip: "Object".into(),
        frame: 0.,
        world,
        ball: false,
        morph_move: None,
        spin_joint: None,
        hidden: false,
        material_tracks,
        duration: 0.,
        commands: vec![],
        object_scale: 1.,
        upper_body_root: None,
        effect_fade: false,
        face_tracks: Vec::new(),
        face: [0.; 8],
        part_clips: Default::default(),
        parts: [None; 5],
    })
}
