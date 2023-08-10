#![allow(unused)]
use std::{
    default,
    io::{stdout, Write},
    time::{Duration, Instant, SystemTime},
};

use crate::{constants, sub_sim::{Submarine, ClosedDOF, OpenDOF}};
use umrsm_rs::{
    sm::{IntoOutcome, OutcomeData, State, StateMachine},
    sm_ext::{TimedState, TimedStateIncome, TimedStateStruct}, controller::{AdditiveController, SpeedController, PIDController, PIDConfig, LinearFeedforward, Controller, SetpointController, ContinuousSpeedController},
};

#[derive(Debug, Default)]
struct UnimplementedController;

impl<T, U> Controller<T, U> for UnimplementedController {
    fn calculate(&mut self, measurement: T) -> U {
        unimplemented!()
    }
}

impl<T> SetpointController<T> for UnimplementedController {
    fn set_setpoint(&mut self, setpoint: Option<T>) {}

    fn get_setpoint(&self) -> Option<T> {
        None
    }
}

#[derive(Debug, Default)]
struct Data {
    sub: Submarine<>,
}

const DATA: Data = {
    Data {
        sub: Submarine::new(
            ClosedDOF::new_heave(AdditiveController::new(
                SpeedController::new(
                    0.,
                    0.,
                    PIDController::new(PIDConfig {
                        proportional: 10.,
                        derivative: -280.,
                        ..PIDConfig::ZERO
                    })),
                    LinearFeedforward::new(1., 0., 0., 0.)), 0., 4.),
            OpenDOF::new(0., 4.),
            OpenDOF::new(0., 4.),
            ClosedDOF::new(ContinuousSpeedController::new(0., 0., -180..180, ), 0., 4.),
        ),

    }
};

// impl Data {
//     fn config(&mut self) {
//         let mut heave = self.sub.heave_mut();
//         heave.pid.config = PIDConfig {
//             proportional: 10.,
//             derivative: -280.,
//             ..Default::default()
//         };
//         heave.pid.feedforward = Some(Box::new(LinearFeedforward::new(1., 0., 0.)));
//         heave.max_app_acc = 4.;
//         heave.set_target_twist(2.);

//         let mut surge = self.sub.surge_mut();
//     }
// }

pub fn run() {
    //     let mut machine = StateMachine::default();
    //     machine.add_state::<Start>();
    //     machine.add_state::<Submerge>();
    //     machine.add_state::<AlignGate>();

    //     let mut data = Data::default();

    //     let mut yaw = data.sub.yaw_mut();
    //     dbg!(data.sub.heave());
    //     for i in 0.. {
    //         let duration = Duration::from_secs_f32(1. / 50.);
    //         data.sub.step(duration);
    //         // data.sub.heave().overwrite_print();
    //         // dbg!(data.sub.heave(), i);
    //         stdout().flush().unwrap();
    //         std::thread::sleep(duration);
    //     }

    //     return;
    //     let mut runner = machine
    //         .runner::<Start>(Default::default(), ())
    //         .expect("Machine has Start");
    //     let data = loop {
    //         runner = match runner.step_debug() {
    //             Ok(machine) => machine,
    //             Err(data) => break data,
    //         };
    //         for _ in 0..10 {
    //             runner.data.sub.step(Duration::from_secs_f32(1. / 500.));
    //         }
    //         dbg!(&runner.data);
    //     };
    //     dbg!(data);
}

// #[derive(Default)]
// struct Start;

// impl State for Start {
//     type Income = ();
//     type Transition = OutcomeData<Submerge>;
//     type Data = Data;

//     fn handle(&mut self, _data: &mut Self::Data) -> Self::Transition {
//         OutcomeData::with_name(
//             TimedStateIncome::with_timeout((), constants::SUBMERGE_TIMEOUT),
//             "Complete".to_string(),
//         )
//     }
// }

// type Submerge = TimedStateStruct<SubmergeInner>;

// struct SubmergeInner {
//     target_heave: f32,
//     tolerance: f32,
// }

// impl Default for SubmergeInner {
//     fn default() -> Self {
//         Self {
//             target_heave: constants::SUBMERGE_HEAVE,
//             tolerance: constants::SUBMERGE_HEAVE_TOLERANCE,
//         }
//     }
// }

// #[derive(Debug)]
// enum SubmergeOutcome {
//     Unreached,
//     Reached,
//     Timeout,
// }

// impl IntoOutcome for SubmergeOutcome {
//     fn into_outcome(self) -> Box<dyn umrsm_rs::sm::Outcome> {
//         let name = format!("{:?}", self);
//         match self {
//             SubmergeOutcome::Reached | SubmergeOutcome::Timeout => {
//                 OutcomeData::<AlignGate>::with_name((), name).into_outcome()
//             }
//             SubmergeOutcome::Unreached => {
//                 OutcomeData::<Submerge>::with_name(Default::default(), name).into_outcome()
//             }
//         }
//     }
// }

// impl TimedState for SubmergeInner {
//     type Income = ();
//     type Transition = SubmergeOutcome;
//     type Data = Data;

//     fn handle_if_not_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
//         if (data.sub.heave().pose() - self.target_heave).abs() < self.tolerance {
//             SubmergeOutcome::Reached
//         } else {
//             data.sub.heave_mut().set_target_pose(self.target_heave);
//             println!("{}", data.sub.heave().pose());
//             SubmergeOutcome::Unreached
//         }
//     }

//     fn handle_once_timeout(&mut self, data: &mut Self::Data) -> Self::Transition {
//         SubmergeOutcome::Timeout
//     }
// }

// #[derive(Default)]
// struct AlignGate;

// enum AlignGateOutcome {
//     Unreached,
//     Reached,
//     Timeout,
// }

// impl State for AlignGate {
//     type Income = ();
//     type Transition = ();
//     type Data = Data;

//     fn handle(&mut self, data: &mut Self::Data) -> Self::Transition {}
// }
