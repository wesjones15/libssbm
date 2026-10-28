//! Held-item presentation shared by the viewer and native consumers.
use glam::Mat4;

/// Public article kind 3 maps to ftDataLink.x48_items[4], It_Kind_Link_Bow.
pub const LINK_BOW: u32 = 3;
/// Public article kind 4 maps to ftDataLink.x48_items[2], It_Kind_Link_HShot.
pub const LINK_HOOKSHOT: u32 = 4;
/// ftlinkspecialn.c::isDrawback attaches the bow to RThumbNb (not LThumbNb,
/// which holds the nocked arrow). Resolved through FighterPartsTable.
pub const LINK_BOW_PART: usize = 49;

/// itlinkbow.c::it_803F6E98 / ftLk_SpecialN_GetIndex. State 6 has motion -1
/// and destroys the bow. End motions remove it at frame 24.
pub fn link_bow_state(clip: &str, frame: f32) -> Option<u32> {
    if !frame.is_finite() || frame < 0. { return None; }
    let state = match clip {
        "SpecialN" | "SpecialNStart" => 0,
        "SpecialNLoop" => 1,
        "SpecialNEnd" => 2,
        "SpecialAirN" | "SpecialAirNStart" => 3,
        "SpecialAirNLoop" => 4,
        "SpecialAirNEnd" => 5,
        _ => return None,
    };
    if matches!(state, 2 | 5) && frame >= 24. { None } else { Some(state) }
}

/// it_80274F48 -> lb_8000C2F8 constrains the descriptor's attachment joint's
/// position and orientation to the hand, retaining the item's authored scale.
/// itLinkbow_UnkMotion5_Anim sets the root scale to the fighter's model scale.
/// Returns a transform for every item joint, in the coordinate space of `hand`.
pub fn attachment_transform(anchor: Mat4, hand: Mat4, model_scale: f32) -> Mat4 {
    let (scale, _, _) = anchor.to_scale_rotation_translation();
    let (_, rotation, position) = hand.to_scale_rotation_translation();
    Mat4::from_scale_rotation_translation(scale * model_scale, rotation, position)
        * anchor.inverse()
}

/// FighterPartsTable mapping shared with the FFI rig reader.
pub fn common_bones(dir: &std::path::Path, fighter: &crate::fighter::FighterAssets<'_>) -> crate::Result<Vec<u8>> {
    let bytes = std::fs::read(dir.join(crate::fighter::COMMON_DATA.file))?;
    let d = crate::dat::Dat::parse(&bytes)?;
    let table = d.required_ptr(d.root(crate::fighter::COMMON_DATA.root)? + 0x10)?;
    let parts = d.required_ptr(table + fighter.common_index * 4)?;
    Ok(d.bytes(d.required_ptr(parts + 4)?, 53)?.to_vec())
}

pub fn model_scale(dir: &std::path::Path, fighter: &crate::fighter::FighterAssets<'_>) -> crate::Result<f32> {
    let bytes = std::fs::read(dir.join(fighter.data.file))?;
    let d = crate::dat::Dat::parse(&bytes)?;
    let attrs = d.required_ptr(d.root(fighter.data.root)?)?;
    let scale = d.f32(attrs + 0x8c)?;
    if !scale.is_finite() || scale <= 0. { return Err("invalid fighter model scale".into()); }
    Ok(scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    #[test]
    fn bow_states_and_removal() {
        for (state, clip) in ["SpecialNStart", "SpecialNLoop", "SpecialNEnd", "SpecialAirNStart", "SpecialAirNLoop", "SpecialAirNEnd"].iter().enumerate() {
            assert_eq!(link_bow_state(clip, 8.), Some(state as u32));
        }
        for clip in ["SpecialNEnd", "SpecialAirNEnd"] {
            assert!(link_bow_state(clip, 23.).is_some());
            assert_eq!(link_bow_state(clip, 24.), None);
        }
        assert_eq!(link_bow_state("Wait", 0.), None);
        assert_eq!(link_bow_state("DamageFlyHi", 0.), None);
        assert_eq!(link_bow_state("SpecialN", f32::NAN), None);
    }
    #[test]
    fn constraint_aligns_anchor_and_keeps_item_scale() {
        let anchor = Mat4::from_translation(Vec3::new(2., 3., 4.));
        let hand = Mat4::from_rotation_y(0.7) * Mat4::from_translation(Vec3::new(5., 6., 7.));
        let result = attachment_transform(anchor, hand, 1.2) * anchor;
        assert!((result.w_axis - hand.w_axis).length() < 1e-5);
        assert!((result.x_axis.truncate().normalize() - hand.x_axis.truncate()).length() < 1e-5);
        assert!((result.x_axis.truncate().length() - 1.2).abs() < 1e-5);
    }
}
