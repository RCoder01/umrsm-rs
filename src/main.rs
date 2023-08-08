fn main() {
    let mut machine = StateMachine::default();
    machine.add_state::<Start>();
    machine.add_state::<Mid>();
    machine.add_state::<Stop>();
    let runner = machine
        .runner::<Start>(0, ())
        .expect("Start exists in machine");
    dbg!(runner.run_to_completion());
}

use umrsm_rs::sm::{IntoOutcome, Outcome, OutcomeData, State, StateMachine};

type Data = usize;

#[derive(Default)]
struct Start;

impl State for Start {
    type Income = ();

    type Transition = OutcomeData<Mid>;

    type Data = Data;

    fn init(&mut self, _previous: Box<Self::Income>) {}

    fn handle(&mut self, data: &mut Self::Data) -> Self::Transition {
        println!("on start: {}", data);
        OutcomeData::with_name(0, "StartTransition".to_string())
    }

    fn name(&self) -> String {
        "Start".to_string()
    }
}

enum MidOutcome {
    Continue(<Mid as State>::Income),
    End(usize),
}

impl IntoOutcome for MidOutcome {
    fn into_outcome(self) -> Box<dyn Outcome> {
        match self {
            MidOutcome::Continue(d) => OutcomeData::<Mid>::with_name(
                d,
                "MidOutcome::Continue".to_string(),
            )
            .into_outcome(),
            MidOutcome::End(v) => OutcomeData::<Stop>::with_name(
                v.to_string(),
                "MidOutcome::End".to_string(),
            )
            .into_outcome(),
        }
    }
}

#[derive(Default)]
struct Mid(i32);

impl State for Mid {
    type Income = i32;
    type Transition = MidOutcome;
    type Data = Data;

    fn init(&mut self, previous: Box<Self::Income>) {
        self.0 = *previous;
    }

    fn handle(&mut self, data: &mut Self::Data) -> Self::Transition {
        if *data > 10 {
            MidOutcome::End(*data)
        } else {
            *data += 1;
            MidOutcome::Continue(*data as i32 + 10000)
        }
    }

    fn name(&self) -> String {
        "Mid".to_string()
    }
}

#[derive(Default)]
struct Stop(String);

impl State for Stop {
    type Income = String;
    type Transition = ();
    type Data = Data;

    fn init(&mut self, previous: Box<Self::Income>) {
        self.0 = *previous;
    }

    fn handle(&mut self, _data: &mut Self::Data) -> Self::Transition {
        println!("on end: {}", self.0);
    }

    fn name(&self) -> String {
        "Stop".to_string()
    }
}
