use std::{
    marker::PhantomData,
    ops::{Add, Div, Neg, Range, Rem, Sub},
};

fn ensure_range<T>(mut value: T, range: &Range<T>) -> T
where
    T: Copy
        + Sub<Output = T>
        + Add<Output = T>
        + Rem<Output = T>
        + Neg<Output = T>
        + Div<f32, Output = T>
        + PartialOrd,
{
    let range_len = range.end - range.start;
    value = value - range.start;
    value = value % range_len;
    value = value + range.start;
    if value < -range_len / (2.).into() {
        value = value - range_len;
    }
    if value > range_len / (2.).into() {
        value = value + range_len;
    }
    value
}

pub trait Controller<T, U> {
    fn calculate(&mut self, measurement: T) -> U;
}

pub trait SetpointController<T> {
    fn set_setpoint(&mut self, setpoint: Option<T>);
    fn get_setpoint(&self) -> Option<T>;
}

macro_rules! pass_setpoint {
    ($member: ident, $type: ty) => {
        fn set_setpoint(&mut self, setpoint: Option<$type>) {
            self.$member.set_setpoint(setpoint);
        }

        fn get_setpoint(&self) -> Option<$type> {
            self.$member.get_setpoint()
        }
    };
}

#[derive(Debug, Default)]
pub struct Speed<T> {
    pos: T,
    vel: T,
    acc: T,
}

impl<T> Speed<T> {
    fn new(pos: T, vel: T, acc: T) -> Self {
        Self { pos, vel, acc }
    }
}

#[derive(Debug, Default)]
pub struct SpeedController<T, U, C>
where
    C: Controller<Speed<T>, U>,
{
    last_pos: T,
    last_vel: T,
    controller: C,
    output: PhantomData<U>,
}

impl<T, U, C> SpeedController<T, U, C>
where
    C: Controller<Speed<T>, U>,
{
    pub const fn new(initial_pos: T, initial_vel: T, controller: C) -> Self {
        Self {
            last_pos: initial_pos,
            last_vel: initial_vel,
            controller,
            output: PhantomData,
        }
    }
}

impl<T, U, C> Controller<T, U> for SpeedController<T, U, C>
where
    T: Copy + Sub<Output = T>,
    C: Controller<Speed<T>, U>,
{
    fn calculate(&mut self, measurement: T) -> U {
        let vel = measurement - self.last_pos;
        let acc = vel - self.last_vel;
        self.controller.calculate(Speed::new(measurement, vel, acc))
    }
}

impl<T, U, V, C> SetpointController<V> for SpeedController<T, U, C>
where
    C: Controller<Speed<T>, U> + SetpointController<V>,
{
    pass_setpoint!(controller, V);
}

pub struct ContinuousSpeedController<T, U, C>
where
    C: Controller<Speed<T>, U>,
{
    last_pos: T,
    last_vel: T,
    range: Range<T>,
    controller: C,
    output: PhantomData<U>,
}

impl<T, U, C> ContinuousSpeedController<T, U, C>
where
    C: Controller<Speed<T>, U>,
{
    pub const fn new(initial_pos: T, initial_vel: T, range: Range<T>, controller: C) -> Self {
        Self {
            last_pos: initial_pos,
            last_vel: initial_vel,
            range,
            controller,
            output: PhantomData,
        }
    }
}

impl<C> Controller<Speed<f32>, f32> for ContinuousSpeedController<f32, f32, C>
where
    C: Controller<Speed<f32>, f32>,
{
    fn calculate(&mut self, measurement: Speed<f32>) -> f32 {
        let pos = ensure_range(measurement.pos, &self.range);
        let vel = ensure_range(measurement.pos - self.last_pos, &self.range);
        let acc = vel - self.last_vel;
        self.controller.calculate(Speed::new(pos, vel, acc))
    }
}

impl<T, U, V, C> SetpointController<V> for ContinuousSpeedController<T, U, C>
where
    C: Controller<Speed<T>, U> + SetpointController<V>,
{
    pass_setpoint!(controller, V);
}

#[derive(Debug, Default)]
pub struct ContinuousController<T, U, C>
where
    C: Controller<T, U>,
{
    range: Range<T>,
    controller: C,
    output: PhantomData<U>,
}

impl<T, U, C> ContinuousController<T, U, C>
where
    C: Controller<T, U>,
{
    pub const fn new(range: Range<T>, controller: C) -> Self {
        Self {
            range,
            controller,
            output: PhantomData,
        }
    }
}

impl<C> Controller<f32, f32> for ContinuousController<f32, f32, C>
where
    C: Controller<f32, f32>,
{
    fn calculate(&mut self, measurement: f32) -> f32 {
        self.controller
            .calculate(ensure_range(measurement, &self.range))
    }
}

impl<T, U, V, C> SetpointController<V> for ContinuousController<T, U, C>
where
    C: Controller<T, U> + SetpointController<V>,
{
    pass_setpoint!(controller, V);
}

#[derive(Debug, Default)]
pub struct AdditiveController<C1, C2> {
    controller_1: C1,
    controller_2: C2,
}

impl<C1, C2> AdditiveController<C1, C2> {
    pub const fn new(controller_1: C1, controller_2: C2) -> Self {
        Self {
            controller_1,
            controller_2,
        }
    }
}

impl<T, C1, C2> Controller<T, f32> for AdditiveController<C1, C2>
where
    T: Copy,
    C1: Controller<T, f32>,
    C2: Controller<T, f32>,
{
    fn calculate(&mut self, measurement: T) -> f32 {
        self.controller_1.calculate(measurement) + self.controller_2.calculate(measurement)
    }
}

impl<T, C1, C2> SetpointController<T> for AdditiveController<C1, C2>
where
    T: Copy,
    C1: SetpointController<T>,
    C2: SetpointController<T>,
{
    fn set_setpoint(&mut self, setpoint: Option<T>) {
        self.controller_1.set_setpoint(setpoint);
        self.controller_2.set_setpoint(setpoint);
    }

    fn get_setpoint(&self) -> Option<T> {
        self.controller_1
            .get_setpoint()
            .or(self.controller_2.get_setpoint())
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

impl Controller<Speed<f32>, f32> for PIDController {
    fn calculate(&mut self, speeds: Speed<f32>) -> f32 {
        let Some(setpoint) = self.setpoint else {return 0.;};
        let error = speeds.pos - setpoint;
        let d_output = speeds.vel * self.config.derivative;
        let p_output = error * self.config.proportional;
        self.accum = (self.accum + error).clamp(-self.config.max_i, self.config.max_i);
        let i_output = self.accum * self.config.integral;
        p_output + i_output + d_output
    }
}

impl SetpointController<f32> for PIDController {
    fn set_setpoint(&mut self, new_setpoint: Option<f32>) {
        if new_setpoint != self.setpoint {
            self.setpoint = new_setpoint;
            self.accum = 0.;
        }
    }

    fn get_setpoint(&self) -> Option<f32> {
        self.setpoint
    }
}


// #[derive(Debug, Default)]
// pub struct ContinuousPIDController {
//     pid: PIDController,
//     range: Range<f32>,
// }

// impl ContinuousPIDController {
//     pub const fn new(pid: PIDController, range: Range<f32>) -> Self {
//         Self {
//             pid,
//             range,
//         }
//     }
// }

// impl Controller<Speed<f32>, f32> for ContinuousPIDController {
//     fn calculate(&mut self, speeds: Speed<f32>) -> f32 {
//         let Some(setpoint) = self.pid.setpoint else {
//             return self.pid.calculate(speeds);
//         };
//         let midpoint = (self.range.end - self.range.start) / 2. + self.range.start;
//         let output = if (setpoint - self.range.start).abs() < (setpoint - midpoint).abs() {
//             self.pid.setpoint = Some()
//         } else {

//         };
//         0.
//     }
// }

// impl SetpointController<f32> for ContinuousPIDController {
//     pass_setpoint!(pid, f32);
// }

#[derive(Debug, Default)]
pub struct LinearFeedforward<T> {
    pub ks: T,
    pub kv: T,
    pub ka: T,
    pub kj: T,
}

impl<T> LinearFeedforward<T> {
    pub const fn new(ks: T, kv: T, ka: T, kj: T) -> Self {
        Self { ks, kv, ka, kj }
    }
}

impl Controller<Speed<f32>, f32> for LinearFeedforward<f32> {
    fn calculate(&mut self, speed: Speed<f32>) -> f32 {
        self.ks + speed.pos * self.kv + speed.vel * self.ka + speed.acc * self.kj
    }
}

impl SetpointController<f32> for LinearFeedforward<f32> {
    fn set_setpoint(&mut self, _setpoint: Option<f32>) {}

    fn get_setpoint(&self) -> Option<f32> {
        None
    }
}
