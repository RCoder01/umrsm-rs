use std::{
    any::type_name,
    time::{Duration, Instant},
};

use crate::sm::{IntoOutcome, State};

pub trait TimedState: Default + 'static {
    type Income: 'static;
    type Transition: IntoOutcome;
    type Data;

    #[allow(unused)]
    fn init(&mut self, previous: Box<Self::Income>) -> Option<Duration> {
        None
    }
    fn handle_if_not_timeout(&mut self, data: &mut Self::Data) -> Self::Transition;
    fn handle_once_timeout(&mut self, data: &mut Self::Data) -> Self::Transition;

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }
}

pub struct TimedStateStruct<S: TimedState> {
    timeout: Duration,
    start_time: Instant,
    state: S,
}

impl<S: TimedState> Default for TimedStateStruct<S> {
    fn default() -> Self {
        Self {
            timeout: Default::default(),
            start_time: Instant::now(),
            state: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct TimedStateIncome<S: TimedState> {
    pub timeout: Option<Duration>,
    pub other: S::Income,
}

impl<S: TimedState> TimedStateIncome<S> {
    pub const fn new(income: S::Income) -> Self {
        Self {
            timeout: None,
            other: income,
        }
    }

    pub const fn with_timeout(income: S::Income, timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
            other: income,
        }
    }
}

impl<S: TimedState> From<(S::Income,)> for TimedStateIncome<S> {
    fn from(value: (S::Income,)) -> Self {
        Self::new(value.0)
    }
}

impl<S: TimedState> From<(S::Income, Duration)> for TimedStateIncome<S> {
    fn from(value: (S::Income, Duration)) -> Self {
        Self::with_timeout(value.0, value.1)
    }
}

impl<S: TimedState> State for TimedStateStruct<S> {
    type Income = S::Income;
    type Transition = S::Transition;
    type Data = S::Data;

    fn init(&mut self, previous: Box<Self::Income>) {
        self.start_time = Instant::now();
        if let Some(timeout) = self.state.init(previous) {
            self.timeout = timeout;
        }
    }

    fn handle(&mut self, data: &mut Self::Data) -> Self::Transition {
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
