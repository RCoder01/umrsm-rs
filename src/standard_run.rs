#![allow(unused)]
use std::{
    default,
    io::{stdout, Write},
    time::{Duration, Instant, SystemTime},
};

use crate::{
    constants,
    sub_sim::{AngleDOF, Heave, Lateral, Submarine},
};
use umrsm_rs::{
    controller::{
        ensure_range, FFConfig, LinearFeedforward, PIDConfig, PIDController, SpeedManager,
    },
    sm::{BoxedOutcome, IntoOutcome, Outcome, OutcomeData, State, StateMachine},
    sm_ext::{TimedState, TimedStateIncome, TimedStateStruct},
};

#[derive(Debug)]
struct Data {
    sub: Submarine,
}

impl Data {
    fn print_pose(&self) {
        self.sub.print_pose()
    }
}

const DATA: Data = {
    Data {
        sub: Submarine::new(
            Heave::new(
                PIDConfig::new(-10., 0., -280., 0.),
                FFConfig::new(1., 0., 0., 0.),
                0.,
                4.,
            ),
            Lateral::new(0., 4.),
            Lateral::new(0., 4.),
            AngleDOF::new(PIDConfig::new(-10., 0., -600., 0.), 0., 40.),
            AngleDOF::new(PIDConfig::new(0., 0., 0., 0.), 0., 4.),
            AngleDOF::new(PIDConfig::new(0., 0., 0., 0.), 0., 4.),
        ),
    }
};

pub fn run() {
    let mut machine = StateMachine::default();
    machine.add_state::<Start>();
    machine.add_state::<Submerge>();
    machine.add_state::<AlignGate>();
    machine.add_state::<ApproachGate>();

    let mut runner = machine
        .runner::<Start>(DATA, ())
        .expect("Machine has Start");
    let data = loop {
        let duration = Duration::from_secs_f32(1. / 50.);
        runner = match runner.step_debug() {
            Ok(machine) => machine,
            Err(data) => break data,
        };
        for _ in 0..10 {
            runner.data.sub.step(duration / 10);
        }
        runner.data.print_pose();
        print!("\r");
        stdout().flush().unwrap();
        std::thread::sleep(duration);
    };
    dbg!(data);
}

#[derive(Default)]
struct Start;

impl State for Start {
    type Income = ();
    type Transition = OutcomeData<Submerge>;
    type Data = Data;

    fn handle(&mut self, _data: &mut Self::Data) -> Self::Transition {
        OutcomeData::with_name((), "Complete".to_string())
    }
}

type Submerge = TimedStateStruct<SubmergeInner>;

struct SubmergeInner {
    target_heave: f32,
    tolerance: f32,
}

impl Default for SubmergeInner {
    fn default() -> Self {
        Self {
            target_heave: constants::SUBMERGE_HEAVE,
            tolerance: constants::SUBMERGE_HEAVE_TOLERANCE,
        }
    }
}

#[derive(Debug)]
enum SubmergeOutcome {
    Unreached,
    Reached,
    Timeout,
}

impl IntoOutcome for SubmergeOutcome {
    fn into_outcome(self) -> BoxedOutcome {
        let name = format!("{:?}", self);
        match self {
            SubmergeOutcome::Reached | SubmergeOutcome::Timeout => {
                OutcomeData::<AlignGate>::with_name((), name).into_outcome()
            }
            SubmergeOutcome::Unreached => {
                OutcomeData::<Submerge>::with_name((), name).into_outcome()
            }
        }
    }
}

impl TimedState for SubmergeInner {
    type Income = ();
    type Transition = SubmergeOutcome;
    type Data = Data;

    fn init(&mut self, previous: Box<Self::Income>) -> Option<Duration> {
        Some(constants::SUBMERGE_TIMEOUT)
    }

    fn handle_if_not_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
        if (data.sub.heave().pose() - self.target_heave).abs() < self.tolerance {
            SubmergeOutcome::Reached
        } else {
            data.sub.heave_mut().set_target_pose(self.target_heave);
            SubmergeOutcome::Unreached
        }
    }

    fn handle_once_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
        SubmergeOutcome::Timeout
    }
}

type AlignGate = TimedStateStruct<AlignGateInner>;

struct AlignGateInner {
    target_yaw: f32,
    tolerance: f32,
}

impl Default for AlignGateInner {
    fn default() -> Self {
        Self {
            target_yaw: constants::ALIGN_GATE_YAW,
            tolerance: constants::ALIGN_GATE_YAW_TOLERANCE,
        }
    }
}

#[derive(Debug)]
enum AlignGateOutcome {
    Unreached,
    Reached,
    Timeout,
}

impl IntoOutcome for AlignGateOutcome {
    fn into_outcome(self) -> BoxedOutcome {
        let name = format!("{self:?}");
        match self {
            AlignGateOutcome::Unreached => {
                OutcomeData::<AlignGate>::with_name(Default::default(), name).into_outcome()
            }
            AlignGateOutcome::Reached | AlignGateOutcome::Timeout => {
                OutcomeData::<ApproachGate>::with_name((), name).into_outcome()
            }
        }
    }
}

impl TimedState for AlignGateInner {
    type Income = ();
    type Transition = AlignGateOutcome;
    type Data = Data;

    fn init(&mut self, previous: Box<Self::Income>) -> Option<Duration> {
        self.target_yaw = ensure_range(self.target_yaw, (0.)..(360.));
        Some(constants::ALIGN_GATE_TIMEOUT)
    }

    fn handle_if_not_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
        data.sub.yaw_mut().set_target_pose(self.target_yaw);
        if ensure_range(data.sub.yaw().pose() - self.target_yaw, (-180.)..(180.)).abs()
            < self.tolerance
        {
            AlignGateOutcome::Reached
        } else {
            AlignGateOutcome::Unreached
        }
    }

    fn handle_once_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
        AlignGateOutcome::Timeout
    }
}

type ApproachGate = TimedStateStruct<ApproachGateInner>;

struct ApproachGateInner {
    surge_speed: f32,
}

impl Default for ApproachGateInner {
    fn default() -> Self {
        Self {
            surge_speed: constants::APPROACH_GATE_SURGE_SPEED,
        }
    }
}

impl TimedState for ApproachGateInner {
    type Income = ();
    type Transition = ();
    type Data = Data;

    fn init(&mut self, previous: Box<Self::Income>) -> Option<Duration> {
        Some(constants::APPROACH_GATE_TIMEOUT)
    }

    fn handle_if_not_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
        // todo!()
    }

    fn handle_once_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
        // todo!()
    }
}
