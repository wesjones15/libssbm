use libssbm_assets::{
    REQUIRED, Result,
    animation::Animations,
    dat::Dat,
    disc::{Disc, extract},
    model::Model,
};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn word(b: &mut [u8], at: usize, n: usize) {
    b[at..at + 4].copy_from_slice(&(n as u32).to_be_bytes());
}
fn synthetic_dat() -> Vec<u8> {
    let mut b = vec![0; 49];
    word(&mut b, 0, 49);
    word(&mut b, 4, 4);
    word(&mut b, 12, 1);
    b[44..].copy_from_slice(b"root\0");
    b
}
fn synthetic_disc() -> Vec<u8> {
    let mut b = vec![0; 0x6000];
    b[..6].copy_from_slice(b"GALE01");
    b[7] = 2;
    word(&mut b, 0x1c, 0xc2339f3d);
    word(&mut b, 0x424, 0x1000);
    // Real FSTs use directory entries, rather than slash-containing names.
    let count = REQUIRED.len() + 3;
    let mut fst = vec![0; 12 * count];
    fst[0] = 1;
    word(&mut fst, 8, count);
    let mut names = Vec::new();
    let mut entry = 1;
    for (i, name) in REQUIRED.iter().enumerate() {
        if *name == "audio/us/samus.ssm" {
            for (dir, parent) in [("audio", 0), ("us", entry)] {
                word(&mut fst, entry * 12, (1 << 24) | names.len());
                word(&mut fst, entry * 12 + 4, parent);
                word(&mut fst, entry * 12 + 8, count);
                names.extend(dir.as_bytes());
                names.push(0);
                entry += 1;
            }
        }
        let payload = if name.ends_with(".ssm") {
            vec![0; 32]
        } else if name.ends_with(".sem") {
            vec![0; 20]
        } else {
            synthetic_dat()
        };
        word(&mut fst, entry * 12, names.len());
        word(&mut fst, entry * 12 + 4, 0x2000 + i * 128);
        word(&mut fst, entry * 12 + 8, payload.len());
        names.extend(name.rsplit('/').next().unwrap().as_bytes());
        names.push(0);
        b[0x2000 + i * 128..0x2000 + i * 128 + payload.len()].copy_from_slice(&payload);
        entry += 1;
    }
    fst.extend(names);
    word(&mut b, 0x428, fst.len());
    b[0x1000..0x1000 + fst.len()].copy_from_slice(&fst);
    b
}
#[test]
fn iso_ciso_roundtrip_and_invalid_images() -> Result<()> {
    let path = root().join("build/synthetic");
    std::fs::create_dir_all(&path)?;
    let iso = synthetic_disc();
    std::fs::write(path.join("disc.iso"), &iso)?;
    let mut ciso = vec![0; 0x8000];
    ciso[..4].copy_from_slice(b"CISO");
    ciso[4..8].copy_from_slice(&0x2000u32.to_le_bytes());
    ciso[8..11].fill(1);
    ciso.extend(&iso);
    std::fs::write(path.join("disc.ciso"), &ciso)?;
    for name in ["disc.iso", "disc.ciso"] {
        let mut disc = Disc::open(&path.join(name))?;
        assert_eq!(disc.file("PlSs.dat")?, synthetic_dat());
        extract(&path.join(name), &path.join("out"))?;
        assert!(libssbm_assets::complete(&path.join("out")));
        // Version-1 caches must trigger DLL extraction of new dependencies.
        std::fs::remove_file(path.join("out/audio/us/samus.ssm"))?;
        assert!(!libssbm_assets::complete(&path.join("out")));
        extract(&path.join(name), &path.join("out"))?;
        assert!(libssbm_assets::complete(&path.join("out")));
    }
    let mut wrong = iso.clone();
    wrong[..6].copy_from_slice(b"GMSE01");
    std::fs::write(path.join("wrong.iso"), wrong)?;
    assert!(Disc::open(&path.join("wrong.iso")).is_err());
    std::fs::write(path.join("short.ciso"), &ciso[..0x9000])?;
    assert!(Disc::open(&path.join("short.ciso")).is_err());
    assert!(extract(&path.join("disc.iso"), &root().join("would-track-assets")).is_err());
    Ok(())
}
#[test]
fn real_assets_decode_all_costumes_and_animation_poses() -> Result<()> {
    let dir = root().join("assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let p = libssbm_assets::params::load(&dir)?;
    p.validate()?;
    let a = Animations::load(&dir)?;
    assert!(a.clips.contains_key("Wait"));
    let actions = libssbm_assets::action::load(&dir, &a)?;
    for name in [
        "Attack11",
        "Attack12",
        "AttackDash",
        "AttackS3S",
        "AttackHi3",
        "AttackLw3",
        "AttackS4S",
        "AttackHi4",
        "AttackLw4",
        "AttackAirN",
        "AttackAirF",
        "AttackAirB",
        "AttackAirHi",
        "AttackAirLw",
        "SpecialHi",
        "SpecialAirHi",
    ] {
        let action = actions
            .get(name)
            .unwrap_or_else(|| panic!("Missing action {name}"));
        assert!(
            action
                .commands
                .iter()
                .any(|e| matches!(e.command, libssbm::action::Command::Hitbox(_))),
            "No hitbox commands: {name}"
        );
        assert!(
            action
                .commands
                .windows(2)
                .all(|pair| pair[0].frame <= pair[1].frame)
        );
        for event in &action.commands {
            if let libssbm::action::Command::Hitbox(h) = &event.command {
                assert!(
                    h.radius > 0. && h.radius < 100. && h.damage > 0. && h.bone < 60,
                    "Bad hitbox: {name}"
                );
            }
        }
    }
    for c in 0..5 {
        let m = Model::load(&dir, c)?;
        assert!(!m.batches.is_empty());
        for (name, clip) in &a.clips {
            for frame in [0., clip.frames * 0.5, clip.frames] {
                let pose = a.pose(&m, name, frame, false);
                assert!(pose.iter().all(|p| p.is_finite()), "{name} {frame}");
            }
        }
        let skin = m.skin(0, &a.pose(&m, "Wait", 3., true));
        assert!(skin.iter().all(|(p, n)| p.is_finite() && n.is_finite()));
        let b = std::fs::read(dir.join(format!("PlSs{}.dat", libssbm_assets::COSTUMES[c])))?;
        Dat::parse(&b)?;
    }
    let model = Model::load(&dir, 0)?;
    let neutral = a.pose(&model, "Fall", 3., true);
    let forward = a.pose(&model, "FallF", 3., true);
    for amount in [0., 0.5, 1.] {
        let blended = libssbm_assets::presentation::blend_pose(&model, &neutral, &forward, amount);
        assert!(blended.iter().all(|m| m.is_finite()));
        if amount == 0. || amount == 1. {
            let expected = if amount == 0. { &neutral } else { &forward };
            for (actual, expected) in blended.iter().zip(expected) {
                for (a, b) in actual
                    .to_cols_array()
                    .into_iter()
                    .zip(expected.to_cols_array())
                {
                    assert!((a - b).abs() < 0.001);
                }
            }
        }
    }
    for clip in [
        "WalkSlow",
        "WalkMiddle",
        "WalkFast",
        "Run",
        "Fall",
        "FallF",
        "FallB",
        "FallAerial",
        "DamageFall",
        "DownBoundU",
        "DownStandU",
    ] {
        assert!(
            a.clips.contains_key(clip),
            "missing shared movement/recovery clip {clip}"
        );
        let length = a.clips[clip].frames;
        let mut poses = std::collections::HashSet::new();
        for sample in 0..20 {
            let frame = length * sample as f32 / 20.;
            let pose = a.pose(&model, clip, frame, true);
            let hash = pose
                .iter()
                .flat_map(|m| m.to_cols_array().map(|v| (v * 1000.).round() as i32))
                .collect::<Vec<_>>();
            poses.insert(hash);
        }
        if clip.starts_with("Walk") || clip == "Run" {
            assert!(
                poses.len() >= 18,
                "{clip} has only {} distinct sampled poses",
                poses.len()
            );
        }
    }
    Ok(())
}
