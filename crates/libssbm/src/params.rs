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
    pub short_hop_speed: f32,
    pub air_jump_scale: f32,
    pub jumps: u32,
    pub gravity: f32,
    pub fall_speed: f32,
    pub air_accel: f32,
    pub air_base: f32,
    pub air_speed: f32,
    pub air_friction: f32,
    pub fast_fall: f32,
    pub charge_steps: u32,
    pub charge_interval: u32,
    pub missile_smash_window: u32,
    pub screw_velocity: f32,
    pub bomb_velocity: f32,
    pub smash_threshold: f32,
    pub smash_window: u32,
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
            self.fast_fall,
            self.air_jump_scale,
            self.smash_threshold,
        ];
        if positive
            .iter()
            .any(|v| !v.is_finite() || *v <= 0. || *v > 100.)
            || !(1..=10).contains(&self.jumps)
            || self.jump_squat > 60
            || !(1..=1000).contains(&self.charge_steps)
            || !(1..=1000).contains(&self.charge_interval)
            || self.smash_window > 60
            || self.smash_threshold > 1.
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
            short_hop_speed: 1.8,
            air_jump_scale: 1.,
            jumps: 2,
            gravity: 0.1,
            fall_speed: 2.,
            air_accel: 0.03,
            air_base: 0.01,
            air_speed: 1.,
            air_friction: 0.01,
            fast_fall: 3.,
            charge_steps: 10,
            charge_interval: 3,
            missile_smash_window: 3,
            screw_velocity: 3.,
            bomb_velocity: 0.5,
            smash_threshold: 0.8,
            smash_window: 4,
        }
    }
}
