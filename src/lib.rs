use sm::{State, Outcome, OutcomeInternal};

pub mod sm;

enum State1Outcome {
    Transition1(<State1 as State>::Income),
}

impl OutcomeInternal for State1Outcome {
    type IntoState = State1;

    fn data(self) -> Box<dyn std::any::Any> {
        todo!()
    }
}

#[derive(Default)]
struct State1;

impl State for State1 {
    type Income = i32;

    type Outcome = State1Outcome;

    type Data = ();

    fn init(&mut self, previous: Box<Self::Income>) {
        todo!()
    }

    fn handle(&mut self, data: &mut Self::Data) -> Self::Outcome {
        todo!()
    }
}


