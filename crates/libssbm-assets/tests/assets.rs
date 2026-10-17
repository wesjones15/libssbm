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
    let mut fst = vec![0; 12 * (REQUIRED.len() + 1)];
    fst[0] = 1;
    word(&mut fst, 8, REQUIRED.len() + 1);
    let mut names = Vec::new();
    let payload = synthetic_dat();
    for (i, name) in REQUIRED.iter().enumerate() {
        word(&mut fst, (i + 1) * 12, names.len());
        word(&mut fst, (i + 1) * 12 + 4, 0x2000 + i * 64);
        word(&mut fst, (i + 1) * 12 + 8, payload.len());
        names.extend(name.as_bytes());
        names.push(0);
        b[0x2000 + i * 64..0x2000 + i * 64 + payload.len()].copy_from_slice(&payload);
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
    Ok(())
}
