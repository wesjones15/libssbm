//! Host adaptations shared by Melee fighters, independent of engine types.
/// Invert the fixed-tick apex after gravity is applied before displacement.
/// Unlike sqrt(2*g*h), this keeps short/ground/aerial jumps at the requested
/// actual height even when the source gravity differs from the host's gravity.
pub fn jump_velocity(height: f32, gravity: f32) -> f32 {
    if !height.is_finite() || !gravity.is_finite() || height <= 0. || gravity <= 0. {
        return 0.;
    }
    let mut low = 0.;
    let mut high = (2. * height * gravity).sqrt() + gravity;
    for _ in 0..32 {
        let v = (low + high) * 0.5;
        let n = (v / gravity).ceil().max(1.) - 1.;
        let apex = n * v - gravity * n * (n + 1.) * 0.5;
        if apex < height {
            low = v;
        } else {
            high = v;
        }
    }
    (low + high) * 0.5
}
