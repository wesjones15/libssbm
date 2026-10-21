//! The sword "afterimage" trail of ftafterimage.c (ftCo_800C2FD8 samples, ftCo_800C2600
//! draws): while a move's script has the trail on (command 49), each tick records where the
//! blade's bone is and which way its blade axis points; the last three records are drawn as
//! a triangle strip between an inner and an outer edge along the axis, fading from the
//! newest edge to the oldest. Link is the only fighter here whose scripts use it.
use glam::Vec3;

/// The blade's tuning, from `SwordAttrs` (ftMars/types.h) inside `ftLk_DatAttrs` at +0x64.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// How far toward the newest edge's scale the oldest edge's inner (`x0`) and outer (`x4`)
    /// positions start.
    pub inner_blend: f32,
    pub outer_blend: f32,
    /// Alpha at the newest (`x8`) and oldest (`x9`) edge.
    pub alpha_newest: u8,
    pub alpha_oldest: u8,
    pub inner_color: [u8; 3],
    pub outer_color: [u8; 3],
    /// Distances along the blade axis of the newest edge's inner (`x18`) and outer (`x1C`) points.
    pub inner_scale: f32,
    pub outer_scale: f32,
}

/// One tick's record: the blade bone's world position and its axis (its first matrix column).
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub position: Vec3,
    pub axis: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    /// RGBA, 0 to 255.
    pub color: [u8; 4],
}

const ANGLE_STEP: f32 = 0.087_266_46;

/// The strip, inner and outer vertices alternating, from the newest edge to the oldest.
/// `samples` is newest first and needs at least two entries.
pub fn strip(samples: &[Sample], p: &Params) -> Vec<Vertex> {
    let n = samples.len().min(3);
    if n < 2 {
        return Vec::new();
    }
    let samples = &samples[..n];
    // Cumulative distances exactly as the source computes them: each step measures the
    // outer tip against the previous step's *delta*, not against the previous tip.
    let mut cumulative = vec![0f32; n];
    let mut total = 0f32;
    let mut previous = Vec3::ZERO;
    for (i, s) in samples.iter().enumerate() {
        let delta = s.axis * p.outer_scale + s.position - previous;
        if i != 0 {
            total += delta.length();
            cumulative[i] = total;
        }
        previous = delta;
    }
    if total < 0.00001 {
        total = 1.;
    }
    let scale_diff = p.outer_scale - p.inner_scale;
    let blended_inner = p.inner_blend * scale_diff + p.inner_scale;
    let blended_outer = p.outer_blend * scale_diff + p.inner_scale;
    let inner_diff = p.inner_scale - blended_inner;
    let outer_diff = p.outer_scale - blended_outer;
    let alpha_range = p.alpha_newest as f32 - p.alpha_oldest as f32;
    let mut out = Vec::new();
    let mut interp = 1f32;
    for (i, cur) in samples.iter().enumerate() {
        let outer = interp * outer_diff + blended_outer;
        let inner = interp * inner_diff + blended_inner;
        let alpha = interp * alpha_range + p.alpha_oldest as f32;
        let color = |rgb: [u8; 3], a: f32| [rgb[0], rgb[1], rgb[2], a.clamp(0., 255.) as u8];
        out.push(Vertex { position: cur.axis * inner + cur.position, color: color(p.inner_color, alpha) });
        out.push(Vertex { position: cur.axis * outer + cur.position, color: color(p.outer_color, alpha) });
        let Some(next) = samples.get(i + 1) else { break };
        let next_interp = 1. - cumulative[i + 1] / total;
        // Bridge a wide swing with extra edges, five degrees apart, so the arc stays round.
        let cross = cur.axis.cross(next.axis);
        if cross.length_squared() > 1e-12 {
            let cross = cross.normalize();
            let angle = cur.axis.normalize_or_zero().dot(next.axis.normalize_or_zero()).clamp(-1., 1.).acos();
            let steps = (angle / ANGLE_STEP) as usize;
            if steps > 0 {
                let frac = 1. / (steps + 1) as f32;
                let step_angle = angle * frac;
                let next_outer = next_interp * outer_diff + blended_outer;
                let next_inner = next_interp * inner_diff + blended_inner;
                let next_alpha = next_interp * alpha_range + p.alpha_oldest as f32;
                for j in 1..=steps {
                    let t = j as f32 * frac;
                    let axis = rotate(cur.axis, cross, step_angle * j as f32);
                    let base = cur.position + (next.position - cur.position) * t;
                    let inner = inner + (next_inner - inner) * t;
                    let outer = outer + (next_outer - outer) * t;
                    let alpha = alpha + (next_alpha - alpha) * t;
                    out.push(Vertex { position: axis * inner + base, color: color(p.inner_color, alpha) });
                    out.push(Vertex { position: axis * outer + base, color: color(p.outer_color, alpha) });
                }
            }
        }
        interp = next_interp;
    }
    out
}

/// Rotates `v` about the unit `axis` by `angle` (lbVector_RotateAboutUnitAxis).
fn rotate(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    glam::Quat::from_axis_angle(axis, angle) * v
}
