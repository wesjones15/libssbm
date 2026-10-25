//! Sample: play Sunshine Mario's idle (`ma_wait.bck`) on Melee Luigi, driving only the
//! joints the two rigs share and leaving the rest (fingers, toes, face, helpers) at bind.
//!
//! Run from `libssbm/` with both asset caches present:
//!   cargo run -p libssbm-assets --release --example sms_idle_on_luigi
//!
//! Writes `target/sms_idle_luigi_<frame>.bmp` for four frames of the clip, front view
//! with a 25 degree yaw. No renderer dependency: a tiny z-buffered triangle rasterizer.
//! The reader and the retarget live in `libssbm_assets::sms`, shared with the viewer.
use glam::{Mat3, Vec3};
use libssbm_assets::{
    fighter::LUIGI,
    model::Model,
    sms::{Clip, Retarget, Rig},
};
use std::io::Write;
use std::path::{Path, PathBuf};

struct Canvas {
    w: usize,
    h: usize,
    rgb: Vec<[u8; 3]>,
    z: Vec<f32>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            rgb: vec![[235, 235, 230]; w * h],
            z: vec![f32::NEG_INFINITY; w * h],
        }
    }

    fn tri(&mut self, p: [Vec3; 3], c: [f32; 3], depth: [f32; 3]) {
        let (minx, maxx) = (
            p[0].x.min(p[1].x).min(p[2].x).floor().max(0.0) as usize,
            p[0].x.max(p[1].x).max(p[2].x).ceil().min(self.w as f32 - 1.0) as usize,
        );
        let (miny, maxy) = (
            p[0].y.min(p[1].y).min(p[2].y).floor().max(0.0) as usize,
            p[0].y.max(p[1].y).max(p[2].y).ceil().min(self.h as f32 - 1.0) as usize,
        );
        let area = (p[1].x - p[0].x) * (p[2].y - p[0].y)
            - (p[2].x - p[0].x) * (p[1].y - p[0].y);
        if area.abs() < 1e-6 {
            return;
        }
        for y in miny..=maxy {
            for x in minx..=maxx {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = ((p[1].x - px) * (p[2].y - py) - (p[2].x - px) * (p[1].y - py)) / area;
                let w1 = ((p[2].x - px) * (p[0].y - py) - (p[0].x - px) * (p[2].y - py)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let d = w0 * depth[0] + w1 * depth[1] + w2 * depth[2];
                let i = y * self.w + x;
                if d <= self.z[i] {
                    continue;
                }
                self.z[i] = d;
                self.rgb[i] = [
                    (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                ];
            }
        }
    }

    fn bmp(&self, path: &Path) -> std::io::Result<()> {
        let row = (self.w * 3 + 3) & !3;
        let size = 54 + row * self.h;
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        let mut hdr = [0u8; 54];
        hdr[0] = b'B';
        hdr[1] = b'M';
        hdr[2..6].copy_from_slice(&(size as u32).to_le_bytes());
        hdr[10..14].copy_from_slice(&54u32.to_le_bytes());
        hdr[14..18].copy_from_slice(&40u32.to_le_bytes());
        hdr[18..22].copy_from_slice(&(self.w as i32).to_le_bytes());
        hdr[22..26].copy_from_slice(&(self.h as i32).to_le_bytes());
        hdr[26..28].copy_from_slice(&1u16.to_le_bytes());
        hdr[28..30].copy_from_slice(&24u16.to_le_bytes());
        hdr[34..38].copy_from_slice(&((row * self.h) as u32).to_le_bytes());
        f.write_all(&hdr)?;
        let pad = vec![0u8; row - self.w * 3];
        for y in (0..self.h).rev() {
            for x in 0..self.w {
                let [r, g, b] = self.rgb[y * self.w + x];
                f.write_all(&[b, g, r])?;
            }
            f.write_all(&pad)?;
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ssbm_assets = PathBuf::from("assets");
    let sms_assets = std::env::var("SMS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../libsms/assets"));
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| "target".into());

    let model = Model::load_for(&ssbm_assets, 0, &LUIGI)?;
    let rig = Rig::from_file(&sms_assets.join("bmd/ma_mdl1.bmd"))?;
    for (i, name) in [
        (1, "center"),
        (14, "jnt_chest"),
        (26, "jnt_head"),
        (15, "jnt_sldr_R"),
        (16, "jnt_arm_R1"),
        (17, "jnt_arm_R2"),
        (18, "jnt_hand_R"),
        (3, "jnt_leg_R1"),
    ] {
        assert_eq!(rig.names[i], name, "unexpected SMS joint order");
    }
    let clip = Clip::from_file(&sms_assets.join("bck/ma_wait.bck"))?;
    let retarget = Retarget::new(&model, &rig);

    // Camera: orthographic, 25 degrees yaw, model faces +Z.
    let (w, h) = (560usize, 760usize);
    let yaw = 25f32.to_radians();
    let (sy, cy) = yaw.sin_cos();

    for k in 0..4 {
        let frame = clip.frames * 0.25 * k as f32;
        let mut world = retarget.pose(&model, &rig, &clip, frame);
        libssbm_assets::sms::ground(&model, &mut world);

        let mut cv = Canvas::new(w, h);
        let skinned: Vec<_> = (0..model.batches.len())
            .map(|bi| model.skin(bi, &world))
            .collect();
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for batch in &skinned {
            for (p, _) in batch {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
        }
        let mid = (lo + hi) * 0.5;
        let projected_w = (hi.x - lo.x) * cy.abs() + (hi.z - lo.z) * sy.abs();
        let scale = (h as f32 * 0.92 / (hi.y - lo.y).max(1.0))
            .min(w as f32 * 0.92 / projected_w.max(1.0));
        let light = Vec3::new(0.35, 0.75, 0.55).normalize();
        let project = |p: Vec3| {
            let x = (p.x - mid.x) * cy - (p.z - mid.z) * sy;
            let z = (p.x - mid.x) * sy + (p.z - mid.z) * cy;
            Vec3::new(
                w as f32 * 0.5 + x * scale,
                h as f32 * 0.5 - (p.y - mid.y) * scale,
                z,
            )
        };

        for (bi, batch) in model.batches.iter().enumerate() {
            let skinned = &skinned[bi];
            let base = if batch.color[0] + batch.color[1] + batch.color[2] > 0.05 {
                [
                    batch.color[0] * 0.55,
                    batch.color[1] * 0.95,
                    batch.color[2] * 0.58,
                ]
            } else {
                [0.36, 0.62, 0.38]
            };
            for tri in batch.indices.chunks_exact(3) {
                let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
                if a >= skinned.len() || b >= skinned.len() || c >= skinned.len() {
                    continue;
                }
                let (pa, na) = skinned[a];
                let (pb, nb) = skinned[b];
                let (pc, nc) = skinned[c];
                let n = ((na + nb + nc) / 3.0).normalize_or_zero();
                let shade = 0.45 + 0.55 * n.dot(light).abs();
                let col = [base[0] * shade, base[1] * shade, base[2] * shade];
                let (qa, qb, qc) = (project(pa), project(pb), project(pc));
                cv.tri([qa, qb, qc], col, [qa.z, qb.z, qc.z]);
            }
        }
        let path = Path::new(&out_dir).join(format!("sms_idle_luigi_{:03}.bmp", frame as i32));
        cv.bmp(&path)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
