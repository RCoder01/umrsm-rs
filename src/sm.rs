use core::fmt;
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

pub struct StateMachine<Data> {
    states: HashMap<TypeId, Box<dyn StateHolder<Data>>>,
}

impl<D: fmt::Debug + 'static> fmt::Debug for StateMachine<D> {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> fmt::Result {
        let state_ids: HashMap<TypeId, TypeId> =
            self.states.iter().map(|(k, v)| (*k, v.type_id())).collect();
        f.debug_struct("StateMachine")
            .field("states", &state_ids)
            .finish()
    }
}

impl<D> Default for StateMachine<D> {
    fn default() -> Self {
        Self {
            states: Default::default(),
        }
    }
}

impl<D> StateMachine<D> {
    pub fn add_state<T: State<Data = D>>(&mut self) {
        self.states
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(StateHolderStruct::<T>::default()));
    }

    pub fn runner<Start: State>(
        &self,
        initial_data: D,
        start_transition_data: Start::Income,
    ) -> Option<StateMachineRunner<D>> {
        StateMachineRunner::new::<Start>(self, initial_data, start_transition_data)
    }

    fn make_state(&self, state: TypeId) -> Option<Box<dyn StateInternal<D>>> {
        self.states.get(&state).map(|sh| sh.new())
    }
}

pub struct StateMachineRunner<'a, Data> {
    machine: &'a StateMachine<Data>,
    pub data: Data,
    state: Box<dyn StateInternal<Data>>,
    state_id: TypeId,
}

impl<'a, D: fmt::Debug + 'static> fmt::Debug for StateMachineRunner<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateMachineRunner")
            .field("machine", &self.machine)
            .field("data", &self.data)
            .field("state", &self.state.type_id())
            .field("state_id", &self.state_id)
            .finish()
    }
}
pub enum StepOutcome<'a, Data> {
    Continue {
        machine: StateMachineRunner<'a, Data>,
    },
    Transition {
        machine: StateMachineRunner<'a, Data>,
        start: String,
        transition: String,
        end: String,
    },
    Complete {
        data: Data,
        start: String,
        transition: String,
    },
    StateNotFound {
        start: String,
        transition: String,
        end: TypeId,
    },
    IncorrectTransition {
        start: String,
        transition: String,
        end: String,
        expected_type: TypeId,
        received_data: Box<dyn Any>,
    },
}

impl<'a, D: fmt::Debug + 'static> fmt::Debug for StepOutcome<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Continue { machine } => f
                .debug_struct("Continue")
                .field("machine", machine)
                .finish(),
            Self::Transition {
                machine,
                start,
                transition,
                end,
            } => f
                .debug_struct("Transition")
                .field("machine", machine)
                .field("start", start)
                .field("transition", transition)
                .field("end", end)
                .finish(),
            Self::Complete {
                data,
                start,
                transition,
            } => f
                .debug_struct("Complete")
                .field("data", data)
                .field("start", start)
                .field("transition", transition)
                .finish(),
            Self::StateNotFound {
                start,
                transition,
                end,
            } => f
                .debug_struct("StateNotFound")
                .field("start", start)
                .field("transition", transition)
                .field("end", end)
                .finish(),
            Self::IncorrectTransition {
                start,
                transition,
                end,
                expected_type,
                received_data,
            } => f
                .debug_struct("IncorrectTransition")
                .field("start", start)
                .field("transition", transition)
                .field("end", end)
                .field("expected_type", expected_type)
                .field("received_data", received_data)
                .finish(),
        }
    }
}

impl<'a, D> StateMachineRunner<'a, D> {
    pub fn new<Start: State>(
        machine: &'a StateMachine<D>,
        data: D,
        start: Start::Income,
    ) -> Option<Self> {
        let state_id = TypeId::of::<Start>();
        let mut state = machine.make_state(state_id)?;
        state
            .enter(Box::new(start))
            .expect("Start::Income will always match Start transition expected data");
        Some(Self {
            machine,
            data,
            state,
            state_id,
        })
    }

    pub fn step(mut self) -> StepOutcome<'a, D> {
        let outcome = self.state.handle(&mut self.data);
        if outcome.state_type() != self.state_id {
            let start = self.state.name();
            let transition = outcome.name();
            if outcome.state_type() == TypeId::of::<()>() {
                return StepOutcome::Complete {
                    data: self.data,
                    start,
                    transition,
                };
            }
            self.state_id = outcome.state_type();
            let Some(state) = self.machine.make_state(self.state_id) else {
                return StepOutcome::StateNotFound {
                    start,
                    transition,
                    end: outcome.state_type(),
                };
            };
            self.state = state;
            let end = self.state.name();
            return match self.state.enter(outcome.data()) {
                Ok(_) => StepOutcome::Transition {
                    machine: self,
                    start,
                    transition,
                    end,
                },
                Err(data) => StepOutcome::IncorrectTransition {
                    start,
                    transition,
                    end,
                    expected_type: data.expected,
                    received_data: data.received,
                },
            };
        }
        return StepOutcome::Continue { machine: self };
    }

    pub fn step_debug(self) -> Result<Self, Option<D>> {
        match self.step() {
            StepOutcome::Continue { machine } => Ok(machine),
            StepOutcome::Transition {
                machine,
                start,
                transition,
                end,
            } => {
                println!("{start} --[{transition}]--> {end}");
                Ok(machine)
            }
            StepOutcome::Complete {
                data,
                start,
                transition,
            } => {
                println!("{start} --[{transition}]--> END");
                Err(Some(data))
            }
            StepOutcome::StateNotFound {
                start,
                transition,
                end,
            } => {
                println!("{start} --[{transition}]--> {end:?}? ABORT!");
                println!("Type {end:?} does not exist in the state machine");
                Err(None)
            }
            StepOutcome::IncorrectTransition {
                start,
                transition,
                end,
                expected_type,
                received_data,
            } => {
                println!("{start} --[{transition}!]--> {end}");
                println!("{end} expected incoming data of type {expected_type:?} but received data of type {:?} from transition {transition}", received_data.type_id());
                Err(None)
            }
        }
    }

    pub fn run_to_completion(mut self) -> Option<D> {
        loop {
            self = match self.step_debug() {
                Ok(machine) => machine,
                Err(data) => return data,
            }
        }
    }
}

#[derive(Default)]
struct StateHolderStruct<T: State>(PhantomData<T>);

trait StateHolder<Data> {
    fn new(&self) -> Box<dyn StateInternal<Data>>;
}

impl<D, T, I, O> StateHolder<D> for StateHolderStruct<T>
where
    T: State<Income = I, Transition = O, Data = D>,
    I: 'static,
    O: IntoOutcome + 'static,
{
    fn new(&self) -> Box<dyn StateInternal<D>> {
        Box::new(T::default())
    }
}

#[derive(Debug)]
struct StateEntryError {
    expected: TypeId,
    received: Box<dyn Any>,
}

impl StateEntryError {
    fn from_any<T: 'static>(any: Box<dyn Any>) -> Self {
        Self {
            expected: TypeId::of::<T>(),
            received: any,
        }
    }
}

trait StateInternal<Data> {
    fn enter(&mut self, meta: Box<dyn Any>) -> Result<(), StateEntryError>;
    fn handle(&mut self, data: &mut Data) -> BoxedOutcome;
    fn name(&self) -> String;
}

pub trait State: Default + 'static {
    type Income: 'static;
    type Transition: IntoOutcome;
    type Data;

    #[allow(unused)]
    fn init(&mut self, previous: Box<Self::Income>) {}
    fn handle(&mut self, data: &mut Self::Data) -> Self::Transition;

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }
}

impl<T, I, O, D> StateInternal<D> for T
where
    T: State<Income = I, Transition = O, Data = D>,
    I: 'static,
    O: IntoOutcome + 'static,
{
    fn enter(&mut self, meta: Box<dyn Any>) -> Result<(), StateEntryError> {
        self.init(meta.downcast().map_err(StateEntryError::from_any::<I>)?);
        Ok(())
    }

    fn handle(&mut self, data: &mut D) -> BoxedOutcome {
        self.handle(data).into_outcome()
    }

    fn name(&self) -> String {
        <Self as State>::name(&self)
    }
}

#[derive(Debug)]
pub struct OutcomeData<T: State>(T::Income, String);

impl<T> OutcomeData<T>
where
    T: State,
{
    pub fn new(data: T::Income) -> OutcomeData<T> {
        OutcomeData(data, type_name::<T>().to_string())
    }

    pub const fn with_name(data: T::Income, name: String) -> OutcomeData<T> {
        OutcomeData(data, name)
    }
}

/// If data does not return the incoming data associated with the state corresponding to the result of state_type then the state machine will likely panic
pub trait Outcome {
    fn state_type(&self) -> TypeId;
    fn data(self: Box<Self>) -> Box<dyn Any>;

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }
}

pub type BoxedOutcome = Box<dyn Outcome>;

pub trait IntoOutcome {
    fn into_outcome(self) -> BoxedOutcome;
}

impl Outcome for () {
    fn state_type(&self) -> TypeId {
        TypeId::of::<()>()
    }

    fn data(self: Box<Self>) -> Box<dyn Any> {
        Box::new(())
    }

    fn name(&self) -> String {
        "(Complete)".to_string()
    }
}

impl<T> Outcome for OutcomeData<T>
where
    T: State,
{
    fn state_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn data(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }

    fn name(&self) -> String {
        self.1.clone()
    }
}

impl<T> IntoOutcome for T
where
    T: Outcome + 'static,
{
    fn into_outcome(self) -> BoxedOutcome {
        Box::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::StateMachine;
    use crate::sm::{BoxedOutcome, IntoOutcome, Outcome, OutcomeData, State, StepOutcome};
    use std::any::TypeId;

    #[derive(Debug, PartialEq, Eq)]
    enum Data {
        Normal,
        IncorrectTransition,
        Counting(u32),
    }

    struct Start(bool);

    impl Default for Start {
        fn default() -> Self {
            Self(true)
        }
    }

    #[derive(Debug)]
    enum StartTransition {
        Continue,
        Working,
        WrongData,
    }

    impl Outcome for StartTransition {
        fn state_type(&self) -> std::any::TypeId {
            match self {
                StartTransition::Working | StartTransition::WrongData => TypeId::of::<End>(),
                StartTransition::Continue => TypeId::of::<Start>(),
            }
        }

        fn data(self: Box<Self>) -> Box<dyn std::any::Any> {
            match *self {
                StartTransition::Working => Box::new(150isize),
                StartTransition::WrongData => Box::new(..),
                StartTransition::Continue => Box::new(0usize),
            }
        }

        fn name(&self) -> String {
            format!("{self:?}")
        }
    }

    impl State for Start {
        type Income = usize;
        type Transition = StartTransition;
        type Data = Data;

        fn init(&mut self, previous: Box<Self::Income>) {
            assert!(self.0);
            if *previous < 10 {
                self.0 = false;
            }
        }

        fn handle(&mut self, data: &mut Self::Data) -> Self::Transition {
            if self.0 {
                self.0 = false;
                StartTransition::Continue
            } else {
                let transition = match data {
                    Data::Normal => StartTransition::Working,
                    Data::IncorrectTransition => StartTransition::WrongData,
                    Data::Counting(_) => panic!("Data should not be initially set to unused"),
                };
                *data = Data::Counting(10);
                transition
            }
        }

        fn name(&self) -> String {
            "Start".to_string()
        }
    }

    #[derive(Default)]
    struct End(i32);

    struct EndTransition(bool);

    impl IntoOutcome for EndTransition {
        fn into_outcome(self) -> BoxedOutcome {
            if self.0 {
                ().into_outcome()
            } else {
                OutcomeData::<End>::with_name(0, "EndTransitionContinue".to_string()).into_outcome()
            }
        }
    }

    impl State for End {
        type Income = isize;
        type Transition = EndTransition;
        type Data = Data;

        fn init(&mut self, previous: Box<Self::Income>) {
            assert_eq!(self.0, 0);
            assert_eq!(*previous, 150);
            self.0 = *previous as i32;
        }

        fn handle(&mut self, data: &mut Self::Data) -> Self::Transition {
            self.0 -= 1;
            match data {
                Data::Counting(i) => *i += 1,
                _ => panic!("Data should be set here"),
            };
            if self.0 == 0 {
                return EndTransition(true);
            } else {
                EndTransition(false)
            }
        }

        fn name(&self) -> String {
            "End".to_string()
        }
    }

    #[derive(Default)]
    struct MissingState;

    impl State for MissingState {
        type Income = ();
        type Transition = ();
        type Data = Data;

        fn handle(&mut self, _data: &mut Self::Data) -> Self::Transition {}
    }

    #[test]
    fn missing_start() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();
        machine.add_state::<End>();

        let runner = machine.runner::<MissingState>(Data::Normal, ());
        assert!(runner.is_none());
    }

    #[test]
    fn missing_state() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();

        let runner = machine.runner::<Start>(Data::Normal, 0).unwrap();
        match runner.step() {
            StepOutcome::StateNotFound {
                start,
                transition,
                end,
            } => {
                assert_eq!(start, "Start");
                assert_eq!(transition, "Working");
                assert_eq!(end, TypeId::of::<End>());
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }
    }

    #[test]
    fn wrong_transition_data() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();
        machine.add_state::<End>();

        let runner = machine
            .runner::<Start>(Data::IncorrectTransition, 0)
            .unwrap();
        match runner.step() {
            StepOutcome::IncorrectTransition {
                start,
                transition,
                end,
                expected_type,
                received_data,
            } => {
                assert_eq!(start, "Start");
                assert_eq!(transition, "WrongData");
                assert_eq!(end, "End");
                assert_eq!(expected_type, TypeId::of::<<End as State>::Income>());
                assert_eq!(
                    received_data.downcast().expect("Given type should be .."),
                    Box::new(..)
                );
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }
    }
    #[test]
    fn wrong_transition_data_2() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();
        machine.add_state::<End>();

        let mut runner = machine
            .runner::<Start>(Data::IncorrectTransition, 100)
            .unwrap();
        match runner.step() {
            StepOutcome::Continue { machine } => {
                runner = machine;
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }

        match runner.step() {
            StepOutcome::IncorrectTransition {
                start,
                transition,
                end,
                expected_type,
                received_data,
            } => {
                assert_eq!(start, "Start");
                assert_eq!(transition, "WrongData");
                assert_eq!(end, "End");
                assert_eq!(expected_type, TypeId::of::<<End as State>::Income>());
                assert_eq!(
                    received_data.downcast().expect("Given type should be .."),
                    Box::new(..)
                );
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }
    }

    #[test]
    fn working() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();
        machine.add_state::<End>();

        let mut runner = machine.runner::<Start>(Data::Normal, 0).unwrap();
        match runner.step() {
            StepOutcome::Transition {
                machine,
                start,
                transition,
                end,
            } => {
                runner = machine;
                assert_eq!(start, "Start");
                assert_eq!(transition, "Working");
                assert_eq!(end, "End");
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }

        for _ in 0..149 {
            match runner.step() {
                StepOutcome::Continue { machine } => {
                    runner = machine;
                }
                e @ _ => panic!("Unexpeced runner outcome {e:?}"),
            }
        }

        match runner.step() {
            StepOutcome::Complete {
                data,
                start,
                transition,
            } => {
                assert_eq!(data, Data::Counting(160));
                assert_eq!(start, "End");
                assert_eq!(transition, "(Complete)");
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }
    }

    #[test]
    fn working_2() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();
        machine.add_state::<End>();

        let mut runner = machine.runner::<Start>(Data::Normal, 1000).unwrap();
        match runner.step() {
            StepOutcome::Continue { machine } => {
                runner = machine;
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }

        match runner.step() {
            StepOutcome::Transition {
                machine,
                start,
                transition,
                end,
            } => {
                runner = machine;
                assert_eq!(start, "Start");
                assert_eq!(transition, "Working");
                assert_eq!(end, "End");
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }

        for _ in 0..149 {
            match runner.step() {
                StepOutcome::Continue { machine } => {
                    runner = machine;
                }
                e @ _ => panic!("Unexpeced runner outcome {e:?}"),
            }
        }

        match runner.step() {
            StepOutcome::Complete {
                data,
                start,
                transition,
            } => {
                assert_eq!(data, Data::Counting(160));
                assert_eq!(start, "End");
                assert_eq!(transition, "(Complete)");
            }
            e @ _ => panic!("Unexpeced runner outcome {e:?}"),
        }
    }

    #[test]
    fn working_3() {
        let mut machine = StateMachine::default();
        machine.add_state::<Start>();
        machine.add_state::<End>();

        let runner = machine.runner::<Start>(Data::Normal, 1000).unwrap();

        let data = runner.run_to_completion().expect("State machine should work successfully");
        assert_eq!(data, Data::Counting(160));
    }
}
