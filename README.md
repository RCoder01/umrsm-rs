UMRSM-RS
---
A state machine framework inspired by the University of Michigan RoboSub State Machine framework in python, but written in rust (🚀🚀🚀🚀🚀🔥🔥🔥🚀🚀) for increased type safety (and performance I guess).

Each state in the state machine is represented by a type which implements `State`, which has three associated types, `Income`, `Transition`, and `Data`.

`Data` is the data type which is shared by all states in the state machine and which persists for the duration of the machine's operation.

`Income` is the data type which should be passed to the new state's init method on transition.

`Transition` is the data type which is output by the current state on each iteration of the state machine. `Transition` must implement `IntoOutcome`, which converts the transition type into a `BoxedOutcome` (`Box<dyn Outcome>`). Note that `IntoOutcome` is blanket implemented for all types which implement `Outcome`.

`Outcome`s determine the state which the state machine should transition to. If the state determined by `Outcome` is different to the current state of the state machine, then the `Outcome` must also provides the `Income` data for the new state. Should the data provided by `Outcome` not match the type of `Income` for the new state or if the type indicated by the `Outcome` is not present, the `StateMachine`'s step function will return an error.

All state machines loop forever, result in error, or end with a transition to `()`.

Disclaimer: this project is not affiliated with The Rust Foundation™ and does not claim to be in any form.