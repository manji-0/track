//! Application use cases that coordinate domain services and external systems.
//!
//! Use cases own multi-step workflows and transaction boundaries where a single
//! service method is not enough.

pub mod complete_todo;

pub use complete_todo::{CompleteTodoOutcome, CompleteTodoUseCase};
