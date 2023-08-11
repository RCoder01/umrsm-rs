use std::{fmt::Debug, time::Duration};

use umrsm_rs::controller::{FFConfig, LinearFeedforward, PIDConfig, PIDController, SpeedManager};

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
pub struct Heave {
    pos: f32,
    vel: f32,
    pub pid: PIDController,
    pub ff: LinearFeedforward,
    speeds: SpeedManager,
    target: DOFTarget,
    pub resistance: f32,
    pub max_app_acc: f32,
    // pub variance: f32,
}

const BUOYANCY: f32 = -1.;
const GRAVITY: f32 = 10.;

impl Heave {
    pub const fn new(
        pid_config: PIDConfig,
        ff_config: FFConfig,
        resistance: f32,
        max_app_acc: f32,
    ) -> Self {
        Self {
            pos: 0.,
            vel: 0.,
            pid: PIDController::new(pid_config),
            ff: LinearFeedforward::new(ff_config),
            speeds: SpeedManager::new(0., 0.),
            target: DOFTarget::Twist(0.),
            resistance,
            max_app_acc,
        }
    }

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
        let speeds = self.speeds.calculate(self.pos);
        match self.target {
            DOFTarget::Twist(applied) => {
                self.pid.set_setpoint(None);
                self.pid.calculate(speeds);
                acceleration += applied.clamp(-self.max_app_acc, self.max_app_acc);
            }
            DOFTarget::Pose(setpoint) => {
                self.pid.set_setpoint(Some(setpoint));
                acceleration += self
                    .pid
                    .calculate(speeds.clone())
                    .clamp(-self.max_app_acc, self.max_app_acc);
                acceleration += self.ff.calculate(speeds)
            }
        }

        if self.pos >= 0. {
            acceleration += BUOYANCY;
        } else {
            acceleration = GRAVITY;
        }

        self.vel += acceleration * delta.as_secs_f32();
        self.pos += self.vel * delta.as_secs_f32();
        print!("{acceleration:3.3}\t{:3.3}\t{:3.3}\t\r", self.vel, self.pos);
    }
}

#[derive(Debug, Default)]
pub struct AngleDOF {
    pos: f32,
    vel: f32,
    speeds: SpeedManager,
    pub pid: PIDController,
    target: DOFTarget,
    pub resistance: f32,
    pub max_app_acc: f32,
    // pub variance: f32,
}

impl AngleDOF {
    pub const fn new(pid: PIDConfig, resistance: f32, max_app_acc: f32) -> Self {
        Self {
            pos: 0.,
            vel: 0.,
            pid: PIDController::new(pid),
            speeds: SpeedManager::new(0., 0.),
            target: DOFTarget::Twist(0.),
            resistance,
            max_app_acc,
        }
    }

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
        let speeds = self.speeds.calculate(self.pos);
        match self.target {
            DOFTarget::Twist(applied) => {
                self.pid.set_setpoint(None);
                self.pid.calculate_continuous(speeds, (0.)..(360.));
                acceleration += applied.clamp(-self.max_app_acc, self.max_app_acc);
            }
            DOFTarget::Pose(setpoint) => {
                self.pid.set_setpoint(Some(setpoint));
                acceleration += self
                    .pid
                    .calculate_continuous(speeds, (0.)..(360.))
                    .clamp(-self.max_app_acc, self.max_app_acc);
            }
        }

        self.vel += acceleration * delta.as_secs_f32();
        self.pos += self.vel * delta.as_secs_f32();
        // if let DOFTarget::Pose(_) = self.target {
        //     print!("{acceleration:3.3}\t{:3.3}\t{:3.3}\t\r", self.vel, self.pos);
        // }
    }
}

#[derive(Debug, Default)]
pub struct Lateral {
    pos: f32,
    vel: f32,
    target: f32,
    pub resistance: f32,
    pub max_app_acc: f32,
    // pub variance: f32,
}

impl Lateral {
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
pub struct Submarine {
    heave: Heave,
    surge: Lateral,
    sway: Lateral,
    yaw: AngleDOF,
    pitch: AngleDOF,
    roll: AngleDOF,
}

impl Submarine {
    pub const fn new(
        heave: Heave,
        surge: Lateral,
        sway: Lateral,
        yaw: AngleDOF,
        pitch: AngleDOF,
        roll: AngleDOF,
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

    pub fn print_pose(&self) {
        print!("{:3.3}    \t", self.heave.pos);
        print!("{:3.3}    \t", self.surge.pos);
        print!("{:3.3}    \t", self.sway.pos);
        print!("{:3.3}    \t", self.yaw.pos);
        print!("{:3.3}    \t", self.pitch.pos);
        print!("{:3.3}    \t", self.roll.pos);
    }

    pub fn step(&mut self, delta: Duration) {
        self.heave.step(delta);
        self.surge.step(delta);
        self.sway.step(delta);
        self.yaw.step(delta);
        self.pitch.step(delta);
        self.roll.step(delta);
    }

    pub const fn heave(&self) -> &Heave {
        &self.heave
    }

    pub fn heave_mut(&mut self) -> &mut Heave {
        &mut self.heave
    }

    pub const fn surge(&self) -> &Lateral {
        &self.surge
    }

    pub fn surge_mut(&mut self) -> &mut Lateral {
        &mut self.surge
    }

    pub const fn sway(&self) -> &Lateral {
        &self.sway
    }

    pub fn sway_mut(&mut self) -> &mut Lateral {
        &mut self.sway
    }

    pub const fn yaw(&self) -> &AngleDOF {
        &self.yaw
    }

    pub fn yaw_mut(&mut self) -> &mut AngleDOF {
        &mut self.yaw
    }

    pub const fn pitch(&self) -> &AngleDOF {
        &self.pitch
    }

    pub fn pitch_mut(&mut self) -> &mut AngleDOF {
        &mut self.pitch
    }

    pub const fn roll(&self) -> &AngleDOF {
        &self.roll
    }

    pub fn roll_mut(&mut self) -> &mut AngleDOF {
        &mut self.roll
    }
}
