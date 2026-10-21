/// Common fighter tuning shared by every character: locomotion, jump, air
/// drift, gravity and smash timing. Special-move tuning lives in each fighter's
/// behavior module (see `crate::samus::Specials`), not here.
#[derive(Clone, Debug)]
pub struct Params {
    pub walk_accel: f32,
    pub walk_base: f32,
    pub walk_speed: f32,
    pub walk_animation_rates: [f32; 3],
    pub walk_animation_thresholds: [f32; 2],
    pub run_animation_rate: f32,
    pub friction: f32,
    pub dash_initial: f32,
    pub dash_accel: f32,
    pub dash_base: f32,
    pub run_speed: f32,
    pub jump_squat: u32,
    pub jump_speed: f32,
    pub jump_horizontal: f32,
    pub jump_momentum: f32,
    pub jump_horizontal_max: f32,
    pub air_jump_horizontal: f32,
    pub jump_sound: u32,
    pub air_jump_sound: u32,
    pub short_hop_speed: f32,
    pub air_jump_scale: f32,
    pub jumps: u32,
    pub gravity: f32,
    pub fall_speed: f32,
    pub air_accel: f32,
    pub air_base: f32,
    pub air_speed: f32,
    pub air_horizontal_max: f32,
    pub fall_pose_threshold: f32,
    pub fall_blend_response: f32,
    pub air_friction: f32,
    pub fast_fall: f32,
    pub smash_threshold: f32,
    pub smash_window: u32,
    /// Ticks after a jab starts in which another press continues the chain
    /// (ftCo_DatAttrs jab_2_input_window, jab_3_input_window).
    pub jab_2_window: u32,
    pub jab_3_window: u32,
    /// Presses and releases of A, counted from the first jab, that start the rapid jab
    /// (ftCo_DatAttrs rapid_jab_window).
    pub rapid_jab_window: u32,
}
impl Params {
    pub fn validate(&self) -> Result<(), &'static str> {
        let positive = [
            self.walk_speed,
            self.walk_animation_rates[0],
            self.walk_animation_rates[1],
            self.walk_animation_rates[2],
            self.run_animation_rate,
            self.run_speed,
            self.jump_speed,
            self.short_hop_speed,
            self.gravity,
            self.fall_speed,
            self.air_speed,
            self.air_horizontal_max,
            self.jump_horizontal_max,
            self.fast_fall,
            self.air_jump_scale,
            self.smash_threshold,
        ];
        if positive
            .iter()
            .any(|v| !v.is_finite() || *v <= 0. || *v > 100.)
            || !(1..=10).contains(&self.jumps)
            || self.jump_squat > 60
            || self.smash_window > 60
            || self.smash_threshold > 1.
            || !self.fall_pose_threshold.is_finite()
            || !(0. ..1.).contains(&self.fall_pose_threshold)
        {
            return Err("invalid fighter tuning");
        }
        Ok(())
    }
    /// Synthetic fixture only. Runtime hosts require shipped tuning from the disc.
    pub fn fixture() -> Self {
        Self {
            walk_accel: 0.1,
            walk_base: 0.02,
            walk_speed: 1.,
            walk_animation_rates: [0.3, 0.6, 1.],
            walk_animation_thresholds: [0.4, 0.8],
            run_animation_rate: 1.5,
            friction: 0.08,
            dash_initial: 1.5,
            dash_accel: 0.06,
            dash_base: 0.02,
            run_speed: 1.5,
            jump_squat: 3,
            jump_speed: 3.,
            jump_horizontal: 0.5,
            jump_momentum: 1.,
            jump_horizontal_max: 1.5,
            air_jump_horizontal: 1.,
            jump_sound: u32::MAX,
            air_jump_sound: u32::MAX,
            short_hop_speed: 1.8,
            air_jump_scale: 1.,
            jumps: 2,
            gravity: 0.1,
            fall_speed: 2.,
            air_accel: 0.03,
            air_base: 0.01,
            air_speed: 1.,
            air_horizontal_max: 1.5,
            fall_pose_threshold: 0.3,
            fall_blend_response: 0.1,
            air_friction: 0.01,
            fast_fall: 3.,
            smash_threshold: 0.8,
            smash_window: 4,
            jab_2_window: 20,
            jab_3_window: 20,
            rapid_jab_window: 4,
        }
    }
}
