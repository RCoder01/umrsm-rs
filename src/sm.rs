use std::any::{Any, TypeId};

pub struct StateMachine<Data> {
    states: Vec<Box<dyn StateInternal<Data>>>,
    data: Data,
}

trait StateInternal<Data> {
    fn enter(&mut self, meta: Box<dyn Any>);
    fn handle(&mut self, data: &mut Data) -> Box<dyn Outcome>;
}

pub trait State: Default + 'static {
    type Income;
    type Outcome: Outcome;
    type Data;

    fn init(&mut self, previous: Box<Self::Income>);
    fn handle(&mut self, data: &mut Self::Data) -> Self::Outcome;
}

impl<T, I, O, Data> StateInternal<Data> for T
where
    T: State<Income = I, Outcome = O, Data = Data>,
    I: 'static,
    O: Outcome + 'static,
{
    fn handle(&mut self, data: &mut Data) -> Box<dyn Outcome> {
        Box::new(self.handle(data))
    }

    fn enter(&mut self, meta: Box<dyn Any>) {
        self.init(meta.downcast().unwrap())
    }
}

pub struct OutcomeData<T: State>(T::Income);

/// If data does not return the incoming data associated with the state corresponding to the result of state_type then the state machine will likely panic
pub trait Outcome {
    fn state_type(&self) -> TypeId;
    fn data(self) -> Box<dyn Any>;
}

// trait IntoOutcome<T: State> {
//     fn data(self) -> T::Outcome;
// }

// impl<S: State, T: IntoOutcome<S>> Outcome for T {

// }

// impl<T: State> Outcome for OutcomeData<T> {
//     fn state_type(&self) -> TypeId {
//         TypeId::of::<T>()
//     }

//     fn data(self) -> Box<dyn Any> {
//         Box::new(self.0)
//     }
// }

pub trait OutcomeInternal {
    type IntoState: State;

    fn data(self) -> Box<dyn Any>;
}

impl<T> Outcome for T
where
    T: OutcomeInternal,
{
    fn state_type(&self) -> TypeId {
        TypeId::of::<T::IntoState>()
    }

    fn data(self) -> Box<dyn Any> {
        self.data()
    }
}

impl<T: State> OutcomeInternal for OutcomeData<T> {
    type IntoState = T;

    fn data(self) -> Box<dyn Any> {
        Box::new(self.0)
    }
}
