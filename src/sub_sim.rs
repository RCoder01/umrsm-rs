use std::{fmt::Debug, time::Duration};

use umrsm_rs::controller::{Controller, SetpointController};

#[derive(Debug)]
enum DOFTarget {
    Twist(f32),
    Pose(f32),
}

impl Default for DOFTarget {
    fn default() -> Self {
        Self::Twist(0.)
    }
}

#[derive(Debug, Default)]
struct DOFController<C>
where
    C: Controller<f32, f32> + SetpointController<f32>,
{
    max_output: f32,
    target: DOFTarget,
    closed_loop_controller: C,
}

impl<C> Controller<f32, f32> for DOFController<C>
where
    C: Controller<f32, f32> + SetpointController<f32>,
{
    fn calculate(&mut self, measurement: f32) -> f32 {
        match self.target {
            DOFTarget::Twist(target) => target * self.max_output,
            DOFTarget::Pose(target) => {
                self.closed_loop_controller.set_setpoint(Some(target));
                self.closed_loop_controller.calculate(measurement)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct ClosedDOF<C>
where
    C: Controller<f32, f32> + SetpointController<f32>,
{
    pos: f32,
    vel: f32,
    pub controller: C,
    target: DOFTarget,
    pub resistance: f32,
    pub max_app_acc: f32,
    is_heave: bool,
    // pub variance: f32,
}

const BUOYANCY: f32 = -1.;
const GRAVITY: f32 = 10.;

impl<C> ClosedDOF<C>
where
    C: Controller<f32, f32> + SetpointController<f32> + Default,
{
    pub const fn new(controller: C, resistance: f32, max_app_acc: f32) -> Self {
        Self {
            pos: 0.,
            vel: 0.,
            controller,
            target: DOFTarget::Twist(0.),
            resistance,
            max_app_acc,
            is_heave: false,
        }
    }

    pub const fn new_heave(controller: C, resistance: f32, max_app_acc: f32) -> Self {
        Self {
            pos: 0.,
            vel: 0.,
            controller,
            target: DOFTarget::Twist(0.),
            resistance,
            max_app_acc,
            is_heave: true,
        }
    }
}

impl<C> ClosedDOF<C>
where
    C: Controller<f32, f32> + SetpointController<f32>,
{
    pub fn set_target_twist(&mut self, target_twist: f32) {
        // twist = percentage of max output
        self.target = DOFTarget::Twist(target_twist * self.max_app_acc);
    }

    pub fn set_target_pose(&mut self, target_pose: f32) {
        self.target = DOFTarget::Pose(target_pose);
    }

    pub const fn pose(&self) -> f32 {
        self.pos
    }

    pub fn step(&mut self, delta: Duration) {
        let mut acceleration = -self.resistance * self.vel;
        match self.target {
            DOFTarget::Twist(applied) => {
                self.controller.set_setpoint(None);
                self.controller.calculate(self.pos);
                acceleration += applied.clamp(-self.max_app_acc, self.max_app_acc);
            }
            DOFTarget::Pose(setpoint) => {
                self.controller.set_setpoint(Some(setpoint));
                acceleration += self
                    .controller
                    .calculate(self.pos)
                    .clamp(-self.max_app_acc, self.max_app_acc);
            }
        }
        if self.is_heave {
            if self.pos >= 0. {
                acceleration += BUOYANCY;
            } else {
                acceleration = GRAVITY;
            }
        }
        self.vel += acceleration * delta.as_secs_f32();
        self.pos += self.vel * delta.as_secs_f32();
        if self.is_heave {
            print!("{acceleration:.4}\t{:.4}\t{:.4}\t\r", self.vel, self.pos);
        }
        // self.pose_heave = self.target_pose_heave + (self.pose_heave - self.target_pose_heave) / delta.as_secs_f32().exp()
    }

    pub fn overwrite_print(&self) {
        print!("\t\t{}\t{}\t\r", self.pos, self.vel);
    }
}

#[derive(Debug, Default)]
pub struct OpenDOF {
    pos: f32,
    vel: f32,
    target: f32,
    pub resistance: f32,
    pub max_app_acc: f32,
    // pub variance: f32,
}

impl OpenDOF {
    pub const fn new(resistance: f32, max_app_acc: f32) -> Self {
        Self {
            pos: 0.,
            vel: 0.,
            target: 0.,
            resistance,
            max_app_acc,
        }
    }

    pub fn set_target_twist(&mut self, target_twist: f32) {
        // twist = percentage of max output
        self.target = target_twist * self.max_app_acc;
    }

    pub fn step(&mut self, delta: Duration) {
        let mut acceleration = -self.resistance * self.vel;
        acceleration += self.target.clamp(-self.max_app_acc, self.max_app_acc);
        self.vel += acceleration * delta.as_secs_f32();
        self.pos += self.vel * delta.as_secs_f32();
    }
}

#[derive(Debug)]
pub struct Submarine<CH, CY, CP, CR>
where
    CH: Controller<f32, f32> + SetpointController<f32>,
    CY: Controller<f32, f32> + SetpointController<f32>,
    CP: Controller<f32, f32> + SetpointController<f32>,
    CR: Controller<f32, f32> + SetpointController<f32>,
{
    heave: ClosedDOF<CH>,
    surge: OpenDOF,
    sway: OpenDOF,
    yaw: ClosedDOF<CY>,
    pitch: ClosedDOF<CP>,
    roll: ClosedDOF<CR>,
}

impl<C1, C4, C5, C6> Submarine<C1, C4, C5, C6>
where
    C1: Controller<f32, f32> + SetpointController<f32>,
    C4: Controller<f32, f32> + SetpointController<f32>,
    C5: Controller<f32, f32> + SetpointController<f32>,
    C6: Controller<f32, f32> + SetpointController<f32>,
{
    pub const fn new(
        heave: ClosedDOF<C1>,
        surge: OpenDOF,
        sway: OpenDOF,
        yaw: ClosedDOF<C4>,
        pitch: ClosedDOF<C5>,
        roll: ClosedDOF<C6>,
    ) -> Self {
        Self {
            heave,
            surge,
            sway,
            yaw,
            pitch,
            roll,
        }
    }

    pub fn step(&mut self, delta: Duration) {
        self.heave.step(delta);
        self.surge.step(delta);
        self.sway.step(delta);
        self.yaw.step(delta);
        self.pitch.step(delta);
        self.roll.step(delta);
    }

    pub const fn heave(&self) -> &ClosedDOF<C1> {
        &self.heave
    }

    pub fn heave_mut(&mut self) -> &mut ClosedDOF<C1> {
        &mut self.heave
    }

    pub const fn surge(&self) -> &OpenDOF {
        &self.surge
    }

    pub fn surge_mut(&mut self) -> &mut OpenDOF {
        &mut self.surge
    }

    pub const fn sway(&self) -> &OpenDOF {
        &self.sway
    }

    pub fn sway_mut(&mut self) -> &mut OpenDOF {
        &mut self.sway
    }

    pub const fn yaw(&self) -> &ClosedDOF<C4> {
        &self.yaw
    }

    pub fn yaw_mut(&mut self) -> &mut ClosedDOF<C4> {
        &mut self.yaw
    }

    pub const fn pitch(&self) -> &ClosedDOF<C5> {
        &self.pitch
    }

    pub fn pitch_mut(&mut self) -> &mut ClosedDOF<C5> {
        &mut self.pitch
    }

    pub const fn roll(&self) -> &ClosedDOF<C6> {
        &self.roll
    }

    pub fn roll_mut(&mut self) -> &mut ClosedDOF<C6> {
        &mut self.roll
    }
}
