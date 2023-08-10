use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

pub struct StateMachine<Data> {
    states: HashMap<TypeId, Box<dyn StateHolder<Data>>>,
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
}

impl<'a, D> StateMachineRunner<'a, D> {
    pub fn new<Start: State>(
        machine: &'a StateMachine<D>,
        data: D,
        start: Start::Income,
    ) -> Option<Self> {
        let state_id = TypeId::of::<Start>();
        let mut state = machine.make_state(state_id)?;
        state.enter(Box::new(start));
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
                return StepOutcome::StateNotFound{
                    start,
                    transition,
                    end: outcome.state_type(),
                };
            };
            self.state = state;
            self.state.enter(outcome.data());
            let end = self.state.name();
            return StepOutcome::Transition {
                machine: self,
                start,
                transition,
                end,
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

trait StateInternal<Data> {
    fn enter(&mut self, meta: Box<dyn Any>);
    fn handle(&mut self, data: &mut Data) -> Box<dyn Outcome>;
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
    fn enter(&mut self, meta: Box<dyn Any>) {
        self.init(
            meta.downcast()
                .expect("Expected 'enter' type to match 'income' type"),
        )
    }

    fn handle(&mut self, data: &mut D) -> Box<dyn Outcome> {
        self.handle(data).into_outcome()
    }

    fn name(&self) -> String {
        <Self as State>::name(&self)
    }
}

pub struct OutcomeData<T: State>(T::Income, String);

/// If data does not return the incoming data associated with the state corresponding to the result of state_type then the state machine will likely panic
pub trait Outcome {
    fn state_type(&self) -> TypeId;
    fn data(self: Box<Self>) -> Box<dyn Any>;

    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }
}

pub trait IntoOutcome {
    fn into_outcome(self) -> Box<dyn Outcome>;
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
    fn into_outcome(self) -> Box<dyn Outcome> {
        Box::new(self)
    }
}
