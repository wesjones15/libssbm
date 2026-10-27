//! Disc-backed held bow identity, animation and attachment checks. No disc data
//! is bundled; absent extracted caches explicitly skip these checks.
use libssbm_assets::{fighter::LINK, held, presentation::Presentation, Result};

#[test]
fn link_bow_all_six_states_animate_at_the_authored_hand_anchor() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !LINK.complete(&dir) { eprintln!("SKIP: Link disc cache absent"); return Ok(()); }
    let fighter = Presentation::load_for(&dir, 0, &LINK)?;
    let bones = held::common_bones(&dir, &LINK)?;
    let scale = held::model_scale(&dir, &LINK)?;
    // Verify the public bow maps to the disc's actual fifth item descriptor.
    let bytes = std::fs::read(dir.join("PlLk.dat"))?;
    let d = libssbm_assets::dat::Dat::parse(&bytes)?;
    let table = d.required_ptr(d.root("ftDataLink")? + 0x48)?;
    let item = d.required_ptr(table + 4 * 4)?;
    let descriptor = d.required_ptr(item + 16)?;
    let joint = d.required_ptr(descriptor)?;
    let clips = ["SpecialN", "SpecialNLoop", "SpecialNEnd", "SpecialAirN", "SpecialAirNLoop", "SpecialAirNEnd"];
    for (state, clip) in clips.iter().enumerate() {
        let mut bow = Presentation::article_for(&dir, held::LINK_BOW, state as u32, &LINK)?;
        assert_eq!(bow.model.joints[0].address, joint);
        assert_eq!(bow.attach_joint, Some(d.u32(descriptor + 8)? as usize));
        assert!(!bow.model.batches.is_empty());
        assert!(bow.duration > 0., "bow state {state} has no animation");
        assert!(fighter.animations.clips.contains_key(*clip));
        let mut raw_samples = Vec::new();
        for frame in [0., 8., 18., 23.] {
            let pose = fighter.animations.pose(&fighter.model, clip, frame, false);
            let hand = pose[bones[held::LINK_BOW_PART] as usize];
            bow.object_pose(frame);
            // Independently follow pobj.c::SetupEnvelopeModelMtx for a blended
            // vertex under the bow's skeleton root. The descriptor's SRT pose
            // is not its skin bind pose; reconstructing an inverse stretches
            // the bow into a long triangle while still yielding finite bounds.
            let batch = &bow.model.batches[0];
            let vertex_index = batch.vertices.iter().position(|v| v.weights.len() > 1).unwrap();
            let vertex = &batch.vertices[vertex_index];
            let mut expected = glam::Vec3::ZERO;
            for &(joint, weight) in &vertex.weights {
                let matrix = d.required_ptr(bow.model.joints[joint].address + 0x38)?;
                let mut values = [0.; 16];
                for row in 0..3 { for col in 0..4 {
                    values[col*4+row] = d.f32(matrix + (row*4+col)*4)?;
                } }
                values[15] = 1.;
                expected += (bow.world[joint] * glam::Mat4::from_cols_array(&values))
                    .transform_point3(vertex.position) * weight;
            }
            let actual = bow.model.skin(0, &bow.world)[vertex_index].0;
            assert!((actual-expected).length() < 1e-4, "state {state}, frame {frame}: authored envelope was ignored");
            raw_samples.push(bow.world.clone());
            bow.attach_to(frame, hand, scale)?;
            let anchor = bow.world[bow.attach_joint.unwrap()];
            assert!((anchor.w_axis - hand.w_axis).length() < 1e-3, "state {state}, frame {frame}: anchor detached");
            for axis in 0..3 {
                assert!((anchor.col(axis).truncate().normalize() - hand.col(axis).truncate().normalize()).length() < 1e-3);
            }
            for (i, _) in bow.model.batches.iter().enumerate() {
                for (p, n) in bow.model.skin(i, &bow.world) {
                    assert!(p.is_finite() && n.is_finite());
                    assert!((p - hand.w_axis.truncate()).length() < 100., "state {state}, frame {frame}: bad bounds {p:?}");
                }
            }
            // Repeating an update must not compound scale or translation.
            let first = bow.world.clone();
            bow.attach_to(frame, hand, scale)?;
            assert_eq!(first, bow.world);
        }
        assert!(raw_samples.windows(2).any(|pair| pair[0].iter().zip(&pair[1]).any(|(a,b)| (*a-*b).to_cols_array().iter().any(|v| v.abs() > 1e-4))), "state {state}: frozen bow animation");
    }
    assert!(Presentation::article_for(&dir, held::LINK_BOW, 6, &LINK).is_err());
    // Uniform model scaling must preserve source skinning, rather than
    // replacing authored envelope matrices with reconstructed SRT inverses.
    let mut a = libssbm_assets::article::Article::load_for(&dir, held::LINK_BOW, 1, &LINK)?;
    let original_world = a.animations.pose_object(&a.model, 8.);
    let original = a.model.skin(0, &original_world);
    a.model.scale(1.22);
    // Model::scale scales the skeleton's translations; use the correspondingly
    // scaled original pose to isolate the skinning and inverse-matrix contract.
    let s = glam::Mat4::from_scale(glam::Vec3::splat(1.22));
    let world: Vec<_> = original_world.iter().map(|m| s * *m * s.inverse()).collect();
    let scaled = a.model.skin(0, &world);
    for ((p,_),(q,_)) in original.iter().zip(&scaled) {
        assert!((*p*1.22-*q).length() < 1e-3, "scaling lost authored envelope bind");
    }
    // Public projectile identities remain independent from the new held item.
    for kind in 0..3 { assert!(Presentation::article_for(&dir, kind, 0, &LINK)?.model.batches.len() > 0); }
    Ok(())
}
