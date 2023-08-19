use core::fmt;
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    fmt::Display,
    marker::PhantomData,
};


/// The struct which holds all the states in a state machine
/// 
/// This struct itself does not _run_ any state machine,
/// a StateMachineRunner contains a reference to a StateMachine
/// and an instance of the machine
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
    /// Adds a state to the state machine
    /// The `Data` associated type of the state must match that of all the other states in the state machine
    pub fn add_state<T: State<Data = D>>(&mut self) {
        self.states
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(StateHolderStruct::<T>::default()));
    }

    /// Returns true if T was already in the state machine
    pub fn remove_state<T: State<Data = D>>(&mut self) -> bool {
        self.states.remove(&TypeId::of::<T>()).is_some()
    }

    /// Create a state machine runner from the provided start state
    /// Returns None if the provided start state is not present in the state machine
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


/// The state machine runner runs an instance of a given StateMachine
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

impl<'a, D> StepOutcome<'a, D> {
    /// Returns false if and only if Self == StepOutcome::Continue
    pub fn is_notable(&self) -> bool {
        match self {
            StepOutcome::Continue { .. } => false,
            _ => true,
        }
    }

    /// Prints self if it is non-empty
    pub fn print_if_notable(&self) {
        if self.is_notable() {
            println!("{self}");
        }
    }
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

impl<'a, D> Display for StepOutcome<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepOutcome::Continue { .. } => {Ok(())}
            StepOutcome::Transition {
                start,
                transition,
                end,
                ..
            } => {
                write!(f, "{start} --[{transition}]--> {end}")
            }
            StepOutcome::Complete {
                start, transition, ..
            } => {
                write!(f, "{start} --[{transition}]--> END")
            }
            StepOutcome::StateNotFound {
                start,
                transition,
                end,
            } => {
                write!(f, "{start} --[{transition}]--> {end:?}? ABORT!").and(
                    write!(f, "Type {end:?} does not exist in the state machine"))
                
            }
            StepOutcome::IncorrectTransition {
                start,
                transition,
                end,
                expected_type,
                received_data,
            } => {
                write!(f, "{start} --[{transition}!]--> {end}").and(
                    write!(f, "{end} expected incoming data of type {expected_type:?} but received data of type {:?} from transition {transition}", received_data.type_id()))
            }
        }
    }
}

impl<'a, D> From<StepOutcome<'a, D>> for Result<StateMachineRunner<'a, D>, Option<D>> {
    fn from(value: StepOutcome<'a, D>) -> Self {
        match value {
            StepOutcome::Continue { machine } => Ok(machine),
            StepOutcome::Transition { machine, .. } => Ok(machine),
            StepOutcome::Complete { data, .. } => Err(Some(data)),
            StepOutcome::StateNotFound { .. } => Err(None),
            StepOutcome::IncorrectTransition { .. } => Err(None),
        }
    }
}

impl<'a, D> StateMachineRunner<'a, D> {
    /// Create a state machine runner from the provided start state
    /// Returns None if the provided start state is not present in the given state machine
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

    /// Perform one step of the state machine
    /// Returns an outcome representing all possible outcomes of the step
    pub fn step(mut self) -> StepOutcome<'a, D> {
        let outcome = self.state.handle(&mut self.data);
        if outcome.state_type() == self.state_id {
            return StepOutcome::Continue { machine: self };
        }
        let start = self.state.name();
        let transition = outcome.name();
        self.state_id = outcome.state_type();
        if self.state_id == TypeId::of::<()>() {
            return StepOutcome::Complete {
                data: self.data,
                start,
                transition,
            };
        }
        let Some(state) = self.machine.make_state(self.state_id) else {
            return StepOutcome::StateNotFound {
                start,
                transition,
                end: self.state_id,
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

    /// Run the state machine until it either errors or completes
    pub fn run_to_completion(mut self) -> Option<D> {
        loop {
            self = match self.step().into() {
                Ok(machine) => machine,
                Err(data) => return data,
            }
        }
    }

    /// Run to completion but print all notable steps
    pub fn run_to_completion_verbose(mut self) -> Option<D> {
        loop {
            let result = self.step();
            result.print_if_notable();
            self = match result.into() {
                Ok(machine) => machine,
                Err(data) => return data,
            }
        }
    }
}

/// Used to remember how to construct states through the StateHolder trait
#[derive(Default)]
struct StateHolderStruct<T: State>(PhantomData<T>);

/// Holding `Box<dyn StateHolder>` allows the state machine to only hold the 
/// active state and construction instructions for all other states
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

/// Internal type used to represent possible missing state errors
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

/// Internal representation of a state which is object safe without specifying the associated types
trait StateInternal<Data> {
    fn enter(&mut self, meta: Box<dyn Any>) -> Result<(), StateEntryError>;
    fn handle(&mut self, data: &mut Data) -> BoxedOutcome;
    fn name(&self) -> String;
}

/// The trait needed to represent a state in a state machine
/// 
/// Each type implementing State can only have one associated Data type,
/// however allowing one State type to work with multiple data types is possible using
/// a generic State type and a generic trait impl
pub trait State: Default + 'static {
    type Income: 'static;
    type Transition: IntoOutcome;
    type Data;

    /// This method is run once when initially transitioning to a state
    /// 
    /// previous contains the data sent by the previous state through its Outcome
    #[allow(unused)]
    fn init(&mut self, previous: Box<Self::Income>) {}
    /// This method contains the logic of the state and returns a transition to indicate
    /// which state the state machine should go to next (which may include the current state)
    fn handle(&mut self, data: &mut Self::Data) -> Self::Transition;

    /// This method returns a state name used for debugging and readability
    /// 
    /// The return value of this method is not used for logic anywhere in the state machine
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

/// An Outcome type useful for transitioning from one state to itself
#[derive(Debug, Default)]
pub struct ContinueOutcome<T: State>(PhantomData<T>);

/// An Outcome type useful for transiting from one state to another
/// 
/// Using this type as a Transition or a return from IntoOutcome ensures that
/// the provided data is accurate to the type being transitioned to, 
/// preventing one possible source of error
#[derive(Debug)]
pub struct OutcomeData<T: State>(T::Income, String);

impl<T> OutcomeData<T>
where
    T: State,
{
    /// Construct a new outcome with the default name
    pub fn new(data: T::Income) -> OutcomeData<T> {
        OutcomeData(data, type_name::<T>().to_string())
    }

    pub const fn with_name(data: T::Income, name: String) -> OutcomeData<T> {
        OutcomeData(data, name)
    }
}

/// If data does not return the incoming data associated with the state corresponding
/// to the result of state_type then the state machine will return an error
pub trait Outcome {
    /// The typeid of the next state
    fn state_type(&self) -> TypeId;

    /// If transitioning to a new state, this method is called to provide the `Income` data
    /// 
    /// If transitioning from one state to itself, this method is not called,
    /// so its return value is arbitrary
    fn data(self: Box<Self>) -> Box<dyn Any>;

    /// This method returns a state name used for debugging and readability
    /// 
    /// The return value of this method is not used for logic anywhere in the state machine
    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }
}

pub type BoxedOutcome = Box<dyn Outcome>;

impl Outcome for BoxedOutcome {
    fn state_type(&self) -> TypeId {
        (**self).state_type()
    }

    fn data(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn name(&self) -> String {
        (**self).name()
    }
}

/// This trait defines which outcome a transition follows
/// 
/// IntoOutcome is blanket implemented for all types which implement Outcome,
/// so for states with only a single possible transition, using an Outcome type
/// directly for the transition is likely preferred
pub trait IntoOutcome {
    fn into_outcome(self) -> BoxedOutcome;
}

/// The singular success endpoint for all state machines
/// 
/// All end states or transitions must return () as their transition or outcome
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

impl<T> Outcome for ContinueOutcome<T>
where
    T: State,
{
    fn state_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn data(self: Box<Self>) -> Box<dyn Any> {
        // Since this outcome is only used for states transitioning to themselves,
        // this method will never be called and its return is arbitrary
        // By returning a unit type, this Box does not allocate any heap memory
        Box::new(())
    }

    fn name(&self) -> String {
        format!("ContinueOutcome::<{}>", type_name::<T>())
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
        // IncorrectStartTransition,
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
        // ContinueWrong,
    }

    impl Outcome for StartTransition {
        fn state_type(&self) -> std::any::TypeId {
            match self {
                StartTransition::Working | StartTransition::WrongData => TypeId::of::<End>(),
                StartTransition::Continue => TypeId::of::<Start>(),
                // StartTransition::ContinueWrong => TypeId::of::<Start>(),
            }
        }

        fn data(self: Box<Self>) -> Box<dyn std::any::Any> {
            match *self {
                StartTransition::Working => Box::new(150isize),
                StartTransition::WrongData => Box::new(..),
                StartTransition::Continue => Box::new(0usize),
                // StartTransition::ContinueWrong => Box::new(..),
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
                    // Data::IncorrectStartTransition => StartTransition::ContinueWrong,
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

        let data = runner
            .run_to_completion()
            .expect("State machine should work successfully");
        assert_eq!(data, Data::Counting(160));
    }

    // #[test]
    // fn unknown() {
    //     let mut machine = StateMachine::default();
    //     machine.add_state::<Start>();
    //     machine.add_state::<End>();

    //     let runner = machine.runner::<Start>(Data::Normal, 0).unwrap();
    //     match runner.step() {
    //         StepOutcome::IncorrectTransition {
    //             start,
    //             transition,
    //             end,
    //             expected_type,
    //             received_data,
    //         } => {
    //             assert_eq!(start, "Start");
    //             assert_eq!(transition, "ContinueWrong");
    //             assert_eq!(end, "End");
    //             assert_eq!(expected_type, TypeId::of::<usize>());
    //             assert_eq!(received_data.downcast().expect("Returned box should be of type .."), Box::new(..));
    //         }
    //         e @ _ => panic!("Unexpeced runner outcome {e:?}"),
    //     }
    // }
}
