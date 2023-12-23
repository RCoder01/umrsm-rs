use std::{
    any::type_name,
    time::{Duration, Instant}, marker::PhantomData,
};

use crate::sm::{IntoOutcome, State};

/// Type useful for States which may loop endlessly
/// 
/// Adds a timeout, after which a separate method is called to allow for
/// short-circuiting after a given period
pub trait TimedState<Data: 'static>: Default + 'static {
    type Income: 'static;
    type Transition: IntoOutcome;

    #[allow(unused)]
    fn init(&mut self, previous: Box<Self::Income>) -> Option<Duration> {
        None
    }
    fn handle_if_not_timeout(&mut self, data: &mut Data) -> Self::Transition;
    fn handle_once_timeout(&mut self, data: &mut Data) -> Self::Transition;

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }
}

/// This struct wraps TimedState types and provides a functional State implementation
/// for all TimedState types
/// 
/// ```
/// use std::time::{Duration, Instant};
/// use umrsm::{sm::{BoxedOutcome, ContinueOutcome, IntoOutcome, StateMachine}, sm_ext::{TimedState, TimedStateStruct}};
/// 
/// #[derive(Default)]
/// struct MayLoopInner;
/// 
/// impl TimedState<usize> for MayLoopInner {
///     type Income = ();
///     type Transition = BoxedOutcome;
/// 
///     fn init(&mut self, previous: Box<Self::Income>) -> Option<Duration> {
///         Some(Duration::from_secs_f32(1.5))
///     }
/// 
///     fn handle_if_not_timeout(&mut self, data: &mut usize) -> Self::Transition {
///         *data += 1;
///         println!("Start: {:?}", Instant::now());
///         ContinueOutcome::<usize, MayLoop>::default().into_outcome()
///     }
/// 
///     fn handle_once_timeout(&mut self, data: &mut usize) -> Self::Transition {
///         println!("End: {:?}", Instant::now());
///         ().into_outcome()
///     }
/// }
/// 
/// type MayLoop = TimedStateStruct<usize, MayLoopInner>;
/// 
/// fn main() {
///     let mut machine = StateMachine::default();
///     machine.add_state::<MayLoop>();
/// 
///     let runner = machine.runner::<MayLoop>(0, ()).expect("MayLoop exists in the machine");
///     assert_ne!(runner.run_to_completion().expect("Should not error"), 0);
/// }
pub struct TimedStateStruct<D: 'static, S: TimedState<D>> {
    timeout: Duration,
    start_time: Instant,
    state: S,
    data: PhantomData<D>,
}

impl<D: 'static, S: TimedState<D>> Default for TimedStateStruct<D, S> {
    fn default() -> Self {
        Self {
            timeout: Default::default(),
            start_time: Instant::now(),
            state: Default::default(),
            data: PhantomData,
        }
    }
}

impl<D: 'static, S: TimedState<D>> State<D> for TimedStateStruct<D, S> {
    type Income = S::Income;
    type Transition = S::Transition;

    fn init(&mut self, previous: Box<Self::Income>) {
        self.start_time = Instant::now();
        if let Some(timeout) = self.state.init(previous) {
            self.timeout = timeout;
        }
    }

    fn handle(&mut self, data: &mut D) -> Self::Transition {
        if (Instant::now() - self.start_time) > self.timeout {
            self.state.handle_once_timeout(data)
        } else {
            self.state.handle_if_not_timeout(data)
        }
    }

    fn name(&self) -> String {
        self.state.name()
    }
}
