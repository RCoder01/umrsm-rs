use std::ops::Range;

pub fn ensure_range(mut value: f32, range: Range<f32>) -> f32 {
    let range_len = range.end - range.start;
    value -= range.start;
    value %= range_len;
    value += range.start;
    if value < -range_len / 2. {
        value -= range_len;
    }
    if value > range_len / 2. {
        value += range_len;
    }
    value
}

#[derive(Debug, Default, Clone)]
pub struct Speeds {
    pos: f32,
    vel: f32,
    acc: f32,
}

impl Speeds {
    pub fn new(pos: f32, vel: f32, acc: f32) -> Self {
        Self { pos, vel, acc }
    }
}

#[derive(Debug, Default)]
pub struct SpeedManager {
    last_pos: f32,
    last_vel: f32,
}

impl SpeedManager {
    pub const fn new(initial_pos: f32, initial_vel: f32) -> Self {
        Self {
            last_pos: initial_pos,
            last_vel: initial_vel,
        }
    }
}

impl SpeedManager {
    pub fn calculate(&mut self, measurement: f32) -> Speeds {
        let vel = measurement - self.last_pos;
        let acc = vel - self.last_vel;
        self.last_pos = measurement;
        self.last_vel = vel;
        Speeds::new(measurement, vel, acc)
    }

    pub fn calculate_continuous(&mut self, measurement: f32, range: Range<f32>) -> Speeds {
        let pos = ensure_range(measurement, range.clone());
        let vel = ensure_range(measurement - self.last_pos, range.clone());
        let acc = vel - self.last_vel;
        self.last_pos = measurement;
        self.last_vel = vel;
        Speeds::new(pos, vel, acc)
    }
}

#[derive(Debug, Default)]
pub struct PIDConfig {
    pub proportional: f32,
    pub integral: f32,
    pub derivative: f32,
    pub max_i: f32,
}

impl PIDConfig {
    pub const ZERO: Self = Self::new(0., 0., 0., 0.);

    pub const fn new(proportional: f32, integral: f32, derivative: f32, max_i: f32) -> Self {
        Self {
            proportional,
            integral,
            derivative,
            max_i,
        }
    }
}

#[derive(Debug, Default)]
pub struct PIDController {
    accum: f32,
    pub config: PIDConfig,
    setpoint: Option<f32>,
}

impl PIDController {
    pub const fn new(config: PIDConfig) -> Self {
        Self {
            accum: 0.,
            config,
            setpoint: None,
        }
    }
}

impl PIDController {
    fn calculate_inner(&mut self, speeds: Speeds, setpoint: f32) -> f32 {
        let error = speeds.pos - setpoint;
        let d_output = speeds.vel * self.config.derivative;
        let p_output = error * self.config.proportional;
        self.accum = (self.accum + error).clamp(-self.config.max_i, self.config.max_i);
        let i_output = self.accum * self.config.integral;
        p_output + i_output + d_output
    }

    pub fn calculate(&mut self, speeds: Speeds) -> f32 {
        let Some(setpoint) = self.setpoint else {return 0.;};
        self.calculate_inner(speeds, setpoint)
    }

    pub fn calculate_continuous(&mut self, speeds: Speeds, range: Range<f32>) -> f32 {
        let Some(setpoint) = self.setpoint else {return 0.;};
        let range_len = range.end - range.start;
        let mut setpoint = ensure_range(setpoint, range);
        if (setpoint - speeds.pos) < -range_len / 2. {
            setpoint += range_len;
        }
        if (setpoint - speeds.pos) > range_len / 2. {
            setpoint -= range_len;
        }
        self.calculate_inner(speeds, setpoint)
    }

    pub fn set_setpoint(&mut self, new_setpoint: Option<f32>) {
        if new_setpoint != self.setpoint {
            self.setpoint = new_setpoint;
            self.accum = 0.;
        }
    }

    pub fn get_setpoint(&self) -> Option<f32> {
        self.setpoint
    }
}

#[derive(Debug, Default)]
pub struct FFConfig {
    pub ks: f32,
    pub kv: f32,
    pub ka: f32,
    pub kj: f32,
}

impl FFConfig {
    pub const fn new(ks: f32, kv: f32, ka: f32, kj: f32) -> Self {
        Self { ks, kv, ka, kj }
    }
}

#[derive(Debug, Default)]
pub struct LinearFeedforward {
    pub config: FFConfig,
}

impl LinearFeedforward {
    pub const fn new(config: FFConfig) -> Self {
        Self { config }
    }

    pub fn calculate(&self, speed: Speeds) -> f32 {
        self.config.ks
            + self.config.kv * speed.pos
            + self.config.ka * speed.vel
            + self.config.kj * speed.acc
    }
}
